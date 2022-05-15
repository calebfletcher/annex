use noto_sans_mono_bitmap::{BitmapHeight, FontWeight};

use crate::{gui::colour::TextColour, gui::Draw};

pub struct Font {
    weight: FontWeight,
    size: BitmapHeight,
    colour: TextColour,
}

impl Font {
    pub fn new(weight: FontWeight, size: BitmapHeight, colour: TextColour) -> Self {
        Self {
            weight,
            size,
            colour,
        }
    }

    pub fn write_char(
        &self,
        surface: &mut impl Draw,
        base_row: usize,
        base_col: usize,
        c: char,
    ) -> Option<usize> {
        let font = noto_sans_mono_bitmap::get_bitmap(c, self.weight, self.size)?;

        // Iterate over each pixel in the bitmap
        for (row_i, &row) in font.bitmap().iter().enumerate() {
            for (col_i, &pixel) in row.iter().enumerate() {
                let pixel_row = base_row + row_i;
                let pixel_col = base_col + col_i;

                surface.write_pixel(
                    pixel_row,
                    pixel_col,
                    self.colour.lerp(pixel as f32 / 255.0).unwrap(),
                );
            }
        }

        Some(font.width())
    }

    pub fn write(
        &self,
        surface: &mut impl Draw,
        base_row: usize,
        base_col: usize,
        s: &str,
    ) -> Option<usize> {
        let mut offset = 0;
        for c in s.chars() {
            if let Some(width) = self.write_char(surface, base_row, base_col + offset, c) {
                offset += width;
            }
        }

        Some(offset)
    }
}
