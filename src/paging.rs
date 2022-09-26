use bitfield::bitfield;
use log::{debug, info, warn};

use crate::memory::FrameAllocator;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageSize {
    Normal,
    Mega,
    Giga,
    Tera,
    Peta,
}

impl PageSize {
    /// Bits needed to represent an address into the page.
    pub const fn bits(&self) -> usize {
        match self {
            PageSize::Normal => 12,
            PageSize::Mega => 21,
            PageSize::Giga => 30,
            PageSize::Tera => 39,
            PageSize::Peta => 48,
        }
    }

    /// Size of the page in bytes.
    pub const fn size(&self) -> usize {
        1 << self.bits()
    }

    #[allow(dead_code)]
    pub const fn round_down(size: usize) -> Self {
        if size >= PageSize::Peta.size() {
            PageSize::Peta
        } else if size >= PageSize::Tera.size() {
            PageSize::Tera
        } else if size >= PageSize::Giga.size() {
            PageSize::Giga
        } else if size >= PageSize::Mega.size() {
            PageSize::Mega
        } else {
            PageSize::Normal
        }
    }
}

pub struct PageTable {
    inner: [PageTableEntry; 512],
}

impl PageTable {
    /// # Safety
    /// The supplied address must be page-aligned and valid for an entire page.
    pub unsafe fn new(addr: *mut u8) -> &'static mut PageTable {
        assert_eq!(addr as usize % PageSize::Normal.size(), 0);

        let addr = addr as *mut PageTable;
        unsafe {
            addr.write_bytes(0, 1);
            &mut *addr
        }
    }

    pub fn ppn(table: *const PageTable) -> usize {
        (table as usize) >> PageSize::Normal.bits()
    }

    /// Setup an identity mapped region at the start of virtual memory
    ///
    /// This should only be called on the root table
    pub fn setup_identity_map(&mut self) {
        let max_map_addr: usize = 0x1_0000_0000;
        let page_size = PageSize::Giga;

        let mut current_base = 0;
        for entry in self.inner.iter_mut() {
            //info!("identity mapping 0x{:X}", current_base);

            let phys_addr = Sv39Physical(current_base as u64);

            entry.set_r(true);
            entry.set_w(true);
            entry.set_x(true);
            entry.set_ppn_2(phys_addr.ppn_2());
            entry.set_ppn_1(phys_addr.ppn_1());
            entry.set_ppn_0(phys_addr.ppn_0());
            entry.set_valid(true);

            current_base += page_size.size();
            if current_base >= max_map_addr {
                break;
            }
        }
    }

    /// Map a page of memory.
    ///
    /// This should only be called on the root table.
    ///
    /// Returns Err if the requested region was already mapped.
    pub fn map<I: Iterator<Item = usize>>(
        &mut self,
        virt: Sv39Virtual,
        phys: Sv39Physical,
        frame_allocator: &mut FrameAllocator<I>,
    ) -> Result<(), ()> {
        // Extract the indexes into each of the tables
        let vpn_0 = virt.vpn(0) as usize;
        let vpn_1 = virt.vpn(1) as usize;
        let vpn_2 = virt.vpn(2) as usize;

        // Descend page tables, creating any missing tables
        let entry = &mut self.inner[vpn_2];
        let next = descend_table(entry, frame_allocator)?;
        let entry = &mut next.inner[vpn_1];
        let next = descend_table(entry, frame_allocator)?;
        let entry = &mut next.inner[vpn_0];

        // Setup the entry to map to the desired physical address
        entry.set_ppn_0(phys.ppn_0());
        entry.set_ppn_1(phys.ppn_1());
        entry.set_ppn_2(phys.ppn_2());

        // Set the entry's permissions
        entry.set_r(true);
        entry.set_w(true);
        entry.set_x(true);

        // TODO: Some implementations might need this depending on which A/D
        //       mode they implement
        //entry.set_accessed(true);
        //entry.set_dirty(true);

        entry.set_valid(true);

        Ok(())
    }

    pub fn lookup(&mut self, virt: Sv39Virtual) -> Result<Sv39Physical, ()> {
        let vpn_0 = virt.vpn(0) as usize;
        let vpn_1 = virt.vpn(1) as usize;
        let vpn_2 = virt.vpn(2) as usize;

        let entry = &mut self.inner[vpn_2];
        let next = entry.as_table_mut().ok_or(())?;
        let entry = &mut next.inner[vpn_1];
        let next = entry.as_table_mut().ok_or(())?;
        let entry = &mut next.inner[vpn_0];

        entry.as_physical_addr().ok_or(())
    }
}

fn descend_table<I: Iterator<Item = usize>>(
    entry: &mut PageTableEntry,
    frame_allocator: &mut FrameAllocator<I>,
) -> Result<&'static mut PageTable, ()> {
    if entry.valid() && !entry.next_level() {
        // Entry already points to a mapping
        warn!("entry points to a an existing mapping");
        return Err(());
    }

    if !entry.valid() {
        // Create new page table
        let table = frame_allocator.next().unwrap();
        //debug!("allocated table at {:p}", table);
        let table = unsafe { PageTable::new(table) };
        let table_phys = Sv39Physical(table as *mut PageTable as u64);
        entry.set_ppn_0(table_phys.ppn_0());
        entry.set_ppn_1(table_phys.ppn_1());
        entry.set_ppn_2(table_phys.ppn_2());
        entry.set_next_level();
        entry.set_valid(true);
    }

    Ok(entry.as_table_mut().unwrap())
}

bitfield! {
    pub struct PageTableEntry(u64);
    impl Debug;

    n, set_n: 63;
    pbmt, set_pbmt: 62, 61;
    ppn_2, set_ppn_2: 53, 28;
    ppn_1, set_ppn_1: 27, 19;
    ppn_0, set_ppn_0: 18, 10;
    ppn, set_ppn: 18, 10, 3;
    rsw, set_rsw: 9, 8;
    dirty, set_dirty: 7;
    accessed, set_accessed: 6;
    global, set_global: 5;
    user, set_user: 4;
    x, set_x: 3;
    w, set_w: 2;
    r, set_r: 1;
    valid, set_valid: 0;
}

#[allow(dead_code)]
impl PageTableEntry {
    pub fn set_next_level(&mut self) {
        self.set_r(false);
        self.set_w(false);
        self.set_x(false);
    }

    pub fn next_level(&self) -> bool {
        !self.r() && !self.w() && !self.x()
    }

    /// Get the physical address the entry points to
    pub fn as_physical_addr(&self) -> Option<Sv39Physical> {
        if !self.valid() {
            return None;
        }

        let mut phys_addr = Sv39Physical(0);
        phys_addr.set_ppn_0(self.ppn_0());
        phys_addr.set_ppn_1(self.ppn_1());
        phys_addr.set_ppn_2(self.ppn_2());

        Some(phys_addr)
    }

    pub fn as_table_mut(&mut self) -> Option<&'static mut PageTable> {
        // # Safety
        // This assumes that physical memory is identiy-mapped, and that the
        // only way to get a mutable reference to the child page table is
        // through the entry itself, and that no page table is referenced from
        // multiple entries at any given time.
        Some(unsafe { &mut *(self.as_physical_addr()?.0 as *mut PageTable) })
    }
}

bitfield! {
    pub struct Sv39Virtual(u64);
    impl Debug;

    vpn, set_vpn: 20, 12, 3;
    page_offset, set_page_offset: 11, 0;
}

bitfield! {
    pub struct Sv39Physical(u64);
    impl Debug;

    ppn_2, set_ppn_2: 55, 30;
    ppn_1, set_ppn_1: 29, 21;
    ppn_0, set_ppn_0: 20, 12;
    page_offset, set_page_offset: 11, 0;
}
