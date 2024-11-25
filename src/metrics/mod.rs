use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use prometheus::{Registry, Gauge, HistogramVec, HistogramOpts, IntCounter};
use lazy_static::lazy_static;
use duckdb::{Connection, params};
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
    db: Arc<Mutex<Connection>>,
}

impl MetricsRegistry {
    pub fn new() -> Result<Self> {
        // Register prometheus metrics
        REGISTRY.register(Box::new(REQUEST_COUNTER.clone())).unwrap();
        REGISTRY.register(Box::new(ACTIVE_CONNECTIONS.clone())).unwrap();
        REGISTRY.register(Box::new(REQUEST_DURATION.clone())).unwrap();
        REGISTRY.register(Box::new(RAFT_CONSENSUS_LATENCY.clone())).unwrap();

        // Initialize DuckDB
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS metrics (
                name VARCHAR NOT NULL,
                value DOUBLE NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS metric_aggregates (
                name VARCHAR PRIMARY KEY,
                count BIGINT NOT NULL,
                sum DOUBLE NOT NULL,
                average DOUBLE NOT NULL,
                min DOUBLE NOT NULL,
                max DOUBLE NOT NULL,
                last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
        ")?;

        Ok(Self {
            metrics: Arc::new(AsyncRwLock::new(HashMap::new())),
            aggregates: Arc::new(AsyncRwLock::new(HashMap::new())),
            db: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn record_metric(&self, name: &str, value: f64) -> Result<()> {
        // Update in-memory state
        let mut metrics = self.metrics.write().await;
        metrics.insert(name.to_string(), value);
        
        // Update aggregate in memory
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

        // Persist to DuckDB
        let mut conn = self.db.lock().await;
        conn.execute(
            "INSERT INTO metrics (name, value) VALUES (?1, ?2)",
            params![name, value],
        )?;

        conn.execute(
            "INSERT INTO metric_aggregates (name, count, sum, average, min, max)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(name) DO UPDATE SET
                count = excluded.count,
                sum = excluded.sum,
                average = excluded.average,
                min = excluded.min,
                max = excluded.max,
                last_updated = CURRENT_TIMESTAMP",
            params![
                name,
                aggregate.count,
                aggregate.sum,
                aggregate.average,
                aggregate.min,
                aggregate.max
            ],
        )?;

        Ok(())
    }

    pub async fn get_metric(&self, name: &str) -> Result<Option<f64>> {
        // Try memory first
        let metrics = self.metrics.read().await;
        if let Some(&value) = metrics.get(name) {
            return Ok(Some(value));
        }

        // Fall back to DB
        let conn = self.db.lock().await;
        let mut stmt = conn.prepare(
            "SELECT value FROM metrics WHERE name = ?1 ORDER BY timestamp DESC LIMIT 1"
        )?;
        let mut rows = stmt.query(params![name])?;
        
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub async fn get_metric_aggregate(&self, name: &str) -> Result<Option<MetricAggregate>> {
        // Try memory first
        let aggregates = self.aggregates.read().await;
        if let Some(agg) = aggregates.get(name) {
            return Ok(Some(agg.clone()));
        }

        // Fall back to DB
        let conn = self.db.lock().await;
        let mut stmt = conn.prepare(
            "SELECT count, sum, average, min, max 
             FROM metric_aggregates 
             WHERE name = ?1"
        )?;
        let mut rows = stmt.query(params![name])?;
        
        if let Some(row) = rows.next()? {
            Ok(Some(MetricAggregate {
                count: row.get(0)?,
                sum: row.get(1)?,
                average: row.get(2)?,
                min: row.get(3)?,
                max: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_all_metrics(&self) -> Result<HashMap<String, f64>> {
        let mut metrics = HashMap::new();
        
        // Get from memory
        let mem_metrics = self.metrics.read().await;
        metrics.extend(mem_metrics.clone());

        // Get from DB
        let conn = self.db.lock().await;
        let mut stmt = conn.prepare(
            "SELECT DISTINCT ON (name) name, value 
             FROM metrics 
             ORDER BY name, timestamp DESC"
        )?;
        let rows = stmt.query_map(params![], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

        for row in rows {
            let (name, value): (String, f64) = row?;
            metrics.entry(name).or_insert(value);
        }

        Ok(metrics)
    }

    pub async fn get_all_aggregates(&self) -> Result<HashMap<String, MetricAggregate>> {
        let mut aggregates = HashMap::new();
        
        // Get from memory
        let mem_aggregates = self.aggregates.read().await;
        aggregates.extend(mem_aggregates.clone());

        // Get from DB
        let conn = self.db.lock().await;
        let mut stmt = conn.prepare(
            "SELECT name, count, sum, average, min, max 
             FROM metric_aggregates"
        )?;
        let rows = stmt.query_map(params![], |row| {
            Ok((
                row.get(0)?,
                MetricAggregate {
                    count: row.get(1)?,
                    sum: row.get(2)?,
                    average: row.get(3)?,
                    min: row.get(4)?,
                    max: row.get(5)?,
                }
            ))
        })?;

        for row in rows {
            let (name, agg) = row?;
            aggregates.entry(name).or_insert(agg);
        }

        Ok(aggregates)
    }

    pub async fn apply_raft_entry(&self, data: &[u8]) -> Result<()> {
        let operation: MetricOperation = serde_json::from_slice(data)
            .map_err(|e| RaftMetricsError::Internal(format!("Failed to deserialize metric: {}", e)))?;
        
        match operation {
            MetricOperation::Record { name, value } => {
                self.record_metric(&name, value).await?;
            }
            MetricOperation::Delete { name } => {
                let mut metrics = self.metrics.write().await;
                metrics.remove(&name);
                let mut aggregates = self.aggregates.write().await;
                aggregates.remove(&name);
                
                let mut conn = self.db.lock().await;
                conn.execute("DELETE FROM metrics WHERE name = ?1", params![name])?;
                conn.execute("DELETE FROM metric_aggregates WHERE name = ?1", params![name])?;
            }
        }
        
        Ok(())
    }

    pub fn serialize_operation(operation: &MetricOperation) -> Result<Vec<u8>> {
        serde_json::to_vec(operation)
            .map_err(|e| RaftMetricsError::Internal(format!("Failed to serialize operation: {}", e)))
    }

    pub fn deserialize_operation(data: &[u8]) -> Result<MetricOperation> {
        serde_json::from_slice(data)
            .map_err(|e| RaftMetricsError::Internal(format!("Failed to deserialize operation: {}", e)))
    }
}
