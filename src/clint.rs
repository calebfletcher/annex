use core::arch::asm;

use conquer_once::noblock::OnceCell;
use fdt::Fdt;
use log::debug;

static CLINT: OnceCell<Clint> = OnceCell::uninit();

struct Clint {
    interval: usize,
}

pub fn init(requested_interval_ns: usize, fdt: &Fdt) {
    // Find out the frequency of the CLINT timer
    let timebase_frequency = fdt
        .find_node("/cpus")
        .unwrap()
        .property("timebase-frequency")
        .unwrap()
        .as_usize()
        .unwrap();
    debug!("using clint timebase frequency of {timebase_frequency:?} Hz");

    // Calculate how many counts is needed for the requested interval
    let timebase_counts = (requested_interval_ns * timebase_frequency) / 1_000_000_000;
    debug!("timebase counts {timebase_counts}");

    // Initialise the CLINT static
    CLINT
        .try_init_once(|| Clint {
            interval: timebase_counts,
        })
        .unwrap();
}

pub fn start() {
    // Get current time
    let time: usize;
    unsafe {
        asm!("csrr {}, time", out(reg) time);
    }
    let next_time = time + CLINT.try_get().unwrap().interval;

    // Set timecmp to timebase_counts
    sbi::timer::set_timer(next_time as u64).unwrap();

    //(timebase_counts * 1_000_000_000) / timebase_frequency
}
