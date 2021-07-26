use std::{vec, fmt, str};
use crate::Frame;
use bytes::Bytes;


#[derive(Debug)]
pub struct Parse {
    parts: vec::IntoIter<Frame>,
}

#[derive(Debug)]
pub enum ParseError {
    EndOfStream,
    Other(crate::Error)
}

impl Parse {
    pub fn new(frame: Frame) -> Result<Parse, ParseError> {
        let array = match frame {
            Frame::Array(array) => array,
            frame => return Err(format!("Wrong frame format,\
                    only can accept arrays but recived {:?}", frame).
                into()),
        };
        Ok(Parse {
            parts: array.into_iter()
        })
    }

    pub fn next(&mut self) -> Result<Frame, ParseError> {
        self.parts.next().ok_or(ParseError::EndOfStream)
    }

    pub fn next_string(&mut self) -> Result<String, ParseError> {
        let frame = self.next()?;
        match frame {
            Frame::Simple(s) => Ok(s),
            Frame::Bulk(data) => str::from_utf8(&data[..])
                .map(|s| s.to_string())
                .map_err(|_| "could not parse Bulk frame".into()),
            frame => Err(format!("Expected bulk or array, recived {:?}",frame).
                into()),
        }
    }

    pub fn next_integer(&mut self) -> Result<u64, ParseError> {
        let frame = self.next()?;
        match frame {
            Frame::Integer(num) => Ok(num),
            frame => Err(format!("protocol error; expected int frame but got {:?}", frame).into()),
        }
    }

    pub fn next_bytes(&mut self) -> Result<Bytes, ParseError> {
        let frame = self.next()?;
        match frame {
            Frame::Simple(s) => Ok(Bytes::from(s.into_bytes())),
            Frame::Bulk(data) => Ok(data),
            frame => Err(format!(
                "protocol error; expected simple frame or bulk frame, got {:?}",
                frame
            )
            .into()),
        }
    }

    pub fn finish(&mut self) -> Result<(), ParseError> {
        if self.parts.next().is_none() {
            Ok(())
        } else {
            Err("Expected end of array but got more".into())
        }
    }

}

impl From<String> for ParseError {
    fn from(src: String) -> ParseError {
        ParseError::Other(src.into())
    }
}

impl From<&str> for ParseError {
    fn from(src: &str) -> ParseError {
        ParseError::Other(src.into())
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::EndOfStream => "End of stream".fmt(fmt),
            ParseError::Other(e) => e.fmt(fmt)
        }
    }
}

impl std::error::Error for ParseError {}


