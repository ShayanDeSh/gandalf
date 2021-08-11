use std::future::Future;
use std::collections::HashSet;

use rand::{thread_rng, Rng};

use tokio::time::{sleep_until, Duration, Instant};
use tokio::sync::{mpsc};

use tracing::{debug, info, error};
use tracing::instrument;

use crate::{NodeID, Node};

use crate::rpc::ask_for_vote;
use crate::raft_rpc::{RequestVoteRequest, RequestVoteResponse};

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Follower,
    Candidate,
    Leader,
    NonVoter
}

pub enum RaftMessage {
}

#[derive(Debug)]
pub struct Raft {
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
    rx_rpc: mpsc::UnboundedReceiver<RaftMessage>,
    election_timeout: u64,
}

#[derive(Debug)]
struct Follower <'a> {
    raft: &'a mut Raft
}

#[derive(Debug)]
struct Candidate <'a> {
    raft: &'a mut Raft,
    number_of_votes: u32
}


pub async fn run(shutdown: impl Future) {
    let (_tx_rpc, rx_rpc) = mpsc::unbounded_channel();

    let mut raft = Raft::new(HashSet::new(), rx_rpc);
    tokio::select! {
        res = raft.run() => {
            if let Err(err) = res {
                error!(cause = %err, "Caused an error: ");
            }
        }
        _ = shutdown => {
            info!("Shutting down the server");
        }
    }
}

impl Raft {
    pub fn new(nodes: HashSet<Node>,
        rx_rpc: mpsc::UnboundedReceiver<RaftMessage>) -> Raft {
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
            nodes: nodes,
            rx_rpc: rx_rpc,
            election_timeout: 1500
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
                    debug!("Running at Leader State");
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

impl<'a> Follower<'a> {
    pub fn new(raft: &'a mut Raft) -> Follower {
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
            }
        }

        Ok(())
    }

    fn is_follower(&self) -> bool {
        self.raft.state == State::Follower
    }

}

impl<'a> Candidate<'a> {
    pub fn new(raft: &'a mut Raft) -> Candidate {
        Candidate {
            raft: raft,
            number_of_votes: 0
        }
    }

    pub async fn run(&mut self) -> crate::Result<()> {
        self.raft.current_term += 1;

        self.raft.voted_for = Some(self.raft.id);
        let number_of_votes = 1;

        let election_timeout = self.raft.generate_timeout();

        self.ask_for_votes();

        Ok(())
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
                    match ask_for_vote(node, request).await {
                        Ok(response) =>  tx.send(response).await,
                        Err(e) => error!(err=e, "Error in comunicating with {:?}", node)
                    }
                }
            );
        }

        return rx;
    }

}


