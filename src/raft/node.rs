use raft::{Config, RawNode};
use raft::storage::MemStorage;

pub fn setup_raft_node(id: u64) -> RawNode<MemStorage> {
    let config = Config::new(id);
    let storage = MemStorage::new();
    RawNode::new(&config, storage, vec![]).unwrap()
}
