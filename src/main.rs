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
    annex::init(info);
    println!("hello");

    // Run the tests if we're running under the test harness
    #[cfg(test)]
    test_main();

    println!("kernel loaded");
    annex::hlt_loop();
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
