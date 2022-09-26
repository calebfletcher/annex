use core::arch::asm;

use fdt::standard_nodes::MemoryRegion;
use log::{debug, info};

use crate::{allocator, csr, paging};

pub const HEAP_START: *mut u8 = 0xFFFF_FFC0_0000_0000 as *mut _;
pub const HEAP_SIZE: usize = 48 * 1024 * 1024; // 16 MiB

// These symbols are exposed by the linkerscript
extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
}

pub struct FrameAllocator<I: Iterator<Item = usize>> {
    available_pages: I,
}

impl FrameAllocator<core::iter::StepBy<core::ops::Range<usize>>> {
    pub fn new(base: usize, size: usize) -> Self {
        let end = base + size;
        let i = (base..end).step_by(paging::PageSize::Normal.size());
        Self { available_pages: i }
    }
}

impl<I: Iterator<Item = usize>> FrameAllocator<I> {
    pub fn next(&mut self) -> Option<*mut u8> {
        self.available_pages.next().map(|addr| addr as _)
    }
}

pub fn init(mut regions: impl Iterator<Item = MemoryRegion>) {
    // Get the main memory region
    let region = regions.next().expect("no memory regions found");
    let remaining_regions = regions.count();
    if remaining_regions > 0 {
        panic!("multiple memory regions found");
    }

    // Find out where the kernel file was loaded to
    let (kernel_start, kernel_end) = get_kernel_range();
    info!("kernel loaded from {:X} to {:X}", kernel_start, kernel_end);
    let kernel_size = kernel_end - kernel_start;

    // Ensure this is the main memory segment
    if kernel_end < region.starting_address as usize
        || kernel_end
            > (region.starting_address as usize
                + region.size.expect("memory region has to have a size"))
    {
        panic!("kernel not loaded in memory segment");
    }

    // Calculate the remaining space
    let memory_base = align_up(kernel_end, paging::PageSize::Normal.size());
    let memory_size = region.size.unwrap() - kernel_size;
    info!(
        "using memory segment at address {:X} with size {:X} bytes",
        memory_base, memory_size
    );

    // Create an allocator with this space
    let mut frame_allocator = FrameAllocator::new(memory_base, memory_size);

    // Create a new page table
    let table = frame_allocator.next().unwrap();
    let table = unsafe { paging::PageTable::new(table) };
    table.setup_identity_map();

    // Update satp with the new page table
    let mut satp = csr::Satp::read();
    satp.set_asid(0);
    satp.set_mode(8);
    satp.set_ppn(paging::PageTable::ppn(table) as u64);
    satp.write();

    // Allocate memory for entire heap range
    let mut length = HEAP_SIZE;
    let mut virt_addr = HEAP_START;
    while length > 0 {
        let next_page = frame_allocator.next().unwrap();

        table
            .map(
                paging::Sv39Virtual(virt_addr as u64),
                paging::Sv39Physical(next_page as u64),
                &mut frame_allocator,
            )
            .unwrap();

        virt_addr = unsafe { virt_addr.add(paging::PageSize::Normal.size()) };
        length -= paging::PageSize::Normal.size();
    }

    let heap_start = HEAP_START as usize;
    let heap_end = heap_start + HEAP_SIZE - 4096;
    info!("heap mapped from 0x{:X} to 0x{:X}", heap_start, heap_end);

    // Flush TLB
    // TODO: Only flush this ASID and relevant address range
    unsafe {
        asm!("sfence.vma x0, x0");
    }

    // Initialise the memory allocator
    allocator::init(|| allocator::FixedSizeBlockAllocator::new(HEAP_START, HEAP_SIZE));
}

fn get_kernel_range() -> (usize, usize) {
    unsafe {
        (
            &__kernel_start as *const u8 as usize,
            &__kernel_end as *const u8 as usize,
        )
    }
}

/// Align the given address `addr` upwards to alignment `align`.
///
/// Requires that `align` is a power of two.
pub fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
