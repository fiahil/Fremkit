use std::cell::UnsafeCell;
use std::collections::LinkedList;
use std::sync::atomic::AtomicPtr;
use std::sync::Arc;

use crate::log::Log;
use crate::notifier::Notifier;
use crate::sync::Mutex;
use crate::LogError;

const DEFAULT_LOG_CAPACITY: usize = 1024;

/// A Canal is a collection of logs.
/// The same canal can serve as a sender and receiver.
#[derive(Debug)]
pub struct Canal<T> {
    log_capacity: usize,
    notifier: Notifier,
    logs: Arc<UnsafeCell<LinkedList<Arc<Log<T>>>>>,
    mutex: Arc<Mutex<bool>>,
}

impl<T> Canal<T> {
    /// Create a new canal.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_LOG_CAPACITY)
    }

    /// Create a new canal with the given log capacity.
    pub fn with_capacity(log_capacity: usize) -> Self {
        let mut log = Arc::new(Log::new(log_capacity));
        let mut list = LinkedList::new();

        list.push_back(log);

        Canal {
            log_capacity,
            notifier: Notifier::new(),
            logs: Arc::new(UnsafeCell::new(list)),
            mutex: Arc::new(Mutex::new(false)),
        }
    }

    /// Add a value to the canal, and notifies all listeners.
    pub fn push(&self, value: T) -> usize {
        let deque = unsafe { &*self.logs.get() };

        match deque.back().unwrap().push(value) {
            Ok(idx) => {
                self.notifier.notify();
                idx
            }
            Err(LogError::LogCapacityExceeded(v)) => {
                let _lock = self.mutex.lock();

                let deque = unsafe { &mut *self.logs.get() };

                // If someone else has already added a log, we just append to it.
                match deque.back().unwrap().push(v) {
                    Ok(idx) => {
                        self.notifier.notify();
                        idx
                    }
                    Err(LogError::LogCapacityExceeded(v)) => {
                        // Otherwise, we create a new log first.
                        deque.push_back(Arc::new(Log::new(self.log_capacity)));

                        let idx = deque.back().unwrap().push(v).unwrap_or_else(|_| {
                            panic!("Unreachable: new log cannot be already full")
                        });

                        self.notifier.notify();
                        idx
                    }
                }
            }
        }
    }

    /// Wait for a value to be added to the canal at the given index.
    /// Skip waiting if the canal already holds anything at this index.
    ///
    /// * `index` - The index of the value we are waiting for.
    pub fn wait_for(&self, index: usize) -> &T {
        // if the current index is already in the log,
        // we skip waiting and return immediately
        self.notifier.wait_if(|| self.get(index).is_none());

        // we are now expected to find a value at the given index
        self.get(index).unwrap()
    }

    /// Get a droplet from the canal.
    /// Return None if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<&T> {
        let deque = unsafe { &*self.logs.get() };

        deque.iter().enumerate().find_map(|(i, log)| {
            if index / self.log_capacity == i {
                log.get(index % self.log_capacity)
            } else {
                None
            }
        })
    }

    /// Get the length of the canal.
    pub fn len(&self) -> usize {
        let deque = unsafe { &*self.logs.get() };

        (deque.len() - 1) * self.log_capacity + deque.back().unwrap().len()
    }

    /// Is the canal empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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

unsafe impl<T: Sync + Send> Send for Canal<T> {}
unsafe impl<T: Sync + Send> Sync for Canal<T> {}

impl<T> Default for Canal<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for Canal<T> {
    fn clone(&self) -> Self {
        Self {
            log_capacity: self.log_capacity,
            notifier: self.notifier.clone(),
            logs: self.logs.clone(),
            mutex: self.mutex.clone(),
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

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn canal_length() {
        init();

        let c: Canal<u32> = Canal::new();

        assert_eq!(c.len(), 0);
        assert!(c.is_empty());

        c.push(1);

        assert_eq!(c.len(), 1);
        assert!(!c.is_empty());
    }

    fn canal_increase() {
        init();

        let c = Canal::with_capacity(2);

        assert_eq!(c.len(), 0);

        for i in 0..21 {
            c.push(i);
        }

        assert_eq!(c.len(), 21);
        assert_eq!(unsafe { (&*c.logs.get()).len() }, 11);

        for i in 0..21 {
            assert_eq!(c.get(i), Some(&i));
        }

        assert_eq!(c.get(22), None);
    }

    fn canal() {
        init();

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
        fn test_canal_length() {
            canal_length()
        }

        #[test]
        fn test_canal_increase() {
            canal_increase()
        }

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
        fn test_canal_length() {
            loom::model(canal_length)
        }

        #[test]
        fn test_canal_increase() {
            loom::model(canal_increase)
        }

        #[test]
        fn test_canal() {
            loom::model(canal)
        }
    }
}
