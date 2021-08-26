use crate::{Raft, ClientData, Tracker, RaftMessage};
use crate::raft::State;
use tracing::{instrument, debug};
use tokio::time::sleep_until;
use crate::raft_rpc::{RequestVoteRequest, RequestVoteResponse};

#[derive(Debug)]
pub struct Follower <'a, T: ClientData, R: Tracker<Entity=T>> {
    raft: &'a mut Raft<T, R>
}

impl<'a, T: ClientData, R: Tracker<Entity=T>> Follower<'a, T, R> {
    pub fn new(raft: &'a mut Raft<T, R>) -> Follower<T, R> {
        Follower { raft }
    }

    #[instrument(level="trace", skip(self))]
    pub async fn run(&mut self) -> crate::Result<()> {
        debug!("Running at Follower State");
        while self.is_follower() {
            let election_timeout = sleep_until(self.raft.generate_timeout());

            tokio::select! {
                _ = election_timeout => self.raft.set_state(State::Candidate),
                Some(request)  = self.raft.rx_rpc.recv() => self.handle_api_request(request),
            }
        }

        Ok(())
    }

    fn is_follower(&self) -> bool {
        self.raft.state == State::Follower
    }

    fn handle_api_request(&self, request: RaftMessage<T>) {
        match request {
            RaftMessage::VoteMsg{body, tx} => {
                let _ = tx.send(self.handle_vote_request(body));
            },
            _ => return
        }
    }

    fn handle_vote_request(&self, body: RequestVoteRequest) -> RaftMessage<T> {
        if self.raft.current_term > body.term {
            return RaftMessage::VoteResp {
                payload: RequestVoteResponse {
                    term: self.raft.current_term,
                    vote_granted: false
                },
                status: None
            }
        }
        else if (self.raft.last_log_term >= body.last_log_term)
            && (self.raft.last_log_index >= body.last_log_index) {
            return RaftMessage::VoteResp {
                payload: RequestVoteResponse {
                    term: self.raft.current_term,
                    vote_granted: false
                },
                status: None
            }
        }
        match self.raft.voted_for {
            Some(candidate_id) if candidate_id.to_string() == body.candidate_id => {
                return RaftMessage::VoteResp {
                    payload: RequestVoteResponse {
                        term: self.raft.current_term,
                        vote_granted: true
                    },
                    status: None
                }
            },
            Some(_) => {
                return RaftMessage::VoteResp {
                    payload: RequestVoteResponse {
                        term: self.raft.current_term,
                        vote_granted: false
                    },
                    status: None
                }
            },
            None => {
                return RaftMessage::VoteResp {
                    payload: RequestVoteResponse {
                        term: self.raft.current_term,
                        vote_granted: true
                    },
                    status: None
                }
            }
        }
    }
}
