use std::sync::{
    atomic::{AtomicI32, Ordering},
    Arc,
};

use parking_lot::{Condvar, Mutex};

/// A Cooldown is like a reverse notifier: it can wait for a given
/// count of signals to be received, and then release the waiting thread.
/// The cooldown is destructed when the waiter is released.
#[derive(Debug, Clone)]
pub struct Cooldown {
    m_counter: Arc<Mutex<bool>>,
    c_counter: Arc<Condvar>,

    counter: Arc<AtomicI32>,

    m_trigger: Arc<Mutex<bool>>,
    c_trigger: Arc<Condvar>,
}

impl Cooldown {
    /// Create a Cooldown.
    pub fn new(counter: i32) -> Cooldown {
        Cooldown {
            m_counter: Arc::new(Mutex::new(false)),
            c_counter: Arc::new(Condvar::new()),

            counter: Arc::new(AtomicI32::new(counter)),

            m_trigger: Arc::new(Mutex::new(false)),
            c_trigger: Arc::new(Condvar::new()),
        }
    }

    /// Wait for the counter to reach 0 and destroy the cooldown.
    pub fn wait(self) -> usize {
        // Lock the counter mutex.
        let mut lock = self.m_counter.lock();

        // while the counter is not 0, wait.
        if self.counter.load(Ordering::SeqCst) > 0 {
            self.c_counter.wait(&mut lock);
        }

        // we don't need the counter anymore, so unlock it.
        drop(lock);

        // The counter reached 0, we can pull the trigger.
        let mut lock = self.m_trigger.lock();

        let count = self.c_trigger.notify_all();

        // The trigger is marked
        *lock = true;

        count
    }

    /// Inform the cooldown that this thread is ready.
    pub fn ready(self) {
        // Lock the trigger mutex.
        let mut lock = self.m_trigger.lock();

        // Notify the counter.
        if self.counter.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.c_counter.notify_all();
        }

        // If the trigger has not been pulled yet, wait for it.
        if !*lock {
            self.c_trigger.wait(&mut lock);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::thread;

    #[test]
    fn test_cooldown() {
        let count = 12;
        let cd = Cooldown::new(count);

        for i in 0..count {
            let xcd = cd.clone();

            thread::spawn(move || {
                println!("starting thread {}", i);
                xcd.ready();
                println!("thread {} ready and started", i);
            });
        }

        println!("waiting for threads to be ready");
        let count = cd.wait();
        println!("all {} are ready, let's start.", count);
    }
}
