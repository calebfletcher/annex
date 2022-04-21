use crate::colour::{self, Colour};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextColour {
    foreground: Option<Colour>,
    background: Option<Colour>,
}

impl TextColour {
    pub const fn new(foreground: Colour, background: Colour) -> Self {
        Self {
            foreground: Some(foreground),
            background: Some(background),
        }
    }
    #[allow(dead_code)]
    pub const fn from_foreground(foreground: Colour) -> Self {
        Self {
            foreground: Some(foreground),
            background: None,
        }
    }
    #[allow(dead_code)]
    pub const fn from_background(background: Colour) -> Self {
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
    info: bootloader::boot_info::FrameBufferInfo,
}

impl<'a> Screen<'a> {
    pub fn new(buffer: &'a mut [u8], info: bootloader::boot_info::FrameBufferInfo) -> Self {
        Self { buffer, info }
    }

    pub fn clear(&mut self, colour: Colour) {
        for row in 0..self.info.vertical_resolution {
            self.write_row(row, colour);
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
    font_weight: noto_sans_mono_bitmap::FontWeight,
    font_size: noto_sans_mono_bitmap::BitmapHeight,
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

        let font_weight = noto_sans_mono_bitmap::FontWeight::Regular;
        let font_size = noto_sans_mono_bitmap::BitmapHeight::Size14;

        Self {
            screen,
            line_height: font_size.val() + 2,
            font_weight,
            font_size,
            width,
            height,
            row: 0,
            col: 0,
        }
    }

    pub fn write_colour(&mut self, line: &str, colour: TextColour) {
        for c in line.chars() {
            match c {
                '\n' => {
                    self.newline();
                }
                _ => {
                    if let Some(font) =
                        noto_sans_mono_bitmap::get_bitmap(c, self.font_weight, self.font_size)
                    {
                        for (row_i, &row) in font.bitmap().iter().enumerate() {
                            for (col_i, &pixel) in row.iter().enumerate() {
                                let pixel_row = self.row + row_i;
                                let pixel_col = self.col + col_i;
                                if pixel > 40 {
                                    // Foreground
                                    if let Some(colour) = colour.foreground {
                                        self.screen.write_pixel(pixel_row, pixel_col, colour);
                                    }
                                } else {
                                    // Background
                                    if let Some(colour) = colour.background {
                                        self.screen.write_pixel(pixel_row, pixel_col, colour);
                                    }
                                }
                            }
                        }
                        self.col += font.width();
                        if self.col >= self.width {
                            self.newline();
                        }
                    }
                }
            };
        }
    }

    pub fn newline(&mut self) {
        self.row += self.line_height;
        self.col = 0;

        if self.row + self.line_height >= self.height {
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

    pub fn clear(&mut self) {
        self.screen.clear(colour::BLACK);
    }

    pub fn goto(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
    }
}

impl core::fmt::Write for Console<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_colour(s, WHITE_ON_BLACK);
        Ok(())
    }
}

pub static CONSOLE: conquer_once::noblock::OnceCell<spin::mutex::SpinMutex<Console>> =
    conquer_once::noblock::OnceCell::uninit();

#[macro_export]
macro_rules! print {
        ($($arg:tt)*) => ($crate::screen::_print(format_args!($($arg)*)));
    }

#[macro_export]
macro_rules! println {
        () => ($crate::print!("\n"));
        ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
    }

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    CONSOLE.try_get().unwrap().lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! dbg {
    () => {
        $crate::println!("[{}:{}]", $crate::file!(), $crate::line!())
    };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                $crate::println!("[{}:{}] {} = {:#?}",
                    file!(), line!(), stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}
