use core::arch::asm;

use log::{debug, warn};

use crate::{clint, riscv::instructions::instruction_size};

#[derive(Debug)]
#[repr(C)]
struct TrapContext {
    regs: [usize; 31],
    //fpu_regs: [usize; 32],
}

static mut TRAP_CONTEXT: TrapContext = TrapContext {
    regs: [0; 31],
    //fpu_regs: [0; 32],
};

pub fn init() {
    debug!("initialising trap scratch location");

    // Put a location to store context in sscratch
    let context = unsafe { &TRAP_CONTEXT as *const TrapContext };
    unsafe {
        asm!(
            "csrw sscratch, {}",
            in(reg) context
        );
    }

    debug!("registering trap handler");
    unsafe {
        let value = handler as *const () as usize;
        if value & 0b11 != 0 {
            panic!("misaligned trap handler");
        }
        asm!("csrw stvec, {}", in(reg) value);
    }

    let status: usize;
    unsafe {
        asm!("csrr {}, sie", out(reg) status);
    }
    debug!("sie {status:X}");

    let sie = 0b1000100010;
    debug!("enabling interrupts");
    unsafe {
        asm!("csrsi sstatus, 0b10");
        asm!("csrs sie, {}", in(reg) sie);
    }

    let status: usize;
    unsafe {
        asm!("csrr {}, sie", out(reg) status);
    }
    debug!("sie {status:X}");
}

#[link_section = ".trap_handler"]
#[no_mangle]
#[naked]
extern "C" fn handler() -> ! {
    unsafe {
        asm!(
            // swap sscratch with the last user register
            "csrrw x31, sscratch, x31",
            // save context to location in sscratch register
            "sd x1, 0(x31)",
            "sd x2, 8(x31)",
            "sd x3, 16(x31)",
            "sd x4, 24(x31)",
            "sd x5, 32(x31)",
            "sd x6, 40(x31)",
            "sd x7, 48(x31)",
            "sd x8, 56(x31)",
            "sd x9, 64(x31)",
            "sd x10, 72(x31)",
            "sd x11, 80(x31)",
            "sd x12, 88(x31)",
            "sd x13, 96(x31)",
            "sd x14, 104(x31)",
            "sd x15, 112(x31)",
            "sd x16, 120(x31)",
            "sd x17, 128(x31)",
            "sd x18, 136(x31)",
            "sd x19, 144(x31)",
            "sd x20, 152(x31)",
            "sd x21, 160(x31)",
            "sd x22, 168(x31)",
            "sd x23, 176(x31)",
            "sd x24, 184(x31)",
            "sd x25, 192(x31)",
            "sd x26, 200(x31)",
            "sd x27, 208(x31)",
            "sd x28, 216(x31)",
            "sd x29, 224(x31)",
            "sd x30, 232(x31)",
            // save original x31 and restore original sscratch value
            "csrrw x30, sscratch, x31",
            "sd x30, 240(x31)",
            // TODO: save floating point registers
            // TODO: look into making this pre-emptible, would need to save all the
            //       exception-related registers then re-enable higher-priority
            //       interrupts
            // extract scause/stval/sepc/sstatus
            "csrr a0, sepc",
            "csrr a1, stval",
            "csrr a2, scause",
            "csrr a3, sstatus",
            // dispatch to rust
            "call dispatch",
            // the dispatch function returned the new sepc value
            "csrw sepc, a0",
            // reload x31 with context address
            "csrr x31, sscratch",
            // TODO: restore floating point registers
            // restore context from sscratch register
            "ld x1, 0(x31)",
            "ld x2, 8(x31)",
            "ld x3, 16(x31)",
            "ld x4, 24(x31)",
            "ld x5, 32(x31)",
            "ld x6, 40(x31)",
            "ld x7, 48(x31)",
            "ld x8, 56(x31)",
            "ld x9, 64(x31)",
            "ld x10, 72(x31)",
            "ld x11, 80(x31)",
            "ld x12, 88(x31)",
            "ld x13, 96(x31)",
            "ld x14, 104(x31)",
            "ld x15, 112(x31)",
            "ld x16, 120(x31)",
            "ld x17, 128(x31)",
            "ld x18, 136(x31)",
            "ld x19, 144(x31)",
            "ld x20, 152(x31)",
            "ld x21, 160(x31)",
            "ld x22, 168(x31)",
            "ld x23, 176(x31)",
            "ld x24, 184(x31)",
            "ld x25, 192(x31)",
            "ld x26, 200(x31)",
            "ld x27, 208(x31)",
            "ld x28, 216(x31)",
            "ld x29, 224(x31)",
            "ld x30, 232(x31)",
            // restore x31, we don't need it after this
            "ld x31, 232(x31)",
            // return from exception
            "sret",
            options(noreturn)
        )
    }
}

#[no_mangle]
extern "C" fn dispatch(epc: usize, _tval: usize, cause: usize, _status: usize) -> usize {
    let is_interrupt = cause >> 63 == 1;
    let cause = cause & !(1 << 63);
    // warn!(
    //     "vector handler: interrupt={} cause={:X} value={:X} epc={:X} status={:X}",
    //     is_interrupt, cause, tval, epc, status
    // );

    if is_interrupt {
        match cause {
            1 => {
                // software
                warn!("software interrupt");
            }
            5 => {
                // timer
                debug!("timer tick");
                clint::start();
            }
            9 => {
                // external
                warn!("external interrupt");
            }
            _ => warn!("unknown or reserved interrupt"),
        }
    } else {
        match cause {
            0 => {
                warn!("instruction address misaligned");
            }
            1 => {
                warn!("instruction access fault");
            }
            2 => {
                warn!("illegal instruction");
            }
            3 => {
                warn!("breakpoint");
            }
            4 => {
                warn!("load address misaligned");
            }
            5 => {
                warn!("load access fault");
            }
            6 => {
                warn!("store/amo address misaligned");
            }
            7 => {
                warn!("store/amo access fault");
            }
            8 => {
                warn!("ecall from u-mode");
            }
            9 => {
                warn!("ecall from s-mode");
            }
            12 => {
                warn!("instruction page fault");
            }
            13 => {
                warn!("load page fault");
            }
            15 => {
                warn!("store/amo page fault");
            }
            _ => warn!("unhandled exception"),
        }
    }

    // return new epc
    epc + instruction_size(epc)
}
