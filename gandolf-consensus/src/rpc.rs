use tonic::{Request, Response, Status};

use crate::raft_rpc::raft_rpc_server::{RaftRpc, RaftRpcServer};
use crate::raft_rpc::raft_rpc_client::RaftRpcClient;

use crate::raft_rpc::{AppendEntriesRequest, Entry, AppendEntriesResponse};
use crate::raft_rpc::{RequestVoteRequest, RequestVoteResponse};

use crate::{Node, RaftMessage};

use tokio::sync::{mpsc, oneshot};

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
        let (tx, rx) = oneshot::channel();
        let resp = self.tx_rpc.send(RaftMessage::AppendMsg{
            body: request.into_inner(),
            tx: tx
        });
        if let Err(err) = resp {
            return Err(Status::internal(err.to_string()));
        }
        let resp = match rx.await {
            Ok(msg) => msg,
            Err(err) => return Err(Status::internal(err.to_string()))
        };
        match resp {
            RaftMessage::AppendResp{payload, status} => {
                if let Some(status) = status {
                    return Err(status);
                }
                return Ok(Response::new(payload));
            },
            _ => {return Err(Status::unknown("Unkown response recived"));}
        }
    }

    async fn request_vote(&self, request: Request<RequestVoteRequest>) 
        -> Result<Response<RequestVoteResponse>, Status> {
        let (tx, rx) = oneshot::channel();
        let resp = self.tx_rpc.send(RaftMessage::VoteMsg{
            body: request.into_inner(),
            tx: tx
        });
        if let Err(err) = resp {
            return Err(Status::internal(err.to_string()));
        }
        let resp = match rx.await {
            Ok(msg) => msg,
            Err(err) => return Err(Status::internal(err.to_string()))
        };
        match resp {
            RaftMessage::VoteResp{payload, status} => {
                if let Some(status) = status {
                    return Err(status);
                }
                return Ok(Response::new(payload));
            },
            _ => {return Err(Status::unknown("Unkown response recived"));}
        }
    }
}

pub async fn ask_for_vote(node: &Node, request: RequestVoteRequest) -> crate::Result<RequestVoteResponse> {
    let addr = format!("http://{}:{}", node.ip, node.port);
    let mut client = RaftRpcClient::connect(addr).await?;
    let response = client.request_vote(request).await?;
    Ok(response.into_inner())
}
