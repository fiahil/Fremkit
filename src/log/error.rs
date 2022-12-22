use thiserror::Error;

/// Error type for Log
#[derive(Debug, Error)]
pub enum LogError<T> {
    /// Log is full. Push operation are not allowed anymore.
    #[error("Log is full.")]
    LogCapacityExceeded(T),
}
