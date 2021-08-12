use tonic::{Request, Response, Status};

use crate::raft_rpc::raft_rpc_server::{RaftRpc, RaftRpcServer};
use crate::raft_rpc::raft_rpc_client::RaftRpcClient;

use crate::raft_rpc::{AppendEntriesRequest, Entry, AppendEntriesResponse};
use crate::raft_rpc::{RequestVoteRequest, RequestVoteResponse};

use crate::{Node, NodeID, RaftMessage};

use tokio::sync::{mpsc};

#[derive(Debug)]
pub struct RaftRpcService {
    tx_rpc: mpsc::UnboundedSender<RaftMessage>
}

impl RaftRpcService {
    pub fn new(tx_rpc: mpsc::UnboundedSender<RaftMessage>) -> RaftRpcService {
        RaftRpcService {
            tx_rpc: tx_rpc
        }
    }
}


#[tonic::async_trait]
impl RaftRpc for RaftRpcService {
    async fn append_entries(&self,
        request: Request<AppendEntriesRequest>) ->
        Result<Response<AppendEntriesResponse>, Status> {
        unimplemented!();
    }

    async fn request_vote(&self, request: Request<RequestVoteRequest>) -> 
        Result<Response<RequestVoteResponse>, Status> {
            unimplemented!();
    }
}

pub async fn ask_for_vote(node: &Node, request: RequestVoteRequest) -> crate::Result<RequestVoteResponse> {
    let addr = format!("http://{}:{}", node.ip, node.port);
    let mut client = RaftRpcClient::connect(addr).await?;
    let response = client.request_vote(request).await?;
    Ok(response.into_inner())
}
