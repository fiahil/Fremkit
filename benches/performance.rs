use aqueduc::{Aqueduc, Canal, Droplet};
use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion, Throughput};
use crossbeam_channel as crossbeam;
use flume;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};

trait Sender: Send + Sized + 'static {
    fn new() -> (Self, JoinHandle<()>);
    fn send(&mut self, msg: Droplet);
    fn close(self);
}

trait Receiver: Send + Sized + 'static {
    fn new() -> (JoinHandle<()>, Self);
    fn recv(&mut self, index: usize) -> Droplet;
    fn close(self);
}

impl Sender for flume::Sender<Droplet> {
    fn new() -> (Self, JoinHandle<()>) {
        let (tx, rx) = flume::unbounded();

        let handle = thread::spawn(move || while let Ok(_) = rx.recv() {});

        (tx, handle)
    }

    fn send(&mut self, msg: Droplet) {
        flume::Sender::send(self, msg).unwrap();
    }

    fn close(self) {}
}

impl Receiver for flume::Receiver<Droplet> {
    fn new() -> (JoinHandle<()>, Self) {
        let (tx, rx) = flume::unbounded();

        let handle = thread::spawn(move || while let Ok(_) = tx.send(Default::default()) {});

        (handle, rx)
    }

    fn recv(&mut self, _index: usize) -> Droplet {
        flume::Receiver::recv(self).unwrap()
    }

    fn close(self) {}
}

impl Sender for crossbeam::Sender<Droplet> {
    fn new() -> (Self, JoinHandle<()>) {
        let (tx, rx) = crossbeam::unbounded();

        let handle = thread::spawn(move || while let Ok(_) = rx.recv() {});

        (tx, handle)
    }

    fn send(&mut self, msg: Droplet) {
        crossbeam::Sender::send(self, msg).unwrap();
    }

    fn close(self) {}
}

impl Receiver for crossbeam::Receiver<Droplet> {
    fn new() -> (JoinHandle<()>, Self) {
        let (tx, rx) = crossbeam::unbounded();

        let handle = thread::spawn(move || while let Ok(_) = tx.send(Default::default()) {});

        (handle, rx)
    }

    fn recv(&mut self, _index: usize) -> Droplet {
        crossbeam::Receiver::recv(self).unwrap()
    }

    fn close(self) {}
}

impl Sender for mpsc::Sender<Droplet> {
    fn new() -> (Self, JoinHandle<()>) {
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || while let Ok(_) = rx.recv() {});

        (tx, handle)
    }

    fn send(&mut self, msg: Droplet) {
        mpsc::Sender::send(self, msg).unwrap();
    }

    fn close(self) {}
}

impl Receiver for mpsc::Receiver<Droplet> {
    fn new() -> (JoinHandle<()>, Self) {
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || while let Ok(_) = tx.send(Default::default()) {});

        (handle, rx)
    }

    fn recv(&mut self, _index: usize) -> Droplet {
        mpsc::Receiver::recv(self).unwrap()
    }

    fn close(self) {}
}

impl Sender for bus::Bus<Droplet> {
    fn new() -> (Self, JoinHandle<()>) {
        let mut bus = bus::Bus::new(10000);
        let mut rx = bus.add_rx();

        let handle = thread::spawn(move || while let Ok(_) = rx.recv() {});

        (bus, handle)
    }

    fn send(&mut self, msg: Droplet) {
        bus::Bus::broadcast(self, msg);
    }

    fn close(self) {}
}

struct MyBusReader<T> {
    b: bus::BusReader<T>,
    c: Arc<AtomicBool>,
}

impl Receiver for MyBusReader<Droplet> {
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

    fn recv(&mut self, _index: usize) -> Droplet {
        bus::BusReader::recv(&mut self.b).unwrap()
    }

    fn close(self) {
        self.c.store(false, Ordering::Relaxed);
    }
}

//
// TEST
//

fn test_sender<S: Sender>(b: &mut Bencher) {
    let (mut s, _) = S::new();

    b.iter(|| {
        s.send(Default::default());
    });

    s.close();
}

fn test_receiver<R: Receiver>(b: &mut Bencher) {
    let (_, mut r) = R::new();

    b.iter(|| {
        black_box(r.recv(0));
    });

    r.close();
}

fn sender(c: &mut Criterion) {
    let mut b = c.benchmark_group("sender");
    b.throughput(Throughput::Elements(1));

    b.bench_function("flume", |b| test_sender::<flume::Sender<Droplet>>(b));
    b.bench_function("crossbeam", |b| {
        test_sender::<crossbeam::Sender<Droplet>>(b)
    });
    b.bench_function("std", |b| test_sender::<mpsc::Sender<Droplet>>(b));
    b.bench_function("bus", |b| test_sender::<bus::Bus<Droplet>>(b));
    // b.bench_function("aqueduc", |b| test_sender::<Arc<Canal>>(b));

    b.finish();
}

fn receiver(c: &mut Criterion) {
    let mut b = c.benchmark_group("receiver");
    b.throughput(Throughput::Elements(1));

    b.bench_function("flume", |b| test_receiver::<flume::Receiver<Droplet>>(b));
    b.bench_function("crossbeam", |b| {
        test_receiver::<crossbeam::Receiver<Droplet>>(b)
    });
    b.bench_function("std", |b| test_receiver::<mpsc::Receiver<Droplet>>(b));
    b.bench_function("bus", |b| test_receiver::<MyBusReader<Droplet>>(b));
    // b.bench_function("aqueduc", |b| test_receiver::<Arc<Canal>>(b));

    b.finish();
}

criterion_group!(benches, sender, receiver);
criterion_main!(benches);
