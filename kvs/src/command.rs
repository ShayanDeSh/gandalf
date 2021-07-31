use crate::{Frame, Parse};

use bytes::Bytes;

use tracing::{debug, error, info, instrument};

#[derive(Debug)]
pub enum Command {
    Get(Get),
    Set(Set)
}


#[derive(Debug)]
pub struct Get {
    key: String,
}

#[derive(Debug)]
pub struct Set {
    key: String,
    value: Bytes,
}

impl Command {
    pub fn from_frame(frame: Frame) -> crate::Result<Command> {
        let mut parse = Parse::new(frame)?;

        let cmd_name = parse.next_string()?.to_lowercase();

        debug!("cmd name is {:?}", cmd_name);

        let cmd = match &cmd_name[..] {
            "get" => Command::Get(Get::from_parse(&mut parse)?),
            "set" => Command::Set(Set::from_parse(&mut parse)?),
            _ => {
                return Err("Could not parse the command".into())
            }
        };

        parse.finish()?;
        return Ok(cmd);
    }
}


impl Get {
    pub fn new(key: String) -> Get {
        Get {
            key: key.to_string()
        }
    }

    pub fn from_parse(parse: &mut Parse) -> crate::Result<Get> {
        let key = parse.next_string()?;
        Ok(Get {
            key: key
        })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

}


impl Set {
    pub fn from_parse(parse: &mut Parse) -> crate::Result<Set> {
        let key = parse.next_string()?;
        let value = parse.next_bytes()?;
        Ok(Set {
            key: key,
            value: value
        })
    }
}


