use crate::core::snapshot::Snapshot;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A query sent from the client to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Query {
    Snapshot,
    Checksum(String),
}

/// A answer sent to a query, from the server to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Answer {
    Snapshot(Snapshot),
    ChecksumOk,
    ChecksumFailed,
}

impl Query {
    /// Create a corresponding answer to a query.
    pub fn apply(self, state: &Snapshot) -> Answer {
        match self {
            Query::Snapshot => Answer::Snapshot(state.clone()),
            Query::Checksum(c) => {
                if state.checksum() == c {
                    Answer::ChecksumOk
                } else {
                    Answer::ChecksumFailed
                }
            }
        }
    }
}

impl TryFrom<Vec<u8>> for Query {
    type Error = anyhow::Error;

    fn try_from(buffer: Vec<u8>) -> Result<Self> {
        serde_json::from_slice(&buffer).with_context(|| "could not deserialize buffer (query)")
    }
}

impl TryFrom<Vec<u8>> for Answer {
    type Error = anyhow::Error;

    fn try_from(buffer: Vec<u8>) -> Result<Self> {
        serde_json::from_slice(&buffer).with_context(|| "could not deserialize buffer (answer)")
    }
}

impl TryInto<Vec<u8>> for Answer {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Vec<u8>> {
        serde_json::to_vec(&self).with_context(|| "could not serialize response")
    }
}
