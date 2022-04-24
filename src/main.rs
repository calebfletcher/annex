#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(panic_info_message)]
#![test_runner(annex::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use annex::{
    allocator, memory, println, screen,
    task::{self, executor::Executor, Task},
    timer,
};
use x86_64::{PhysAddr, VirtAddr};

mod panic;

bootloader::entry_point!(entry_point);
fn entry_point(info: &'static mut bootloader::BootInfo) -> ! {
    let framebuffer = info.framebuffer.as_mut().unwrap();

    annex::init(framebuffer);
    println!("starting kernel");

    // Run the tests if we're running under the test harness
    #[cfg(test)]
    test_main();

    let physical_memory_offset = VirtAddr::new(info.physical_memory_offset.into_option().unwrap());
    let rsdp_address = PhysAddr::new(info.rsdp_addr.into_option().unwrap());

    let phys_mem_offset = VirtAddr::new(info.physical_memory_offset.into_option().unwrap());
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(&info.memory_regions) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    let acpi = annex::acpi::Acpi::init(rsdp_address, physical_memory_offset);
    let apic_addr = physical_memory_offset + acpi.local_apic_address().as_u64();
    acpi.ioapic();
    timer::init(apic_addr);
    task::keyboard::init();

    let mut executor = Executor::new();
    executor.spawn(Task::new(annex::task::keyboard::handle_keyboard()));
    executor.spawn(Task::new(annex::user::line_edit::run()));

    println!("loaded kernel");
    executor.run();
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
