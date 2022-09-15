#![no_std]
#![no_main]
#![feature(panic_info_message)]

global_asm!(include_str!("asm/boot.S"));

use core::{
    arch::{asm, global_asm},
    ffi::CStr,
};

use fdt::Fdt;

mod panic;
mod serial;

#[no_mangle]
pub extern "C" fn kmain(argc: usize, argv: usize) -> ! {
    if argc < 2 {
        panic!("no device tree passed in command line arguments");
    }
    let dtb = unsafe {
        // Horrible hack to get a c-style argv argument from uboot
        let dtb_addr_ptr = *(argv as *const u64).offset(1) as *const i8;
        let dtb_addr_str = CStr::from_ptr(dtb_addr_ptr).to_str().unwrap();
        let dtb_ptr = usize::from_str_radix(dtb_addr_str, 16).unwrap() as *const u8;
        println!("found device tree at {:p}", dtb_ptr);
        Fdt::from_ptr(dtb_ptr).unwrap()
    };

    entrypoint(dtb);
}

fn entrypoint(dtb: Fdt) -> ! {
    println!("Booting ANNEX Kernel");

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
