use gandolf_kvs::{Frame, Connection};

use bytes::Bytes;

use crate::Tracker;
use crate::tracker::{Index, Term};

use tokio::net::TcpStream;
use std::net::SocketAddr;

pub struct Cell(Term, Frame);

pub struct KvsTracker {
    log: Vec<Cell>,
    last_log_index: Index,
    last_log_term: Term,
    last_commited_index: Index,
    addr: SocketAddr
}

impl KvsTracker {
    pub fn new(addr: SocketAddr) -> KvsTracker {
        KvsTracker {
            log: Vec::new(),
            last_log_index: 0,
            last_log_term: 0,
            last_commited_index: 0,
            addr
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
        &self.log[index].1
    }

    fn get_log_term(&self, index: Index) -> Term {
        self.log[index].0
    }

    fn append_log(&mut self, entity: Self::Entity, term: Term) -> crate::Result<Index> {
        if term > self.last_log_term {
            self.last_log_term = term;
        }
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

    async fn commit(&mut self, index: Index) -> crate::Result<Self::Entity> {
        if index != self.last_commited_index + 1 {
            return Err("Wrong commit index".into());
        }

        let response = self.propagate(&self.log[index].1).await?;

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
