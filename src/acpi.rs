use core::ptr;

use acpi::sdt::Signature;
use conquer_once::noblock::OnceCell;
use x2apic::ioapic::IoApic;
use x86_64::{PhysAddr, VirtAddr};

use crate::println;

pub struct Acpi<'a> {
    platform_info: acpi::PlatformInfo,
    physical_memory_offset: VirtAddr,
    fadt: acpi::PhysicalMapping<&'a Handler, acpi::fadt::Fadt>,
}

impl<'a> Acpi<'a> {
    pub fn init(
        handler: &'a Handler,
        rsdp_address: PhysAddr,
        physical_memory_offset: VirtAddr,
    ) -> Self {
        let table = unsafe {
            acpi::AcpiTables::from_rsdp(handler, rsdp_address.as_u64() as usize).unwrap()
        };

        for signature in table.sdts.keys() {
            println!("found table {}", signature);
        }

        let platform_info = table.platform_info().unwrap();

        let fadt: acpi::PhysicalMapping<&Handler, acpi::fadt::Fadt> =
            unsafe { table.get_sdt(Signature::FADT).unwrap().unwrap() };

        Self {
            platform_info,
            physical_memory_offset,
            fadt,
        }
    }

    pub fn local_apic_address(&self) -> PhysAddr {
        let apic = if let acpi::InterruptModel::Apic(apic) = &self.platform_info.interrupt_model {
            apic
        } else {
            unimplemented!("no apic found");
        };

        PhysAddr::new(apic.local_apic_address)
    }

    pub fn ioapic(&self) {
        let ioapic = if let acpi::InterruptModel::Apic(apic) = &self.platform_info.interrupt_model {
            apic.io_apics.get(0).unwrap()
        } else {
            unimplemented!("no apic found");
        };

        IOAPIC
            .try_init_once(|| {
                let mut ioapic = unsafe {
                    x2apic::ioapic::IoApic::new(
                        (self.physical_memory_offset + ioapic.address as u64).as_u64(),
                    )
                };
                unsafe {
                    ioapic.init(90);
                    ioapic.enable_irq(1);
                }
                spin::Mutex::new(ioapic)
            })
            .unwrap();
    }

    /// Get a reference to the acpi's fadt.
    #[must_use]
    pub fn fadt(&self) -> &acpi::PhysicalMapping<&'a Handler, acpi::fadt::Fadt> {
        &self.fadt
    }
}

pub static IOAPIC: OnceCell<spin::Mutex<IoApic>> = OnceCell::uninit();

pub struct Handler {
    pub physical_memory_offset: VirtAddr,
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
