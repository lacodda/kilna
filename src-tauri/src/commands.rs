use tauri::State;

use crate::assistant::{self, Chat, Message, NewChat, Transcript, cli, prompt};
use crate::collection::{self, Collection, CollectionPatch, NewCollection};
use crate::error::{Error, Result};
use crate::exchange::backup;
use crate::exchange::export::{self, ExportReport};
use crate::exchange::import::{self, ImportReport};
use crate::note::{self, NewNote, Note, NoteFilter, NotePatch};
use crate::plugin::{self, manifest::Plugin, manifest::Target};
use crate::profile::{self, Profile, Workspace};
use crate::release::{self, NewRelease, Release, ReleasePatch, ScheduledRelease, Scheduling};
use crate::score::{self, NewScore, Score, ScoredWork};
use crate::state::AppState;
use crate::work::version::{self, NewVersion, Version, VersionSummary};
use crate::work::{self, NewWork, Work, WorkFilter, WorkPatch};

/// Everything the status screen needs in one round trip.
#[tauri::command]
pub fn get_workspace(state: State<'_, AppState>) -> Result<Workspace> {
    let conn = state.conn();
    profile::workspace(&conn)
}

#[tauri::command]
pub fn list_profiles(state: State<'_, AppState>) -> Result<Vec<Profile>> {
    let conn = state.conn();
    profile::list(&conn)
}

#[tauri::command]
pub fn activate_profile(state: State<'_, AppState>, id: String) -> Result<()> {
    let mut conn = state.conn();
    profile::activate(&mut conn, &id)
}

/// Replace a profile's configuration.
///
/// Existing works keep their status and kind even when the vocabulary that
/// named them is edited away — an old value stays visible rather than being
/// rewritten, because the alternative is silently changing what a work is.
#[tauri::command]
pub fn update_profile_config(
    state: State<'_, AppState>,
    id: String,
    config: profile::config::ProfileConfig,
) -> Result<Profile> {
    let conn = state.conn();
    profile::update_config(&conn, &id, &config)
}

/// The id of the active profile, or an error the frontend can show.
///
/// Every work and note command is scoped to it: the frontend never has to carry
/// the profile id around, and a request cannot land in the wrong profile.
fn active_profile_id(conn: &rusqlite::Connection) -> Result<String> {
    profile::active(conn)?
        .map(|profile| profile.id)
        .ok_or_else(|| Error::Other("no active profile".into()))
}

#[tauri::command]
pub fn list_works(state: State<'_, AppState>, filter: Option<WorkFilter>) -> Result<Vec<Work>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    work::list(&conn, &profile_id, &filter.unwrap_or_default())
}

#[tauri::command]
pub fn get_work(state: State<'_, AppState>, id: String) -> Result<Option<Work>> {
    let conn = state.conn();
    work::get(&conn, &id)
}

#[tauri::command]
pub fn create_work(state: State<'_, AppState>, work: NewWork) -> Result<Work> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    work::create(&conn, &profile_id, work)
}

#[tauri::command]
pub fn update_work(state: State<'_, AppState>, id: String, patch: WorkPatch) -> Result<Work> {
    let conn = state.conn();
    work::update(&conn, &id, patch)
}

#[tauri::command]
pub fn delete_work(state: State<'_, AppState>, id: String) -> Result<()> {
    let conn = state.conn();
    work::delete(&conn, &id)
}

#[tauri::command]
pub fn list_versions(state: State<'_, AppState>, work_id: String) -> Result<Vec<VersionSummary>> {
    let conn = state.conn();
    version::list(&conn, &work_id)
}

#[tauri::command]
pub fn get_version(state: State<'_, AppState>, id: String) -> Result<Option<Version>> {
    let conn = state.conn();
    version::get(&conn, &id)
}

#[tauri::command]
pub fn create_version(
    state: State<'_, AppState>,
    work_id: String,
    version: NewVersion,
) -> Result<Version> {
    let mut conn = state.conn();
    version::create(&mut conn, &work_id, version)
}

#[tauri::command]
pub fn set_current_version(
    state: State<'_, AppState>,
    work_id: String,
    version_id: String,
) -> Result<()> {
    let conn = state.conn();
    version::set_current(&conn, &work_id, &version_id)
}

#[tauri::command]
pub fn delete_version(state: State<'_, AppState>, id: String) -> Result<()> {
    let mut conn = state.conn();
    version::delete(&mut conn, &id)
}

#[tauri::command]
pub fn list_notes(state: State<'_, AppState>, filter: Option<NoteFilter>) -> Result<Vec<Note>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    note::list(&conn, &profile_id, &filter.unwrap_or_default())
}

#[tauri::command]
pub fn create_note(state: State<'_, AppState>, note: NewNote) -> Result<Note> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    note::create(&conn, &profile_id, note)
}

#[tauri::command]
pub fn update_note(state: State<'_, AppState>, id: String, patch: NotePatch) -> Result<Note> {
    let conn = state.conn();
    note::update(&conn, &id, patch)
}

#[tauri::command]
pub fn delete_note(state: State<'_, AppState>, id: String) -> Result<()> {
    let conn = state.conn();
    note::delete(&conn, &id)
}

#[tauri::command]
pub fn list_tags(state: State<'_, AppState>) -> Result<Vec<(String, i64)>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    note::tags(&conn, &profile_id)
}

#[tauri::command]
pub fn score_work(state: State<'_, AppState>, work_id: String, score: NewScore) -> Result<Score> {
    let conn = state.conn();
    score::create(&conn, &work_id, score)
}

#[tauri::command]
pub fn score_history(state: State<'_, AppState>, work_id: String) -> Result<Vec<Score>> {
    let conn = state.conn();
    score::history(&conn, &work_id)
}

#[tauri::command]
pub fn latest_score(state: State<'_, AppState>, work_id: String) -> Result<Option<Score>> {
    let conn = state.conn();
    score::latest(&conn, &work_id)
}

#[tauri::command]
pub fn delete_score(state: State<'_, AppState>, id: String) -> Result<()> {
    let conn = state.conn();
    score::delete(&conn, &id)
}

#[tauri::command]
pub fn catalogue(state: State<'_, AppState>) -> Result<Vec<ScoredWork>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    score::catalogue(&conn, &profile_id)
}

#[tauri::command]
pub fn create_release(state: State<'_, AppState>, release: NewRelease) -> Result<Release> {
    let conn = state.conn();
    release::create(&conn, release)
}

#[tauri::command]
pub fn update_release(
    state: State<'_, AppState>,
    id: String,
    patch: ReleasePatch,
) -> Result<Release> {
    let conn = state.conn();
    release::update(&conn, &id, patch)
}

#[tauri::command]
pub fn delete_release(state: State<'_, AppState>, id: String) -> Result<()> {
    let conn = state.conn();
    release::delete(&conn, &id)
}

/// Claim a calendar slot. Displacing a weaker release is reported back rather
/// than done silently — the user should see what moved.
#[tauri::command]
pub fn schedule_release(
    state: State<'_, AppState>,
    id: String,
    slot: String,
) -> Result<Scheduling> {
    let mut conn = state.conn();
    release::schedule(&mut conn, &id, &slot)
}

#[tauri::command]
pub fn unschedule_release(state: State<'_, AppState>, id: String) -> Result<Release> {
    let conn = state.conn();
    release::unschedule(&conn, &id)
}

#[tauri::command]
pub fn mark_released(
    state: State<'_, AppState>,
    id: String,
    url: Option<String>,
) -> Result<Release> {
    let conn = state.conn();
    release::mark_released(&conn, &id, url)
}

#[tauri::command]
pub fn calendar(state: State<'_, AppState>) -> Result<Vec<ScheduledRelease>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    release::calendar(&conn, &profile_id)
}

#[tauri::command]
pub fn release_queue(state: State<'_, AppState>) -> Result<Vec<ScheduledRelease>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    release::queue(&conn, &profile_id)
}

#[tauri::command]
pub fn releases_for_work(state: State<'_, AppState>, work_id: String) -> Result<Vec<Release>> {
    let conn = state.conn();
    release::for_work(&conn, &work_id)
}

#[tauri::command]
pub fn list_collections(state: State<'_, AppState>) -> Result<Vec<Collection>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    collection::list(&conn, &profile_id)
}

#[tauri::command]
pub fn create_collection(
    state: State<'_, AppState>,
    collection: NewCollection,
) -> Result<Collection> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    collection::create(&conn, &profile_id, collection)
}

#[tauri::command]
pub fn update_collection(
    state: State<'_, AppState>,
    id: String,
    patch: CollectionPatch,
) -> Result<Collection> {
    let conn = state.conn();
    collection::update(&conn, &id, patch)
}

#[tauri::command]
pub fn delete_collection(state: State<'_, AppState>, id: String) -> Result<()> {
    let conn = state.conn();
    collection::delete(&conn, &id)
}

/// Write the active profile out as markdown.
#[tauri::command]
pub fn export_markdown(state: State<'_, AppState>, directory: String) -> Result<ExportReport> {
    let conn = state.conn();
    export::to_markdown(&conn, std::path::Path::new(&directory))
}

/// Copy the workspace somewhere safe.
#[tauri::command]
pub fn backup_workspace(state: State<'_, AppState>, destination: String) -> Result<String> {
    let conn = state.conn();
    let written = backup::write(&conn, std::path::Path::new(&destination))?;
    Ok(written.display().to_string())
}

/// Suggested file name for a backup taken now.
#[tauri::command]
pub fn suggested_backup_name() -> String {
    backup::suggested_name(&crate::time::now())
}

/// Where the workspace file lives, so the user can find or replace it.
#[tauri::command]
pub fn workspace_path(state: State<'_, AppState>) -> String {
    state.path().display().to_string()
}

/// Bring in a slice of a predecessor workspace. Existing titles are skipped.
#[tauri::command]
pub fn import_legacy(state: State<'_, AppState>, source: String) -> Result<ImportReport> {
    let mut conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    import::from_legacy(&mut conn, std::path::Path::new(&source), &profile_id)
}

/// Everything installed under the plugin naming convention, usable or not.
#[tauri::command]
pub fn list_plugins(state: State<'_, AppState>) -> Vec<Plugin> {
    let directory = state
        .path()
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();
    plugin::discover(&directory)
}

/// Run a plugin command against a release or a work, and merge whatever it
/// returns into that row's `meta`.
///
/// A plugin can add and overwrite its own keys but cannot clear the rest —
/// losing unrelated metadata to a third-party integration is not recoverable.
#[tauri::command]
pub fn run_plugin(
    state: State<'_, AppState>,
    executable: String,
    command: String,
    target: Target,
    id: String,
) -> Result<Option<String>> {
    let directory = state
        .path()
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();

    let found = plugin::discover(&directory)
        .into_iter()
        .find(|candidate| candidate.executable == executable)
        .ok_or_else(|| Error::Other(format!("no plugin named `{executable}`")))?;

    if !found.usable {
        return Err(Error::Other(
            found
                .reason
                .unwrap_or_else(|| format!("`{executable}` cannot be used")),
        ));
    }

    let conn = state.conn();
    let subject = match target {
        Target::Release => serde_json::to_value(
            release::get(&conn, &id)?
                .ok_or_else(|| Error::Other(format!("no release with id `{id}`")))?,
        )?,
        Target::Work => {
            let found = work::get(&conn, &id)?
                .ok_or_else(|| Error::Other(format!("no work with id `{id}`")))?;
            let mut value = serde_json::to_value(&found)?;

            // A plugin acting on a work almost always wants its text. Sending
            // only the row would make every plugin ask for the body back.
            let mut bodies = serde_json::Map::new();
            for summary in version::list(&conn, &id)? {
                if bodies.contains_key(&summary.role) {
                    continue;
                }
                if let Some(full) = version::get(&conn, &summary.id)? {
                    bodies.insert(summary.role.clone(), serde_json::Value::String(full.body));
                }
            }
            if let Some(object) = value.as_object_mut() {
                object.insert("bodies".into(), serde_json::Value::Object(bodies));
            }
            value
        }
    };
    drop(conn);

    let outcome = plugin::invoke(
        std::path::Path::new(&found.path),
        &plugin::Invocation {
            command: &command,
            target,
            subject: subject.clone(),
        },
    )?;

    if !outcome.meta.is_empty() {
        let conn = state.conn();
        let existing = subject
            .get("meta")
            .and_then(serde_json::Value::as_object)
            .cloned()
            .unwrap_or_default();
        let merged = plugin::merge_meta(&existing, &outcome.meta);

        match target {
            Target::Release => {
                release::update(
                    &conn,
                    &id,
                    ReleasePatch {
                        meta: Some(merged),
                        ..Default::default()
                    },
                )?;
            }
            Target::Work => {
                work::update(
                    &conn,
                    &id,
                    WorkPatch {
                        meta: Some(merged),
                        ..Default::default()
                    },
                )?;
            }
        }
    }

    Ok(outcome.message)
}

/// Whether the AI panel can work on this machine. Never an error: "not
/// installed" is something the panel shows, and the rest of kilna is unaffected.
#[tauri::command]
pub fn assistant_status() -> cli::Availability {
    cli::probe()
}

#[tauri::command]
pub fn list_chats(state: State<'_, AppState>, work_id: Option<String>) -> Result<Vec<Chat>> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    assistant::list(&conn, &profile_id, work_id.as_deref())
}

#[tauri::command]
pub fn create_chat(state: State<'_, AppState>, chat: NewChat) -> Result<Chat> {
    let conn = state.conn();
    let profile_id = active_profile_id(&conn)?;
    assistant::create(&conn, &profile_id, chat)
}

#[tauri::command]
pub fn get_transcript(state: State<'_, AppState>, chat_id: String) -> Result<Option<Transcript>> {
    let conn = state.conn();
    assistant::transcript(&conn, &chat_id)
}

#[tauri::command]
pub fn delete_chat(state: State<'_, AppState>, id: String) -> Result<()> {
    let conn = state.conn();
    assistant::delete(&conn, &id)
}

/// Send a prompt and wait for the reply.
///
/// This blocks for as long as the CLI takes. Tauri runs commands off the UI
/// thread, so the window stays responsive; the panel shows its own pending
/// state.
#[tauri::command]
pub fn ask_assistant(
    state: State<'_, AppState>,
    chat_id: String,
    prompt: String,
) -> Result<Message> {
    let mut conn = state.conn();
    assistant::ask(&mut conn, &chat_id, &prompt)
}

/// Fill a profile prompt template with a work's details.
#[tauri::command]
pub fn render_prompt(
    state: State<'_, AppState>,
    work_id: String,
    template: String,
) -> Result<String> {
    let conn = state.conn();
    prompt::for_work(&conn, &work_id, &template)
}

#[tauri::command]
pub fn set_collection_contents(
    state: State<'_, AppState>,
    id: String,
    work_ids: Vec<String>,
) -> Result<()> {
    let mut conn = state.conn();
    collection::set_contents(&mut conn, &id, &work_ids)
}
