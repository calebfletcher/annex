#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(type_alias_impl_trait)]
#![feature(const_mut_refs)]
#![feature(alloc_error_handler)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]
// This is bad, but in the early stages it is going to be used everywhere
// in the kernel
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(clippy::new_without_default)]

pub mod acpi;
pub mod allocator;
pub mod colour;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod screen;
pub mod serial;
pub mod task;
pub mod user;

#[allow(unused_imports)]
pub mod test;
pub mod timer;

extern crate alloc;

pub fn init(framebuffer: &'static mut bootloader::boot_info::FrameBuffer) {
    let frame_buffer = framebuffer;
    let buffer_info = frame_buffer.info();
    let buffer = frame_buffer.buffer_mut();

    // Initialise screen
    let mut screen = screen::Screen::new(buffer, buffer_info);
    screen.clear(colour::BLACK);

    // Initialise text console
    init_terminal(screen);

    gdt::init();
    interrupts::init_idt();

    // Disable PIC interrupts since we're using the APIC
    unsafe { interrupts::PICS.lock().initialize() };
    unsafe { interrupts::PICS.lock().write_masks(0xFF, 0xFF) };

    x86_64::instructions::interrupts::enable();
}

fn init_terminal(screen: screen::Screen<'static>) {
    let console = screen::Terminal::new(screen);

    screen::TERMINAL
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
    init(info.framebuffer.as_mut().unwrap());
    test_main();
    hlt_loop();
}
