use core::fmt::Write;

use conquer_once::spin::OnceCell;
use log::Level;
use uart_16550::MmioSerialPort;

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

static LOGGER: OnceCell<Logger> = OnceCell::uninit();

pub fn init(uart_addr: *const u8) {
    // Initialise UART
    LOGGER.init_once(|| unsafe {
        let mut uart = MmioSerialPort::new(uart_addr as usize);
        uart.init();
        Logger {
            uart: spin::Mutex::new(uart),
        }
    });

    // Initialise logger
    log::set_logger(LOGGER.get().unwrap()).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
}

struct Logger {
    uart: spin::Mutex<MmioSerialPort>,
}

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        writeln!(
            self.uart.lock(),
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
        )
        .unwrap();
    }

    fn flush(&self) {}
}
