use crate::{Raft, ClientData, Tracker, RaftMessage};
use crate::raft::State;
use tracing::{instrument, info, error};
use tokio::time::sleep_until;
use crate::raft_rpc::{RequestVoteRequest, RequestVoteResponse, AppendEntriesResponse, AppendEntriesRequest};
use crate::raft_rpc::ForwardEntryRequest;
use crate::rpc::forward;

#[derive(Debug)]
pub struct Follower <'a, T: ClientData, R: Tracker<Entity=T>> {
    raft: &'a mut Raft<T, R>
}

impl<'a, T: ClientData, R: Tracker<Entity=T>> Follower<'a, T, R> {
    pub fn new(raft: &'a mut Raft<T, R>) -> Follower<T, R> {
        Follower { raft }
    }

    #[instrument(level="info", skip(self))]
    pub async fn run(&mut self) -> crate::Result<()> {
        info!("Running at Follower State");
        info!("Current term is {}.", self.raft.current_term);
        while self.is_follower() {
            let election_timeout = sleep_until(self.raft.generate_timeout());

            tokio::select! {
                _ = election_timeout => {
                    info!("Timed out");
                    self.raft.set_state(State::Candidate)
                },
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
                info!("Recived a vote msg from {}", body.candidate_id);
                let _ = tx.send(self.handle_vote_request(body));
            },
            RaftMessage::AppendMsg{body, tx} => {
                let _ = tx.send(self.handle_append_entry(body).await);
            },
            RaftMessage::ClientReadMsg{body, tx} => {
                let _ = tx.send(self.forward_client_request(body, false).await);
            },
            RaftMessage::ClientWriteMsg{body, tx} => {
                let _ = tx.send(self.forward_client_request(body, true).await);
            }
            _ => unreachable!()
        }
        Ok(())
    }

    #[instrument(level="info", skip(self))]
    async fn forward_client_request(&self, body: T, iswrite: bool) -> RaftMessage<T> {
        let leader = self.raft.current_leader.as_ref();
        if let Some(id) = leader {
            let mut iter = self.raft.get_all_nodes().into_iter().filter(|x| x.id == id.to_owned());
            let node = iter.next().unwrap();
            let payload = serde_json::to_string(&body).unwrap();
            let request = ForwardEntryRequest { payload, iswrite };
            let resp = forward(&node, request).await;
            match resp {
                Ok(resp) => {
                    let body = serde_json::from_str(&resp.payload).unwrap();
                    return RaftMessage::ClientResp {
                        body 
                    };
                },
                Err(err) => {
                    return RaftMessage::ClientError{ body: err.to_string() };
                }
            }
        } else {
            return RaftMessage::ClientError{ body: "No leader exist".into() };
        };

    }

    #[instrument(level="info", skip(self))]
    fn handle_vote_request(&mut self, body: RequestVoteRequest) -> RaftMessage<T> {
        if self.raft.current_term > body.term {
            return RaftMessage::VoteResp {
                payload: RequestVoteResponse {
                    term: self.raft.current_term,
                    vote_granted: false
                },
                status: None
            }
        }
        self.raft.current_term = body.term;
        if (self.raft.last_log_term > body.last_log_term) || (self.raft.last_log_index > body.last_log_index) {
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

    async fn check_for_commit(&mut self, index: u64, leader_commit: u64) -> crate::Result<()> {
        let mut tracker = self.raft.tracker.write().await;
        if leader_commit > self.raft.commit_index {
            for i in self.raft.commit_index..std::cmp::min(leader_commit, index) {
                info!("Recived an append entry: Comiting");
                match tracker.commit(i).await {
                    Ok(_) => {
                        self.raft.commit_index = i + 1;
                    },
                    Err(err) => {
                        return Err(err);
                    }
                };
            }
        }
        Ok(())
    }

    #[instrument(level="info", skip(self))]
    async fn handle_append_entry(&mut self, body: AppendEntriesRequest) -> RaftMessage<T> {
        if self.raft.current_term > body.term {
            info!("Recived an append entry: False Response");
            return RaftMessage::AppendResp {
                status: None,
                payload: Some(AppendEntriesResponse {
                    success: false,
                    term: self.raft.current_term
                })
            };
        }
        if self.raft.last_log_term != body.prev_log_term || self.raft.last_log_index != body.prev_log_index {
            info!("Recived an append entry: False Response, last_log_term = {}, last_log_index = {}",
                self.raft.last_log_term, self.raft.last_log_index);
            return RaftMessage::AppendResp {
                status: None,
                payload: Some(AppendEntriesResponse {
                    success: false,
                    term: self.raft.current_term
                })
            };
        }
        self.raft.current_leader = Some(body.leader_id);
        if body.entries.len() == 0 {
            self.raft.current_term = body.term;
            match self.check_for_commit(self.raft.last_log_index, body.leader_commit).await {
                Ok(_) => {
                },
                Err(err) => {
                    error!(cause = %err, "Caused an error: ");
                    return RaftMessage::AppendResp {
                        status: Some(tonic::Status::cancelled("Coud not append to log")),
                        payload: None 
                    }
                }
            };

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
        let last_log_index = match tracker.append_log(entity, entry.term) {
            Ok(index) => {
                info!("Recived an append entry: Appending to log");
                index
            },
            Err(err) => {
                error!(cause = %err, "Caused an error: ");
                return RaftMessage::AppendResp {
                    status: Some(tonic::Status::cancelled("Coud not append to log")),
                    payload: None 
                }
            }
        };
        drop(tracker);
        self.raft.last_log_index = last_log_index;
        self.raft.last_log_term = entry.term;
        match self.check_for_commit(last_log_index, body.leader_commit).await {
            Ok(_) => {
            },
            Err(err) => {
                error!(cause = %err, "Caused an error: ");
                return RaftMessage::AppendResp {
                    status: Some(tonic::Status::cancelled("Coud not append to log")),
                    payload: None 
                }
            }
        };
        return RaftMessage::AppendResp {
            status: None,
            payload: Some(AppendEntriesResponse {
                success: true,
                term: self.raft.current_term
            })
        }
    }

}
