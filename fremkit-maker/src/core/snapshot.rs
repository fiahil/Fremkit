use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::protocol::command::{Command, Message};

/// Copy of the current state of the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    data: HashMap<String, String>,
    pub version: u32,
}

impl Snapshot {
    pub fn new() -> Self {
        Snapshot {
            data: HashMap::new(),
            version: 0,
        }
    }

    pub fn checksum(&self) -> String {
        format!("v{}", self.version)
    }

    pub fn update(&mut self, command: &Command) {
        match command {
            Command::Update { key, val } => {
                self.data.insert(key.clone(), val.clone());
                self.version += 1;
            }
        }
    }

    pub fn update_msg(&mut self, message: &Message) {
        match message {
            Message::StateUpdated { key, val } => {
                self.data.insert(key.clone(), val.clone());
                self.version += 1;
            }
        }
    }
}
