use std::{vec, fmt};
use crate::Frame;

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
    pub fn next(&mut self) -> Result<Frame, ParseError> {
        self.parts.next().ok_or(ParseError::EndOfStream)
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


