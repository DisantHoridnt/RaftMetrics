pub mod api;
pub mod raft;
pub mod proto;
pub mod error;
pub mod metrics;
pub mod partitioning;
pub mod logging;

pub use error::{Result, RaftMetricsError};

// Re-export commonly used types
pub use proto::{
    MetricEntry,
    MetricBatch,
    MetricResponse,
    RaftMessage,
    metrics_service_client::MetricsServiceClient,
    metrics_service_server::{MetricsService, MetricsServiceServer},
};
