use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use crate::screen;
    use core::fmt::Write;

    // If running in a debug build, force the kernel panic to be shown, regardless
    // of what was happening when it occurred. This can be used to diagnose issues
    // with the graphics system.
    #[cfg(debug_assertions)]
    unsafe {
        let cnsl = screen::TERMINAL.get_unchecked();
        cnsl.force_unlock();
    };

    // Try to print an error message if possible
    if let Ok(cnsl) = screen::TERMINAL.try_get() {
        if let Some(mut cnsl) = cnsl.try_lock() {
            writeln!(cnsl, "kernel panic :(").unwrap();
            write!(cnsl, "panicked at ").unwrap();

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
            writeln!(cnsl).unwrap();
        }
    }

    annex::hlt_loop();
}
