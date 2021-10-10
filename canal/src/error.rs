use thiserror::Error;

/// Error type for Log
#[derive(Debug, Error)]
pub enum LogError {
    #[error("Log is full.")]
    LogCapacityExceeded,
}
