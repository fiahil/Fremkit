use anyhow::Result;
use clap::Parser;
use log::info;
use zmq::{Context, Socket};

use libmaker::helpers;

/// Command Line Interface definition
#[derive(Parser, Debug)]
#[clap(name = "maker-client")]
#[clap(author, about, version)]
pub struct Setup {
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: u8,
}

struct Client {
    ctx: Context,
    sub_socket: Socket,
    dealer_socket: Socket,
}

impl Client {
    pub fn new() -> Result<Self> {
        let ctx = Context::new();
        let sub_socket = ctx.socket(zmq::SUB)?;
        let dealer_socket = ctx.socket(zmq::DEALER)?;

        sub_socket.set_subscribe(b"")?;

        sub_socket.connect("tcp://0.0.0.0:5555")?;
        dealer_socket.connect("tcp://0.0.0.0:5566")?;

        Ok(Client {
            ctx,
            sub_socket,
            dealer_socket,
        })
    }
}

fn main() -> Result<()> {
    let setup = Setup::parse();
    helpers::loginit(setup.verbose);

    let client = Client::new()?;

    info!("Client started");
    info!("> {:?}", "HI");

    client.dealer_socket.send("HI", 0)?;

    let msg = client.sub_socket.recv_msg(0)?;
    info!("< {:?}", msg.as_str().unwrap());

    let msg = client.dealer_socket.recv_msg(0)?;
    info!("< {:?}", msg.as_str().unwrap());

    Ok(())
}
