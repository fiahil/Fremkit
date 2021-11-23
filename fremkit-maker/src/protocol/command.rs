use serde::{Deserialize, Serialize};

/// A command sent from the client to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Update { key: String, val: String },
}

/// A response sent as an answer to a command, from the server to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    StateUpdated { key: String, val: String },
}
