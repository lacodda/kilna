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

/// The event a batch's progress is reported under (v0.155).
pub const BATCH_EVENT: &str = "plugin-batch";

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

/// Where plugins are looked for: the application's directory, not a space's -
/// a plugin is installed on this machine, whatever space is open.
fn plugin_directory(state: &AppState) -> std::path::PathBuf {
    state.app_dir().to_path_buf()
}

/// Everything installed under the plugin naming convention, usable or not,
/// with what the window keeps for each. Off the main thread: every plugin found
/// is started to ask what it is.
#[tauri::command(async)]
pub fn list_plugins(state: State<'_, AppState>) -> Result<Vec<Plugin>> {
    let mut plugins = plugin::discover(&plugin_directory(&state));
    plugin::settings::fill_in(&state.conn(), &mut plugins)?;
    // A secret set on this machine shows as set, never as itself (v0.155).
    for one in &mut plugins {
        let kept = plugin::settings::secrets(state.app_dir(), &one.executable);
        let Some(manifest) = one.manifest.as_ref() else {
            continue;
        };
        let secret: Vec<String> = manifest
            .settings
            .iter()
            .filter(|setting| setting.secret)
            .map(|setting| setting.key.clone())
            .collect();
        for key in secret {
            if kept.contains_key(&key) {
                one.settings
                    .insert(key, Value::String(plugin::settings::SET_SECRET.into()));
            } else {
                one.settings.remove(&key);
            }
        }
    }
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
            .param(
                "reason",
                found.reason.clone().map(Value::from).unwrap_or_default(),
            )
    })?;
    // A secret is kept on this machine, outside the space (v0.155): what the
    // database held of it before is let go.
    if manifest
        .settings
        .iter()
        .any(|setting| setting.key == key && setting.secret)
    {
        if value.as_ref().and_then(Value::as_str) == Some(plugin::settings::SET_SECRET) {
            return Ok(());
        }
        plugin::settings::set_secret(state.app_dir(), &executable, &key, value)?;
        return plugin::settings::set_value(&state.conn(), &executable, manifest, &key, None);
    }
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
    /// Results to choose one from (v0.155): kept only when one is picked.
    pub candidates: Vec<plugin::PluginCandidate>,
    /// What the run spent of its service's allowance (v0.155).
    pub quota: Option<plugin::PluginQuota>,
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
            .param("reason", found.reason.map(Value::from).unwrap_or_default()));
    }
    let kept = plugin::settings::kept(&state.conn(), &executable)?;
    if !kept.enabled {
        return Err(Error::refused("plugin.turnedOff").param("plugin", executable.as_str()));
    }
    let mut settings = found
        .manifest
        .as_ref()
        .map_or_else(serde_json::Map::new, |manifest| {
            manifest.values(&kept.settings)
        });
    // The secrets kept on this machine join the settings the plugin is
    // handed, and only there (v0.155).
    settings.extend(plugin::settings::secrets(state.app_dir(), &executable));
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
        candidates: outcome.candidates,
        quota: outcome.quota,
    })
}

/// One of the results a command offered, picked (v0.155): its fields merged
/// into the work's and its files kept as the run's would have been.
#[tauri::command]
pub fn keep_plugin_candidate(
    state: State<'_, AppState>,
    work_id: String,
    candidate: plugin::PluginCandidate,
) -> Result<Option<String>> {
    let conn = state.conn();
    actions::plugin::keep(&conn, Target::Work, &work_id, &candidate.meta)?;
    actions::plugin::keep_files(&conn, &work_id, &candidate.files)
}

/// How far a batch is, as the window hears it (v0.155).
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct PluginBatchProgress {
    /// The batch, as the window named it.
    pub batch: String,
    pub done: u32,
    pub total: u32,
    pub failed: u32,
    /// The work just finished.
    pub title: Option<String>,
}

/// One work of a batch, as it ended.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct PluginBatchItem {
    pub id: String,
    pub title: String,
    pub ok: bool,
    /// The plugin's words, or why it failed.
    pub message: Option<String>,
    pub files: Vec<String>,
}

/// What a batch did, work by work.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct PluginBatchReport {
    pub items: Vec<PluginBatchItem>,
    /// Stopped before every work was run.
    pub cancelled: bool,
}

/// Run one plugin command over many works at once (v0.155, FR-21 21.3): a
/// render by a preset for every track of a month. Up to `parallelism` runs
/// at a time, one to four; what each returns is kept as a single run's is,
/// work by work, as it comes back; the batch reports its progress as
/// [`BATCH_EVENT`] under the `batch` id the window gave, and [`cancel_plugin`]
/// with that id stops what is running and what has not started.
#[tauri::command(async)]
#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
pub fn run_plugin_batch(
    app: AppHandle,
    state: State<'_, AppState>,
    executable: String,
    command: String,
    ids: Vec<String>,
    parallelism: u8,
    batch: String,
) -> Result<PluginBatchReport> {
    let found = plugin::find(&plugin_directory(&state), &executable)
        .ok_or_else(|| Error::not_found("plugin", executable.clone()))?;
    if !found.usable {
        return Err(Error::refused("plugin.unusable")
            .param("plugin", executable.as_str())
            .param("reason", found.reason.map(Value::from).unwrap_or_default()));
    }
    let kept = plugin::settings::kept(&state.conn(), &executable)?;
    if !kept.enabled {
        return Err(Error::refused("plugin.turnedOff").param("plugin", executable.as_str()));
    }
    let mut settings = found
        .manifest
        .as_ref()
        .map_or_else(serde_json::Map::new, |manifest| {
            manifest.values(&kept.settings)
        });
    settings.extend(plugin::settings::secrets(state.app_dir(), &executable));
    let limit = found
        .manifest
        .as_ref()
        .and_then(|manifest| manifest.commands.iter().find(|one| one.key == command))
        .map_or(plugin::PLUGIN_TIMEOUT, plugin::manifest::Command::allowance);

    // Every subject read first, on this thread: the runs touch no database.
    let mut jobs = std::collections::VecDeque::new();
    for id in &ids {
        let title = crate::work::get(&state.conn(), id)?
            .map(|work| work.title)
            .unwrap_or_default();
        let subject = actions::plugin::subject(&state.conn(), Target::Work, id)?;
        jobs.push_back((id.clone(), title, subject));
    }
    let total = u32::try_from(jobs.len()).unwrap_or(u32::MAX);
    let queue = Mutex::new(jobs);
    let cancel = Arc::new(AtomicBool::new(false));
    if let Ok(mut running) = RUNNING.lock() {
        running.insert(batch.clone(), cancel.clone());
    }
    super::activity::announce(&app);

    let path = std::path::PathBuf::from(&found.path);
    let mut items = Vec::new();
    let (mut done, mut failed) = (0u32, 0u32);
    let (sender, results) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        for _ in 0..usize::from(parallelism.clamp(1, 4)) {
            let sender = sender.clone();
            let (queue, cancel, path, settings, command) =
                (&queue, &cancel, &path, &settings, &command);
            scope.spawn(move || {
                loop {
                    if cancel.load(Ordering::SeqCst) {
                        break;
                    }
                    let next = queue.lock().ok().and_then(|mut left| left.pop_front());
                    let Some((id, title, subject)) = next else {
                        break;
                    };
                    let outcome = plugin::invoke(
                        path,
                        &plugin::Invocation {
                            command,
                            target: Target::Work,
                            subject,
                            settings: settings.clone(),
                        },
                        plugin::Run {
                            limit,
                            cancel,
                            on_progress: &mut |_| {},
                        },
                    );
                    if sender.send((id, title, outcome)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);
        for (id, title, outcome) in results {
            done += 1;
            let kept = outcome.and_then(|outcome| {
                actions::plugin::keep(&state.conn(), Target::Work, &id, &outcome.meta)?;
                actions::plugin::keep_files(&state.conn(), &id, &outcome.files)?;
                Ok(outcome)
            });
            match kept {
                Ok(outcome) => items.push(PluginBatchItem {
                    id,
                    title: title.clone(),
                    ok: true,
                    message: outcome.message,
                    files: outcome.files,
                }),
                Err(cause) => {
                    failed += 1;
                    items.push(PluginBatchItem {
                        id,
                        title: title.clone(),
                        ok: false,
                        message: Some(cause.to_string()),
                        files: Vec::new(),
                    });
                }
            }
            let _ = app.emit(
                BATCH_EVENT,
                PluginBatchProgress {
                    batch: batch.clone(),
                    done,
                    total,
                    failed,
                    title: Some(title),
                },
            );
        }
    });
    let cancelled = cancel.load(Ordering::SeqCst) && done < total;
    if let Ok(mut running) = RUNNING.lock() {
        running.remove(&batch);
    }
    super::activity::announce(&app);
    Ok(PluginBatchReport { items, cancelled })
}

/// What a work's simple video is built from: its picture and its track
/// (v0.149) - what the Build tab previews and the renderer renders.
#[tauri::command]
pub fn build_sources(
    state: State<'_, AppState>,
    work_id: String,
) -> Result<crate::build::BuildSources> {
    crate::build::sources(&state.conn(), &work_id)
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
