use core::{
    arch::asm,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{borrow::ToOwned, boxed::Box, string::String, vec};
use arrayvec::ArrayVec;
use spin::Mutex;
use x86_64::{instructions::interrupts, registers::control::Cr3, PhysAddr, VirtAddr};

static NEXT_THREAD_ID: AtomicUsize = AtomicUsize::new(0);

enum ThreadState {
    Running,
    ReadyToRun,
    Blocked,
}

#[repr(C)]
pub struct Thread {
    id: usize,
    stack_top: VirtAddr,
    page_table: PhysAddr,
    name: String,
    state: ThreadState,
}

// Create a thread struct for the initial kernel thread that is running (id 0)
pub fn init() {
    let (page_table, _) = Cr3::read();
    let page_table = page_table.start_address();

    THREADS.lock().push(Thread {
        id: NEXT_THREAD_ID.fetch_add(1, Ordering::AcqRel),
        name: "kernel".to_owned(),
        stack_top: VirtAddr::new(0),
        page_table,
        state: ThreadState::Running,
    });
}

pub fn add_thread(entry: fn() -> !, stack_size: usize) {
    // Create stack for the new thread
    let stack = Box::leak(vec![0u64; stack_size].into_boxed_slice());

    // Initialise stack in the reverse order registers get popped off it
    stack[stack_size - 1] = entry as *const () as u64; // rip
    stack[stack_size - 2] = 0x0; // rbx
    stack[stack_size - 3] = 0x0; // r12
    stack[stack_size - 4] = 0x0; // r13
    stack[stack_size - 5] = 0x0; // r14
    stack[stack_size - 6] = 0x0; // r15

    // Pointer to where the stack pointer needs to be so the ret lines up with the entry point
    let stack_pointer = unsafe { (&mut stack[stack.len() - 1] as *mut u64).sub(5) };

    // Get address to kernel's page table
    let (page_table, _) = Cr3::read();
    let page_table = page_table.start_address();

    // Add a new thread to the list
    THREADS.lock().push(Thread {
        id: NEXT_THREAD_ID.fetch_add(1, Ordering::AcqRel),
        name: "kernel2".to_owned(),
        stack_top: VirtAddr::new(stack_pointer as u64),
        page_table,
        state: ThreadState::ReadyToRun,
    });
}

static THREADS: Mutex<ArrayVec<Thread, 100>> = Mutex::new(ArrayVec::new_const());
// TODO: what happens when a thread is removed from the arrayvec?
pub static ACTIVE_THREAD_INDEX: AtomicUsize = AtomicUsize::new(0);

/// # Safety
/// Interrupts must be disabled before calling this function
#[naked]
pub unsafe extern "C" fn switch_to_thread(
    current_index: *mut usize,
    from_tcb: *const Thread,
    to_tcb: *const Thread,
    to_index: usize,
) {
    asm!(
        "
        // store registers
        push rbx
        push r12
        push r13
        push r14
        push r15

        mov [rsi+8], rsp        // save old thead's stack pointer

        mov [rdi], rcx          // update CURRENT_THREAD_INDEX with new thread

        mov rsp, [rdx + 8]      // load new thread's stack pointer
        mov rax, [rdx + 16]     // load new thread's page table
        
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

pub fn switch(to: usize) {
    interrupts::disable();

    let from = ACTIVE_THREAD_INDEX.load(Ordering::SeqCst);

    // TODO: fix this force unlock
    let threads = unsafe {
        THREADS.force_unlock();
        THREADS.lock()
    };
    // TODO: fix dodgy hack that handles interrupts before init
    if to >= threads.len() {
        return;
    }
    let current_thread = &threads[from] as *const Thread;
    let next_thread = &threads[to] as *const Thread;

    unsafe {
        switch_to_thread(
            ACTIVE_THREAD_INDEX.as_mut_ptr(),
            current_thread,
            next_thread,
            to,
        )
    };

    interrupts::enable();
}
