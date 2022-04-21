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

    // Initialise screen
    let mut screen = screen::Screen::new(buffer, buffer_info);
    screen.clear(colour::BLACK);

    // Initialise text console
    init_console(screen);

    println!("something {}", 1. / 3.);
    println!("{}", MSG);

    dbg!(colour::BLACK);

    for i in 0..100 {
        println!("row {}", i);
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

fn init_console(screen: screen::Screen<'static>) {
    let console = screen::Console::new(screen);

    screen::CONSOLE
        .try_init_once(move || spin::mutex::SpinMutex::new(console))
        .unwrap();
}
