use axum::{
    extract::State,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::{
    Result,
    RaftMetricsError,
    metrics::MetricsRegistry,
    raft::storage::MemStorage,
    partitioning::get_partition,
};

#[derive(Clone)]
pub struct ControlState {
    pub storage: Arc<MemStorage>,
    pub metrics: Arc<MetricsRegistry>,
    pub worker_urls: Arc<Vec<String>>,
    pub http_client: Arc<reqwest::Client>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricRequest {
    pub metric_name: String,
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricResponse {
    pub success: bool,
    pub message: String,
}

pub fn control_router(state: ControlState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", post(record_metric))
        .route("/metrics/:name", get(get_metric))
        .route("/metrics/:name/aggregate", get(get_metric_aggregate))
        .with_state(state)
}

async fn health_check() -> impl axum::response::IntoResponse {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "message": "Control node is operational"
    }))
}

async fn record_metric(
    State(state): State<ControlState>,
    axum::Json(request): axum::Json<MetricRequest>,
) -> Result<impl axum::response::IntoResponse, RaftMetricsError> {
    info!("Recording metric: {} = {}", request.metric_name, request.value);
    
    let worker_count = state.worker_urls.len();
    info!("Total workers: {}", worker_count);
    
    let partition = get_partition(&request.metric_name, worker_count);
    info!("Selected partition {} for metric {}", partition, request.metric_name);
    
    let worker_url = &state.worker_urls[partition];
    info!("Routing metric {} to worker {}", request.metric_name, worker_url);

    let response = state.http_client.post(&format!("{}/process", worker_url))
        .json(&request)
        .send()
        .await
        .map_err(|e| RaftMetricsError::Internal(format!("Failed to forward metric: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response.text().await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(RaftMetricsError::Internal(format!("Worker failed to process metric: {}", error_text)));
    }

    Ok(axum::Json(MetricResponse {
        success: true,
        message: format!("Metric recorded on worker {}", partition + 1),
    }))
}

async fn get_metric(
    State(state): State<ControlState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<impl axum::response::IntoResponse, RaftMetricsError> {
    info!("Retrieving metric: {}", name);
    
    let worker_count = state.worker_urls.len();
    let partition = get_partition(&name, worker_count);
    let worker_url = &state.worker_urls[partition];
    
    info!("Fetching metric {} from worker {} (partition {})", name, worker_url, partition);
    
    let response = state.http_client.get(&format!("{}/metrics/{}", worker_url, name))
        .send()
        .await
        .map_err(|e| RaftMetricsError::Internal(format!("Failed to fetch metric: {}", e)))?;
        
    if !response.status().is_success() {
        return Err(RaftMetricsError::NotFound);
    }
    
    let metric_response: crate::api::worker::MetricResponse = response.json().await
        .map_err(|e| RaftMetricsError::Internal(format!("Failed to parse response: {}", e)))?;
    
    Ok(axum::Json(metric_response))
}

async fn get_metric_aggregate(
    State(state): State<ControlState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<impl axum::response::IntoResponse, RaftMetricsError> {
    info!("Calculating aggregate for metric: {}", name);
    
    let worker_count = state.worker_urls.len();
    let partition = get_partition(&name, worker_count);
    let worker_url = &state.worker_urls[partition];
    
    info!("Fetching aggregate for metric {} from worker {} (partition {})", name, worker_url, partition);
    
    let response = state.http_client.get(&format!("{}/metrics/{}/aggregate", worker_url, name))
        .send()
        .await
        .map_err(|e| RaftMetricsError::Internal(format!("Failed to fetch aggregate: {}", e)))?;
        
    if !response.status().is_success() {
        return Err(RaftMetricsError::NotFound);
    }
    
    let aggregate_response: crate::api::worker::AggregateResponse = response.json().await
        .map_err(|e| RaftMetricsError::Internal(format!("Failed to parse response: {}", e)))?;
    
    Ok(axum::Json(aggregate_response))
}

pub async fn start_control_node() {
    let storage = Arc::new(MemStorage::new());
    let metrics = Arc::new(MetricsRegistry::new());

    // Parse worker URLs from environment variable
    let worker_urls: Vec<String> = std::env::var("WORKER_HOSTS")
        .unwrap_or_else(|_| "http://localhost:8081".to_string())
        .split(',')
        .map(|host| {
            if host.starts_with("http://") {
                host.to_string()
            } else {
                format!("http://{}", host)
            }
        })
        .collect();

    info!("Configured worker URLs: {:?}", worker_urls);

    let state = ControlState {
        storage: storage.clone(),
        metrics: metrics.clone(),
        worker_urls: Arc::new(worker_urls),
        http_client: Arc::new(reqwest::Client::new()),
    };

    let app = Router::new()
        .route("/metrics", post(record_metric))
        .route("/metrics/:name", get(get_metric))
        .route("/metrics/:name/aggregate", get(get_metric_aggregate))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    info!("Starting control node on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
