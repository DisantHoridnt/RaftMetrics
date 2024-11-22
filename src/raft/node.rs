use raft::{
    Config, Node, NodeId, RawNode, Peer,
    eraftpb::{ConfChange, ConfChangeType, Entry, Message},
    prelude::ConfState,
    StateRole,
};
use tokio::sync::mpsc;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use slog::{Logger, info, warn, error};
use thiserror::Error;

use super::storage::MemStorage;

#[derive(Error, Debug)]
pub enum RaftError {
    #[error("Node is not the leader")]
    NotLeader,
    #[error("Failed to propose: {0}")]
    ProposeFailed(String),
    #[error("Storage error: {0}")]
    Storage(#[from] raft::Error),
}

pub struct RaftNode {
    id: u64,
    node: RawNode<MemStorage>,
    peers: HashMap<u64, String>,
    logger: Logger,
    msg_tx: mpsc::UnboundedSender<Message>,
    proposals: Arc<Mutex<HashMap<u64, Proposal>>>,
}

struct Proposal {
    propose_time: Instant,
    cb: Box<dyn FnOnce(Result<(), RaftError>) + Send>,
}

impl RaftNode {
    pub fn new(
        id: u64,
        peers: HashMap<u64, String>,
        logger: Logger,
        msg_tx: mpsc::UnboundedSender<Message>,
    ) -> Result<Self, RaftError> {
        let storage = MemStorage::new();
        let config = Config {
            id,
            election_tick: 10,
            heartbeat_tick: 3,
            ..Default::default()
        };

        let node = RawNode::new(&config, storage, &logger)?;

        Ok(Self {
            id,
            node,
            peers,
            logger,
            msg_tx,
            proposals: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn step(&mut self, msg: Message) -> Result<(), RaftError> {
        self.node.step(msg)?;
        Ok(())
    }

    pub fn propose(&mut self, data: Vec<u8>) -> Result<(), RaftError> {
        if !self.is_leader() {
            return Err(RaftError::NotLeader);
        }

        self.node.propose(vec![], data)?;
        Ok(())
    }

    pub fn tick(&mut self) {
        self.node.tick();
    }

    pub fn is_leader(&self) -> bool {
        self.node.raft.state == StateRole::Leader
    }

    pub fn campaign(&mut self) -> Result<(), RaftError> {
        self.node.campaign()?;
        Ok(())
    }

    pub fn ready(&mut self) -> bool {
        self.node.has_ready()
    }

    pub async fn handle_ready(&mut self) -> Result<(), RaftError> {
        if !self.ready() {
            return Ok(());
        }

        let mut ready = self.node.ready();

        // Handle messages
        for msg in ready.messages.drain(..) {
            if let Err(e) = self.msg_tx.send(msg) {
                error!(self.logger, "Failed to send message"; "error" => %e);
            }
        }

        // Handle committed entries
        if !ready.committed_entries.is_empty() {
            let entries = ready.committed_entries.take().unwrap();
            for entry in entries {
                self.handle_committed_entry(entry)?;
            }
        }

        // Advance the Raft node
        self.node.advance(ready);
        Ok(())
    }

    fn handle_committed_entry(&mut self, entry: Entry) -> Result<(), RaftError> {
        if entry.data.is_empty() {
            // From time to time empty entries may appear, skip them.
            return Ok(());
        }

        // Handle configuration changes
        if entry.get_entry_type() == raft::eraftpb::EntryType::EntryConfChange {
            let change: ConfChange = protobuf::parse_from_bytes(&entry.data)
                .map_err(|e| RaftError::ProposeFailed(e.to_string()))?;
            
            let cs = self.node.apply_conf_change(&change)?;
            
            match change.get_change_type() {
                ConfChangeType::AddNode => {
                    info!(self.logger, "Node added to cluster"; "node_id" => change.node_id);
                }
                ConfChangeType::RemoveNode => {
                    info!(self.logger, "Node removed from cluster"; "node_id" => change.node_id);
                    if change.node_id == self.id {
                        // We've been removed from the cluster, shut down
                        return Err(RaftError::ProposeFailed("Node removed from cluster".into()));
                    }
                }
                _ => {}
            }
        }

        // Handle normal entries (your application data)
        // In a real application, you would apply these changes to your state machine
        info!(self.logger, "Applying entry"; "data" => format!("{:?}", entry.data));
        
        Ok(())
    }
}

// Helper function to create a new Raft node
pub async fn create_raft_node(
    node_id: u64,
    peers: HashMap<u64, String>,
    logger: Logger,
) -> Result<(RaftNode, mpsc::UnboundedReceiver<Message>), RaftError> {
    let (tx, rx) = mpsc::unbounded_channel();
    let node = RaftNode::new(node_id, peers, logger, tx)?;
    Ok((node, rx))
}
