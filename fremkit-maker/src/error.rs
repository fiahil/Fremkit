use thiserror::Error;

#[derive(Debug, Error)]
pub enum FremkitError {
    #[error("Unrecoverable network error: {0}")]
    NetworkError(#[from] zmq::Error),

    #[error("ser/de error: {0}")]
    SerdeError(#[from] serde_json::Error),
}
