use core::mem;

use alloc::{
    borrow::ToOwned,
    collections::{BTreeMap, BTreeSet, VecDeque},
    vec::Vec,
};
use spin::Mutex;
use x86_64::{instructions::interrupts, registers::control::Cr3};

use crate::hpet;

use super::{
    thread::{BlockReason, Stack, ThreadId, ThreadView},
    Thread, ThreadState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Deadline(u64);

impl Deadline {
    pub fn absolute(deadline: u64) -> Self {
        Self(deadline)
    }

    pub fn relative(deadline: u64) -> Self {
        Self(hpet::nanoseconds().checked_add(deadline).unwrap())
    }
}

static SCHEDULER: Mutex<Option<Scheduler>> = Mutex::new(None);

pub struct Scheduler {
    /// Whether the scheduler is currently active
    active: bool,

    /// A mapping between threads and their control blocks
    threads: BTreeMap<ThreadId, Thread>,

    /// Thread assigned to be the idle thread
    idle_thread_id: Option<ThreadId>,

    /// Thread that is currently being executed
    current_thread_id: ThreadId,

    /// Queue of threads that are currently paused, and should be scheduled soon
    paused_threads: VecDeque<ThreadId>,

    /// Threads which are blocked on something happening (I/O, delay, etc.)
    //blocked_threads: BTreeSet<ThreadId>,

    /// Map of threads which are currently sleeping, ordered by deadline
    sleeping_threads: BTreeMap<Deadline, BTreeSet<ThreadId>>,

    /// Time of the last accounting update in nanoseconds since boot
    last_accounting_update: u64,
}

impl Scheduler {
    /// Initialise a new instance of the scheduler.
    fn new() -> Self {
        // Create kernel bootstrap thread
        let (page_table, _) = Cr3::read();
        let page_table = page_table.start_address();
        let tcb = Thread::bootstrap(page_table);
        let root_id = tcb.id();

        // Create thread map
        let mut threads = BTreeMap::new();
        threads.insert(root_id, tcb);

        Self {
            active: false,
            threads,
            idle_thread_id: None,
            current_thread_id: root_id,
            paused_threads: VecDeque::new(),
            //blocked_threads: BTreeSet::new(),
            sleeping_threads: BTreeMap::new(),
            last_accounting_update: 0,
        }
    }

    /// Add a new thread to the paused list.
    pub fn add_paused_thread(&mut self, name: &str, entrypoint: fn() -> !, stack_size: usize) {
        // Get address to kernel's page table
        let (page_table, _) = Cr3::read();
        let page_table = page_table.start_address();

        // Create and initialise a new stack for this thread
        let stack = Stack::new(stack_size, entrypoint);

        // Add a new thread to the list
        let tcb = Thread::new(
            stack.initial_stack_pointer(),
            page_table,
            name.to_owned(),
            stack,
        );
        let thread_id = tcb.id();

        // Add thread to the scheduler data structures
        self.threads.insert(thread_id, tcb);
        self.paused_threads.push_back(thread_id);
    }

    pub fn set_idle_thread(&mut self, entrypoint: fn() -> !, stack_size: usize) {
        // Get address to kernel's page table
        let (page_table, _) = Cr3::read();
        let page_table = page_table.start_address();

        // Create and initialise a new stack for this thread
        let stack = Stack::new(stack_size, entrypoint);

        // Add a new thread to the list
        let mut thread = Thread::new(
            stack.initial_stack_pointer(),
            page_table,
            "idle".to_owned(),
            stack,
        );
        thread.set_state(ThreadState::ReadyToRun);

        self.idle_thread_id = Some(thread.id());
        self.threads.insert(thread.id(), thread);
    }

    /// Get a view into the current threads in the scheduler.
    pub fn to_view(&self) -> Vec<ThreadView> {
        self.threads.values().map(|tcb| tcb.to_view()).collect()
    }

    /// Remove the next thread from the paused thread list.
    pub fn next_thread(&mut self) -> Option<ThreadId> {
        self.paused_threads.pop_front()
    }

    /// Get whether the idle thread is currently executing.
    pub fn is_idle_thread_active(&self) -> bool {
        Some(self.current_thread_id) == self.idle_thread_id
    }

    /// Put a thread to sleep
    pub fn sleep_thread(&mut self, id: ThreadId, deadline: Deadline) {
        self.threads
            .get_mut(&id)
            .unwrap()
            .set_state(ThreadState::Blocked(BlockReason::Other));
        let entry = self.sleeping_threads.entry(deadline);
        entry.or_default().insert(id);
    }

    /// Get the current and next thread blocks as pointers.
    ///
    /// # Safety
    /// The pointers are only valid for the duration of the scheduler lock.
    pub unsafe fn thread_pointers(
        &mut self,
        from_id: ThreadId,
        next_id: ThreadId,
    ) -> (*mut Thread, *const Thread) {
        let current_thread = self.threads.get_mut(&from_id).unwrap() as *mut Thread;
        let next_thread = self.threads.get(&next_id).unwrap() as *const Thread;

        (current_thread, next_thread)
    }

    /// Schedule a new thread.
    ///
    /// Returns either Some((previous thread id, next thread id)), or None if
    /// no context switch should occur.
    pub fn schedule(&mut self) -> Option<(ThreadId, ThreadId)> {
        if !self.active() {
            return None;
        }

        // Update time accounting
        self.update_time_used();

        // Check on sleeping threads that have met their deadline
        while let Some((deadline, threads)) = self.sleeping_threads.pop_first() {
            if deadline <= Deadline(hpet::nanoseconds()) {
                for thread_id in threads {
                    self.threads
                        .get_mut(&thread_id)
                        .unwrap()
                        .set_state(ThreadState::ReadyToRun);

                    self.paused_threads.push_back(thread_id);
                }
            } else {
                self.sleeping_threads.insert(deadline, threads);

                // No other deadlines could have been met, so stop trying
                break;
            }
        }

        // Get next thread with round robin scheduler
        let mut next_thread_id = self.next_thread();

        // Check if the current thread still wants to run
        if next_thread_id.is_none() && !self.is_idle_thread_active() {
            // Thread currently executing is the only available thread, but only if it still wants to run
            if let Some(tcb) = self.threads.get(&self.current_thread_id) {
                if tcb.state() == &ThreadState::Running {
                    next_thread_id = Some(self.current_thread_id);
                }
            }
        }

        // If there really is no threads wanting to run, switch to idle thread
        if next_thread_id.is_none() {
            next_thread_id = self.idle_thread_id;
        }

        if let Some(next_id) = next_thread_id {
            // Update current thread id with next thread id
            let prev_thread_id = mem::replace(&mut self.current_thread_id, next_id);

            // If the next thread is the same as the previous thread, don't context switch
            if next_thread_id == Some(prev_thread_id) {
                return None;
            }

            // Update previous thread's state
            let previous_thread = self.threads.get_mut(&prev_thread_id).unwrap();
            if previous_thread.state() == &ThreadState::Running {
                previous_thread.set_state(ThreadState::ReadyToRun);
                self.paused_threads.push_back(prev_thread_id);
            }

            // Update next thread's state
            let next_thread = self.threads.get_mut(&next_id).unwrap();
            next_thread.set_state(ThreadState::Running);

            Some((prev_thread_id, next_id))
        } else {
            // There are no threads to be scheduled, so keep on executing the
            // idle thread
            None
        }
    }

    /// Update the time accounting on the current thread.
    fn update_time_used(&mut self) {
        // Calculate elapsed time
        let current_time = hpet::nanoseconds();
        let elapsed = current_time - self.last_accounting_update;
        self.last_accounting_update = current_time;

        // Update thread time accounting
        self.threads
            .get_mut(&self.current_thread_id)
            .unwrap()
            .add_time(elapsed);
    }

    /// Set the scheduler's active state.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        self.last_accounting_update = hpet::nanoseconds();
    }

    /// Get the scheduler's active state.
    #[must_use]
    pub fn active(&self) -> bool {
        self.active
    }

    /// Get the scheduler's current thread id.
    #[must_use]
    pub fn current_thread_id(&self) -> ThreadId {
        self.current_thread_id
    }
}

/// Call a closure under the context of the scheduler lock.
pub fn with_scheduler<F, T>(f: F) -> T
where
    F: FnOnce(&mut Scheduler) -> T,
{
    interrupts::disable();
    let result = f(SCHEDULER.lock().get_or_insert_with(Scheduler::new));
    interrupts::enable();
    result
}

/// Call a closure under the context of the scheduler lock, without disabling
/// interrupts.
///
/// # Safety
/// This function must only be called in the context of an interrupt, as it
/// gets the scheduler lock without disabling interrupts.
pub unsafe fn with_scheduler_from_irq<F, T>(f: F) -> T
where
    F: FnOnce(&mut Scheduler) -> T,
{
    let result = f(SCHEDULER
        .try_lock()
        .unwrap()
        .get_or_insert_with(Scheduler::new));
    result
}
