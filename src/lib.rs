pub mod api;
pub mod raft;
pub mod logging;
pub mod metrics;

pub fn hash_partition(key: &str, num_partitions: usize) -> usize {
    // Simple hash function using sum of bytes
    let hash: u64 = key.bytes().map(|b| b as u64).sum();
    (hash % num_partitions as u64) as usize
}

// Error types for the application
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Raft error: {0}")]
    Raft(#[from] raft::Error),
    
    #[error("Database error: {0}")]
    Database(#[from] duckdb::Error),
    
    #[error("Not the leader")]
    NotLeader,
    
    #[error("Node is shutting down")]
    ShuttingDown,
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

pub type Result<T> = std::result::Result<T, Error>;
