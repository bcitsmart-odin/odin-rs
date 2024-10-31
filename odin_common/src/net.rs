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

///! common utility functions for network operations

use std::{fs::File, io::Write, path::Path, collections::HashMap};
use reqwest::{Client,IntoUrl,header::{HeaderMap,HeaderName,HeaderValue}};
use regex::Regex;
use crate::{define_error, if_let, fs::file_length};
use lazy_static::lazy_static;

const SCHEME: usize = 1;
const USR: usize = 2;
const HOST: usize = 3;
const PORT: usize = 4;
const PATH: usize = 5;
const QUERY: usize = 6;

lazy_static! {
    // [scheme,user,host,port,path,query]
    static ref URL_RE: Regex = Regex::new( r"(.+)://(?:(.+)@)?([^:/]+)(?::(\d+))?(?:/([^?]+))?(?:\?(.+))?").unwrap();
    static ref FNAME_RE: Regex = Regex::new( r"(?:.*/)(.*)").unwrap();
}

define_error!{ pub OdinNetError = 
    IOError(#[from] std::io::Error) : "IO error: {0}",
    HttpError(#[from] reqwest::Error) : "http error: {0}",
    OpFailed(String) : "operation failed: {0}"
}

pub type Result<T> = std::result::Result<T, OdinNetError>;

pub fn get_headermap (headers: &Vec<String>) -> Result<HeaderMap> {
    if headers.is_empty() {
        Ok(HeaderMap::new())
    } else {
        let mut hm = HeaderMap::new();
        for h in headers {
            if let Some(idx) = h.find(':') {
                let k = h[0..idx].trim();
                let v = h[idx+1..].trim();
                hm.append( 
                    HeaderName::from_bytes( k.as_bytes()).map_err(|e| OdinNetError::OpFailed(e.to_string()))?, 
                    HeaderValue::from_str(v).map_err(|e| OdinNetError::OpFailed(e.to_string()))?
                );
            }
        }
        Ok(hm)
    }
}

/// fetch file from URL using HTTP GET method. Retrieve in chunks to support large files
/// Note this requires a full URL
pub async fn get_file (client: &Client, url: &str, opt_headers: &Option<HeaderMap>, dir: &str) -> Result<u64>  {
    if let Some(fname) = url_file_name( url) {
        let path = Path::new( dir).join(fname);
        download_url( client, url, opt_headers, &path).await
    } else {
        Err( OdinNetError::OpFailed(format!("not a file URL: {}", url)) )
    }
}

pub async fn get_differing_size_file (client: &Client, url: &str, opt_headers: &Option<HeaderMap>, dir: &str) -> Result<u64>  {
    if let Some(fname) = url_file_name( url) {
        let path = Path::new( dir).join(fname);

        if_let! {
            Ok(file) = File::open( &path),
            Ok(local_len) = file_length(&file),
            Ok(remote_len) = get_content_length( client, url, opt_headers).await => {
                if local_len == remote_len {
                    return Ok(local_len) // we assume equal length means same content
                }
            }
        }

        // if we get here there either was no local file or it has a differing content size so we retrieve and overwrite
        download_url( client, url, opt_headers, &path).await

    } else {
        Err( OdinNetError::OpFailed(format!("not a file URL: {}", url)) )
    }
}

async fn download_url<P: AsRef<Path>> (client: &Client, url: &str, opt_headers: &Option<HeaderMap>, path: P) -> Result<u64> {
    let mut file = File::create(path)?;
    let mut len: u64 = 0;

    let mut req = client.get(url);
    if let Some(headermap) = &opt_headers {
        req = req.headers(headermap.clone())
    }
    
    let mut response = req.send().await?;

    while let Some(chunk) = response.chunk().await? {
       len += chunk.len() as u64;
       file.write_all(&chunk)?;
    }

    file.flush()?;
    Ok(len)
}

/// get content-length of URL without retrieving the actual content
pub async fn get_content_length (client: &Client, url: &str, opt_headers: &Option<HeaderMap>)->Result<u64> {
    let mut req = client.head(url);
    if let Some(headermap) = &opt_headers {
        req = req.headers(headermap.clone())
    }

    let response = req.send().await?;

    let headers = response.headers();
    if let Some(content_length) = headers.get("content-length") {
        content_length.to_str()
            .map_err(|e| OdinNetError::OpFailed("invalid header value".into()))?
            .parse()
            .map_err(|e| OdinNetError::OpFailed("invalid content-length".into()))
    } else {
        Err( OdinNetError::OpFailed("no content-length".into()))
    }
}


/// get filename part (last path element) of complete URL
/// NOTE - this does not work for partial (relative) URLs
pub fn url_file_name<'a> (url: &'a str) -> Option<&'a str> {
    URL_RE.captures( url)
    .and_then( |cap| cap.get( PATH))
    .map( |m| m.as_str())
    .and_then( |p| FNAME_RE.captures( p))
    .and_then( |cap| cap.get(1))
    .map( |m| m.as_str())
}

/// Note - we assume lower case extension without '.'
pub fn mime_type_for_extension (ext: &str)->Option<&'static str> {
    MIME_MAP.get(ext).map(|v| &**v)
}

lazy_static! {
    static ref MIME_MAP: HashMap<&'static str, &'static str> = HashMap::from( [ // file extension -> mime type
        //-- well known raster drivers
        ("tif", "image/tiff"),
        ("tiff", "image/tiff"),
        ("png", "image/png"),
        ("jpg", "image/jpeg"),
        ("jpeg", "image/jpeg"),
        ("svg", "image/svg+xml"),
        ("webp", "image/webp"),
        ("webm", "video/webm"),
        ("mpeg", "video/mpeg"),
        ("mp3", "audio/mp3"),
        ("mp4", "video/mp4"),
        ("js", "text/javascript"),
        ("mjs", "text/javascript"),
        ("json", "application/json"),
        ("jsonld", "application/ld+json"),
        ("pdf", "application/pdf"),
        ("csv", "text/csv"),
        // and many more to follow..
    ]);
}