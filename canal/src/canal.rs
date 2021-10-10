use std::sync::Arc;

use crate::log::Log;
use crate::notifier::Notifier;

/// A Canal is an ordered collection of Droplets.
/// Droplets are can be added to, or retrieved from, the Canal ; but
/// they cannot be removed.
///
/// The same canal can serve as a sender and receiver.
#[derive(Debug)]
pub struct Canal<T> {
    notifier: Notifier,
    log: Arc<Log<T>>,
}

impl<T> Canal<T> {
    /// Create a new canal.
    pub fn new() -> Self {
        let notifier = Notifier::new();

        Canal {
            notifier,
            log: Arc::new(Log::new(10)),
        }
    }

    /// Add a value to the canal, and notifies all listeners.
    pub fn push(&self, value: T) -> usize {
        let idx = self.log.push(value);
        self.notifier.notify();

        idx.unwrap()
    }

    /// Wait for a new droplet to be added to the canal.
    /// Skip waiting if the canal already holds a droplet at the given index.
    ///
    /// * `index` - The index of the droplet we are waiting for.
    pub fn wait_for(&self, index: usize) -> &T {
        // if the current index is already in the log,
        // we skip waiting and return immediately
        self.notifier.wait_if(|| self.log.get(index).is_none());

        // we are now expected to find a droplet at the given index
        self.log.get(index).unwrap()
    }

    /// Get a droplet from the canal.
    /// Return None if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.log.get(index)
    }

    /// Get the length of the canal.
    pub fn len(&self) -> usize {
        self.log.len()
    }

    /// Is the canal empty?
    pub fn is_empty(&self) -> bool {
        self.log.is_empty()
    }

    pub fn iter(&self) -> CanalIterator<T> {
        CanalIterator {
            idx: 0,
            canal: self,
        }
    }

    pub fn blocking_iter(&self) -> CanalBlockingIterator<T> {
        CanalBlockingIterator {
            idx: 0,
            canal: self,
        }
    }
}

impl<T> Default for Canal<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for Canal<T> {
    fn clone(&self) -> Self {
        Self {
            notifier: self.notifier.clone(),
            log: self.log.clone(),
        }
    }
}

pub struct CanalIterator<'a, T> {
    idx: usize,
    canal: &'a Canal<T>,
}

pub struct CanalBlockingIterator<'a, T> {
    idx: usize,
    canal: &'a Canal<T>,
}

impl<'a, T> Iterator for CanalIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 1;

        self.canal.get(idx)
    }
}

impl<'a, T> Iterator for CanalBlockingIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 1;

        Some(self.canal.wait_for(idx))
    }
}

#[cfg(test)]
mod test {

    use log::debug;

    use crate::sync::thread;

    use super::*;

    fn canal() {
        let n = Notifier::new();
        let (a, b) = (n.clone(), n.clone());

        let canal = Canal::new();
        let (c1, c2) = (canal.clone(), canal.clone());

        let h1 = thread::spawn(move || {
            // starts threads simultaneously
            a.wait();

            for i in 0..10 {
                c1.push(i);
            }
        });

        let h2 = thread::spawn(move || {
            // starts threads simultaneously
            b.wait();

            for i in 0..10 {
                let x = c2.wait_for(i);
                debug!("## {:?}", x);
            }
        });

        while n.count() < 2 {
            thread::yield_now();
        }

        n.notify();

        assert!(h1.join().is_ok());
        assert!(h2.join().is_ok());
    }

    #[cfg(not(loom))]
    mod test {
        use super::*;

        #[test]
        fn test_canal() {
            canal()
        }
    }
    #[cfg(loom)]
    mod test {

        use super::*;

        use loom;

        #[test]
        fn test_canal() {
            loom::model(canal)
        }
    }
}
