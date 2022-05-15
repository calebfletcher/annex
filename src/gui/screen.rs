use alloc::string::String;
use alloc::vec;
use alloc::{sync::Arc, vec::Vec};
use bootloader::boot_info::PixelFormat;
use conquer_once::noblock::OnceCell;

use crate::utils::font::Font;

use super::colour::{self, Colour, RGBA_BYTES_PER_PIXEL};
use super::{Coordinates, Draw, Window};

pub static SCREEN: OnceCell<spin::Mutex<Screen>> = OnceCell::uninit();

static TITLE_HEIGHT: usize = 20;
static TITLE_FONT: Font = Font::new(
    noto_sans_mono_bitmap::FontWeight::Regular,
    noto_sans_mono_bitmap::BitmapHeight::Size14,
    colour::TextColour::new(colour::WHITE, colour::LIGHT_GREY),
);

pub struct Screen {
    front_buffer: &'static mut [u8],
    back_buffer: Vec<u8>,
    info: bootloader::boot_info::FrameBufferInfo,
    windows: Arc<spin::Mutex<Vec<Arc<spin::Mutex<Window>>>>>,
}

impl Screen {
    pub fn init(front_buffer: &'static mut [u8], info: bootloader::boot_info::FrameBufferInfo) {
        let num_bytes =
            info.horizontal_resolution * info.vertical_resolution * RGBA_BYTES_PER_PIXEL;
        let back_buffer = vec![0; num_bytes];

        SCREEN
            .try_init_once(|| {
                spin::Mutex::new(Self {
                    front_buffer,
                    back_buffer,
                    info,
                    windows: Arc::new(spin::Mutex::new(Vec::new())),
                })
            })
            .unwrap();

        SCREEN.try_get().unwrap().lock().clear(colour::BLACK);
    }

    pub fn render(&mut self) {
        self.clear(colour::GREY);

        let windows = Arc::clone(&self.windows);
        for window in windows.lock().iter() {
            let mut window = window.lock();

            // Draw title box
            for row in 0..TITLE_HEIGHT as isize {
                for col in 0..window.coordinates.width as isize {
                    let screen_x = window.coordinates.x + col;
                    let screen_y = window.coordinates.y + row - TITLE_HEIGHT as isize;

                    // Don't print pixels outside the screen area
                    if screen_x < 0
                        || screen_x >= self.width() as isize
                        || screen_y < 0
                        || screen_y >= self.height() as isize
                    {
                        continue;
                    }

                    self.write_pixel(screen_y, screen_x, colour::LIGHT_GREY);
                }
            }

            // Draw title
            TITLE_FONT.write(
                self,
                window.coordinates.y - TITLE_HEIGHT as isize + 3,
                window.coordinates.x + 4,
                window.name(),
            );

            for row in 0..window.coordinates.height as isize {
                for col in 0..window.coordinates.width as isize {
                    let screen_x = window.coordinates.x + col;
                    let screen_y = window.coordinates.y + row;

                    // Don't print pixels outside the screen area
                    if screen_x < 0
                        || screen_x >= self.width() as isize
                        || screen_y < 0
                        || screen_y >= self.height() as isize
                    {
                        continue;
                    }

                    // Get screen pixel
                    let screen_offset = (screen_y as usize * self.width() + screen_x as usize)
                        * RGBA_BYTES_PER_PIXEL;
                    let screen_pixel =
                        &mut self.back_buffer[screen_offset..screen_offset + RGBA_BYTES_PER_PIXEL];

                    // Get window pixel
                    let window_offset = (row as usize * window.coordinates.width + col as usize)
                        * RGBA_BYTES_PER_PIXEL;
                    let window_pixel =
                        &mut window.buffer[window_offset..window_offset + RGBA_BYTES_PER_PIXEL];

                    // Copy pixel from window to screen
                    // TODO: lerp colours with alpha channel for transparency
                    screen_pixel[..3].copy_from_slice(&window_pixel[..3]);
                }
            }
        }

        self.swap();
    }

    // Render the back buffer onto the front buffer
    fn swap(&mut self) {
        for row in 0..self.info.vertical_resolution {
            for col in 0..self.info.horizontal_resolution {
                // Get front buffer pixel
                let front_offset = (row * self.info.stride + col) * self.info.bytes_per_pixel;
                let front_pixel =
                    &mut self.front_buffer[front_offset..front_offset + self.info.bytes_per_pixel];

                // Get back buffer pixel
                let back_offset =
                    (row * self.info.horizontal_resolution + col) * RGBA_BYTES_PER_PIXEL;
                let back_pixel = &self.back_buffer[back_offset..back_offset + RGBA_BYTES_PER_PIXEL];

                // Copy pixel from back buffer to front buffer
                Self::swap_pixel(self.info.pixel_format, back_pixel, front_pixel);
            }
        }
    }

    fn swap_pixel(pixel_format: PixelFormat, back_pixel: &[u8], front_pixel: &mut [u8]) {
        match pixel_format {
            bootloader::boot_info::PixelFormat::RGB => {
                front_pixel[..3].copy_from_slice(&back_pixel[..3]);
            }
            bootloader::boot_info::PixelFormat::BGR => {
                front_pixel[..3].copy_from_slice(&[back_pixel[2], back_pixel[1], back_pixel[0]]);
            }
            _ => unimplemented!(),
        }
    }

    pub fn new_window(&mut self, name: String, initial: Coordinates) -> Arc<spin::Mutex<Window>> {
        let buffer_size = initial.width * initial.height * RGBA_BYTES_PER_PIXEL;
        let buffer = vec![0; buffer_size];

        let window = Arc::new(spin::Mutex::new(Window {
            name,
            coordinates: initial,
            buffer,
        }));

        self.windows.lock().push(Arc::clone(&window));

        window
    }
}

impl Draw for Screen {
    fn width(&self) -> usize {
        self.info.horizontal_resolution
    }

    fn height(&self) -> usize {
        self.info.vertical_resolution
    }

    /// Set a pixel on the screen
    fn write_pixel(&mut self, row: isize, col: isize, colour: Colour) {
        if col < 0 || col >= self.width() as isize || row < 0 || row >= self.height() as isize {
            return;
        }

        let offset = (row as usize * self.width() + col as usize) * RGBA_BYTES_PER_PIXEL;
        let pixel = &mut self.back_buffer[offset..offset + RGBA_BYTES_PER_PIXEL];

        pixel[..3].copy_from_slice(&[colour.r, colour.g, colour.b]);
    }

    fn write_row(&mut self, row: usize, colour: Colour) {
        let stride = self.width() * RGBA_BYTES_PER_PIXEL;
        let offset = row * stride;
        let frame_line = &mut self.back_buffer[offset..offset + stride];

        for pixel in frame_line.chunks_exact_mut(RGBA_BYTES_PER_PIXEL) {
            pixel[..3].copy_from_slice(&[colour.r, colour.g, colour.b]);
        }
    }

    fn copy_row(&mut self, row_from: usize, row_to: usize) {
        let bytes_per_row = self.width() * RGBA_BYTES_PER_PIXEL;

        let row_from_offset = row_from * self.width() * RGBA_BYTES_PER_PIXEL;
        let row_from = row_from_offset..row_from_offset + bytes_per_row;

        let row_to_offset = row_to * self.width() * RGBA_BYTES_PER_PIXEL;

        self.back_buffer.copy_within(row_from, row_to_offset);
    }
}
