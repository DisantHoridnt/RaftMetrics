use axum::{
    Router,
    routing::post,
    extract::{Extension, Json, State},
    response::IntoResponse,
    http::StatusCode,
};
use tokio::sync::Mutex;
use duckdb::{Connection, Result as DuckResult};
use std::{sync::Arc, env};
use serde::{Deserialize, Serialize};

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
}

impl WorkerNode {
    pub fn new() -> DuckResult<Self> {
        let db_path = env::var("DB_PATH").unwrap_or_else(|_| "/data/worker.duckdb".to_string());
        let node_id = env::var("NODE_ID")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .unwrap_or(1);

        let conn = Connection::open(&db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS data (partition_key TEXT, value DOUBLE);",
            [],
        )?;
        
        Ok(Self { 
            db: conn,
            node_id,
        })
    }

    pub async fn insert(&self, data: InsertData) -> DuckResult<()> {
        self.db.execute(
            "INSERT INTO data (partition_key, value) VALUES (?, ?);",
            [data.partition_key.as_str(), &data.value.to_string()],
        )?;
        Ok(())
    }

    pub async fn compute_average(&self) -> DuckResult<ComputeResponse> {
        let mut stmt = self.db.prepare("SELECT AVG(value) as avg, COUNT(*) as cnt FROM data;")?;
        let mut rows = stmt.query([])?;
        
        if let Some(row) = rows.next()? {
            Ok(ComputeResponse {
                average: row.get(0)?,
                count: row.get(1)?,
            })
        } else {
            Ok(ComputeResponse {
                average: 0.0,
                count: 0,
            })
        }
    }
}

pub async fn start_worker_node() {
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8081".to_string())
        .parse()
        .unwrap_or(8081);

    let node = Arc::new(Mutex::new(WorkerNode::new().unwrap()));

    let app = Router::new()
        .route("/insert", post(insert_handler))
        .route("/compute", post(compute_handler))
        .with_state(node);

    println!("Worker node starting on port {}", port);
    
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn insert_handler(
    State(node): axum::extract::State<Arc<Mutex<WorkerNode>>>,
    Json(data): Json<InsertData>,
) -> impl IntoResponse {
    let node = node.lock().await;
    match node.insert(data).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn compute_handler(
    State(node): axum::extract::State<Arc<Mutex<WorkerNode>>>,
) -> impl IntoResponse {
    let node = node.lock().await;
    match node.compute_average().await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
