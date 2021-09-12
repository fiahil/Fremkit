use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicBool, Arc};

use parking_lot::RwLock;

use crate::sync::Notifier;
use crate::CanalError;

/// A Canal is an ordered collection of Droplets.
/// Droplets are can be added to, or retrieved from, the Canal ; but
/// they cannot be removed.
///
/// The same canal can serve as a sender and receiver.
#[derive(Debug, Clone)]
pub struct Canal<T> {
    notifier: Notifier,
    closed: Arc<AtomicBool>,
    data: Arc<RwLock<VecDeque<T>>>,
}

impl<T> Canal<T>
where
    T: Clone,
{
    /// Create a new canal.
    pub fn new() -> Self {
        let (n, _) = Notifier::new();

        Canal {
            notifier: n,
            closed: Arc::new(AtomicBool::new(false)),
            data: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Add a value to the canal, and notifies all listeners.
    pub fn add(&self, val: T) -> Result<(), CanalError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(CanalError::CanalClosed);
        }

        let mut guard = self.data.write();

        guard.push_back(val);

        self.notifier.notify();
        Ok(())
    }

    /// Wait for a new droplet to be added to the canal.
    /// Skip waiting if the canal already holds a droplet at the given index.
    ///
    /// * `index` - The index of the droplet we are waiting for.
    pub fn get_blocking(&self, index: usize) -> Option<T> {
        let guard = self.data.read();

        // if the current index is lower than the current canal size,
        // we skip waiting and return immediately
        match guard.get(index).cloned() {
            None => {
                self.notifier.wait_if(move || {
                    drop(guard);
                    !self.closed.load(Ordering::Acquire)
                });

                self.get(index)
            }
            opt => opt,
        }
    }

    /// Get a droplet from the canal.
    /// Return None if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<T> {
        let guard = self.data.read();

        guard.get(index).cloned()
    }

    /// Get the length of the canal.
    pub fn len(&self) -> usize {
        let guard = self.data.read();

        guard.len()
    }

    /// Close the canal.
    /// Further attempts to add a droplet to the canal will fail.
    /// Droplets can still be retrieved from the canal as long as the canal is not dropped.
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);

        // unblock any waiting threads
        self.notifier.notify();
    }
}

#[cfg(test)]
mod test_canal {
    use crate::sync::Cooldown;

    use super::*;

    use std::thread;

    #[test]
    fn test_canal() {
        let canal = Canal::new();
        let cd = Cooldown::new(2);
        let (c1, c2) = (canal.clone(), canal.clone());
        let (cd1, cd2) = (cd.clone(), cd.clone());

        let h1 = thread::spawn(move || {
            // starts threads simultaneously
            cd1.ready();

            let mut i = 0;

            while i < 10 {
                c1.add(1).unwrap();
                i += 1;
            }

            i
        });

        let h2 = thread::spawn(move || {
            // starts threads simultaneously
            cd2.ready();

            let mut i = 0;

            loop {
                let x = c2.get_blocking(i);
                println!("## {:?}", x);

                i += 1;

                if i == 10 {
                    break;
                }
            }

            i
        });

        cd.wait();
        assert_eq!(h1.join().unwrap(), 10);
        assert_eq!(h2.join().unwrap(), 10);
    }
}
