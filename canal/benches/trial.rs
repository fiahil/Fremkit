use std::{sync::Arc, sync::Barrier, thread, time::Instant};

use canal::trial::{MySimpleBuffer, MySuperBuffer, MyVec};

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use crossbeam_channel::unbounded;
use parking_lot::RwLock;

fn basic(c: &mut Criterion) {
    let mut b = c.benchmark_group("basic");
    b.throughput(Throughput::Elements(1));

    b.bench_function("vec", |b| {
        let mut vec = Vec::new();

        b.iter(|| {
            vec.push(black_box(1));
        });
    });

    b.bench_function("trial", |b| {
        let vec = MyVec::new();

        b.iter(|| {
            vec.push(black_box(1));
        });
    });

    b.bench_function("crossbeam", |b| {
        let (tx, _rx) = unbounded();

        b.iter(|| {
            tx.send(black_box(1)).unwrap();
        });
    });

    b.finish();
}

fn basic_concurrent(c: &mut Criterion) {
    let mut b = c.benchmark_group("basic_concurrent");
    b.throughput(Throughput::Elements(1));

    b.bench_function("vec", |b| {
        let vec = Arc::new(RwLock::new(Vec::new()));

        b.iter_custom(|iters| {
            let v1 = vec.clone();
            let v2 = vec.clone();
            let b = Arc::new(Barrier::new(3));
            let b1 = b.clone();
            let b2 = b.clone();

            let h1 = thread::spawn(move || {
                b1.wait();

                for i in 0..iters {
                    let mut g = v1.write();

                    g.push(black_box(i));
                }
            });

            let h2 = thread::spawn(move || {
                b2.wait();

                for i in 0..iters {
                    let mut g = v2.write();

                    g.push(black_box(i));
                }
            });

            let start = Instant::now();
            b.wait();
            h1.join().unwrap();
            h2.join().unwrap();

            start.elapsed()
        });
    });

    b.bench_function("trial", |b| {
        let vec = Arc::new(MyVec::new());

        b.iter_custom(|iters| {
            let v1 = vec.clone();
            let v2 = vec.clone();
            let b = Arc::new(Barrier::new(3));
            let b1 = b.clone();
            let b2 = b.clone();

            let h1 = thread::spawn(move || {
                b1.wait();

                for i in 0..iters {
                    v1.push(black_box(i));
                }
            });

            let h2 = thread::spawn(move || {
                b2.wait();

                for i in 0..iters {
                    v2.push(black_box(i));
                }
            });

            let start = Instant::now();
            b.wait();
            h1.join().unwrap();
            h2.join().unwrap();

            start.elapsed()
        });
    });

    b.bench_function("crossbeam", |b| {
        let (tx, _rx) = unbounded();

        b.iter_custom(|iters| {
            let v1 = tx.clone();
            let v2 = tx.clone();
            let b = Arc::new(Barrier::new(3));
            let b1 = b.clone();
            let b2 = b.clone();

            let h1 = thread::spawn(move || {
                b1.wait();

                for i in 0..iters {
                    v1.send(black_box(i)).unwrap();
                }
            });

            let h2 = thread::spawn(move || {
                b2.wait();

                for i in 0..iters {
                    v2.send(black_box(i)).unwrap();
                }
            });

            let start = Instant::now();
            b.wait();
            h1.join().unwrap();
            h2.join().unwrap();

            start.elapsed()
        });
    });

    b.finish();
}

criterion_group!(benches, basic, basic_concurrent);
criterion_main!(benches);
