use std::collections::HashMap;
use std::sync::Arc;
use prometheus::{Registry, Gauge, HistogramVec, HistogramOpts, IntCounter};
use lazy_static::lazy_static;
use tokio::sync::RwLock as AsyncRwLock;
use serde::{Serialize, Deserialize};
use crate::Result;
use crate::RaftMetricsError;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricAggregate {
    pub count: u64,
    pub sum: f64,
    pub average: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MetricOperation {
    Record { name: String, value: f64 },
    Delete { name: String },
}

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref REQUEST_COUNTER: IntCounter =
        IntCounter::new("requests_total", "Total number of requests received").unwrap();
    pub static ref ACTIVE_CONNECTIONS: Gauge =
        Gauge::new("active_connections", "Number of active connections").unwrap();
    pub static ref REQUEST_DURATION: HistogramVec =
        HistogramVec::new(
            HistogramOpts::new("request_duration_seconds", "Request duration in seconds"),
            &["endpoint"]
        ).unwrap();
    pub static ref METRIC_OPS_TOTAL: IntCounter =
        IntCounter::new("metric_operations_total", "Total number of metric operations processed").unwrap();
    pub static ref RAFT_CONSENSUS_LATENCY: HistogramVec =
        HistogramVec::new(
            HistogramOpts::new("raft_consensus_latency_seconds", "Time taken to reach consensus"),
            &["operation"]
        ).unwrap();
}

#[derive(Debug, Clone)]
pub struct MetricsRegistry {
    metrics: Arc<AsyncRwLock<HashMap<String, f64>>>,
    aggregates: Arc<AsyncRwLock<HashMap<String, MetricAggregate>>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        // Register prometheus metrics
        REGISTRY.register(Box::new(REQUEST_COUNTER.clone())).unwrap();
        REGISTRY.register(Box::new(ACTIVE_CONNECTIONS.clone())).unwrap();
        REGISTRY.register(Box::new(REQUEST_DURATION.clone())).unwrap();
        REGISTRY.register(Box::new(METRIC_OPS_TOTAL.clone())).unwrap();
        REGISTRY.register(Box::new(RAFT_CONSENSUS_LATENCY.clone())).unwrap();

        Self {
            metrics: Arc::new(AsyncRwLock::new(HashMap::new())),
            aggregates: Arc::new(AsyncRwLock::new(HashMap::new())),
        }
    }

    pub async fn record_metric(&self, name: &str, value: f64) -> Result<()> {
        let mut metrics = self.metrics.write().await;
        metrics.insert(name.to_string(), value);
        
        // Update aggregate
        let mut aggregates = self.aggregates.write().await;
        let aggregate = aggregates.entry(name.to_string()).or_insert_with(|| MetricAggregate {
            count: 0,
            sum: 0.0,
            average: 0.0,
            min: f64::MAX,
            max: f64::MIN,
        });

        aggregate.count += 1;
        aggregate.sum += value;
        aggregate.average = aggregate.sum / aggregate.count as f64;
        aggregate.min = aggregate.min.min(value);
        aggregate.max = aggregate.max.max(value);

        METRIC_OPS_TOTAL.inc();
        Ok(())
    }

    pub async fn get_metric(&self, name: &str) -> Result<Option<f64>> {
        let metrics = self.metrics.read().await;
        Ok(metrics.get(name).copied())
    }

    pub async fn get_metric_aggregate(&self, name: &str) -> Result<Option<MetricAggregate>> {
        let aggregates = self.aggregates.read().await;
        Ok(aggregates.get(name).cloned())
    }

    pub async fn get_all_metrics(&self) -> Result<HashMap<String, f64>> {
        let metrics = self.metrics.read().await;
        Ok(metrics.clone())
    }

    pub async fn get_all_aggregates(&self) -> Result<HashMap<String, MetricAggregate>> {
        let aggregates = self.aggregates.read().await;
        Ok(aggregates.clone())
    }

    // New methods for Raft integration
    pub async fn apply_raft_operation(&self, operation: MetricOperation) -> Result<()> {
        let timer = RAFT_CONSENSUS_LATENCY.with_label_values(&["apply"]).start_timer();
        
        match operation {
            MetricOperation::Record { name, value } => {
                self.record_metric(&name, value).await?;
            }
            MetricOperation::Delete { name } => {
                let mut metrics = self.metrics.write().await;
                metrics.remove(&name);
                let mut aggregates = self.aggregates.write().await;
                aggregates.remove(&name);
            }
        }

        timer.observe_duration();
        Ok(())
    }

    pub fn serialize_operation(operation: &MetricOperation) -> Result<Vec<u8>> {
        serde_json::to_vec(operation).map_err(|e| RaftMetricsError::SerdeJson(e))
    }

    pub fn deserialize_operation(data: &[u8]) -> Result<MetricOperation> {
        serde_json::from_slice(data).map_err(|e| RaftMetricsError::SerdeJson(e))
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}
