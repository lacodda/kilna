//! The registries of style packs, and installing from them (v0.131, FR-06).

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::error::Result;
use crate::packs::{self, Installed, ListedPack, OpenedPack, Source};
use crate::state::AppState;

/// The snapshot shipped beside the application: in its resources once
/// installed, in the source tree in a development run.
fn builtin(app: &AppHandle) -> PathBuf {
    app.path()
        .resource_dir()
        .map(|dir| dir.join(packs::BUILTIN_DIR))
        .ok()
        .filter(|dir| dir.join("index.json").is_file())
        .unwrap_or_else(packs::snapshot_in_tree)
}

/// What the catalogue of packs shows.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct PackCatalogue {
    pub packs: Vec<ListedPack>,
    /// The sources that could not be read, each said.
    pub failed: Vec<String>,
    pub sources: Vec<Source>,
    pub offline: bool,
}

#[tauri::command(async)]
pub fn style_packs(app: AppHandle, state: State<'_, AppState>) -> Result<PackCatalogue> {
    let (sources, offline) = packs::sources(&state.app())?;
    let (listed, failed) = packs::catalogue(&state.conn(), &sources, offline, &builtin(&app));
    Ok(PackCatalogue {
        packs: listed,
        failed,
        sources,
        offline,
    })
}

#[tauri::command(async)]
pub fn style_pack(
    app: AppHandle,
    state: State<'_, AppState>,
    source: Source,
    pack: String,
) -> Result<OpenedPack> {
    packs::open(&state.conn(), &source, &builtin(&app), &pack)
}

/// Install a pack into the open space: whole, or only the bricks named.
#[tauri::command(async)]
pub fn install_style_pack(
    app: AppHandle,
    state: State<'_, AppState>,
    source: Source,
    pack: String,
    only: Option<Vec<String>>,
) -> Result<Installed> {
    packs::install_from(
        &state.conn(),
        &source,
        &builtin(&app),
        &pack,
        only.as_deref(),
    )
}

#[tauri::command]
pub fn add_style_source(state: State<'_, AppState>, source: Source) -> Result<()> {
    packs::add_source(&state.app(), &source)
}

#[tauri::command]
pub fn remove_style_source(state: State<'_, AppState>, source: Source) -> Result<()> {
    packs::remove_source(&state.app(), &source)
}

#[tauri::command]
pub fn set_packs_offline(state: State<'_, AppState>, offline: bool) -> Result<()> {
    packs::set_offline(&state.app(), offline)
}
