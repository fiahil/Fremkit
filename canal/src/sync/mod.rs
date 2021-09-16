//! This module is for synchronisation primitives.

mod notifier;
pub(crate) use notifier::*;

#[cfg(loom)]
pub(crate) use loom::{
    sync::atomic::{AtomicUsize, Ordering},
    sync::Mutex,
};

#[cfg(not(loom))]
pub(crate) use parking_lot::Mutex;

#[allow(unused_imports)]
#[cfg(not(loom))]
pub(crate) use std::sync::atomic::{AtomicUsize, Ordering};
