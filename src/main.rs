#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod colour;
mod screen;
mod serial;

#[cfg(not(test))]
mod panic;

#[cfg(test)]
mod test;

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

    // Run the tests if we're running under the test harness
    #[cfg(test)]
    test_main();

    for i in 0..100 {
        println!("row {}", i);
        delay(10);
    }

    panic!("kernel loaded");
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

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
