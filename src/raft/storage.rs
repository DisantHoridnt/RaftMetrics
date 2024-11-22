use raft::storage::MemStorage;

pub fn setup_storage() -> MemStorage {
    MemStorage::new()
}
