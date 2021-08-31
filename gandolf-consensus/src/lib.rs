use uuid::Uuid;
use std::net::{IpAddr, SocketAddr};
use tokio::sync::oneshot;
use std::collections::{HashSet, BTreeMap};
use serde::{de::DeserializeOwned, Serialize};

pub const DEFAULT_PORT: &str = "7899";
pub const HEARTBEAT: &str = "500";
pub const TIMEOUT: &str = "1500";

pub mod raft_rpc {
    tonic::include_proto!("raft_rpc");
}

pub mod raft;
pub use raft::Raft;

pub mod rpc;

pub mod tracker;
pub use tracker::Tracker;

pub mod parser;

pub mod client;

pub mod server;

pub mod state_machine;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

pub type NodeID = String;

pub trait ClientData: Send + Sync + Clone + Serialize + DeserializeOwned + std::fmt::Debug + 'static {}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Node {
    id: NodeID,
    ip: IpAddr,
    port: u16,
}

#[derive(Debug)]
pub struct NodeState {
    match_index: u64,
    next_index: u64
}

#[derive(Debug)]
pub enum RaftMessage<T: ClientData> {
    VoteMsg {
        body: raft_rpc::RequestVoteRequest,
        tx: oneshot::Sender<RaftMessage<T>>
    },
    VoteResp {
        payload: raft_rpc::RequestVoteResponse,
        status: Option<tonic::Status>
    },
    AppendMsg {
        body: raft_rpc::AppendEntriesRequest,
        tx: oneshot::Sender<RaftMessage<T>>
    },
    AppendResp {
        payload: Option<raft_rpc::AppendEntriesResponse>,
        status: Option<tonic::Status>
    },
    ClientReadMsg {
        body: T,
        tx: oneshot::Sender<RaftMessage<T>>
    },
    ClientWriteMsg {
        body: T,
        tx: oneshot::Sender<RaftMessage<T>>
    },
    ClientResp {
        body: T
    }
}

pub struct ConfigMap {
    host: String,
    port: u16,
    nodes: HashSet<Node>,
    nodes_state: BTreeMap<NodeID, NodeState>,
    heartbeat: u64,
    timeout: u64
}

impl Node {
    pub fn new(id: NodeID, ip: IpAddr, port: u16) -> Node {
        Node { id, ip, port }
    }
}

impl NodeState {
    pub fn new(match_index: u64, next_index: u64) -> NodeState {
        NodeState { match_index, next_index }
    }
}

impl ConfigMap {
    pub fn new(host: String, port: u16, nodes_raw: Vec<String>, heartbeat: u64,
        timeout: u64) -> Result<ConfigMap> {

        let mut nodes = HashSet::new();
        let mut nodes_state = BTreeMap::new();

        for node_raw in nodes_raw.into_iter() {
            let node: SocketAddr = node_raw.parse()?;
            let id = format!("{}:{}", node.ip(), node.port());
            let node_state = NodeState::new(0, 0);
            nodes.insert(Node::new(id, node.ip(), node.port()));
            nodes_state.insert(id, node_state);
        }

        Ok(ConfigMap { host, port, nodes, heartbeat, timeout, nodes_state})

    }
}
