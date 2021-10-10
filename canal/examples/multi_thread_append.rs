use std::sync::{Arc, Barrier};
use std::thread;

use canal::Canal;
use log::{info, warn};

const THREADS: usize = 64;

pub fn main() {
    env_logger::init();

    let canal: Canal<u64> = Canal::new();

    let mut threads = Vec::with_capacity(THREADS);
    let barrier = Arc::new(Barrier::new(THREADS + 1));

    for _ in 0..THREADS {
        let b = barrier.clone();
        let tx = canal.clone();

        let thread = thread::spawn(move || {
            b.wait();

            for i in 0..1_000_000_000 {
                info!("idx: {} | val: {}", tx.push(i), i);
            }
        });

        threads.push(thread);
    }

    warn!("{} threads ready!", threads.len());
    barrier.wait();

    for thread in threads {
        thread.join().unwrap();
    }
}
