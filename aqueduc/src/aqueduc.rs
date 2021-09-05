use std::sync::Arc;
use std::thread::{self, JoinHandle};

use dashmap::DashMap;
use lazy_static::lazy_static;
use log::trace;

use crate::core::{Canal, CanalId, Droplet};

lazy_static! {
    static ref AQ: Aqueduc = Aqueduc::new();
}

/// An Aqueduc is a collection of Canals. It is the main entry point for
/// creating Canals and spawning threads.
#[derive(Debug, Clone)]
pub struct Aqueduc {
    db: Arc<DashMap<CanalId, Arc<Canal>>>,
}

impl Aqueduc {
    pub fn new() -> Aqueduc {
        Aqueduc {
            db: Arc::new(DashMap::new()),
        }
    }

    /// Open a canal or create it, if it doesn't exist.
    pub fn canal(canal_id: CanalId) -> Arc<Canal> {
        AQ.db
            .entry(canal_id)
            .or_insert_with(|| Arc::new(Canal::new(canal_id)))
            .clone()
    }

    /// Select multiple canals according to a selector.
    /// TODO: unimplemented
    pub fn select(_selector: &str) -> Vec<Arc<Canal>> {
        unimplemented!()
    }

    /// Spawn a new thread that will wait for new droplets on a Canal and call
    /// the given callback for each new addition.
    /// This is equivalent to `listen_after(canal_id, 0, f, initial_state)`
    pub fn listen<F, S>(canal_id: CanalId, callback: F, initial_state: S) -> JoinHandle<S>
    where
        F: Fn(&mut S, Droplet) -> bool + Send + 'static,
        S: Send + 'static,
    {
        Aqueduc::listen_after(canal_id, 0, callback, initial_state)
    }

    /// Spawn a new thread that will wait for new droplets on a Canal and call
    /// the given Callback for each new addition starting at the given index.
    /// The loop will stop when the callback returns `false`.
    ///
    /// * `canal_id`: the Canal to listen to
    /// * `index`: the index to start listening from
    /// * `callback`: the function to call for each new addition
    /// * `initial_state`: the initial state to pass to the callback. Can be `()`.
    pub fn listen_after<F, S>(
        canal_id: CanalId,
        mut index: usize,
        callback: F,
        mut initial_state: S,
    ) -> JoinHandle<S>
    where
        F: Fn(&mut S, Droplet) -> bool + Send + 'static,
        S: Send + 'static,
    {
        let canal = Aqueduc::canal(canal_id);

        thread::spawn(move || {
            loop {
                canal.wait_for_droplet(index);

                match canal.get(index) {
                    Some(droplet) => {
                        if !callback(&mut initial_state, droplet) {
                            break;
                        }

                        index += 1;
                    }
                    None => {
                        // Do nothing: given index is out of bounds
                        // we wait for items to appear
                        trace!("listen_after: index out of bound for x = {}", index);
                    }
                }
            }
            initial_state
        })
    }

    /// Spawn a new thread that will call the given callback repeatedly, and
    /// add the resulting droplet to the given canal.
    /// The loop stops when the given function returns None.
    ///
    /// * `canal_id`: the Canal to add the droplet to
    /// * `callback`: the function to call for each iteration
    /// * `initial_state`: the initial state to pass to the callback. Can be `()`.
    pub fn spawn<F, S>(canal_id: CanalId, callback: F, mut initial_state: S) -> JoinHandle<S>
    where
        F: Fn(&mut S) -> Option<Droplet> + Send + 'static,
        S: Send + 'static,
    {
        let canal = Aqueduc::canal(canal_id);

        thread::spawn(move || {
            loop {
                let droplet = callback(&mut initial_state);

                match droplet {
                    Some(droplet) => canal.add_and_notify(droplet),
                    None => break,
                }
            }

            initial_state
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_aqueduc() {
        init();

        let j = Aqueduc::listen(
            "hello",
            |state, _droplet| {
                println!("> droplet-1 {}", state);

                *state += 1;
                *state <= 9
            },
            0,
        );

        let k = Aqueduc::listen_after(
            "hello",
            5,
            |state, _droplet| {
                println!("> droplet-2 {}", state);

                *state += 1;
                *state <= 4
            },
            0,
        );

        let i = Aqueduc::spawn(
            "hello",
            |state| {
                if *state < 10 {
                    *state += 1;
                    Some(Droplet::Data)
                } else {
                    None
                }
            },
            0,
        );

        let (i, j, k) = (i.join().unwrap(), j.join().unwrap(), k.join().unwrap());

        assert_eq!(i, 10);
        assert_eq!(j, 10);
        assert_eq!(k, 5);
    }
}
