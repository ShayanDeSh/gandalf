use tonic::{Request, Response, Status};

use crate::raft_rpc::raft_rpc_server::RaftRpc;
use crate::raft_rpc::raft_rpc_client::RaftRpcClient;

use crate::raft_rpc::{AppendEntriesRequest, AppendEntriesResponse};
use crate::raft_rpc::{RequestVoteRequest, RequestVoteResponse};
use crate::raft_rpc::{ForwardEntryRequest, ForwardEntryResponse};

use crate::{Node, RaftMessage, ClientData};

use tokio::sync::{mpsc, oneshot};

use tracing::{info, error};

#[derive(Debug)]
pub struct RaftRpcService<T: ClientData> {
    tx_rpc: mpsc::UnboundedSender<RaftMessage<T>>
}

impl<T: ClientData> RaftRpcService<T> {
    pub fn new(tx_rpc: mpsc::UnboundedSender<RaftMessage<T>>) -> RaftRpcService<T> {
        RaftRpcService { tx_rpc }
    }
}

#[tonic::async_trait]
impl<T: ClientData> RaftRpc for RaftRpcService<T> {
    async fn append_entries(&self,
        request: Request<AppendEntriesRequest>) ->
        Result<Response<AppendEntriesResponse>, Status> {
        let (tx, rx) = oneshot::channel();
        let resp = self.tx_rpc.send(RaftMessage::AppendMsg{
            body: request.into_inner(),
            tx
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
                return Ok(Response::new(payload.unwrap()));
            },
            _ => {return Err(Status::unknown("Unkown response recived"));}
        }
    }

    async fn request_vote(&self, request: Request<RequestVoteRequest>) 
        -> Result<Response<RequestVoteResponse>, Status> {
        let (tx, rx) = oneshot::channel();
        let body = request.into_inner();
        info!("{:?} Asked for vote", &body.candidate_id);
        let resp = self.tx_rpc.send(RaftMessage::VoteMsg{
            body,
            tx
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

    async fn forward_entry(&self, request: Request<ForwardEntryRequest>) ->
        Result<Response<ForwardEntryResponse>, Status> {
        let (tx, rx) = oneshot::channel();
        let req = request.into_inner();

        let entity: T = match serde_json::from_str(&req.payload) {
            Ok(entity) => entity,
            Err(err) => {
                error!(cause = %err, "Caused an error: ");
                return Err(Status::invalid_argument("Could not parse the entity")); 
            }
        };

        let msg = if req.iswrite {
            RaftMessage::ClientWriteMsg {
                body: entity,
                tx
            }
        } else {
            RaftMessage::ClientReadMsg {
                body: entity,
                tx
            }
        };
        let resp = self.tx_rpc.send(msg);

        if let Err(err) = resp {
            return Err(Status::internal(err.to_string()));
        }
        let resp = match rx.await {
            Ok(msg) => msg,
            Err(err) => return Err(Status::internal(err.to_string()))
        };
        match resp {
            RaftMessage::ClientResp{body} => {
                let payload = serde_json::to_string(&body).unwrap();
                return Ok(Response::new(ForwardEntryResponse { payload }));
            },
            _ => {return Err(Status::unknown("Unkown response recived"));}
        }

    }

}

pub async fn ask_for_vote(node: &Node, request: RequestVoteRequest) 
    -> crate::Result<RequestVoteResponse> {
    info!("Asking {} for vote", node.id);
    let addr = format!("http://{}:{}", node.ip, node.port);
    let mut client = RaftRpcClient::connect(addr).await?;
    let response = client.request_vote(request).await?;
    let resp = response.into_inner();
    info!("Answerd with {:?}", resp);
    Ok(resp)
}

pub async fn append_entries(node: &Node, request: AppendEntriesRequest) 
    -> crate::Result<AppendEntriesResponse> {
    let addr = format!("http://{}:{}", node.ip, node.port);
    let mut client = RaftRpcClient::connect(addr).await?;
    let response = client.append_entries(request).await?;
    Ok(response.into_inner())
}

pub async fn forward(node: &Node, request: ForwardEntryRequest) 
    -> crate::Result<ForwardEntryResponse> {
    let addr = format!("http://{}:{}", node.ip, node.port);
    info!("Forwarding entry to {}:{}", node.ip, node.port);
    let mut client = RaftRpcClient::connect(addr).await?;
    let response = client.forward_entry(request).await?;
    Ok(response.into_inner())
}
