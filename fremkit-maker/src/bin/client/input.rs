use std::io::{stdin, stdout, Write};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crossbeam_channel::Sender;
use libmaker::core::snapshot::Snapshot;
use libmaker::protocol::command::Command;

use anyhow::Result;

pub struct Input {
    state: Arc<Mutex<Snapshot>>,
    tx_com: Sender<Command>,
    tx_data: Sender<(String, String)>,
}

impl Input {
    /// Create a new Input that will listen for commands
    /// given on the command line
    pub fn new(
        state: Arc<Mutex<Snapshot>>,
        tx_com: Sender<Command>,
        tx_data: Sender<(String, String)>,
    ) -> Self {
        Self {
            state,
            tx_com,
            tx_data,
        }
    }

    /// Start a new thread that will interact with the command line
    pub fn start(self) -> JoinHandle<Result<()>> {
        thread::spawn(move || {
            loop {
                disp("> ")?;

                match stdread()?.as_str() {
                    "show" | "w" => {
                        println!("COM: show");
                        println!("{:#?}", *self.state.lock().unwrap());
                        println!("OK!");
                    }
                    "checksum" | "c" => {
                        println!("COM: checksum");
                        let lock = self.state.lock().unwrap();
                        let command = Command::Checksum(lock.checksum());

                        self.tx_com.send(command)?;

                        println!("OK!");
                    }
                    "snapshot" | "s" => {
                        println!("COM: snapshot");
                        self.tx_com.send(Command::Snapshot)?;

                        println!("OK!");
                    }
                    "update" | "u" => {
                        println!("COM: update");

                        disp("update key > ")?;
                        let key = stdread()?;

                        disp("update val > ")?;
                        let val = stdread()?;

                        self.tx_data.send((key, val))?;

                        println!("OK!");
                    }
                    "exit" | "" => {
                        println!("COM: exit");
                        println!("OK!");
                        break;
                    }
                    "help" | _ => {
                        println!("HELP:");

                        println!("show     | w :  show local state");
                        println!("checksum | c :  send a checksum validation command");
                        println!("snapshot | s :  get an up-to-date snapshot from the server");
                        println!("update   | u :  send an update to the server");
                        println!("exit         :  exit the program");
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
