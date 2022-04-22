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

    for i in 0..10 {
        println!("row {}", i);
        delay(10);
    }

    panic!("kernel loaded");
}

fn delay(factor: usize) {
    let value = 0;
    for _ in 0..factor * 1000000 {
        unsafe {
            core::ptr::read_volatile(&value);
        }
    }
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
