use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use prometheus::{Registry, Gauge, HistogramVec, HistogramOpts, IntCounter, opts};
use lazy_static::lazy_static;
use tokio::sync::RwLock as AsyncRwLock;
use crate::Result;

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
}

#[derive(Debug, Clone)]
pub struct MetricsRegistry {
    metrics: Arc<AsyncRwLock<HashMap<String, f64>>>,
    aggregates: Arc<AsyncRwLock<HashMap<String, f64>>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
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
        let current_aggregate = aggregates.entry(name.to_string()).or_insert(0.0);
        *current_aggregate += value;
        
        Ok(())
    }

    pub async fn get_metric(&self, name: &str) -> Result<Option<f64>> {
        let metrics = self.metrics.read().await;
        Ok(metrics.get(name).copied())
    }

    pub async fn get_metric_aggregate(&self, name: &str) -> Result<Option<f64>> {
        let aggregates = self.aggregates.read().await;
        Ok(aggregates.get(name).copied())
    }

    pub async fn get_all_metrics(&self) -> Result<HashMap<String, f64>> {
        Ok(self.metrics.read().await.clone())
    }

    pub async fn get_all_aggregates(&self) -> Result<HashMap<String, f64>> {
        Ok(self.aggregates.read().await.clone())
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}
