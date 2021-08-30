use crate::{Raft, ClientData, Tracker, RaftMessage, Node, NodeID};
use crate::raft::State;
use tracing::{instrument, error, debug};
use tokio::time::{Instant, sleep_until};
use tokio::sync::{mpsc, RwLock};
use crate::raft_rpc::{AppendEntriesRequest, Entry};
use crate::rpc;

use std::sync::{Arc};

#[derive(Debug)]
pub struct Leader <'a, T: ClientData, R: Tracker<Entity=T>> {
    raft: &'a mut Raft<T, R>,
    replicators: Vec<mpsc::UnboundedSender<ReplicatorMsg>>,
    rx_repl: mpsc::UnboundedReceiver<ReplicatorMsg>
}

impl<'a, T: ClientData, R: Tracker<Entity=T>> Leader<'a, T, R> {
    pub fn new(raft:&'a mut Raft<T, R>) -> Leader<T, R> {
        let mut replicators = Vec::new();
        let (tx_repl, rx_core_repl) = mpsc::unbounded_channel();
        for node in raft.get_all_nodes().into_iter() {
            let (tx_core_repl, rx_repl) = mpsc::unbounded_channel();
            let mut replicator = Replicator::new(node, raft.last_log_index + 1,
                raft.current_term, raft.tracker.clone(), raft.id.clone(), rx_repl, tx_repl.clone());
            tokio::spawn(async move {
                replicator.run().await;
            });
            replicators.push(tx_core_repl);
        }
        Leader { raft , replicators, rx_repl: rx_core_repl }
    }

    #[instrument(level="trace", skip(self))]
    pub async fn run(&mut self) -> crate::Result<()> {
        debug!("Running at Leader State");
        while self.is_leader() {
            tokio::select! {
                Some(request) = self.raft.rx_rpc.recv() => self.handle_api_request(request).await?,
            }
        }
        Ok(())
    }

    async fn handle_api_request(&mut self, request: RaftMessage<T>) -> crate::Result<()> {
       match request {
           RaftMessage::ClientReadMsg {body, tx} => {
               let tracker = self.raft.tracker.read().await;
               let response = tracker.propagate(&body).await?;
               if let Err(_) = tx.send(RaftMessage::ClientResp { body: response }) {
                   error!("Peer drop the client response");
               }
           },
           RaftMessage::ClientWriteMsg {body, tx} => {
               let mut tracker = self.raft.tracker.write().await;
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

}

#[derive(Debug)]
enum ReplicatorMsg {
    ReplicateReq {
        index: u64
    },
    ReplicateResp {
        next_index: u64,
        match_index: u64,
        id: NodeID
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
struct Replicator <T: ClientData, R: Tracker<Entity=T>> {
    term: u64,
    match_index: u64,
    next_index: u64,
    node: Node,
    tracker: Arc<RwLock<R>>,
    id: NodeID,
    state: ReplicationState,
    rx_repl: mpsc::UnboundedReceiver<ReplicatorMsg>,
    tx_repl: mpsc::UnboundedSender<ReplicatorMsg>
}

impl<T: ClientData, R: Tracker<Entity=T>> Replicator<T, R> {
    pub fn new(node: Node, next_index: u64, term: u64, tracker: Arc<RwLock<R>>,
        id: NodeID, rx_repl: mpsc::UnboundedReceiver<ReplicatorMsg>, 
        tx_repl: mpsc::UnboundedSender<ReplicatorMsg>) -> Replicator<T, R> {
        Replicator {
            node,
            next_index,
            term,
            tracker,
            id,
            match_index: 0,
            state: ReplicationState::UpToDate,
            rx_repl,
            tx_repl
        }
    }

    async fn run(&mut self) {
        let _ = self.beat().await;

        loop {
            match &self.state {
                ReplicationState::Lagged => Lagged::new(self).run().await,
                ReplicationState::Updating => Updating::new(self).run().await,
                ReplicationState::UpToDate => UpToDate::new(self).run().await,
                _ => unreachable!()
            }
        }
    }

    pub async fn beat(&mut self) -> crate::Result<()> {
        let tracker = self.tracker.read().await;
        let request = AppendEntriesRequest {
            term: self.term,
            leader_id: self.id.to_string(),
            prev_log_index: self.next_index - 1,
            prev_log_term: tracker.get_log_term(self.next_index - 1),
            entries: vec![]
        };
        drop(tracker);
        let node = self.get_node();
        let response = rpc::append_entries(&node, request).await?;
        if !response.success {
            self.state = ReplicationState::Lagged;
            self.next_index -= 1;
        }
        Ok(())
    }

    pub fn get_node(&self) -> Node {
        self.node.clone()
    }

}

struct Lagged<'a, T: ClientData, R: Tracker<Entity=T>> {
    replicator: &'a mut Replicator<T, R>
}


impl<'a, T: ClientData, R: Tracker<Entity=T>> Lagged<'a, T, R> {
    pub fn new(replicator: &'a mut Replicator<T, R>) -> Lagged<'a, T, R> {
        Lagged {
            replicator
        }
    }

    pub async fn run(&mut self) {
        loop {
            if self.replicator.next_index -1 == self.replicator.match_index {
                self.replicator.state = ReplicationState::Updating;
                break;
            }
            let tracker = self.replicator.tracker.read().await;
            let request = AppendEntriesRequest {
                term: self.replicator.term,
                leader_id: self.replicator.id.to_string(),
                prev_log_index: self.replicator.next_index - 1,
                prev_log_term: tracker.get_log_term(self.replicator.next_index - 1),
                entries: vec![]
            };
            drop(tracker);
            let node = self.replicator.get_node();
            let result = rpc::append_entries(&node, request).await;
            let response = match result {
                Ok(resp) => resp,
                Err(err) => {
                    error!(cause = %err, "Caused an error: ");
                    continue;
                }
            };
            if response.success {
                self.replicator.state = ReplicationState::Updating;
                break;
            }
            self.replicator.next_index -= 1;
        }
    }
}

struct Updating<'a, T: ClientData, R: Tracker<Entity=T>> {
    replicator: &'a mut Replicator<T, R>
}

impl<'a, T: ClientData, R: Tracker<Entity=T>> Updating<'a, T, R> {
    pub fn new(replicator: &'a mut Replicator<T, R>) -> Updating<'a, T, R> {
        Updating {
            replicator
        }
    }

    pub async fn run(&mut self) {
        loop {
            let tracker = self.replicator.tracker.read().await;
            let last_log_index = tracker.get_last_log_index();
            if self.replicator.next_index > last_log_index {
                self.replicator.state = ReplicationState::UpToDate;
                break;
            }
            let entity = tracker.get_log_entity(self.replicator.next_index);
            let s_entity = match serde_json::to_string(&entity) {
                Ok(string) => string,
                Err(err) => {
                    error!(cause = %err, "Caused an error: ");
                    self.replicator.state = ReplicationState::Lagged;
                    break;
                }
            };
            let entry = Entry {
                payload: s_entity
            };
            let request = AppendEntriesRequest {
                term: self.replicator.term,
                leader_id: self.replicator.id.to_string(),
                prev_log_index: self.replicator.next_index - 1,
                prev_log_term: tracker.get_log_term(self.replicator.next_index - 1),
                entries: vec![entry]
            };
            drop(tracker);
            let node = self.replicator.get_node();
            let result = rpc::append_entries(&node, request).await;
            let response = match result {
                Ok(resp) => resp,
                Err(err) => {
                    error!(cause = %err, "Caused an error: ");
                    continue;
                }
            };
            if !response.success {
                self.replicator.state = ReplicationState::Lagged;
                break;
            }
            self.replicator.next_index += 1;
        }
    }
}

struct UpToDate<'a, T: ClientData, R: Tracker<Entity=T>> {
    replicator: &'a mut Replicator<T, R>
}


impl<'a, T: ClientData, R: Tracker<Entity=T>> UpToDate<'a, T, R> {
    pub fn new(replicator: &'a mut Replicator<T, R>) -> UpToDate<'a, T, R> {
        UpToDate {
            replicator
        }
    }

    pub async fn run(&mut self) {

    }
}

