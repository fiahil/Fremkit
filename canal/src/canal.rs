use std::sync::Arc;

use parking_lot::RwLock;

use crate::sync::Notifier;

/// A Canal is an ordered collection of Droplets.
/// Droplets are can be added to, or retrieved from, the Canal ; but
/// they cannot be removed.
///
/// The same canal can serve as a sender and receiver.
#[derive(Debug, Clone)]
pub struct Canal<T> {
    notifier: Notifier,
    data: Arc<RwLock<Vec<T>>>,
}

impl<T> Canal<T>
where
    T: Clone,
{
    /// Create a new canal.
    pub fn new() -> Self {
        Canal {
            notifier: Notifier::new(),
            data: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a droplet to the canal, and notifies all listeners.
    pub fn add(&self, droplet: T) {
        let mut guard = self.data.write();

        guard.push(droplet);

        self.notifier.notify();
    }

    /// Wait for a new droplet to be added to the canal.
    /// Skip waiting if the canal already holds a droplet at the given index.
    ///
    /// * `current` - The index of the droplet we are waiting for.
    pub fn wait(&self, current: usize) {
        let guard = self.data.read();

        // if the current index is lower than the current canal size,
        // we skip waiting and return immediately
        if current < guard.len() {
            return;
        } else {
            self.notifier.drop_wait(guard);
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
}

#[cfg(test)]
mod test_canal {
    use super::*;

    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_canal() {
        let canal = Canal::new();
        let notifier = Notifier::new();
        let (n1, n2) = (notifier.clone(), notifier.clone());
        let (c1, c2) = (canal.clone(), canal.clone());

        let h1 = thread::spawn(move || {
            // starts threads simultaneously
            n1.wait();

            let mut i = 0;

            while i < 10 {
                c1.add(1);
                i += 1;
            }

            i
        });

        let h2 = thread::spawn(move || {
            // starts threads simultaneously
            n2.wait();

            let mut i = 0;

            loop {
                c2.wait(i);
                println!("## {:?}", c2.get(i));

                i += 1;

                if i == 10 {
                    break;
                }
            }

            i
        });

        thread::sleep(Duration::from_millis(250));
        notifier.notify();
        assert_eq!(h1.join().unwrap(), 10);
        assert_eq!(h2.join().unwrap(), 10);
    }
}
