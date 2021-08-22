use std::collections::HashSet;

use rand::{thread_rng, Rng};

use tokio::time::{sleep_until, Duration, Instant};
use tokio::sync::mpsc;


use tracing::{debug, error};
use tracing::instrument;

use crate::{NodeID, Node, RaftMessage, ConfigMap, ClientData};

use crate::rpc::{self, ask_for_vote};
use crate::raft_rpc::{RequestVoteRequest, RequestVoteResponse};
use crate::raft_rpc::{AppendEntriesRequest, AppendEntriesResponse};

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Follower,
    Candidate,
    Leader,
    NonVoter
}


#[derive(Debug)]
pub struct Raft<T: ClientData> {
    id: NodeID,
    state: State,
    current_term: u64,
    commit_index: u64,
    last_applied: u64,
    last_log_index: u64,
    last_log_term: u64,
    voted_for: Option<NodeID>,
    current_leader: Option<NodeID>,
    nodes: HashSet<Node>,
    rx_rpc: mpsc::UnboundedReceiver<RaftMessage<T>>,
    election_timeout: u64,
    heartbeat: Duration
}

#[derive(Debug)]
struct Follower <'a, T: ClientData> {
    raft: &'a mut Raft<T>
}

#[derive(Debug)]
struct Candidate <'a, T: ClientData> {
    raft: &'a mut Raft<T>,
    number_of_votes: u32
}

#[derive(Debug)]
struct Leader <'a, T: ClientData> {
    raft: &'a mut Raft<T>
}

impl<T: ClientData> Raft<T> {
    pub fn new(config: ConfigMap, rx_rpc: mpsc::UnboundedReceiver<RaftMessage<T>>) -> Raft<T> {
        Raft {
            id: NodeID::new_v4(),
            state: State::Follower,
            current_term: 0,
            commit_index: 0,
            last_applied: 0,
            last_log_index: 0,
            last_log_term: 0,
            voted_for: None,
            current_leader: None,
            nodes: config.nodes,
            rx_rpc: rx_rpc,
            election_timeout: config.timeout,
            heartbeat: Duration::from_millis(config.heartbeat)
        }
    }

    pub async fn run(&mut self) -> crate::Result<()> {
        loop {
            match self.state {
                State::Follower => {
                    Follower::new(self).run().await?;
                },
                State::Candidate => {
                    Candidate::new(self).run().await?;
                    return Ok(());
                },
                State::Leader => {
                    Leader::new(self).run().await?;
                    return Ok(());
                },
                State::NonVoter => {
                    debug!("Running at NonVoter State");
                    return Ok(());
                }
            }
        }
        unreachable!();
        Ok(())
    }

    pub fn set_state(&mut self, state: State) {
        self.state = state;
    }

    fn generate_timeout(&self) -> Instant {
        let random = thread_rng().
            gen_range(self.election_timeout..self.election_timeout * 2);
        Instant::now() + Duration::from_millis(random)
    }

    fn get_all_nodes(&self) -> HashSet<Node> {
        self.nodes.clone()
    }

}

impl<'a, T: ClientData> Follower<'a, T> {
    pub fn new(raft: &'a mut Raft<T>) -> Follower<T> {
        Follower {
            raft: raft

        }
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

impl<'a, T: ClientData> Candidate<'a, T> {
    pub fn new(raft: &'a mut Raft<T>) -> Candidate<T> {
        Candidate {
            raft: raft,
            number_of_votes: 0
        }
    }

    pub async fn run(&mut self) -> crate::Result<()> {
        debug!("Running at Candidate State");
        while self.is_candidate() {
            self.raft.current_term += 1;

            self.raft.voted_for = Some(self.raft.id);
            self.number_of_votes = 1;

            let mut vote_rx = self.ask_for_votes();

            while self.is_candidate() {
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

    fn handle_api_request(&self, request: RaftMessage<T>) {
        match request {
            RaftMessage::VoteMsg{tx, ..} => {
                let resp = RaftMessage::VoteResp {
                    status: None,
                    payload: RequestVoteResponse {
                        term: self.raft.current_term,
                        vote_granted: false
                    }
                };
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

    pub fn ask_for_votes(&self) -> mpsc::Receiver<RequestVoteResponse> {
        let nodes = self.raft.get_all_nodes();

        let len = nodes.len();

        let (tx, rx) = mpsc::channel(len);

        for node in nodes.into_iter() {
            let res_tx = tx.clone();
            let request = RequestVoteRequest {
                term: self.raft.current_term,
                candidate_id: self.raft.id.to_string(),
                last_log_index: self.raft.last_log_index,
                last_log_term: self.raft.last_log_term
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

impl<'a, T: ClientData> Leader<'a, T> {
    pub fn new(raft:&'a mut Raft<T>) -> Leader<T> {
        Leader {
            raft: raft
        }
    }

    #[instrument(level="trace", skip(self))]
    pub async fn run(&self) -> crate::Result<()> {
        debug!("Running at Leader State");
        while self.is_leader() {
            let next_heart_beat = Instant::now() + self.raft.heartbeat;
            let next_heart_beat = sleep_until(next_heart_beat);

            tokio::select! {
                _ = next_heart_beat => {
                    self.beat().await?;
                }
            }
        }
        Ok(())
    }

    fn is_leader(&self) -> bool {
        self.raft.state == State::Leader
    }

    pub async fn beat(&self) -> crate::Result<()> {
        let nodes = self.raft.get_all_nodes().into_iter();
        for node in nodes {
            // TODO:
            // Must get prev_log_term and prev_log_term form tracker
            // Just left it 0 for now
            let request = AppendEntriesRequest {
                term: self.raft.current_term,
                leader_id: self.raft.id.to_string(),
                prev_log_index: 0,
                prev_log_term: 0,
                entries: vec![]
            };
            tokio::spawn(async move {
                let response = rpc::append_entries(&node, request).await;
                match response {
                    // TODO:
                    // The response must be handled
                    Ok(resp) => debug!("recived {:?} from {:?}", resp, node),
                    Err(err) => error!(cause = %err, "Caused an error: ")
                }
            });
        }
        Ok(())
    }
}
