use std::time::Duration;
use raft::{
    eraftpb::Message,
    storage::MemStorage,
    Config, RawNode,
    prelude::*,
};
use slog::{Logger, o};
use tracing::{info, warn};
use protobuf::Message as PbMessage;

use crate::{Result, RaftMetricsError};

pub struct RaftNode {
    id: u64,
    node: RawNode<MemStorage>,
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
            applied: 0,
            max_uncommitted_size: 1024 * 1024,
            ..Default::default()
        };

        // Create a logger for Raft
        let logger = Logger::root(slog::Discard, o!());
        
        // Initialize storage with configuration
        let mut s = storage;
        s.wl().set_conf_state(ConfState::from((peers, vec![])));
        
        let node = RawNode::new(&config, s, &logger)?;
        info!("Initialized Raft node {} with peers {:?}", id, peers);

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

    pub fn propose(&mut self, data: Vec<u8>) -> Result<()> {
        self.node.propose(vec![], data).map_err(|e| {
            warn!("Failed to propose data: {}", e);
            RaftMetricsError::Internal(format!("Failed to propose data: {}", e))
        })
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
            
            // Handle messages
            for msg in ready.messages() {
                info!("Processing Raft message: {:?}", msg);
            }
            
            // Handle committed entries
            for entry in ready.committed_entries() {
                if !entry.data.is_empty() {
                    info!("Applying committed entry: {:?}", entry);
                }
            }
            
            node.advance(ready);
        }
    }
}
