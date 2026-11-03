//! What the application is busy with, for a window that may not be open
//! (v0.107).
//!
//! Until v0.107 closing the window ended everything it had started: the runs
//! were stopped at exit, the queue dropped, a plugin's cut killed with its
//! job object. Now a window closed while something is going hides instead,
//! and the work carries on behind the tray. What decides between the two - and
//! what the tray's badge and the taskbar's say - is one count, kept here: the
//! processes going and the tasks waiting for a slot.
//!
//! The registries that hold those - the runs, the queue, the plugins - each
//! say when they change through a [`Watcher`], and the window is told the new
//! count as one event. Asking each of them separately is how a badge, a close
//! button and a tray come to disagree about whether kilna is busy.

use std::sync::{Arc, OnceLock};

use serde::Serialize;

/// What to call when a registry changes. Set once, by the application, when
/// there is a window to tell; a registry in a test has nobody to tell.
pub type Hook = Arc<dyn Fn() + Send + Sync>;

/// A registry's line to whoever counts.
#[derive(Default)]
pub struct Watcher(OnceLock<Hook>);

impl Watcher {
    /// Say who to tell. A second call is ignored: there is one application.
    pub fn set(&self, hook: Hook) {
        let _ = self.0.set(hook);
    }

    /// Tell them. Called after the registry has let go of its own lock, so
    /// the hook may ask the registry how many it holds.
    pub fn changed(&self) {
        if let Some(hook) = self.0.get() {
            hook();
        }
    }
}

/// How busy the application is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    /// Processes going now: assistant runs and plugin commands.
    pub running: u32,
    /// Tasks waiting for a slot to free.
    pub queued: u32,
}

impl Activity {
    /// From the registries' counts, which are `usize` where they are kept.
    pub fn of(runs: usize, plugins: usize, queued: usize) -> Self {
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        Self {
            running: count(runs).saturating_add(count(plugins)),
            queued: count(queued),
        }
    }

    /// Whether closing the window would end nothing.
    pub fn is_idle(self) -> bool {
        self.running == 0 && self.queued == 0
    }
}

/// The event the window hears the count on, every time it changes.
pub const EVENT: &str = "app:activity";

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn plugins_and_runs_both_count_as_running() {
        let busy = Activity::of(2, 1, 4);
        assert_eq!(busy.running, 3);
        assert_eq!(busy.queued, 4);
        assert!(!busy.is_idle());
    }

    #[test]
    fn a_queue_alone_keeps_the_application_busy() {
        // The waiting tasks start when a slot frees; closing would drop them.
        assert!(!Activity::of(0, 0, 1).is_idle());
        assert!(Activity::of(0, 0, 0).is_idle());
    }

    #[test]
    fn a_watcher_with_nobody_to_tell_is_quiet() {
        Watcher::default().changed();
    }

    #[test]
    fn a_watcher_tells_the_one_it_was_given() {
        let told = Arc::new(AtomicUsize::new(0));
        let watcher = Watcher::default();
        let counter = Arc::clone(&told);
        watcher.set(Arc::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        }));
        // A second hook is not a second listener.
        watcher.set(Arc::new(|| panic!("there is one application")));

        watcher.changed();
        watcher.changed();
        assert_eq!(told.load(Ordering::SeqCst), 2);
    }
}
