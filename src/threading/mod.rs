use core::arch::asm;

pub mod scheduler;
pub mod thread;

use crate::interrupts;
pub use scheduler::Deadline;
pub use thread::{Thread, ThreadState};

#[repr(C)]
pub struct ContextSwitchResponse {
    previous: *mut Thread,
    next: *const Thread,
}

/// # Safety
/// This function preserves no registers.
#[no_mangle]
pub unsafe extern "C" fn context_switch() -> ContextSwitchResponse {
    unsafe {
        scheduler::with_scheduler(|s| {
            if let Some((prev_thread_id, next_thread_id)) = s.schedule() {
                let (previous, next) = s.thread_pointers(prev_thread_id, next_thread_id);
                ContextSwitchResponse { previous, next }
            } else {
                ContextSwitchResponse {
                    previous: core::ptr::null_mut::<Thread>(),
                    next: core::ptr::null::<Thread>(),
                }
            }
        })
    }
}

pub fn yield_now() {
    unsafe { x86_64::software_interrupt!(interrupts::InterruptIndex::CTXSWITCH as usize) };
}

pub fn sleep(deadline: Deadline) {
    scheduler::with_scheduler(|s| s.sleep_thread(s.current_thread_id(), deadline));
    yield_now();
}
