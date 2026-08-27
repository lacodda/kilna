//! Tasks waiting for a slot.
//!
//! Three runs may be alive at once ([`PARALLEL_LIMIT`]), and until now asking
//! for a fourth was simply refused. That is the right answer for a person
//! typing into the panel — they are watching, and being told the machine is
//! busy is information they can act on. It is the wrong answer for a bulk
//! action: ticking ten works and being handed three runs and seven errors is
//! not "the limit protected you", it is the feature not working.
//!
//! So a bulk action queues instead. What waits here is what a task *is* — the
//! action and the work — never a run, because there is no run yet. When a slot
//! frees, the next entry is started with exactly the call a single task would
//! have made; nothing downstream can tell the difference, which is the point.
//!
//! **The queue is memory, not a table.** A queued task has produced nothing
//! yet: no chat, no prompt, no row anywhere. Closing the application drops it,
//! and that is honest — the alternative is a workspace that starts spawning
//! CLI processes the moment it opens, for work asked of a session that is
//! gone.
//!
//! [`PARALLEL_LIMIT`]: super::run::PARALLEL_LIMIT

use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};

/// A task that has not started yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub work_id: String,
    pub action: String,
}

impl Pending {
    /// The key this task will run under, the same one a card builds.
    ///
    /// Waiting and running share a key space on purpose: it is what lets the
    /// same button say "working" whether the task is third in line or already
    /// talking to the CLI.
    pub fn key(&self) -> String {
        super::task::key(&self.action, &self.work_id)
    }
}

/// Tasks waiting for a slot, oldest first.
#[derive(Default)]
pub struct Queue {
    waiting: Mutex<VecDeque<Pending>>,
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    fn waiting(&self) -> MutexGuard<'_, VecDeque<Pending>> {
        self.waiting
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Put `task` at the back, unless it is already waiting.
    ///
    /// Returns whether it was added. The duplicate check is by key, matching
    /// the one the registry does for live runs: asking twice for the same
    /// action on the same work is one request, whichever side of the limit it
    /// lands on.
    pub fn push(&self, task: Pending) -> bool {
        let mut waiting = self.waiting();
        let key = task.key();
        if waiting.iter().any(|held| held.key() == key) {
            return false;
        }
        waiting.push_back(task);
        true
    }

    /// Take the oldest waiting task, if there is one.
    pub fn pop(&self) -> Option<Pending> {
        self.waiting().pop_front()
    }

    /// Whether a task by this key is waiting.
    pub fn holds(&self, key: &str) -> bool {
        self.waiting().iter().any(|task| task.key() == key)
    }

    /// Keys of everything waiting, so a screen can show those buttons busy
    /// too.
    pub fn keys(&self) -> Vec<String> {
        self.waiting().iter().map(Pending::key).collect()
    }

    /// How many are waiting.
    pub fn len(&self) -> usize {
        self.waiting().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drop everything waiting and say how many there were.
    ///
    /// Used when a person cancels a batch: the runs already going are stopped
    /// by the registry, and this is the rest of that same gesture.
    pub fn clear(&self) -> usize {
        let mut waiting = self.waiting();
        let held = waiting.len();
        waiting.clear();
        held
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(action: &str, work: &str) -> Pending {
        Pending {
            work_id: work.into(),
            action: action.into(),
        }
    }

    #[test]
    fn tasks_come_out_in_the_order_they_went_in() {
        let queue = Queue::new();

        queue.push(task("critique", "w1"));
        queue.push(task("critique", "w2"));
        queue.push(task("critique", "w3"));

        assert_eq!(queue.pop(), Some(task("critique", "w1")));
        assert_eq!(queue.pop(), Some(task("critique", "w2")));
        assert_eq!(queue.pop(), Some(task("critique", "w3")));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn the_same_task_is_not_queued_twice() {
        let queue = Queue::new();

        assert!(queue.push(task("critique", "w1")));
        assert!(!queue.push(task("critique", "w1")));

        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn the_same_action_on_another_work_is_a_different_task() {
        let queue = Queue::new();

        assert!(queue.push(task("critique", "w1")));
        assert!(queue.push(task("critique", "w2")));

        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn another_action_on_the_same_work_is_a_different_task() {
        let queue = Queue::new();

        assert!(queue.push(task("critique", "w1")));
        assert!(queue.push(task("score", "w1")));

        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn a_waiting_task_is_findable_by_its_key() {
        let queue = Queue::new();
        let waiting = task("critique", "w1");

        queue.push(waiting.clone());

        assert!(queue.holds(&waiting.key()));
        assert!(!queue.holds(&task("critique", "w2").key()));
    }

    /// A waiting key has to look exactly like a running one, or a button
    /// cannot use the two lists together.
    #[test]
    fn a_queued_key_matches_the_one_a_card_builds() {
        assert_eq!(
            task("critique", "w1").key(),
            super::super::task::key("critique", "w1")
        );
    }

    #[test]
    fn taking_a_task_stops_it_being_held() {
        let queue = Queue::new();
        let waiting = task("critique", "w1");
        queue.push(waiting.clone());

        let taken = queue.pop().unwrap();

        assert_eq!(taken, waiting);
        assert!(!queue.holds(&waiting.key()));
    }

    #[test]
    fn clearing_drops_everything_and_says_how_much() {
        let queue = Queue::new();
        queue.push(task("critique", "w1"));
        queue.push(task("critique", "w2"));

        assert_eq!(queue.clear(), 2);
        assert!(queue.is_empty());
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn clearing_an_empty_queue_is_nothing_happening() {
        let queue = Queue::new();

        assert_eq!(queue.clear(), 0);
    }

    /// The queue must refuse what the registry is already running, and the
    /// only thing making that possible is that both name a task the same way.
    /// If these ever diverge, a batch would queue a second copy of every task
    /// already going.
    #[test]
    fn a_queued_task_is_refused_by_the_key_the_registry_uses() {
        let queue = Queue::new();
        let running = super::super::task::key("critique", "w1");

        // What `start_tasks` does with a work already running: it holds the
        // registry's key and asks the queue about it.
        assert!(!queue.holds(&running));
        queue.push(task("critique", "w1"));
        assert!(
            queue.holds(&running),
            "the queue answers to the registry's key, or nothing can be deduplicated across the two"
        );
    }

    #[test]
    fn keys_list_everything_waiting() {
        let queue = Queue::new();
        queue.push(task("critique", "w1"));
        queue.push(task("score", "w2"));

        let keys = queue.keys();

        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&super::super::task::key("critique", "w1")));
        assert!(keys.contains(&super::super::task::key("score", "w2")));
    }
}
