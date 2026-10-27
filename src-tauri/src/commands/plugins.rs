//! Plugins: executables beside the workspace that act on a release or a work.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::actions;
use crate::error::{Error, Result};
use crate::plugin::{self, Progress, manifest::Plugin, manifest::Target};
use crate::state::AppState;

/// The event a running plugin's progress reaches the window by.
pub const PROGRESS_EVENT: &str = "plugin-progress";

/// The runs going on now, by the id the window gave each, with the flag that
/// stops one. Memory only: a run does not outlive the window that started it.
static RUNNING: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn plugin_directory(state: &AppState) -> std::path::PathBuf {
    state
        .path()
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default()
}

/// Everything installed under the plugin naming convention, usable or not.
#[tauri::command]
pub fn list_plugins(state: State<'_, AppState>) -> Vec<Plugin> {
    plugin::discover(&plugin_directory(&state))
}

/// One line of a run's progress, as the window hears it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct PluginProgress {
    /// The run, as the window named it.
    pub run: String,
    #[serde(flatten)]
    pub progress: Progress,
}

/// What a run ended with, for the window to say.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct PluginReply {
    /// The plugin's own words, when it said any.
    pub message: Option<String>,
    /// The files it says it made.
    pub files: Vec<String>,
    /// The one of them that is now the work's finished file - found by the
    /// kind's rule or chosen for it - when one is.
    pub finished: Option<String>,
}

/// Run a plugin command against a release or a work, and keep whatever it
/// returns in that row's fields.
///
/// The workspace is let go while the plugin runs: it is someone else's
/// executable, and the window must not wait on it for every other read. The
/// run takes as long as its command's allowance says, reports its progress as
/// [`PROGRESS_EVENT`] under the `run` id the window gave, and stops at once
/// when [`cancel_plugin`] names that id.
#[tauri::command(async)]
pub fn run_plugin(
    app: AppHandle,
    state: State<'_, AppState>,
    executable: String,
    command: String,
    target: Target,
    id: String,
    run: String,
) -> Result<PluginReply> {
    let found = plugin::discover(&plugin_directory(&state))
        .into_iter()
        .find(|candidate| candidate.executable == executable)
        .ok_or_else(|| Error::not_found("plugin", executable.clone()))?;
    if !found.usable {
        return Err(Error::refused("plugin.unusable")
            .param("plugin", executable.as_str())
            .param("reason", found.reason.unwrap_or_default()));
    }
    let limit = found
        .manifest
        .as_ref()
        .and_then(|manifest| manifest.commands.iter().find(|one| one.key == command))
        .map_or(plugin::PLUGIN_TIMEOUT, plugin::manifest::Command::allowance);

    let subject = actions::plugin::subject(&state.conn(), target, &id)?;
    let cancel = Arc::new(AtomicBool::new(false));
    if let Ok(mut running) = RUNNING.lock() {
        running.insert(run.clone(), cancel.clone());
    }
    let mut on_progress = |progress: Progress| {
        let _ = app.emit(
            PROGRESS_EVENT,
            PluginProgress {
                run: run.clone(),
                progress,
            },
        );
    };
    let outcome = plugin::invoke(
        std::path::Path::new(&found.path),
        &plugin::Invocation {
            command: &command,
            target,
            subject,
        },
        plugin::Run {
            limit,
            cancel: &cancel,
            on_progress: &mut on_progress,
        },
    );
    if let Ok(mut running) = RUNNING.lock() {
        running.remove(&run);
    }
    let outcome = outcome?;

    actions::plugin::keep(&state.conn(), target, &id, &outcome.meta)?;
    let finished = if target.is_work() {
        actions::plugin::keep_files(&state.conn(), &id, &outcome.files)?
    } else {
        None
    };
    Ok(PluginReply {
        message: outcome.message,
        files: outcome.files,
        finished,
    })
}

/// Stop a run the window started. A run already over is nothing to stop.
#[tauri::command]
pub fn cancel_plugin(run: String) {
    if let Ok(running) = RUNNING.lock()
        && let Some(cancel) = running.get(&run)
    {
        cancel.store(true, Ordering::SeqCst);
    }
}
