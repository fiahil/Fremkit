use env_logger::WriteStyle;
use log::LevelFilter;

/// Initialized the logger
pub fn loginit(verbosity: u8) {
    let level = match verbosity {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };

    env_logger::builder()
        .filter_level(level)
        .write_style(WriteStyle::Always)
        .format_timestamp_millis()
        .format_indent(Some(4))
        .init();
}
