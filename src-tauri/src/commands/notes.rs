//! Notes: ideas, lore, references - and the links a body carries.

use tauri::State;

use super::active;
use crate::actions;
use crate::error::Result;
use crate::note::{self, NewNote, Note, NoteFilter, NotePatch};
use crate::state::AppState;
use crate::trash::Entity;
use crate::work::{self, version};

#[tauri::command]
pub fn list_notes(state: State<'_, AppState>, filter: Option<NoteFilter>) -> Result<Vec<Note>> {
    let conn = state.conn();
    note::list(&conn, &active(&conn)?, &filter.unwrap_or_default())
}

#[tauri::command]
pub fn create_note(state: State<'_, AppState>, note: NewNote) -> Result<Note> {
    actions::note::create(&state.conn(), note)
}

#[tauri::command]
pub fn update_note(state: State<'_, AppState>, id: String, mut patch: NotePatch) -> Result<Note> {
    // A card's description is written through `describe_card`, which knows
    // the facts it answers to; written here it would carry no fingerprint and
    // read as stale forever.
    patch.prompt = None;
    patch.prompt_basis = None;
    actions::note::update(&state.conn(), &id, patch)
}

#[tauri::command]
pub fn delete_note(state: State<'_, AppState>, id: String) -> Result<String> {
    actions::trash::discard(&state.conn(), Entity::Note, &id)
}

/// Turn a note into a work, its body the work's first version.
#[tauri::command]
pub fn promote_note(
    state: State<'_, AppState>,
    id: String,
    promotion: note::Promotion,
) -> Result<note::Promoted> {
    actions::note::promote(&state.conn(), &id, promotion)
}

/// Tags in use on notes, most used first.
#[tauri::command]
pub fn list_tags(state: State<'_, AppState>) -> Result<Vec<(String, i64)>> {
    let conn = state.conn();
    note::tags(&conn, &active(&conn)?)
}

/// What a `[[work:id]]` or `[[version:id]]` link points at.
///
/// One row per link that resolves: the title to draw, and for a version the
/// work whose card it opens. A link to something deleted is simply absent, and
/// the window draws it as the text it was — a dead link is worse than prose.
#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedLink {
    pub id: String,
    /// `work` or `version`, echoed so the window can match the row to the link
    /// without assuming an order.
    pub target: String,
    pub title: String,
    /// The work whose card opens. The work's own id for a work.
    pub work_id: String,
}

/// Resolve the links in a body, in one round trip rather than one per link:
/// a note with twenty references would otherwise be twenty queries drawn one
/// frame apart.
#[tauri::command]
pub fn resolve_links(
    state: State<'_, AppState>,
    works: Vec<String>,
    versions: Vec<String>,
) -> Result<Vec<ResolvedLink>> {
    let conn = state.conn();
    let mut out = Vec::with_capacity(works.len() + versions.len());

    for id in works {
        let Some(work) = work::get(&conn, &id)? else {
            continue;
        };
        out.push(ResolvedLink {
            id,
            target: "work".into(),
            title: work.title,
            work_id: work.id,
        });
    }

    for id in versions {
        let Some(found) = version::get(&conn, &id)? else {
            continue;
        };
        // The label when it has one, else the role and revision: "lyrics 3"
        // says more in a sentence than a uuid ever will.
        let title = found
            .label
            .clone()
            .filter(|label| !label.trim().is_empty())
            .unwrap_or_else(|| format!("{} {}", found.role, found.revision));
        out.push(ResolvedLink {
            id,
            target: "version".into(),
            title,
            work_id: found.work_id,
        });
    }

    Ok(out)
}
