#![no_std]
#![no_main]

mod colour;
mod panic;
mod screen;

static MSG: &str = "Hello World!";

bootloader::entry_point!(entry_point);
fn entry_point(info: &'static mut bootloader::BootInfo) -> ! {
    let frame_buffer = info.framebuffer.as_mut().unwrap();
    let buffer_info = frame_buffer.info();
    let buffer = frame_buffer.buffer_mut();

    let mut screen = screen::Screen::new(buffer, &buffer_info);
    screen.clear(colour::BLACK);

    screen.write_chars(MSG, 0, 0, colour::WHITE);

    #[allow(clippy::empty_loop)]
    loop {}
}
