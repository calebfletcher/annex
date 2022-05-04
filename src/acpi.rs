use core::{ptr, slice};

use acpi::sdt::Signature;
use alloc::boxed::Box;
use aml::{AmlContext, AmlError, AmlName, AmlValue};
use conquer_once::noblock::OnceCell;
use log::{debug, error, warn};
use x2apic::ioapic::IoApic;
use x86_64::{instructions::port::Port, PhysAddr};

use crate::{dbg, memory, println};

pub struct Acpi<'a> {
    platform_info: acpi::PlatformInfo,
    fadt: acpi::PhysicalMapping<&'a Handler, acpi::fadt::Fadt>,
    context: AmlContext,
    hpet: acpi::HpetInfo,
}

impl<'a> Acpi<'a> {
    pub fn init(handler: &'a Handler, rsdp_address: PhysAddr) -> Self {
        debug!("decoding acpi tables");
        let table = unsafe {
            acpi::AcpiTables::from_rsdp(handler, rsdp_address.as_u64() as usize).unwrap()
        };

        let pci_regions = acpi::PciConfigRegions::new(&table).unwrap();

        let dsdt = table.dsdt.as_ref().unwrap();
        let dsdt: &[u8] = unsafe {
            slice::from_raw_parts(
                memory::translate_physical(dsdt.address).as_ptr(),
                dsdt.length as usize,
            )
        };
        let context = load_aml(dsdt, pci_regions);

        let platform_info = table.platform_info().unwrap();
        debug!(
            "found local apic id {}",
            platform_info
                .processor_info
                .as_ref()
                .unwrap()
                .boot_processor
                .local_apic_id
        );

        let fadt: acpi::PhysicalMapping<&Handler, acpi::fadt::Fadt> =
            unsafe { table.get_sdt(Signature::FADT).unwrap().unwrap() };

        let hpet = acpi::HpetInfo::new(&table).unwrap();

        Self {
            platform_info,
            fadt,
            context,
            hpet,
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
        debug!(
            "using ioapic at physical address {:p}",
            ioapic.address as *const u8
        );

        IOAPIC
            .try_init_once(|| {
                let mut ioapic = unsafe {
                    x2apic::ioapic::IoApic::new(memory::translate_physical(ioapic.address).as_u64())
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

    pub fn shutdown(&mut self) {
        debug!("beginning shutdown");
        let shutdown = self
            .context
            .namespace
            .get_by_path(&AmlName::from_str("\\_S5_").unwrap())
            .unwrap();

        dbg!(shutdown);

        let mut args = aml::value::Args::EMPTY;
        args.store_arg(0, AmlValue::Integer(5)).unwrap();
        let res = self
            .context
            .invoke_method(&AmlName::from_str("\\_PTS").unwrap(), args);
        match res {
            Ok(_) => debug!("notified oem firmware of intent to shutdown"),
            Err(AmlError::ValueDoesNotExist(name)) => {
                warn!("unable to notify oem firmware of shutdown, could not find {name}")
            }
            Err(e) => error!("shutdown error: {e:?}"),
        }

        let pm1a_cnt = self.fadt.pm1a_control_block().unwrap();
        let pm1a_cnt = memory::translate_physical(pm1a_cnt.address);
        let current_value: u16 = unsafe { *pm1a_cnt.as_ptr() };
        println!("value: {:b}", current_value,);
        unsafe {
            *pm1a_cnt.as_mut_ptr() = current_value | 6 << 10 | 1 << 13;
        }
    }

    /// Get a reference to the acpi's hpet.
    #[must_use]
    pub fn hpet(&self) -> &acpi::HpetInfo {
        &self.hpet
    }
}

pub static IOAPIC: OnceCell<spin::Mutex<IoApic>> = OnceCell::uninit();

pub struct Handler {}

impl<'a> acpi::AcpiHandler for &Handler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        let virtual_start =
            ptr::NonNull::new((memory::translate_physical(physical_address as u64)).as_mut_ptr())
                .unwrap();

        unsafe { acpi::PhysicalMapping::new(physical_address, virtual_start, size, size, self) }
    }

    fn unmap_physical_region<T>(_region: &acpi::PhysicalMapping<Self, T>) {}
}

pub fn load_aml(dsdt: &[u8], pci_regions: acpi::PciConfigRegions) -> AmlContext {
    debug!("parsing aml from dsdt table");
    let handler = AmlHandler { pci_regions };
    let mut context = AmlContext::new(Box::new(handler), aml::DebugVerbosity::None);

    context.parse_table(dsdt).unwrap();
    //context.initialize_objects().unwrap();

    // context
    //     .namespace
    //     .traverse(|name, level| {
    //         if name.as_string() == "\\" {
    //             serial_println!("{:#?}", &level.values);
    //         }
    //         Ok(false)
    //     })
    //     .unwrap();

    context
}

struct AmlHandler {
    pci_regions: acpi::PciConfigRegions,
}

impl aml::Handler for AmlHandler {
    fn read_u8(&self, address: usize) -> u8 {
        unsafe { *memory::translate_physical(address).as_ptr() }
    }

    fn read_u16(&self, address: usize) -> u16 {
        unsafe { *memory::translate_physical(address).as_ptr() }
    }

    fn read_u32(&self, address: usize) -> u32 {
        unsafe { *memory::translate_physical(address).as_ptr() }
    }

    fn read_u64(&self, address: usize) -> u64 {
        unsafe { *memory::translate_physical(address).as_ptr() }
    }

    fn write_u8(&mut self, address: usize, value: u8) {
        unsafe { *memory::translate_physical(address).as_mut_ptr() = value }
    }

    fn write_u16(&mut self, address: usize, value: u16) {
        unsafe { *memory::translate_physical(address).as_mut_ptr() = value }
    }

    fn write_u32(&mut self, address: usize, value: u32) {
        unsafe { *memory::translate_physical(address).as_mut_ptr() = value }
    }

    fn write_u64(&mut self, address: usize, value: u64) {
        unsafe { *memory::translate_physical(address).as_mut_ptr() = value }
    }

    fn read_io_u8(&self, port: u16) -> u8 {
        unsafe { Port::new(port).read() }
    }

    fn read_io_u16(&self, port: u16) -> u16 {
        unsafe { Port::new(port).read() }
    }

    fn read_io_u32(&self, port: u16) -> u32 {
        unsafe { Port::new(port).read() }
    }

    fn write_io_u8(&self, port: u16, value: u8) {
        unsafe { Port::new(port).write(value) }
    }

    fn write_io_u16(&self, port: u16, value: u16) {
        unsafe { Port::new(port).write(value) }
    }

    fn write_io_u32(&self, port: u16, value: u32) {
        unsafe { Port::new(port).write(value) }
    }

    fn read_pci_u8(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16) -> u8 {
        let phys_addr = self
            .pci_regions
            .physical_address(segment, bus, device, function)
            .unwrap()
            + offset as u64;

        unsafe { *memory::translate_physical(phys_addr).as_ptr() }
    }

    fn read_pci_u16(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16) -> u16 {
        let phys_addr = self
            .pci_regions
            .physical_address(segment, bus, device, function)
            .unwrap()
            + offset as u64;

        unsafe { *memory::translate_physical(phys_addr).as_ptr() }
    }

    fn read_pci_u32(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16) -> u32 {
        let phys_addr = self
            .pci_regions
            .physical_address(segment, bus, device, function)
            .unwrap()
            + offset as u64;

        unsafe { *memory::translate_physical(phys_addr).as_ptr() }
    }

    fn write_pci_u8(
        &self,
        segment: u16,
        bus: u8,
        device: u8,
        function: u8,
        offset: u16,
        value: u8,
    ) {
        let phys_addr = self
            .pci_regions
            .physical_address(segment, bus, device, function)
            .unwrap()
            + offset as u64;

        unsafe { *memory::translate_physical(phys_addr).as_mut_ptr() = value }
    }

    fn write_pci_u16(
        &self,
        segment: u16,
        bus: u8,
        device: u8,
        function: u8,
        offset: u16,
        value: u16,
    ) {
        let phys_addr = self
            .pci_regions
            .physical_address(segment, bus, device, function)
            .unwrap()
            + offset as u64;

        unsafe { *memory::translate_physical(phys_addr).as_mut_ptr() = value }
    }

    fn write_pci_u32(
        &self,
        segment: u16,
        bus: u8,
        device: u8,
        function: u8,
        offset: u16,
        value: u32,
    ) {
        let phys_addr = self
            .pci_regions
            .physical_address(segment, bus, device, function)
            .unwrap()
            + offset as u64;

        unsafe { *memory::translate_physical(phys_addr).as_mut_ptr() = value }
    }
}
