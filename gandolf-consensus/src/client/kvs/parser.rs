use bytes::{Buf, BytesMut};
use std::io::Cursor;

pub use gandolf_kvs::frame::{self, Frame};

use crate::parser::Parser;

#[derive(Clone, Debug)]
pub struct KvsParser;

impl crate::ClientData for Frame {}

impl Parser<Frame> for KvsParser {
    fn parse(&self, buffer: &mut BytesMut) -> crate::Result<Option<Frame>> {
        let mut cursor = Cursor::new(&buffer[..]);
        match Frame::check(&mut cursor) {
            Ok(_) => {
                let len = cursor.position() as usize;
                cursor.set_position(0);

                let frame = Frame::parse(&mut cursor)?;
                
                buffer.advance(len);

                Ok(Some(frame))
            }

            Err(frame::Error::Incomplete) => Ok(None),

            Err(e) => Err(e.into()),
        }
    }
}
