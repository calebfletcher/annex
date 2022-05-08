use alloc::{format, string::String, vec::Vec};
use bootloader::boot_info::{MemoryRegionKind, MemoryRegions};
use conquer_once::noblock::OnceCell;
use log::warn;
use spin::Mutex;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PageTable, PageTableFlags,
        PhysFrame, Size2MiB, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use crate::serial_println;

/// Returns a mutable reference to the active level 4 table.
///
/// # Safety
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address().as_u64();
    let virt = physical_memory_offset + phys;
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

/// Initialize a new OffsetPageTable.
///
/// # Safety
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

type FrameIter = impl Iterator<Item = PhysFrame>;
/// A FrameAllocator that returns usable frames from the bootloader's memory map.
pub struct BootInfoFrameAllocator {
    usable_frames: FrameIter,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    ///
    /// # Safety
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    pub unsafe fn init(memory_map: &'static MemoryRegions) -> Self {
        // get usable regions from memory map
        let regions = memory_map.iter();
        let usable_regions = regions.filter(|r| r.kind == MemoryRegionKind::Usable);
        // map each region to its address range
        let addr_ranges = usable_regions.map(|r| r.start..r.end);
        // transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // create `PhysFrame` types from the start addresses
        let usable_frames: FrameIter =
            frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)));

        BootInfoFrameAllocator { usable_frames }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames.next();

        match frame {
            Some(_frame) => {
                //debug!("allocating frame at {:p}", _frame.start_address())
            }
            None => warn!("boot info frame allocator exhausted"),
        }
        frame
    }
}

pub fn format_bytes(mut value: u64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];

    let mut unit = units[units.len() - 1];
    for suffix_unit in units {
        if value < 1024 {
            unit = suffix_unit;
            break;
        }
        value >>= 10;
    }

    format!("{} {}", value, unit)
}

/// An object responsible for mapping pages between the kernel's virtual memory
/// and the physical memory.
pub struct MemoryManager {
    /// Virtual address that physical memory has been mapped to
    physical_memory_offset: VirtAddr,
    page_table: OffsetPageTable<'static>,
    frame_allocator: BootInfoFrameAllocator,
}

pub static MANAGER: OnceCell<Mutex<MemoryManager>> = OnceCell::uninit();
pub static PHYSICAL_OFFSET: OnceCell<VirtAddr> = OnceCell::uninit();

impl MemoryManager {
    pub fn init(
        physical_memory_offset: VirtAddr,
        page_table: OffsetPageTable<'static>,
        frame_allocator: BootInfoFrameAllocator,
    ) {
        PHYSICAL_OFFSET
            .try_init_once(|| physical_memory_offset)
            .unwrap();
        MANAGER
            .try_init_once(|| {
                Mutex::new(MemoryManager {
                    physical_memory_offset,
                    page_table,
                    frame_allocator,
                })
            })
            .unwrap();
    }

    pub fn map_physical_address(&mut self, addr: PhysAddr, additional_flags: PageTableFlags) {
        let flags = PageTableFlags::PRESENT | additional_flags;
        let page: Page<Size4KiB> =
            Page::containing_address(self.physical_memory_offset + addr.as_u64());
        let frame = PhysFrame::containing_address(addr);
        unsafe {
            // Flush TLB if mapping was successful
            if let Ok(mapping) =
                self.page_table
                    .map_to(page, frame, flags, &mut self.frame_allocator)
            {
                mapping.flush();
            }
        };
    }
}

/// Gets a lock on the mutex of the memory manager
pub fn manager<'a>() -> spin::MutexGuard<'a, MemoryManager> {
    MANAGER.try_get().unwrap().lock()
}

pub fn translate_physical(addr: impl AsU64) -> VirtAddr {
    *PHYSICAL_OFFSET.try_get().unwrap() + addr.as_u64()
}

struct Mapping {
    virt: VirtAddr,
    physical: PhysAddr,
    length: u64,
}

pub fn display_page_table() {
    let mut mappings = Vec::new();

    let (level_4_table_frame, _) = Cr3::read();
    let addr = translate_physical(level_4_table_frame.start_address());

    // Level 4
    let level_4_page_table = unsafe { &*(addr.as_ptr() as *const PageTable) };
    for (level_4_index, page_table_entry) in level_4_page_table.iter().enumerate() {
        if page_table_entry.is_unused()
            || !page_table_entry.flags().contains(PageTableFlags::PRESENT)
        {
            continue;
        }
        if page_table_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
            serial_println!("- {:p} - HUGE PAGE", page_table_entry.addr());
            continue;
        }

        // Level 3
        let level_3_page_table =
            unsafe { &*(translate_physical(page_table_entry.addr()).as_ptr() as *const PageTable) };
        for (level_3_index, page_table_entry) in level_3_page_table.iter().enumerate() {
            if page_table_entry.is_unused()
                || !page_table_entry.flags().contains(PageTableFlags::PRESENT)
            {
                continue;
            }
            if page_table_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                serial_println!("  - {:p} - 1GiB HUGE PAGE", page_table_entry.addr());
                continue;
            }

            // Level 2
            let level_2_page_table = unsafe {
                &*(translate_physical(page_table_entry.addr()).as_ptr() as *const PageTable)
            };
            for (level_2_index, page_table_entry) in level_2_page_table.iter().enumerate() {
                if page_table_entry.is_unused()
                    || !page_table_entry.flags().contains(PageTableFlags::PRESENT)
                {
                    continue;
                }
                if page_table_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                    // 4MiB page
                    let physical_address = page_table_entry.addr();
                    let addr = level_4_index << 39 | level_3_index << 30 | level_2_index << 21;
                    let virtual_address = VirtAddr::new(addr as u64);

                    let length = Size2MiB::SIZE;
                    mappings.push(Mapping {
                        virt: virtual_address,
                        physical: physical_address,
                        length,
                    });
                    continue;
                }

                // Level 1
                let level_1_page_table = unsafe {
                    &*(translate_physical(page_table_entry.addr()).as_ptr() as *const PageTable)
                };
                for (level_1_index, page_table_entry) in level_1_page_table.iter().enumerate() {
                    if page_table_entry.is_unused()
                        || !page_table_entry.flags().contains(PageTableFlags::PRESENT)
                    {
                        continue;
                    }
                    if page_table_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                        panic!("unexpected huge page in level 1 page table")
                    }

                    let physical_address = page_table_entry.addr();
                    let addr = level_4_index << 39
                        | level_3_index << 30
                        | level_2_index << 21
                        | level_1_index << 12;
                    let virtual_address = VirtAddr::new(addr as u64);

                    let length = Size4KiB::SIZE;
                    mappings.push(Mapping {
                        virt: virtual_address,
                        physical: physical_address,
                        length,
                    });
                }
            }
        }
    }

    let mut joined_mappings = Vec::new();
    let mut mapping: Option<Mapping> = None;
    for next_mapping in mappings {
        match mapping {
            Some(mut current_mapping) => {
                // Check if mappings are consecutive in physical and in virtual address spaces
                if current_mapping.physical + current_mapping.length == next_mapping.physical
                    && current_mapping.virt + current_mapping.length == next_mapping.virt
                {
                    // Join mappings
                    current_mapping.length += next_mapping.length;
                    mapping = Some(current_mapping);
                } else {
                    // Push old mapping, start at new mapping
                    joined_mappings.push(current_mapping);
                    mapping = Some(next_mapping);
                }
            }
            None => {
                mapping = Some(next_mapping);
            }
        }
    }

    // Print each mapping
    for mapping in joined_mappings {
        serial_println!(
            "{:014p}-{:014p} to {:014p}-{:014p} ({})",
            mapping.virt,
            mapping.virt + mapping.length,
            mapping.physical,
            mapping.physical + mapping.length,
            format_bytes(mapping.length as u64)
        );
    }
}

pub trait AsU64 {
    fn as_u64(&self) -> u64;
}

impl AsU64 for u64 {
    fn as_u64(&self) -> u64 {
        *self
    }
}

impl AsU64 for usize {
    fn as_u64(&self) -> u64 {
        (*self).try_into().unwrap()
    }
}

impl AsU64 for VirtAddr {
    fn as_u64(&self) -> u64 {
        VirtAddr::as_u64(*self)
    }
}

impl AsU64 for PhysAddr {
    fn as_u64(&self) -> u64 {
        PhysAddr::as_u64(*self)
    }
}

impl AsU64 for u32 {
    fn as_u64(&self) -> u64 {
        *self as u64
    }
}
