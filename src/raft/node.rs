use std::time::Duration;
use std::collections::HashMap;
use tokio::sync::mpsc;
use raft::{
    eraftpb::Message,
    storage::MemStorage,
    Config, RawNode,
    prelude::*,
};
use slog::{Logger, o};
use tracing::{info, warn, error, debug};
use prometheus::HistogramTimer;

use crate::{
    Result,
    RaftMetricsError,
    metrics::{MetricsRegistry, MetricOperation, RAFT_CONSENSUS_LATENCY},
};

pub struct RaftNode {
    id: u64,
    node: RawNode<MemStorage>,
    peers: HashMap<u64, String>,
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
            max_size_per_msg: 1024 * 1024,
            max_inflight_msgs: 256,
            applied: 0,
            max_uncommitted_size: 1024 * 1024,
            ..Default::default()
        };

        let logger = Logger::root(slog::Discard, o!());
        
        let s = storage;
        let peer_ids: Vec<u64> = peers.keys().cloned().collect();
        s.wl().set_conf_state(ConfState::from((peer_ids.clone(), vec![])));
        
        let node = RawNode::new(&config, s, &logger)?;
        info!("Initialized Raft node {} with peers {:?}", id, peer_ids);

        Ok(Self { 
            id, 
            node,
            peers,
            msg_tx,
            metrics,
        })
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn step(&mut self, msg: Message) -> Result<()> {
        self.node.step(msg).map_err(|e| {
            RaftMetricsError::Internal(format!("Failed to step Raft message: {}", e))
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

    pub fn propose(&mut self, operation: MetricOperation) -> Result<()> {
        let timer = RAFT_CONSENSUS_LATENCY.with_label_values(&["propose"]).start_timer();
        let data = MetricsRegistry::serialize_operation(&operation)?;
        self.node.propose(vec![], data).map_err(|e| {
            RaftMetricsError::Internal(format!("Failed to propose: {}", e))
        })?;
        timer.observe_duration();
        Ok(())
    }

    async fn send_messages(&self, msgs: Vec<Message>) -> Result<()> {
        let timer = RAFT_CONSENSUS_LATENCY.with_label_values(&["send_messages"]).start_timer();
        for msg in msgs {
            if let Some(addr) = self.peers.get(&msg.to) {
                debug!("Sending Raft message to peer {}: {:?}", msg.to, msg);
                self.msg_tx.send(msg).await.map_err(|e| {
                    RaftMetricsError::Internal(format!("Failed to send message: {}", e))
                })?;
            }
        }
        timer.observe_duration();
        Ok(())
    }

    async fn apply_committed_entries(&self, entries: Vec<Entry>) -> Result<()> {
        let timer = RAFT_CONSENSUS_LATENCY.with_label_values(&["apply"]).start_timer();
        for entry in entries {
            if entry.data.is_empty() {
                continue;
            }
            match MetricsRegistry::deserialize_operation(&entry.data) {
                Ok(operation) => {
                    debug!("Applying operation: {:?}", operation);
                    self.metrics.apply_raft_operation(operation).await?;
                }
                Err(e) => {
                    error!("Failed to deserialize operation: {}", e);
                }
            }
        }
        timer.observe_duration();
        Ok(())
    }
}

pub async fn run_raft_node(
    mut node: RaftNode,
    mut proposal_rx: mpsc::Receiver<MetricOperation>,
    mut msg_rx: mpsc::Receiver<Message>,
) {
    let mut tick_interval = tokio::time::interval(Duration::from_millis(100));

    loop {
        tokio::select! {
            _ = tick_interval.tick() => {
                node.tick();
            }
            Some(msg) = msg_rx.recv() => {
                if let Err(e) = node.step(msg) {
                    error!("Failed to step Raft message: {}", e);
                }
            }
            Some(operation) = proposal_rx.recv() => {
                if let Err(e) = node.propose(operation) {
                    error!("Failed to propose operation: {}", e);
                }
            }
        }

        if !node.has_ready() {
            continue;
        }

        let mut ready = node.ready();

        if let Err(e) = node.send_messages(ready.take_messages()).await {
            error!("Failed to send messages: {}", e);
        }

        if let Some(hs) = ready.hs() {
            debug!("Persisting hard state: {:?}", hs);
        }

        if !ready.entries().is_empty() {
            debug!("Persisting {} entries", ready.entries().len());
        }

        if !ready.committed_entries().is_empty() {
            debug!("Applying {} committed entries", ready.committed_entries().len());
            if let Err(e) = node.apply_committed_entries(ready.take_committed_entries()).await {
                error!("Failed to apply committed entries: {}", e);
            }
        }

        node.advance(ready);
    }
}
