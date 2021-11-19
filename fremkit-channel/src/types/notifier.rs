use std::sync::Arc;

use crate::sync::{AtomicUsize, Condvar, Mutex, Ordering};

/// A notifier is a synchronization primitive that allows threads to wait for
/// a signal. The notifier is implemented on top of a condition variable, and
/// provides broadcast semantics.
#[derive(Debug, Clone)]
pub struct Notifier {
    mutex: Arc<Mutex<bool>>,
    condvar: Arc<Condvar>,
    count: Arc<AtomicUsize>,
}

impl Notifier {
    /// Create a Notifier.
    pub fn new() -> Self {
        Notifier {
            mutex: Arc::new(Mutex::new(false)),
            condvar: Arc::new(Condvar::new()),
            count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Get the current count of waiting threads.
    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    /// Wait for a notification.
    #[allow(dead_code)]
    pub fn wait(&self) {
        let mut lock = self.mutex.lock();

        self.count.fetch_add(1, Ordering::SeqCst);

        #[cfg(not(loom))]
        self.condvar.wait(&mut lock);
        #[cfg(loom)]
        let _ = self.condvar.wait(lock);
    }

    /// Wait for a notification if callback returns true.
    /// No notifications will be received while the callback is running.
    /// Returns true if the callback returned true.
    pub fn wait_if<F>(&self, callback: F) -> bool
    where
        F: FnOnce() -> bool,
    {
        let mut lock = self.mutex.lock();

        if callback() {
            self.count.fetch_add(1, Ordering::SeqCst);

            #[cfg(not(loom))]
            self.condvar.wait(&mut lock);
            #[cfg(loom)]
            let _ = self.condvar.wait(lock);

            true
        } else {
            false
        }
    }

    /// Send a notification to all waiting notifiers.
    pub fn notify(&self) {
        // Avoid sending notifications while setting up a waiter.
        let _lock = self.mutex.lock();

        self.condvar.notify_all();
        self.count.store(0, Ordering::SeqCst);
    }

    /// Send a notification to all waiting notifiers if callback returns true.
    /// Returns true if the callback returned true.
    #[allow(dead_code)]
    pub fn notify_if<F>(&self, callback: F) -> bool
    where
        F: FnOnce() -> bool,
    {
        if callback() {
            let _lock = self.mutex.lock();

            self.condvar.notify_all();
            self.count.store(0, Ordering::SeqCst);

            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod test {
    use fremkit_macro::with_loom;

    use super::*;

    use crate::sync::thread;

    #[test]
    #[with_loom]
    fn test_can_notify() {
        let notifier = Notifier::new();

        notifier.notify();
    }

    #[test]
    #[with_loom]
    fn test_notifier() {
        let n = Notifier::new();
        let nx = n.clone();

        let h = thread::spawn(move || {
            nx.wait();
        });

        while n.count() < 1 {
            thread::yield_now();
        }

        n.notify();

        assert!(h.join().is_ok());
    }

    #[test]
    #[with_loom]
    fn test_broadcast() {
        let n = Notifier::new();
        let (a, b) = (n.clone(), n.clone());

        let h1 = thread::spawn(move || {
            a.wait();
        });
        let h2 = thread::spawn(move || {
            b.wait();
        });

        while n.count() < 2 {
            thread::yield_now();
        }

        n.notify();

        assert!(h1.join().is_ok());
        assert!(h2.join().is_ok());
    }

    #[test]
    #[with_loom]
    fn test_notify_if() {
        let n = Notifier::new();
        let (a, b) = (n.clone(), n.clone());

        let h1 = thread::spawn(move || {
            a.wait();
        });
        let h2 = thread::spawn(move || {
            b.wait();
        });

        while !n.notify_if(|| n.count() == 2) {
            thread::yield_now();
        }

        assert!(h1.join().is_ok());
        assert!(h2.join().is_ok());
    }
}
