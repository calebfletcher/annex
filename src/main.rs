#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(panic_info_message)]
#![test_runner(annex::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::arch::asm;

use annex::{
    println, screen,
    task::{executor::Executor, Task},
    threading::{self, thread::BlockReason},
};
use log::info;
use x86_64::{instructions::interrupts, PhysAddr, VirtAddr};

mod panic;

bootloader::entry_point!(entry_point);
fn entry_point(info: &'static mut bootloader::BootInfo) -> ! {
    annex::logger::init();

    let framebuffer = info.framebuffer.as_mut().unwrap();
    let rsdp_address = PhysAddr::new(info.rsdp_addr.into_option().unwrap());
    let physical_memory_offset = VirtAddr::new(info.physical_memory_offset.into_option().unwrap());
    let memory_regions = &info.memory_regions;
    annex::init(
        framebuffer,
        rsdp_address,
        physical_memory_offset,
        memory_regions,
    );

    info!("starting kernel");
    println!("starting kernel");

    // Run the tests if we're running under the test harness
    #[cfg(test)]
    test_main();

    threading::init();
    threading::add_thread(task2, 4096);
    threading::start();

    let mut executor = Executor::new();
    executor.spawn(Task::new(annex::task::keyboard::handle_keyboard()));
    executor.spawn(Task::new(annex::user::shell::run()));

    println!("loaded kernel");

    executor.run();
}

fn task2() -> ! {
    interrupts::enable();
    loop {
        unsafe {
            asm! {
                "
                mov dx, 0x3F8
                mov al, 0x41
                out dx, al
            ",
            }
        };
        threading::block_current_thread(BlockReason::Other);
        unsafe {
            asm! {
                "
                mov dx, 0x3F8
                mov al, 0x42
                out dx, al
            ",
            }
        };
    }
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
