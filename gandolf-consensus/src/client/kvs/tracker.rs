use gandolf_kvs::{Frame, Connection};

use crate::Tracker;
use crate::tracker::{Index, Term};

use tokio::net::TcpStream;
use std::net::SocketAddr;

use std::fs::OpenOptions;

use tracing::info;

pub struct Cell(Term, Frame);

pub struct KvsTracker {
    log: Vec<Cell>,
    last_log_index: Index,
    snapshot_no: u64,
    snapshot_offset: u64,
    last_log_term: Term,
    last_commited_index: Index,
    addr: SocketAddr,
    snapshot_path: String,
    last_snapshot_term: Term,
}

impl KvsTracker {
    pub fn new(addr: SocketAddr, snapshot_path: String, snapshot_offset: u64) -> KvsTracker {
        KvsTracker {
            log: Vec::new(),
            last_log_index: 0,
            last_log_term: 0,
            last_commited_index: 0,
            snapshot_no: 0,
            snapshot_offset,
            addr,
            snapshot_path,
            last_snapshot_term: 0,
        }
    }
}

#[tonic::async_trait]
impl Tracker for KvsTracker {
    type Entity = Frame;

    async fn propagate(&self, entity: &Self::Entity) -> crate::Result<Self::Entity> {
        let socket = TcpStream::connect(self.addr).await?;
        let mut connection = Connection::new(socket);
        connection.write_frame(entity).await?;

        let response = read_response(&mut connection).await?;

        Ok(response)
    }

    fn get_last_log_index(&self) -> Index {
        self.last_log_index
    }

    fn get_last_log_term(&self) -> Term {
        self.last_log_term
    }

    fn get_last_commited_index(&self) -> Index {
        self.last_commited_index
    } 

    fn get_log_entity(&self, index: Index) -> &Self::Entity {
        let i = index - 1 - (self.snapshot_no as u64 * self.snapshot_offset as u64);
        &self.log[i as usize].1
    }

    fn get_log_term(&self, index: Index) -> Term {
        let i = index - 1 - (self.snapshot_no * self.snapshot_offset);
        if i as i64 == -1 {
            return self.last_snapshot_term;
        }
        info!("index is {}", i as i64);
        self.log[i as usize].0
    }

    fn append_log(&mut self, entity: Self::Entity, term: Term) -> crate::Result<Index> {
        self.last_log_term = term;
        self.log.push(Cell(term, entity));
        self.last_log_index += 1;
        Ok(self.last_log_index)
    }

    fn delete_last_log(&mut self) -> crate::Result<()> {
        if let None = self.log.pop() {
            return Err("The log is empty".into());
        }
        Ok(())
    }

    async fn take_snapshot(&mut self) -> crate::Result<()> {
        let socket = TcpStream::connect(self.addr).await?;
        let mut connection = Connection::new(socket);
        let snap = Frame::Array(vec![Frame::Simple("snap".to_string())]);
        connection.write_frame(&snap).await?;

        let response = read_response(&mut connection).await?;

        match response {
            frame => {
                let file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(format!("{}/{}.ga", self.snapshot_path, self.snapshot_no))?;
                serde_json::to_writer(file, &frame)?;
            }
        }

        self.snapshot_no += 1;
        self.last_snapshot_term = self.get_last_log_term();
        self.log.clear();

        Ok(())
    }

    async fn load_snappshot(&mut self, entity: &Self::Entity, len: u64, last_log_term: Term) -> crate::Result<()> {
        let socket = TcpStream::connect(self.addr).await?;
        let mut connection = Connection::new(socket);
        connection.write_frame(entity).await?;

        let response = read_response(&mut connection).await?;

        match response {
            Frame::Simple(_) => {
                let file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(format!("{}.ga", self.snapshot_no))?;
                serde_json::to_writer(file, entity)?;
                self.snapshot_no += 1;
                self.last_log_index = self.snapshot_no * len;
                self.last_log_term = last_log_term;
                self.last_snapshot_term = last_log_term;
                self.log.clear();
            },
            _ => unreachable!()
        }
        Ok(())
    }

    async fn commit(&mut self, index: Index) -> crate::Result<Self::Entity> {
        if index + 1 != self.last_commited_index + 1 {
            return Err("Wrong commit index".into());
        }

        let response = self.propagate(&self.log[index as usize].1).await?;

        match response {
            Frame::Simple(_) => {
                self.last_commited_index += 1;
                Ok(response)
            },
            Frame::Bulk(_) =>{
                self.last_commited_index += 1;
                Ok(response)
            },
            frame => Err(format!("{:?}", frame).into()),
        }
    }
}

async fn read_response(connection: &mut Connection) -> crate::Result<Frame> {
    let response = connection.read().await?;

    match response {
        Some(Frame::Error(msg)) => Err(msg.into()),
        Some(frame) => Ok(frame),
        None => {
            return Err("Connection closed by the peer".into());
        }
    }
}
