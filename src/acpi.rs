use core::ptr;

use x86_64::{PhysAddr, VirtAddr};

pub struct Acpi {
    platform_info: acpi::PlatformInfo,
}

impl Acpi {
    pub fn init(rsdp_address: PhysAddr, physical_memory_offset: VirtAddr) -> Self {
        let handler = Handler {
            physical_memory_offset,
        };
        let table = unsafe {
            acpi::AcpiTables::from_rsdp(&handler, rsdp_address.as_u64() as usize).unwrap()
        };

        let platform_info = table.platform_info().unwrap();

        Self { platform_info }
    }

    pub fn local_apic_address(&self) -> PhysAddr {
        let apic = if let acpi::InterruptModel::Apic(apic) = &self.platform_info.interrupt_model {
            apic
        } else {
            unimplemented!("no apic found");
        };

        PhysAddr::new(apic.local_apic_address)
    }
}

struct Handler {
    physical_memory_offset: VirtAddr,
}

impl<'a> acpi::AcpiHandler for &Handler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        let virtual_start =
            ptr::NonNull::new((self.physical_memory_offset + physical_address).as_mut_ptr())
                .unwrap();

        acpi::PhysicalMapping::new(physical_address, virtual_start, size, size, self)
    }

    fn unmap_physical_region<T>(_region: &acpi::PhysicalMapping<Self, T>) {}
}
