use crossbeam::queue::ArrayQueue;
use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::mutex::Mutex;

use crate::task::keyboard::KEY_WAKER;

lazy_static! {
    pub static ref KEY_STREAM: ArrayQueue<DecodedKey> = ArrayQueue::new(128);
    static ref KEYBOARD_DECODER: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(
        Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore)
    );
}

pub fn process_keyboard(scancode: u8) {
    let mut kb = KEYBOARD_DECODER.try_lock().unwrap();

    if let Some(event) = kb.add_byte(scancode).unwrap() {
        if let Some(key) = kb.process_keyevent(event) {
            KEY_STREAM.force_push(key);

            // TODO: Abstract away how an interrupt-based io event notifies tasks
            KEY_WAKER.wake();
        }
    }
}
