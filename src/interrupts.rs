use core::arch::asm;

use log::{debug, error};

pub fn init() {
    debug!("initialising trap scratch location");
    // TODO: put a location to store context in sscratch

    debug!("registering trap handler");
    unsafe {
        let value = handler as *const () as usize;
        if value & 0b11 != 0 {
            panic!("misaligned trap handler");
        }
        asm!("csrw stvec, {}", in(reg) value);
    }
}

// #[link_section = ".trap_handler"]
// #[no_mangle]
// #[naked]
// extern "C" fn handler() -> ! {
//     unsafe {
//         asm!(
//             // TODO: save context to location in sscratch register
//             //"csrrw t6, sscratch, t6",
//             // TODO: extract scause/stval/sepc
//             // TODO: dispatch
//             // TODO: restore context from sscratch register
//             "sret",
//             options(noreturn)
//         )
//     }
// }

#[link_section = ".trap_handler"]
#[no_mangle]
extern "C" fn handler() -> ! {
    let cause: usize;
    let value: usize;
    let epc: usize;
    unsafe {
        asm!("csrr {}, scause", out(reg) cause);
        asm!("csrr {}, stval", out(reg) value);
        asm!("csrr {}, sepc", out(reg) epc);
    }

    error!(
        "vector handler: cause={} value={} epc={:X}",
        cause, value, epc
    );

    crate::halt();
}
