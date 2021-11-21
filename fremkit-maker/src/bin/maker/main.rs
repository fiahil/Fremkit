use anyhow::Result;
use clap::Parser;
use log::info;
use zmq::{Context, Socket};

use libmaker::helpers;

/// Command Line Interface definition
#[derive(Parser, Debug)]
#[clap(name = "maker")]
#[clap(author, about, version)]
pub struct Setup {
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: u8,
}

struct Server {
    ctx: Context,
    pub_socket: Socket,
    router_socket: Socket,
}

impl Server {
    pub fn new() -> Result<Self> {
        let ctx = Context::new();
        let pub_socket = ctx.socket(zmq::PUB)?;
        let router_socket = ctx.socket(zmq::ROUTER)?;

        pub_socket.bind("tcp://0.0.0.0:5555")?;
        router_socket.bind("tcp://0.0.0.0:5566")?;

        Ok(Server {
            ctx,
            pub_socket,
            router_socket,
        })
    }
}

fn main() -> Result<()> {
    let setup = Setup::parse();
    helpers::loginit(setup.verbose);

    let server = Server::new()?;

    loop {
        let identity = server.router_socket.recv_msg(0)?;
        let msg = server.router_socket.recv_msg(0)?;

        info!("< {:?}", msg.as_str().unwrap());
        info!("> {:?}", "Hello !");

        server.router_socket.send(identity, zmq::SNDMORE)?;
        server.router_socket.send("Hello ! 1", 0)?;
        server.pub_socket.send("Hello ! 2", 0)?;
    }

    Ok(())
}
