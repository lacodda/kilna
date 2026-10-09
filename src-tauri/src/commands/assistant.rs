//! The assistant: chats, runs of the CLI, background tasks and their queue,
//! and the proposals they bring back.
//!
//! This is the one part of the command layer with machinery of its own: a
//! run is a process the window started, pumped on a thread of its own, and
//! the queue is drained when one ends. None of it writes the workspace - a
//! run writes its chat, which is this device's conversation - and what a
//! proposal says is kept through [`crate::actions::proposal`].

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};

use super::active;
use crate::actions;
use crate::assistant::run::{self as assistant_run, Emission, Run, Sink};
use crate::assistant::{self, Chat, NewChat, Transcript, cli, prompt};
use crate::error::{Error, Result};
use crate::profile;
use crate::state::AppState;

/// Whether the AI panel can work on this machine. Never an error: "not
/// installed" is something the panel shows, and the rest of kilna is
/// unaffected.
#[tauri::command]
pub fn assistant_status() -> cli::Availability {
    cli::probe()
}

/// The command that registers this build with Claude Code, for the settings
/// screen to show. Read-only: it names this executable, nothing more.
#[tauri::command]
pub fn mcp_registration() -> Result<String> {
    crate::mcp::registration_command()
}

/// Chats of the active profile as the list draws them — named, priced, tied
/// to their work. `work_id` narrows to one work's chats.
#[tauri::command]
pub fn list_chat_summaries(
    state: State<'_, AppState>,
    work_id: Option<String>,
) -> Result<Vec<assistant::ChatSummary>> {
    let conn = state.conn();
    assistant::summaries(&conn, &active(&conn)?, work_id.as_deref())
}

#[tauri::command]
pub fn create_chat(state: State<'_, AppState>, chat: NewChat) -> Result<Chat> {
    let conn = state.conn();
    assistant::create(&conn, &active(&conn)?, chat)
}

/// Name a chat, or clear the name with `None` so it borrows its first
/// question again.
#[tauri::command]
pub fn rename_chat(state: State<'_, AppState>, id: String, title: Option<String>) -> Result<()> {
    assistant::rename(&state.conn(), &id, title.as_deref())
}

#[tauri::command]
pub fn get_transcript(state: State<'_, AppState>, chat_id: String) -> Result<Option<Transcript>> {
    assistant::transcript(&state.conn(), &chat_id)
}

#[tauri::command]
pub fn delete_chat(state: State<'_, AppState>, id: String) -> Result<()> {
    assistant::delete(&state.conn(), &id)
}

/// Keep what a message proposes — a version, a score, a note, a whole
/// package — and mark the message applied, all as one unit.
#[tauri::command]
pub fn apply_proposal(
    state: State<'_, AppState>,
    message_id: String,
    overrides: Option<actions::proposal::Overrides>,
) -> Result<actions::proposal::Outcome> {
    actions::proposal::apply(&state.conn(), &message_id, overrides.unwrap_or_default())
}

/// Keep every proposal in a chat nobody has answered yet — one click for a
/// week of an agent's suggestions.
#[tauri::command]
pub fn apply_pending_proposals(
    state: State<'_, AppState>,
    chat_id: String,
) -> Result<Vec<actions::proposal::Outcome>> {
    actions::proposal::apply_pending(&state.conn(), &chat_id)
}

/// Every proposal waiting for an answer, across every chat of the profile:
/// what the bell reads.
#[tauri::command]
pub fn pending_proposals(state: State<'_, AppState>) -> Result<Vec<assistant::apply::Pending>> {
    let conn = state.conn();
    assistant::apply::pending(&conn, &active(&conn)?)
}

/// Every comment read off a screenshot and every drafted reply that waits for
/// the person, with what each says.
#[tauri::command]
pub fn pending_comment_proposals(
    state: State<'_, AppState>,
) -> Result<Vec<assistant::apply::CommentProposal>> {
    let conn = state.conn();
    assistant::apply::pending_comments(&conn, &active(&conn)?)
}

/// Turn a proposal down. The answer stays in the chat; it stops waiting.
#[tauri::command]
pub fn dismiss_proposal(state: State<'_, AppState>, message_id: String) -> Result<()> {
    assistant::apply::dismiss(&state.conn(), &message_id)
}

/// Sends run events to the window.
///
/// A failed emit is not worth failing a run over: the panel replays from the
/// stored events whenever it comes back.
struct WindowSink(AppHandle);

impl Sink for WindowSink {
    fn emit(&self, emission: &Emission) {
        let _ = self.0.emit(assistant_run::EVENT, emission);
    }
}

/// Start a run and return before the CLI answers.
///
/// The run belongs to the chat from here on: navigating away, closing the
/// panel or opening another work leaves it going. What it says arrives as
/// `assistant:run` events, and is stored as it arrives so a panel that was
/// elsewhere can replay it.
#[tauri::command]
pub fn start_run(
    app: AppHandle,
    state: State<'_, AppState>,
    chat_id: String,
    prompt: String,
) -> Result<Run> {
    let runs = Arc::clone(state.runs());
    let workdir = state.assistant_dir();

    let (run, stream) = {
        let conn = state.conn();
        assistant_run::start(&conn, &runs, &chat_id, &prompt, workdir.as_deref())?
    };

    let sink: Arc<dyn Sink> = Arc::new(WindowSink(app.clone()));
    let started = run.clone();

    // A thread rather than an async task: the CLI is read with blocking IO,
    // and a run holds its thread for as long as the answer takes.
    std::thread::spawn(move || {
        let open = || app.state::<AppState>().inner().open_alongside();
        assistant_run::pump(&runs, &sink, &started, &stream, open);
    });

    Ok(run)
}

/// Stop a run. Whatever it had already said stays in the chat. A run that
/// finished between the click and the call has nothing to stop, and nothing
/// worth telling a person.
#[tauri::command]
pub fn cancel_run(state: State<'_, AppState>, id: String) -> Result<()> {
    state.runs().cancel(&id);
    Ok(())
}

/// Runs of a chat, newest first — the panel's replay.
#[tauri::command]
pub fn list_runs(state: State<'_, AppState>, chat_id: String) -> Result<Vec<Run>> {
    assistant_run::list(&state.conn(), &chat_id)
}

/// Which chats are working right now, for the panel's badge.
#[tauri::command]
pub fn active_runs(state: State<'_, AppState>) -> Vec<String> {
    state.runs().active_chats()
}

/// What a task is about, as the window sends it.
///
/// One argument rather than four: the version, the scene, the block and the
/// reference files travel together everywhere else, and a command with eight
/// parameters is one where a caller swaps two of the same type without the
/// compiler noticing.
#[derive(Default, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct TaskAbout {
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub scene_id: Option<String>,
    /// One prompt block of that scene, when the action is aimed at one.
    #[serde(default)]
    pub block: Option<String>,
    #[serde(default)]
    pub attachments: Option<Vec<String>>,
    /// The style bricks picked for this run, in the order they were picked.
    #[serde(default)]
    pub style_brick_ids: Option<Vec<String>>,
    /// The lines selected in the text, for an action about a selection.
    #[serde(default)]
    pub selection: Option<String>,
}

impl TaskAbout {
    /// The borrowed form the task module reads.
    fn as_about<'a>(
        &'a self,
        attachments: &'a [String],
        style_brick_ids: &'a [String],
    ) -> assistant::task::About<'a> {
        assistant::task::About {
            version_id: self.version_id.as_deref(),
            scene_id: self.scene_id.as_deref(),
            block: self.block.as_deref(),
            attachments,
            style_brick_ids,
            selection: self.selection.as_deref(),
        }
    }
}

/// What a started task tells the card: where it went, and what it is.
#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct StartedTask {
    pub chat_id: String,
    pub run_id: String,
    /// The key the card disables its button by.
    pub task_key: String,
    /// The chat's name, so a toast can say where the answer will be.
    pub title: String,
}

/// Run a profile action against a work without opening the panel.
///
/// The action is rendered, given a chat of its own, and started — the call
/// returns as soon as the CLI is spawned, like [`start_run`]. Nothing is
/// inserted anywhere by the run itself: the answer lands in its chat, and what
/// the card does with it is the card's decision.
#[tauri::command]
pub fn start_task(
    app: AppHandle,
    state: State<'_, AppState>,
    work_id: String,
    action: String,
    about: Option<TaskAbout>,
) -> Result<StartedTask> {
    let about = about.unwrap_or_default();
    let attachments = about.attachments.clone().unwrap_or_default();
    let styles = about.style_brick_ids.clone().unwrap_or_default();
    spawn_task(
        &app,
        state.inner(),
        &work_id,
        &action,
        about.as_about(&attachments, &styles),
    )
}

/// What a task would send, without sending it: the prompt and the method,
/// composed by the very call that starts one, so the preview and the run
/// cannot part.
#[tauri::command]
pub fn preview_task(
    state: State<'_, AppState>,
    work_id: String,
    action: String,
    about: Option<TaskAbout>,
) -> Result<assistant::task::Composed> {
    let about = about.unwrap_or_default();
    let attachments = about.attachments.clone().unwrap_or_default();
    let styles = about.style_brick_ids.clone().unwrap_or_default();
    assistant::task::compose(
        &state.conn(),
        &work_id,
        &action,
        about.as_about(&attachments, &styles),
    )
}

/// What describing a brick would send, without sending it.
#[tauri::command]
pub fn preview_style_task(
    state: State<'_, AppState>,
    id: String,
    action: String,
) -> Result<assistant::task::Composed> {
    assistant::task::compose_for_style(&state.conn(), &id, &action).map(|(composed, _)| composed)
}

/// Describe a brick from its references: the same machinery a work's action
/// uses, aimed at the dictionary instead of a card.
#[tauri::command]
pub fn start_style_task(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    action: String,
) -> Result<StartedTask> {
    let state = state.inner();
    // Checked before the chat is opened, for the reason `spawn_task` checks
    // first: a refused duplicate must not leave an empty chat behind for
    // every impatient second click.
    if state
        .runs()
        .task_running(&assistant::task::style_key(&action, &id))
    {
        return Err(Error::AlreadyRunning);
    }
    let prepared = assistant::task::prepare_for_style(&state.conn(), &id, &action)?;
    launch(&app, state, prepared)
}

/// What explaining phrases would send, without sending it.
#[tauri::command]
pub fn preview_phrases_task(
    state: State<'_, AppState>,
    composition: String,
    phrases: Vec<assistant::task::PhraseAsked>,
    action: String,
) -> Result<assistant::task::Composed> {
    assistant::task::compose_for_phrases(&state.conn(), &composition, &phrases, &action)
        .map(|(composed, _)| composed)
}

/// Explain phrases the dictionary does not know: an action about phrases,
/// answered with bricks to keep (v0.94).
#[tauri::command]
pub fn start_phrases_task(
    app: AppHandle,
    state: State<'_, AppState>,
    composition: String,
    phrases: Vec<assistant::task::PhraseAsked>,
    action: String,
) -> Result<StartedTask> {
    let state = state.inner();
    let key = assistant::task::phrases_key(&action, &composition, &phrases);
    if state.runs().task_running(&key) {
        return Err(Error::AlreadyRunning);
    }
    let prepared =
        assistant::task::prepare_for_phrases(&state.conn(), &composition, &phrases, &action)?;
    launch(&app, state, prepared)
}

/// What an action on a card of the canon would send, without sending it.
#[tauri::command]
pub fn preview_card_task(
    state: State<'_, AppState>,
    id: String,
    action: String,
) -> Result<assistant::task::Composed> {
    assistant::task::compose_for_card(&state.conn(), &id, &action).map(|(composed, _)| composed)
}

/// Gather a card's facts out of its note, or describe it for a picture
/// generator: the same machinery a work's action uses, aimed at the canon.
#[tauri::command]
pub fn start_card_task(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    action: String,
) -> Result<StartedTask> {
    let state = state.inner();
    if state
        .runs()
        .task_running(&assistant::task::card_key(&action, &id))
    {
        return Err(Error::AlreadyRunning);
    }
    let prepared = assistant::task::prepare_for_card(&state.conn(), &id, &action)?;
    launch(&app, state, prepared)
}

/// What writing a release's metadata would send, without sending it.
#[tauri::command]
pub fn preview_release_task(
    state: State<'_, AppState>,
    id: String,
    action: String,
) -> Result<assistant::task::Composed> {
    assistant::task::compose_for_release(&state.conn(), &id, &action).map(|(composed, _)| composed)
}

/// Write what a release goes out under, in the background (v0.86). The
/// fields nobody has written yet are filled when the answer comes; the ones
/// already started wait beside it as a proposal.
#[tauri::command]
pub fn start_release_task(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    action: String,
) -> Result<StartedTask> {
    let state = state.inner();
    if state
        .runs()
        .task_running(&assistant::task::release_key(&action, &id))
    {
        return Err(Error::AlreadyRunning);
    }
    let prepared = assistant::task::prepare_for_release(&state.conn(), &id, &action)?;
    launch(&app, state, prepared)
}

/// What asking a cover's board for ideas would send, without sending it.
#[tauri::command]
pub fn preview_cover_task(
    state: State<'_, AppState>,
    id: String,
    action: String,
    request: crate::cover::idea::IdeaRequest,
) -> Result<assistant::task::Composed> {
    assistant::task::compose_for_cover(&state.conn(), &id, &action, &request)
        .map(|(composed, _)| composed)
}

/// Ask for ideas for a publication's cover, in the background (v0.89). They
/// land on its board as soon as the answer comes (ADR 0050).
#[tauri::command]
pub fn start_cover_task(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    action: String,
    request: crate::cover::idea::IdeaRequest,
) -> Result<StartedTask> {
    let state = state.inner();
    if state
        .runs()
        .task_running(&assistant::task::cover_key(&action, &id))
    {
        return Err(Error::AlreadyRunning);
    }
    let prepared = assistant::task::prepare_for_cover(&state.conn(), &id, &action, &request)?;
    launch(&app, state, prepared)
}

/// Stop a task by what it is rather than by which run carries it: the board
/// that asked for ideas did not necessarily start the run - "Make…" may have.
/// Says whether there was one to stop.
#[tauri::command]
pub fn stop_task(state: State<'_, AppState>, key: String) -> bool {
    state.runs().cancel_task(&key)
}

/// The proposals for one release that still wait, each field beside what is
/// written now.
#[tauri::command]
pub fn release_proposals(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<crate::release_meta::ReleaseProposal>> {
    crate::release_meta::pending(&state.conn(), &id)
}

/// What drafting a reply to a comment would send, without sending it.
#[tauri::command]
pub fn preview_comment_task(
    state: State<'_, AppState>,
    id: String,
    action: String,
) -> Result<assistant::task::Composed> {
    assistant::task::compose_for_comment(&state.conn(), &id, &action).map(|(composed, _)| composed)
}

/// Draft a reply to a comment in the background, in the voice of its channel.
/// The answer arrives as a proposal the comment shows; nothing is written
/// until the person keeps it.
#[tauri::command]
pub fn start_comment_task(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    action: String,
) -> Result<StartedTask> {
    let state = state.inner();
    if state
        .runs()
        .task_running(&assistant::task::comment_key(&action, &id))
    {
        return Err(Error::AlreadyRunning);
    }
    let prepared = assistant::task::prepare_for_comment(&state.conn(), &id, &action)?;
    launch(&app, state, prepared)
}

/// How long a pasted screenshot is kept in the temporary folder. Long
/// enough for its run to finish and be retried; after that the comment, if
/// one was kept, is its text, and the picture is not what anyone needs.
const SCREENSHOT_LIFETIME: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

/// Read a pasted screenshot of a comment in the background.
///
/// The picture goes to a temporary folder outside the workspace — it is a
/// way in, not something to keep (migration 0027) — and is read by the
/// profile's action into a proposal the comments screen offers to keep. The
/// channel and the work are where it was pasted.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn start_screenshot_task(
    app: AppHandle,
    state: State<'_, AppState>,
    bytes: Vec<u8>,
    name: String,
    channel: String,
    work_id: Option<String>,
    action: String,
    today: String,
) -> Result<StartedTask> {
    if bytes.is_empty() {
        return Err(Error::refused("comment.clipboardEmpty"));
    }
    let state = state.inner();

    let not_kept = |cause: std::io::Error| {
        Error::refused("comment.screenshotNotKept").param("cause", cause.to_string())
    };
    let folder = std::env::temp_dir().join("kilna-comment-screenshots");
    std::fs::create_dir_all(&folder).map_err(not_kept)?;
    sweep_screenshots(&folder);
    let extension = std::path::Path::new(&name)
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| ext.chars().all(|c| c.is_ascii_alphanumeric()) && ext.len() <= 5)
        .unwrap_or("png")
        .to_ascii_lowercase();
    let path = folder.join(format!("{}.{extension}", uuid::Uuid::new_v4()));
    std::fs::write(&path, &bytes).map_err(not_kept)?;

    let prepared = assistant::task::prepare_for_screenshot(
        &state.conn(),
        &action,
        &assistant::task::Screenshot {
            path: &path,
            channel: &channel,
            work_id: work_id.as_deref(),
            today: &today,
        },
    )?;
    launch(&app, state, prepared)
}

/// Remove screenshots older than their lifetime. Best effort: a file that
/// cannot be removed now will be tried again at the next paste.
fn sweep_screenshots(folder: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let stale = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|at| at.elapsed().ok())
            .is_some_and(|age| age > SCREENSHOT_LIFETIME);
        if stale {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Start a prepared task and put a thread on it: the run is recorded, the CLI
/// spawned, and the call returns while it answers. What the style, the
/// comment and the screenshot tasks share; a work's task has its own path
/// because the queue starts it too.
fn launch(
    app: &AppHandle,
    state: &AppState,
    prepared: assistant::task::Prepared,
) -> Result<StartedTask> {
    let runs = Arc::clone(state.runs());
    let workdir = state.assistant_dir();
    let (run, stream) = assistant_run::start_as(
        &state.conn(),
        &runs,
        &prepared.chat_id,
        &prepared.prompt,
        workdir.as_deref(),
        Some(prepared.key.clone()),
        &prepared.attachments,
    )?;

    let sink: Arc<dyn Sink> = Arc::new(WindowSink(app.clone()));
    let started = run.clone();
    let handle = app.clone();

    std::thread::spawn(move || {
        {
            let inner = handle.clone();
            let open = move || inner.state::<AppState>().inner().open_alongside();
            assistant_run::pump(&runs, &sink, &started, &stream, open);
        }
        drain_queue(&handle);
    });

    Ok(StartedTask {
        chat_id: prepared.chat_id,
        run_id: run.id,
        task_key: prepared.key,
        title: prepared.title,
    })
}

/// Start one task and put a thread on it.
///
/// Shared by the single-task command and by whatever picks the queue up, so
/// that a task started third in line is started exactly the way a clicked one
/// is. The thread it spawns outlives the call: when the run ends, it looks for
/// the next waiting task itself.
fn spawn_task(
    app: &AppHandle,
    state: &AppState,
    work_id: &str,
    action: &str,
    about: assistant::task::About<'_>,
) -> Result<StartedTask> {
    // Checked before the chat is opened: a refused duplicate must not leave an
    // empty chat behind for every impatient second click.
    let key = match about.scene_id {
        Some(scene_id) => assistant::task::scene_key(action, work_id, scene_id),
        None => assistant::task::key(action, work_id),
    };
    if state.runs().task_running(&key) {
        return Err(Error::AlreadyRunning);
    }

    let prepared = assistant::task::prepare(&state.conn(), work_id, action, about)?;
    launch(app, state, prepared)
}

/// Start waiting tasks until the slots are full or nothing is left.
///
/// A task that cannot start is dropped rather than put back: the reasons it
/// fails here — the work is gone, the profile no longer has the action — do not
/// get better by waiting, and a queue that retries them forever would spawn a
/// process per attempt. What is lost is a task nobody could have run; the log
/// keeps the record.
fn drain_queue(app: &AppHandle) {
    loop {
        let state = app.state::<AppState>();
        let state = state.inner();

        if !state.runs().has_slot() {
            return;
        }

        let Some(next) = state.queue().pop() else {
            return;
        };

        // A task that cannot start takes nothing with it; only a failure keeps
        // this loop going, and only to reach the next task that might work.
        let outcome = spawn_task(
            app,
            state,
            &next.work_id,
            &next.action,
            assistant::task::About::default(),
        );
        let _ = app.emit(TASK_QUEUE_EVENT, queue_state(state));

        match outcome {
            Ok(_) => return,
            Err(cause) => crate::log::warn(
                "assistant",
                &format!("a queued task on {} could not start: {cause}", next.work_id),
            ),
        }
    }
}

/// The event carrying how much of a batch is left.
pub const TASK_QUEUE_EVENT: &str = "assistant:queue";

/// What is waiting and what is going, as one answer.
///
/// Both halves travel together because a button asks one question — "is this
/// action busy?" — and a running task and a queued one are both a yes.
#[derive(serde::Serialize, Clone, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct TaskQueue {
    /// Keys of tasks with a process alive.
    pub running: Vec<String>,
    /// Keys of tasks waiting for a slot.
    pub waiting: Vec<String>,
}

fn queue_state(state: &AppState) -> TaskQueue {
    TaskQueue {
        running: state.runs().active_tasks(),
        waiting: state.queue().keys(),
    }
}

/// Chats holding a question a background task asked, oldest first - findable
/// from every screen, because a task was left alone and came back with a
/// question.
#[tauri::command]
pub fn waiting_chats(state: State<'_, AppState>) -> Result<Vec<assistant::ChatSummary>> {
    let conn = state.conn();
    assistant::waiting(&conn, &active(&conn)?)
}

/// Clear a chat's question — answered, or dismissed by hand.
#[tauri::command]
pub fn clear_waiting(state: State<'_, AppState>, chat_id: String) -> Result<()> {
    assistant::clear_waiting(&state.conn(), &chat_id)
}

/// Which profile actions are running or waiting right now, as task keys: a
/// card asks on mount, because a run started before this screen existed still
/// owns its button.
#[tauri::command]
pub fn active_tasks(state: State<'_, AppState>) -> Vec<String> {
    let mut keys = state.runs().active_tasks();
    keys.extend(state.queue().keys());
    keys.sort();
    keys.dedup();
    keys
}

/// What a batch became: started, queued, and each work passed over with why.
#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct StartedBatch {
    /// Works whose run is already going.
    pub started: usize,
    /// Works waiting for a slot.
    pub queued: usize,
    /// Works passed over — already asked for, or unable to start.
    pub skipped: Vec<actions::Skipped>,
}

/// Run one profile action against many works.
///
/// The slots are filled and the rest queued, because the alternative — three
/// runs and a row of "the machine is busy" errors — is the feature not
/// working. Returns what the batch became so the catalogue can say it in one
/// sentence rather than one toast per work.
#[tauri::command]
pub fn start_tasks(
    app: AppHandle,
    state: State<'_, AppState>,
    work_ids: Vec<String>,
    action: String,
) -> Result<StartedBatch> {
    let inner = state.inner();

    // Refused before anything is started rather than once per work: an action
    // the profile does not have is a mistake about the whole batch. The label
    // comes from the same read — the feed is written for a person, and
    // `critique` is not what the button said.
    let label = {
        let conn = inner.conn();
        let profile =
            profile::active(&conn)?.ok_or_else(|| Error::refused("profile.noneActive"))?;
        profile
            .config
            .prompts
            .iter()
            .find(|prompt| prompt.key == action)
            .ok_or_else(|| {
                Error::refused("assistant.unknownAction").param("action", action.as_str())
            })?
            .label
            .clone()
    };

    let mut started = 0usize;
    let mut queued = 0usize;
    let mut skipped: Vec<actions::Skipped> = Vec::new();
    let title_of = |work_id: &str| crate::journal::work_title(&inner.conn(), work_id);

    for work_id in &work_ids {
        let key = assistant::task::key(&action, work_id);

        // Already asked for, whether it is running or waiting. In a batch,
        // "that one was already going" is not an error, it is the reason the
        // number is smaller.
        if inner.runs().task_running(&key) || inner.queue().holds(&key) {
            skipped.push(actions::Skipped {
                id: work_id.clone(),
                title: title_of(work_id),
                reason: crate::error::Reason::of("skip.alreadyAsked"),
            });
            continue;
        }

        if inner.runs().has_slot() {
            match spawn_task(
                &app,
                inner,
                work_id,
                &action,
                assistant::task::About::default(),
            ) {
                Ok(_) => started += 1,
                // One work failing must not take the batch with it: the others
                // are unrelated, and a half-run batch is more useful than none.
                Err(cause) => skipped.push(actions::Skipped {
                    id: work_id.clone(),
                    title: title_of(work_id),
                    reason: cause.reason(),
                }),
            }
        } else if inner.queue().push(assistant::queue::Pending {
            work_id: work_id.clone(),
            action: action.clone(),
        }) {
            queued += 1;
        } else {
            skipped.push(actions::Skipped {
                id: work_id.clone(),
                title: title_of(work_id),
                reason: crate::error::Reason::of("skip.queueFull"),
            });
        }
    }

    if started + queued > 0 {
        let conn = inner.conn();
        let profile_id = active(&conn)?;
        crate::journal::record(
            &conn,
            &profile_id,
            crate::journal::Record::new("assistant.batchStarted")
                // The word as it was, not the label: a journal line is a record
                // of a past event, and it reads the same tomorrow whichever
                // language the window is in then.
                .param("action", label.as_str())
                .param("count", actions::count(started + queued)),
        );
    }

    let _ = app.emit(TASK_QUEUE_EVENT, queue_state(inner));

    Ok(StartedBatch {
        started,
        queued,
        skipped,
    })
}

/// What is running and what is waiting.
#[tauri::command]
pub fn task_queue(state: State<'_, AppState>) -> TaskQueue {
    queue_state(state.inner())
}

/// Drop everything still waiting, leaving the runs already going alone.
///
/// A person who queued forty works and changed their mind wants the forty
/// stopped, not the three already talking to the CLI killed mid-answer.
#[tauri::command]
pub fn clear_task_queue(app: AppHandle, state: State<'_, AppState>) -> usize {
    let dropped = state.queue().clear();
    let _ = app.emit(TASK_QUEUE_EVENT, queue_state(state.inner()));
    dropped
}

/// Fill a profile prompt template with a work's details.
#[tauri::command]
pub fn render_prompt(
    state: State<'_, AppState>,
    work_id: String,
    template: String,
) -> Result<String> {
    prompt::for_work(
        &state.conn(),
        &work_id,
        &template,
        prompt::Context::default(),
    )
}
