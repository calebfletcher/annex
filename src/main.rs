#![no_std]
#![no_main]

mod panic;


bootloader::entry_point!(entry_point);
fn entry_point(info: &'static mut bootloader::BootInfo) -> ! {
    #[allow(clippy::empty_loop)]
    loop {}
}
