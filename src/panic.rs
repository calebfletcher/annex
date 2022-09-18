use core::panic::PanicInfo;

use log::error;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("kernel panic :(");
    if let Some(&message) = info.message() {
        error!("  '{}'", message);
    }
    if let Some(&payload) = info.payload().downcast_ref::<&str>() {
        error!("  '{}'", payload);
    }
    if let Some(location) = info.location() {
        error!("  {}", location);
    }

    crate::abort();
}
