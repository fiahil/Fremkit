//! Fremkit is simple broadcast log.
//!
//! It provides `Log`, a simple, fast, and thread-safe log.
//!
//! A Log's primary use case is to store an immutable sequence of messages, events, or other data, and to allow
//! multiple readers to access the data concurrently.

mod log;
mod sync;

pub use crate::log::bounded;
pub use crate::log::error::LogError;
