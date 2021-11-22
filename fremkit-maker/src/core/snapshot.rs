use serde::{Deserialize, Serialize};

/// Copy of the current state of the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
}

impl Snapshot {
    pub fn new() -> Self {
        Snapshot { version: 0 }
    }

    pub fn checksum(&self) -> String {
        format!("v{}", self.version)
    }

    pub fn increment(&mut self) {
        self.version += 1;
    }
}
