use crate::colour::Colour;

use font8x8::{UnicodeFonts, BASIC_FONTS};

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

    pub fn write_char(&mut self, c: char, base_row: usize, base_col: usize, colour: Colour) {
        let font = BASIC_FONTS.get(c).unwrap();
        for (row, font_row) in font.into_iter().enumerate() {
            for col in 0..8 {
                if font_row & 1 << col != 0 {
                    let pixel_row = base_row + row;
                    let pixel_col = base_col + col;

                    self.set_pixel(pixel_row, pixel_col, colour);
                }
            }
        }
    }

    pub fn write_chars(&mut self, chars: &str, base_row: usize, base_col: usize, colour: Colour) {
        for (i, c) in chars.chars().enumerate() {
            self.write_char(c, base_row, base_col + 8 * i, colour);
        }
    }
}
