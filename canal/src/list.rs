//! This module contains the implementation of the `List` type.
//! The `List` type is a thread-safe, simply-linked, append-only, list.

use crate::sync::{AtomicPtr, Mutex, Ordering};

use std::ptr;

#[derive(Debug)]
/// A block is a node in a linked list.
struct Block<T> {
    next: AtomicPtr<Block<T>>,
    value: T,
}

#[derive(Debug)]
/// A thread-safe linked list.
pub struct List<T> {
    head: AtomicPtr<Block<T>>,
    tail: AtomicPtr<Block<T>>,
    len: Mutex<usize>,
}

impl<T> List<T> {
    /// Creates a new list with a given element as first.
    pub fn new(value: T) -> Self {
        let block = Box::new(Block {
            next: AtomicPtr::new(ptr::null_mut()),
            value,
        });

        let ptr: *mut Block<T> = Box::leak(block);

        List {
            tail: AtomicPtr::new(ptr),
            head: AtomicPtr::new(ptr),
            len: Mutex::new(1),
        }
    }

    /// Return the length of the list
    pub fn len(&self) -> usize {
        *self.len.lock()
    }

    #[allow(dead_code)]
    /// Is the list empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Append an element to the back of the list.
    /// This operation is O(1) and thread-safe.
    pub fn append(&self, value: T) {
        let block = Box::new(Block {
            next: AtomicPtr::new(ptr::null_mut()),
            value,
        });
        let ptr: *mut Block<T> = Box::leak(block);

        let mut lock = self.len.lock();
        let tail = unsafe { self.tail.load(Ordering::SeqCst).as_ref().unwrap() };

        tail.next.store(ptr, Ordering::SeqCst);
        self.tail.store(ptr, Ordering::SeqCst);

        *lock += 1;
    }

    /// Return a reference to an element at the given index.
    /// Return None if the index is out of bounds.
    /// This operation is O(n) and thread-safe.
    pub fn get(&self, index: usize) -> Option<&T> {
        let mut current = unsafe { self.head.load(Ordering::SeqCst).as_ref().unwrap() };

        // TODO: use cache to speed common operations (like tail get).
        for _ in 0..index {
            match unsafe { current.next.load(Ordering::SeqCst).as_ref() } {
                None => return None,
                Some(next) => current = next,
            }
        }

        Some(&current.value)
    }

    // Return a reference to the last element of the list.
    pub fn tail(&self) -> &T {
        unsafe { &self.tail.load(Ordering::Relaxed).as_ref().unwrap().value }
    }

    #[allow(dead_code)]
    /// Create a new head -> tail list iterator
    pub fn iter(&self) -> ListIterator<T> {
        ListIterator { cursor: &self.head }
    }
}

impl<T> Drop for List<T> {
    fn drop(&mut self) {
        let mut current = self.head.load(Ordering::SeqCst);

        loop {
            let next = unsafe { (&*current).next.load(Ordering::SeqCst) };

            // taking ownership of the current block.
            // will be deallocated at the end of the current iteration.
            unsafe { Box::from_raw(current) };

            if next.is_null() {
                break;
            } else {
                current = next;
            }
        }
    }
}

/// An iterator over a list.
pub struct ListIterator<'a, T> {
    cursor: &'a AtomicPtr<Block<T>>,
}

impl<'a, T> Iterator for ListIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let v = unsafe { self.cursor.load(Ordering::Relaxed).as_ref() };

        if let Some(ptr) = v {
            self.cursor = &ptr.next;
        }

        v.map(|block| &block.value)
    }
}

#[cfg(test)]
mod test {

    use log::debug;

    use super::*;

    use crate::notifier::Notifier;
    use crate::sync::thread;

    use std::sync::Arc;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn list_basics() {
        init();

        let list = List::new(0);

        for i in 1..10 {
            list.append(i);
        }

        assert_eq!(list.len(), 10);

        for i in 0..10 {
            debug!("{:?}", list.get(i));
            assert_eq!(list.get(i), Some(&i));
        }

        assert_eq!(list.get(100), None);
    }

    fn list_iter() {
        init();

        let list = List::new(0);

        list.append(1);
        list.append(2);
        list.append(3);

        for (x, y) in list.iter().zip(0..4) {
            debug!("idx: {} | val: {}", y, x);
            assert_eq!(x, &y);
        }
    }

    fn list_multi_thread_append() {
        init();

        // Barrier doesn't work with Loom
        let notifier = Arc::new(Notifier::new());
        let list = Arc::new(List::new(-1));

        let (b1, l1) = (notifier.clone(), list.clone());
        let t1 = thread::spawn(move || {
            b1.wait();

            for i in 0..100 {
                if i % 2 == 0 {
                    l1.append(i);
                }
            }
        });

        let (b2, l2) = (notifier.clone(), list.clone());
        let t2 = thread::spawn(move || {
            b2.wait();

            for i in 0..100 {
                if i % 2 == 1 {
                    l2.append(i);
                }
            }
        });

        while notifier.count() < 2 {
            thread::yield_now();
        }

        notifier.notify();

        t1.join().unwrap();
        t2.join().unwrap();

        let mut vec = list
            .iter()
            .cloned()
            .inspect(|x| debug!("{}", x))
            .collect::<Vec<_>>();

        vec.sort();

        assert_eq!(vec, (-1..100).into_iter().collect::<Vec<_>>());
    }

    #[cfg(not(loom))]
    mod test {
        use super::*;

        #[test]
        fn test_list_basics() {
            list_basics()
        }

        #[test]
        fn test_list_iter() {
            list_iter()
        }

        #[test]
        fn test_list_multi_thread_append() {
            list_multi_thread_append()
        }
    }
    #[cfg(loom)]
    mod test {
        use super::*;

        use loom;

        #[test]
        fn test_list_basics() {
            loom::model(list_basics)
        }

        #[test]
        fn test_list_iter() {
            loom::model(list_iter)
        }

        #[test]
        fn test_list_multi_thread_append() {
            loom::model(list_multi_thread_append)
        }
    }
}
