use core::{marker, mem};

use alloc::string::String;
use futures_util::{Stream, StreamExt};
use pc_keyboard::DecodedKey;

use crate::{print, println, task::keyboard};

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
    pub async fn prompt(&mut self, prompt: &str) -> String {
        print!("{}", prompt);
        while let Some(key) = self.input.next().await {
            // Ignore raw keys (Alt, Ctrl, other modifier keys)
            let key = match key {
                DecodedKey::RawKey(key) => {
                    println!("ignoring {:?}", key);
                    continue;
                }
                DecodedKey::Unicode(key) => key,
            };

            match key {
                ' '..='~' => {
                    self.current_line.push(key);
                    print!("{}", key);
                }
                '\n' => {
                    // Don't include final newline
                    print!("{}", key);
                    break;
                }
                '\x08' => {
                    // Backspace
                    if self.current_line.pop().is_some() {
                        print!("\x1b[D \x1b[D");
                    }
                }
                '\x7F' => {
                    // Delete
                    self.current_line.pop();
                }
                _ => {
                    println!("ignoring {:?}", key);
                }
            }
        }

        // Take the current line, and leave a new String instance in its place
        mem::take(&mut self.current_line)
    }
}

pub async fn run() {
    let stream = keyboard::KeyStream::new();
    let mut editor = Editor::new(stream);

    loop {
        let line = editor.prompt("> ").await;
        println!("got line: {line}");
    }
}
