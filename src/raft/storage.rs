use protobuf::Message;
use raft::{
    prelude::*,
    storage::Storage,
    StorageError,
    GetEntriesContext,
};
use std::{
    cmp,
    sync::Mutex,
};

use crate::RaftMetricsError;

#[derive(Debug)]
pub struct MemStorage {
    hard_state: Mutex<HardState>,
    snapshot: Mutex<Snapshot>,
    entries: Mutex<Vec<Entry>>,
}

impl MemStorage {
    pub fn new() -> Self {
        MemStorage {
            hard_state: Mutex::new(HardState::default()),
            snapshot: Mutex::new(Snapshot::default()),
            entries: Mutex::new(vec![]),
        }
    }

    pub fn compute_size(&self) -> u64 {
        let mut size = 0;

        // Add size of entries
        let entries = self.entries.lock().unwrap();
        for entry in entries.iter() {
            size += entry.compute_size() as u64;
        }

        // Add size of hard state
        let hard_state = self.hard_state.lock().unwrap();
        size += hard_state.compute_size() as u64;

        // Add size of snapshot
        let snapshot = self.snapshot.lock().unwrap();
        size += snapshot.compute_size() as u64;

        size
    }
}

impl Storage for MemStorage {
    fn initial_state(&self) -> raft::Result<RaftState> {
        let hard_state = self.hard_state.lock().unwrap().clone();
        let conf_state = self.snapshot.lock().unwrap().get_metadata().get_conf_state().clone();
        Ok(RaftState { hard_state, conf_state })
    }

    fn entries(
        &self,
        low: u64,
        high: u64,
        max_size: impl Into<Option<u64>>,
        context: GetEntriesContext,
    ) -> raft::Result<Vec<Entry>> {
        let max_size = max_size.into();
        let entries = self.entries.lock().unwrap();
        let offset = entries.first().map_or(0, |e| e.index);
        let low = low.checked_sub(offset).ok_or(raft::Error::Store(
            StorageError::Compacted,
        ))?;
        let high = high.checked_sub(offset).ok_or(raft::Error::Store(
            StorageError::Compacted,
        ))?;
        let high = cmp::min(high, entries.len() as u64);
        let mut result = Vec::with_capacity(high as usize - low as usize);
        let mut size = 0;
        for i in low..high {
            let entry = &entries[i as usize];
            size += entry.compute_size() as u64;
            if let Some(max) = max_size {
                if size > max {
                    break;
                }
            }
            result.push(entry.clone());
        }
        Ok(result)
    }

    fn term(&self, idx: u64) -> raft::Result<u64> {
        let entries = self.entries.lock().unwrap();
        let offset = entries.first().map_or(0, |e| e.index);
        if idx < offset {
            return Err(raft::Error::Store(StorageError::Compacted));
        }
        if idx - offset >= entries.len() as u64 {
            return Err(raft::Error::Store(StorageError::Unavailable));
        }
        Ok(entries[(idx - offset) as usize].term)
    }

    fn first_index(&self) -> raft::Result<u64> {
        let entries = self.entries.lock().unwrap();
        Ok(entries.first().map_or(1, |e| e.index))
    }

    fn last_index(&self) -> raft::Result<u64> {
        let entries = self.entries.lock().unwrap();
        Ok(entries.last().map_or(0, |e| e.index))
    }

    fn snapshot(&self, request_index: u64, to: u64) -> raft::Result<Snapshot> {
        let snapshot = self.snapshot.lock().unwrap().clone();
        let meta = snapshot.get_metadata();
        if request_index <= meta.get_index() {
            return Ok(snapshot);
        }
        Err(raft::Error::Store(StorageError::Unavailable))
    }
}

pub fn setup_storage() -> MemStorage {
    MemStorage::new()
}
