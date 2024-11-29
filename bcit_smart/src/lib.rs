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
/// ```rust
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
