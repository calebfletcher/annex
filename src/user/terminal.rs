use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    gui::colour,
    gui::{Draw, Window},
    serial_println,
    utils::font::Font,
};

pub struct TextConsole {
    window: Arc<Mutex<Window>>,
    font: Font,
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
            font: Font::new(font_weight, font_size, colour::WHITE_ON_BLACK),
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
        let height = self.window.lock().height();

        // Move existing content up
        for new_row in 0..(height - pixel_delta).max(0) {
            let old_row = new_row + pixel_delta;

            self.window.lock().copy_row(old_row, new_row);
        }

        // Blank lines that have come in
        for new_row in (height - pixel_delta).max(0)..height {
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
                let width = self
                    .font
                    .write_char(&mut *self.window.lock(), self.row, self.col, c);
                if let Some(width) = width {
                    self.col += width;
                    if self.col >= self.window.lock().width() {
                        self.newline();
                    }
                }
            }
        };
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
