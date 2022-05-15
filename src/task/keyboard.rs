use core::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::{task::AtomicWaker, Stream};
use pc_keyboard::DecodedKey;

use crate::hardware;

pub static KEY_WAKER: AtomicWaker = AtomicWaker::new();

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
        let queue = &hardware::keyboard::KEY_STREAM;

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
