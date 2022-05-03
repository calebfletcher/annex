#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(type_alias_impl_trait)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]
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

use bootloader::boot_info::{FrameBuffer, MemoryRegions};
use log::debug;
use x86_64::{PhysAddr, VirtAddr};

pub mod acpi;
pub mod allocator;
pub mod colour;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod screen;
pub mod serial;
pub mod task;
pub mod user;

pub mod apic;
pub mod cmos;
pub mod logger;
pub mod pic;
#[allow(unused_imports)]
pub mod test;
pub mod threading;

extern crate alloc;

pub fn init(
    framebuffer: &'static mut FrameBuffer,
    rsdp_address: PhysAddr,
    physical_memory_offset: VirtAddr,
    memory_regions: &'static MemoryRegions,
) {
    let buffer_info = framebuffer.info();
    let buffer = framebuffer.buffer_mut();

    // Initialise screen
    let mut screen = screen::Screen::new(buffer, buffer_info);
    screen.clear(colour::BLACK);

    // Initialise text console
    init_terminal(screen);

    gdt::init();
    interrupts::init_idt();

    // Disable PIC interrupts since we're using the APIC
    pic::disable();

    // Enable interrupts
    x86_64::instructions::interrupts::enable();

    let mut mapper = unsafe { memory::init(physical_memory_offset) };
    let mut frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(memory_regions) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialisation failed");

    memory::MemoryManager::init(physical_memory_offset, mapper, frame_allocator);

    let handler = acpi::Handler {};
    let acpi = acpi::Acpi::init(&handler, rsdp_address);
    debug!("acpi lapic addr {:p}", acpi.local_apic_address());

    let apic_address = memory::translate_physical(acpi.local_apic_address());
    apic::init(apic_address);

    acpi.ioapic();
    task::keyboard::init();
    cmos::RTC
        .try_init_once(|| cmos::Rtc::new(acpi.fadt().century))
        .unwrap();
}

fn init_terminal(screen: screen::Screen<'static>) {
    let console = screen::Terminal::new(screen);

    screen::TERMINAL
        .try_init_once(move || spin::mutex::SpinMutex::new(console))
        .unwrap();
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
