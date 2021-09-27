//! This module is for synchronisation primitives imports.

#[allow(unused_imports)]
#[cfg(not(loom))]
pub(crate) use std::{
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
    thread,
};

#[allow(unused_imports)]
#[cfg(loom)]
pub(crate) use loom::{
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
    thread,
};
