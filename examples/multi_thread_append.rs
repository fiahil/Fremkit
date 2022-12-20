use std::collections::HashMap;
use std::io::stdin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use fremkit::bounded::Log;
use fremkit::ChannelError;

const THREADS: usize = 8;

pub fn main() {
    // Setting up a channel
    let log = Arc::new(Log::new(50_000_000));

    // Setting up threads
    let mut threads: Vec<thread::JoinHandle<Result<(), ChannelError<u64>>>> =
        Vec::with_capacity(THREADS);
    let barrier = Arc::new(Barrier::new(THREADS + 1));
    let alarm = Arc::new(AtomicBool::new(false));

    for id in 0..THREADS {
        let b = barrier.clone();
        let lg = log.clone();
        let alr = alarm.clone();

        // Each thread will try to push as many items as possible
        // into the channel before the timer stops.
        let thread = thread::spawn(move || {
            b.wait();

            loop {
                lg.push(id as u64)?;

                if alr.load(Ordering::Relaxed) {
                    break;
                }
            }

            Ok(())
        });

        threads.push(thread);
    }

    println!("> {} threads ready!", threads.len());
    println!("> Press enter to start...");
    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();

    println!("> GO!");

    let start = std::time::Instant::now();
    barrier.wait();

    thread::sleep(Duration::from_secs(1));

    // Ring the bell!
    alarm.store(true, Ordering::Relaxed);

    let elapsed = start.elapsed();
    println!("> Elapsed: {:?}s", elapsed.as_secs_f32());

    // Counting the number of items in the channel
    let tally = log.iter().fold(HashMap::new(), |mut tally, &id| {
        *tally.entry(id).or_insert(0) += 1;
        tally
    });

    for (id, th) in threads.into_iter().enumerate() {
        th.join().unwrap().unwrap();
        println!("> Thread {} pushed {} items", id, tally[&(id as u64)]);
    }

    println!(
        "> Thread {} wins!",
        tally.iter().max_by_key(|(_, &v)| v).unwrap().0
    );
}
