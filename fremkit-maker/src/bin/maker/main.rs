use anyhow::Result;
use clap::Parser;
use log::info;

use libmaker::helpers;
use libmaker::net::server::Server;

/// Command Line Interface definition
#[derive(Parser, Debug)]
#[clap(name = "maker")]
#[clap(author, about, version)]
pub struct Setup {
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: u8,
}

fn main() -> Result<()> {
    let setup = Setup::parse();
    helpers::loginit(setup.verbose);

    let server = Server::new("0.0.0.0")?;

    info!("Server started!");
    server.run()?;

    Ok(())
}
