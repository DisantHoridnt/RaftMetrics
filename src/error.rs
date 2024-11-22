use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RaftMetricsError {
    #[error("Database error: {0}")]
    Database(#[from] duckdb::Error),
    
    #[error("Raft error: {0}")]
    Raft(#[from] raft::Error),
    
    #[error("Request error: {0}")]
    Request(String),
    
    #[error("Protobuf error: {0}")]
    Protobuf(String),
    
    #[error("Resource not found")]
    NotFound,
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

impl IntoResponse for RaftMetricsError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            RaftMetricsError::NotFound => (
                StatusCode::NOT_FOUND,
                self.to_string(),
            ),
            RaftMetricsError::InvalidRequest(_) => (
                StatusCode::BAD_REQUEST,
                self.to_string(),
            ),
            RaftMetricsError::Raft(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Raft error: {}", e),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                self.to_string(),
            ),
        };

        let body = Json(serde_json::json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, RaftMetricsError>;
