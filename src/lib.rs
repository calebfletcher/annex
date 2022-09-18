#![no_std]
#![no_main]
#![feature(panic_info_message)]

global_asm!(include_str!("asm/boot.S"));

use core::arch::{asm, global_asm};

use fdt::Fdt;
use log::{debug, error, info, warn};
use sbi::system_reset::{ResetReason, ResetType};

mod logger;
mod panic;
mod serial;

#[no_mangle]
pub extern "C" fn kmain(hart_id: usize, fdt_addr: usize) -> ! {
    let fdt = unsafe { Fdt::from_ptr(fdt_addr as *const u8).unwrap() };

    entrypoint(hart_id, fdt);
}

fn entrypoint(hart_id: usize, fdt: Fdt) -> ! {
    logger::init();

    info!("Booting ANNEX Kernel");
    debug!("Currently running on hart {}", hart_id);

    debug!("Hart Status:");
    for hart in fdt.cpus().flat_map(|cpu| cpu.ids().all()) {
        debug!("  {}: {:?}", hart, sbi::hsm::hart_status(hart).unwrap());
    }

    warn!("kernel terminated");
    sbi::system_reset::system_reset(ResetType::Shutdown, ResetReason::NoReason).unwrap();
    unreachable!("kernel exit");
}

#[no_mangle]
extern "C" fn abort() -> ! {
    error!("aborting execution");
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}
