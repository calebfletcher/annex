use fdt::standard_nodes::MemoryRegion;
use log::info;

use crate::{csr, paging};

// These symbols are exposed by the linkerscript
extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
}

struct FrameAllocator<I: Iterator<Item = usize>> {
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
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
