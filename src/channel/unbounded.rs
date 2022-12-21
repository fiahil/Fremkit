use std::sync::Arc;

use super::bounded::Log;
use crate::sync::{thread, Mutex};
use crate::types::List;
use crate::types::Notifier;
use crate::ChannelError;

const DEFAULT_LOG_CAPACITY: usize = 1024;

/// Unbounded version of `Log`.
/// The same channel can serve as a thread-safe sender and receiver.
/// Appending to a channel can lead to a new Log being created.
#[derive(Debug)]
pub struct Channel<T> {
    log_capacity: usize,
    notifier: Notifier,
    logs: Arc<List<Log<T>>>,
    mutex: Arc<Mutex<bool>>,
}

impl<T> Channel<T> {
    /// Create a new channel.
    pub fn new() -> Self {
        Self::with_log_capacity(DEFAULT_LOG_CAPACITY)
    }

    /// Create a new channel with the given log capacity.
    pub fn with_log_capacity(log_capacity: usize) -> Self {
        let list = List::new(Log::new(log_capacity));

        Channel {
            log_capacity,
            notifier: Notifier::new(),
            logs: Arc::new(list),
            mutex: Arc::new(Mutex::new(false)),
        }
    }

    /// Add a value to the channel, and notifies all listeners.
    pub fn push(&self, value: T) -> usize {
        let idx = match self.logs.tail().push(value) {
            Ok(idx) => idx,
            Err(ChannelError::LogCapacityExceeded(v)) => {
                let _lock = self.mutex.lock();

                // If someone else has already added a log, we just append to it.
                match self.logs.tail().push(v) {
                    Ok(idx) => idx,
                    Err(ChannelError::LogCapacityExceeded(v)) => {
                        // Otherwise, we create a new log first.
                        self.logs.append(Log::new(self.log_capacity));

                        let idx = self.logs.tail().push(v).unwrap_or_else(|_| {
                            panic!("Unreachable: new log cannot be already full")
                        });

                        idx
                    }
                }
            }
        };

        self.notifier.notify();
        idx
    }

    /// Wait for a value to be added to the channel at the given index.
    /// Skip waiting if the channel already holds anything at this index.
    ///
    /// * `index` - The index of the value we are waiting for.
    pub fn wait_for(&self, index: usize) -> &T {
        // if the current index is already in the log,
        // we skip waiting and return immediately.
        // Otherwise we wait until we are sure the index is in the log.
        // Note: While loop is to handle spurious wake-ups
        while self.notifier.wait_if(|| self.get(index).is_none()) {
            thread::yield_now();
        }

        // we are now expected to find a value at the given index
        self.get(index).unwrap()
    }

    /// Get an element from the channel.
    /// Return None if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.logs
            .get(index / self.log_capacity)
            .and_then(|log| log.get(index % self.log_capacity))
    }

    /// Get the length of the channel.
    pub fn len(&self) -> usize {
        (self.logs.len() - 1) * self.log_capacity + self.logs.tail().len()
    }

    /// Is the channel empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Create a finite iterator over the channel.
    /// When reaching the end of the channel, the iterator will stop.
    pub fn iter(&self) -> ChannelIterator<T> {
        ChannelIterator {
            idx: 0,
            channel: self,
        }
    }

    /// Create an infinite, blocking iterator over the channel.
    /// When reaching the end of the channel, the iterator will block until a new value is added.
    pub fn blocking_iter(&self) -> ChannelBlockingIterator<T> {
        ChannelBlockingIterator {
            idx: 0,
            channel: self,
        }
    }
}

unsafe impl<T: Sync + Send> Send for Channel<T> {}
unsafe impl<T: Sync + Send> Sync for Channel<T> {}

impl<T> Default for Channel<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for Channel<T> {
    /// Clone the channel.
    fn clone(&self) -> Self {
        Self {
            log_capacity: self.log_capacity,
            notifier: self.notifier.clone(),
            logs: self.logs.clone(),
            mutex: self.mutex.clone(),
        }
    }
}

pub struct ChannelIterator<'a, T> {
    idx: usize,
    channel: &'a Channel<T>,
}

pub struct ChannelBlockingIterator<'a, T> {
    idx: usize,
    channel: &'a Channel<T>,
}

impl<'a, T> Iterator for ChannelIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 1;

        self.channel.get(idx)
    }
}

impl<'a, T> Iterator for ChannelBlockingIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 1;

        Some(self.channel.wait_for(idx))
    }
}

//
// Public API similar to std::sync::mpsc::channel for easier consumption.
//

impl<T> Channel<T> {
    /// Convert the Channel into a Sender.
    pub fn into_sender(self) -> Sender<T> {
        Sender { channel: self }
    }

    /// Convert the Channel into a Reader.
    pub fn into_reader(self) -> Reader<T> {
        Reader { channel: self }
    }
}

/// Open a new channel with a given capacity.
/// Returns a Sender and a Reader.
pub fn open<T>() -> (Sender<T>, Reader<T>) {
    let channel = Channel::new();

    (
        Sender {
            channel: channel.clone(),
        },
        Reader { channel },
    )
}

/// Sender half of a Log.
#[derive(Debug, Clone)]
pub struct Sender<T> {
    channel: Channel<T>,
}

impl<T> Sender<T> {
    /// Send an item to the Log.
    pub fn send(&self, value: T) -> usize {
        self.channel.push(value)
    }

    /// Convert the sender into its inner Log.
    pub fn into_inner(self) -> Channel<T> {
        self.channel
    }
}

/// Reader half of a Log.
#[derive(Debug, Clone)]
pub struct Reader<T> {
    channel: Channel<T>,
}

impl<T> Reader<T> {
    /// Read an item from the Log at a given index.
    pub fn read(&self, index: usize) -> Option<&T> {
        self.channel.get(index)
    }

    /// Convert the Reader into its inner Log.
    pub fn into_inner(self) -> Channel<T> {
        self.channel
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

    #[test]
    fn test_channel_length() {
        init();

        let c: Channel<u32> = Channel::new();

        assert_eq!(c.len(), 0);
        assert!(c.is_empty());

        c.push(1);

        assert_eq!(c.len(), 1);
        assert!(!c.is_empty());
    }

    #[test]
    fn test_channel_increase() {
        init();

        let c = Channel::with_log_capacity(2);

        assert_eq!(c.len(), 0);

        for i in 0..21 {
            c.push(i);
        }

        assert_eq!(c.len(), 21);
        assert_eq!(c.logs.len(), 11);

        for i in 0..21 {
            assert_eq!(c.get(i), Some(&i));
        }

        assert_eq!(c.get(22), None);
    }

    #[test]
    fn test_channel() {
        init();

        // Barrier doesn't work with Loom
        let n = Notifier::new();
        let (a, b) = (n.clone(), n.clone());

        let channel = Channel::new();
        let (c1, c2) = (channel.clone(), channel.clone());

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
}