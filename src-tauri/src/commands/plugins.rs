//! Plugins: executables beside the workspace that act on a release or a work,
//! and what the window keeps for each of them.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

use crate::actions;
use crate::error::{Error, Result};
use crate::plugin::{self, Progress, manifest::Plugin, manifest::Target};
use crate::state::AppState;

/// The event a running plugin's progress reaches the window by.
pub const PROGRESS_EVENT: &str = "plugin-progress";

/// The runs going on now, by the id the window gave each, with the flag that
/// stops one. Memory only: a run does not outlive the application - a window
/// closed while one goes hides instead (v0.107), and the run goes on.
static RUNNING: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// How many plugin commands are going, for the count of what keeps the
/// application busy ([`super::activity`]).
pub fn running_count() -> usize {
    RUNNING.lock().map_or(0, |running| running.len())
}

fn plugin_directory(state: &AppState) -> std::path::PathBuf {
    state
        .path()
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default()
}

/// Everything installed under the plugin naming convention, usable or not,
/// with what the window keeps for each. Off the main thread: every plugin found
/// is started to ask what it is.
#[tauri::command(async)]
pub fn list_plugins(state: State<'_, AppState>) -> Result<Vec<Plugin>> {
    let mut plugins = plugin::discover(&plugin_directory(&state));
    plugin::settings::fill_in(&state.conn(), &mut plugins)?;
    Ok(plugins)
}

/// Where a plugin is put to be found, made if it is not there yet: the person
/// asking is about to put one there.
#[tauri::command(async)]
pub fn plugins_folder(state: State<'_, AppState>) -> Result<String> {
    let folder = plugin_directory(&state).join("plugins");
    std::fs::create_dir_all(&folder)?;
    Ok(folder.display().to_string())
}

/// Turn a plugin on or off.
#[tauri::command]
pub fn set_plugin_enabled(
    state: State<'_, AppState>,
    executable: String,
    enabled: bool,
) -> Result<()> {
    plugin::settings::set_enabled(&state.conn(), &executable, enabled)
}

/// Set one setting of a plugin, or put it back to the plugin's default with
/// nothing. Off the main thread: the plugin is asked what its settings are.
#[tauri::command(async)]
pub fn set_plugin_setting(
    state: State<'_, AppState>,
    executable: String,
    key: String,
    value: Option<Value>,
) -> Result<()> {
    let found = plugin::find(&plugin_directory(&state), &executable)
        .ok_or_else(|| Error::not_found("plugin", executable.clone()))?;
    let manifest = found.manifest.as_ref().ok_or_else(|| {
        Error::refused("plugin.unusable")
            .param("plugin", executable.as_str())
            .param("reason", found.reason.clone().unwrap_or_default())
    })?;
    plugin::settings::set_value(&state.conn(), &executable, manifest, &key, value)
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
    /// The message holding the score it proposed, when it proposed one.
    pub proposed_score: Option<String>,
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
    let found = plugin::find(&plugin_directory(&state), &executable)
        .ok_or_else(|| Error::not_found("plugin", executable.clone()))?;
    if !found.usable {
        return Err(Error::refused("plugin.unusable")
            .param("plugin", executable.as_str())
            .param("reason", found.reason.unwrap_or_default()));
    }
    let kept = plugin::settings::kept(&state.conn(), &executable)?;
    if !kept.enabled {
        return Err(Error::refused("plugin.turnedOff").param("plugin", executable.as_str()));
    }
    let settings = found
        .manifest
        .as_ref()
        .map_or_else(serde_json::Map::new, |manifest| {
            manifest.values(&kept.settings)
        });
    let limit = found
        .manifest
        .as_ref()
        .and_then(|manifest| manifest.commands.iter().find(|one| one.key == command))
        .map_or(plugin::PLUGIN_TIMEOUT, plugin::manifest::Command::allowance);
    let heard_by = match found.manifest.as_ref() {
        Some(manifest) => format!("{} {}", found.executable, manifest.version),
        None => found.executable.clone(),
    };

    let subject = actions::plugin::subject(&state.conn(), target, &id)?;
    let cancel = Arc::new(AtomicBool::new(false));
    if let Ok(mut running) = RUNNING.lock() {
        running.insert(run.clone(), cancel.clone());
    }
    super::activity::announce(&app);
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
            settings,
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
    super::activity::announce(&app);
    let outcome = outcome?;

    actions::plugin::keep(&state.conn(), target, &id, &outcome.meta)?;
    let finished = if target.is_work() {
        actions::plugin::keep_files(&state.conn(), &id, &outcome.files)?
    } else {
        None
    };
    if target == Target::Cuts {
        actions::plugin::keep_heard(&state.conn(), &id, &heard_by, &outcome.suggestions)?;
    }
    // Only where a judge is offered: a score from a command that is not one has
    // no tab waiting for it, and no one who asked.
    let proposed_score = match outcome.score {
        Some(marks) if target == Target::Score => Some(actions::plugin::keep_score(
            &state.conn(),
            &id,
            &executable,
            marks,
        )?),
        _ => None,
    };
    Ok(PluginReply {
        message: outcome.message,
        files: outcome.files,
        finished,
        proposed_score,
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
