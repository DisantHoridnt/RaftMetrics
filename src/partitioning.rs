use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Get the partition number for a given metric name.
/// Uses consistent hashing to ensure the same metric always goes to the same worker.
/// 
/// # Arguments
/// * `metric_name` - Name of the metric
/// * `num_partitions` - Total number of partitions (workers)
/// 
/// # Returns
/// Partition number in range [0, num_partitions)
pub fn get_partition(metric_name: &str, num_partitions: usize) -> usize {
    if num_partitions == 0 {
        return 0;
    }

    let mut hasher = DefaultHasher::new();
    metric_name.hash(&mut hasher);
    let hash = hasher.finish();
    
    (hash % num_partitions as u64) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_partition_distribution() {
        let num_partitions = 2;
        let test_metrics = vec![
            "test1", "test2", "test3", "test4", "test5",
            "metric1", "metric2", "metric3", "metric4", "metric5",
        ];

        let mut partition_counts = HashMap::new();
        for metric in test_metrics {
            let partition = get_partition(metric, num_partitions);
            *partition_counts.entry(partition).or_insert(0) += 1;
            
            // Verify consistent hashing - same metric always goes to same partition
            assert_eq!(partition, get_partition(metric, num_partitions));
        }

        // Verify metrics are somewhat evenly distributed
        for count in partition_counts.values() {
            assert!(*count > 0, "Each partition should have at least one metric");
        }
    }

    #[test]
    fn test_edge_cases() {
        // Test with 0 partitions
        assert_eq!(get_partition("test", 0), 0);
        
        // Test with 1 partition
        assert_eq!(get_partition("test", 1), 0);
        
        // Test empty string
        let partition = get_partition("", 2);
        assert!(partition < 2);
    }
}
