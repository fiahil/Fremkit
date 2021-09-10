use canal;

use canal::sync::Cooldown;
use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion, Throughput};
use crossbeam_channel as crossbeam;
use flume;
use parking_lot::Mutex;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::Instant;

trait Sender<T>: Send + Sized + 'static {
    fn new() -> (Self, JoinHandle<()>);
    fn send(&mut self, msg: T);
    fn close(self);
}

trait Receiver<T>: Send + Sized + 'static {
    fn new() -> (JoinHandle<()>, Self);
    fn recv(&mut self, index: usize) -> T;
    fn close(self);
}

impl<T: Send + Sync + Clone + Default + 'static> Sender<T> for flume::Sender<T> {
    fn new() -> (Self, JoinHandle<()>) {
        let (tx, rx) = flume::unbounded();

        let handle = thread::spawn(move || while let Ok(_) = rx.recv() {});

        (tx, handle)
    }

    fn send(&mut self, msg: T) {
        flume::Sender::send(self, msg).unwrap();
    }

    fn close(self) {}
}

impl<T: Send + Sync + Clone + Default + 'static> Receiver<T> for flume::Receiver<T> {
    fn new() -> (JoinHandle<()>, Self) {
        let (tx, rx) = flume::unbounded();

        let handle = thread::spawn(move || while let Ok(_) = tx.send(Default::default()) {});

        (handle, rx)
    }

    fn recv(&mut self, _index: usize) -> T {
        flume::Receiver::recv(self).unwrap()
    }

    fn close(self) {}
}

impl<T: Send + Sync + Clone + Default + 'static> Sender<T> for crossbeam::Sender<T> {
    fn new() -> (Self, JoinHandle<()>) {
        let (tx, rx) = crossbeam::unbounded();

        let handle = thread::spawn(move || while let Ok(_) = rx.recv() {});

        (tx, handle)
    }

    fn send(&mut self, msg: T) {
        crossbeam::Sender::send(self, msg).unwrap();
    }

    fn close(self) {}
}

impl<T: Send + Sync + Clone + Default + 'static> Receiver<T> for crossbeam::Receiver<T> {
    fn new() -> (JoinHandle<()>, Self) {
        let (tx, rx) = crossbeam::unbounded();

        let handle = thread::spawn(move || while let Ok(_) = tx.send(Default::default()) {});

        (handle, rx)
    }

    fn recv(&mut self, _index: usize) -> T {
        crossbeam::Receiver::recv(self).unwrap()
    }

    fn close(self) {}
}

impl<T: Send + Sync + Clone + Default + 'static> Sender<T> for mpsc::Sender<T> {
    fn new() -> (Self, JoinHandle<()>) {
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || while let Ok(_) = rx.recv() {});

        (tx, handle)
    }

    fn send(&mut self, msg: T) {
        mpsc::Sender::send(self, msg).unwrap();
    }

    fn close(self) {}
}

impl<T: Send + Sync + Clone + Default + 'static> Receiver<T> for mpsc::Receiver<T> {
    fn new() -> (JoinHandle<()>, Self) {
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || while let Ok(_) = tx.send(Default::default()) {});

        (handle, rx)
    }

    fn recv(&mut self, _index: usize) -> T {
        mpsc::Receiver::recv(self).unwrap()
    }

    fn close(self) {}
}

impl<T: Send + Sync + Clone + Default + 'static> Sender<T> for bus::Bus<T> {
    fn new() -> (Self, JoinHandle<()>) {
        let mut bus = bus::Bus::new(10000);
        let mut rx = bus.add_rx();

        let handle = thread::spawn(move || while let Ok(_) = rx.recv() {});

        (bus, handle)
    }

    fn send(&mut self, msg: T) {
        bus::Bus::broadcast(self, msg);
    }

    fn close(self) {}
}

struct MyBusReader<T> {
    bus: Arc<Mutex<bus::Bus<T>>>,
    reader: bus::BusReader<T>,
    closed: Arc<AtomicBool>,
}

impl<T: Send + Sync + Clone + Default + 'static> Receiver<T> for MyBusReader<T> {
    fn new() -> (JoinHandle<()>, Self) {
        let bus = Arc::new(Mutex::new(bus::Bus::new(10000)));
        let rx = bus.lock().add_rx();

        let c = Arc::new(AtomicBool::new(true));

        let reader = MyBusReader {
            bus: bus.clone(),
            reader: rx,
            closed: c.clone(),
        };

        let handle = thread::spawn(move || {
            while c.load(Ordering::Relaxed) {
                bus.lock().broadcast(Default::default())
            }
        });

        (handle, reader)
    }

    fn recv(&mut self, _index: usize) -> T {
        bus::BusReader::recv(&mut self.reader).unwrap()
    }

    fn close(self) {
        self.closed.store(false, Ordering::Relaxed);
    }
}

impl<T: Send + Sync + Clone + Default + 'static> Clone for MyBusReader<T> {
    fn clone(&self) -> Self {
        Self {
            bus: self.bus.clone(),
            reader: self.bus.lock().add_rx(),
            closed: self.closed.clone(),
        }
    }
}

impl<T: Send + Sync + Clone + Default + 'static> Sender<T> for canal::Canal<T> {
    fn new() -> (Self, JoinHandle<()>) {
        let canal = canal::Canal::new();
        let c1 = canal.clone();

        let handle = thread::spawn(move || {
            let mut i = 0;

            loop {
                c1.get_blocking(i);
                i += 1;
            }
        });

        (canal, handle)
    }

    fn send(&mut self, msg: T) {
        canal::Canal::add(self, msg).unwrap();
    }

    fn close(self) {
        canal::Canal::close(&self);
    }
}

impl<T: Send + Sync + Clone + Default + 'static> Receiver<T> for canal::Canal<T> {
    fn new() -> (JoinHandle<()>, Self) {
        let canal = canal::Canal::new();
        let c1 = canal.clone();

        let handle = thread::spawn(move || while let Ok(_) = c1.add(Default::default()) {});

        (handle, canal)
    }

    fn recv(&mut self, index: usize) -> T {
        canal::Canal::get_blocking(self, index)
    }

    fn close(self) {
        canal::Canal::close(&self);
    }
}

//
// TEST
//

fn test_sender<S: Sender<T>, T: Default>(b: &mut Bencher) {
    let (mut s, _) = S::new();

    b.iter(|| {
        s.send(Default::default());
    });

    s.close();
}

fn test_receiver<R: Receiver<T>, T>(b: &mut Bencher) {
    let (_, mut r) = R::new();
    let mut i = 0;

    b.iter(|| {
        black_box(r.recv(i));
        i += 1;
    });

    r.close();
}

fn test_one_to_many<R: Receiver<T> + Clone, T>(b: &mut Bencher, num_threads: usize) {
    b.iter_custom(|iters| {
        let (_, r) = R::new();
        let cd = Cooldown::new(num_threads as i32);

        let mut handles = Vec::new();
        for _ in 0..num_threads {
            let mut r = r.clone();
            let cd = cd.clone();

            let handle = thread::spawn(move || {
                cd.ready();

                // Warning: Channels are not broadcast!
                for i in 0..iters {
                    black_box(r.recv(i as usize));
                }
            });

            handles.push(handle);
        }

        cd.wait();
        let start = Instant::now();

        for h in handles {
            h.join().unwrap();
        }

        let elapsed = start.elapsed();

        r.close();
        elapsed
    });
}

fn sender(c: &mut Criterion) {
    let mut b = c.benchmark_group("sender");
    b.throughput(Throughput::Elements(1));

    b.bench_function("flume", |b| test_sender::<flume::Sender<u32>, u32>(b));
    b.bench_function("crossbeam", |b| {
        test_sender::<crossbeam::Sender<u32>, u32>(b)
    });
    b.bench_function("std", |b| test_sender::<mpsc::Sender<u32>, u32>(b));
    b.bench_function("bus", |b| test_sender::<bus::Bus<u32>, u32>(b));
    b.bench_function("canal", |b| test_sender::<canal::Canal<u32>, u32>(b));

    b.finish();
}

fn receiver(c: &mut Criterion) {
    let mut b = c.benchmark_group("receiver");
    b.throughput(Throughput::Elements(1));

    b.bench_function("flume", |b| test_receiver::<flume::Receiver<u32>, u32>(b));
    b.bench_function("crossbeam", |b| {
        test_receiver::<crossbeam::Receiver<u32>, u32>(b)
    });
    b.bench_function("std", |b| test_receiver::<mpsc::Receiver<u32>, u32>(b));
    b.bench_function("bus", |b| test_receiver::<MyBusReader<u32>, u32>(b));
    b.bench_function("canal", |b| test_receiver::<canal::Canal<u32>, u32>(b));

    b.finish();
}

fn one_to_many_8(c: &mut Criterion) {
    let mut b = c.benchmark_group("one_to_many_8");
    b.throughput(Throughput::Elements(1));

    b.bench_function("flume", |b| {
        test_one_to_many::<flume::Receiver<u32>, u32>(b, 8)
    });
    b.bench_function("crossbeam", |b| {
        test_one_to_many::<crossbeam::Receiver<u32>, u32>(b, 8)
    });
    b.bench_function("bus", |b| test_one_to_many::<MyBusReader<u32>, u32>(b, 8));
    b.bench_function("canal", |b| {
        test_one_to_many::<canal::Canal<u32>, u32>(b, 8)
    });

    b.finish();
}
fn one_to_many_32(c: &mut Criterion) {
    let mut b = c.benchmark_group("one_to_many_32");
    b.throughput(Throughput::Elements(1));

    b.bench_function("flume", |b| {
        test_one_to_many::<flume::Receiver<u32>, u32>(b, 32)
    });
    b.bench_function("crossbeam", |b| {
        test_one_to_many::<crossbeam::Receiver<u32>, u32>(b, 32)
    });
    b.bench_function("bus", |b| test_one_to_many::<MyBusReader<u32>, u32>(b, 32));
    b.bench_function("canal", |b| {
        test_one_to_many::<canal::Canal<u32>, u32>(b, 32)
    });

    b.finish();
}

criterion_group!(benches, sender, receiver, one_to_many_8, one_to_many_32);
criterion_main!(benches);
