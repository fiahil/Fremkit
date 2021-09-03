use std::sync::Arc;

use log::trace;
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
    pub fn new() -> Notifier {
        Notifier {
            mutex: Arc::new(Mutex::new(false)),
            condvar: Arc::new(Condvar::new()),
        }
    }

    /// Lock the notifier, drop the given guard and wait on the condvar.
    /// This function avoid race conditions by droping the guard only when
    /// ready to receive notifications.
    pub fn drop_wait(&self, guard: impl Drop) {
        let mut lock = self.mutex.lock();

        trace!("# dropping guard...");
        drop(guard);

        trace!("# waiting for notification...");
        self.condvar.wait(&mut lock);
        trace!("# click!");
    }

    #[allow(dead_code)]
    /// Wait for a notification.
    pub fn wait(&self) {
        let mut lock = self.mutex.lock();

        trace!("# waiting for notification...");
        self.condvar.wait(&mut lock);
        trace!("# click!");
    }

    /// Send a notification to all waiting notifiers.
    pub fn notify(&self) {
        trace!("# sending notification...");
        self.condvar.notify_all();
        trace!("# tac!");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::thread;
    use std::time::Duration;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_notifier() {
        init();

        let notifier = Notifier::new();
        let b = notifier.clone();

        let h = thread::spawn(move || {
            b.wait();
        });

        thread::sleep(Duration::from_millis(100));

        notifier.notify();

        assert!(h.join().is_ok());
    }

    #[test]
    fn test_broadcast() {
        init();

        let notifier = Notifier::new();
        let b = notifier.clone();
        let c = notifier.clone();

        let h1 = thread::spawn(move || {
            b.wait();
        });
        let h2 = thread::spawn(move || {
            c.wait();
        });

        thread::sleep(Duration::from_millis(100));

        notifier.notify();

        assert!(h1.join().is_ok());
        assert!(h2.join().is_ok());
    }
}
