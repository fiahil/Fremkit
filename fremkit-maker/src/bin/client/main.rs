use std::io::{stdin, stdout, Write};

use anyhow::Result;
use clap::Parser;
use log::{debug, info};

use libmaker::helpers;
use libmaker::net::client::Client;
use libmaker::protocol::command::Command;

/// Command Line Interface definition
#[derive(Parser, Debug)]
#[clap(name = "maker-client")]
#[clap(author, about, version)]
pub struct Setup {
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: u8,
}

fn main() -> Result<()> {
    let setup = Setup::parse();
    helpers::loginit(setup.verbose);

    let mut client = Client::new("0.0.0.0")?;

    info!("Client started!");

    loop {
        print!("> ");
        stdout().flush()?;

        // Match stdin input to a command
        let mut buf = String::new();
        stdin().read_line(&mut buf)?;

        match buf.trim() {
            "checksum" | "c" => {
                debug!("COM: checksum");
                client.com(Command::Checksum(client.state.checksum()))?;
            }
            "snapshot" | "s" => {
                debug!("COM: snapshot");
                client.com(Command::Snapshot)?;
            }
            "exit" | "" => {
                debug!("COM: exit");
                break;
            }
            "help" | _ => {
                println!("HELP:");
                println!("checksum | c :  send a checksum validation command");
                println!("snapshot | s :  get an up-to-date snapshot from the server");
                println!("exit         :  exit the program");
            }
        }
    }

    Ok(())
}
