use std::collections::HashMap;

use fremkit_channel::unbounded::Channel;

use crate::protocol::command::{Command, Message};

use super::snapshot::Snapshot;

/// Copy of the current state of the system
#[derive(Debug, Clone)]
pub struct State {
    data: HashMap<String, Channel<String>>,
    pub version: u32,
}

impl State {
    pub fn new() -> Self {
        State {
            data: HashMap::new(),
            version: 0,
        }
    }

    pub fn checksum(&self) -> String {
        format!("v{}", self.version)
    }

    pub fn update_com(&mut self, command: Command) {
        match command {
            Command::Update { key, val } => {
                self.data
                    .entry(key)
                    .and_modify(|c| {
                        c.push(val);
                    })
                    .or_default();

                self.version += 1;
            }
        }
    }

    pub fn update_msg(&mut self, message: Message) {
        match message {
            Message::StateUpdated { key, val } => {
                self.data
                    .entry(key)
                    .and_modify(|c| {
                        c.push(val);
                    })
                    .or_default();

                self.version += 1;
            }
        }
    }
}

impl From<Snapshot> for State {
    fn from(snapshot: Snapshot) -> Self {
        State {
            data: snapshot
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
            version: snapshot.version,
        }
    }
}

impl From<&State> for Snapshot {
    fn from(state: &State) -> Self {
        Snapshot {
            data: state
                .data
                .iter()
                .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
                .collect(),
            version: state.version,
        }
    }
}
