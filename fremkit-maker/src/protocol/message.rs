use serde::{Deserialize, Serialize};

use crate::error::FremkitError;

/// Messages broadcasted from the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Heartbeat,
}

impl TryFrom<Vec<u8>> for Message {
    type Error = FremkitError;

    fn try_from(buffer: Vec<u8>) -> Result<Self, FremkitError> {
        serde_json::from_slice(&buffer).map_err(|e| FremkitError::from(e))
    }
}

impl TryInto<Vec<u8>> for Message {
    type Error = FremkitError;

    fn try_into(self) -> Result<Vec<u8>, FremkitError> {
        serde_json::to_vec(&self).map_err(|e| FremkitError::from(e))
    }
}
