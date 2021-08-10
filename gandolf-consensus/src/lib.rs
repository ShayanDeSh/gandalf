use uuid::Uuid;

pub mod raft_rpc {
    tonic::include_proto!("raft_rpc");
}

pub mod raft;

pub mod rpc;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

pub type NodeID = Uuid;
