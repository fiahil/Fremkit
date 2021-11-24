use serde::{Deserialize, Serialize};

/// Messages broadcasted from the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Heartbeat,
}
