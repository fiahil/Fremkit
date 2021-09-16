use std::sync::Arc;

use crate::log::Log;
use crate::sync::Notifier;

/// A Canal is an ordered collection of Droplets.
/// Droplets are can be added to, or retrieved from, the Canal ; but
/// they cannot be removed.
///
/// The same canal can serve as a sender and receiver.
#[derive(Debug, Clone)]
pub struct Canal<T> {
    notifier: Notifier,
    log: Arc<Log<T>>,
}

impl<T> Canal<T>
where
    T: Clone,
{
    /// Create a new canal.
    pub fn new() -> Self {
        let notifier = Notifier::new();

        Canal {
            notifier,
            log: Arc::new(Log::new()),
        }
    }

    /// Add a value to the canal, and notifies all listeners.
    pub fn push(&self, value: T) -> usize {
        let idx = self.log.push(value);
        self.notifier.notify();

        idx
    }

    /// Wait for a new droplet to be added to the canal.
    /// Skip waiting if the canal already holds a droplet at the given index.
    ///
    /// * `index` - The index of the droplet we are waiting for.
    pub fn wait_for(&self, index: usize) -> Option<Arc<T>> {
        // if the current index is already in the log,
        // we skip waiting and return immediately
        self.notifier.wait_if(|| self.log.get(index).is_some());

        // we are now expected to find a droplet at the given index
        self.log.get(index)
    }

    /// Get a droplet from the canal.
    /// Return None if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<Arc<T>> {
        self.log.get(index)
    }

    /// Get the length of the canal.
    pub fn len(&self) -> usize {
        self.log.len()
    }
}
