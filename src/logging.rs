use crate::opts::OPTS;
use fern::colors::{Color, ColoredLevelConfig};
use std::io;

pub fn init() {
    let mut base_config = fern::Dispatch::new();
    base_config = match OPTS.verbose.max(
        match std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "".to_string())
            .to_lowercase()
            .as_ref()
        {
            "trace" => 4,
            "debug" => 3,
            "info" => 2,
            "warn" => 1,
            _ => 0,
        },
    ) {
        0 => base_config.level(log::LevelFilter::Error),
        1 => base_config.level(log::LevelFilter::Warn),
        2 => base_config.level(log::LevelFilter::Info),
        3 => base_config.level(log::LevelFilter::Debug),
        _ => base_config.level(log::LevelFilter::Trace),
    };

    let colors_level = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Green);
    let stdout_config = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}{}: {} - {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                colors_level.color(record.level()),
                record.target(),
                message
            ))
        })
        .chain(io::stdout());

    base_config
        .chain(stdout_config)
        .apply()
        .expect("Could not initialize logger");
}
