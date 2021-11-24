use serde::{Deserialize, Serialize};

use crate::{core::state::State, error::FremkitError};

/// A command sent from the client to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Put { key: String, val: String },
    Get { key: String, idx: usize },
}

/// A response sent to a command, from the server to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Artefact { key: String, val: String },
    NewArtefact { key: String, idx: usize },
    MissingArtefact { key: String, idx: usize },
}

impl Command {
    pub fn apply(self, state: &State) -> Response {
        match self {
            Command::Put { key, val } => {
                let idx = state.put(&key, val);

                Response::NewArtefact { key, idx }
            }

            Command::Get { key, idx } => {
                let val = state.get(&key, idx);

                match val {
                    Some(val) => Response::Artefact {
                        key,
                        val: val.clone(),
                    },
                    None => Response::MissingArtefact { key, idx },
                }
            }
        }
    }
}

impl TryFrom<Vec<u8>> for Command {
    type Error = FremkitError;

    fn try_from(buffer: Vec<u8>) -> Result<Self, FremkitError> {
        serde_json::from_slice(&buffer).map_err(|e| FremkitError::from(e))
    }
}

impl TryInto<Vec<u8>> for Command {
    type Error = FremkitError;

    fn try_into(self) -> Result<Vec<u8>, FremkitError> {
        serde_json::to_vec(&self).map_err(|e| FremkitError::from(e))
    }
}

impl TryFrom<Vec<u8>> for Response {
    type Error = FremkitError;

    fn try_from(buffer: Vec<u8>) -> Result<Self, FremkitError> {
        serde_json::from_slice(&buffer).map_err(|e| FremkitError::from(e))
    }
}

impl TryInto<Vec<u8>> for Response {
    type Error = FremkitError;

    fn try_into(self) -> Result<Vec<u8>, FremkitError> {
        serde_json::to_vec(&self).map_err(|e| FremkitError::from(e))
    }
}
