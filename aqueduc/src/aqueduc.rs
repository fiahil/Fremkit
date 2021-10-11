use std::collections::HashMap;

use bytes::Bytes;
use canal::Canal;
use log::debug;

use crate::com::{Action, Program, Status};

/// An Aqueduc is a collection of Canals. It is the main entry point for
/// creating Canals and spawning threads.
#[derive(Debug, Clone)]
pub struct Aqueduc {
    log: Canal<Action>,
    canal: Canal<Bytes>,
}

impl Aqueduc {
    pub fn new() -> Self {
        Aqueduc {
            log: Canal::new(),
            canal: Canal::new(),
        }
    }

    pub fn get_canal(&self) -> Canal<Bytes> {
        self.canal.clone()
    }

    pub fn log(&self, action: Action) {
        self.log.push(action);
    }

    pub fn log_blocking_aggregate<F>(&self, f: F)
    where
        F: Fn(&HashMap<Program, Status>) -> bool,
    {
        let mut h = HashMap::new();

        for action in self.log.blocking_iter() {
            match *action {
                Action::Program(p, s) => {
                    let existing = h.insert(p, s);
                    debug!("{:?} : {:?} -> {:?}", p, existing, s);
                }
            };

            if !f(&h) {
                break;
            }
        }
    }
}

impl Default for Aqueduc {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use crate::com::{Command, Program};

    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    // #[test]
    fn test_aqueduc() {
        init();

        let aq = Aqueduc::new();

        Command::Program(Program {
            cmd: "python3",
            args: &["01-world.py"],
        })
        .execute(aq.clone());

        Command::Program(Program {
            cmd: "python3",
            args: &["00-hello.py"],
        })
        .execute(aq.clone());

        aq.log_blocking_aggregate(|state| {
            // return false once all programs are completed
            !state.values().all(|s| match s {
                Status::Started => false,
                Status::Completed(_) => true,
            })
        });

        for (i, b) in aq.canal.iter().enumerate() {
            println!("{}: {:?}", i, b);
        }
    }
}
