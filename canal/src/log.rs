use std::sync::Arc;

use crate::sync::{AtomicUsize, Mutex, Ordering};

/// A Log stores an immutable sequence of items.
/// It's a wrapper around a vector of `Arc<T>`, and it's thread-safe.
#[derive(Debug)]
pub struct Log<T> {
    data: Mutex<Vec<Arc<T>>>,
    len: AtomicUsize,
}

impl<T> Log<T> {
    /// Create a new empty Log.
    pub fn new() -> Self {
        Self {
            data: Mutex::new(Vec::new()),
            len: AtomicUsize::new(0),
        }
    }

    /// Get the current length of the log.
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed)
    }

    /// Get an item from the Log.
    /// Returns `None` if the given index is out of bounds.
    pub fn get(&self, index: usize) -> Option<Arc<T>> {
        let vec = self.data.lock();

        vec.get(index).cloned()
    }

    /// Append an item to the Log.
    /// Returns the index of the appended item.
    pub fn push(&self, value: T) -> usize {
        // TODO: Slow: allocate and move value
        let arc = Arc::from(value);

        let mut vec = self.data.lock();

        vec.push(arc);

        self.len.fetch_add(1, Ordering::Relaxed)
    }
}
