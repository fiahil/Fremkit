use log::{debug, trace};
use parking_lot::{RwLock, RwLockUpgradableReadGuard, RwLockWriteGuard};
use std::{
    cell::UnsafeCell,
    collections::LinkedList,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::NonNull,
    sync::Arc,
};

use sync::{yield_now, AtomicBool, AtomicPtr, AtomicUsize, Mutex, MutexGuard, Ordering};

const CAPACITY: usize = 32;

mod sync {
    #[cfg(loom)]
    pub(crate) use loom::{
        sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering},
        sync::{Mutex, MutexGuard, RwLock},
        thread::yield_now,
    };

    #[cfg(not(loom))]
    pub(crate) use std::{
        sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering},
        sync::{Mutex, MutexGuard, RwLock},
        thread::yield_now,
    };
}

unsafe impl<T: Send> Send for MyList<T> {}
unsafe impl<T: Sync> Sync for MyList<T> {}

struct Slot<T> {
    data: [UnsafeCell<MaybeUninit<T>>; CAPACITY],
}

pub struct MyList<T> {
    vec: UnsafeCell<LinkedList<Pin<Box<Slot<T>>>>>,
    lock: AtomicBool,
    len: AtomicUsize,
}

impl<T> Slot<T> {
    fn new() -> Self {
        unsafe { MaybeUninit::zeroed().assume_init() }
    }
}

impl<T> MyList<T> {
    pub fn new() -> Self {
        Self {
            vec: UnsafeCell::new(LinkedList::new()),
            lock: AtomicBool::new(false),
            len: AtomicUsize::new(0),
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        let vec: &LinkedList<_> = unsafe { &*self.vec.get() };

        if index >= self.len.load(Ordering::SeqCst) {
            return None;
        }

        unsafe {
            vec.iter()
                .enumerate()
                .find(|(i, _)| i == &(index / CAPACITY))
                .map(|(_, t)| {
                    t.as_ref().get_ref().data[index % CAPACITY]
                        .get()
                        .as_ref()
                        .unwrap()
                        .as_ptr()
                        .as_ref()
                        .unwrap()
                })
        }
    }

    pub fn push(&self, value: T) {
        let token = self.len.fetch_add(1, Ordering::SeqCst);

        while let Err(_) =
            self.lock
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        {
            yield_now();
        }

        let vec: &mut LinkedList<_> = unsafe { &mut *self.vec.get() };

        if token / CAPACITY >= vec.len() {
            // New slot
            vec.push_back(Box::pin(Slot::new()));
        }

        unsafe {
            let r = vec.back_mut().unwrap();
            let x = r.as_ref().get_ref();
            (&mut *x.data[token % CAPACITY].get())
                .as_mut_ptr()
                .write(value)
        }

        self.lock.store(false, Ordering::SeqCst);
    }
}

unsafe impl<T: Send> Send for MyVec<T> {}
unsafe impl<T: Sync> Sync for MyVec<T> {}

pub struct MyVec<T> {
    vec: Mutex<Vec<Arc<T>>>,
    // vec: UnsafeCell<Vec<Pin<Box<T>>>>,
    // vec: UnsafeCell<Vec<*mut T>>,
    // lock: Mutex<bool>,
}

impl<T> MyVec<T> {
    pub fn new() -> Self {
        Self {
            vec: Mutex::new(Vec::new()),
            // lock: Mutex::new(false),
        }
    }

    pub fn get(&self, index: usize) -> Option<Arc<T>> {
        // let vec: &Vec<_> = unsafe { &*self.vec.get() };
        let vec = self.vec.lock().unwrap();

        vec.get(index).cloned()
        // vec.get(index).map(|p| p.as_ref().get_ref())
        // vec.get(index).and_then(|p| unsafe { p.as_ref() })
    }

    pub fn push(&self, value: T) {
        // TODO: slow...
        // let pin = Box::pin(value);
        let arc = Arc::from(value);

        // let vec: &mut Vec<_> = unsafe { &mut *self.vec.get() };
        let mut vec = self.vec.lock().unwrap();

        // while let Err(_) =
        //     self.lock
        //         .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        // {
        //     yield_now();
        // }

        // let lock = self.lock.lock().unwrap();

        // vec.push(&mut value);
        // vec.push(pin);
        vec.push(arc);

        // drop(lock);

        // self.lock.store(false, Ordering::SeqCst);
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
