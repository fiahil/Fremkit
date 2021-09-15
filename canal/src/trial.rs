use log::{debug, trace};
use parking_lot::{RwLock, RwLockUpgradableReadGuard, RwLockWriteGuard};
use std::cell::UnsafeCell;

use sync::{yield_now, Arc, AtomicBool, AtomicPtr, AtomicUsize, Mutex, MutexGuard, Ordering};

const CAPACITY: usize = 12;

mod sync {
    #[cfg(loom)]
    pub(crate) use loom::{
        sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering},
        sync::{Arc, Mutex, MutexGuard},
        thread::yield_now,
    };

    #[cfg(not(loom))]
    pub(crate) use std::{
        sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering},
        sync::{Arc, Mutex, MutexGuard},
        thread::yield_now,
    };
}

unsafe impl<T> Sync for MyVec<T> {}

pub struct MyVec<T> {
    vec: UnsafeCell<Vec<T>>,
    lock: AtomicBool,
}

impl<T> MyVec<T> {
    pub fn new() -> Self {
        Self {
            vec: UnsafeCell::new(Vec::new()),
            lock: AtomicBool::new(false),
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        let vec: &Vec<T> = unsafe { &*self.vec.get() };

        vec.get(index)
    }

    pub fn push(&self, value: T) {
        while let Err(_) =
            self.lock
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        {
            yield_now();
        }

        let vec: &mut Vec<T> = unsafe { &mut *self.vec.get() };

        vec.push(value);

        self.lock.store(false, Ordering::SeqCst);
    }
}

pub struct MySuperBuffer<T> {
    pool: RwLock<Vec<MyBuffer<T>>>,
    cursor: AtomicUsize,
}

impl<T> MySuperBuffer<T> {
    pub fn new() -> Self {
        Self {
            pool: RwLock::new(Vec::new()),
            cursor: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, mut value: T) -> usize {
        // box pointer
        // let value = Box::into_raw(Box::new(value));
        // Retrieve the cursor for this element
        let cursor = self.cursor.fetch_add(1, Ordering::Relaxed);

        let index = cursor / CAPACITY;
        let cursor = cursor % CAPACITY;

        // We need to instert a new pool if we are at the end of the current one
        let guard = self.pool.upgradable_read();
        let guard = if index >= guard.len() {
            let mut guard = RwLockUpgradableReadGuard::upgrade(guard);

            guard.push(MyBuffer::new());
            RwLockWriteGuard::downgrade(guard)
        } else {
            RwLockUpgradableReadGuard::downgrade(guard)
        };
        // let guard = if cursor == 0 {
        //     let mut guard = self.pool.write();
        //     guard.push(MyBuffer::new());

        //     RwLockWriteGuard::downgrade(guard)
        // } else {
        //     self.pool.read()
        // };

        let buffer = &guard[index];

        buffer.push(cursor, &mut value);

        cursor
    }

    pub fn get(&self, cursor: usize) -> Option<&'static T> {
        let index = cursor / CAPACITY;
        let cursor = cursor % CAPACITY;

        let guard = self.pool.read();

        guard.get(index).and_then(|pool| pool.get(cursor))
    }

    pub fn len(&self) -> usize {
        self.cursor.load(Ordering::Relaxed)
    }
}

pub struct MyBuffer<T> {
    data: [AtomicPtr<T>; CAPACITY],
}

impl<T> MyBuffer<T> {
    pub fn new() -> Self {
        Self {
            data: array_init::array_init(|_| AtomicPtr::default()),
        }
    }

    pub fn get(&self, index: usize) -> Option<&'static T> {
        // if index >= CAPACITY {
        //     panic!("Index out of bounds");
        // }

        let ptr = self.data[index].load(Ordering::Relaxed);
        trace!("load @ {} | ptr = {:?}", index, ptr);

        // this operation is safe as long as we can guarantee that no reallocation will ever happen
        unsafe { ptr.as_ref() }
    }

    pub fn push(&self, index: usize, value: *mut T) {
        // if index >= CAPACITY {
        //     panic!("Buffer is full");
        // }

        self.data[index].store(value, Ordering::Relaxed);
        // trace!(" store @ {} | ptr = {:?}", index, value);
    }
}

pub struct MySimpleBuffer<T> {
    data: [AtomicPtr<T>; CAPACITY],
    off: AtomicUsize,
}

impl<T> MySimpleBuffer<T> {
    pub fn new() -> Self {
        Self {
            data: array_init::array_init(|_| AtomicPtr::default()),
            off: AtomicUsize::new(0),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.off.load(Ordering::Relaxed)
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        // if more than capacity, return None
        if index >= CAPACITY {
            return None;
        }

        let ptr = self.data[index].load(Ordering::Relaxed);
        trace!("load @ {} | ptr = {:?}", index, ptr);

        // this operation is safe as long as we can guarantee that no reallocation will ever happen
        unsafe { ptr.as_ref() }
    }

    pub fn push(&self, mut value: T) {
        let cell_idx = self.off.fetch_add(1, Ordering::Relaxed);

        // if cell_idx == CAPACITY {
        //     panic!("Buffer is full");
        // }

        self.data[cell_idx % CAPACITY].store(&mut value, Ordering::Relaxed);
        // trace!(" store @ {} | ptr = {:?}", cell_idx, value);
    }
}
