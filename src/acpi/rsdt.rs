#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Rsdt {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

impl Rsdt {
    fn new(addr: *const u8) -> &'static Self {
        let rsdt = unsafe { &*(addr as *mut Self) };
        assert!(rsdt.validate());

        rsdt
    }

    fn validate(&self) -> bool {
        let bytes = unsafe {
            core::slice::from_raw_parts(self as *const Self as *const u8, self.length as usize)
        };
        let result = bytes.iter().fold(0_u8, |acc, &x| acc.wrapping_add(x));
        result == 0
    }

    fn get_tables(&self) -> &[u32] {
        // TODO: Value 4 assumes using RsdtPointer not XsdtPointer
        let header_length = core::mem::size_of::<Self>();
        let num_entries = (self.length as usize - header_length) / 4;
        unsafe {
            core::slice::from_raw_parts(
                (self as *const Self as *const u8).add(header_length) as *const u32,
                num_entries,
            )
        }
    }
    pub fn find_table<T: SystemDescriptor>(&self, physical_offset: *const u8) -> Option<&T> {
        let tables = self.get_tables();

        for &table in tables {
            let header = unsafe { &*(physical_offset.add(table as usize) as *const Rsdt) };
            if &header.signature == T::SIGNATURE {
                return Some(unsafe { &*(physical_offset.add(table as usize) as *const T) });
            }
        }

        None
    }
}

pub fn init(addr: *const u8) -> &'static Rsdt {
    Rsdt::new(addr)
}

pub trait SystemDescriptor {
    const SIGNATURE: &'static [u8; 4];
}
