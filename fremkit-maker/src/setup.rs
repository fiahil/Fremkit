use clap::Parser;
use env_logger::WriteStyle;
use log::LevelFilter;

/// Initialized the logger
fn loginit(level: LevelFilter) {
    env_logger::builder()
        .filter_level(level)
        .write_style(WriteStyle::Always)
        .format_timestamp_millis()
        .format_indent(Some(4))
        .init();
}

/// Command Line Interface definition
#[derive(Parser, Debug)]
#[clap(name = "maker")]
#[clap(author, about, version)]
pub struct Setup {
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: u8,
}

/// Parse command line arguments and initialize logger
pub fn setup() -> Setup {
    let setup = Setup::parse();

    match setup.verbose {
        0 => loginit(LevelFilter::Warn),
        1 => loginit(LevelFilter::Info),
        _ => loginit(LevelFilter::Debug),
    };

    setup
}
