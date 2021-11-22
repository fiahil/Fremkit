use crate::core::snapshot::{self, Snapshot};

use anyhow::{Context, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Snapshot(SnapshotCommand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Snapshot(SnapshotResponse),
}

/// Ask for a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotCommand {
    Get,
    Checksum(String),
}

/// Response to a snapshot command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotResponse {
    ChecksumOK,
    ChecksumKO,
    Snapshot(Snapshot),
}

impl Command {
    /// Create a corresponding response to a command.
    pub fn apply(self, state: &Snapshot) -> Response {
        match self {
            Command::Snapshot(SnapshotCommand::Get) => {
                Response::Snapshot(SnapshotResponse::Snapshot(state.clone()))
            }
            Command::Snapshot(SnapshotCommand::Checksum(checksum)) => {
                if checksum == state.checksum() {
                    Response::Snapshot(SnapshotResponse::ChecksumOK)
                } else {
                    Response::Snapshot(SnapshotResponse::ChecksumKO)
                }
            }
        }
    }
}

impl TryFrom<Bytes> for Command {
    type Error = anyhow::Error;

    fn try_from(buffer: Bytes) -> Result<Self> {
        serde_json::from_slice(&buffer).with_context(|| "could not deserialize buffer")
    }
}

impl TryInto<Bytes> for Response {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Bytes> {
        serde_json::to_vec(&self)
            .map(Bytes::from)
            .with_context(|| "could not serialize response")
    }
}
