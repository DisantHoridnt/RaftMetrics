use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;
use serde_json;
use duckdb;

#[derive(Debug, Error)]
pub enum RaftMetricsError {
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    
    #[error(transparent)]
    Raft(#[from] raft::Error),
    
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Database(#[from] duckdb::Error),
}

impl IntoResponse for RaftMetricsError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            RaftMetricsError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                self.to_string(),
            ),
            RaftMetricsError::InvalidRequest(_) => (
                StatusCode::BAD_REQUEST,
                self.to_string(),
            ),
            RaftMetricsError::Raft(_) | RaftMetricsError::Io(_) | RaftMetricsError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                self.to_string(),
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
