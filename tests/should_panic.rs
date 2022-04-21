#![no_std]
#![no_main]

use annex::{
    println, serial_println,
    test::{exit_qemu, QemuExitCode},
};
use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    should_fail();
    serial_println!("[test did not panic]");
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

fn should_fail() {
    // Console isn't set up, so we shouldn't be able to print yet
    println!("hello")
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}
