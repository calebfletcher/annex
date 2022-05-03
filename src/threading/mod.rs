use core::{
    arch::asm,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{borrow::ToOwned, boxed::Box, string::String};
use arrayvec::ArrayVec;
use spin::Mutex;
use x86_64::{instructions::interrupts, registers::control::Cr3, PhysAddr, VirtAddr};

enum TaskState {
    Running,
    Blocked,
}

#[repr(C)]
pub struct Task {
    id: usize,
    stack_top: VirtAddr,
    page_table: PhysAddr,
    name: String,
    state: TaskState,
}

// Creates a task struct for the initial kernel task that is running (id 0)
pub fn init() {
    let (page_table, _) = Cr3::read();
    let page_table = page_table.start_address();
    TASKS.lock().push(Task {
        id: 4,
        name: "kernel1".to_owned(),
        stack_top: VirtAddr::new(0),
        page_table,
        state: TaskState::Running,
    });

    let stack = Box::leak(Box::new([0u64; 4096]));

    // Order from the register pops below
    stack[4095] = task2 as *const () as u64; // rip
    stack[4094] = 0x0; // rbx
    stack[4093] = 0x0; // r12
    stack[4092] = 0x0; // r13
    stack[4091] = 0x0; // r14
    stack[4090] = 0x0; // r15

    let stack_pointer = unsafe { (&mut stack[stack.len() - 1] as *mut u64).sub(5) };

    TASKS.lock().push(Task {
        id: 7,
        name: "kernel2".to_owned(),
        stack_top: VirtAddr::new(stack_pointer as u64),
        page_table,
        state: TaskState::Running,
    });
}

pub extern "C" fn task2() -> ! {
    interrupts::enable();
    loop {
        unsafe {
            asm! {
                "
                mov dx, 0x3F8
                mov al, 0x42
                out dx, al
            ",
            }
        };
    }
}

static TASKS: Mutex<ArrayVec<Task, 100>> = Mutex::new(ArrayVec::new_const());
// TODO: what happens when a task is removed from the arrayvec?
pub static CURRENT_TASK_INDEX: AtomicUsize = AtomicUsize::new(0);

/// # Safety
/// Interrupts must be disabled before calling this function
#[naked]
pub unsafe extern "C" fn switch_to_task(
    current_index: *mut usize,
    from_tcb: *const Task,
    to_tcb: *const Task,
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

        //mov rax, [rsi]          // id of old thread
        //mov rbx, [rdx]          // id of new thread

        mov [rsi+8], rsp        // save old thead's stack pointer

        mov [rdi], rcx          // update CURRENT_TASK_INDEX with new task

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
    let from = CURRENT_TASK_INDEX.load(Ordering::SeqCst);
    interrupts::disable();
    // serial_println!(
    //     "old current task {}",
    //     CURRENT_TASK_INDEX.load(Ordering::SeqCst)
    // );
    let tasks = unsafe {
        TASKS.force_unlock();
        TASKS.lock()
    };
    // TODO: fix dodgy hack that handles interrupts before init
    if to >= tasks.len() {
        return;
    }
    let current_task = &tasks[from] as *const Task;
    let next_task = &tasks[to] as *const Task;

    //serial_println!("task ids {} {}", tasks[from].id, tasks[to].id);
    unsafe { switch_to_task(CURRENT_TASK_INDEX.as_mut_ptr(), current_task, next_task, to) };
    // serial_println!(
    //     "new current task {}",
    //     CURRENT_TASK_INDEX.load(Ordering::SeqCst)
    // );
    interrupts::enable();
}
