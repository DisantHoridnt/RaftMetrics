// Include the generated protobuf code
pub mod metrics {
    tonic::include_proto!("raftmetrics.v1");
}

// Re-export commonly used types from the generated code
pub use self::metrics::{
    MetricEntry,
    MetricBatch,
    MetricResponse,
    RaftMessage,
};

// Re-export service types
pub use self::metrics::metrics_service_client::MetricsServiceClient;
pub use self::metrics::metrics_service_server::{MetricsService, MetricsServiceServer};
