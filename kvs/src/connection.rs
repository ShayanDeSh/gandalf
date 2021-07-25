use bytes::{Buf, BytesMut};

use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};

use tracing::{debug, error, info, instrument};

use std::io::{Cursor};

use crate::frame::{self, Frame};

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

    pub async fn read(&mut self) -> crate::Result<Option<Frame>> {
        loop {
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err("connection reset by peer".into());
                }
            }
        }
    }

    fn parse_frame(&mut self) -> crate::Result<Option<Frame>> {
        use frame::Error::Incomplete;


        let mut cursor = Cursor::new(&self.buffer[..]);
        match Frame::check(&mut cursor) {

            Ok(_) => {
                let len = cursor.position() as usize;
                cursor.set_position(0);
                
            }
            Err(Incomplete) => Ok(None),
            Err(e) => Err(e.into())
        }


    }

}


