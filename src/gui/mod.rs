use alloc::{sync::Arc, vec::Vec};

use self::colour::{Colour, RGBA_BYTES_PER_PIXEL};

pub mod colour;
pub mod screen;

pub use screen::Screen;

pub trait Draw {
    fn width(&self) -> usize;
    fn height(&self) -> usize;

    fn clear(&mut self, colour: Colour) {
        for row in 0..self.height() {
            self.write_row(row, colour);
        }
    }

    fn write_pixel(&mut self, row: usize, col: usize, colour: Colour);

    fn write_row(&mut self, row: usize, colour: Colour);

    fn copy_row(&mut self, row_from: usize, row_to: usize);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coordinates {
    pub x: isize,
    pub y: isize,
    width: usize,
    height: usize,
}

impl Coordinates {
    pub fn new(x: isize, y: isize, width: usize, height: usize) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, x: isize, y: isize) -> bool {
        x >= self.x
            && x < self.x + self.width as isize
            && y >= self.y
            && y < self.y + self.height as isize
    }
}

pub struct Window {
    pub coordinates: Coordinates,
    buffer: Vec<u8>,
}

impl Draw for Window {
    fn width(&self) -> usize {
        self.coordinates.width
    }

    fn height(&self) -> usize {
        self.coordinates.height
    }

    /// Set a pixel on the screen
    fn write_pixel(&mut self, row: usize, col: usize, colour: Colour) {
        let offset = (row * self.width() + col) * RGBA_BYTES_PER_PIXEL;
        let pixel = &mut self.buffer[offset..offset + RGBA_BYTES_PER_PIXEL];

        pixel[..3].copy_from_slice(&[colour.r, colour.g, colour.b]);
    }

    fn write_row(&mut self, row: usize, colour: Colour) {
        let stride = self.width() * RGBA_BYTES_PER_PIXEL;
        let offset = row * stride;
        let frame_line = &mut self.buffer[offset..offset + stride];

        for pixel in frame_line.chunks_exact_mut(RGBA_BYTES_PER_PIXEL) {
            pixel[..3].copy_from_slice(&[colour.r, colour.g, colour.b]);
        }
    }

    fn copy_row(&mut self, row_from: usize, row_to: usize) {
        let bytes_per_row = self.width() * RGBA_BYTES_PER_PIXEL;

        let row_from_offset = row_from * self.width() * RGBA_BYTES_PER_PIXEL;
        let row_from = row_from_offset..row_from_offset + bytes_per_row;

        let row_to_offset = row_to * self.width() * RGBA_BYTES_PER_PIXEL;

        self.buffer.copy_within(row_from, row_to_offset);
    }
}

pub fn new_window(initial: Coordinates) -> Arc<spin::Mutex<Window>> {
    screen::SCREEN.try_get().unwrap().lock().new_window(initial)
}
