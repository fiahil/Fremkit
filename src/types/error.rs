use thiserror::Error;

/// Error type for Log
#[derive(Debug, Error)]
pub enum ChannelError<T> {
    #[error("Log is full.")]
    LogCapacityExceeded(T),
}
