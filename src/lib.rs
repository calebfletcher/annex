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

fn entrypoint(fdt: Fdt) -> ! {
    println!("Booting ANNEX Kernel");

    println!(
        "This is a devicetree representation of a {}",
        fdt.root().model()
    );
    println!(
        "...which is compatible with at least: {}",
        fdt.root().compatible().first()
    );
    println!("...and has {} CPU(s)", fdt.cpus().count());
    println!(
        "...and has at least one memory location at: {:#X}\n",
        fdt.memory().regions().next().unwrap().starting_address as usize
    );

    let chosen = fdt.chosen();
    if let Some(bootargs) = chosen.bootargs() {
        println!("The bootargs are: {:?}", bootargs);
    }

    if let Some(stdout) = chosen.stdout() {
        println!("It would write stdout to: {}", stdout.name);
    }

    let soc = fdt.find_node("/soc");
    println!(
        "Does it have a `/soc` node? {}",
        if soc.is_some() { "yes" } else { "no" }
    );
    if let Some(soc) = soc {
        println!("...and it has the following children:");
        for child in soc.children() {
            println!("    {}", child.name);
        }
    }

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
