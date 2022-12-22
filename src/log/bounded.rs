//! This module contains the implementation of the bounded `Log` type.

use crate::sync::{AtomicUsize, Ordering};
use crate::LogError;

use std::cell::UnsafeCell;
use std::sync::Arc;

use cache_padded::CachePadded;

/// This Log stores an immutable, append-only, bounded, concurrent sequence of items.
///
/// It's a performance-minded wrapper around a fixed-size vector, and is thread-safe.
///
/// A Log's primary use case is to store an immutable sequence of messages, events, or other data, and to allow
/// multiple readers to access the data concurrently.
///
/// Performance-wise, the Log aim to be almost as fast as a `Vec` for single-threaded push operations, and
/// will be equally fast for get operations.
/// For multi-threaded push operations, the Log should be as fast as a `Vec` wrapped in a `Mutex` or `RwLock`.
/// For multi-threaded get operations, the Log will be faster than a `Vec` wrapped in a `RwLock`.
/// Additional performance analysis are available in the benchmarks.
///
/// Operations on Log are lock-free, and will never block.
/// The Log also supports concurrent push get operations.
/// The Log will never be resized, and will always have the same capacity.
///
/// All data pushed on the Log will become available for get in the same order as it was pushed,
/// and will always be available at the returned index.
///
/// When the Log becomes full, push will fail and return an error. A get to an existing index will always succeed.
///
/// # Examples
/// ```
/// use fremkit::bounded::Log;
///
/// let log: Log<u64> = Log::new(100);
/// log.push(1).unwrap();
/// log.push(2).unwrap();
///
/// assert_eq!(log.get(0), Some(&1));
/// assert_eq!(log.get(1), Some(&2));
/// assert_eq!(log.get(2), None);
///
/// assert_eq!(log.len(), 2);
/// assert_eq!(log.capacity(), 100);
/// ```
#[derive(Debug)]
pub struct Log<T> {
    len: CachePadded<AtomicUsize>,
    capacity: usize,
    data: Vec<UnsafeCell<Option<T>>>,
}

impl<T> Log<T> {
    /// Create a new empty Log. It will be able to hold at least `capacity` items.
    /// If `capacity` is 0, the Log will be created with a capacity of 1.
    ///
    /// # Examples
    /// ```
    /// use fremkit::bounded::Log;
    ///
    /// let log: Log<u64> = Log::new(100);
    /// ```
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);

        // Specifying capacity here, means we are able to hold at least
        // this many items without reallocating.
        let mut data = Vec::with_capacity(capacity);

        // Initialize the data.
        for _ in 0..capacity {
            data.push(UnsafeCell::new(None));
        }

        Self {
            capacity,
            len: CachePadded::new(AtomicUsize::new(0)),
            data,
        }
    }

    /// Get the current length of the log.
    ///
    /// This is the number of items that have been pushed on the log.
    /// It will never be greater than the capacity of the log.
    ///
    /// # Examples
    /// ```
    /// use fremkit::bounded::Log;
    ///
    /// let log: Log<u64> = Log::new(100);
    /// log.push(1).unwrap();
    /// log.push(2).unwrap();
    ///
    /// assert_eq!(log.len(), 2);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed).min(self.capacity())
    }

    /// Get the capacity of the log.
    ///
    /// This is the maximum number of items that can be pushed on the log.
    ///
    /// # Examples
    /// ```
    /// use fremkit::bounded::Log;
    ///
    /// let log: Log<u64> = Log::new(100);
    ///
    /// assert_eq!(log.capacity(), 100);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Is the log empty ?
    ///
    /// # Examples
    /// ```
    /// use fremkit::bounded::Log;
    ///
    /// let log: Log<u64> = Log::new(100);
    ///
    /// assert!(log.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get an item from the log.
    ///
    /// # Arguments
    /// * `index` - The index of the item to get.
    ///
    /// # Returns
    /// A reference to the item at the given index, or `None` if the index is out of bounds.
    ///
    /// # Examples
    /// ```
    /// use fremkit::bounded::Log;
    ///
    /// let log: Log<u64> = Log::new(100);
    /// log.push(1).unwrap();
    /// log.push(2).unwrap();
    ///
    /// assert_eq!(log.get(0), Some(&1));
    /// assert_eq!(log.get(1), Some(&2));
    /// assert_eq!(log.get(2), None);
    /// assert_eq!(log.get(123), None);
    /// ```
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len() {
            return None;
        }

        // SAFETY: We know that the index is in bounds, and that the cell is initialized.
        // We also know that the cell will not be modified while we are holding a reference to it.
        // This is because the cell is never modified. The only way to modify the cell is to push an item,
        // and this will only happen if the cell is empty.
        // We also know that the cell will not be dropped while we are holding a reference to it.
        let cell = &self.data[index];

        unsafe { (*cell.get()).as_ref() }
    }

    /// Append an item to the log.
    ///
    /// Once the item has been appended, it will be available for get at the returned index.
    /// The index will always be in the range [0, capacity). Items cannot be removed from the log.
    /// If the log is full, the item will not be appended, and an error containing the item will be returned.
    ///
    /// # Arguments
    /// * `value` - The item to append.
    ///
    /// # Returns
    /// The index of the item in the log, or an error containing the item if the log is full.
    ///
    /// # Examples
    /// ```
    /// use fremkit::bounded::Log;
    ///
    /// let log: Log<u64> = Log::new(100);
    /// assert_eq!(log.push(1).unwrap(), 0);
    /// assert_eq!(log.push(2).unwrap(), 1);
    ///
    /// assert_eq!(log.get(0), Some(&1));
    /// assert_eq!(log.get(1), Some(&2));
    /// ```
    pub fn push(&self, value: T) -> Result<usize, LogError<T>> {
        // Get the next token.
        // This is the index the item will be written to.
        // INVARIANT: The token will always be in the range [0, capacity).
        // INVARIANT: The token will always be unique.
        // INVARIANT: The series of tokens will always be monotonically increasing.
        let token = self.len.fetch_add(1, Ordering::Relaxed);

        if token >= self.capacity() {
            return Err(LogError::LogCapacityExceeded(value));
        }

        // Get the cell to write to.
        // SAFETY: The token is always in the range [0, capacity).
        let cell = &self.data[token];

        // SAFETY: Cells can only be written to once, and we are the only writer.
        // SAFETY: It is safe to write to the cell, as it cannot be read from until we first write to it.
        let slot = unsafe { &mut *cell.get() };
        *slot = Some(value);

        Ok(token)
    }
}

unsafe impl<T: Sync + Send> Send for Log<T> {}
unsafe impl<T: Sync + Send> Sync for Log<T> {}

//
// Public API similar to std::sync::mpsc::channel simplified consumption.
// Please note that the API does not make complete sense for a bounded log.
//

impl<T> Log<T> {
    /// Convert the Log into a Sender.
    pub fn into_sender(self: Arc<Self>) -> Sender<T> {
        Sender { log: self }
    }

    /// Convert the Log into a Receiver.
    ///
    /// Please note that 'Receiver' is not a good name for the reading end of a Log,
    /// but it is used for consistency with the std::sync::mpsc::channel API.
    pub fn into_receiver(self: Arc<Self>) -> Receiver<T> {
        Receiver { log: self }
    }

    /// Create an iterator over the log.
    ///
    /// The iterator will start at the beginning of the channel.
    /// When reaching the end of the channel, the iterator will stop.
    ///
    /// # Examples
    /// ```
    /// use fremkit::bounded::Log;
    ///
    /// let log: Log<u64> = Log::new(100);
    /// log.push(1).unwrap();
    /// log.push(2).unwrap();
    ///
    /// for item in log.iter() {
    ///    println!("{}", item);
    /// }
    /// ```
    pub fn iter(&self) -> LogReaderIterator<T> {
        LogReaderIterator { idx: 0, log: self }
    }
}

/// Open a new log with a given capacity.
///
/// The capacity is the maximum number of items that can be stored in the log.
///
/// # Arguments
/// * `capacity` - The maximum number of items that can be stored in the log.
///
/// # Returns
/// A Sender and a Receiver.
pub fn open<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let log = Arc::new(Log::new(capacity));

    (Sender { log: log.clone() }, Receiver { log })
}

/// Sender half of a Log.
///
/// The Sender can be cloned, and the clones will all refer to the same Log.
/// Note, this struct is provided for compatibilities with the std::sync::mpsc::channel API.
#[derive(Debug, Clone)]
pub struct Sender<T> {
    log: Arc<Log<T>>,
}

impl<T> Sender<T> {
    /// Send an item to the Log.
    ///
    /// # Arguments
    /// * `value` - The item to send.
    ///
    /// # Returns
    /// The index of the item in the log, or an error containing the item if the log is full.
    pub fn send(&self, value: T) -> Result<usize, LogError<T>> {
        self.log.push(value)
    }

    /// Convert the sender into its inner Log.
    pub fn into_inner(self) -> Arc<Log<T>> {
        self.log
    }
}

/// Reader half of a Log.
///
/// The Reader can be cloned, and the clones will all refer to the same Log.
/// Note, this struct is provided for compatibilities with the std::sync::mpsc::channel API.
#[derive(Debug, Clone)]
pub struct Receiver<T> {
    log: Arc<Log<T>>,
}

impl<T> Receiver<T> {
    /// Read an item from the Log at a given index.
    ///
    /// # Arguments
    /// * `index` - The index of the item to read, or receive.
    ///
    /// # Returns
    /// The item at the given index, or None if the index is out of bounds.
    pub fn recv(&self, index: usize) -> Option<&T> {
        self.log.get(index)
    }

    /// Convert the Reader into its inner Log.
    pub fn into_inner(self) -> Arc<Log<T>> {
        self.log
    }
}

/// Iterator over the items in a Log.
pub struct LogReaderIterator<'a, T> {
    idx: usize,
    log: &'a Log<T>,
}

impl<'a, T> Iterator for LogReaderIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 1;

        self.log.get(idx)
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
    #[cfg(loom)]
    fn test_loom() {
        loom::model(test_log_capacity);
        loom::model(test_log_capacity_excess);
        loom::model(test_log_capacity_excess_len);
        loom::model(test_log_immutable_entries);
        loom::model(test_basic_log);
        loom::model(test_log_iter);
        loom::model(test_send_recv);
        loom::model(test_eventual_consistency);
    }

    #[test]
    fn test_log_capacity() {
        init();

        let log: Log<u32> = Log::new(0);

        assert_eq!(log.capacity(), 1);
    }

    #[test]
    fn test_log_capacity_excess() {
        init();

        let log = Log::new(1);

        log.push(0).unwrap();

        assert!(log.push(1).is_err());
    }

    #[test]
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
    fn test_basic_log() {
        init();

        let log = Log::new(3);

        log.push(1).unwrap();
        log.push(2).unwrap();
        log.push(3).unwrap();

        assert_eq!(log.get(0), Some(&1));
        assert_eq!(log.get(1), Some(&2));
        assert_eq!(log.get(2), Some(&3));
        assert_eq!(log.get(3), None);
    }

    #[test]
    fn test_log_iter() {
        init();

        let log = Log::new(3);

        log.push(1).unwrap();
        log.push(2).unwrap();
        log.push(3).unwrap();

        let mut iter = log.iter();

        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_send_recv() {
        init();

        let (tx, rx) = open(4);

        tx.send(1).unwrap();
        tx.send(2).unwrap();
        tx.send(3).unwrap();

        assert_eq!(rx.recv(0), Some(&1));
        assert_eq!(rx.recv(1), Some(&2));
        assert_eq!(rx.recv(2), Some(&3));
        assert_eq!(rx.recv(3), None);

        tx.into_inner().push(4).unwrap();

        assert_eq!(rx.recv(3), Some(&4));
    }

    #[test]
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
