use conquer_once::noblock::OnceCell;
use x2apic::lapic::LocalApic;
use x86_64::VirtAddr;

pub fn detect() -> bool {
    let cpu_features = unsafe { core::arch::x86_64::__cpuid(1) };
    cpu_features.edx >> 9 & 1 == 1
}

pub static APIC: OnceCell<spin::Mutex<LocalApic>> = OnceCell::uninit();

pub fn init(addr: VirtAddr) {
    APIC.try_init_once(|| {
        let mut lapic = x2apic::lapic::LocalApicBuilder::new()
            .timer_vector(61)
            .error_vector(62)
            .spurious_vector(63)
            .set_xapic_base(addr.as_u64())
            .timer_divide(x2apic::lapic::TimerDivide::Div16)
            .timer_mode(x2apic::lapic::TimerMode::Periodic)
            .timer_initial(10_000_000)
            .build()
            .unwrap_or_else(|err| panic!("{}", err));

        unsafe {
            lapic.enable();
        }
        spin::Mutex::new(lapic)
    })
    .unwrap();
}
