use std::sync::Arc;

use log::debug;
use parking_lot::{Condvar, Mutex};

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

    /// Wait for a notification
    pub fn wait(&self) {
        let mut lock = self.mutex.lock();

        debug!("# waiting for notification...");
        self.condvar.wait(&mut lock);
        debug!("# click!");
    }

    /// Send a notification
    pub fn notify(&self) {
        debug!("# sending notification...");

        self.condvar.notify_all();
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
