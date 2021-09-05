mod canal;
mod sync;

pub use crate::canal::Canal;
pub use crate::error::CanalError;

mod error {
    use thiserror::Error;

    /// Error type for the Canal library.
    #[derive(Debug, Error)]
    pub enum CanalError {
        #[error("Cannot add to a `closed` Canal")]
        CanalClosed,
    }
}
