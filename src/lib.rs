#![no_std]
#![no_main]

global_asm!(include_str!("asm/boot.S"));

use core::{
    arch::{asm, global_asm},
    panic::PanicInfo,
};

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    unsafe {
        let uart_ptr = 0x10000000 as *mut u8;
        uart_ptr.write_volatile(b'A');
        uart_ptr.write_volatile(b'N');
        uart_ptr.write_volatile(b'N');
        uart_ptr.write_volatile(b'E');
        uart_ptr.write_volatile(b'X');
    }

    abort();
}

#[panic_handler]
fn panic_handler(_info: &PanicInfo) -> ! {
    abort();
}

#[no_mangle]
extern "C" fn abort() -> ! {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}
