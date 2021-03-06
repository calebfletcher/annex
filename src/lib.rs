#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(type_alias_impl_trait)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]
#![feature(const_btree_new)]
#![feature(result_option_inspect)]
#![feature(map_first_last)]
#![feature(never_type)]
#![feature(asm_const)]
#![feature(atomic_mut_ptr)]
#![feature(alloc_error_handler)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]
// This is bad, but in the early stages it is going to be used everywhere
// in the kernel
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(clippy::new_without_default)]
#![deny(unsafe_op_in_unsafe_fn)]

use bootloader::boot_info::{FrameBuffer, MemoryRegions};
use log::debug;
use x86_64::{PhysAddr, VirtAddr};

pub mod acpi;
pub mod allocator;
pub mod apic;
pub mod cmos;
pub mod emulators;
pub mod gdt;
pub mod gui;
pub mod hardware;
pub mod hpet;
pub mod interrupts;
pub mod logger;
pub mod memory;
pub mod pic;
pub mod task;
#[allow(unused_imports)]
pub mod test;
pub mod threading;
pub mod user;
pub mod utils;

extern crate alloc;

pub fn init(
    framebuffer: &'static mut FrameBuffer,
    rsdp_address: PhysAddr,
    physical_memory_offset: VirtAddr,
    memory_regions: &'static MemoryRegions,
) {
    let mut mapper = unsafe { memory::init(physical_memory_offset) };
    let mut frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(memory_regions) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialisation failed");

    let buffer_info = framebuffer.info();
    let buffer = framebuffer.buffer_mut();

    // Initialise screen
    gui::Screen::init(buffer, buffer_info);

    gdt::init();
    interrupts::init_idt();

    // Disable PIC interrupts since we're using the APIC
    pic::disable();

    // Enable interrupts
    x86_64::instructions::interrupts::enable();

    memory::MemoryManager::init(physical_memory_offset, mapper, frame_allocator);

    acpi::Acpi::init(rsdp_address);
    debug!(
        "acpi lapic addr {:p}",
        acpi::ACPI.try_get().unwrap().lock().local_apic_address()
    );

    let apic_address =
        memory::translate_physical(acpi::ACPI.try_get().unwrap().lock().local_apic_address());
    apic::init(apic_address);

    acpi::ACPI.try_get().unwrap().lock().ioapic();
    cmos::RTC
        .try_init_once(|| cmos::Rtc::new(acpi::ACPI.try_get().unwrap().lock().fadt().century))
        .unwrap();

    hpet::init(acpi::ACPI.try_get().unwrap().lock().hpet());
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(test)]
bootloader::entry_point!(entry_point);
#[cfg(test)]
fn entry_point(info: &'static mut bootloader::BootInfo) -> ! {
    logger::init();

    let framebuffer = info.framebuffer.as_mut().unwrap();
    let rsdp_address = PhysAddr::new(info.rsdp_addr.into_option().unwrap());
    let physical_memory_offset = VirtAddr::new(info.physical_memory_offset.into_option().unwrap());
    let memory_regions = &info.memory_regions;
    init(
        framebuffer,
        rsdp_address,
        physical_memory_offset,
        memory_regions,
    );
    test_main();
    hlt_loop();
}
