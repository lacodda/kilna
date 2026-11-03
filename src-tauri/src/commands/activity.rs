//! How busy the application is (v0.107).
//!
//! What closing the window does is decided from [`current`]
//! ([`crate::tray::close_requested`]): closed while something is going, the
//! window hides behind the tray and the work carries on; closed with nothing
//! going, it closes. Everything after that - the tray's badge, the taskbar's,
//! the notice when the last task ends - follows the
//! [`crate::activity::EVENT`] this module sends whenever the count moves.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::activity::{self, Activity};
use crate::state::AppState;

/// The count now: runs and plugin commands going, tasks waiting.
pub fn current(state: &AppState) -> Activity {
    Activity::of(
        state.runs().active(),
        super::plugins::running_count(),
        state.queue().len(),
    )
}

/// Tell the window the count. A window that is not listening misses nothing
/// it cannot ask for again with [`activity`].
pub fn announce(app: &AppHandle) {
    let state = app.state::<AppState>();
    let _ = app.emit(activity::EVENT, current(state.inner()));
}

/// Have the registries say when they change. Called once, at setup, when
/// there is a window to tell.
pub fn watch(app: &AppHandle) {
    let state = app.state::<AppState>();
    let told = app.clone();
    let hook: activity::Hook = Arc::new(move || announce(&told));
    state.runs().watch(Arc::clone(&hook));
    state.queue().watch(hook);
}

#[tauri::command]
pub fn activity(state: State<'_, AppState>) -> Activity {
    current(state.inner())
}
