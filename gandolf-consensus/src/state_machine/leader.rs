use crate::{Raft, ClientData, Tracker, RaftMessage, Node, NodeID};
use crate::raft::State;
use tracing::{instrument, error, debug};
use tokio::time::{Instant, sleep_until};
use crate::raft_rpc::AppendEntriesRequest;
use crate::rpc;

use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct Leader <'a, T: ClientData, R: Tracker<Entity=T>> {
    raft: &'a mut Raft<T, R>,
        replicators: Vec<Replicator<T, R>>
}

impl<'a, T: ClientData, R: Tracker<Entity=T>> Leader<'a, T, R> {
    pub fn new(raft:&'a mut Raft<T, R>) -> Leader<T, R> {
        let mut replicators = Vec::new();
        for node in raft.get_all_nodes().into_iter() {
            let replicator = Replicator::new(node, raft.last_log_index + 1,
                raft.current_term, raft.tracker.clone(), raft.id.clone());
            replicators.push(replicator);
        }
        Leader { raft , replicators}
    }

    #[instrument(level="trace", skip(self))]
    pub async fn run(&mut self) -> crate::Result<()> {
        debug!("Running at Leader State");
        while self.is_leader() {
            let next_heart_beat = Instant::now() + self.raft.heartbeat;
            let next_heart_beat = sleep_until(next_heart_beat);

            tokio::select! {
                _ = next_heart_beat => {
                    self.beat().await?;
                },
                Some(request) = self.raft.rx_rpc.recv() => self.handle_api_request(request).await?,
            }
        }
        Ok(())
    }

    async fn handle_api_request(&mut self, request: RaftMessage<T>) -> crate::Result<()> {
       match request {
           RaftMessage::ClientReadMsg {body, tx} => {
               let tracker = self.raft.tracker.read().unwrap();
               let response = tracker.propagate(&body).await?;
               if let Err(_) = tx.send(RaftMessage::ClientResp { body: response }) {
                   error!("Peer drop the client response");
               }
           },
           RaftMessage::ClientWriteMsg {body, tx} => {
               let mut tracker = self.raft.tracker.write().unwrap();
               let index = tracker.append_log(body, self.raft.last_log_term)?;
               self.raft.last_log_index = index;
               
           },
           _ => unreachable!()
       }
       Ok(())
    }

    fn is_leader(&self) -> bool {
        self.raft.state == State::Leader
    }

    pub async fn beat(&self) -> crate::Result<()> {
        let tracker = self.raft.tracker.read().unwrap();
        let nodes = self.raft.get_all_nodes().into_iter();
        for node in nodes {
            let request = AppendEntriesRequest {
                term: self.raft.current_term,
                leader_id: self.raft.id.to_string(),
                prev_log_index: tracker.get_last_log_index(),
                prev_log_term: tracker.get_last_log_term(),
                entries: vec![]
            };
            tokio::spawn(async move {
                let response = rpc::append_entries(&node, request).await;
                match response {
                    // TODO:
                    // The response must be handled
                    Ok(resp) => {
                    },
                    Err(err) => error!(cause = %err, "Caused an error: ")
                }
            });
        }
        Ok(())
    }
}

#[derive(Debug)]
enum ReplicationState {
    UpToDate,
    Lagged,
    NeedSnappshot,
    Updating
}

#[derive(Debug)]
struct Replicator <T: ClientData, R: Tracker<Entity=T>>{
    term: u64,
    match_index: u64,
    next_index: u64,
    node: Node,
    tracker: Arc<RwLock<R>>,
    id: NodeID,
    state: ReplicationState
}

impl<T: ClientData, R: Tracker<Entity=T>> Replicator<T, R> {
    pub fn new(node: Node, next_index: u64, term: u64, tracker: Arc<RwLock<R>>,
        id: NodeID) -> Replicator<T, R> {
        Replicator {
            node,
            next_index,
            term,
            tracker,
            id,
            match_index: 0,
            state: ReplicationState::UpToDate
        }
    }

    async fn run(&mut self) {
        let _ = self.beat().await;

    }

    pub async fn beat(&mut self) -> crate::Result<()> {
        let tracker = self.tracker.read().unwrap();
        let request = AppendEntriesRequest {
            term: self.term,
            leader_id: self.id.to_string(),
            prev_log_index: tracker.get_last_log_index(),
            prev_log_term: tracker.get_last_log_term(),
            entries: vec![]
        };
        drop(tracker);
        let node = self.node.clone();
        let response = rpc::append_entries(&node, request).await?;
        if !response.success {
            self.state = ReplicationState::Lagged;
            self.next_index -= 1;
        }
        Ok(())
    }

}

