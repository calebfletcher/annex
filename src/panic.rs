use annex::{serial_print, serial_println};
use core::panic::PanicInfo;
use x86_64::instructions::interrupts;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    interrupts::disable();

    serial_println!("kernel panic :(");
    serial_print!("panicked at ");

    if let Some(&message) = info.message() {
        serial_print!("'{}', ", message);
    }
    if let Some(&payload) = info.payload().downcast_ref::<&str>() {
        serial_print!("'{}', ", payload);
    }
    if let Some(location) = info.location() {
        // Ignore error from write macro
        serial_print!("{}", location);
    }
    serial_println!();

    annex::hlt_loop();
}
