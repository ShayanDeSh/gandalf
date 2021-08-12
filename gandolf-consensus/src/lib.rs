use uuid::Uuid;
use std::net::Ipv4Addr;
use tokio::sync::{oneshot};

pub mod raft_rpc {
    tonic::include_proto!("raft_rpc");
}

pub mod raft;

pub mod rpc;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

pub type NodeID = Uuid;

#[derive(Debug, Clone)]
pub struct Node {
    id: NodeID,
    ip: Ipv4Addr,
    port: u16,
}

pub enum RaftMessage {
    VoteMsg {
        body: raft_rpc::RequestVoteRequest,
        tx: oneshot::Sender<RaftMessage>
    },
    VoteResp {
        payload: raft_rpc::RequestVoteResponse,
        status: Option<tonic::Status>
    }
}
