use raft::{
    eraftpb::Entry,
    Storage, Result as RaftResult,
    Error as RaftError,
    StorageError,
    RaftState,
    snapshot::{Snapshot, SnapshotMetadata},
};
use std::{
    sync::{Arc, RwLock},
    collections::HashMap,
};

#[derive(Clone)]
pub struct MemStorage {
    state: Arc<RwLock<InnerState>>,
}

struct InnerState {
    hard_state: raft::eraftpb::HardState,
    snapshot_metadata: SnapshotMetadata,
    entries: Vec<Entry>,
    snapshot: Snapshot,
}

impl MemStorage {
    pub fn new() -> Self {
        MemStorage {
            state: Arc::new(RwLock::new(InnerState {
                hard_state: Default::default(),
                snapshot_metadata: Default::default(),
                entries: vec![],
                snapshot: Default::default(),
            })),
        }
    }

    pub fn append(&self, entries: Vec<Entry>) -> RaftResult<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut state = self.state.write().unwrap();
        state.entries.extend_from_slice(&entries);
        Ok(())
    }

    pub fn set_hard_state(&self, hs: raft::eraftpb::HardState) -> RaftResult<()> {
        let mut state = self.state.write().unwrap();
        state.hard_state = hs;
        Ok(())
    }
}

impl Storage for MemStorage {
    fn initial_state(&self) -> RaftResult<RaftState> {
        let state = self.state.read().unwrap();
        Ok(RaftState {
            hard_state: state.hard_state.clone(),
            conf_state: Default::default(),
        })
    }

    fn entries(
        &self,
        low: u64,
        high: u64,
        max_size: impl Into<Option<u64>>,
    ) -> RaftResult<Vec<Entry>> {
        let state = self.state.read().unwrap();
        if low < state.snapshot_metadata.index {
            return Err(StorageError::Compacted.into());
        }

        if high > state.entries.len() as u64 + state.snapshot_metadata.index {
            return Err(StorageError::Unavailable.into());
        }

        let offset = state.snapshot_metadata.index;
        let lo = (low - offset) as usize;
        let hi = (high - offset) as usize;
        
        let mut entries = state.entries[lo..hi].to_vec();
        let max_size = max_size.into();
        if let Some(max_size) = max_size {
            let mut size = 0;
            let mut truncate_idx = entries.len();
            for (i, e) in entries.iter().enumerate() {
                size += e.compute_size() as u64;
                if size > max_size {
                    truncate_idx = i;
                    break;
                }
            }
            entries.truncate(truncate_idx);
        }
        Ok(entries)
    }

    fn term(&self, idx: u64) -> RaftResult<u64> {
        let state = self.state.read().unwrap();
        if idx == state.snapshot_metadata.index {
            return Ok(state.snapshot_metadata.term);
        }

        let offset = state.snapshot_metadata.index;
        if idx < offset {
            return Err(StorageError::Compacted.into());
        }

        if idx > offset + state.entries.len() as u64 - 1 {
            return Err(StorageError::Unavailable.into());
        }

        Ok(state.entries[(idx - offset) as usize].term)
    }

    fn first_index(&self) -> RaftResult<u64> {
        let state = self.state.read().unwrap();
        Ok(state.snapshot_metadata.index + 1)
    }

    fn last_index(&self) -> RaftResult<u64> {
        let state = self.state.read().unwrap();
        Ok(state.snapshot_metadata.index + state.entries.len() as u64)
    }

    fn snapshot(&self, request_index: u64) -> RaftResult<Snapshot> {
        let state = self.state.read().unwrap();
        Ok(state.snapshot.clone())
    }
}

pub fn setup_storage() -> MemStorage {
    MemStorage::new()
}
