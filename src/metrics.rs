use lazy_static::lazy_static;
use prometheus::{
    Counter, Gauge, Histogram, HistogramOpts, IntCounter, IntGauge, Registry,
    register_counter, register_gauge, register_histogram, register_int_counter, register_int_gauge,
};
use std::time::Instant;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    // Node metrics
    pub static ref NODE_UP: IntGauge = register_int_gauge!(
        "node_up",
        "Whether the node is up (1) or down (0)"
    ).unwrap();

    pub static ref IS_LEADER: IntGauge = register_int_gauge!(
        "is_leader",
        "Whether this node is the leader (1) or not (0)"
    ).unwrap();

    // Request metrics
    pub static ref REQUEST_COUNTER: IntCounter = register_int_counter!(
        "requests_total",
        "Total number of requests received"
    ).unwrap();

    pub static ref REQUEST_DURATION: Histogram = register_histogram!(
        "request_duration_seconds",
        "Request duration in seconds",
        vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0]
    ).unwrap();

    // Raft metrics
    pub static ref RAFT_PROPOSALS: Counter = register_counter!(
        "raft_proposals_total",
        "Total number of proposals made"
    ).unwrap();

    pub static ref RAFT_PROPOSAL_FAILURES: Counter = register_counter!(
        "raft_proposal_failures_total",
        "Total number of failed proposals"
    ).unwrap();

    pub static ref RAFT_ROUND_TRIP_TIME: Histogram = register_histogram!(
        "raft_round_trip_time_seconds",
        "Round trip time for Raft proposals",
        vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0]
    ).unwrap();

    // Storage metrics
    pub static ref STORAGE_SIZE_BYTES: Gauge = register_gauge!(
        "storage_size_bytes",
        "Size of the storage in bytes"
    ).unwrap();

    pub static ref STORAGE_OPERATIONS: IntCounter = register_int_counter!(
        "storage_operations_total",
        "Total number of storage operations"
    ).unwrap();
}

pub fn init_metrics() {
    // Set initial values
    NODE_UP.set(1);
    IS_LEADER.set(0);
}

pub struct RequestTimer {
    start: Instant,
}

impl RequestTimer {
    pub fn new() -> Self {
        REQUEST_COUNTER.inc();
        Self {
            start: Instant::now(),
        }
    }
}

impl Drop for RequestTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        REQUEST_DURATION.observe(duration);
    }
}

pub fn record_proposal() {
    RAFT_PROPOSALS.inc();
}

pub fn record_proposal_failure() {
    RAFT_PROPOSAL_FAILURES.inc();
}

pub fn update_storage_size(size_bytes: f64) {
    STORAGE_SIZE_BYTES.set(size_bytes);
}

pub fn record_storage_operation() {
    STORAGE_OPERATIONS.inc();
}

pub fn set_leader_status(is_leader: bool) {
    IS_LEADER.set(if is_leader { 1 } else { 0 });
}

pub fn record_raft_rtt(duration: f64) {
    RAFT_ROUND_TRIP_TIME.observe(duration);
}
