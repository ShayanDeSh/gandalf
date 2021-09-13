use std::collections::{HashSet, BTreeMap};
use std::sync::Arc;

use rand::{thread_rng, Rng};

use tokio::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};

use tracing::info;

use crate::{NodeID, Node, NodeState, RaftMessage, ConfigMap, ClientData, Tracker};
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
    commit_index: u64,
    pub last_applied: u64,
    last_log_index: u64,
    last_log_term: u64,
    pub voted_for: Option<NodeID>,
    pub current_leader: Option<NodeID>,
    pub nodes: HashSet<Node>,
    pub nodes_state: BTreeMap<NodeID, NodeState>,
    pub rx_rpc: mpsc::UnboundedReceiver<RaftMessage<T>>,
    pub rx_snap: mpsc::UnboundedReceiver<RaftMessage<T>>,
    pub tx_snap: mpsc::UnboundedSender<RaftMessage<T>>,
    pub election_timeout: u64,
    pub heartbeat: Duration,
    pub snapshot_offset: u64,
    pub snapshot_num: u64,
    pub tracker: Arc<RwLock<R>>
}

impl<T: ClientData, R: Tracker<Entity=T>> Raft<T, R> {
    pub fn new(config: ConfigMap, rx_rpc: mpsc::UnboundedReceiver<RaftMessage<T>>,
        tracker: Arc<RwLock<R>>, id: String) -> Raft<T, R> {
        let (tx_snap, rx_snap) = mpsc::unbounded_channel();
        Raft {
            id,
            state: State::Follower,
            current_term: 0,
            commit_index: 0,
            last_applied: 0,
            last_log_index: 0,
            last_log_term: 0,
            voted_for: None,
            current_leader: None,
            nodes: config.nodes,
            nodes_state: config.nodes_state,
            rx_rpc,
            rx_snap,
            tx_snap,
            election_timeout: config.timeout,
            heartbeat: Duration::from_millis(config.heartbeat),
            snapshot_offset: config.snapshot_offset,
            snapshot_num: 0,
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
                },
                State::Leader => {
                    Leader::new(self).run().await?;
                },
                State::NonVoter => {
                    info!("Running at NonVoter State");
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

    pub fn update_last_log(&mut self, index: u64, term: u64) {
        self.last_log_index = index;
        self.last_log_term = term; 
    }

    pub fn update_commit_index(&mut self, index: u64) {
        self.commit_index = index;
        if index % self.snapshot_offset == 0 {
            let _ = self.tx_snap.send(RaftMessage::SnapMsg);
        }
    }

    pub async fn take_snapshot(&mut self) -> crate::Result<()> {
        let mut tracker = self.tracker.write().await;
        tracker.take_snapshot().await?;
        self.snapshot_num += 1;
        Ok(())
    }

    pub fn last_index(&self) -> u64 {
        self.last_log_index
    }

    pub fn last_term(&self) -> u64 {
        self.last_log_term
    }

    pub fn get_commit_index(&self) -> u64 {
        self.commit_index
    }
}
