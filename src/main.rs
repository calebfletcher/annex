#![no_std]
#![no_main]

mod panic;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    #[allow(clippy::empty_loop)]
    loop {}
}
