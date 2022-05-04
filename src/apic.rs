use core::arch::x86_64::__cpuid;

use bit_field::BitField;
use conquer_once::noblock::OnceCell;
use log::warn;
use x2apic::lapic::LocalApic;
use x86_64::{structures::paging::PageTableFlags, PhysAddr, VirtAddr};

use crate::memory;

pub static LAPIC: OnceCell<spin::Mutex<LocalApic>> = OnceCell::uninit();

pub fn init(addr: VirtAddr) {
    check_status();
    map_apic_pages();

    LAPIC
        .try_init_once(|| {
            let mut lapic = x2apic::lapic::LocalApicBuilder::new()
                .timer_vector(61)
                .error_vector(62)
                .spurious_vector(63)
                .set_xapic_base(addr.as_u64())
                .timer_divide(x2apic::lapic::TimerDivide::Div16)
                .timer_mode(x2apic::lapic::TimerMode::Periodic)
                .timer_initial(100_000)
                .build()
                .unwrap_or_else(|err| panic!("{}", err));

            unsafe {
                lapic.enable();
            }

            spin::Mutex::new(lapic)
        })
        .unwrap();
}

pub fn check_status() {
    let cpuid = unsafe { __cpuid(0x01) };
    let lapic_supported = cpuid.edx.get_bit(9);
    if !lapic_supported {
        warn!("lapic is not supported");
    }
    let x2apic_supported = cpuid.ecx.get_bit(21);
    if !x2apic_supported {
        warn!("lapic is not an x2apic");
    }
    let ia32_apic_base = unsafe { x86_64::registers::model_specific::Msr::new(0x1B).read() };
    let apic_enabled = ia32_apic_base.get_bit(11);
    if !apic_enabled {
        warn!("apic is not enabled");
    }
}

pub fn map_apic_pages() {
    // TODO: Correctly retrieve the values used here
    // map lapic
    memory::manager().map_physical_address(PhysAddr::new(0xfee00000), PageTableFlags::WRITABLE);

    // map ioapic
    memory::manager().map_physical_address(PhysAddr::new(0xfec00000), PageTableFlags::WRITABLE)
}
