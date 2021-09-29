use std::fmt::Debug;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

use canal::Log;

use bus;
use criterion::measurement::WallTime;
use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkGroup, Criterion, Throughput,
};
use crossbeam_channel;
use parking_lot::RwLock;

trait Lx: Send + Sized + 'static {
    type Item: Default;
    type Sender: Tx;
    type Receiver: Rx;

    fn new_pair() -> (Self::Sender, Self::Receiver);
    fn new_tx(tx: &Self::Sender) -> Self::Sender;
    fn new_rx(rx: &Self::Receiver) -> Self::Receiver;
}

trait Tx: Send + Sized + 'static {
    type Item: Default;

    fn write(&mut self, msg: Self::Item);
}

trait Rx: Send + Sized + 'static {
    type Item: Default;

    fn read(&mut self, index: usize) -> Option<Self::Item>;
}

//
// VECTOR
//
impl<T: Send + Sync + Debug + Default + Clone + 'static> Lx for Vec<T> {
    type Item = T;
    type Sender = Arc<RwLock<Vec<T>>>;
    type Receiver = Arc<RwLock<Vec<T>>>;

    fn new_pair() -> (Self::Sender, Self::Receiver) {
        let v = Arc::new(RwLock::new(Vec::new()));

        (v.clone(), v)
    }

    fn new_tx(tx: &Self::Sender) -> Self::Sender {
        tx.clone()
    }

    fn new_rx(rx: &Self::Receiver) -> Self::Receiver {
        rx.clone()
    }
}

impl<T: Send + Sync + Debug + Default + 'static> Tx for Arc<RwLock<Vec<T>>> {
    type Item = T;

    fn write(&mut self, msg: Self::Item) {
        let mut lock = RwLock::write(&self);

        lock.push(msg);
    }
}

impl<T: Send + Sync + Debug + Default + Clone + 'static> Rx for Arc<RwLock<Vec<T>>> {
    type Item = T;

    fn read(&mut self, index: usize) -> Option<Self::Item> {
        let lock = RwLock::read(&self);

        lock.get(index).cloned()
    }
}

//
// CROSSBEAM
//
impl<T: Send + Sync + Debug + Default + 'static> Lx for crossbeam_channel::Sender<T> {
    type Item = T;
    type Sender = Self;
    type Receiver = crossbeam_channel::Receiver<T>;

    fn new_pair() -> (Self::Sender, Self::Receiver) {
        let (tx, rx) = crossbeam_channel::unbounded();

        (tx, rx)
    }

    fn new_tx(tx: &Self::Sender) -> Self::Sender {
        tx.clone()
    }

    fn new_rx(rx: &Self::Receiver) -> Self::Receiver {
        rx.clone()
    }
}

impl<T: Send + Sync + Debug + Default + 'static> Tx for crossbeam_channel::Sender<T> {
    type Item = T;

    fn write(&mut self, msg: Self::Item) {
        self.send(msg).unwrap();
    }
}

impl<T: Send + Sync + Debug + Default + 'static> Rx for crossbeam_channel::Receiver<T> {
    type Item = T;

    fn read(&mut self, _index: usize) -> Option<Self::Item> {
        self.try_recv().ok()
    }
}

//
// BUS
//
impl<T: Send + Sync + Clone + Debug + Default + 'static> Lx for bus::Bus<T> {
    type Item = T;
    type Sender = Self;
    type Receiver = bus::BusReader<T>;

    fn new_pair() -> (Self::Sender, Self::Receiver) {
        let mut b = bus::Bus::new(1_000_000);

        let rx = b.add_rx();

        (b, rx)
    }

    fn new_tx(_tx: &Self::Sender) -> Self::Sender {
        unimplemented!()
    }

    fn new_rx(_rx: &Self::Receiver) -> Self::Receiver {
        unimplemented!()
    }
}

impl<T: Send + Sync + Debug + Default + 'static> Tx for bus::Bus<T> {
    type Item = T;

    fn write(&mut self, msg: Self::Item) {
        let _ = self.try_broadcast(msg);
    }
}

impl<T: Send + Sync + Clone + Debug + Default + 'static> Rx for bus::BusReader<T> {
    type Item = T;

    fn read(&mut self, _index: usize) -> Option<Self::Item> {
        self.recv().ok()
    }
}

//
// LOG
//
impl<T: Send + Sync + Clone + Debug + Default + 'static> Lx for Log<T> {
    type Item = T;
    type Sender = Arc<Self>;
    type Receiver = Arc<Self>;

    fn new_pair() -> (Self::Sender, Self::Receiver) {
        let l = Arc::new(Log::new());

        (l.clone(), l)
    }

    fn new_tx(tx: &Self::Sender) -> Self::Sender {
        tx.clone()
    }

    fn new_rx(rx: &Self::Receiver) -> Self::Receiver {
        rx.clone()
    }
}

impl<T: Send + Sync + Debug + Default + 'static> Tx for Arc<Log<T>> {
    type Item = T;

    fn write(&mut self, msg: Self::Item) {
        self.push(msg);
    }
}

impl<T: Send + Sync + Clone + Debug + Default + 'static> Rx for Arc<Log<T>> {
    type Item = Arc<T>;

    fn read(&mut self, index: usize) -> Option<Self::Item> {
        self.get(index)
    }
}

fn single_thread_append<T: Lx>(b: &mut BenchmarkGroup<WallTime>, name: &str) {
    b.bench_function(name, |b| {
        let (mut tx, _rx) = T::new_pair();

        b.iter(|| {
            tx.write(black_box(Default::default()));
        });
    });
}

fn multi_thread_append<T: Lx>(b: &mut BenchmarkGroup<WallTime>, name: &str, n_threads: usize) {
    b.bench_function(name, |b| {
        let (tx, _rx) = T::new_pair();

        b.iter_custom(|iters| {
            let mut threads = Vec::with_capacity(n_threads);
            let barrier = Arc::new(Barrier::new(n_threads + 1));

            for _ in 0..n_threads {
                let b = barrier.clone();
                let mut tx = T::new_tx(&tx);

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

fn multi_thread_read<T: Lx>(b: &mut BenchmarkGroup<WallTime>, name: &str, n_threads: usize) {
    b.bench_function(name, |b| {
        let (tx, rx) = T::new_pair();

        b.iter_custom(|iters| {
            let mut threads = Vec::with_capacity(n_threads);
            let barrier = Arc::new(Barrier::new(n_threads + 1));

            for _ in 0..n_threads {
                let b = barrier.clone();
                let mut rx = T::new_rx(&rx);
                let mut tx = T::new_tx(&tx);

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

fn bench_single_thread_append(c: &mut Criterion) {
    let mut b = c.benchmark_group("single_thread_append");
    b.throughput(Throughput::Elements(1));

    b.bench_function("vec", |b| {
        let mut vec = Vec::new();

        b.iter(|| {
            vec.push(black_box(1u64));
        });
    });

    single_thread_append::<Vec<u64>>(&mut b, "rwlock_vec");
    single_thread_append::<crossbeam_channel::Sender<u64>>(&mut b, "crossbeam");
    single_thread_append::<bus::Bus<u64>>(&mut b, "bus");
    single_thread_append::<Log<u64>>(&mut b, "my_log");

    b.finish();
}

fn bench_single_thread_append_arc(c: &mut Criterion) {
    let mut b = c.benchmark_group("single_thread_append_arc");
    b.throughput(Throughput::Elements(1));

    b.bench_function("vec", |b| {
        let mut vec = Vec::new();

        b.iter(|| {
            vec.push(black_box(Arc::new(1u64)));
        });
    });

    single_thread_append::<Vec<Arc<u64>>>(&mut b, "rwlock_vec");
    single_thread_append::<crossbeam_channel::Sender<Arc<u64>>>(&mut b, "crossbeam");
    single_thread_append::<bus::Bus<Arc<u64>>>(&mut b, "bus");
    single_thread_append::<Log<Arc<u64>>>(&mut b, "my_log");

    b.finish();
}

fn bench_2_thread_append(c: &mut Criterion) {
    let mut b = c.benchmark_group("2_thread_append");
    b.throughput(Throughput::Elements(2));

    multi_thread_append::<Vec<u64>>(&mut b, "rwlock_vec", 2);
    multi_thread_append::<crossbeam_channel::Sender<u64>>(&mut b, "crossbeam", 2);
    multi_thread_append::<Log<u64>>(&mut b, "my_log", 2);

    b.finish();
}

fn bench_8_thread_append(c: &mut Criterion) {
    let mut b = c.benchmark_group("8_thread_append");
    b.throughput(Throughput::Elements(8));

    multi_thread_append::<Vec<u64>>(&mut b, "rwlock_vec", 8);
    multi_thread_append::<crossbeam_channel::Sender<u64>>(&mut b, "crossbeam", 8);
    multi_thread_append::<Log<u64>>(&mut b, "my_log", 8);

    b.finish();
}

fn bench_2_thread_read(c: &mut Criterion) {
    let mut b = c.benchmark_group("2_thread_read");
    b.throughput(Throughput::Elements(2));

    multi_thread_read::<Vec<u64>>(&mut b, "rwlock_vec", 2);
    multi_thread_read::<crossbeam_channel::Sender<u64>>(&mut b, "crossbeam", 2);
    multi_thread_read::<Log<u64>>(&mut b, "my_log", 2);

    b.finish();
}

fn bench_8_thread_read(c: &mut Criterion) {
    let mut b = c.benchmark_group("8_thread_read");
    b.throughput(Throughput::Elements(8));

    multi_thread_read::<Vec<u64>>(&mut b, "rwlock_vec", 8);
    multi_thread_read::<crossbeam_channel::Sender<u64>>(&mut b, "crossbeam", 8);
    multi_thread_read::<Log<u64>>(&mut b, "my_log", 8);

    b.finish();
}

criterion_group!(
    benches,
    bench_single_thread_append,
    bench_single_thread_append_arc,
    bench_2_thread_append,
    bench_8_thread_append,
    bench_2_thread_read,
    bench_8_thread_read
);
criterion_main!(benches);
