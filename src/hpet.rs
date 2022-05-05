use acpi::HpetInfo;
use bit_field::BitField;
use conquer_once::noblock::OnceCell;
use log::debug;
use x86_64::{structures::paging::PageTableFlags, PhysAddr, VirtAddr};

use crate::memory;

static HPET: OnceCell<Hpet> = OnceCell::uninit();

pub struct Hpet {
    base_address: VirtAddr,

    /// Clock period in nanoseconds (1e-12s)
    clock_period_ns: u64,
}

const HPET_ENABLE_CNF: usize = 0;

pub fn init(info: &HpetInfo) {
    HPET.try_init_once(|| {
        let physical_address = PhysAddr::new(info.base_address as u64);
        debug!("found hpet at {:p}", physical_address);

        memory::manager().map_physical_address(physical_address, PageTableFlags::WRITABLE);
        let base_address = memory::translate_physical(info.base_address);

        let capabilities: u64 = unsafe { *base_address.as_ptr() };

        let clock_period_fs = capabilities.get_bits(32..63);
        let clock_period_ns = clock_period_fs / 1_000_000;

        debug!("hpet period: {} ns", clock_period_ns);

        if !capabilities.get_bit(13) {
            debug!("hpet is not 64 bit");
        }

        // Enable HPET
        let mut configuration: u64 = unsafe {
            base_address
                .as_mut_ptr::<u64>()
                .add(0x10 >> 3)
                .read_volatile()
        };
        configuration.set_bit(HPET_ENABLE_CNF, true);
        unsafe {
            base_address
                .as_mut_ptr::<u64>()
                .add(0x10 >> 3)
                .write_volatile(configuration)
        };

        Hpet {
            base_address,
            clock_period_ns,
        }
    })
    .unwrap();
}

/// Get the counter value in nanoseconds
pub fn nanoseconds() -> u64 {
    let hpet = HPET.try_get().unwrap();
    let base: *const u64 = hpet.base_address.as_ptr();
    let ticks = unsafe { base.add(0xF0 >> 3).read_volatile() };
    ticks * hpet.clock_period_ns
}

/// Get the counter value in seconds
pub fn seconds() -> f64 {
    let ns = nanoseconds();
    ns as f64 / 1e9
}
