use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use fremkit_channel::unbounded::Channel;

use super::snapshot::Snapshot;

/// Copy of the current state of the system
#[derive(Debug)]
pub struct State {
    data: Arc<Mutex<HashMap<String, Channel<String>>>>,
    version: Arc<Mutex<u32>>,
}

impl State {
    pub fn new() -> Self {
        State {
            data: Arc::new(Mutex::new(HashMap::new())),
            version: Arc::new(Mutex::new(0)),
        }
    }

    pub fn checksum(&self) -> String {
        format!("v{}", self.version.lock().unwrap())
    }

    /// put a new value into a channel
    pub fn put(&self, key: &String, val: String) -> usize {
        let mut lock = self.data.lock().unwrap();
        let channel = lock.entry(key.clone()).or_default();

        let idx = channel.push(val);

        *self.version.lock().unwrap() += 1;

        idx
    }

    /// get a value from a channel
    pub fn get(&self, key: &String, idx: usize) -> Option<String> {
        self.data
            .lock()
            .unwrap()
            .get(key)
            .and_then(|channel| channel.get(idx).cloned())
    }
}

impl From<Snapshot> for State {
    fn from(snapshot: Snapshot) -> Self {
        State {
            data: Arc::new(Mutex::new(
                snapshot
                    .data
                    .into_iter()
                    .map(|(k, vs)| {
                        // TODO: Maybe improve ergonomics for channel to vec and vice-versa
                        let channel = Channel::new();

                        for v in vs {
                            channel.push(v);
                        }

                        (k, channel)
                    })
                    .collect(),
            )),
            version: Arc::new(Mutex::new(snapshot.version)),
        }
    }
}

impl From<&State> for Snapshot {
    fn from(state: &State) -> Self {
        Snapshot {
            data: state
                .data
                .lock()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
                .collect(),
            version: *state.version.lock().unwrap(),
        }
    }
}
