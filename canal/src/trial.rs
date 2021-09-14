use std::ptr;
use sync::{AtomicPtr, AtomicUsize, Ordering};

use log::debug;

const CAPACITY: usize = 12;

mod sync {
    #[cfg(loom)]
    pub(crate) use loom::sync::atomic::{fence, AtomicPtr, AtomicU8, AtomicUsize, Ordering};

    #[cfg(not(loom))]
    pub(crate) use std::sync::atomic::{fence, AtomicPtr, AtomicU8, AtomicUsize, Ordering};
}

pub struct MyVec<T> {
    head: AtomicPtr<MyNode<T>>,
    tail: AtomicPtr<MyNode<T>>,
    // len: AtomicUsize,
}

impl<T> MyVec<T> {
    pub fn new() -> Self {
        let first = MyNode::new();

        Self {
            head: AtomicPtr::new(first),
            tail: AtomicPtr::new(first),
            // len: AtomicUsize::new(0),
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        let first = self.head.load(Ordering::SeqCst);

        let mut i = 0;
        let mut node = first;
        while i < index && !node.is_null() {
            node = unsafe { (&*node).next.load(Ordering::SeqCst) };
            i += 1;
        }

        unsafe {
            node.as_ref()
                .and_then(|n| n.data.load(Ordering::SeqCst).as_ref())
        }
    }

    pub fn push(&self, value: T) {
        let boxed = Box::new(value);
        let new = MyNode::new();
        let old = self.tail.swap(new, Ordering::SeqCst);

        // TODO: if we pause (1) here, we can't read the value of (2)
        // TODO: until we resume (1). We rely on the `next` pointer to read values
        unsafe { old.as_ref().unwrap() }.set_next(new);
        unsafe { old.as_ref().unwrap() }.set_value(Box::into_raw(boxed));
    }
}

struct MyNode<T> {
    data: AtomicPtr<T>,
    next: AtomicPtr<MyNode<T>>,
}

impl<T> MyNode<T> {
    fn new() -> *mut Self {
        let s = Self {
            data: AtomicPtr::new(ptr::null_mut()),
            next: AtomicPtr::new(ptr::null_mut()),
        };

        Box::into_raw(Box::new(s))
    }

    #[inline]
    fn set_next(&self, next: *mut MyNode<T>) {
        self.next.store(next, Ordering::SeqCst);
    }

    #[inline]
    fn set_value(&self, value: *mut T) {
        self.data.store(value, Ordering::SeqCst);
    }
}

pub struct MyBuffer<T> {
    data: [AtomicPtr<T>; CAPACITY],
    off: AtomicUsize,
}

impl<T> MyBuffer<T> {
    pub fn new() -> Self {
        Self {
            data: array_init::array_init(|_| AtomicPtr::default()),
            off: AtomicUsize::new(0),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.off.load(Ordering::SeqCst)
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        // if more than capacity, return None
        if index >= CAPACITY {
            return None;
        }

        let ptr = self.data[index].load(Ordering::Acquire);
        debug!(
            "======================== load @ {} | ptr = {:?}",
            index, ptr
        );

        // this operation is safe as long as we can guarantee that no reallocation will ever happen
        unsafe { ptr.as_ref() }
    }

    pub fn push(&self, value: T) {
        let value = Box::into_raw(Box::new(value));
        let cell_idx = self.off.fetch_add(1, Ordering::SeqCst);

        // if cell_idx == CAPACITY {
        //     panic!("Buffer is full");
        // }

        self.data[cell_idx].store(value, Ordering::Release);
        debug!(
            "======================== store @ {} | ptr = {:?}",
            cell_idx, value
        );
    }
}
