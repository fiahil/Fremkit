use std::io::{stdin, stdout, Write};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crossbeam_channel::Sender;
use libmaker::core::state::State;
use libmaker::protocol::command::Command;
use libmaker::protocol::query::Query;

use anyhow::Result;

pub struct Input {
    state: Arc<Mutex<State>>,
    tx_qry: Sender<Query>,
    tx_cmd: Sender<Command>,
}

impl Input {
    /// Create a new Input that will listen for commands
    /// given on the command line
    pub fn new(state: Arc<Mutex<State>>, tx_qry: Sender<Query>, tx_cmd: Sender<Command>) -> Self {
        Self {
            state,
            tx_qry,
            tx_cmd,
        }
    }

    /// Start a new thread that will interact with the command line
    pub fn start(self) -> JoinHandle<Result<()>> {
        thread::spawn(move || {
            loop {
                disp("> ")?;

                match stdread()?
                    .as_str()
                    .split(' ')
                    .collect::<Vec<_>>()
                    .as_slice()
                {
                    ["w"] => {
                        println!("COM: show");
                        println!("{:#?}", *self.state.lock().unwrap());
                        println!("OK!");
                    }
                    ["q", "k"] => {
                        println!("COM: checksum");
                        let lock = self.state.lock().unwrap();
                        let command = Query::Checksum(lock.checksum());

                        self.tx_qry.send(command)?;

                        println!("OK!");
                    }
                    ["q", "s"] => {
                        println!("COM: snapshot");
                        self.tx_qry.send(Query::Snapshot)?;

                        println!("OK!");
                    }
                    ["c", "p", key, val] => {
                        println!("COM: update");

                        self.tx_cmd.send(Command::Put {
                            key: key.to_string(),
                            val: val.to_string(),
                        })?;

                        println!("OK!");
                    }
                    ["c", "g", key, idx] => {
                        println!("COM: fetch");

                        self.tx_cmd.send(Command::Get {
                            key: key.to_string(),
                            idx: idx.parse()?,
                        })?;

                        println!("OK!");
                    }
                    ["e"] | [""] => {
                        println!("COM: exit");
                        println!("OK!");
                        break;
                    }
                    ["h"] | _ => {
                        println!("HELP:");

                        println!("h            |  (help)      show this help");
                        println!("w            |  (show)      show local state");
                        println!("q k          |  (checksum)  send a checksum validation query");
                        println!("q s          |  (snapshot)  fetch an up-to-date snapshot");
                        println!("c p key val  |  (update)    send an update to the server");
                        println!("c g key idx  |  (fetch)     get an artefact from the server");
                        println!("e            |  (exit)      exit the program");
                    }
                }
            }

            Ok(())
        })
    }
}

fn disp(s: &str) -> Result<()> {
    print!("{}", s);
    stdout().flush()?;

    Ok(())
}

fn stdread() -> Result<String> {
    let mut buf = String::new();
    stdin().read_line(&mut buf)?;

    Ok(buf.trim().to_string())
}
