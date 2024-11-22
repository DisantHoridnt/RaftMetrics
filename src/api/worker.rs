use axum::{
    Router,
    routing::{post, get},
    extract::{Extension, Json, State},
    response::IntoResponse,
    http::StatusCode,
};
use tokio::sync::Mutex;
use duckdb::{Connection, Result as DuckResult};
use std::{sync::Arc, env};
use serde::{Deserialize, Serialize};
use slog::{Logger, info, warn, error};

use crate::{
    metrics::{self, RequestTimer},
    Error, Result,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct InsertData {
    partition_key: String,
    value: f64,
}

#[derive(Debug, Serialize)]
pub struct ComputeResponse {
    average: f64,
    count: i64,
}

pub struct WorkerNode {
    db: Connection,
    node_id: u32,
    logger: Logger,
}

impl WorkerNode {
    pub fn new(logger: Logger) -> Result<Self> {
        let db_path = env::var("DB_PATH").unwrap_or_else(|_| "/data/worker.duckdb".to_string());
        let node_id = env::var("NODE_ID")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .unwrap_or(1);

        info!(logger, "Initializing worker node"; 
            "node_id" => node_id,
            "db_path" => &db_path
        );

        let conn = Connection::open(&db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS data (partition_key TEXT, value DOUBLE);",
            [],
        )?;
        
        metrics::init_metrics();
        
        Ok(Self { 
            db: conn,
            node_id,
            logger,
        })
    }

    pub async fn insert(&self, data: InsertData) -> Result<()> {
        let _timer = RequestTimer::new();
        
        info!(self.logger, "Inserting data"; 
            "partition_key" => &data.partition_key,
            "value" => data.value
        );

        self.db.execute(
            "INSERT INTO data (partition_key, value) VALUES (?, ?);",
            [data.partition_key.as_str(), &data.value.to_string()],
        )?;

        metrics::record_storage_operation();
        Ok(())
    }

    pub async fn compute_average(&self) -> Result<ComputeResponse> {
        let _timer = RequestTimer::new();
        
        info!(self.logger, "Computing average");

        let mut stmt = self.db.prepare("SELECT AVG(value) as avg, COUNT(*) as cnt FROM data;")?;
        let mut rows = stmt.query([])?;
        
        if let Some(row) = rows.next()? {
            let response = ComputeResponse {
                average: row.get(0)?,
                count: row.get(1)?,
            };
            
            info!(self.logger, "Computed average"; 
                "average" => response.average,
                "count" => response.count
            );
            
            Ok(response)
        } else {
            Ok(ComputeResponse {
                average: 0.0,
                count: 0,
            })
        }
    }

    pub async fn get_health(&self) -> Result<()> {
        let mut stmt = self.db.prepare("SELECT 1;")?;
        stmt.query([])?;
        Ok(())
    }
}

pub async fn start_worker_node(logger: Logger) {
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8081".to_string())
        .parse()
        .unwrap_or(8081);

    let node = Arc::new(Mutex::new(WorkerNode::new(logger.clone()).unwrap()));

    let app = Router::new()
        .route("/insert", post(insert_handler))
        .route("/compute", post(compute_handler))
        .route("/health", get(health_handler))
        .with_state(node);

    info!(logger, "Worker node starting"; "port" => port);
    
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn insert_handler(
    State(node): State<Arc<Mutex<WorkerNode>>>,
    Json(data): Json<InsertData>,
) -> impl IntoResponse {
    let node = node.lock().await;
    match node.insert(data).await {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            error!(node.logger, "Insert failed"; "error" => %e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn compute_handler(
    State(node): State<Arc<Mutex<WorkerNode>>>,
) -> impl IntoResponse {
    let node = node.lock().await;
    match node.compute_average().await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => {
            error!(node.logger, "Compute failed"; "error" => %e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn health_handler(
    State(node): State<Arc<Mutex<WorkerNode>>>,
) -> impl IntoResponse {
    let node = node.lock().await;
    match node.get_health().await {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            error!(node.logger, "Health check failed"; "error" => %e);
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}
