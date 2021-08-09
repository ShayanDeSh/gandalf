use std::future::Future;
use rand::{thread_rng, Rng};

use tokio::time::{sleep_until, Duration, Instant};

use tracing::{debug, info, error};
use tracing::instrument;

use crate::{NodeID};

#[derive(Debug, PartialEq, Eq)]
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
    election_timeout: u64
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
        loop {
            match self.state {
                State::Follower => {
                    Follower::new(self, 1500).run().await?;
                },
                State::Candidate => {
                    debug!("Running at Candidate State");
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

}

impl<'a> Follower<'a> {
    pub fn new(raft: &'a mut Raft, election_timeout: u64) -> Follower {
        Follower {
            raft: raft,
            election_timeout: election_timeout
        }
    }

    #[instrument(level="trace", skip(self))]
    pub async fn run(&mut self) -> crate::Result<()> {
        debug!("Running at Follower State");
        while self.is_follower() {
            let time_out = sleep_until(self.generate_timeout());

            tokio::select! {
                _ = time_out => self.raft.set_state(State::Candidate),
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
