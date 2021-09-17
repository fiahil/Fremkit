//! This module is for synchronisation primitives.

mod notifier;
pub(crate) use notifier::*;

#[cfg(not(loom))]
pub(crate) use parking_lot::{Condvar, Mutex};

#[allow(unused_imports)]
#[cfg(not(loom))]
pub(crate) use std::{
    sync::atomic::{AtomicUsize, Ordering},
    thread,
};

#[cfg(loom)]
pub(crate) use loom::{
    sync::atomic::{AtomicUsize, Ordering},
    sync::{Condvar, Mutex as OldMutex, MutexGuard as OldMutexGuard},
    thread,
};

#[cfg(loom)]
#[derive(Debug)]
pub(crate) struct Mutex<T> {
    mutex: OldMutex<T>,
}

#[cfg(loom)]
impl<T> Mutex<T> {
    pub fn new(val: T) -> Self {
        Mutex {
            mutex: OldMutex::new(val),
        }
    }

    pub fn lock(&self) -> OldMutexGuard<T> {
        self.mutex.lock().unwrap()
    }
}
