use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use bytes::Bytes;
use uuid::Uuid;


pub struct Db {
    shared: Arc<Shared>,
}

pub struct Shared {
    state: RwLock<State>,
}

pub struct State {
    kv: HashMap<String, Entity>,
}

pub struct Entity {
    id: Uuid,
    data: Bytes,
}

impl Db {
    pub fn new() -> Db {
        Db {
            shared: Arc::new(Shared {
                state: RwLock::new(State {
                    kv: HashMap::new()
                })
            })
        }
    }

}

