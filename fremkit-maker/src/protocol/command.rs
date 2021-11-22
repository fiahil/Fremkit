use crate::core::snapshot::Snapshot;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A command sent from the client to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Snapshot,
    Checksum(String),
}

/// A response sent as an answer to a command, from the server to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Snapshot(Snapshot),
    ChecksumOk,
    ChecksumFailed,
}

impl Command {
    /// Create a corresponding response to a command.
    pub fn apply(self, state: &Snapshot) -> Response {
        match self {
            Command::Snapshot => Response::Snapshot(state.clone()),
            Command::Checksum(c) => {
                if state.checksum() == c {
                    Response::ChecksumOk
                } else {
                    Response::ChecksumFailed
                }
            }
        }
    }
}

impl TryFrom<Vec<u8>> for Command {
    type Error = anyhow::Error;

    fn try_from(buffer: Vec<u8>) -> Result<Self> {
        serde_json::from_slice(&buffer).with_context(|| "could not deserialize buffer")
    }
}

impl TryInto<Vec<u8>> for Response {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Vec<u8>> {
        serde_json::to_vec(&self).with_context(|| "could not serialize response")
    }
}
