// Include the generated protobuf code
pub mod metrics {
    tonic::include_proto!("raftmetrics.v1");
}

pub use self::metrics::*;

// Re-export commonly used types
pub use metrics::{
    MetricEntry,
    MetricBatch,
    MetricResponse,
    RaftMessage,
    metrics_service_client::MetricsServiceClient,
    metrics_service_server::{MetricsService, MetricsServiceServer},
};
