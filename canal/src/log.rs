use crate::sync::{AtomicUsize, Ordering};
use crate::LogError;

use std::cell::UnsafeCell;
use std::num::NonZeroUsize;

/// A Log stores an immutable, append-only, sequence of items.
/// It's a wrapper around a fixed-size vector, and it's thread-safe.
#[derive(Debug)]
pub struct Log<T> {
    capacity: NonZeroUsize,
    data: Vec<UnsafeCell<Option<T>>>,
    len: AtomicUsize,
}

impl<T> Log<T> {
    /// Create a new empty Log.
    pub fn new(capacity: usize) -> Self {
        // Specifying capacity here, means we are able to hold at least
        // this many items without reallocating.
        let mut data = Vec::with_capacity(capacity);

        // Initialize the data.
        for _ in 0..capacity {
            data.push(UnsafeCell::new(None));
        }

        Self {
            capacity: NonZeroUsize::new(capacity).expect("Cannot create a 0 capacity Log"),
            len: AtomicUsize::new(0),
            data,
        }
    }

    /// Get the current length of the log.
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed)
    }

    /// Get the capacity of the log.
    #[allow(dead_code)]
    pub fn capacity(&self) -> usize {
        self.capacity.get()
    }

    /// Is the Log empty ?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get an item from the Log.
    /// Returns `None` if the given index is out of bounds.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index > self.capacity.get() - 1 {
            return None;
        }

        let cell = &self.data[index];

        unsafe { (*cell.get()).as_ref() }
    }

    /// Append an item to the Log.
    /// Returns the index of the appended item.
    pub fn push(&self, value: T) -> Result<usize, LogError> {
        let token = self.len.fetch_add(1, Ordering::Relaxed);

        if token >= self.capacity.get() {
            return Err(LogError::LogCapacityExceeded);
        }

        let cell = &self.data[token];

        unsafe {
            cell.get().write(Some(value));
        }

        Ok(token)
    }
}

unsafe impl<T: Sync + Send> Send for Log<T> {}
unsafe impl<T: Sync + Send> Sync for Log<T> {}

impl<T> Default for Log<T> {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use log::debug;

    use crate::sync::thread;

    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    #[should_panic]
    fn test_log_capacity() {
        init();

        let _log: Log<u32> = Log::new(0);
    }

    fn log_capacity_excess() {
        init();

        let log = Log::new(1);

        log.push(0).unwrap();
        log.push(1).unwrap();
    }

    fn log_immutable_entries() {
        init();

        let log = Log::new(200);

        log.push(0).unwrap();
        log.push(42).unwrap();

        assert_eq!(log.get(1).map(|s| *s), Some(42));

        for i in 0..100 {
            log.push(i).unwrap();
        }

        assert_eq!(log.get(1).map(|s| *s), Some(42));
    }

    fn basic_log() {
        init();

        let log = Log::new(3);

        log.push(1).unwrap();
        log.push(2).unwrap();
        log.push(3).unwrap();

        assert_eq!(log.get(0).map(|s| *s), Some(1));
        assert_eq!(log.get(1).map(|s| *s), Some(2));
        assert_eq!(log.get(2).map(|s| *s), Some(3));
        assert_eq!(log.get(3), None);
    }

    /// test for validating our eventually consistent log
    fn eventual_consistency() {
        init();

        let vec = Arc::new(Log::new(2));
        let v1 = vec.clone();
        let v2 = vec.clone();

        let h1 = thread::spawn(move || {
            v1.push('a').unwrap();

            let x0 = v1.get(0);
            let x1 = v1.get(1);

            (x0.cloned(), x1.cloned())
        });

        let h2 = thread::spawn(move || {
            v2.push('b').unwrap();

            let x0 = v2.get(0);
            let x1 = v2.get(1);

            (x0.cloned(), x1.cloned())
        });

        let (x0h1, x1h1) = h1.join().unwrap();
        let (x0h2, x1h2) = h2.join().unwrap();
        let (x0, x1) = (vec.get(0).cloned(), vec.get(1).cloned());

        debug!(
            "0: h1(a) {:<10} h2(c) {:<10}  f {:<10}",
            format!("{:?}", x0h1),
            format!("{:?}", x0h2),
            format!("{:?}", x0)
        );
        debug!(
            "1: h1(b) {:<10} h2(d) {:<10}  f {:<10}",
            format!("{:?}", x1h1),
            format!("{:?}", x1h2),
            format!("{:?}", x1)
        );
        debug!("");

        match (x0h1, x1h1, x0h2, x1h2) {
            (None, None, _, _) | (_, _, None, None) => {
                assert!(false, "1|2: (Read your own write)");
            }
            (None, Some(_), None, Some(_)) => {
                assert!(false, "1: (Read your own write)");
            }
            (Some(_), None, Some(_), None) => {
                assert!(false, "2: (Read your own write)");
            }
            (None, Some(_), Some(_), None) => {
                assert!(false, "(Observed state are global)");
            }

            (Some(a), None, None, Some(d)) => {
                assert_eq!(Some(a), x0, "a == x0 (Observed state are immutable)");
                assert_eq!(Some(d), x1, "d == x1 (Observed state are immutable)");
            }
            (None, Some(b), Some(c), Some(d)) => {
                assert_eq!(b, d, "b == d (Observed state are in-order)");
                assert_eq!(Some(b), x1, "b == x1 (Observed state are immutable)");
                assert_eq!(Some(c), x0, "c == x0 (Observed state are immutable)");
                assert_eq!(Some(d), x1, "d == x1 (Observed state are immutable)");
            }
            (Some(a), None, Some(c), Some(d)) => {
                assert_eq!(a, c, "a == c (Observed state are in-order)");
                assert_eq!(Some(a), x0, "a == x0 (Observed state are immutable)");
                assert_eq!(Some(c), x0, "c == x0 (Observed state are immutable)");
                assert_eq!(Some(d), x1, "d == x1 (Observed state are immutable)");
            }
            (Some(a), Some(b), Some(c), None) => {
                assert_eq!(a, c, "a == c (Observed state are in-order)");
                assert_eq!(Some(a), x0, "a == x0 (Observed state are immutable)");
                assert_eq!(Some(b), x1, "b == x1 (Observed state are immutable)");
                assert_eq!(Some(c), x0, "c == x0 (Observed state are immutable)");
            }
            (Some(a), Some(b), None, Some(d)) => {
                assert_eq!(b, d, "b == d (Observed state are in-order)");
                assert_eq!(Some(a), x0, "a == x0 (Observed state are immutable)");
                assert_eq!(Some(b), x1, "b == x1 (Observed state are immutable)");
                assert_eq!(Some(d), x1, "d == x1 (Observed state are immutable)");
            }
            (Some(a), Some(b), Some(c), Some(d)) => {
                assert_eq!(a, c, "a == c");
                assert_eq!(b, d, "b == d");
                assert_eq!(Some(a), x0, "a == x0 (Observed state are immutable)");
                assert_eq!(Some(b), x1, "b == x1 (Observed state are immutable)");
                assert_eq!(Some(c), x0, "c == x0 (Observed state are immutable)");
                assert_eq!(Some(d), x1, "d == x1 (Observed state are immutable)");
            }
        }

        let pair = [x0, x1];

        assert!(
            pair == [Some('a'), Some('b')] || pair == [Some('b'), Some('a')],
            "final state is always complete."
        );
    }

    #[cfg(not(loom))]
    mod test {
        use super::*;

        #[test]
        #[should_panic]
        fn test_log_capacity_excess() {
            log_capacity_excess()
        }

        #[test]
        fn test_log_immutable_entries() {
            log_immutable_entries()
        }

        #[test]
        fn test_basic_log() {
            basic_log()
        }

        #[test]
        fn test_eventual_consistency() {
            eventual_consistency()
        }
    }
    #[cfg(loom)]
    mod test {
        use super::*;

        use loom;

        #[test]
        #[should_panic]
        fn test_log_capacity_excess() {
            loom::model(log_capacity_excess);
        }

        #[test]
        fn test_log_immutable_entries() {
            loom::model(log_immutable_entries)
        }

        #[test]
        fn test_basic_log() {
            loom::model(basic_log)
        }

        #[test]
        fn test_eventual_consistency() {
            loom::model(eventual_consistency)
        }
    }
}
