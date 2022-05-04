use acpi::HpetInfo;
use bit_field::BitField;
use conquer_once::noblock::OnceCell;
use log::debug;
use x86_64::{structures::paging::PageTableFlags, PhysAddr, VirtAddr};

use crate::memory;

static HPET: OnceCell<Hpet> = OnceCell::uninit();

pub struct Hpet {
    base_address: VirtAddr,

    /// Clock period in femtoseconds (1e-15s)
    clock_period: u64,
}

pub fn init(info: &HpetInfo) {
    HPET.try_init_once(|| {
        let physical_address = PhysAddr::new(info.base_address as u64);
        debug!("found hpet at {:p}", physical_address);

        memory::manager().map_physical_address(physical_address, PageTableFlags::WRITABLE);
        let base_address = memory::translate_physical(info.base_address);

        let capabilities: u64 = unsafe { *base_address.as_ptr() };

        let clock_period = capabilities.get_bits(32..64);

        //unsafe { *base_address.as_mut_ptr().add(2) =  }

        Hpet {
            base_address,
            clock_period,
        }
    })
    .unwrap();
}

pub fn get() -> u64 {
    let base: *const u64 = HPET.try_get().unwrap().base_address.as_ptr();
    unsafe { *base.add(30) }
}

pub fn get_seconds() -> f64 {
    let femto = get();
    femto as f64 / HPET.try_get().unwrap().clock_period as f64
}
