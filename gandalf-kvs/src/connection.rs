use bytes::{Buf, BytesMut};

use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use async_recursion::async_recursion;

use tracing::debug;

use std::io::{self, Cursor};

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

                let frame = Frame::parse(&mut cursor)?;
                
                self.buffer.advance(len);

                debug!("Frame Recived: {:?}", frame);
                Ok(Some(frame))
            }

            Err(Incomplete) => Ok(None),

            Err(e) => Err(e.into()),
        }
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        match frame {
            Frame::Array(val) => {
                self.stream.write_u8(b'*').await?;
                self.write_decimal(val.len() as u64).await?;
                for entity in &*val {
                    self.write_value(entity).await?;
                }
            }
            _ => self.write_value(&frame).await?
        }
        self.stream.flush().await
    }


    #[async_recursion]
    async fn write_value(&'async_recursion mut self, frame: &Frame) -> io::Result<()> {
        match frame {
            Frame::Simple(val) => {
                self.stream.write_u8(b'+').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Error(val) => {
                self.stream.write_u8(b'-').await?;
                self.stream.write_all(val.as_bytes()).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Integer(val) => {
                self.stream.write_u8(b':').await?;
                self.write_decimal(*val).await?;
            }
            Frame::Null => {
                self.stream.write_all(b"$-1\r\n").await?;
            }
            Frame::Bulk(val) => {
                let len = val.len();

                self.stream.write_u8(b'$').await?;
                self.write_decimal(len as u64).await?;
                self.stream.write_all(val).await?;
                self.stream.write_all(b"\r\n").await?;
            }
            Frame::Array(val) => {
                self.stream.write_u8(b'*').await?;
                self.write_decimal(val.len() as u64).await?;
                for entity in &*val {
                    self.write_value(entity).await?;
                }
            },
        }
        Ok(())
    }

    async fn write_decimal(&mut self, val: u64) -> io::Result<()> {
        use std::io::Write;

        let mut buf = [0u8; 20];
        let mut buf = Cursor::new(&mut buf[..]);

        write!(&mut buf, "{}", val)?;

        let pos = buf.position() as usize;
        self.stream.write_all(&buf.get_ref()[..pos]).await?;
        self.stream.write_all(b"\r\n").await?;

        Ok(())
    }

}


