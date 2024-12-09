use odin_actor::prelude::*;
use odin_build;
use odin_build::prelude::*;
use odin_server::prelude::*;

pub mod errors;
pub use errors::*;

pub mod web;
pub use web::*;

pub mod actor;
pub use actor::*;

pub mod live_importer;
pub use live_importer::*;

pub mod oasis_actor;
pub use oasis_actor::*;

pub mod oasis_web;
pub use oasis_web::*;

use tracing::Level;
use tracing_subscriber::fmt::format;
use tracing_subscriber::EnvFilter;

use serde::{Serialize,Deserialize};

use chrono::{DateTime, Utc}; // for timestamps
use serde_json::Value;
use sqlx::{Postgres, FromRow};
use std::error::Error as StdError;

define_load_config! {}
define_load_asset! {}

const MAX_DEBUG_MESSAGE_LENGTH: usize = 1000;

fn truncate_message(msg: &str) -> String {
    if msg.len() > MAX_DEBUG_MESSAGE_LENGTH {
        format!("{}...", &msg[..MAX_DEBUG_MESSAGE_LENGTH])
    } else {
        msg.to_string()
    }
}

/// Creates a customized tracing subscriber with an optional maximum log level.
///
/// - If `max_level` is specified, the subscriber will filter logs up to that level.
/// - If `max_level` is `None`, the subscriber will use the `RUST_LOG` environment variable
///   to determine the logging level.
/// - The log format is compact and includes truncated `message` fields if they exceed a certain length.
///
/// # Arguments
///
/// - `max_level`: An optional `tracing::Level` to set the maximum log level.
///
/// # Usage
///
/// ```no_run
/// use bcit_smart::create_customized_tracing_subscriber;
/// use tracing::Level;
/// create_customized_tracing_subscriber(Some(Level::DEBUG)); // Explicit max level
/// create_customized_tracing_subscriber(None); // Use RUST_LOG environment variable
/// ```
pub fn create_customized_tracing_subscriber(max_level: Option<Level>) {
    let custom_fmt = tracing_subscriber::fmt()
        .event_format(format().compact())
        .fmt_fields(format::debug_fn(|writer, field, value| {
            if field.name() == "message" {
                let truncated = truncate_message(&format!("{:?}", value));
                write!(writer, "{}", truncated)
            } else {
                write!(writer, "{}: {:?}", field.name(), value)
            }
        }));

    if let Some(level) = max_level {
        custom_fmt
            .with_max_level(level)
            .init(); // Explicit max level
    } else {
        custom_fmt
            .with_env_filter(EnvFilter::from_default_env())
            .init(); // Use RUST_LOG environment variable
    }
}


// Awesense API Area

/// Configuration struct for the Awesense API.
#[derive(Debug,Serialize,Deserialize)]
pub struct AwesenseApiInfo {
    pub api_key: String,
    pub encrypted_user_credentials: String,
    pub base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AwesenseGrid {
    pub active: bool,
    pub description: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AwesenseSqlInfo {
    pub user: String,
    pub password: String,
    pub host: String,
    pub port: u32,
    pub db_name: String,
}


#[derive(Debug, FromRow)]
pub struct Grid {
    grid_id: String,
    description: String,
    last_updated: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
pub struct GridElement {
    pub grid_id: Option<String>,
    pub grid_element_id: Option<String>,
    pub type_: Option<String>, // Use `type_` to avoid conflict with the `type` keyword in Rust
    pub customer_type: Option<String>,
    pub phases: Option<String>,
    pub is_underground: Option<bool>,
    pub is_producer: Option<bool>,
    pub is_consumer: Option<bool>,
    pub is_switchable: Option<bool>,
    pub switch_is_open: Option<bool>,
    pub terminal1_cn: Option<String>,
    pub terminal2_cn: Option<String>,
    pub power_flow_direction: Option<String>,
    pub upstream_grid_element_id: Option<String>,
    pub geometry: Option<GeometryType>, // Custom type for handling geometry
    pub meta: Option<Value>, // JSONB field is handled as serde_json::Value
}

// Define a custom enum for handling geometry
#[derive(Debug)]
pub enum GeometryType {
    Point(f64, f64),
    LineString(Vec<(f64, f64)>),
}

impl sqlx::Type<sqlx::Postgres> for GeometryType {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("TEXT")
    }
}

impl<'r> sqlx::Decode<'r, Postgres> for GeometryType {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> std::result::Result<Self, Box<dyn StdError + Send + Sync>> {
        // Decode the WKT as a string
        let wkt: String = sqlx::Decode::<Postgres>::decode(value)?;

        if wkt.starts_with("POINT") {
            // Parse POINT
            let coordinates = wkt.trim_start_matches("POINT(").trim_end_matches(')');
            let coords: Vec<f64> = coordinates
                .split_whitespace()
                .map(|s| s.parse::<f64>().map_err(|e| e.into()))
                .collect::<Result<_>>()?;
            if coords.len() == 2 {
                Ok(GeometryType::Point(coords[0], coords[1]))
            } else {
                Err(misc_error("Invalid POINT geometry").into())
            }
        } else if wkt.starts_with("LINESTRING") {
            // Parse LINESTRING
            let coordinates = wkt.trim_start_matches("LINESTRING(").trim_end_matches(')');
            let points: Vec<(f64, f64)> = coordinates
                .split(',')
                .map(|coord| {
                    let coords: Vec<f64> = coord
                        .split_whitespace()
                        .map(|s| s.parse::<f64>().map_err(|e| e.into()))
                        .collect::<Result<_>>()?;
                    if coords.len() == 2 {
                        Ok((coords[0], coords[1]))
                    } else {
                        Err(misc_error("Invalid coordinate in LINESTRING").into())
                    }
                })
                .collect::<Result<_>>()?;
            Ok(GeometryType::LineString(points))
        } else {
            Err(misc_error("Unsupported geometry type").into())
        }
    }
}