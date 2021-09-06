use crate::{Raft, ClientData, Tracker, RaftMessage, Node, NodeID};
use crate::raft::State;
use tracing::{instrument, error, info};
use tokio::time::{Instant, sleep_until, Duration, sleep};
use tokio::sync::{mpsc, RwLock, oneshot};
use crate::raft_rpc::{AppendEntriesRequest, Entry, AppendEntriesResponse};
use crate::rpc;
use std::collections::BTreeMap;

use std::cmp::min;

use std::sync::Arc;

#[derive(Debug)]
pub struct Leader <'a, T: ClientData, R: Tracker<Entity=T>> {
    raft: &'a mut Raft<T, R>,
    replicators: Vec<mpsc::UnboundedSender<ReplicatorMsg>>,
    rx_repl: mpsc::UnboundedReceiver<ReplicatorMsg>,
    commit_queue: BTreeMap<u64, oneshot::Sender<RaftMessage<T>>>
}

#[derive(Debug, Clone)]
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

#[derive(Debug, PartialEq, Eq)]
enum ReplicationState {
    UpToDate,
    Lagged,
    NeedSnappshot,
    Updating,

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
    tx_repl: mpsc::UnboundedSender<ReplicatorMsg>,
    heartbeat: Duration
}

impl<'a, T: ClientData, R: Tracker<Entity=T>> Leader<'a, T, R> {
    pub fn new(raft:&'a mut Raft<T, R>) -> Leader<T, R> {
        let mut replicators = Vec::new();

        let (tx_repl, rx_core_repl) = mpsc::unbounded_channel();

        for node in raft.get_all_nodes().into_iter() {
            let (tx_core_repl, rx_repl) = mpsc::unbounded_channel();
            let match_index = if let Some(state) = raft.nodes_state.get(&node.id) {
                state.match_index
            } else {
                0
            };
            let mut replicator = Replicator::new(node, raft.last_log_index + 1,
                match_index, raft.current_term, raft.tracker.clone(), raft.id.clone(),
                rx_repl, tx_repl.clone(), raft.heartbeat);

            tokio::spawn(async move {
                replicator.run().await;
            });

            replicators.push(tx_core_repl);
        }

        Leader { raft, replicators, rx_repl: rx_core_repl,
        commit_queue: BTreeMap::new()}
    }

    #[instrument(level="info", skip(self))]
    pub async fn run(&mut self) -> crate::Result<()> {
        info!("Running at Leader State");
        while self.is_leader() {
            tokio::select! {
                Some(request) = self.raft.rx_rpc.recv() => 
                    self.handle_api_request(request).await?,
                Some(request) = self.rx_repl.recv() =>  {
                    self.handle_replicator_resp(request).await?
                }
            }
        }
        Ok(())
    }

    #[instrument(level="info", skip(self))]
    async fn handle_api_request(&mut self, request: RaftMessage<T>) -> crate::Result<()> {
       match request {
            RaftMessage::ClientReadMsg {body, tx} => {
                info!("Received A client read message.");
                let tracker = self.raft.tracker.read().await;
                let response = tracker.propagate(&body).await?;
                info!("Received Response {:?}", response);
                if let Err(_) = tx.send(RaftMessage::ClientResp { body: response }) {
                    error!("Peer drop the client response");
                }
            },
            RaftMessage::ClientWriteMsg {body, tx} => {
                info!("Received A client write message.");
                let mut tracker = self.raft.tracker.write().await;
                let index = tracker.append_log(body, self.raft.current_term)?;
                self.raft.last_log_index = index;
                self.raft.last_log_term = self.raft.current_term;
                self.commit_queue.insert(index, tx);
                let repl_req = ReplicatorMsg::ReplicateReq{index};
                for replicator in &self.replicators {
                    info!("Sending to {:?}", replicator);
                    let _ = replicator.send(repl_req.clone());
                }
            },
           _ => unreachable!()
       }
       Ok(())
    }

    #[instrument(level="info", skip(self))]
    async fn handle_replicator_resp(&mut self, request: ReplicatorMsg) -> crate::Result<()> {
        match request {
            ReplicatorMsg::ReplicateResp{next_index, match_index, id} => {
                info!("Recived A Replicator message");
                let node_state = self.raft.nodes_state.get_mut(&id);
                if let Some(state) = node_state {
                    state.next_index = next_index;
                    state.match_index = match_index;
                }
                self.check_for_commit(match_index).await?;
            },
            _ => unreachable!()
        }
        Ok(())
    }

    fn is_leader(&self) -> bool {
        self.raft.state == State::Leader
    }

    #[instrument(level="info", skip(self))]
    async fn check_for_commit(&mut self, index: u64) -> crate::Result<()> {
        info!("Checking possible commit.");
        if self.raft.commit_index < index {
            let number = self.raft.nodes_state.values()
                .into_iter()
                .fold(0, |acc, s| if s.match_index >= index {acc + 1} else {acc});
            info!("matched number is {}", number);

            let mut tracker = self.raft.tracker.write().await;

            if number >= self.raft.nodes.len() / 2 {
                for i in self.raft.commit_index..index {
                    info!("Commiting index {}.", i);
                    let frame = tracker.commit(i).await?;
                    self.raft.commit_index = i + 1;
                    if let Some(tx) = self.commit_queue.remove(&(i + 1)) {
                        let _ = tx.send(RaftMessage::ClientResp{ body: frame });
                    }
                }
            }
        }
        Ok(())
    }

}

impl<T: ClientData, R: Tracker<Entity=T>> Replicator<T, R> {
    pub fn new(node: Node, next_index: u64, match_index: u64, term: u64,
        tracker: Arc<RwLock<R>>, id: NodeID,
        rx_repl: mpsc::UnboundedReceiver<ReplicatorMsg>, 
        tx_repl: mpsc::UnboundedSender<ReplicatorMsg>, heartbeat: Duration)
        -> Replicator<T, R> {
        Replicator {
            node,
            next_index,
            term,
            tracker,
            id,
            match_index,
            state: ReplicationState::UpToDate,
            rx_repl,
            tx_repl,
            heartbeat
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
            entries: vec![],
            leader_commit: tracker.get_last_commited_index()
        };
        drop(tracker);
        let node = self.get_node();
//        info!("beating for {} with {:?}", node.id, request);
        let response = rpc::append_entries(&node, request).await?;
        if !response.success {
            self.state = ReplicationState::Lagged;
            self.next_index -= 1;
        }
        Ok(())
    }

    pub async fn creat_append_request(&self, index: u64) -> crate::Result<AppendEntriesRequest> {
        let tracker = self.tracker.read().await;
        let entity = tracker.get_log_entity(index);
        let term = tracker.get_log_term(index);
        let s_entity = serde_json::to_string(&entity)?; 
        let entry = Entry {
            payload: s_entity,
            term
        };
        Ok(AppendEntriesRequest {
            term: self.term,
            leader_id: self.id.to_string(),
            prev_log_index: self.next_index - 1,
            prev_log_term: tracker.get_log_term(self.next_index - 1),
            entries: vec![entry],
            leader_commit: tracker.get_last_commited_index()
        })
    }

    pub async fn append_entry(&self, index: u64) 
        -> crate::Result<AppendEntriesResponse> {
        let request = self.creat_append_request(index).await?; 
        let node = self.get_node();
        info!("sending append_entries for {} with {:?}", node.id, request);
        let response = rpc::append_entries(&node, request).await?;
        Ok(response)
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

    #[instrument(level="info", skip(self))]
    pub async fn run(&mut self) {
        info!(
            id=self.replicator.node.id.as_str(),
            "Replicator running at Lagged state."
            );
        loop {
            info!("next_index: {}, match_index: {}", self.replicator.next_index, self.replicator.match_index);
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
                entries: vec![],
                leader_commit: tracker.get_last_commited_index()
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

    #[instrument(level="info", skip(self))]
    pub async fn run(&mut self) {
        info!(
            id=self.replicator.node.id.as_str(),
            "Replicator running at Updating state."
            );
        let mut backoff = Duration::from_millis(1);
        loop {
            let tracker = self.replicator.tracker.read().await;
            let last_log_index = tracker.get_last_log_index();
            drop(tracker);
            if self.replicator.next_index > last_log_index {
                self.replicator.state = ReplicationState::UpToDate;
                break;
            }
            let response = match self.replicator
                .append_entry(self.replicator.next_index).await {
                Ok(resp) => {
                    backoff = Duration::from_millis(1);
                    resp
                },
                Err(err) => {
                    error!(cause = %err, "Caused an error: ");
                    sleep(backoff).await;
                    backoff = min(backoff * 2, self.replicator.heartbeat);
                    continue;
                }
            };
            if !response.success {
                self.replicator.state = ReplicationState::Lagged;
                break;
            }
            self.replicator.match_index = self.replicator.next_index;
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

    #[instrument(level="info", skip(self))]
    pub async fn run(&mut self) {
        info!(
            id=self.replicator.node.id.as_str(),
            "Replicator running at UpToDate state."
            );
        while self.replicator.state == ReplicationState::UpToDate {
            let timeout = sleep_until(Instant::now() + self.replicator.heartbeat);
            tokio::select! {
                _ = timeout => { 
                    let _ = self.replicator.beat().await;
                },
                Some(msg) = self.replicator.rx_repl.recv() => { 
                    let _ = self.handle_replication_msg(msg).await;
                }
            }
        }
    }

    #[instrument(level="info", skip(self))]
    pub async fn handle_replication_msg(&mut self, msg: ReplicatorMsg) 
        -> crate::Result<()> {
        match msg {
            ReplicatorMsg::ReplicateReq{ index } => {
                info!(
                    id=self.replicator.node.id.as_str(),
                    "Handling replication message."
                    );
                let response = match self.replicator.append_entry(index).await {
                    Ok(resp) => resp,
                    Err(_) => {
                        info!("Did not respond Replicator switching to Lagged");
                        self.replicator.state = ReplicationState::Lagged;
                        return Ok(());
                    }
                };
                info!("Response is {:?}", response);
                if !response.success {
                    self.replicator.state = ReplicationState::Lagged;
                    return Ok(());
                }
                self.replicator.match_index = self.replicator.next_index;
                self.replicator.next_index += 1;

                self.replicator.tx_repl.send(ReplicatorMsg::ReplicateResp {
                    match_index: self.replicator.match_index,
                    next_index: self.replicator.next_index,
                    id: self.replicator.node.id.clone()
                })?;

            },
            _ => unreachable!()
        }
        Ok(())
    }

}
