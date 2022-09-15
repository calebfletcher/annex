#![no_std]
#![no_main]
#![feature(panic_info_message)]

global_asm!(include_str!("asm/boot.S"));

use core::arch::{asm, global_asm};

mod panic;
mod serial;

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    println!("\n\nBooting ANNEX Kernel\n\n");

    panic!("kernel terminated");
}

#[no_mangle]
extern "C" fn abort() -> ! {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}
