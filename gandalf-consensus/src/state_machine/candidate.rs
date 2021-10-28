use crate::{Raft, ClientData, Tracker, RaftMessage};
use crate::raft::State;
use tracing::{error, info};
use tokio::time::sleep_until;
use tokio::sync::mpsc;
use crate::rpc::ask_for_vote;

use crate::raft_rpc::{RequestVoteRequest, RequestVoteResponse};

#[derive(Debug)]
pub struct Candidate <'a, T: ClientData, R: Tracker<Entity=T>> {
    raft: &'a mut Raft<T, R>,
    number_of_votes: u32
}

impl<'a, T: ClientData, R: Tracker<Entity=T>> Candidate<'a, T, R> {
    pub fn new(raft: &'a mut Raft<T, R>) -> Candidate<T, R> {
        Candidate {
            raft,
            number_of_votes: 0
        }
    }

    pub async fn run(&mut self) -> crate::Result<()> {
        info!("Running at Candidate State");
        info!("Current term is {}.", self.raft.current_term);
        while self.is_candidate() {
            self.raft.current_term += 1;

            self.raft.voted_for = Some(self.raft.id.clone());
            self.number_of_votes = 1;

            let mut vote_rx = self.ask_for_votes();

            while self.is_candidate() {
                info!("waiting for vote");
                let election_timeout = sleep_until(self.raft.generate_timeout());
                tokio::select! {
                    _ = election_timeout => break,
                    Some(response) = vote_rx.recv() => self.handle_vote(response)?,
                    Some(request)  = self.raft.rx_rpc.recv() => self.handle_api_request(request),
                }
            }
        }

        Ok(())
    }

    fn handle_api_request(&mut self, request: RaftMessage<T>) {
        match request {
            RaftMessage::VoteMsg{tx, body} => {
                let resp = self.handle_vote_request(body);
                let _ = tx.send(resp);
            }
            RaftMessage::AppendMsg{tx, ..} => {
                let resp = RaftMessage::AppendResp {
                    status: Some(tonic::Status::cancelled("Node is in Candidate state")),
                    payload: None 
                };
                let _ = tx.send(resp);
            },
            _ => unreachable!(),
        }
    }

    fn handle_vote_request(&mut self, body: RequestVoteRequest) -> RaftMessage<T> {
        if body.term > self.raft.current_term {
            self.raft.current_term = body.term;
        }
        if body.last_log_index >= self.raft.last_index() && 
            body.last_log_term >= self.raft.last_term() {
            self.raft.set_state(State::Follower);
            self.raft.voted_for = Some(body.candidate_id);
            return RaftMessage::VoteResp {
                status: None,
                payload: RequestVoteResponse {
                    term: self.raft.current_term,
                    vote_granted: true
                }
            };
        }
        return RaftMessage::VoteResp {
            status: None,
            payload: RequestVoteResponse {
                term: self.raft.current_term,
                vote_granted: false
            }
        };
    }

    pub fn ask_for_votes(&self) -> mpsc::Receiver<RequestVoteResponse> {
        let nodes = self.raft.get_all_nodes();

        let len = nodes.len();

        let (tx, rx) = mpsc::channel(len);

        for node in nodes.into_iter() {
            let res_tx = tx.clone();
            let request = RequestVoteRequest {
                term: self.raft.current_term,
                candidate_id: self.raft.id.to_string(),
                last_log_index: self.raft.last_index(),
                last_log_term: self.raft.last_term()
            };
            let _ = tokio::spawn(
                async move {
                    match ask_for_vote(&node, request).await {
                        Ok(response) =>  {
                            let _ = res_tx.send(response).await;
                        },
                        Err(e) => error!(err=%e,"Error in comunicating with {:?}", node)
                    }
                }
            );
        }

        return rx;
    }

    fn handle_vote(&mut self, response: RequestVoteResponse) -> crate::Result<()> {
        if response.term > self.raft.current_term {
            self.raft.set_state(State::Follower);
            self.raft.current_term = response.term;
            self.raft.current_leader = None;
            self.raft.voted_for = None;
            return Ok(());
        }

        if response.vote_granted {
            self.number_of_votes += 1;
            info!("A vote granted no {}", self.number_of_votes);
            if self.has_enough_vote() {
                self.raft.set_state(State::Leader);
            }
        }

        Ok(())
    }

    fn has_enough_vote(&self) -> bool {
        self.number_of_votes > (self.raft.nodes.len() as u32) / 2 
    }

    fn is_candidate(&self) -> bool {
        self.raft.state == State::Candidate
    }

}
