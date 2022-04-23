use crate::println;

trait Rsdp {
    fn validate(&self) -> bool
    where
        Self: core::marker::Sized,
        [(); core::mem::size_of::<Self>()]:,
    {
        let bytes: &[u8; core::mem::size_of::<Self>()] =
            unsafe { &*(self as *const Self as *const _) };

        let result = bytes.iter().fold(0_u8, |acc, &x| acc.wrapping_add(x));
        result == 0
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct RsdpV1 {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

impl Rsdp for RsdpV1 {}

impl RsdpV1 {
    pub fn rsdt_address(&self, physical_offset: *const u8) -> *const u8 {
        unsafe { physical_offset.add(self.rsdt_address as usize) }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct RsdpV2 {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

impl Rsdp for RsdpV2 {}

pub fn init(base: *const u8) -> &'static RsdpV1 {
    let rsdp = unsafe { &*(base as *mut RsdpV1) };
    assert!(rsdp.validate());

    match rsdp.revision {
        0 => {
            // Version 1.0
            rsdp
        }
        2 => {
            // Version 2.0
            let rsdp = unsafe { &*(base as *mut RsdpV2) };
            assert!(rsdp.validate());

            println!("xsdt: {:p}", rsdp.xsdt_address as *const u8);
            unimplemented!("rsdp version 2");
        }
        v => panic!("unknown rsdp revision: {}", v),
    }
}
