use core::{
    pin::Pin,
    task::{Context, Poll},
};

use conquer_once::noblock::OnceCell;
use crossbeam::queue::ArrayQueue;
use futures_util::{task::AtomicWaker, Stream, StreamExt};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

use crate::serial_println;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static SCANCODE_WAKER: AtomicWaker = AtomicWaker::new();
static KEY_WAKER: AtomicWaker = AtomicWaker::new();

pub fn init() {
    SCANCODE_QUEUE
        .try_init_once(|| ArrayQueue::new(100))
        .unwrap();

    KEY_STREAM.try_init_once(|| ArrayQueue::new(100)).unwrap();
}

/// Called by the keyboard interrupt handler
///
/// Must not block or allocate.
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if queue.push(scancode).is_err() {
            serial_println!("WARNING: scancode queue full; dropping keyboard input");
        } else {
            SCANCODE_WAKER.wake();
        }
    } else {
        serial_println!("WARNING: scancode queue uninitialized");
    }
}

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE.try_get().unwrap();

        // fast path
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        SCANCODE_WAKER.register(ctx.waker());
        match queue.pop() {
            Some(scancode) => {
                SCANCODE_WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

static KEY_STREAM: OnceCell<ArrayQueue<DecodedKey>> = OnceCell::uninit();

pub struct KeyStream {
    _private: (),
}

impl KeyStream {
    pub fn new() -> Self {
        KeyStream { _private: () }
    }
}

impl Stream for KeyStream {
    type Item = DecodedKey;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<DecodedKey>> {
        let queue = KEY_STREAM.try_get().unwrap();

        // fast path
        if let Some(key) = queue.pop() {
            return Poll::Ready(Some(key));
        }

        KEY_WAKER.register(ctx.waker());
        match queue.pop() {
            Some(key) => {
                KEY_WAKER.take();
                Poll::Ready(Some(key))
            }
            None => Poll::Pending,
        }
    }
}

pub async fn handle_keyboard() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                KEY_STREAM.try_get().unwrap().push(key).unwrap();
                KEY_WAKER.wake();
            }
        }
    }
}
