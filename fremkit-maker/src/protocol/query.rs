use crate::core::snapshot::Snapshot;
use crate::core::state::State;
use crate::error::FremkitError;

use anyhow::Result;
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
    pub fn apply(self, state: &State) -> Answer {
        match self {
            Query::Snapshot => Answer::Snapshot(Snapshot::from(state)),
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
    type Error = FremkitError;

    fn try_from(buffer: Vec<u8>) -> Result<Self, FremkitError> {
        serde_json::from_slice(&buffer).map_err(|e| FremkitError::from(e))
    }
}

impl TryInto<Vec<u8>> for Query {
    type Error = FremkitError;

    fn try_into(self) -> Result<Vec<u8>, FremkitError> {
        serde_json::to_vec(&self).map_err(|e| FremkitError::from(e))
    }
}

impl TryFrom<Vec<u8>> for Answer {
    type Error = FremkitError;

    fn try_from(buffer: Vec<u8>) -> Result<Self, FremkitError> {
        serde_json::from_slice(&buffer).map_err(|e| FremkitError::from(e))
    }
}

impl TryInto<Vec<u8>> for Answer {
    type Error = FremkitError;

    fn try_into(self) -> Result<Vec<u8>, FremkitError> {
        serde_json::to_vec(&self).map_err(|e| FremkitError::from(e))
    }
}
