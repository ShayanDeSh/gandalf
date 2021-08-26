use std::collections::HashSet;

use rand::{thread_rng, Rng};

use tokio::time::{Duration, Instant};
use tokio::sync::mpsc;


use tracing::debug;

use crate::{NodeID, Node, RaftMessage, ConfigMap, ClientData, Tracker};
use crate::state_machine::{Follower, Candidate, Leader};


#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Follower,
    Candidate,
    Leader,
    NonVoter
}


#[derive(Debug)]
pub struct Raft<T: ClientData, R: Tracker<Entity=T>> {
    pub id: NodeID,
    pub state: State,
    pub current_term: u64,
    pub commit_index: u64,
    pub last_applied: u64,
    pub last_log_index: u64,
    pub last_log_term: u64,
    pub voted_for: Option<NodeID>,
    pub current_leader: Option<NodeID>,
    pub nodes: HashSet<Node>,
    pub rx_rpc: mpsc::UnboundedReceiver<RaftMessage<T>>,
    pub election_timeout: u64,
    pub heartbeat: Duration,
    pub tracker: R
}




impl<T: ClientData, R: Tracker<Entity=T>> Raft<T, R> {
    pub fn new(config: ConfigMap, rx_rpc: mpsc::UnboundedReceiver<RaftMessage<T>>, tracker: R) -> Raft<T, R> {
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
            rx_rpc,
            election_timeout: config.timeout,
            heartbeat: Duration::from_millis(config.heartbeat),
            tracker
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

    pub fn generate_timeout(&self) -> Instant {
        let random = thread_rng().
            gen_range(self.election_timeout..self.election_timeout * 2);
        Instant::now() + Duration::from_millis(random)
    }

    pub fn get_all_nodes(&self) -> HashSet<Node> {
        self.nodes.clone()
    }

}
