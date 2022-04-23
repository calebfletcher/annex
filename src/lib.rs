#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]
// This is bad, but in the early stages it is going to be used everywhere
// in the kernel
#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod colour;
pub mod gdt;
pub mod interrupts;
pub mod screen;
pub mod serial;

pub mod acpi;
#[allow(unused_imports)]
pub mod test;
pub mod timer;

pub fn init(framebuffer: &'static mut bootloader::boot_info::FrameBuffer) {
    let frame_buffer = framebuffer;
    let buffer_info = frame_buffer.info();
    let buffer = frame_buffer.buffer_mut();

    // Initialise screen
    let mut screen = screen::Screen::new(buffer, buffer_info);
    screen.clear(colour::BLACK);

    // Initialise text console
    init_console(screen);

    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };

    x86_64::instructions::interrupts::enable();
}

fn init_console(screen: screen::Screen<'static>) {
    let console = screen::Console::new(screen);

    screen::CONSOLE
        .try_init_once(move || spin::mutex::SpinMutex::new(console))
        .unwrap();
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(test)]
bootloader::entry_point!(entry_point);
#[cfg(test)]
fn entry_point(info: &'static mut bootloader::BootInfo) -> ! {
    init(info);
    test_main();
    hlt_loop();
}
