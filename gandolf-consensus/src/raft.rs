use tracing::{debug};

#[derive(Debug)]
pub enum State {
    Follower,
    Candidate,
    Leader,
    NonVoter
}

#[derive(Debug)]
pub struct Raft {
    state: State
}


impl Raft {
    pub fn new() -> Raft {
        Raft {
            state: State::Follower
        }
    }

    pub async fn run(&mut self) {
        match self.state {
            State::Follower => {
                debug!("Running at Follower State");
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
    }

}

