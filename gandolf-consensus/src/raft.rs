use std::future::Future;

use tracing::{debug, info, error};
use tracing::instrument;

use crate::{NodeID};

#[derive(Debug)]
pub enum State {
    Follower,
    Candidate,
    Leader,
    NonVoter
}

#[derive(Debug)]
pub struct Raft {
    id: NodeID,
    state: State,
    current_term: u64,
    commit_index: u64,
    last_applied: u64,
    voted_for: Option<NodeID>
    
}

#[derive(Debug)]
struct Follower <'a> {
    raft: &'a mut Raft,
}


pub async fn run(shutdown: impl Future) {
    let mut raft = Raft::new();
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
    pub fn new() -> Raft {
        Raft {
            id: NodeID::new_v4(),
            state: State::Follower,
            current_term: 0,
            commit_index: 0,
            last_applied: 0,
            voted_for: None
        }
    }

    pub async fn run(&mut self) -> crate::Result<()> {
        match self.state {
            State::Follower => {
                Follower::new(self).run().await?;
            },
            State::Candidate => {
                debug!("Running at Candidate State");
            },
            State::Leader => {
                debug!("Running at Leader State");
            },
            State::NonVoter => {
                debug!("Running at NonVoter State");
            }
        }
        Ok(())
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
        Ok(())
    }

}
