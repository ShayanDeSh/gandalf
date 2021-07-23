use bytes::BytesMut;

use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};

use tracing::{debug, error, info, instrument};

#[derive(Debug)]
pub struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut
}

impl Connection {
    pub fn new(socket: TcpStream) -> Connection {
        Connection {
            stream: BufWriter::new(socket),
            buffer: BytesMut::with_capacity(4096)
        }
    }

    pub async fn read(&mut self) {
        self.stream.read_buf(&mut self.buffer).await;
        info!("Read {:?}", self.buffer)
    }
}


