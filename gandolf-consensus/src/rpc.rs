use tonic::{Request, Response, Status};

use crate::raft_rpc::raft_rpc_server::{RaftRpc, RaftRpcServer};
use crate::raft_rpc::{AppendEntriesRequest, Entry, AppendEntriesResponse};
use crate::raft_rpc::{RequestVoteRequest, RequestVoteResponse};

#[derive(Debug)]
struct RaftRpcService;

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
