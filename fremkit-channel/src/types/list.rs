//! This module contains the implementation of the `List` type.
//! The `List` type is a thread-safe, simply-linked, append-only, list.

use crate::sync::{AtomicPtr, AtomicUsize, Mutex, Ordering};

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr;

/// A block is a node in a linked list.
#[derive(Debug)]
struct Block<T> {
    next: AtomicPtr<Block<T>>,
    value: T,
}

const CACHE_SIZE: usize = 32;

/// A cache for fast pointer lookups.
#[derive(Debug)]
struct Cache<T> {
    store: UnsafeCell<[(usize, *mut T); CACHE_SIZE]>,
    cur: AtomicUsize,
}

unsafe impl<T: Send + Sync> Send for Cache<T> {}
unsafe impl<T: Send + Sync> Sync for Cache<T> {}

impl<T> Cache<T> {
    fn new(ptr: *mut T) -> Self {
        let cache = Cache {
            // Initialize the cache with null pointers.
            // SAFETY: this is safe because the array only contains null pointers at first.
            store: unsafe { MaybeUninit::zeroed().assume_init() },
            cur: AtomicUsize::new(0),
        };

        cache.put(0, ptr);
        cache
    }

    /// Store a pointer in the cache.
    #[inline]
    fn put(&self, key: usize, ptr: *mut T) {
        let cur = self.cur.fetch_add(1, Ordering::Relaxed);

        unsafe {
            (*self.store.get())[cur % CACHE_SIZE] = (key, ptr);
        }
    }

    // Get a pointer from the cache.
    #[inline]
    fn get(&self, key: usize) -> *mut T {
        for i in 0..CACHE_SIZE {
            let (k, v) = unsafe { (*self.store.get())[i] };

            if k == key {
                return v;
            }
        }

        ptr::null_mut()
    }
}

/// A thread-safe linked list.
#[derive(Debug)]
pub struct List<T> {
    head: AtomicPtr<Block<T>>,
    tail: AtomicPtr<Block<T>>,
    cache: Cache<Block<T>>,
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
            cache: Cache::new(ptr),
            len: Mutex::new(1),
        }
    }

    /// Return the length of the list
    #[inline]
    pub fn len(&self) -> usize {
        *self.len.lock()
    }

    /// Is the list empty?
    #[inline]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Append an element to the back of the list.
    /// This operation is O(1) and thread-safe.
    pub fn append(&self, value: T) {
        // Allocate the block
        let block = Box::new(Block {
            next: AtomicPtr::new(ptr::null_mut()),
            value,
        });
        let ptr: *mut Block<T> = Box::leak(block);

        // Lock the list and update the pointers
        let mut lock = self.len.lock();
        let tail = unsafe { self.tail.load(Ordering::SeqCst).as_ref().unwrap() };

        tail.next.store(ptr, Ordering::SeqCst);
        self.tail.store(ptr, Ordering::SeqCst);

        // Update the length
        *lock += 1;
    }

    /// Return a reference to an element at the given index.
    /// Return None if the index is out of bounds.
    /// This operation is O(n) and thread-safe.
    pub fn get(&self, index: usize) -> Option<&T> {
        let mut current = unsafe { self.head.load(Ordering::SeqCst).as_mut().unwrap() };

        // Check the cache
        let ptr = self.cache.get(index);
        if !ptr.is_null() {
            return unsafe { Some(&(*ptr).value) };
        }

        for _ in 0..index {
            match unsafe { current.next.load(Ordering::SeqCst).as_mut() } {
                None => return None,
                Some(next) => current = next,
            }
        }

        // Update the cache
        self.cache.put(index, current);

        Some(&current.value)
    }

    // Return a reference to the last element of the list.
    #[inline]
    pub fn tail(&self) -> &T {
        let ptr = unsafe { &*self.tail.load(Ordering::Relaxed) };

        &ptr.value
    }

    /// Create a new head -> tail list iterator
    #[allow(dead_code)]
    pub fn iter(&self) -> ListIterator<T> {
        ListIterator { cursor: &self.head }
    }
}

impl<T> Drop for List<T> {
    fn drop(&mut self) {
        let mut current = self.head.load(Ordering::SeqCst);

        loop {
            let next = unsafe { (*current).next.load(Ordering::SeqCst) };

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

    use fremkit_macro::with_loom;
    use log::debug;

    use super::*;

    use crate::sync::thread;
    use crate::types::Notifier;

    use std::sync::Arc;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    #[with_loom]
    fn test_list_basics() {
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

    #[test]
    #[with_loom]
    fn test_list_iter() {
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

    #[test]
    #[with_loom]
    fn test_list_multi_thread_append() {
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

    #[test]
    #[with_loom]
    fn test_cache_basics() {
        init();

        let cache = Cache::new(ptr::null_mut());

        for i in 0..10 {
            cache.put(i, i as *mut u8);
        }

        for i in 0..10 {
            assert_eq!(cache.get(i), i as *mut u8);
        }
    }

    #[test]
    #[with_loom]
    fn test_rollover_basics() {
        init();

        let cache = Cache::new(ptr::null_mut());

        for i in 0..CACHE_SIZE {
            cache.put(i, (i + 10) as *mut u8);
        }

        assert_eq!(cache.get(0), 10 as *mut u8);
        cache.put(666, 666 as *mut u8);
        assert_eq!(cache.get(0), ptr::null_mut());
        assert_eq!(cache.get(666), 666 as *mut u8);
    }

    #[test]
    #[with_loom]
    fn test_rollover_complete() {
        init();

        let cache = Cache::new(ptr::null_mut());

        for i in 0..(CACHE_SIZE * 2) {
            cache.put(i, i as *mut u8);
        }

        // assert that all previous keys got rolled over
        for i in 0..CACHE_SIZE {
            assert_eq!(cache.get(i), ptr::null_mut());
        }

        // assert that all new keys are in the cache
        for i in CACHE_SIZE..(CACHE_SIZE * 2) {
            assert_eq!(cache.get(i), i as *mut u8);
        }
    }
}
