use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use raft::{
    prelude::*,
    storage::MemStorage,
    Config,
};
use tracing::{info, error, debug};
use prometheus::HistogramTimer;

use crate::{
    Result,
    RaftMetricsError,
    metrics::{MetricsRegistry, MetricOperation, RAFT_CONSENSUS_LATENCY},
};

pub struct RaftNode {
    id: u64,
    peers: HashMap<u64, String>,
    node: Node<MemStorage>,
    msg_tx: mpsc::Sender<Message>,
    metrics: Arc<MetricsRegistry>,
}

impl RaftNode {
    pub fn new(
        id: u64, 
        peers: HashMap<u64, String>,
        msg_tx: mpsc::Sender<Message>,
        metrics: Arc<MetricsRegistry>,
    ) -> Result<Self> {
        let storage = MemStorage::new();
        let config = Config {
            id,
            election_tick: 10,
            heartbeat_tick: 3,
            max_size_per_msg: 1024 * 1024 * 1024,
            ..Default::default()
        };

        let node = Node::new(&config, storage)?;

        Ok(Self {
            id,
            peers,
            node,
            msg_tx,
            metrics,
        })
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

    pub fn has_ready(&mut self) -> bool {
        self.node.has_ready()
    }

    pub async fn handle_ready(&mut self, ready: Ready) -> Result<()> {
        let timer = RAFT_CONSENSUS_LATENCY.with_label_values(&["handle_ready"]).start_timer();

        // Handle messages
        for msg in ready.messages {
            if let Some(addr) = self.peers.get(&msg.to) {
                if let Err(e) = self.msg_tx.send(msg).await {
                    error!("Failed to send message: {}", e);
                }
            }
        }

        // Handle committed entries
        if !ready.committed_entries.is_empty() {
            for entry in ready.committed_entries {
                if entry.get_entry_type() == EntryType::EntryNormal && !entry.data.is_empty() {
                    match MetricsRegistry::deserialize_operation(&entry.data) {
                        Ok(operation) => {
                            if let Err(e) = self.metrics.apply_raft_entry(&entry.data).await {
                                error!("Failed to apply entry: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to deserialize entry: {}", e);
                        }
                    }
                }
            }
        }

        timer.observe_duration();
        Ok(())
    }
}

pub async fn run_raft_node(
    mut node: RaftNode,
    mut proposal_rx: mpsc::Receiver<Vec<u8>>,
    mut msg_rx: mpsc::Receiver<Message>,
) {
    let tick_interval = Duration::from_millis(100);
    
    loop {
        tokio::select! {
            Some(data) = proposal_rx.recv() => {
                if let Err(e) = node.propose(data) {
                    error!("Failed to propose: {}", e);
                }
            }
            Some(msg) = msg_rx.recv() => {
                if let Err(e) = node.step(msg) {
                    error!("Failed to step: {}", e);
                }
            }
            _ = sleep(tick_interval) => {
                node.tick();
            }
        }

        if node.has_ready() {
            let ready = node.node.ready();
            if let Err(e) = node.handle_ready(ready).await {
                error!("Failed to handle ready: {}", e);
            }
        }
    }
}
