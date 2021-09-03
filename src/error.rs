use thiserror::Error;

/// Error type for the Aqueduc library.
#[derive(Debug, Error)]
pub enum AqueducError {
    #[error("Canal {0} does not exist")]
    CanalDoesNotExist(String),
}
