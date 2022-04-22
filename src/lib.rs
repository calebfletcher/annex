#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

pub mod colour;
pub mod gdt;
pub mod interrupts;
pub mod screen;
pub mod serial;

#[allow(unused_imports)]
pub mod test;

pub fn init(info: &'static mut bootloader::BootInfo) {
    let frame_buffer = info.framebuffer.as_mut().unwrap();
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

#[cfg(test)]
bootloader::entry_point!(entry_point);
#[cfg(test)]
fn entry_point(info: &'static mut bootloader::BootInfo) -> ! {
    init(info);
    test_main();
    loop {}
}
