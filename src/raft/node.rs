use raft::{
    prelude::*,
    eraftpb::Message,
    storage::MemStorage,
    Config,
    StateRole,
};
use std::time::Duration;
use tracing::{info, error};
use slog::{Logger, o};

use crate::{
    Result,
    RaftMetricsError,
};

pub struct RaftNode {
    id: u64,
    raw_node: RawNode<MemStorage>,
}

impl RaftNode {
    pub fn new(id: u64, peers: Vec<u64>) -> Result<Self> {
        let storage = MemStorage::new();
        let config = Config {
            id,
            election_tick: 10,
            heartbeat_tick: 3,
            max_size_per_msg: 1024 * 1024,
            max_inflight_msgs: 256,
            ..Default::default()
        };

        let logger = Logger::root(slog::Discard, o!());
        let raw_node = RawNode::new(&config, storage, &logger)?;

        Ok(Self {
            id,
            raw_node,
        })
    }

    pub fn step(&mut self, msg: Message) -> Result<()> {
        self.raw_node
            .step(msg)
            .map_err(RaftMetricsError::Raft)
    }

    pub fn tick(&mut self) {
        self.raw_node.tick();
    }

    pub fn propose(&mut self, data: Vec<u8>) -> Result<()> {
        if !self.is_leader() {
            return Err(RaftMetricsError::Request("Not the leader".to_string()));
        }

        self.raw_node
            .propose(vec![], data)
            .map_err(RaftMetricsError::Raft)
    }

    pub fn ready(&mut self) -> Ready {
        self.raw_node.ready()
    }

    pub fn advance(&mut self, ready: Ready) {
        self.raw_node.advance(ready);
    }

    pub fn has_ready(&self) -> bool {
        self.raw_node.has_ready()
    }

    pub fn is_leader(&self) -> bool {
        self.raw_node.raft.state == StateRole::Leader
    }
}

pub async fn run_raft_node(mut node: RaftNode) {
    let tick_interval = Duration::from_millis(100);
    let mut tick_timer = tokio::time::interval(tick_interval);

    loop {
        tokio::select! {
            _ = tick_timer.tick() => {
                node.tick();
                if node.has_ready() {
                    let ready = node.ready();
                    // Process ready state
                    node.advance(ready);
                }
            }
        }
    }
}
