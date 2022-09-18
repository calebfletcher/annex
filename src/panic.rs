use core::panic::PanicInfo;

use log::error;

use crate::{print, println};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("kernel panic :(");
    print!("panicked at: ");

    if let Some(&message) = info.message() {
        print!("'{}', ", message);
    }
    if let Some(&payload) = info.payload().downcast_ref::<&str>() {
        print!("'{}', ", payload);
    }
    if let Some(location) = info.location() {
        // Ignore error from write macro
        print!("{}", location);
    }
    println!();

    crate::abort();
}
