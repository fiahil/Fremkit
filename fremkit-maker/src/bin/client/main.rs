use std::sync::{Arc, Mutex};

use anyhow::Result;
use clap::Parser;
use crossbeam_channel::unbounded;
use libmaker::core::snapshot::Snapshot;
use log::info;

use libmaker::helpers;
use libmaker::net::client::Client;

use crate::input::Input;
use crate::network::Network;

mod input;
mod network;

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

    let state = Arc::new(Mutex::new(Snapshot::new()));

    let client = Client::new("0.0.0.0", state.clone())?;
    let (tx_qry, rx_qry) = unbounded();
    let (tx_cmd, rx_cmd) = unbounded();

    info!("Client started!");

    let net = Network::new(client, rx_qry, rx_cmd);
    let inp = Input::new(state, tx_qry, tx_cmd);

    let j1 = inp.start();
    let _ = net.start();

    j1.join().expect("input thread panicked")
}
