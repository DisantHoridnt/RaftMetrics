use std::time::Duration;
use raft::{
    eraftpb::Message,
    storage::MemStorage,
    Config, RawNode,
    prelude::*,
};
use slog::{Logger, o};
use tracing::warn;

use crate::{Result, RaftMetricsError};

pub struct RaftNode {
    id: u64,
    node: RawNode<MemStorage>,
}

impl RaftNode {
    pub fn new(id: u64, _peers: Vec<u64>) -> Result<Self> {
        let storage = MemStorage::new();
        let config = Config {
            id,
            ..Default::default()
        };

        // Create a logger for Raft
        let logger = Logger::root(slog::Discard, o!());
        let node = RawNode::new(&config, storage, &logger)?;

        Ok(Self { id, node })
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn step(&mut self, msg: Message) -> Result<()> {
        self.node.step(msg).map_err(|e| {
            warn!("Raft step error: {}", e);
            RaftMetricsError::Internal(format!("Raft step error: {}", e))
        })
    }

    pub fn tick(&mut self) {
        self.node.tick();
    }

    pub fn has_ready(&self) -> bool {
        self.node.has_ready()
    }

    pub fn ready(&mut self) -> Ready {
        self.node.ready()
    }

    pub fn advance(&mut self, ready: Ready) {
        self.node.advance(ready);
    }
}

pub async fn run_raft_node(mut node: RaftNode) {
    let tick_interval = Duration::from_millis(100);
    let mut tick_timer = tokio::time::interval(tick_interval);

    loop {
        tokio::select! {
            _ = tick_timer.tick() => {
                node.tick();
            }
            _ = tokio::signal::ctrl_c() => {
                warn!("Received ctrl-c signal, shutting down Raft node {}", node.get_id());
                break;
            }
        }

        if node.has_ready() {
            let ready = node.ready();
            node.advance(ready);
        }
    }
}
