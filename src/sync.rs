//! This module is for synchronisation primitives imports.

#[cfg(not(loom))]
pub(crate) use parking_lot::{Condvar, Mutex};

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

// Not needed anymore
//
// #[cfg(loom)]
// #[derive(Debug)]
// pub(crate) struct RwLock<T> {
//     rwlock: OldRwLock<T>,
// }

// #[cfg(loom)]
// impl<T> RwLock<T> {
//     pub fn new(val: T) -> Self {
//         RwLock {
//             rwlock: OldRwLock::new(val),
//         }
//     }

//     pub fn write(&self) -> OldRwLockWriteGuard<T> {
//         self.rwlock.write().unwrap()
//     }

//     pub fn read(&self) -> OldRwLockReadGuard<T> {
//         self.rwlock.read().unwrap()
//     }
// }