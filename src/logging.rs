use slog::{Drain, Logger, o};
use slog_term::{TermDecorator, CompactFormat};
use slog_async::Async;

pub fn setup_logger(node_id: u64, role: &str) -> Logger {
    let decorator = TermDecorator::new().build();
    let drain = CompactFormat::new(decorator).build().fuse();
    let drain = Async::new(drain).build().fuse();

    Logger::root(
        drain,
        o!(
            "node_id" => node_id,
            "role" => role.to_string(),
            "version" => env!("CARGO_PKG_VERSION"),
        ),
    )
}

pub fn get_metrics_logger() -> Logger {
    let decorator = TermDecorator::new().build();
    let drain = CompactFormat::new(decorator).build().fuse();
    let drain = Async::new(drain).build().fuse();

    Logger::root(
        drain,
        o!(
            "component" => "metrics",
            "version" => env!("CARGO_PKG_VERSION"),
        ),
    )
}

use tracing_subscriber::{
    fmt,
    EnvFilter,
    layer::SubscriberExt,
    util::SubscriberInitExt,
};
use tracing::Level;

/// Sets up the logging subscriber for the application.
/// 
/// # Arguments
/// * `node_id` - Unique identifier for the node (unused for now but kept for future use)
/// * `node_type` - Type of node (control/worker)
pub fn init_logger(_node_id: u64, node_type: &str) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new(format!("{}={}", node_type, Level::INFO))
        });

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_thread_names(true)
        .with_level(true)
        .with_ansi(true)
        .compact();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .try_init()
        .expect("Failed to initialize logger");
}
