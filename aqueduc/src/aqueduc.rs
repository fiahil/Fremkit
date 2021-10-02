use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::{self, ExitStatus};
use std::thread;

use bytes::Bytes;
use canal::Canal;

/// An Aqueduc is a collection of Canals. It is the main entry point for
/// creating Canals and spawning threads.
#[derive(Debug)]
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

    pub fn command(&self, command: Command) {
        command.execute(self.log.clone(), self.canal.clone());
    }

    pub fn join_all(&self, minimum_count: usize) {
        let mut h = HashMap::new();

        for action in self.log.blocking_iter() {
            match *action {
                Action::ProgramStarted { source } => h.insert(source, "started"),
                Action::ProgramCompleted { source, .. } => h.insert(source, "completed"),
            };

            if h.len() >= minimum_count && h.values().all(|v| *v == "completed") {
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

#[derive(Debug)]
pub enum Action {
    ProgramStarted {
        source: Program,
    },

    ProgramCompleted {
        source: Program,
        outcome: ExitStatus,
    },
}

#[derive(Debug)]
pub enum Command {
    Program(Program),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Program {
    cmd: &'static str,
    args: &'static [&'static str],
}

impl Command {
    pub fn execute(self, log: Canal<Action>, canal: Canal<Bytes>) {
        match self {
            Command::Program(program) => {
                log.push(Action::ProgramStarted { source: program });

                thread::spawn(move || {
                    let mut process = process::Command::new(program.cmd)
                        .args(program.args)
                        .stdin(process::Stdio::piped())
                        .stdout(process::Stdio::piped())
                        .spawn()
                        .expect("failed to execute process");

                    // TODO: fix this ugly stuff
                    let mut stdin = process.stdin.take().expect("failed to take stdin");
                    let mut stdout = process.stdout.take().expect("failed to take stdout");
                    let c1 = canal.clone();
                    let c2 = canal.clone();
                    thread::spawn(move || {
                        let mut buf = Vec::new();
                        stdout.read_to_end(&mut buf).expect("failed to read stdout");

                        c1.push(Bytes::from(buf));
                    });
                    thread::spawn(move || {
                        let buf = c2.wait_for(0);
                        let _ = stdin.write_all(&buf);
                    });
                    // TODO: end of ugly stuff

                    // TODO: gather exist status when process is disconnected ?
                    let exit_status = process.wait().expect("failed to wait for process");

                    log.push(Action::ProgramCompleted {
                        source: program,
                        outcome: exit_status,
                    });
                });
            }
        }
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

        let aq = Aqueduc::new();

        aq.command(Command::Program(Program {
            cmd: "python3",
            args: &["01-world.py"],
        }));

        aq.command(Command::Program(Program {
            cmd: "python3",
            args: &["00-hello.py"],
        }));

        aq.join_all(2);

        for (i, b) in aq.canal.iter().enumerate() {
            println!("{}: {:?}", i, b);
        }
    }
}
