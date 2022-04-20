use crate::colour::Colour;

pub struct Screen<'a> {
    buffer: &'a mut [u8],
    info: &'a bootloader::boot_info::FrameBufferInfo,
}

impl<'a> Screen<'a> {
    pub fn new(buffer: &'a mut [u8], info: &'a bootloader::boot_info::FrameBufferInfo) -> Self {
        Self { buffer, info }
    }

    pub fn clear(&mut self, colour: Colour) {
        for row in 0..self.info.vertical_resolution {
            for col in 0..self.info.horizontal_resolution {
                self.set_pixel(row, col, colour);
            }
        }
    }

    /// Set a pixel on the screen
    pub fn set_pixel(&mut self, row: usize, col: usize, colour: Colour) {
        let offset = (row * self.info.stride + col) * self.info.bytes_per_pixel;
        let pixel = &mut self.buffer[offset..offset + self.info.bytes_per_pixel];

        match self.info.pixel_format {
            bootloader::boot_info::PixelFormat::RGB => {
                pixel[..3].copy_from_slice(&[colour.r, colour.g, colour.b]);
            }
            bootloader::boot_info::PixelFormat::BGR => {
                pixel[..3].copy_from_slice(&[colour.b, colour.g, colour.r]);
            }
            bootloader::boot_info::PixelFormat::U8 => {
                pixel[..1].copy_from_slice(&[colour.r]);
            }
            _ => unimplemented!(),
        }
    }
}
