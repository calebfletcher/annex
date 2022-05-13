use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    colour::{self},
    gui::{Draw, Window},
    serial_println,
};

pub struct TextConsole {
    window: Arc<Mutex<Window>>,
    font_weight: noto_sans_mono_bitmap::FontWeight,
    font_size: noto_sans_mono_bitmap::BitmapHeight,
    line_height: usize,
    row: usize,
    col: usize,
}

impl TextConsole {
    pub fn new(window: Arc<Mutex<Window>>) -> Self {
        let font_weight = noto_sans_mono_bitmap::FontWeight::Regular;
        let font_size = noto_sans_mono_bitmap::BitmapHeight::Size14;

        Self {
            window,
            line_height: font_size.val() + 2,
            font_weight,
            font_size,
            row: 0,
            col: 0,
        }
    }

    pub fn newline(&mut self) {
        self.row += self.line_height;
        self.col = 0;

        if self.row + self.line_height >= self.window.lock().height() {
            self.scroll_by(1);
            self.row -= self.line_height;
        }
    }

    /// Scroll the terminal up by `n` lines.
    ///
    /// After this has been called, the cursor will be in the same place on the
    /// window, i.e. will seem to have been moved `n` lines down in the content.
    pub fn scroll_by(&mut self, n: usize) {
        let pixel_delta = n * self.line_height;

        // Move existing content up
        for new_row in 0..(self.window.lock().height() - pixel_delta).max(0) {
            let old_row = new_row + pixel_delta;

            self.window.lock().copy_row(old_row, new_row);
        }

        // Blank lines that have come in
        for new_row in
            (self.window.lock().height() - pixel_delta).max(0)..self.window.lock().height()
        {
            self.window.lock().write_row(new_row, colour::BLACK);
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.window.lock().clear(colour::BLACK);
    }

    #[allow(dead_code)]
    pub fn goto(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
    }

    pub fn move_cursor(&mut self, dir: Direction, amount: usize) {
        match dir {
            Direction::Up => self.row = self.row.saturating_sub(amount * self.line_height),
            Direction::Down => {
                self.row = self
                    .window
                    .lock()
                    .height()
                    .min(self.row + amount * self.line_height)
            }
            Direction::Left => self.col = self.col.saturating_sub(amount * 8),
            Direction::Right => self.col = self.window.lock().width().min(self.col + amount * 8),
        };
    }

    pub fn write_char(&mut self, c: char) {
        match c {
            '\n' => {
                self.newline();
            }
            _ => {
                if let Some(width) =
                    self.write_char_at(self.row, self.col, c, colour::WHITE_ON_BLACK)
                {
                    self.col += width;
                    if self.col >= self.window.lock().width() {
                        self.newline();
                    }
                }
            }
        };
    }

    pub fn write_char_at(
        &mut self,
        base_row: usize,
        base_col: usize,
        c: char,
        colour: colour::TextColour,
    ) -> Option<usize> {
        if let Some(font) = noto_sans_mono_bitmap::get_bitmap(c, self.font_weight, self.font_size) {
            for (row_i, &row) in font.bitmap().iter().enumerate() {
                for (col_i, &pixel) in row.iter().enumerate() {
                    let pixel_row = base_row + row_i;
                    let pixel_col = base_col + col_i;

                    self.window.lock().write_pixel(
                        pixel_row,
                        pixel_col,
                        colour.lerp(pixel as f32 / 255.0).unwrap(),
                    );
                }
            }
            Some(font.width())
        } else {
            None
        }
    }
}

pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl vte::Perform for TextConsole {
    fn print(&mut self, c: char) {
        self.write_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            8 => serial_println!("backspace"),
            9 => serial_println!("tab"),
            10 => self.write_char('\n'),
            _ => serial_println!("execute {}", byte),
        }
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
        serial_println!("hook");
    }

    fn put(&mut self, _byte: u8) {
        serial_println!("put");
    }

    fn unhook(&mut self) {
        serial_println!("unhook");
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        serial_println!("osc dispatch");
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        match action {
            'D' => {
                self.move_cursor(Direction::Left, 1);
            }
            'J' => {
                if let Some(mode) = params.iter().next() {
                    if mode.get(0) == Some(&2) {
                        self.clear();
                        self.goto(0, 0);
                    }
                }
            }
            _ => {
                serial_println!("csi dispatch {} {:?} {:?}", action, params, intermediates);
            }
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        serial_println!("esc dispatch");
    }
}

pub struct Terminal {
    console: TextConsole,
    state_machine: vte::Parser,
}

impl Terminal {
    pub fn new(window: Arc<Mutex<Window>>) -> Self {
        Self {
            console: TextConsole::new(window),
            state_machine: vte::Parser::new(),
        }
    }
}

impl core::fmt::Write for Terminal {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for &byte in s.as_bytes() {
            self.state_machine.advance(&mut self.console, byte);
        }

        Ok(())
    }
}
