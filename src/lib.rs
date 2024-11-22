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
