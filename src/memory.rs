use fdt::standard_nodes::MemoryRegion;
use log::info;

const PAGE_SIZE: usize = 4096; // 4KiB

// These symbols are exposed by the linkerscript
extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
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
    let memory_base = align_up(kernel_end, PAGE_SIZE);
    let memory_size = region.size.unwrap() - kernel_size;
    info!(
        "using memory segment at address {:X} with size {:X} bytes",
        memory_base, memory_size
    )

    // TODO: Create an allocator with this space
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
