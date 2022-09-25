#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(naked_functions)]
#![forbid(unsafe_op_in_unsafe_fn)]
#![forbid(unused_unsafe)]
#![forbid(clippy::not_unsafe_ptr_arg_deref)]

global_asm!(include_str!("asm/boot.S"));

use core::arch::{asm, global_asm};

use fdt::Fdt;
use log::{debug, error, info, warn};
use sbi::system_reset::{ResetReason, ResetType};

mod clint;
mod csr;
mod interrupts;
mod logger;
mod memory;
mod paging;
mod panic;
mod plic;
mod riscv;

#[no_mangle]
pub extern "C" fn kmain(hart_id: usize, fdt_addr: usize) -> ! {
    let fdt = unsafe { Fdt::from_ptr(fdt_addr as *const u8).unwrap() };

    entrypoint(hart_id, fdt);
}

fn entrypoint(hart_id: usize, fdt: Fdt) -> ! {
    logger::init(
        fdt.chosen()
            .stdout()
            .or_else(|| fdt.find_node("/soc/uart"))
            .unwrap()
            .reg()
            .unwrap()
            .next()
            .unwrap()
            .starting_address,
    );

    interrupts::init();

    info!("booting ANNEX kernel");
    debug!("currently running on hart {}", hart_id);

    debug!("hart status:");
    for hart in fdt.cpus().flat_map(|cpu| cpu.ids().all()) {
        debug!("  {}: {:?}", hart, sbi::hsm::hart_status(hart).unwrap());
    }

    memory::init(fdt.memory().regions());

    clint::init(1_000_000_000, &fdt);
    clint::start();

    plic::init(&fdt);

    abort();
}

#[allow(dead_code)]
fn abort() -> ! {
    error!("aborting execution");
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

#[allow(dead_code)]
fn halt() -> ! {
    warn!("kernel terminated");
    sbi::system_reset::system_reset(ResetType::Shutdown, ResetReason::NoReason).unwrap();
    abort();
}
