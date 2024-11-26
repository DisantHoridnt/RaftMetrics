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
    RecordMetricRequest,
    RecordMetricResponse,
    GetMetricRequest,
    GetMetricResponse,
    GetMetricAggregateRequest,
    GetMetricAggregateResponse,
};

// Re-export service types
pub use proto::metrics::metrics_service_client::MetricsServiceClient;
pub use proto::metrics::metrics_service_server::{MetricsService, MetricsServiceServer};
