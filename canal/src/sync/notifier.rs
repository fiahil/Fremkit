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
    pub fn new() -> (Notifier, Notifier) {
        let n = Notifier {
            mutex: Arc::new(Mutex::new(false)),
            condvar: Arc::new(Condvar::new()),
        };

        (n.clone(), n)
    }

    /// Wait for a notification.
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

#[cfg(test)]
mod test {
    use super::*;

    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_notifier() {
        let (nx, rx) = Notifier::new();

        let h = thread::spawn(move || {
            rx.wait();
        });

        thread::sleep(Duration::from_millis(100));

        nx.notify();

        assert!(h.join().is_ok());
    }

    #[test]
    fn test_broadcast() {
        let (a, b) = Notifier::new();
        let c = a.clone();

        let h1 = thread::spawn(move || {
            b.wait();
        });
        let h2 = thread::spawn(move || {
            c.wait();
        });

        thread::sleep(Duration::from_millis(100));

        a.notify();

        assert!(h1.join().is_ok());
        assert!(h2.join().is_ok());
    }
}
