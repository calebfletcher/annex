use crate::acpi;

pub fn detect() -> bool {
    let cpu_features = unsafe { core::arch::x86_64::__cpuid(1) };
    cpu_features.edx >> 9 & 1 == 1
}

pub fn get_apic_address(physical_memory_offset: *const u8, rsdp_offset: usize) -> *const u8 {
    let rsdp_addr = unsafe { physical_memory_offset.add(rsdp_offset) };

    let rsdp = acpi::rsdp::init(rsdp_addr);
    let rsdt = acpi::rsdt::init(rsdp.rsdt_address(physical_memory_offset));
    let madt = rsdt
        .find_table::<acpi::madt::Madt>(physical_memory_offset)
        .unwrap();
    madt.local_apic_address(physical_memory_offset)
}
