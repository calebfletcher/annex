use core::panic::PanicInfo;

use crate::{colour, screen};
use core::fmt::Write;

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Try to print an error message if possible
    if let Ok(cnsl) = screen::CONSOLE.try_get() {
        if let Some(mut cnsl) = cnsl.try_lock() {
            let colour = screen::TextColour::new(colour::RED, colour::BLACK);
            cnsl.write_colour("kernel panic :(\n", colour);
            cnsl.write_colour("panicked at ", colour);

            if let Some(&message) = info.message() {
                let _: Result<_, _> = write!(cnsl, "'{}', ", message);
            }
            if let Some(&payload) = info.payload().downcast_ref::<&str>() {
                let _: Result<_, _> = write!(cnsl, "'{}', ", payload);
            }
            if let Some(location) = info.location() {
                // Ignore error from write macro
                let _: Result<_, _> = write!(cnsl, "{}", location);
            }
            cnsl.write_colour("\n", colour);
        }
    }

    loop {}
}
