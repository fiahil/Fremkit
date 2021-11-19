use crate::sync::{AtomicUsize, Ordering};
use crate::ChannelError;

use std::cell::UnsafeCell;
use std::num::NonZeroUsize;
use std::sync::Arc;

/// This Log stores an immutable, append-only, bounded, sequence of items.
/// It's a wrapper around a fixed-size vector, and it's thread-safe.
///
/// All data sent on the Log will become available in the same order as it was sent,
/// and will always be available at the returned index. No push will ever block the
/// calling thread. When the log becomes full, pushes will fail and return an error.
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
    #[inline]
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed).min(self.capacity())
    }

    /// Get the capacity of the log.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity.get()
    }

    /// Is the log empty ?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get an item from the log.
    /// Returns `None` if the given index is out of bounds.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index > self.capacity() - 1 {
            return None;
        }

        let cell = &self.data[index];

        unsafe { (*cell.get()).as_ref() }
    }

    /// Append an item to the log.
    /// Returns the index of the appended item.
    pub fn push(&self, value: T) -> Result<usize, ChannelError<T>> {
        let token = self.len.fetch_add(1, Ordering::AcqRel);

        if token >= self.capacity() {
            return Err(ChannelError::LogCapacityExceeded(value));
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

//
// Public API similar to std::sync::mpsc::channel for easier consumption.
//

impl<T> Log<T> {
    /// Convert the Log into a Sender.
    pub fn into_sender(self: Arc<Self>) -> Sender<T> {
        Sender { log: self }
    }

    /// Convert the Log into a Reader.
    pub fn into_reader(self: Arc<Self>) -> Reader<T> {
        Reader { log: self }
    }
}

/// Open a new log with a given capacity.
/// Returns a Sender and a Reader.
pub fn open<T>(capacity: usize) -> (Sender<T>, Reader<T>) {
    let channel = Arc::new(Log::new(capacity));

    (
        Sender {
            log: channel.clone(),
        },
        Reader { log: channel },
    )
}

/// Sender half of a Log.
#[derive(Debug, Clone)]
pub struct Sender<T> {
    log: Arc<Log<T>>,
}

impl<T> Sender<T> {
    /// Send an item to the Log.
    pub fn send(&self, value: T) -> Result<usize, ChannelError<T>> {
        self.log.push(value)
    }

    /// Convert the sender into its inner Log.
    pub fn into_inner(self) -> Arc<Log<T>> {
        self.log
    }
}

/// Reader half of a Log.
#[derive(Debug, Clone)]
pub struct Reader<T> {
    log: Arc<Log<T>>,
}

impl<T> Reader<T> {
    /// Read an item from the Log at a given index.
    pub fn read(&self, index: usize) -> Option<&T> {
        self.log.get(index)
    }

    /// Convert the Reader into its inner Log.
    pub fn into_inner(self) -> Arc<Log<T>> {
        self.log
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use fremkit_macro::with_loom;
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

    #[test]
    #[with_loom]
    fn test_log_capacity_excess() {
        init();

        let log = Log::new(1);

        log.push(0).unwrap();
        log.push(1).unwrap();
    }

    #[test]
    #[with_loom]
    fn test_log_capacity_excess_len() {
        init();

        let log = Log::new(1);

        log.push(0).unwrap();
        log.push(1).unwrap_err();
        log.push(2).unwrap_err();
        log.push(3).unwrap_err();
        log.push(4).unwrap_err();

        assert_eq!(log.len(), 1);
    }

    #[test]
    #[with_loom]
    fn test_log_immutable_entries() {
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

    #[test]
    #[with_loom]
    fn test_basic_log() {
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

    #[test]
    #[with_loom]
    fn test_eventual_consistency() {
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
}
