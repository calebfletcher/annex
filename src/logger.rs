use log::Level;

use crate::println;

const RESET: &str = "\x1B[0m";
const SUBTLE: &str = "\x1B[30;1m";

fn get_colour(level: log::Level) -> &'static str {
    match level {
        Level::Trace => "\x1B[36m",   // Cyan
        Level::Debug => "\x1B[34m",   // Blue
        Level::Info => "\x1B[32m",    // Green
        Level::Warn => "\x1B[33m",    // Yellow
        Level::Error => "\x1B[1;31m", // Bold red
    }
}

static LOGGER: Logger = Logger {};

pub fn init() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
}

struct Logger {}

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        println!(
            "{}[{}{}{:<5}{} {}:{}{}]{} {}",
            SUBTLE,
            RESET,
            get_colour(record.level()),
            record.level(),
            RESET,
            record
                .file()
                .unwrap_or("UNKNOWN")
                .trim_start_matches("src/"),
            record.line().unwrap_or(0),
            SUBTLE,
            RESET,
            record.args()
        );
    }

    fn flush(&self) {}
}
