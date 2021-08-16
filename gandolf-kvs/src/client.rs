use crate::{Connection, Frame};
use crate::command::{Get, Set};
use tokio::net::{TcpStream, ToSocketAddrs};

use bytes::Bytes;

pub struct Client {
    connection: Connection,
}


pub async fn connect<T: ToSocketAddrs>(addr: T) -> crate::Result<Client> {
    let socket = TcpStream::connect(addr).await?;

    let connection = Connection::new(socket);

    Ok(Client { connection })
}


impl Client {
    pub async fn get(&mut self, key: &str) -> crate::Result<Option<Bytes>>{
        let frame = Get::new(key.to_string()).into_frame();
        self.connection.write_frame(&frame).await?;

        let resp = self.read_response().await?;

        match resp {
            Frame::Simple(value) => Ok(Some(value.into())),
            Frame::Bulk(value) => Ok(Some(value)),
            Frame::Null => Ok(None),
            frame => Err(format!("{:?}", frame).into()),
        }
    }

    pub async fn set(&mut self, key: &str, value: Bytes) -> crate::Result<()> {
        let frame = Set::new(key.to_string(), value).into_frame();
        self.connection.write_frame(&frame).await?;
        match self.read_response().await? {
            Frame::Simple(response) if response == "OK" => Ok(()),
            frame => Err(format!("{:?}", frame).into()),
        }
    }

    async fn read_response(&mut self) -> crate::Result<Frame> {
        let response = self.connection.read().await?;

        match response {
            Some(Frame::Error(msg)) => Err(msg.into()),
            Some(frame) => Ok(frame),
            None => {
                return Err("Connection closed by the peer".into());
            }
        }
    }
}
