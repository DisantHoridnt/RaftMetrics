use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricData {
    pub timestamp: i64,
    pub value: f64,
    pub labels: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsertData {
    pub metric_name: String,
    pub value: f64,
    pub timestamp: i64,
    pub tags: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComputeResponse {
    pub result: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricQuery {
    pub metric_name: String,
    pub start_time: i64,
    pub end_time: i64,
    pub aggregation: String,
}
