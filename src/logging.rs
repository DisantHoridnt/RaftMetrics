use slog::{Drain, Logger, o};
use slog_term::{TermDecorator, CompactFormat};
use slog_async::Async;
use std::sync::Arc;

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

use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

pub fn init() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .pretty()
        .try_init();

    match subscriber {
        Ok(_) => info!("Logger initialized successfully"),
        Err(e) => eprintln!("Failed to initialize logger: {}", e),
    }
}
