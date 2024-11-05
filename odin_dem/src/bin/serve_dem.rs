/*
 * Copyright © 2024, United States Government, as represented by the Administrator of 
 * the National Aeronautics and Space Administration. All rights reserved.
 *
 * The “ODIN” software is licensed under the Apache License, Version 2.0 (the "License"); 
 * you may not use this file except in compliance with the License. You may obtain a copy 
 * of the License at http://www.apache.org/licenses/LICENSE-2.0.
 *
 * Unless required by applicable law or agreed to in writing, software distributed under
 * the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND,
 * either express or implied. See the License for the specific language governing permissions
 * and limitations under the License.
 */
#![allow(unused)]

//! module to (eventually) implement a minimal [WMS](https://portal.ogc.org/files/?artifact_id=14416) server for
//! elevation data. The main end point is
//! 
//!    GET <host>:<port>/GetMap?<query>
//! 
//! with query parameters
//! 
//!       crs    : coordinate reference system ("epsg:<number>")
//!       bbox   : comma separated list of coordinate boundaries in crs dimensions 
//!                (xmin,ymin,xmax,ymax - corresponds to west,south,east,north in epsg:4326)
//!       format : response data image type ("tif", "png")
//!       width  : response data (image) width in pixels - we keep this optional and if not set use source data resolution
//!       height : response data (image) height in pixels - see 'width'


use std::{default::Default, error::Error, net::{IpAddr, SocketAddr}, sync::Arc, path::{Path,PathBuf}, collections::HashMap};

use axum::{
    extract::{MatchedPath,Query},
    http::Request,
    response::{Html,IntoResponse},
    Router,
    routing::get
};
use http::StatusCode;
use serde_derive::{Serialize,Deserialize};
use structopt::StructOpt;
use tokio::net::TcpListener;
use tower_http::{
    classify::{ServerErrorsAsFailures, SharedClassifier},
    trace::TraceLayer,
};
use tracing::{info_span, Level, Span};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};
use anyhow::Result;

use odin_build::set_bin_context;
use odin_common::{define_serde_struct, fs::ensure_writable_dir, geo::BoundingBox, strings::{deserialize_arr4,parse_array}, if_let};
use odin_server::{spawn_server_task,ServerConfig, server_error};
use odin_dem::{load_config, DemSRS, DemImgType, get_wh_dem, get_res_dem};


/// DEM configuration data
define_serde_struct! { pub DemConfig: Debug = 
    pub vrt_path: String,
    pub wms_capabilities_path: Option<String>,
    pub max_cache: u64       [default = "default_max_cache"]
}

fn default_max_cache() -> u64 { 1024*1024*100 } // 100MB - wouldn't be enough if used for map tiles

/// non-WMS query version for given width/height
define_serde_struct! { GetWhDemQuery: Debug =
    crs: String              [default = "default_crs"],
    bbox:[f64;4]             [deserialize_with="odin_common::strings::deserialize_arr4"],
    width: u32,
    height: u32,
    format: String           [default = "default_format"],
}

// non-WMS version for given res_x, res_y
define_serde_struct! { GetResDemQuery: Debug =
    crs: String              [default = "default_crs"],
    bbox:[f64;4]             [deserialize_with="odin_common::strings::deserialize_arr4"],
    res_x: f64,
    res_y: f64,
    format: String           [default = "default_format"]
}

fn default_crs()->String { "EPSG:4326".into() }
fn default_format()->String { "image/tif".into() }


#[tokio::main]
async fn main () -> Result<()> {
    odin_build::set_bin_context!();

    let dem_config: Arc<DemConfig> = Arc::new( load_config("dem.ron")?);
    let srv_config: ServerConfig = load_config("dem_server.ron")?;
    let cache_dir = Arc::new(odin_build::cache_dir().join("odin_dem"));
    ensure_writable_dir( cache_dir.as_ref());

    let mut router = Router::new()
        .route( "/GetWhDem", get({
            let cfg = dem_config.clone();
            let cache_dir = cache_dir.clone();
            move |query:Query<GetWhDemQuery>| { get_wh_dem_handler( query, cfg, cache_dir) }
        }))
        .route( "/GetResDem", get({
            let cfg = dem_config.clone();
            let cache_dir = cache_dir.clone();
            move |query:Query<GetResDemQuery>| { get_res_dem_handler( query, cfg, cache_dir) }
        }));

    if dem_config.wms_capabilities_path.is_some() {
        router = router.route( "/WMS", get({
            let cfg = dem_config.clone();
            let cache_dir = cache_dir.clone();
            move |query: Query<HashMap<String, String>>| { get_wms_handler( query, cfg, cache_dir) }
        }))
    }

    // start cache cleanup task
    if dem_config.max_cache > 0 {
    }

    println!("serving WMS DEM on {}", srv_config.url());
    let server_task = spawn_server_task( &srv_config, router);
    Ok( server_task.await? )
}

//--- WMS handlers

async fn get_wms_handler (q: Query<HashMap<String, String>>, config: Arc<DemConfig>, cache_dir: Arc<PathBuf>) -> impl IntoResponse {
    match q.get("request").map(|s| s.as_str()) {
        Some("GetMap") => get_map_request( q, config, cache_dir).await.into_response(),
        Some("GetCapabilities") => get_capabilities_request( q, config, cache_dir).await.into_response(),
        _ => (StatusCode::BAD_REQUEST, "invalid REQUEST param").into_response()
    }
}

async fn get_map_request( q: Query<HashMap<String, String>>, config: Arc<DemConfig>, cache_dir: Arc<PathBuf>) -> impl IntoResponse {
    if_let! {
        Some(dem_srs) = { q.get("crs").and_then(|s| DemSRS::from_srs_spec(s)) } else { (StatusCode::BAD_REQUEST, "bad or missing CRS param").into_response() },
        Some(dem_img) = { q.get("format").and_then(|s| DemImgType::for_mime_type(s)) } else { (StatusCode::BAD_REQUEST, "bad or missing FORMAT param").into_response() },
        Some(width) = { q.get("width").and_then(|s| s.parse::<u32>().ok()) } else { (StatusCode::BAD_REQUEST, "bad or missing WIDTH param").into_response() },
        Some(height) = { q.get("height").and_then(|s| s.parse::<u32>().ok()) } else { (StatusCode::BAD_REQUEST, "bad or missing HEIGHT param").into_response() },
        Some(bbox) = { q.get("bbox").and_then(|s| parse_array::<f64,4>(s, ',').ok()) } else { (StatusCode::BAD_REQUEST, "bad or missing BBOX param").into_response() } => {
            let bbox = BoundingBox::from_wsen(&bbox);

            match get_wh_dem( &bbox, dem_srs, width, height, dem_img, &config.vrt_path, cache_dir.as_ref()) {
                Ok(file_path) =>  odin_server::file_response( &file_path, true).await.into_response(),
                Err(e) => server_error("failed to create DEM file").into_response()
            }
        }
    }
}

async fn get_capabilities_request( q: Query<HashMap<String, String>>, config: Arc<DemConfig>, cache_dir: Arc<PathBuf>) -> impl IntoResponse {
    if_let! {
        Some("WMS") = { q.get("service").map(|s| s.as_str()) } else { (StatusCode::BAD_REQUEST, "invalid SERVICE param").into_response() },
        Some(path) = { &config.wms_capabilities_path.as_ref().map(|p| Path::new(p).to_path_buf()) } else { (StatusCode::INTERNAL_SERVER_ERROR, "no capabilities").into_response() },
        true = { path.is_file() } else { (StatusCode::INTERNAL_SERVER_ERROR, "no capabilities").into_response() } => {
            odin_server::file_response( &path, false).await.into_response()
        }
    }
}

// TODO - we still need a GetCapabilities handler for a proper WMS server

//--- the non-WMS handlers

async fn get_wh_dem_handler (Query(q): Query<GetWhDemQuery>, config: Arc<DemConfig>, cache_dir: Arc<PathBuf>) -> impl IntoResponse {
    if_let! {
        Some(dem_srs) = { DemSRS::from_srs_spec( &q.crs) } else { (StatusCode::BAD_REQUEST, "unsupported target SRS").into_response() },
        Some(dem_img) = { DemImgType::for_mime_type( &q.format) } else { (StatusCode::BAD_REQUEST, "unsupported DEM image type").into_response() } => {
            let bbox = BoundingBox::from_wsen( &q.bbox);

            match get_wh_dem( &bbox, dem_srs, q.width, q.height, dem_img, &config.vrt_path, cache_dir.as_ref()) {
                Ok(file_path) =>  odin_server::file_response( &file_path, true).await.into_response(),
                Err(e) => server_error("failed to create DEM file").into_response()
            }
        }
    }
}

async fn get_res_dem_handler (Query(q): Query<GetResDemQuery>, config: Arc<DemConfig>, cache_dir: Arc<PathBuf>) -> impl IntoResponse {
    if_let! {
        Some(dem_srs) = { DemSRS::from_srs_spec( &q.crs) } else { (StatusCode::BAD_REQUEST, "unsupported target SRS").into_response() },
        Some(dem_img) = { DemImgType::for_mime_type( &q.format) } else { (StatusCode::BAD_REQUEST, "unsupported DEM image type").into_response() } => {
            let bbox = BoundingBox::from_wsen( &q.bbox);

            match get_res_dem( &bbox, dem_srs, q.res_x, q.res_y, dem_img, &config.vrt_path, cache_dir.as_ref()) {
                Ok(file_path) =>  odin_server::file_response( &file_path, true).await.into_response(),
                Err(e) => server_error("failed to create DEM file").into_response()
            }
        }
    }
}