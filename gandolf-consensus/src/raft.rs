use std::future::Future;
use std::collections::HashSet;
use rand::{thread_rng, Rng};

use tokio::time::{sleep_until, Duration, Instant};
use tokio::sync::{mpsc};

use tracing::{debug, info, error};
use tracing::instrument;

use crate::{NodeID, Node};

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
    voted_for: Option<NodeID>,
    all_nodes: HashSet<Node>,
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
            voted_for: None,
            all_nodes: nodes,
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

    fn generate_timeout(&self) -> Instant {
        let random = thread_rng().
            gen_range(self.election_timeout..self.election_timeout * 2);
        Instant::now() + Duration::from_millis(random)
    }
}

impl<'a> Candidate<'a> {
    pub fn new(raft: &'a mut Raft) -> Candidate {
        Candidate {
            raft: raft
        }
    }

    pub async fn run(&mut self) -> crate::Result<()> {
        unimplemented!();
    }
}
