extern crate clap;

mod setup;

use anyhow::Result;
use log::info;

fn main() -> Result<()> {
    let _ = setup::setup();

    info!("Bless the Maker and his water.");

    Ok(())
}
