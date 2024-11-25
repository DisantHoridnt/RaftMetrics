use prost::Message;
use raft::{
    prelude::*,
    Storage,
    StorageError,
    GetEntriesContext,
    Error as RaftError,
};
use std::sync::{Arc, Mutex};

use crate::RaftMetricsError;

#[derive(Debug)]
pub struct MemStorage {
    entries: Arc<Mutex<Vec<Entry>>>,
    hard_state: Arc<Mutex<HardState>>,
    snapshot: Arc<Mutex<Snapshot>>,
}

impl MemStorage {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
            hard_state: Arc::new(Mutex::new(HardState::default())),
            snapshot: Arc::new(Mutex::new(Snapshot::default())),
        }
    }
}

impl Storage for MemStorage {
    fn initial_state(&self) -> raft::Result<RaftState> {
        let hs = self.hard_state.lock().map_err(|e| 
            RaftError::Store(StorageError::Other(Box::new(RaftMetricsError::Internal(e.to_string())))))?;
        let cs = self.snapshot.lock().map_err(|e|
            RaftError::Store(StorageError::Other(Box::new(RaftMetricsError::Internal(e.to_string())))))?
            .get_metadata().get_conf_state().clone();
        Ok(RaftState { hard_state: (*hs).clone(), conf_state: cs })
    }

    fn entries(
        &self,
        low: u64,
        high: u64,
        max_size: impl Into<Option<u64>>,
        _context: GetEntriesContext,
    ) -> raft::Result<Vec<Entry>> {
        let max_size = max_size.into();
        let entries = self.entries.lock().map_err(|e|
            RaftError::Store(StorageError::Other(Box::new(RaftMetricsError::Internal(e.to_string())))))?;

        if entries.is_empty() {
            return Ok(Vec::new());
        }

        let first_idx = entries[0].index;
        if low < first_idx {
            return Err(RaftError::Store(StorageError::Compacted));
        }

        let last_idx = entries[entries.len() - 1].index;
        if high > last_idx + 1 {
            return Err(RaftError::Store(StorageError::Unavailable));
        }

        let mut ents: Vec<Entry> = vec![];
        let mut size = 0;
        for entry in entries.iter().skip((low - first_idx) as usize).take((high - low) as usize) {
            size += entry.encoded_len();
            if let Some(max) = max_size {
                if size > max as usize {
                    break;
                }
            }
            ents.push(entry.clone());
        }
        Ok(ents)
    }

    fn term(&self, idx: u64) -> raft::Result<u64> {
        let entries = self.entries.lock().map_err(|e|
            RaftError::Store(StorageError::Other(Box::new(RaftMetricsError::Internal(e.to_string())))))?;

        if entries.is_empty() {
            return Ok(0);
        }

        let first_idx = entries[0].index;
        let last_idx = entries[entries.len() - 1].index;

        if idx < first_idx {
            return Err(RaftError::Store(StorageError::Compacted));
        }

        if idx > last_idx {
            return Err(RaftError::Store(StorageError::Unavailable));
        }

        Ok(entries[(idx - first_idx) as usize].term)
    }

    fn first_index(&self) -> raft::Result<u64> {
        let entries = self.entries.lock().map_err(|e|
            RaftError::Store(StorageError::Other(Box::new(RaftMetricsError::Internal(e.to_string())))))?;
        Ok(entries.first().map_or(1, |e| e.index))
    }

    fn last_index(&self) -> raft::Result<u64> {
        let entries = self.entries.lock().map_err(|e|
            RaftError::Store(StorageError::Other(Box::new(RaftMetricsError::Internal(e.to_string())))))?;
        Ok(entries.last().map_or(0, |e| e.index))
    }

    fn snapshot(&self, request_index: u64, to: u64) -> raft::Result<Snapshot> {
        let snapshot = self.snapshot.lock().map_err(|e|
            RaftError::Store(StorageError::Other(Box::new(RaftMetricsError::Internal(e.to_string())))))?;
        
        let meta = snapshot.get_metadata();
        if request_index <= meta.index {
            return Ok((*snapshot).clone());
        }
        
        Err(RaftError::Store(StorageError::SnapshotTemporarilyUnavailable))
    }
}
