use core::{
    cmp,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{borrow::ToOwned, boxed::Box, string::String, vec};
use log::trace;
use x86_64::{PhysAddr, VirtAddr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ThreadId(usize);

impl ThreadId {
    pub fn as_usize(&self) -> usize {
        self.0
    }

    pub fn from_usize(id: usize) -> Self {
        Self(id)
    }

    fn new() -> Self {
        static NEXT_THREAD_ID: AtomicUsize = AtomicUsize::new(0);
        ThreadId(NEXT_THREAD_ID.fetch_add(1, Ordering::SeqCst))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadState {
    Starting,
    Running,
    ReadyToRun,
    Blocked(BlockReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockReason {
    // How long to sleep
    Sleep(u64),
    Other,
}

#[derive(Debug)]
#[repr(C)]
pub struct Thread {
    id: ThreadId,
    stack_top: VirtAddr,
    page_table: PhysAddr,

    // Items below should not be accessed from assembly
    state: ThreadState,
    name: String,
    stack: Option<Stack>,
    time: u64,
}

impl Thread {
    pub fn new(stack_top: VirtAddr, page_table: PhysAddr, name: String, stack: Stack) -> Self {
        Self {
            id: ThreadId::new(),
            stack_top,
            page_table,
            state: ThreadState::Starting,
            name,
            stack: Some(stack),
            time: 0,
        }
    }

    /// Creates a thread descriptor for the root kernel thread
    ///
    /// This thread is special because we know it is the first one, it is
    /// already executing, it doesn't need an initial stack pointer (it gets
    /// filled in the first context switch), and the stack won't be deallocated
    /// if the thread ever gets deleted (which it won't)
    pub fn bootstrap(page_table: PhysAddr) -> Self {
        Self {
            id: ThreadId::new(),
            stack_top: VirtAddr::zero(),
            page_table,
            state: ThreadState::Running,
            name: "kernel".to_owned(),
            stack: None,
            time: 0,
        }
    }

    /// Get the thread's id.
    #[must_use]
    pub fn id(&self) -> ThreadId {
        self.id
    }

    /// Get a reference to the thread's state.
    #[must_use]
    pub fn state(&self) -> &ThreadState {
        &self.state
    }

    /// Set the thread's state.
    pub fn set_state(&mut self, state: ThreadState) {
        self.state = state;
    }

    /// Get a reference to the thread's name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn to_view(&self) -> ThreadView {
        ThreadView {
            id: self.id,
            stack_top: self.stack_top,
            page_table: self.page_table,
            state: self.state.clone(),
            name: self.name.clone(),
            stack_size: self.stack.as_ref().map(|stack| stack.buffer.len()),
            stack_bottom: self
                .stack
                .as_ref()
                .map(|stack| self.stack_top + stack.buffer.len()),
            time: self.time,
        }
    }

    /// Add to the thread's time in nanoseconds.
    pub fn add_time(&mut self, value: u64) {
        self.time += value;
    }

    /// Get a reference to the thread's stack.
    #[must_use]
    pub fn stack(&self) -> Option<&Stack> {
        self.stack.as_ref()
    }

    /// Get the thread's stack top.
    #[must_use]
    pub fn stack_top(&self) -> VirtAddr {
        self.stack_top
    }
}

impl PartialOrd for Thread {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl Ord for Thread {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialEq for Thread {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Thread {}

#[derive(Debug)]
#[repr(C)]
pub struct Stack {
    buffer: Box<[u64]>,
}

impl Stack {
    pub fn new(stack_size: usize, entry: fn() -> !) -> Self {
        // Create stack for the new thread
        let mut stack = vec![0u64; stack_size].into_boxed_slice();
        trace!(
            "allocating stack from {:p} to {:p}",
            &stack[0],
            &stack[stack_size - 1]
        );

        // Initialise stack in the reverse order registers get popped off it
        // Should contain an interrupt stack frame before the general registers
        stack[stack_size - 1] = 0x10; // return ss
        stack[stack_size - 2] = &stack[stack_size - 1] as *const u64 as u64; // return rsp
        stack[stack_size - 3] = 0x202; // return rflags (reserved flag set)
        stack[stack_size - 4] = 0x8; // return cs
        stack[stack_size - 5] = entry as *const () as u64; // return rip
        stack[stack_size - 12] = &stack[stack_size - 1] as *const u64 as u64; // rbp, should be same as return rsp

        Self { buffer: stack }
    }

    pub fn initial_stack_pointer(&self) -> VirtAddr {
        // Pointer to where the stack pointer needs to be so the iret will pop the interrupt stack frame
        let stack_pointer = &self.buffer[self.buffer.len() - 20] as *const u64;

        VirtAddr::new(stack_pointer as u64)
    }
}

/// A view into a thread, that cannot be scheduled itself
#[derive(Debug, Clone)]
pub struct ThreadView {
    id: ThreadId,
    stack_top: VirtAddr,
    page_table: PhysAddr,
    state: ThreadState,
    name: String,
    stack_size: Option<usize>,
    stack_bottom: Option<VirtAddr>,
    time: u64,
}

impl ThreadView {
    /// Get the thread view's id.
    #[must_use]
    pub fn id(&self) -> ThreadId {
        self.id
    }

    /// Get the thread view's stack top.
    #[must_use]
    pub fn stack_top(&self) -> VirtAddr {
        self.stack_top
    }

    /// Get the thread view's page table.
    #[must_use]
    pub fn page_table(&self) -> PhysAddr {
        self.page_table
    }

    /// Get a reference to the thread view's state.
    #[must_use]
    pub fn state(&self) -> &ThreadState {
        &self.state
    }

    /// Get a reference to the thread view's name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Get the thread view's stack size.
    #[must_use]
    pub fn stack_size(&self) -> Option<usize> {
        self.stack_size
    }

    /// Get the thread view's stack bottom.
    #[must_use]
    pub fn stack_bottom(&self) -> Option<VirtAddr> {
        self.stack_bottom
    }

    /// Get the thread view's time.
    #[must_use]
    pub fn time(&self) -> u64 {
        self.time
    }
}
