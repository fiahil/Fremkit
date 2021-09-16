use std::sync::Arc;

use parking_lot::{Condvar, Mutex};

/// A notifier is a synchronization primitive that allows threads to wait for
/// a signal. The notifier is implemented on top of a condition variable, and
/// provides broadcast semantics.
#[derive(Debug, Clone)]
pub struct Notifier {
    mutex: Arc<Mutex<bool>>,
    condvar: Arc<Condvar>,
}

impl Notifier {
    /// Create a Notifier and return it with a copy
    pub fn new() -> Self {
        Notifier {
            mutex: Arc::new(Mutex::new(false)),
            condvar: Arc::new(Condvar::new()),
        }
    }

    /// Wait for a notification.
    #[allow(dead_code)]
    pub fn wait(&self) {
        let mut lock = self.mutex.lock();

        self.condvar.wait(&mut lock);
    }

    /// Wait for a notification if callback returns true.
    pub fn wait_if<F>(&self, callback: F)
    where
        F: FnOnce() -> bool,
    {
        let mut lock = self.mutex.lock();

        if !callback() {
            return;
        }

        self.condvar.wait(&mut lock);
    }

    /// Send a notification to all waiting notifiers.
    pub fn notify(&self) {
        self.condvar.notify_all();
    }
}
