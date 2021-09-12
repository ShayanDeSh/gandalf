use crate::{Frame, Parse, Db, Connection};

use bytes::Bytes;

use tracing::debug;

#[derive(Debug)]
pub enum Command {
    Get(Get),
    Set(Set),
    Load(Load),
    Snap(Snap)
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

#[derive(Debug)]
pub struct Load {
    elements: Vec<Set>
}

#[derive(Debug)]
pub struct Snap;

impl Command {
    pub fn from_frame(frame: Frame) -> crate::Result<Command> {
        let mut parse = Parse::new(frame)?;

        let cmd_name = parse.next_string()?.to_lowercase();

        debug!("cmd name is {:?}", cmd_name);

        let cmd = match &cmd_name[..] {
            "get" => Command::Get(Get::from_parse(&mut parse)?),
            "set" => Command::Set(Set::from_parse(&mut parse)?),
            "load" => Command::Load(Load::from_parse(&mut parse)?),
            "snap" => Command::Snap(Snap),
            _ => {
                return Err("Could not parse the command".into())
            }
        };

        parse.finish()?;
        return Ok(cmd);
    }


    pub async fn apply(self, db: &Db, con: &mut Connection) -> crate::Result<()>{
        match self {
            Command::Get(cmd) => cmd.apply(db, con).await,
            Command::Set(cmd) => cmd.apply(db, con).await,
            Command::Load(cmd) => cmd.apply(db, con).await,
            Command::Snap(cmd) => cmd.apply(db, con).await,
        }
    }
}

impl Load {
    pub fn from_parse(parse: &mut Parse) -> crate::Result<Load> {
        debug!("parse is {:?}", parse);
        let mut elements = Vec::new();
        let mut els = parse.next_array()?;
        while let Ok(mut p) = els.next_array() {
            let set = Set::from_parse(&mut p)?;
            elements.push(set);
        }
        Ok(Load {
            elements
        })
    }

    pub async fn apply(self, db: &Db, con: &mut Connection) -> crate::Result<()> {
        for set in self.elements.into_iter() {
            db.set(set.key, set.value);
        }
        let response = Frame::Simple("OK".into());
        con.write_frame(&response).await?;

        Ok(())
    }
}

impl Snap {
    pub async fn apply(self, db: &Db, con: &mut Connection) -> crate::Result<()> {
        let kv = db.snap();
        let mut load = Vec::new();
        let mut elements = Vec::new();

        //Array([Simple("load"), Array([Array([Simple("a"), Bulk(b"asdfq")]), Array([Simple("b"), Bulk(b"asd")])])] 
        for (k, v) in kv.into_iter() {
            let el = vec![Frame::Simple(k), Frame::Bulk(v.data)];
            elements.push(Frame::Array(el));
        }

        load.push(Frame::Simple("load".to_string()));
        load.push(Frame::Array(elements));

        con.write_frame(&Frame::Array(load)).await?;

        Ok(())
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

    pub async fn apply(self, db: &Db, con: &mut Connection) -> crate::Result<()> {
        let response = if let Some(value) = db.get(&self.key) {
            Frame::Bulk(value)
        } else {
            Frame::Null
        };

        debug!(?response);
        con.write_frame(&response).await?;

        Ok(())
    }

    pub fn into_frame(self) -> Frame {
        let name = Frame::Bulk(Bytes::from("get".as_bytes()));
        let key = Frame::Bulk(Bytes::from(self.key.into_bytes()));
        Frame::Array(vec![name, key])
    }

}


impl Set {
    pub fn new(key: impl ToString, value: Bytes) -> Set {
        Set {
            key: key.to_string(),
            value: value
        }
    }

    pub fn from_parse(parse: &mut Parse) -> crate::Result<Set> {
        let key = parse.next_string()?;
        let value = parse.next_bytes()?;
        Ok(Set {
            key: key,
            value: value
        })
    }

    pub async fn apply(self, db: &Db, con: &mut Connection) -> crate::Result<()>{
        db.set(self.key, self.value);
        let response = Frame::Simple("OK".into());

        debug!(?response);
        con.write_frame(&response).await?;

        Ok(())
    }

    pub fn into_frame(self) -> Frame {
        let name = Frame::Bulk(Bytes::from("set".as_bytes()));
        let key = Frame::Bulk(Bytes::from(self.key.into_bytes()));
        let value = Frame::Bulk(Bytes::from(self.value));
        Frame::Array(vec![name, key, value])
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &Bytes {
        &self.value
    }
}
