use core::{
    arch::asm,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use alloc::{borrow::ToOwned, collections::BTreeMap};

use log::error;
use spin::Mutex;
use x86_64::{instructions::interrupts, registers::control::Cr3};

pub mod thread;
pub use thread::{Thread, ThreadState};

use self::thread::Stack;

/// Map between thread id and thread object
static THREADS: Mutex<BTreeMap<usize, Thread>> = Mutex::new(BTreeMap::new());
pub static ACTIVE_THREAD_ID: AtomicUsize = AtomicUsize::new(0);
pub static SCHEDULER_ENABLED: AtomicBool = AtomicBool::new(false);

// Create a thread struct for the initial kernel thread that is running (id 0)
pub fn init() {
    let (page_table, _) = Cr3::read();
    let page_table = page_table.start_address();

    let tcb = Thread::bootstrap(page_table);
    THREADS.lock().insert(tcb.id(), tcb);
}

pub fn add_thread(entry: fn() -> !, stack_size: usize) {
    // Get address to kernel's page table
    let (page_table, _) = Cr3::read();
    let page_table = page_table.start_address();

    let stack = Stack::new(stack_size, entry);

    // Add a new thread to the list
    let tcb = Thread::new(
        stack.initial_stack_pointer(),
        page_table,
        "kernel2".to_owned(),
        stack,
    );
    THREADS.lock().insert(tcb.id(), tcb);
}

pub fn start() {
    SCHEDULER_ENABLED.store(true, Ordering::Release);
}

pub fn schedule() {
    interrupts::disable();

    if !SCHEDULER_ENABLED.load(Ordering::Acquire) {
        return;
    }

    let threads = THREADS.lock();

    // Round robin scheduler
    let current_id = ACTIVE_THREAD_ID.load(Ordering::Acquire);

    // Create an iterator
    let mut thread_iter = threads.iter();
    let _ = thread_iter.find(|(_, tcb)| tcb.id() == current_id);

    // Either get the next thread in the list, or wrap back around to the start
    let next_id = match thread_iter.next() {
        Some((&next_id, _next_tcb)) => Some(next_id),
        None => threads.first_key_value().map(|(&id, _tcb)| id),
    };

    // Drop mutex lock
    drop(threads);

    // If there was a thread, switch to it
    match next_id {
        Some(next_id) => unsafe {
            switch(next_id);
        },
        None => {
            error!("no threads available to be scheduled");
        }
    }

    interrupts::enable();
}

/// # Safety
/// Interrupts must be disabled before calling this function
#[naked]
pub unsafe extern "C" fn switch_to_thread(
    current_id: *mut usize,
    from_tcb: *const Thread,
    to_tcb: *const Thread,
) {
    unsafe {
        asm!(
            "
        // store registers
        push rbx
        push r12
        push r13
        push r14
        push r15

        mov [rsi+8], rsp        // save old thead's stack pointer

        mov rsp, [rdx + 8]      // load new thread's stack pointer
        mov rax, [rdx + 16]     // load new thread's page table

        mov rcx, [rdx]
        mov [rdi], rcx          // update CURRENT_THREAD_ID with new thread's id
        
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
pub unsafe fn switch(to: usize) {
    let from = ACTIVE_THREAD_ID.load(Ordering::SeqCst);

    let threads = THREADS.lock();
    let current_thread = threads.get(&from).unwrap() as *const Thread;
    let next_thread = threads.get(&to).unwrap() as *const Thread;
    drop(threads);

    unsafe { switch_to_thread(ACTIVE_THREAD_ID.as_mut_ptr(), current_thread, next_thread) };
}
