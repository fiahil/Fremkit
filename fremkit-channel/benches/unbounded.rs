use std::fmt::Debug;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

use fremkit_channel::unbounded::UnboundedChannel;

use criterion::measurement::WallTime;
use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkGroup, Criterion, Throughput,
};
use crossbeam_channel;
use parking_lot::RwLock;

//
// Trait Definitions
//

pub trait Item: Send + Sync + Debug + Default + Copy + 'static {}

impl Item for u64 {}
impl Item for i32 {}

pub trait Tx: Send + Sized + Clone + 'static {
    type Item: Item;
    type Receiver: Rx;

    fn open() -> (Self, Self::Receiver);
    fn write(&mut self, msg: Self::Item);
}

pub trait Rx: Send + Sized + Clone + 'static {
    type Item: Item;
    type Sender: Tx;

    fn read(&mut self, index: usize) -> Option<Self::Item>;
}

//
// VECTOR
//
impl<T: Item> Tx for Arc<RwLock<Vec<T>>> {
    type Item = T;
    type Receiver = Arc<RwLock<Vec<T>>>;

    fn open() -> (Self, Self::Receiver) {
        let v = Arc::new(RwLock::new(Vec::new()));

        (v.clone(), v)
    }

    fn write(&mut self, msg: Self::Item) {
        let mut lock = RwLock::write(&self);

        lock.push(msg);
    }
}

impl<T: Item> Rx for Arc<RwLock<Vec<T>>> {
    type Item = T;
    type Sender = Arc<RwLock<Vec<T>>>;

    fn read(&mut self, index: usize) -> Option<Self::Item> {
        let lock = RwLock::read(&self);

        lock.get(index).cloned()
    }
}

//
// CROSSBEAM
//
impl<T: Item> Tx for crossbeam_channel::Sender<T> {
    type Item = T;
    type Receiver = crossbeam_channel::Receiver<T>;

    fn open() -> (Self, Self::Receiver) {
        let (tx, rx) = crossbeam_channel::unbounded();

        (tx, rx)
    }

    fn write(&mut self, msg: Self::Item) {
        self.send(msg).expect("crossbeam_channel write failed");
    }
}

impl<T: Item> Rx for crossbeam_channel::Receiver<T> {
    type Item = T;
    type Sender = crossbeam_channel::Sender<T>;

    fn read(&mut self, _index: usize) -> Option<Self::Item> {
        self.try_recv().ok()
    }
}

//
// UnboundedChannel
//
impl<T: Item> Tx for UnboundedChannel<T> {
    type Item = T;
    type Receiver = UnboundedChannel<T>;

    fn open() -> (Self, Self::Receiver) {
        let l = UnboundedChannel::new();

        (l.clone(), l)
    }

    fn write(&mut self, msg: Self::Item) {
        self.push(msg);
    }
}

impl<T: Item> Rx for UnboundedChannel<T> {
    type Item = T;
    type Sender = UnboundedChannel<T>;

    fn read(&mut self, index: usize) -> Option<Self::Item> {
        self.get(index).copied()
    }
}

//
// Benchmark Helpers
//

fn single_thread_append<T: Tx>(b: &mut BenchmarkGroup<WallTime>, name: &str) {
    b.bench_function(name, |b| {
        b.iter_custom(|iters| {
            let (mut tx, _rx) = T::open();

            let start = Instant::now();

            for _ in 0..iters {
                tx.write(black_box(Default::default()));
            }

            start.elapsed()
        });
    });
}

fn multi_thread_append<T: Tx>(b: &mut BenchmarkGroup<WallTime>, name: &str, n_threads: usize) {
    b.bench_function(name, |b| {
        b.iter_custom(|iters| {
            let (tx, _rx) = T::open();

            let mut threads = Vec::with_capacity(n_threads);
            let barrier = Arc::new(Barrier::new(n_threads + 1));

            for _ in 0..n_threads {
                let b = barrier.clone();
                let mut tx = tx.clone();

                let thread = thread::spawn(move || {
                    b.wait();

                    for _ in 0..iters {
                        tx.write(black_box(Default::default()));
                    }
                });

                threads.push(thread);
            }

            let start = Instant::now();
            barrier.wait();

            for thread in threads {
                thread.join().unwrap();
            }

            start.elapsed()
        });
    });
}

fn multi_thread_read<T: Tx>(b: &mut BenchmarkGroup<WallTime>, name: &str, n_threads: usize) {
    b.bench_function(name, |b| {
        b.iter_custom(|iters| {
            let (tx, rx) = T::open();

            let mut threads = Vec::with_capacity(n_threads);
            let barrier = Arc::new(Barrier::new(n_threads + 1));

            for _ in 0..n_threads {
                let b = barrier.clone();
                let mut rx = rx.clone();
                let mut tx = tx.clone();

                for _ in 0..iters {
                    tx.write(black_box(Default::default()));
                }

                let thread = thread::spawn(move || {
                    b.wait();

                    for i in 0..iters {
                        rx.read(i as usize);
                    }
                });

                threads.push(thread);
            }

            let start = Instant::now();
            barrier.wait();

            for thread in threads {
                thread.join().unwrap();
            }

            start.elapsed()
        });
    });
}

//
// Benchmark Scenarios
//

fn bench_single_thread_append(c: &mut Criterion) {
    let mut b = c.benchmark_group("unbounded_single_thread_append");
    b.throughput(Throughput::Elements(1));

    b.bench_function("vec", |b| {
        b.iter_custom(|iters| {
            let mut vec = Vec::new();
            let start = Instant::now();

            for _ in 0..iters {
                vec.push(black_box(1u64));
            }

            start.elapsed()
        });
    });

    single_thread_append::<Arc<RwLock<Vec<u64>>>>(&mut b, "rwlock_vec");
    single_thread_append::<crossbeam_channel::Sender<u64>>(&mut b, "crossbeam");
    single_thread_append::<UnboundedChannel<u64>>(&mut b, "my_channel");

    b.finish();
}

fn bench_2_thread_append(c: &mut Criterion) {
    let mut b = c.benchmark_group("unbounded_2_thread_append");
    b.throughput(Throughput::Elements(2));

    multi_thread_append::<Arc<RwLock<Vec<u64>>>>(&mut b, "rwlock_vec", 2);
    multi_thread_append::<crossbeam_channel::Sender<u64>>(&mut b, "crossbeam", 2);
    multi_thread_append::<UnboundedChannel<u64>>(&mut b, "my_channel", 2);

    b.finish();
}

fn bench_8_thread_append(c: &mut Criterion) {
    let mut b = c.benchmark_group("unbounded_8_thread_append");
    b.throughput(Throughput::Elements(8));

    multi_thread_append::<Arc<RwLock<Vec<u64>>>>(&mut b, "rwlock_vec", 8);
    multi_thread_append::<crossbeam_channel::Sender<u64>>(&mut b, "crossbeam", 8);
    multi_thread_append::<UnboundedChannel<u64>>(&mut b, "my_channel", 8);

    b.finish();
}

fn bench_2_thread_read(c: &mut Criterion) {
    let mut b = c.benchmark_group("unbounded_2_thread_read");
    b.throughput(Throughput::Elements(2));

    multi_thread_read::<Arc<RwLock<Vec<u64>>>>(&mut b, "rwlock_vec", 2);
    multi_thread_read::<crossbeam_channel::Sender<u64>>(&mut b, "crossbeam", 2);
    multi_thread_read::<UnboundedChannel<u64>>(&mut b, "my_channel", 2);

    b.finish();
}

fn bench_8_thread_read(c: &mut Criterion) {
    let mut b = c.benchmark_group("unbounded_8_thread_read");
    b.throughput(Throughput::Elements(8));

    multi_thread_read::<Arc<RwLock<Vec<u64>>>>(&mut b, "rwlock_vec", 8);
    multi_thread_read::<crossbeam_channel::Sender<u64>>(&mut b, "crossbeam", 8);
    multi_thread_read::<UnboundedChannel<u64>>(&mut b, "my_channel", 8);

    b.finish();
}

criterion_group!(
    benches,
    bench_single_thread_append,
    bench_2_thread_append,
    bench_8_thread_append,
    bench_2_thread_read,
    bench_8_thread_read
);
criterion_main!(benches);
