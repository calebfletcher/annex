use core::arch::asm;

use bitfield::bitfield;

bitfield! {
    pub struct Satp(u64);
    impl Debug;

    pub mode, set_mode: 63, 60;
    pub asid, set_asid: 59, 44;
    pub ppn, set_ppn: 43, 0;
}

impl Satp {
    pub fn read() -> Self {
        let satp: u64;
        unsafe {
            asm!("csrr {}, satp", out(reg) satp);
        }
        Self(satp)
    }

    pub fn write(&self) {
        unsafe {
            asm!("csrw satp, {}", in(reg) self.0);
        }
    }
}
