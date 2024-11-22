use axum::{Router, routing::post, AddExtensionLayer};
use tokio::sync::Mutex;
use duckdb::{Connection, Result};
use std::sync::Arc;

pub struct WorkerNode {
    db: Connection,
}

impl WorkerNode {
    pub fn new() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute("CREATE TABLE data (partition_key TEXT, value DOUBLE);")?;
        Ok(Self { db: conn })
    }
}

pub async fn start_worker_node() {
    let node = Arc::new(Mutex::new(WorkerNode::new().unwrap()));

    let app = Router::new()
        .route("/insert", post(insert_handler))
        .route("/compute", post(compute_handler))
        .layer(AddExtensionLayer::new(node));

    axum::Server::bind(&"0.0.0.0:8081".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn insert_handler() {
    // Implementation for inserting data into DuckDB
}

async fn compute_handler() {
    // Implementation for performing computations on stored data
}
