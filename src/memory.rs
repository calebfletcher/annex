use fdt::standard_nodes::MemoryRegion;
use log::info;

// We have our available regions here, but what can we actually use?
// The kernel is mapped somewhere in this space, it doesn't get excluded
// like on UEFI systems.
//
// For now, we will just skip the first 64MB.
const KERNEL_SKIP_OFFSET: usize = 0x4000000;

pub fn init(mut regions: impl Iterator<Item = MemoryRegion>) {
    let region = regions.next().expect("no memory regions found");
    let remaining_regions = regions.count();
    if remaining_regions > 0 {
        panic!("multiple memory regions found");
    }

    let memory_base = unsafe { region.starting_address.add(KERNEL_SKIP_OFFSET) };
    let memory_size = region.size.expect("memory region has to have a size") - KERNEL_SKIP_OFFSET;

    info!(
        "using memory segment at address {:p} with size {:X} bytes",
        memory_base, memory_size
    )

    // TODO: Create an allocator
}
