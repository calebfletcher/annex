use super::SwitchReason;
use crate::multitasking::thread::{Thread, ThreadId};
use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use core::{assert_matches::assert_matches, mem};
use x86_64::VirtAddr;

pub struct Scheduler {
    threads: BTreeMap<ThreadId, Thread>,
    idle_thread_id: Option<ThreadId>,
    current_thread_id: ThreadId,
    paused_threads: VecDeque<ThreadId>,
    blocked_threads: BTreeSet<ThreadId>,
    wakeups: BTreeSet<ThreadId>,
}

impl Scheduler {
    pub fn new() -> Self {
        let root_thread = Thread::create_root_thread();
        let root_id = root_thread.id();
        let mut threads = BTreeMap::new();
        assert_matches!(threads.insert(root_id, root_thread), None);
        Scheduler {
            threads,
            current_thread_id: root_id,
            paused_threads: VecDeque::new(),
            blocked_threads: BTreeSet::new(),
            wakeups: BTreeSet::new(),
            idle_thread_id: None,
        }
    }

    fn next_thread(&mut self) -> Option<ThreadId> {
        self.paused_threads.pop_front()
    }

    pub fn schedule(&mut self) -> Option<(VirtAddr, ThreadId)> {
        let mut next_thread_id = self.next_thread();
        if next_thread_id.is_none() && Some(self.current_thread_id) != self.idle_thread_id {
            next_thread_id = self.idle_thread_id
        }
        if let Some(next_id) = next_thread_id {
            let next_thread = self
                .threads
                .get_mut(&next_id)
                .expect("next thread does not exist");
            let next_stack_pointer = next_thread
                .stack_pointer()
                .take()
                .expect("paused thread has no stack pointer");
            let prev_thread_id = mem::replace(&mut self.current_thread_id, next_thread.id());
            Some((next_stack_pointer, prev_thread_id))
        } else {
            None
        }
    }

    pub(super) fn add_paused_thread(
        &mut self,
        paused_stack_pointer: VirtAddr,
        paused_thread_id: ThreadId,
        switch_reason: SwitchReason,
    ) {
        let paused_thread = self
            .threads
            .get_mut(&paused_thread_id)
            .expect("paused thread does not exist");
        assert_matches!(
            paused_thread.stack_pointer().replace(paused_stack_pointer),
            None
        );
        if Some(paused_thread_id) == self.idle_thread_id {
            return; // do nothing
        }
        match switch_reason {
            SwitchReason::Paused | SwitchReason::Yield => {
                self.paused_threads.push_back(paused_thread_id)
            }
            SwitchReason::Blocked => {
                self.blocked_threads.insert(paused_thread_id);
                self.check_for_wakeup(paused_thread_id);
            }
            SwitchReason::Exit => {
                let thread = self
                    .threads
                    .remove(&paused_thread_id)
                    .expect("thread not found");
                // TODO: free stack memory again
            }
        }
    }

    pub fn add_new_thread(&mut self, thread: Thread) {
        let thread_id = thread.id();
        assert_matches!(self.threads.insert(thread_id, thread), None);
        self.paused_threads.push_back(thread_id);
    }

    pub fn set_idle_thread(&mut self, thread: Thread) {
        let thread_id = thread.id();
        assert_matches!(self.threads.insert(thread_id, thread), None);
        assert_matches!(self.idle_thread_id.replace(thread_id), None);
    }

    pub fn current_thread_id(&self) -> ThreadId {
        self.current_thread_id
    }

    fn check_for_wakeup(&mut self, thread_id: ThreadId) {
        if self.wakeups.remove(&thread_id) {
            assert!(self.blocked_threads.remove(&thread_id));
            self.paused_threads.push_back(thread_id);
        }
    }
}
