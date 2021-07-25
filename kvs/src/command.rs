use crate::{Frame}

#[derive(Debug)]
enum Command {
    Get()
}


#[derive(Debug)]
pub struct Get {
    key: String
}


impl Command {
    pub fn from_frame(frame: Frame) -> crate::Result<Command> {
    }
}


impl Get {
    pub fn new(key: String) -> Get {
        Get {
            key: key.to_string()
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

}


