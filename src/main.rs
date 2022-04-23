#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(panic_info_message)]
#![test_runner(annex::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

use annex::{colour, println, screen};

mod panic;

bootloader::entry_point!(entry_point);
fn entry_point(info: &'static mut bootloader::BootInfo) -> ! {
    let framebuffer = info.framebuffer.as_mut().unwrap();

    annex::init(framebuffer);
    println!("hello");

    // Run the tests if we're running under the test harness
    #[cfg(test)]
    test_main();

    let physical_memory_offset = info.physical_memory_offset.into_option().unwrap() as *const u8;
    let rsdp_addr =
        unsafe { physical_memory_offset.add(info.rsdp_addr.into_option().unwrap() as usize) };

    let rsdp = annex::acpi::rsdp::init(rsdp_addr);
    let rsdt = annex::acpi::rsdt::init(rsdp.rsdt_address(physical_memory_offset));
    println!("kernel loaded");
    annex::hlt_loop();
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
