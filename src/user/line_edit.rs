use core::{fmt::Write, marker, mem};

use alloc::{format, string::String};
use futures_util::{Stream, StreamExt};
use pc_keyboard::DecodedKey;

use crate::screen::Terminal;

pub struct Editor<S>
where
    S: Stream<Item = DecodedKey>,
{
    input: S,
    current_line: String,
}

impl<S: Stream<Item = DecodedKey> + marker::Unpin> Editor<S> {
    pub fn new(input: S) -> Self {
        Self {
            input,
            current_line: String::new(),
        }
    }

    /// Get a line from the user
    ///
    /// Does not include the final newline.
    pub async fn prompt(&mut self, prompt: &str, terminal: &mut Terminal) -> String {
        terminal.write_str(prompt).unwrap();
        while let Some(key) = self.input.next().await {
            // Ignore raw keys (Alt, Ctrl, other modifier keys)
            let key = match key {
                DecodedKey::RawKey(_key) => {
                    continue;
                }
                DecodedKey::Unicode(key) => key,
            };

            match key {
                ' '..='~' => {
                    self.current_line.push(key);
                    terminal.write_str(&format!("{}", key)).unwrap();
                }
                '\n' => {
                    // Don't include final newline
                    terminal.write_str(&format!("{}", key)).unwrap();
                    break;
                }
                '\x08' => {
                    // Backspace
                    if self.current_line.pop().is_some() {
                        terminal.write_str("\x1b[D \x1b[D").unwrap();
                    }
                }
                '\x7F' => {
                    // Delete
                    self.current_line.pop();
                }
                _ => {}
            }
        }

        // Take the current line, and leave a new String instance in its place
        mem::take(&mut self.current_line)
    }
}
