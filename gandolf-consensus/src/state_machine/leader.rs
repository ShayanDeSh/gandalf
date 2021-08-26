use crate::{Raft, ClientData, Tracker, RaftMessage};
use crate::raft::State;
use tracing::{instrument, error, debug};
use tokio::time::{Instant, sleep_until};
use crate::raft_rpc::AppendEntriesRequest;
use crate::rpc;

#[derive(Debug)]
pub struct Leader <'a, T: ClientData, R: Tracker<Entity=T>> {
    raft: &'a mut Raft<T, R>
}

impl<'a, T: ClientData, R: Tracker<Entity=T>> Leader<'a, T, R> {
    pub fn new(raft:&'a mut Raft<T, R>) -> Leader<T, R> {
        Leader { raft }
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
           RaftMessage::ClientReadMsg{body, tx} => {
               let response = self.raft.tracker.propagate(&body).await?;
               if let Err(_) = tx.send(RaftMessage::ClientResp { body: response }) {
                   error!("Peer drop the client response");
               }
           },
           RaftMessage::ClientWriteMsg {body, tx} => {
               let index = self.raft.tracker.append_log(body, self.raft.last_log_term)?;
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
        let nodes = self.raft.get_all_nodes().into_iter();
        for node in nodes {
            let request = AppendEntriesRequest {
                term: self.raft.current_term,
                leader_id: self.raft.id.to_string(),
                prev_log_index: self.raft.tracker.get_last_log_index(),
                prev_log_term: self.raft.tracker.get_last_log_term(),
                entries: vec![]
            };
            tokio::spawn(async move {
                let response = rpc::append_entries(&node, request).await;
                match response {
                    // TODO:
                    // The response must be handled
                    Ok(resp) => debug!("recived {:?} from {:?}", resp, node),
                    Err(err) => error!(cause = %err, "Caused an error: ")
                }
            });
        }
        Ok(())
    }
}
