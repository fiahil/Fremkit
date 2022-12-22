use std::fmt::Debug;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

use bus;
use fremkit::bounded::Log;

use criterion::measurement::WallTime;
use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkGroup, Criterion, Throughput,
};
use crossbeam_channel;
use multiqueue;
use parking_lot::{Mutex, RwLock};

//
// Trait Definitions
//

#[derive(Debug, Clone, Copy)]
struct LargeItem {
    _array: [u64; 1024],
}

impl Default for LargeItem {
    fn default() -> Self {
        LargeItem { _array: [0; 1024] }
    }
}

pub trait Item: Send + Sync + Debug + Default + Copy + 'static {}

impl Item for u64 {}
impl Item for LargeItem {}

pub trait Chan<T: Item>: Send + Clone + 'static {
    type Sender;
    type Receiver;

    fn new(capacity: usize) -> Self;
    fn read(&mut self, index: usize);
    fn write(&mut self, msg: T);
}

//
// VECTOR
//
impl<T: Item> Chan<T> for Arc<RwLock<Vec<T>>> {
    type Sender = Arc<RwLock<Vec<T>>>;
    type Receiver = Arc<RwLock<Vec<T>>>;

    fn new(capacity: usize) -> Self {
        Arc::new(RwLock::new(Vec::with_capacity(capacity)))
    }

    fn read(&mut self, index: usize) {
        let lock = RwLock::read(&self);

        black_box(lock.get(index));
    }

    fn write(&mut self, msg: T) {
        let mut lock = RwLock::write(&self);

        lock.push(msg);
    }
}

//
// CROSSBEAM
//

#[derive(Clone)]
struct CrossbeamChannel<T> {
    tx: crossbeam_channel::Sender<T>,
    rx: crossbeam_channel::Receiver<T>,
}

impl<T: Item> Chan<T> for CrossbeamChannel<T> {
    type Sender = crossbeam_channel::Sender<T>;
    type Receiver = crossbeam_channel::Receiver<T>;

    fn new(capacity: usize) -> Self {
        let (tx, rx) = crossbeam_channel::bounded(capacity);

        CrossbeamChannel { tx, rx }
    }

    fn read(&mut self, _index: usize) {
        black_box(self.rx.recv().ok());
    }

    fn write(&mut self, msg: T) {
        self.tx.send(msg).expect("crossbeam_channel write failed");
    }
}

//
// BUS
//

struct BusBroadcast<T> {
    bus: Arc<Mutex<bus::Bus<T>>>,
    rx: bus::BusReader<T>,
}

impl<T> Clone for BusBroadcast<T> {
    fn clone(&self) -> Self {
        BusBroadcast {
            bus: self.bus.clone(),
            rx: self.bus.lock().add_rx(),
        }
    }
}

impl<T: Item> Chan<T> for BusBroadcast<T> {
    type Sender = Arc<bus::Bus<T>>;
    type Receiver = Arc<bus::Bus<T>>;

    fn new(capacity: usize) -> Self {
        let mut b = bus::Bus::new(capacity);
        let rx = b.add_rx();

        let bus = Arc::new(Mutex::new(b));

        BusBroadcast { bus, rx }
    }

    fn read(&mut self, _index: usize) {
        black_box(self.rx.recv().ok());
    }

    fn write(&mut self, msg: T) {
        let mut lock = self.bus.lock();

        lock.broadcast(msg);
    }
}

//
// MULTIQUEUE
//

struct MultiqueueBroadcast<T: Clone> {
    mq: Arc<Mutex<multiqueue::BroadcastSender<T>>>,
    rx: multiqueue::BroadcastReceiver<T>,
}

impl<T: Clone> Clone for MultiqueueBroadcast<T> {
    fn clone(&self) -> Self {
        MultiqueueBroadcast {
            mq: self.mq.clone(),
            rx: self.rx.add_stream(),
        }
    }
}

impl<T: Item> Chan<T> for MultiqueueBroadcast<T> {
    type Sender = Arc<bus::Bus<T>>;
    type Receiver = Arc<bus::Bus<T>>;

    fn new(capacity: usize) -> Self {
        let (mq, rx) = multiqueue::broadcast_queue(capacity as u64);
        let mq = Arc::new(Mutex::new(mq));

        MultiqueueBroadcast { mq, rx }
    }

    fn read(&mut self, _index: usize) {
        black_box(self.rx.recv().ok());
    }

    fn write(&mut self, msg: T) {
        let lock = self.mq.lock();

        lock.try_send(msg).expect("multiqueue write failed");
    }
}

//
// Log
//
impl<T: Item> Chan<T> for Arc<Log<T>> {
    type Sender = Arc<Log<T>>;
    type Receiver = Arc<Log<T>>;

    fn new(capacity: usize) -> Self {
        Arc::new(Log::new(capacity))
    }

    fn read(&mut self, index: usize) {
        black_box(self.get(index));
    }

    fn write(&mut self, msg: T) {
        self.push(msg).expect("fremkit write failed");
    }
}

//
// Benchmark Helpers
//

fn single_thread_write<C: Chan<u64>>(b: &mut BenchmarkGroup<WallTime>, name: &str) {
    b.bench_function(name, |b| {
        b.iter_custom(|iters| {
            let mut c = C::new(iters as usize);

            let start = Instant::now();

            for i in 0..iters {
                c.write(i);
            }

            start.elapsed()
        });
    });
}

fn multi_thread_concurrent_write<T: Item, C: Chan<T>>(
    b: &mut BenchmarkGroup<WallTime>,
    name: &str,
    n_threads: usize,
) {
    b.bench_function(name, |b| {
        b.iter_custom(|iters| {
            let c = C::new(iters as usize * n_threads);

            let mut threads = Vec::with_capacity(n_threads);
            let barrier = Arc::new(Barrier::new(n_threads + 1));

            for _ in 0..n_threads {
                let b = barrier.clone();
                let mut tx = c.clone();

                let thread = thread::spawn(move || {
                    b.wait();

                    for _ in 0..iters {
                        tx.write(T::default());
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

fn multi_thread_concurrent_mixio<T: Item, C: Chan<T>>(
    b: &mut BenchmarkGroup<WallTime>,
    name: &str,
    n_threads: usize,
) {
    b.bench_function(name, |b| {
        b.iter_custom(|iters| {
            let c = C::new(iters as usize * n_threads * 2);

            let mut threads = Vec::with_capacity(n_threads * 2);
            let barrier = Arc::new(Barrier::new(n_threads * 2 + 1));

            for _ in 0..n_threads {
                let mut tx = c.clone();
                let mut rx = c.clone();

                // Writer Thread

                let b = barrier.clone();
                let thread = thread::spawn(move || {
                    b.wait();

                    for _ in 0..iters {
                        tx.write(T::default());
                    }
                });

                threads.push(thread);

                // Reader Thread

                let b = barrier.clone();
                let thread = thread::spawn(move || {
                    b.wait();

                    for i in 0..(iters as usize) {
                        rx.read(i);
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

fn bench_single_thread_write(c: &mut Criterion) {
    let mut b = c.benchmark_group("bounded_single_thread_write");
    b.throughput(Throughput::Elements(1));

    b.bench_function("vec", |b| {
        b.iter_custom(|iters| {
            let mut vec: Vec<u64> = Vec::with_capacity(iters as usize);
            let start = Instant::now();

            for i in 0..iters {
                vec.push(i);
            }

            start.elapsed()
        });
    });

    single_thread_write::<Arc<RwLock<Vec<u64>>>>(&mut b, "rwlock_vec");
    single_thread_write::<CrossbeamChannel<u64>>(&mut b, "crossbeam");
    single_thread_write::<BusBroadcast<u64>>(&mut b, "bus");
    single_thread_write::<MultiqueueBroadcast<u64>>(&mut b, "multiqueue");
    single_thread_write::<Arc<Log<u64>>>(&mut b, "log");

    b.finish();
}

fn bench_n(
    c: &mut Criterion,
    title: &str,
    n_threads: usize,
    fs: &[fn(&mut BenchmarkGroup<WallTime>, &str, usize)],
) {
    let mut b = c.benchmark_group(&format!("bounded_{n_threads}_{title}"));
    b.throughput(Throughput::Elements(n_threads as u64));

    fs[0](&mut b, "rwlock_vec", n_threads);
    fs[1](&mut b, "crossbeam", n_threads);
    fs[2](&mut b, "bus", n_threads);
    fs[3](&mut b, "multiqueue", n_threads);
    fs[4](&mut b, "log", n_threads);

    b.finish();
}

fn bench_2_thread_concurrent_write(c: &mut Criterion) {
    bench_n(
        c,
        "thread_concurrent_write",
        2,
        &[
            multi_thread_concurrent_write::<_, Arc<RwLock<Vec<u64>>>>,
            multi_thread_concurrent_write::<_, CrossbeamChannel<u64>>,
            multi_thread_concurrent_write::<_, BusBroadcast<u64>>,
            multi_thread_concurrent_write::<_, MultiqueueBroadcast<u64>>,
            multi_thread_concurrent_write::<_, Arc<Log<u64>>>,
        ],
    );
}

fn bench_4_thread_concurrent_write(c: &mut Criterion) {
    bench_n(
        c,
        "thread_concurrent_write",
        4,
        &[
            multi_thread_concurrent_write::<_, Arc<RwLock<Vec<u64>>>>,
            multi_thread_concurrent_write::<_, CrossbeamChannel<u64>>,
            multi_thread_concurrent_write::<_, BusBroadcast<u64>>,
            multi_thread_concurrent_write::<_, MultiqueueBroadcast<u64>>,
            multi_thread_concurrent_write::<_, Arc<Log<u64>>>,
        ],
    );
}

fn bench_8_thread_concurrent_write(c: &mut Criterion) {
    bench_n(
        c,
        "thread_concurrent_write",
        8,
        &[
            multi_thread_concurrent_write::<_, Arc<RwLock<Vec<u64>>>>,
            multi_thread_concurrent_write::<_, CrossbeamChannel<u64>>,
            multi_thread_concurrent_write::<_, BusBroadcast<u64>>,
            multi_thread_concurrent_write::<_, MultiqueueBroadcast<u64>>,
            multi_thread_concurrent_write::<_, Arc<Log<u64>>>,
        ],
    );
}

fn bench_2_thread_concurrent_mixio(c: &mut Criterion) {
    bench_n(
        c,
        "thread_concurrent_mixio",
        2,
        &[
            multi_thread_concurrent_mixio::<_, Arc<RwLock<Vec<u64>>>>,
            multi_thread_concurrent_mixio::<_, CrossbeamChannel<u64>>,
            multi_thread_concurrent_mixio::<_, BusBroadcast<u64>>,
            multi_thread_concurrent_write::<_, MultiqueueBroadcast<u64>>,
            multi_thread_concurrent_mixio::<_, Arc<Log<u64>>>,
        ],
    );
}

fn bench_4_thread_concurrent_mixio(c: &mut Criterion) {
    bench_n(
        c,
        "thread_concurrent_mixio",
        4,
        &[
            multi_thread_concurrent_mixio::<_, Arc<RwLock<Vec<u64>>>>,
            multi_thread_concurrent_mixio::<_, CrossbeamChannel<u64>>,
            multi_thread_concurrent_mixio::<_, BusBroadcast<u64>>,
            multi_thread_concurrent_write::<_, MultiqueueBroadcast<u64>>,
            multi_thread_concurrent_mixio::<_, Arc<Log<u64>>>,
        ],
    );
}

fn bench_4_thread_concurrent_large_item_mixio(c: &mut Criterion) {
    bench_n(
        c,
        "thread_concurrent_large_item_mixio",
        4,
        &[
            multi_thread_concurrent_mixio::<_, Arc<RwLock<Vec<LargeItem>>>>,
            multi_thread_concurrent_mixio::<_, CrossbeamChannel<LargeItem>>,
            multi_thread_concurrent_mixio::<_, BusBroadcast<LargeItem>>,
            multi_thread_concurrent_write::<_, MultiqueueBroadcast<LargeItem>>,
            multi_thread_concurrent_mixio::<_, Arc<Log<LargeItem>>>,
        ],
    );
}

fn bench_8_thread_concurrent_mixio(c: &mut Criterion) {
    bench_n(
        c,
        "thread_concurrent_mixio",
        8,
        &[
            multi_thread_concurrent_mixio::<_, Arc<RwLock<Vec<u64>>>>,
            multi_thread_concurrent_mixio::<_, CrossbeamChannel<u64>>,
            multi_thread_concurrent_mixio::<_, BusBroadcast<u64>>,
            multi_thread_concurrent_write::<_, MultiqueueBroadcast<u64>>,
            multi_thread_concurrent_mixio::<_, Arc<Log<u64>>>,
        ],
    );
}

criterion_group!(
    benches,
    bench_single_thread_write,
    bench_2_thread_concurrent_write,
    bench_4_thread_concurrent_write,
    bench_8_thread_concurrent_write,
    bench_2_thread_concurrent_mixio,
    bench_4_thread_concurrent_mixio,
    bench_4_thread_concurrent_large_item_mixio,
    bench_8_thread_concurrent_mixio
);
criterion_main!(benches);
