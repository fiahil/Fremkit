use std::sync::Arc;

use log::trace;
use parking_lot::RwLock;

use crate::core::Droplet;
use crate::sync::Notifier;

pub type CanalId = &'static str;

/// A Canal is an ordered collection of Droplets.
/// Droplets are can be added to, or retrieved from, the Canal ; but
/// they cannot be removed.
#[derive(Debug, Clone)]
pub struct Canal {
    id: CanalId,
    notifier: Notifier,
    data: Arc<RwLock<Vec<Droplet>>>,
}

impl Canal {
    /// Create a new canal.
    pub(crate) fn new(id: CanalId) -> Canal {
        trace!("> canal `{}`: created", id);

        Canal {
            id,
            notifier: Notifier::new(),
            data: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a droplet to the canal, and notifies all listeners.
    pub fn add_and_notify(&self, droplet: Droplet) {
        let mut guard = self.data.write();

        trace!("> canal `{}`: put droplet", self.id);

        guard.push(droplet);

        self.notifier.notify();
    }

    /// Wait for a new droplet to be added to the canal.
    /// Skip waiting if the canal already holds a droplet at the given index.
    ///
    /// * `current` - The index of the droplet we are waiting for.
    pub fn wait_for_droplet(&self, current: usize) {
        let guard = self.data.read();

        // if the current index is lower than the current canal size,
        // we skip waiting and return immediately
        if current < guard.len() {
            trace!(
                "> canal `{}`: cur {} | len {} | return",
                self.id,
                current,
                guard.len()
            );
            return;
        } else {
            trace!(
                "> canal `{}`: cur {} | len {} | wait",
                self.id,
                current,
                guard.len()
            );

            self.notifier.drop_wait(guard);
        }
    }

    /// Get a droplet from the canal.
    /// Return None if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<Droplet> {
        let guard = self.data.read();

        trace!("> canal `{}`: get droplet {}", self.id, index);

        guard.get(index).cloned()
    }

    /// Get the length of the canal.
    pub fn len(&self) -> usize {
        let guard = self.data.read();

        trace!("> canal `{}`: len {}", self.id, guard.len());

        guard.len()
    }
}

#[cfg(test)]
mod test_canal {
    use super::*;

    use std::thread;
    use std::time::Duration;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_canal() {
        init();

        let canal = Canal::new("hello");
        let notifier = Notifier::new();
        let (n1, n2) = (notifier.clone(), notifier.clone());
        let (c1, c2) = (canal.clone(), canal.clone());

        let h1 = thread::spawn(move || {
            // starts threads simultaneously
            n1.wait();

            let mut i = 0;

            while i < 10 {
                c1.add_and_notify(Droplet::Data);
                i += 1;
            }

            i
        });

        let h2 = thread::spawn(move || {
            // starts threads simultaneously
            n2.wait();

            let mut i = 0;

            loop {
                c2.wait_for_droplet(i);
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
