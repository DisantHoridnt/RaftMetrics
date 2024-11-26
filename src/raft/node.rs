use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use raft::{
    prelude::*,
    storage::MemStorage,
    Config,
    RawNode,
};
use tracing::error;
use slog::{self, Drain, o};

use crate::{
    Result,
    error::RaftMetricsError,
    metrics::{MetricsRegistry, RAFT_CONSENSUS_LATENCY},
};

pub struct RaftNode {
    id: u64,
    peers: HashMap<u64, String>,
    node: RawNode<MemStorage>,
    msg_tx: mpsc::Sender<Message>,
}

impl RaftNode {
    pub fn new(
        id: u64, 
        peers: HashMap<u64, String>,
        msg_tx: mpsc::Sender<Message>,
        metrics: Arc<MetricsRegistry>,
    ) -> Result<Self> {
        // Create storage and initialize it
        let storage = MemStorage::new();
        let config = Config {
            id,
            election_tick: 10,
            heartbeat_tick: 3,
            max_size_per_msg: 1024 * 1024 * 1024,
            max_inflight_msgs: 256,
            ..Default::default()
        };

        // Create logger
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = slog::Logger::root(drain, o!("tag" => "raft-node"));

        // Initialize configuration state
        let peer_ids: Vec<u64> = peers.keys().cloned().collect();
        storage.wl().set_conf_state(ConfState::from((peer_ids, vec![])));

        let node = RawNode::new(&config, storage, &logger)?;

        Ok(Self {
            id,
            peers,
            node,
            msg_tx,
        })
    }

    pub fn has_ready(&mut self) -> bool {
        self.node.has_ready()
    }

    async fn send_messages(&mut self) -> Result<()> {
        let ready = self.node.ready();
        
        for msg in ready.messages() {
            if let Err(e) = self.msg_tx.send(msg.clone()).await {
                error!("Failed to send message: {}", e);
                return Err(RaftMetricsError::Internal(e.to_string()));
            }
        }
        Ok(())
    }

    pub async fn handle_ready(&mut self) -> Result<()> {
        let timer = RAFT_CONSENSUS_LATENCY.with_label_values(&["handle_ready"]).start_timer();

        let ready = self.node.ready();

        // Handle messages
        if let Err(e) = self.send_messages().await {
            return Err(e);
        }

        // Handle committed entries
        if !ready.committed_entries().is_empty() {
            for entry in ready.committed_entries() {
                if entry.get_entry_type() == EntryType::EntryNormal && !entry.data.is_empty() {
                    // Store the entry in the log
                    if let Err(e) = self.node.store().wl().append(&[entry.clone()]) {
                        error!("Failed to append entry: {}", e);
                        return Err(RaftMetricsError::Internal(e.to_string()));
                    }
                }
            }
        }

        // Advance the Raft state machine
        self.node.advance(ready);

        timer.observe_duration();
        Ok(())
    }

    pub fn propose(&mut self, data: Vec<u8>) -> Result<()> {
        let timer = RAFT_CONSENSUS_LATENCY.with_label_values(&["propose"]).start_timer();
        self.node.propose(vec![], data)?;
        timer.observe_duration();
        Ok(())
    }

    pub fn step(&mut self, msg: Message) -> Result<()> {
        self.node.step(msg)?;
        Ok(())
    }

    pub fn tick(&mut self) {
        self.node.tick();
    }
}

pub async fn run_raft_node(
    mut node: RaftNode,
    mut proposal_rx: mpsc::Receiver<Vec<u8>>,
) -> Result<()> {
    let tick_interval = Duration::from_millis(100);
    
    loop {
        tokio::select! {
            Some(data) = proposal_rx.recv() => {
                if let Err(e) = node.propose(data) {
                    error!("Failed to propose: {}", e);
                }
            }
            _ = sleep(tick_interval) => {
                node.tick();
            }
        }

        if node.has_ready() {
            if let Err(e) = node.handle_ready().await {
                error!("Failed to handle ready: {}", e);
            }
        }
    }
}
