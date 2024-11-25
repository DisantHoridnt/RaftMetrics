use lazy_static::lazy_static;
use prometheus::{
    Counter, CounterVec, Gauge, Histogram, HistogramVec, IntGauge, Registry,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

use crate::RaftMetricsError;

lazy_static! {
    // Node status metrics
    pub static ref NODE_STATUS: IntGauge = IntGauge::new(
        "raftmetrics_node_status",
        "Current status of the node (0: offline, 1: online)"
    ).unwrap();

    // Leader election metrics
    pub static ref LEADER_ELECTIONS_TOTAL: Counter = Counter::new(
        "raftmetrics_leader_elections_total",
        "Total number of leader elections participated in"
    ).unwrap();

    pub static ref LEADER_CHANGES_TOTAL: Counter = Counter::new(
        "raftmetrics_leader_changes_total",
        "Total number of leader changes"
    ).unwrap();

    // Request metrics
    pub static ref REQUEST_DURATION_SECONDS: HistogramVec = HistogramVec::new(
        prometheus::HistogramOpts::new(
            "raftmetrics_request_duration_seconds",
            "Request duration in seconds"
        ),
        &["endpoint"]
    ).unwrap();

    pub static ref REQUEST_TOTAL: CounterVec = CounterVec::new(
        prometheus::Opts::new(
            "raftmetrics_requests_total",
            "Total number of requests"
        ),
        &["endpoint", "status"]
    ).unwrap();

    // Storage metrics
    pub static ref STORAGE_OPERATIONS_TOTAL: CounterVec = CounterVec::new(
        prometheus::Opts::new(
            "raftmetrics_storage_operations_total",
            "Total number of storage operations"
        ),
        &["operation", "status"]
    ).unwrap();

    pub static ref STORAGE_SIZE_BYTES: Gauge = Gauge::new(
        "raftmetrics_storage_size_bytes",
        "Current storage size in bytes"
    ).unwrap();

    // Raft metrics
    pub static ref RAFT_PROPOSALS_TOTAL: CounterVec = CounterVec::new(
        prometheus::Opts::new(
            "raftmetrics_raft_proposals_total",
            "Total number of proposals"
        ),
        &["status"]
    ).unwrap();

    pub static ref RAFT_PROPOSAL_DURATION_SECONDS: Histogram = Histogram::with_opts(
        prometheus::HistogramOpts::new(
            "raftmetrics_raft_proposal_duration_seconds",
            "Proposal duration in seconds"
        )
    ).unwrap();

    static ref REGISTRY: Registry = Registry::new();
    static ref REQUEST_PROCESSING_TIME: Histogram = Histogram::with_opts(
        prometheus::HistogramOpts::new(
            "request_processing_time",
            "Time spent processing requests"
        )
    ).unwrap();
}

#[derive(Clone)]
pub struct MetricsRegistry {
    registry: Arc<Registry>,
    metrics: Arc<RwLock<HashMap<String, f64>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateResult {
    pub count: usize,
    pub sum: f64,
    pub average: f64,
    pub min: f64,
    pub max: f64,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        let registry = Arc::new(Registry::new());
        
        // Register all metrics
        registry.register(Box::new(NODE_STATUS.clone())).unwrap();
        registry.register(Box::new(LEADER_ELECTIONS_TOTAL.clone())).unwrap();
        registry.register(Box::new(LEADER_CHANGES_TOTAL.clone())).unwrap();
        registry.register(Box::new(REQUEST_DURATION_SECONDS.clone())).unwrap();
        registry.register(Box::new(REQUEST_TOTAL.clone())).unwrap();
        registry.register(Box::new(STORAGE_OPERATIONS_TOTAL.clone())).unwrap();
        registry.register(Box::new(STORAGE_SIZE_BYTES.clone())).unwrap();
        registry.register(Box::new(RAFT_PROPOSALS_TOTAL.clone())).unwrap();
        registry.register(Box::new(RAFT_PROPOSAL_DURATION_SECONDS.clone())).unwrap();

        Self {
            registry,
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_metric(&self, name: &str, value: f64) -> Result<(), RaftMetricsError> {
        let mut metrics = self.metrics.write().await;
        metrics.insert(name.to_string(), value);
        
        STORAGE_OPERATIONS_TOTAL
            .with_label_values(&["write", "success"])
            .inc();
            
        Ok(())
    }

    pub async fn record_metric_with_timestamp(
        &self,
        name: &str,
        value: f64,
        timestamp: i64,
    ) -> Result<(), RaftMetricsError> {
        let mut metrics = self.metrics.write().await;
        metrics.insert(name.to_string(), value);
        
        STORAGE_OPERATIONS_TOTAL
            .with_label_values(&["write", "success"])
            .inc();
            
        Ok(())
    }

    pub async fn get_metric(&self, name: &str) -> Result<f64, RaftMetricsError> {
        let metrics = self.metrics.read().await;
        metrics
            .get(name)
            .copied()
            .ok_or(RaftMetricsError::NotFound)
    }

    pub async fn get_metrics_aggregate(&self) -> Result<AggregateResult, RaftMetricsError> {
        let metrics = self.metrics.read().await;
        
        if metrics.is_empty() {
            return Err(RaftMetricsError::NotFound);
        }
        
        let values: Vec<f64> = metrics.values().copied().collect();
        let count = values.len();
        let sum: f64 = values.iter().sum();
        let average = sum / count as f64;
        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        Ok(AggregateResult {
            count,
            sum,
            average,
            min,
            max,
        })
    }

    pub fn get_registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }
}

pub fn init_metrics() {
    REGISTRY.register(Box::new(REQUEST_PROCESSING_TIME.clone())).unwrap();
}
