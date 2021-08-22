use uuid::Uuid;
use std::net::{IpAddr, SocketAddr};
use tokio::sync::{oneshot};
use std::collections::HashSet;

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

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

pub type NodeID = Uuid;

pub trait ClientData: Send + Sync + Clone + 'static {}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Node {
    id: Option<NodeID>,
    ip: IpAddr,
    port: u16,
}

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
    }
}

pub struct ConfigMap {
    host: String,
    port: u16,
    nodes: HashSet<Node>,
    heartbeat: u64,
    timeout: u64
}

impl Node {
    pub fn new(id: Option<NodeID>, ip: IpAddr, port: u16) -> Node {
        Node {
            id: id,
            ip: ip,
            port: port
        }
    }
}

impl ConfigMap {
    pub fn new(host: String, port: u16, nodes_raw: Vec<String>, heartbeat: u64,
        timeout: u64) -> Result<ConfigMap> {

        let mut nodes = HashSet::new();

        for node_raw in nodes_raw.into_iter() {
            let node: SocketAddr = node_raw.parse()?;
            nodes.insert(Node::new(None, node.ip(), node.port()));
        }

        Ok(ConfigMap {
            host: host,
            port: port,
            nodes: nodes,
            heartbeat: heartbeat,
            timeout: timeout
        })
    }
}
