use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;
use serde_json;
use duckdb;

#[derive(Debug, Error)]
pub enum RaftMetricsError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Raft error: {0}")]
    Raft(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<duckdb::Error> for RaftMetricsError {
    fn from(err: duckdb::Error) -> Self {
        RaftMetricsError::Database(err.to_string())
    }
}

impl From<raft::Error> for RaftMetricsError {
    fn from(err: raft::Error) -> Self {
        RaftMetricsError::Raft(err.to_string())
    }
}

impl From<serde_json::Error> for RaftMetricsError {
    fn from(err: serde_json::Error) -> Self {
        RaftMetricsError::Serialization(err.to_string())
    }
}

impl IntoResponse for RaftMetricsError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            RaftMetricsError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                self.to_string(),
            ),
            RaftMetricsError::Raft(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                self.to_string(),
            ),
            RaftMetricsError::Serialization(_) => (
                StatusCode::BAD_REQUEST,
                self.to_string(),
            ),
            RaftMetricsError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                self.to_string(),
            ),
            RaftMetricsError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                self.to_string(),
            ),
        };

        let body = Json(json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, RaftMetricsError>;
