use bitfield::bitfield;
use log::info;

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
            info!("identity mapping 0x{:X}", current_base);

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
