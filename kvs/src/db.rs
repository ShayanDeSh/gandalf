use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use bytes::Bytes;
use uuid::Uuid;


pub struct Db {
    shared: Arc<Shared>,
}

pub struct Shared {
    state: Mutex<State>,
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
                state: Mutex::new(State {
                    kv: HashMap::new()
                })
            })
        }
    }

    pub fn set(&self, key: String, value: Bytes) {
        let id = Uuid::new_v4();
        let mut state = self.shared.state.lock().unwrap();

        let entity = Entity {
            id: id,
            data: value
        };

        state.kv.insert(key, entity);

        drop(state);
    }

    pub fn get(&self, key: &str) -> Option<Bytes> {
        let state = self.shared.state.lock().unwrap();
        state.kv.get(key).map(|entity| entity.data.clone())
    }

}

