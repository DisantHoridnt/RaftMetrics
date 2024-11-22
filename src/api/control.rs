use axum::{
    Router,
    routing::{post, get},
    extract::Json,
    response::IntoResponse,
    http::StatusCode,
};
use tokio::sync::Mutex;
use std::{sync::Arc, env, collections::HashMap, time::Instant};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use slog::{Logger, info, warn, error};

use crate::{
    hash_partition,
    metrics::{self, RequestTimer},
    Error, Result,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricData {
    key: String,
    value: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AggregateResponse {
    total_average: f64,
    total_count: i64,
}

pub struct ControlNode {
    id: u64,
    worker_hosts: Vec<String>,
    http_client: Client,
    logger: Logger,
}

impl ControlNode {
    pub fn new(id: u64, logger: Logger) -> Self {
        let worker_hosts = env::var("WORKER_HOSTS")
            .unwrap_or_else(|_| "http://worker1:8081,http://worker2:8082".to_string())
            .split(',')
            .map(String::from)
            .collect();

        info!(logger, "Initializing control node";
            "node_id" => id,
            "worker_hosts" => format!("{:?}", worker_hosts)
        );

        metrics::init_metrics();

        Self {
            id,
            worker_hosts,
            http_client: Client::new(),
            logger,
        }
    }

    pub async fn distribute_data(&self, data: MetricData) -> Result<()> {
        let _timer = RequestTimer::new();
        let start = Instant::now();

        let worker_idx = hash_partition(&data.key, self.worker_hosts.len());
        let worker_url = format!("{}/insert", self.worker_hosts[worker_idx]);
        
        info!(self.logger, "Distributing data";
            "key" => &data.key,
            "worker_url" => &worker_url
        );

        let response = self.http_client
            .post(&worker_url)
            .json(&data)
            .send()
            .await
            .map_err(|e| Error::InvalidRequest(e.to_string()))?;

        if !response.status().is_success() {
            let error_msg = format!("Worker request failed with status: {}", response.status());
            error!(self.logger, "Distribution failed"; "error" => &error_msg);
            metrics::record_proposal_failure();
            return Err(Error::InvalidRequest(error_msg));
        }

        metrics::record_proposal();
        metrics::record_raft_rtt(start.elapsed().as_secs_f64());
        
        Ok(())
    }

    pub async fn aggregate_metrics(&self) -> Result<AggregateResponse> {
        let _timer = RequestTimer::new();
        
        info!(self.logger, "Aggregating metrics from workers");

        let mut total_sum = 0.0;
        let mut total_count = 0;

        for host in &self.worker_hosts {
            let worker_url = format!("{}/compute", host);
            
            match self.http_client
                .post(&worker_url)
                .send()
                .await
                .map_err(|e| Error::InvalidRequest(e.to_string()))?
                .json::<HashMap<String, f64>>()
                .await
            {
                Ok(response) => {
                    if let (Some(&avg), Some(&count)) = (response.get("average"), response.get("count")) {
                        total_sum += avg * count;
                        total_count += count as i64;
                    }
                }
                Err(e) => {
                    warn!(self.logger, "Failed to get metrics from worker"; 
                        "worker_url" => worker_url,
                        "error" => %e
                    );
                }
            }
        }

        let total_average = if total_count > 0 {
            total_sum / total_count as f64
        } else {
            0.0
        };

        let response = AggregateResponse {
            total_average,
            total_count,
        };

        info!(self.logger, "Aggregation complete";
            "total_average" => response.total_average,
            "total_count" => response.total_count
        );

        Ok(response)
    }

    pub async fn check_worker_health(&self) -> Result<()> {
        for host in &self.worker_hosts {
            let health_url = format!("{}/health", host);
            match self.http_client.get(&health_url).send().await {
                Ok(response) if response.status().is_success() => {
                    info!(self.logger, "Worker healthy"; "worker_url" => host);
                }
                Ok(response) => {
                    warn!(self.logger, "Worker unhealthy";
                        "worker_url" => host,
                        "status" => response.status().as_u16()
                    );
                }
                Err(e) => {
                    error!(self.logger, "Worker health check failed";
                        "worker_url" => host,
                        "error" => %e
                    );
                }
            }
        }
        Ok(())
    }
}

pub async fn start_control_node(logger: Logger) {
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);

    let node_id = env::var("NODE_ID")
        .unwrap_or_else(|_| "1".to_string())
        .parse()
        .unwrap_or(1);

    let node = Arc::new(Mutex::new(ControlNode::new(node_id, logger.clone())));

    let app = Router::new()
        .route("/metrics", post(insert_metric))
        .route("/aggregate", get(aggregate_metrics))
        .route("/health", get(health_check))
        .with_state(node);

    info!(logger, "Control node starting"; "port" => port);
    
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn insert_metric(
    State(node): axum::extract::State<Arc<Mutex<ControlNode>>>,
    Json(data): Json<MetricData>,
) -> impl IntoResponse {
    let node = node.lock().await;
    match node.distribute_data(data).await {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            error!(node.logger, "Metric insertion failed"; "error" => %e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn aggregate_metrics(
    State(node): axum::extract::State<Arc<Mutex<ControlNode>>>,
) -> impl IntoResponse {
    let node = node.lock().await;
    match node.aggregate_metrics().await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => {
            error!(node.logger, "Metrics aggregation failed"; "error" => %e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn health_check(
    State(node): axum::extract::State<Arc<Mutex<ControlNode>>>,
) -> impl IntoResponse {
    let node = node.lock().await;
    match node.check_worker_health().await {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            error!(node.logger, "Health check failed"; "error" => %e);
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}
