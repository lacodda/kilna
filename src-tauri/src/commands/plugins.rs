//! Plugins: executables beside the workspace that act on a release or a work.

use tauri::State;

use crate::actions;
use crate::error::{Error, Result};
use crate::plugin::{self, manifest::Plugin, manifest::Target};
use crate::state::AppState;

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

/// Run a plugin command against a release or a work, and keep whatever it
/// returns in that row's fields.
///
/// The workspace is let go while the plugin runs: it is someone else's
/// executable, and the window must not wait on it for every other read.
#[tauri::command(async)]
pub fn run_plugin(
    state: State<'_, AppState>,
    executable: String,
    command: String,
    target: Target,
    id: String,
) -> Result<Option<String>> {
    let found = plugin::discover(&plugin_directory(&state))
        .into_iter()
        .find(|candidate| candidate.executable == executable)
        .ok_or_else(|| Error::not_found("plugin", executable.clone()))?;
    if !found.usable {
        return Err(Error::refused("plugin.unusable")
            .param("plugin", executable.as_str())
            .param("reason", found.reason.unwrap_or_default()));
    }

    let subject = actions::plugin::subject(&state.conn(), target, &id)?;
    let outcome = plugin::invoke(
        std::path::Path::new(&found.path),
        &plugin::Invocation {
            command: &command,
            target,
            subject,
        },
    )?;
    actions::plugin::keep(&state.conn(), target, &id, &outcome.meta)?;
    Ok(outcome.message)
}
