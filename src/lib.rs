pub mod api;
pub mod raft;
pub mod utils;

pub mod partitioning {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    pub fn get_partition(key: &str, num_partitions: usize) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % num_partitions
    }
}

pub fn hash_partition(key: &str, num_partitions: usize) -> usize {
    // Simple hash function using sum of bytes
    let hash: u64 = key.bytes().map(|b| b as u64).sum();
    (hash % num_partitions as u64) as usize
}
