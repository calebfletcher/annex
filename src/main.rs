#![no_std]
#![no_main]
#![feature(slice_as_chunks)]

mod colour;
mod panic;
mod screen;

use core::fmt::Write;

static MSG: &str = "Hello World!";

bootloader::entry_point!(entry_point);
fn entry_point(info: &'static mut bootloader::BootInfo) -> ! {
    let frame_buffer = info.framebuffer.as_mut().unwrap();
    let buffer_info = frame_buffer.info();
    let buffer = frame_buffer.buffer_mut();

    let mut screen = screen::Screen::new(buffer, &buffer_info);
    screen.clear(colour::BLACK);

    let mut console = screen::Console::new(screen);

    writeln!(console, "something {}", 1. / 3.).unwrap();
    writeln!(console, "{}", MSG).unwrap();

    for i in 0..100 {
        writeln!(console, "row {}", i).unwrap();
        delay(10);
    }

    #[allow(clippy::empty_loop)]
    loop {}
}

fn delay(factor: usize) {
    let value = 0;
    for _ in 0..factor * 1000000 {
        unsafe {
            core::ptr::read_volatile(&value);
        }
    }
}
