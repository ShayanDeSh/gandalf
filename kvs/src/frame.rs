use std::io::{Cursor};
use bytes::{Buf, Bytes};

use std::fmt;
use std::num::TryFromIntError;
use std::string::FromUtf8Error;

use std::convert::TryInto;

use atoi::atoi;


#[derive(Debug)]
pub enum Error {
    Incomplete,
    Other(crate::Error),
}


#[derive(Clone, Debug)]
pub enum Frame {
    Integer(u64),
    Simple(String),
    Error(String),
    Array(Vec<Frame>),
    Bulk(Bytes),
}


impl Frame {
    pub fn check(cursor: &mut Cursor<&[u8]>) -> Result<(), Error>{
        match get_u8(cursor)? {
            b'+' => {
                get_line(cursor)?;
                Ok(())
            }
            b'-' => {
                get_line(cursor)?;
                Ok(())
            }
            b'*' => {
                let len = get_decimal(cursor)?;
                for _ in 0..len {
                    Frame::check(cursor)?;
                }
                Ok(())
            }
            b'$' => {
                if peek_u8(cursor)? == b'-' {
                    skip(cursor, 4)
                } else {
                    let len: usize = get_decimal(cursor)?.try_into()?;
                    skip(cursor, len + 2)
                }
            }
            b':' => {
                get_decimal(cursor)?;
                Ok(())
            }
            other => Err(format!("Invalid byte `{}` for start of frame", other).into())
        }
    }

    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        match get_u8(cursor)? {
            b'+' => {
                let line = get_line(cursor)?.to_vec();
                let string = String::from_utf8(line)?;
                Ok(Frame::Simple(string))
            }
            _ => unimplemented!(),
        }
    }

}

fn skip(cursor: &mut Cursor<&[u8]>, len: usize) -> Result<(), Error> {
    if !cursor.has_remaining() {
        return Err(Error::Incomplete);
    }
    cursor.advance(len);
    Ok(())
}


fn peek_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, Error> {
    if !cursor.has_remaining() {
        return Err(Error::Incomplete);
    }
    Ok(cursor.get_ref()[0])
}


fn get_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, Error>{
    if !cursor.has_remaining() {
        return Err(Error::Incomplete);
    } 

    Ok(cursor.get_u8())
}


fn get_decimal(cursor: &mut Cursor<&[u8]>) -> Result<u64, Error> {
    let line = get_line(cursor)?;
    atoi::<u64>(line).ok_or_else(|| format!("Can not {:?} cast to u64", line).into())
}


fn get_line<'a>(cursor: &'a mut Cursor<&[u8]>) -> Result<&'a [u8], Error> {
    let start = cursor.position() as usize;
    let end = cursor.get_ref().len() - 1;

    for i in start..end {
        if cursor.get_ref()[i] == b'\r' && cursor.get_ref()[i + 1] == b'\n' {
            cursor.set_position((i + 2) as u64);

            return Ok(&cursor.get_ref()[start..i]);
        }
    }

    Err(Error::Incomplete)
}

impl From<String> for Error {
    fn from(src: String) -> Error {
        Error::Other(src.into())
    }
}

impl From<&str> for Error {
    fn from(src: &str) -> Error {
        src.to_string().into()
    }
}

impl From<FromUtf8Error> for Error {
    fn from(_src: FromUtf8Error) -> Error {
        "protocol error; invalid frame format".into()
    }
}

impl From<TryFromIntError> for Error {
    fn from(_src: TryFromIntError) -> Error {
        "protocol error; invalid frame format".into()
    }
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Incomplete => "stream ended early".fmt(fmt),
            Error::Other(err) => err.fmt(fmt),
        }
    }
}
