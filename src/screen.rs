use crate::colour::{self, Colour};

use font8x8::{UnicodeFonts, BASIC_FONTS};

static CHAR_REPLACEMENT: [u8; 8] = [
    0b11111111, 0b10000001, 0b10000001, 0b10000001, 0b10000001, 0b10000001, 0b10000001, 0b11111111,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextColour {
    foreground: Option<Colour>,
    background: Option<Colour>,
}

impl TextColour {
    const fn new(foreground: Colour, background: Colour) -> Self {
        Self {
            foreground: Some(foreground),
            background: Some(background),
        }
    }
    #[allow(dead_code)]
    const fn from_foreground(foreground: Colour) -> Self {
        Self {
            foreground: Some(foreground),
            background: None,
        }
    }
    #[allow(dead_code)]
    const fn from_background(background: Colour) -> Self {
        Self {
            foreground: None,
            background: Some(background),
        }
    }
}

#[allow(dead_code)]
pub static WHITE_ON_BLACK: TextColour = TextColour::new(colour::WHITE, colour::BLACK);
#[allow(dead_code)]
pub static BLACK_ON_WHITE: TextColour = TextColour::new(colour::BLACK, colour::WHITE);

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
                self.write_pixel(row, col, colour);
            }
        }
    }

    /// Set a pixel on the screen
    pub fn write_pixel(&mut self, row: usize, col: usize, colour: Colour) {
        let offset = (row * self.info.stride + col) * self.info.bytes_per_pixel;
        let pixel = &mut self.buffer[offset..offset + self.info.bytes_per_pixel];

        set_pixel_slice(self.info.pixel_format, pixel, colour);
    }

    pub fn write_row(&mut self, row: usize, colour: Colour) {
        let offset = row * self.info.stride * self.info.bytes_per_pixel;
        let frame_line = &mut self.buffer
            [offset..offset + (self.info.horizontal_resolution * self.info.bytes_per_pixel)];
        for pixel in frame_line.chunks_exact_mut(self.info.bytes_per_pixel) {
            set_pixel_slice(self.info.pixel_format, pixel, colour);
        }
    }

    pub fn write_char(&mut self, c: char, base_row: usize, base_col: usize, colour: TextColour) {
        let font = BASIC_FONTS.get(c).unwrap_or(CHAR_REPLACEMENT);
        for (row, font_row) in font.into_iter().enumerate() {
            for col in 0..8 {
                let pixel_row = base_row + row;
                let pixel_col = base_col + col;
                if font_row & 1 << col != 0 {
                    // Foreground

                    if let Some(colour) = colour.foreground {
                        self.write_pixel(pixel_row, pixel_col, colour);
                    }
                } else {
                    // Background
                    if let Some(colour) = colour.background {
                        self.write_pixel(pixel_row, pixel_col, colour);
                    }
                }
            }
        }
    }

    pub fn copy_row(&mut self, row_from: usize, row_to: usize) {
        let bytes_per_row = self.info.horizontal_resolution * self.info.bytes_per_pixel;

        let row_from_offset = row_from * self.info.stride * self.info.bytes_per_pixel;
        let row_from = row_from_offset..row_from_offset + bytes_per_row;

        let row_to_offset = row_to * self.info.stride * self.info.bytes_per_pixel;

        self.buffer.copy_within(row_from, row_to_offset);
    }
}

fn set_pixel_slice(
    pixel_format: bootloader::boot_info::PixelFormat,
    pixel: &mut [u8],
    colour: Colour,
) {
    match pixel_format {
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

pub struct Console<'a> {
    screen: Screen<'a>,
    line_height: usize,
    width: usize,
    height: usize,
    row: usize,
    col: usize,
}

impl<'a> Console<'a> {
    pub fn new(screen: Screen<'a>) -> Self {
        let width = screen.info.horizontal_resolution;
        let height = screen.info.vertical_resolution;
        Self {
            screen,
            line_height: 8,
            width,
            height,
            row: 0,
            col: 0,
        }
    }

    pub fn write_char_colour(&mut self, c: char, colour: TextColour) {
        match c {
            '\n' => {
                self.newline();
            }
            _ => {
                self.screen.write_char(c, self.row, self.col, colour);
                self.col += 8; // TODO: Support different font widths
                if self.col >= self.width {
                    self.newline();
                }
            }
        }
    }

    pub fn write_colour(&mut self, line: &str, colour: TextColour) {
        for c in line.chars() {
            self.write_char_colour(c, colour);
        }
    }

    pub fn newline(&mut self) {
        self.row += self.line_height;
        self.col = 0;

        if self.row >= self.height {
            self.scroll_by(1);
            self.row -= self.line_height;
        }
    }

    /// Scroll the terminal up by `n` lines.
    ///
    /// After this has been called, the cursor will be in the same place on the
    /// screen, i.e. will seem to have been moved `n` lines down in the content.
    pub fn scroll_by(&mut self, n: usize) {
        let pixel_delta = n * self.line_height;

        // Move existing content up
        for new_row in 0..(self.height - pixel_delta).max(0) {
            let old_row = new_row + pixel_delta;

            self.screen.copy_row(old_row, new_row);
        }

        // Blank lines that have come in
        for new_row in (self.height - pixel_delta).max(0)..self.height {
            self.screen.write_row(new_row, colour::BLACK);
        }
    }
}

impl core::fmt::Write for Console<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_colour(s, WHITE_ON_BLACK);
        Ok(())
    }
}
