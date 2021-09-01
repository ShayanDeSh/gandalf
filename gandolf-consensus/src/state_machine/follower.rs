use crate::{Raft, ClientData, Tracker, RaftMessage};
use crate::raft::State;
use tracing::{instrument, debug, error};
use tokio::time::sleep_until;
use crate::raft_rpc::{RequestVoteRequest, RequestVoteResponse, AppendEntriesResponse, AppendEntriesRequest};

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
                Some(request)  = self.raft.rx_rpc.recv() => {
                    match self.handle_api_request(request).await {
                        Ok(_) => continue,
                        Err(err) => error!(cause = %err, "Caused an error: ")
                    }
                },
            }
        }

        Ok(())
    }

    fn is_follower(&self) -> bool {
        self.raft.state == State::Follower
    }

    async fn handle_api_request(&mut self, request: RaftMessage<T>) -> crate::Result<()> {
        match request {
            RaftMessage::VoteMsg{body, tx} => {
                let _ = tx.send(self.handle_vote_request(body));
            },
            RaftMessage::AppendMsg{body, tx} => {
                let _ = tx.send(self.handle_append_entry(body).await);
            },
            _ => unreachable!()
        }
        Ok(())
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
        match &self.raft.voted_for {
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

    async fn handle_append_entry(&mut self, body: AppendEntriesRequest) -> RaftMessage<T> {
        if self.raft.current_term > body.term {
            return RaftMessage::AppendResp {
                status: None,
                payload: Some(AppendEntriesResponse {
                    success: false,
                    term: self.raft.current_term
                })
            };
        }
        if self.raft.last_log_term != body.prev_log_term || self.raft.last_log_index != body.prev_log_index {
            return RaftMessage::AppendResp {
                status: None,
                payload: Some(AppendEntriesResponse {
                    success: false,
                    term: self.raft.current_term
                })
            };
        }
        if body.entries.len() == 0 {
            return RaftMessage::AppendResp {
                status: None,
                payload: Some(AppendEntriesResponse {
                    success: true,
                    term: self.raft.current_term
                })
            }
        }
        let entry = body.entries[0].clone();
        let entity: T = match serde_json::from_str(&entry.payload) {
            Ok(entity) => entity,
            Err(err) => {
                error!(cause = %err, "Caused an error: ");
                return RaftMessage::AppendResp {
                    status: Some(tonic::Status::cancelled("Could not parse the message")),
                    payload: None 
                }
            }
        };
        let mut tracker = self.raft.tracker.write().await;
        let last_log_index = match tracker.append_log(entity, body.term) {
            Ok(index) => index,
            Err(err) => {
                error!(cause = %err, "Caused an error: ");
                return RaftMessage::AppendResp {
                    status: Some(tonic::Status::cancelled("Coud not append to log")),
                    payload: None 
                }
            }
        };
        if body.leader_commit > last_log_index {
            for i in self.raft.commit_index..last_log_index {
                match tracker.commit(i).await {
                    Ok(index) => index,
                    Err(err) => {
                        error!(cause = %err, "Caused an error: ");
                        return RaftMessage::AppendResp {
                            status: Some(tonic::Status::cancelled("Coud not append to log")),
                            payload: None 
                        }
                    }
                };
                self.raft.commit_index = i;
            }
        }
        return RaftMessage::AppendResp {
            status: None,
            payload: Some(AppendEntriesResponse {
                success: true,
                term: self.raft.current_term
            })
        }
    }

}
