use core::{
    arch::asm,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{borrow::ToOwned, collections::BTreeMap};

use spin::Mutex;
use x86_64::{instructions::interrupts, registers::control::Cr3};

pub mod thread;
pub use thread::{Thread, ThreadState};

use self::thread::Stack;

/// Map between thread id and thread object
static THREADS: Mutex<BTreeMap<usize, Thread>> = Mutex::new(BTreeMap::new());
pub static ACTIVE_THREAD_ID: AtomicUsize = AtomicUsize::new(0);

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

pub fn switch(to: usize) {
    interrupts::disable();

    let from = ACTIVE_THREAD_ID.load(Ordering::SeqCst);

    // TODO: fix this force unlock
    let threads = unsafe {
        THREADS.force_unlock();
        THREADS.lock()
    };
    // TODO: fix dodgy hack that handles interrupts before init
    if to >= threads.len() {
        return;
    }
    let current_thread = threads.get(&from).unwrap() as *const Thread;
    let next_thread = threads.get(&to).unwrap() as *const Thread;

    unsafe { switch_to_thread(ACTIVE_THREAD_ID.as_mut_ptr(), current_thread, next_thread) };

    interrupts::enable();
}
