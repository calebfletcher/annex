use core::arch::asm;

pub mod scheduler;
pub mod thread;
pub use thread::{Thread, ThreadState};

use self::thread::ThreadId;

/// # Safety
/// Supplied pointers must be valid threads.
#[naked]
pub unsafe extern "C" fn switch_to_thread(from_tcb: *const Thread, to_tcb: *const Thread) {
    unsafe {
        asm!(
            "
        // store registers
        push rbx
        push r12
        push r13
        push r14
        push r15

        mov [rdi+8], rsp        // save old thead's stack pointer

        mov rsp, [rsi + 8]      // load new thread's stack pointer
        mov rax, [rsi + 16]     // load new thread's page table

        // mov rcx, [rdx]
        // mov [rdi], rcx          // update CURRENT_THREAD_ID with new thread's id
        
        // TODO: load TSS ESP0?

        mov rcx, cr3            // get old thread's page table
        cmp rax, rcx            // check if page tables are the same
        je 2f                   // skip if the same
        mov cr3, rax            // load new page table (also flushes the TLB)
    
    2:
        // load registers
        pop r15
        pop r14
        pop r13
        pop r12
        pop rbx

        // last thing on the stack is rip
        ret

    ",
            options(noreturn)
        );
    }
}

/// # Safety
/// Interrupts must be disabled before calling this function, and the
/// scheduler lock must be unlocked.
pub unsafe fn switch(from: ThreadId, to: ThreadId) {
    let (current_thread, next_thread) =
        unsafe { scheduler::with_scheduler_from_irq(|s| s.thread_pointers(from, to)) };

    unsafe { switch_to_thread(current_thread, next_thread) };
}

pub fn yield_now() {
    unsafe {
        if let Some((prev_thread, next_thread)) = scheduler::with_scheduler(|s| {
            s.schedule()
                .map(|(from_id, next_id)| s.thread_pointers(from_id, next_id))
        }) {
            switch_to_thread(prev_thread, next_thread);
        }
    };
}

// pub fn block_current_thread(reason: BlockReason) {
//     interrupts::disable();

//     if let Some(tcb) = THREADS.lock().get_mut(&ThreadId::from_usize(
//         ACTIVE_THREAD_ID.load(Ordering::Acquire),
//     )) {
//         // Add task to blocked list
//         #[allow(clippy::single_match)]
//         match reason {
//             BlockReason::Sleep(deadline) => {
//                 let mut sleeping_threads = SLEEPING_THREADS.try_get().unwrap().lock();
//                 let entry = sleeping_threads.entry(deadline);
//                 entry.or_default().insert(tcb.id());
//             }

//             _ => {
//                 //warn!("no thread list defined for {:?}", reason);
//             }
//         }

//         tcb.set_state(ThreadState::Blocked(reason));
//     }

//     // TODO: make is to this schedule call can be done. Currently, schedule()
//     // requires interrupts to be enabled, but if we call this after enabling
//     // then there could be a race condition where the schedule call happens
//     // immediately after an interrupt-based schedule.
//     //unsafe { schedule() };

//     interrupts::enable();
// }

// pub fn unblock_thread(id: usize) {
//     interrupts::disable();

//     let _next_id = if let Some(tcb) = THREADS.lock().get_mut(&ThreadId::from_usize(id)) {
//         tcb.set_state(ThreadState::ReadyToRun);
//         READY_THREADS.try_get().unwrap().lock().push_back(tcb.id());
//         tcb.id()
//     } else {
//         warn!("attempted to unblock a thread which is not blocked");
//         return;
//     };

//     // TODO: potentially switch to the unblocked task now iff there is only one
//     // active task (since it presumably got a lot of CPU time)

//     interrupts::enable();
// }
