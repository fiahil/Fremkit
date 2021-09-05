use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion, Throughput};
use crossbeam_channel as crossbeam;
use flume;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};

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
    b: bus::BusReader<T>,
    c: Arc<AtomicBool>,
}

impl<T: Send + Sync + Clone + Default + 'static> Receiver<T> for MyBusReader<T> {
    fn new() -> (JoinHandle<()>, Self) {
        let mut bus = bus::Bus::new(10000);
        let rx = bus.add_rx();

        let c = Arc::new(AtomicBool::new(true));

        let reader = MyBusReader {
            b: rx,
            c: c.clone(),
        };

        let handle = thread::spawn(move || {
            while c.load(Ordering::Relaxed) {
                bus.broadcast(Default::default())
            }
        });

        (handle, reader)
    }

    fn recv(&mut self, _index: usize) -> T {
        bus::BusReader::recv(&mut self.b).unwrap()
    }

    fn close(self) {
        self.c.store(false, Ordering::Relaxed);
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

    b.iter(|| {
        black_box(r.recv(0));
    });

    r.close();
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
    // b.bench_function("aqueduc", |b| test_sender::<Arc<Canal>>(b));

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
    // b.bench_function("aqueduc", |b| test_receiver::<Arc<Canal>>(b));

    b.finish();
}

criterion_group!(benches, sender, receiver);
criterion_main!(benches);
