use crate::serial_println;

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

        serial_println!(
            "[{:<5} {}:{}] {}",
            record.level(),
            record.file().unwrap_or("UNKNOWN"),
            record.line().unwrap_or(0),
            record.args()
        );
    }

    fn flush(&self) {}
}
