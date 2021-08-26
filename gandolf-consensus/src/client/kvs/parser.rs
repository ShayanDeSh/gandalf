use bytes::{Buf, BytesMut};
use std::io::Cursor;

use gandolf_kvs::frame::{self, Frame};
use gandolf_kvs::Command;

use crate::parser::{Parser, Kind};

#[derive(Clone, Debug)]
pub struct KvsParser;

impl crate::ClientData for Frame {}

impl Parser<Frame> for KvsParser {
    fn parse(&self, buffer: &mut BytesMut) -> crate::Result<Option<Kind<Frame>>> {
        let mut cursor = Cursor::new(&buffer[..]);
        match Frame::check(&mut cursor) {
            Ok(_) => {
                let len = cursor.position() as usize;
                cursor.set_position(0);

                let frame = Frame::parse(&mut cursor)?;
                
                buffer.advance(len);

                match Command::from_frame(frame.clone())? {
                    Command::Get(_) => return Ok(Some(Kind::Read(frame))),
                    Command::Set(_) => return Ok(Some(Kind::Write(frame)))
                }

            }

            Err(frame::Error::Incomplete) => Ok(None),

            Err(e) => Err(e.into()),
        }
    }
}
