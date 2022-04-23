use super::rsdt::{Rsdt, SystemDescriptor};

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Madt {
    header: Rsdt,
    local_apic_address: u32,
    flags: u32,
}

impl SystemDescriptor for Madt {
    const SIGNATURE: &'static [u8; 4] = b"APIC";
}

impl Madt {
    pub fn local_apic_address(&self, physical_offset: *const u8) -> *const u8 {
        // Enumerate records to see if there is an override

        let mut ptr =
            unsafe { (self as *const Self as *const u8).add(core::mem::size_of::<Self>()) };
        let length = self.header.length as usize - core::mem::size_of::<Self>();
        let end_ptr = unsafe { ptr.add(length) };

        while ptr < end_ptr {
            let record = unsafe { &*(ptr as *const RecordHeader) };
            ptr = unsafe { ptr.add(record.record_length as usize) };

            if record.entry_type == 5 {
                unimplemented!("local apic address override");
            }
        }

        unsafe { physical_offset.add(self.local_apic_address as usize) }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct RecordHeader {
    entry_type: u8,
    record_length: u8,
}
