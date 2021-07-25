use std::io::{Cursor};
use bytes::{Buf};

use std::fmt;
use std::num::TryFromIntError;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum Error {
    Incomplete,
    Other(crate::Error),
}


#[derive(Clone, Debug)]
pub enum Frame {
    Integer(u64)
}


impl Frame {
    pub fn check(cursor: &mut Cursor<&[u8]>) -> Result<(), Error>{
        match get_u8(cursor)? {
            b'+' => {
                get_line(cursor)?;
                Ok(())
            }
            other => Err(format!("Invalid byte `{}` for start of frame", other).into())
        }
    }

}


fn get_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, Error>{
    if !cursor.has_remaining() {
        return Err(Error::Incomplete);
    } 

    Ok(cursor.get_u8())
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
