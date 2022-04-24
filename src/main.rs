#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(panic_info_message)]
#![test_runner(annex::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use annex::{allocator, colour, memory, println, screen, timer};
use x86_64::{PhysAddr, VirtAddr};

mod panic;

bootloader::entry_point!(entry_point);
fn entry_point(info: &'static mut bootloader::BootInfo) -> ! {
    let framebuffer = info.framebuffer.as_mut().unwrap();

    annex::init(framebuffer);
    println!("hello");

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

    let idle_thread = Thread::create(idle_thread, 2, &mut mapper, &mut frame_allocator).unwrap();
    with_scheduler(|s| s.set_idle_thread(idle_thread));

    for _ in 0..10 {
        let thread = Thread::create(thread_entry, 2, &mut mapper, &mut frame_allocator).unwrap();
        with_scheduler(|s| s.add_new_thread(thread));
    }
    let thread =
        Thread::create_from_closure(|| thread_entry(), 2, &mut mapper, &mut frame_allocator)
            .unwrap();
    with_scheduler(|s| s.add_new_thread(thread));
fn idle_thread() -> ! {
    loop {
        x86_64::instructions::hlt();
        multitasking::yield_now();
    }
}

fn thread_entry() -> ! {
    let thread_id = with_scheduler(|s| s.current_thread_id()).as_u64();
    for _ in 0..=thread_id {
        print!("{}", thread_id);
        x86_64::instructions::hlt();
    }
    multitasking::exit_thread();
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
