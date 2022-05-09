use pic8259::ChainedPics;
use spin::Mutex;

pub const PIC_1_OFFSET: u8 = 0x20;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

static PICS: Mutex<ChainedPics> =
    unsafe { Mutex::new(ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET)) };

pub fn disable() {
    unsafe {
        let mut pics = PICS.lock();
        pics.initialize();
        pics.write_masks(0xFF, 0xFF)
    };
}
