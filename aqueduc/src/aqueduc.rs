use std::collections::HashMap;
use std::fmt;

use bytes::Bytes;
use canal::Canal;
use log::debug;
use zmq::Context;

use crate::com::{Action, Program, Status};

/// An Aqueduc is a collection of Canals.
#[derive(Clone)]
pub struct Aqueduc {
    zmq_ctx: Context,
    log: Canal<Action>,
    canal: Canal<Bytes>,
}

impl Aqueduc {
    pub fn new() -> Self {
        let zmq_ctx = Context::new();

        Aqueduc {
            zmq_ctx,
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

impl fmt::Debug for Aqueduc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Aqueduc")
            .field("log", &self.log)
            .field("canal", &self.canal)
            .finish()
    }
}

#[cfg(test)]
mod test {
    use crate::com::{Command, Program};

    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_aqueduc() {
        init();

        let aq = Aqueduc::new();

        Command::Program(Program {
            cmd: "python3",
            args: &["00-hello.py"],
        })
        .execute(&aq);

        Command::Program(Program {
            cmd: "python3",
            args: &["01-world.py"],
        })
        .execute(&aq);

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
