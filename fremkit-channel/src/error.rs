use thiserror::Error;

/// Error type for Log
#[derive(Debug, Error)]
pub enum LogError<T> {
    #[error("Log is full.")]
    LogCapacityExceeded(T),
}
