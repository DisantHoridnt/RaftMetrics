pub mod api;
pub mod error;
pub mod logging;
pub mod metrics;
pub mod raft;

pub use error::{RaftMetricsError, Result};

// Define partitioning module inline since it's small
pub mod partitioning {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    pub fn get_partition(key: &str, num_partitions: usize) -> usize {
        if num_partitions == 0 {
            return 0;
        }
        
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Use lower 32 bits for better distribution
        let hash_32 = (hash & 0xFFFFFFFF) as usize;
        hash_32 % num_partitions
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_partition_distribution() {
            let keys = vec!["test1", "test2", "test3", "test4", "test5"];
            let num_partitions = 2;
            let mut partition_counts = vec![0; num_partitions];

            for key in keys {
                let partition = get_partition(key, num_partitions);
                partition_counts[partition] += 1;
            }

            // Check that we have a somewhat even distribution
            for count in partition_counts {
                assert!(count > 0, "Each partition should get at least one key");
            }
        }
    }
}
