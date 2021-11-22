use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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

    pub fn update(&mut self, key: String, val: String) {
        self.data.insert(key, val);
        self.version += 1;
    }
}
