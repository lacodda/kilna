//! Runs that outlive the request that started them.
//!
//! A run belongs to its chat, not to the call that asked for it: starting one
//! returns as soon as the CLI is spawned, and everything it says afterwards
//! reaches the panel as events. Leaving the work, closing the panel, opening
//! another chat — none of it touches a run in flight.
//!
//! What is stored and what is held in memory are deliberately different. The
//! `chat_run` row is the durable record: the prompt, how it ended, and every
//! event it produced, so a panel returning to a chat can replay the run
//! instead of finding a gap. The registry in [`Runs`] is only what is needed to
//! talk to a live process — a handle to kill it, and the events so far.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::error::{Error, Result};
use crate::time::now;

use super::stream::{self, Event, Stream};

/// How many CLI processes may be alive at once.
///
/// Each spawned CLI costs a few hundred megabytes, so this is a memory budget
/// rather than a throughput knob. Three covers the way the panel is actually
/// used — ask in one chat, move to another while it thinks — without putting a
/// working machine under pressure. It becomes a profile setting in the
/// configuration stage; until then a constant is honest about not being one.
pub const PARALLEL_LIMIT: usize = 3;

/// Where a run got to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RunState {
    Running,
    Done,
    Failed,
    Cancelled,
    /// The application stopped while this run was going. Nothing is left to
    /// wait for: the process died with the parent.
    Broken,
}

impl RunState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Broken => "broken",
        }
    }

    fn from_str(raw: &str) -> Self {
        match raw {
            "running" => Self::Running,
            "done" => Self::Done,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => Self::Broken,
        }
    }

    pub fn is_over(self) -> bool {
        self != Self::Running
    }
}

/// One run, as the panel sees it.
#[derive(Debug, Clone, Serialize)]
pub struct Run {
    pub id: String,
    pub chat_id: String,
    pub prompt: String,
    pub state: RunState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Everything the run has said so far, oldest first.
    pub events: Vec<Event>,
    /// What this run is, when it was started as a named task. Held in memory
    /// only: a restart breaks every run in flight, so a stored key would name
    /// a button nothing can still be waiting on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
}

/// What the frontend is handed for each event as it happens.
#[derive(Debug, Clone, Serialize)]
pub struct Emission {
    pub run_id: String,
    pub chat_id: String,
    /// The task key, when this run was started as one. What lets a listener
    /// that is nowhere near the chat — the launcher, a card's action bar —
    /// recognise the run as the button it disabled, and say so when it ends.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    pub event: Event,
}

/// The channel events go out on. One name for every run: the panel filters by
/// `chat_id`, and a per-run event name would leave listeners to clean up.
pub const EVENT: &str = "assistant:run";

/// Somewhere to send events. Tauri's app handle in the application, a collector
/// in tests — the run loop itself does not care.
pub trait Sink: Send + Sync + 'static {
    fn emit(&self, emission: &Emission);
}

/// A live run, kept only so it can be stopped.
///
/// What is held is a way to stop it rather than the process itself: the loop
/// reading the output owns that, and the registry has no other use for it.
struct Live {
    chat_id: String,
    stop: Arc<dyn Fn() + Send + Sync>,
    /// Set when someone asked for it to stop, so the loop can tell a kill from
    /// a crash when the output ends.
    cancelled: bool,
    /// What this run is, for a run started as a named task — "this action on
    /// this work". Set only for tasks; a typed prompt is never a duplicate of
    /// anything.
    task: Option<String>,
}

/// The runs this process is carrying.
#[derive(Default)]
pub struct Runs {
    live: Mutex<HashMap<String, Live>>,
}

impl Runs {
    pub fn new() -> Self {
        Self::default()
    }

    fn live(&self) -> MutexGuard<'_, HashMap<String, Live>> {
        self.live
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// How many processes are alive.
    pub fn active(&self) -> usize {
        self.live().len()
    }

    /// Ids of the chats currently running something, for the panel's badge.
    pub fn active_chats(&self) -> Vec<String> {
        let mut chats: Vec<String> = self
            .live()
            .values()
            .map(|live| live.chat_id.clone())
            .collect();
        chats.sort();
        chats.dedup();
        chats
    }

    fn insert(
        &self,
        run_id: String,
        chat_id: String,
        stop: Arc<dyn Fn() + Send + Sync>,
        task: Option<String>,
    ) {
        self.live().insert(
            run_id,
            Live {
                chat_id,
                stop,
                cancelled: false,
                task,
            },
        );
    }

    /// Whether a task by this key is already going.
    ///
    /// The check lives here rather than in the frontend because a disabled
    /// button is only as durable as the screen holding it: closing the card
    /// and opening it again brings the button back enabled while the run is
    /// still going. The registry outlives the screen.
    pub fn task_running(&self, key: &str) -> bool {
        self.live()
            .values()
            .any(|live| live.task.as_deref() == Some(key))
    }

    /// Keys of the tasks going right now, so a reopened card can show which of
    /// its actions are already working.
    pub fn active_tasks(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .live()
            .values()
            .filter_map(|live| live.task.clone())
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    fn remove(&self, run_id: &str) -> bool {
        self.live().remove(run_id).map(|live| live.cancelled) == Some(true)
    }

    /// Stop every run this process is carrying, and say how many there were.
    ///
    /// Called when the application is closing. Without it the spawned CLIs
    /// outlive kilna and keep working — measured: three runs killed with the
    /// window were still going afterwards, spending the user's tokens on
    /// answers nothing would ever read.
    pub fn stop_all(&self) -> usize {
        let stops: Vec<_> = {
            let mut live = self.live();
            live.values_mut()
                .map(|entry| {
                    entry.cancelled = true;
                    Arc::clone(&entry.stop)
                })
                .collect()
        };

        for stop in &stops {
            stop();
        }
        stops.len()
    }

    /// Stop a run. Returns whether there was one to stop.
    ///
    /// The registry lock is given back before the process is touched: killing a
    /// process tree takes about half a second on Windows, and everything else
    /// that asks what is running would queue behind it.
    pub fn cancel(&self, run_id: &str) -> bool {
        let stop = {
            let mut live = self.live();
            let Some(entry) = live.get_mut(run_id) else {
                return false;
            };
            entry.cancelled = true;
            Arc::clone(&entry.stop)
        };

        stop();
        true
    }
}

/// Start a run in `chat_id` and return before the CLI answers.
///
/// The prompt is recorded as a user message first, so a run that fails — or a
/// crash mid-answer — still leaves what was asked. The stream comes back with
/// the run so the caller can pump it on a thread of its own.
pub fn start(
    conn: &Connection,
    runs: &Arc<Runs>,
    chat_id: &str,
    prompt: &str,
    workdir: Option<&std::path::Path>,
) -> Result<(Run, Arc<Mutex<Stream>>)> {
    start_as(conn, runs, chat_id, prompt, workdir, None)
}

/// Start a run that is also a named task, refusing if that task is already
/// going.
///
/// `task` is what the run *is* rather than which run it is: the same action on
/// the same work produces the same key every time, which is what makes a
/// second click a duplicate instead of a second opinion.
pub fn start_as(
    conn: &Connection,
    runs: &Arc<Runs>,
    chat_id: &str,
    prompt: &str,
    workdir: Option<&std::path::Path>,
    task: Option<String>,
) -> Result<(Run, Arc<Mutex<Stream>>)> {
    let chat = super::get(conn, chat_id)?.ok_or_else(|| Error::not_found("chat", chat_id))?;

    // Checked before the parallel limit: telling someone their own click is
    // already running is more useful than telling them the machine is busy.
    if let Some(key) = &task {
        if runs.task_running(key) {
            return Err(Error::Assistant(
                "This is already running. Wait for it to finish.".into(),
            ));
        }
    }

    if runs.active() >= PARALLEL_LIMIT {
        return Err(Error::Assistant(format!(
            "{PARALLEL_LIMIT} runs are already going. Wait for one to finish, or cancel it."
        )));
    }

    let id = uuid::Uuid::new_v4().to_string();
    record_asked(conn, chat_id, prompt, &id)?;

    // Saying something in a chat that was waiting is the answer to whatever it
    // asked. Done here rather than in the command so it holds however a run is
    // started, and before the CLI is spawned so the banner clears at once
    // rather than minutes later when the reply lands.
    let _ = super::clear_waiting(conn, chat_id);

    let stream = Stream::start(prompt, chat.session_id.as_deref(), workdir)?;

    let started_at = now();
    conn.execute(
        "INSERT INTO chat_run (id, chat_id, prompt, state, events, started_at)
         VALUES (?1, ?2, ?3, 'running', '[]', ?4)",
        params![id, chat_id, prompt, started_at],
    )?;

    // The stopper is taken before the stream is locked away, so cancelling
    // never has to wait for the loop that is reading it.
    let stopper = stream.stopper();
    let stream = Arc::new(Mutex::new(stream));
    runs.insert(
        id.clone(),
        chat_id.to_owned(),
        Arc::new(move || {
            stopper.stop();
        }),
        task.clone(),
    );

    let run = Run {
        id,
        chat_id: chat_id.to_owned(),
        prompt: prompt.to_owned(),
        state: RunState::Running,
        detail: None,
        events: Vec::new(),
        task,
        started_at,
        ended_at: None,
    };

    Ok((run, stream))
}

/// Where a run's events come from.
///
/// The application reads a CLI process; a test reads a script of events. The
/// loop below is the part worth testing, and it has no business knowing which
/// it is talking to.
pub trait Source {
    /// The next event, or `None` when there is nothing more coming.
    fn next_event(&mut self) -> Option<Event>;
    /// Called once the source has run dry, to release whatever it held.
    fn finish(&mut self) {}
}

impl Source for Stream {
    fn next_event(&mut self) -> Option<Event> {
        Stream::next_event(self)
    }

    fn finish(&mut self) {
        self.wait();
    }
}

/// Read a started run to its end, emitting as it goes.
///
/// Meant to be called on a thread of its own. `open` hands back a connection
/// per write rather than holding one for the length of the run: a run can take
/// minutes, and the rest of the application must keep writing meanwhile.
pub fn pump<S, F>(
    runs: &Arc<Runs>,
    sink: &Arc<dyn Sink>,
    run: &Run,
    source: &Arc<Mutex<S>>,
    open: F,
) where
    S: Source,
    F: Fn() -> Option<Connection>,
{
    let mut events: Vec<Event> = Vec::new();
    let mut outcome: Option<(RunState, Option<String>)> = None;

    loop {
        let next = source
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .next_event();

        let Some(event) = next else { break };

        // Persist before emitting: a panel told about an event must be able to
        // find it in the replay, and the reverse order can lose one to a crash
        // in between.
        events.push(event.clone());
        if let Some(conn) = open() {
            let _ = store_events(&conn, &run.id, &events);
            if let Event::Started { session_id } = &event {
                let _ = remember_session(&conn, &run.chat_id, session_id);
            }
            if let Event::Finished {
                body,
                cost_usd,
                duration_ms,
            } = &event
            {
                let mut meta = stream::finished_meta(*cost_usd, *duration_ms);
                meta.insert("run_id".into(), json!(run.id));

                // A structured result travels with the message it came in, so
                // a panel replaying the chat offers the same thing it offered
                // live. Parsed here rather than in the frontend because the
                // profile it is checked against lives on this side.
                if let Some(proposal) = proposed(&conn, run, body) {
                    meta.insert("proposal".into(), proposal);
                }

                let _ = super::append(&conn, &run.chat_id, super::ASSISTANT, body, meta);

                // Only a task is read for a question. A prompt typed in the
                // panel was asked by someone looking at the reply, and telling
                // them about a question on their own screen is noise; a task
                // was left alone on purpose, and its question would otherwise
                // sit unread.
                if run.task.is_some()
                    && super::waiting::read(body) == super::waiting::Ending::Waiting
                {
                    let _ = super::mark_waiting(&conn, &run.chat_id);
                }
            }
        }

        match &event {
            Event::Finished { .. } => outcome = Some((RunState::Done, None)),
            Event::Failed { message } => outcome = Some((RunState::Failed, Some(message.clone()))),
            _ => {}
        }

        sink.emit(&Emission {
            run_id: run.id.clone(),
            chat_id: run.chat_id.clone(),
            task: run.task.clone(),
            event,
        });
    }

    source
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .finish();

    let cancelled = runs.remove(&run.id);

    // A run whose output stopped ended one of three ways: it reported a result,
    // someone cancelled it, or the CLI died without saying anything. A result
    // that arrived before the kill landed counts as an answer — the reply is in
    // the chat either way, and calling that "cancelled" would contradict it.
    let (state, detail) = match (outcome, cancelled) {
        (Some(outcome), _) => outcome,
        (None, true) => (RunState::Cancelled, None),
        (None, false) => (
            RunState::Failed,
            Some("Claude Code stopped without answering".into()),
        ),
    };

    // A cancelled run gets its own event rather than a failure carrying the
    // word "cancelled": the panel has to be told the run is over, and a
    // sentence written here would put English into a Russian interface — which
    // is exactly what it did the first time.
    let closing = match state {
        RunState::Cancelled => Some(Event::Stopped),
        RunState::Failed
            if detail.is_some() && !matches!(events.last(), Some(Event::Failed { .. })) =>
        {
            detail.clone().map(|message| Event::Failed { message })
        }
        _ => None,
    };

    if let Some(conn) = open() {
        if let Some(event) = &closing {
            events.push(event.clone());
            let _ = store_events(&conn, &run.id, &events);
        }
        let _ = finish(&conn, &run.id, state, detail.as_deref());
    }

    if let Some(event) = closing {
        sink.emit(&Emission {
            run_id: run.id.clone(),
            chat_id: run.chat_id.clone(),
            task: run.task.clone(),
            event,
        });
    }
}

/// What this run's answer proposed, when its action asked for something the
/// application can act on.
///
/// Nothing is proposed for a typed prompt: only a profile action declares what
/// it produces, and only a task carries the key that names the action.
fn proposed(conn: &Connection, run: &Run, body: &str) -> Option<Value> {
    let action = run.task.as_ref()?.split(':').next()?.to_owned();
    let profile = crate::profile::active(conn).ok()??;

    let template = profile
        .config
        .prompts
        .iter()
        .find(|prompt| prompt.key == action)?;

    if template.produces.as_deref() != Some(super::task::SCORE) {
        return None;
    }

    // The answer is handed in rather than read off the run: `run` is the
    // snapshot taken when the run started, and its event list is empty. Reading
    // it there returned nothing, always — caught by the test that expected a
    // proposal and found none.
    let proposal = super::proposal::read_score(body, &profile.config)?;
    serde_json::to_value(proposal).ok()
}

/// Record what was asked, tied to the run that will answer it.
///
/// The tie is what lets the panel pair the stored message with the run's
/// events instead of guessing by text.
fn record_asked(conn: &Connection, chat_id: &str, prompt: &str, run_id: &str) -> Result<()> {
    let mut meta = Map::new();
    meta.insert("run_id".into(), json!(run_id));
    super::append(conn, chat_id, super::USER, prompt, meta)?;
    Ok(())
}

fn store_events(conn: &Connection, run_id: &str, events: &[Event]) -> Result<()> {
    conn.execute(
        "UPDATE chat_run SET events = ?2 WHERE id = ?1",
        params![run_id, serde_json::to_string(events)?],
    )?;
    Ok(())
}

fn remember_session(conn: &Connection, chat_id: &str, session_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE chat SET session_id = ?2, updated_at = ?3 WHERE id = ?1",
        params![chat_id, session_id, now()],
    )?;
    Ok(())
}

fn finish(conn: &Connection, run_id: &str, state: RunState, detail: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE chat_run SET state = ?2, detail = ?3, ended_at = ?4 WHERE id = ?1",
        params![run_id, state.as_str(), detail, now()],
    )?;
    Ok(())
}

/// Runs of a chat, newest first. This is what the panel replays.
pub fn list(conn: &Connection, chat_id: &str) -> Result<Vec<Run>> {
    let mut statement = conn.prepare(
        "SELECT id, chat_id, prompt, state, detail, events, started_at, ended_at
         FROM chat_run WHERE chat_id = ?1 ORDER BY started_at DESC, id",
    )?;
    let rows = statement.query_map(params![chat_id], read_run)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .map(hydrate)
        .collect()
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Run>> {
    let row = conn
        .query_row(
            "SELECT id, chat_id, prompt, state, detail, events, started_at, ended_at
             FROM chat_run WHERE id = ?1",
            params![id],
            read_run,
        )
        .optional()?;

    row.map(hydrate).transpose()
}

/// Mark runs left `running` by a previous life of the application as broken.
///
/// Called at startup. The processes died with the parent, so there is nothing
/// to adopt — the only honest thing is to stop showing them as alive.
pub fn sweep(conn: &Connection) -> Result<usize> {
    let swept = conn.execute(
        "UPDATE chat_run
         SET state = 'broken', detail = ?2, ended_at = ?1
         WHERE state = 'running'",
        params![now(), "kilna closed while this run was going"],
    )?;
    Ok(swept)
}

type RawRun = (
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    Option<String>,
);

fn read_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawRun> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
    ))
}

fn hydrate(raw: RawRun) -> Result<Run> {
    let (id, chat_id, prompt, state, detail, events, started_at, ended_at) = raw;

    // An unreadable event log must not hide the run itself: what it was and how
    // it ended is worth more than the blow-by-blow.
    let events = serde_json::from_str::<Vec<Value>>(&events)
        .ok()
        .map(|raw| {
            raw.into_iter()
                .filter_map(|value| serde_json::from_value(value).ok())
                .collect()
        })
        .unwrap_or_default();

    Ok(Run {
        id,
        chat_id,
        prompt,
        state: RunState::from_str(&state),
        detail,
        events,
        task: None,
        started_at,
        ended_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::NewChat;
    use crate::db;
    use crate::profile;

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    fn chat(conn: &Connection, profile_id: &str) -> String {
        super::super::create(
            conn,
            profile_id,
            NewChat {
                work_id: None,
                title: None,
            },
        )
        .unwrap()
        .id
    }

    fn record(conn: &Connection, chat_id: &str, state: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO chat_run (id, chat_id, prompt, state, events, started_at)
             VALUES (?1, ?2, 'ask', ?3, '[]', ?4)",
            params![id, chat_id, state, now()],
        )
        .unwrap();
        id
    }

    #[test]
    fn a_run_that_survived_a_crash_is_swept_at_startup() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        let orphan = record(&conn, &chat_id, "running");
        let finished = record(&conn, &chat_id, "done");

        assert_eq!(sweep(&conn).unwrap(), 1);

        assert_eq!(
            get(&conn, &orphan).unwrap().unwrap().state,
            RunState::Broken
        );
        assert_eq!(
            get(&conn, &finished).unwrap().unwrap().state,
            RunState::Done,
            "a run that ended before the crash must be left alone"
        );
    }

    #[test]
    fn sweeping_twice_changes_nothing_the_second_time() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        record(&conn, &chat_id, "running");

        assert_eq!(sweep(&conn).unwrap(), 1);
        assert_eq!(sweep(&conn).unwrap(), 0);
    }

    #[test]
    fn a_swept_run_says_why_it_ended() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        let orphan = record(&conn, &chat_id, "running");

        sweep(&conn).unwrap();

        let run = get(&conn, &orphan).unwrap().unwrap();
        assert!(run.detail.is_some(), "a broken run must explain itself");
        assert!(run.ended_at.is_some());
    }

    #[test]
    fn runs_come_back_newest_first() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);

        let first = record(&conn, &chat_id, "done");
        // Same-instant runs are ordered by id, so make the second one later.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let second = record(&conn, &chat_id, "done");

        let ids: Vec<_> = list(&conn, &chat_id)
            .unwrap()
            .into_iter()
            .map(|run| run.id)
            .collect();
        assert_eq!(ids, vec![second, first]);
    }

    #[test]
    fn a_runs_events_are_replayed_in_order() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        let run_id = record(&conn, &chat_id, "running");

        let events = vec![
            Event::Started {
                session_id: "s".into(),
            },
            Event::Tool {
                name: "Read".into(),
                detail: "notes.md".into(),
            },
            Event::Text {
                body: "here".into(),
            },
        ];
        store_events(&conn, &run_id, &events).unwrap();

        assert_eq!(get(&conn, &run_id).unwrap().unwrap().events, events);
    }

    #[test]
    fn a_corrupt_event_log_does_not_hide_the_run() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        let run_id = record(&conn, &chat_id, "failed");
        conn.execute(
            "UPDATE chat_run SET events = 'not json' WHERE id = ?1",
            params![run_id],
        )
        .unwrap();

        let run = get(&conn, &run_id).unwrap().unwrap();

        assert_eq!(run.state, RunState::Failed);
        assert!(run.events.is_empty());
    }

    #[test]
    fn an_event_shape_from_a_newer_build_is_skipped_not_fatal() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        let run_id = record(&conn, &chat_id, "done");
        conn.execute(
            r#"UPDATE chat_run SET events = '[{"kind":"text","body":"kept"},{"kind":"telepathy"}]' WHERE id = ?1"#,
            params![run_id],
        )
        .unwrap();

        let run = get(&conn, &run_id).unwrap().unwrap();

        assert_eq!(
            run.events,
            vec![Event::Text {
                body: "kept".into()
            }]
        );
    }

    #[test]
    fn deleting_a_chat_takes_its_runs_with_it() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        record(&conn, &chat_id, "done");

        super::super::delete(&conn, &chat_id).unwrap();

        let left: i64 = conn
            .query_row("SELECT count(*) FROM chat_run", [], |row| row.get(0))
            .unwrap();
        assert_eq!(left, 0);
    }

    #[test]
    fn the_schema_refuses_a_state_the_code_does_not_know() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);

        let invented = conn.execute(
            "INSERT INTO chat_run (id, chat_id, prompt, state, events, started_at)
             VALUES ('x', ?1, 'ask', 'pondering', '[]', ?2)",
            params![chat_id, now()],
        );

        assert!(invented.is_err());
    }

    #[test]
    fn every_state_survives_a_round_trip_through_the_database() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);

        for state in [
            RunState::Running,
            RunState::Done,
            RunState::Failed,
            RunState::Cancelled,
            RunState::Broken,
        ] {
            let id = record(&conn, &chat_id, state.as_str());
            assert_eq!(
                get(&conn, &id).unwrap().unwrap().state,
                state,
                "`{}` did not come back as itself",
                state.as_str()
            );
        }
    }

    #[test]
    fn only_a_running_run_is_unfinished() {
        assert!(!RunState::Running.is_over());
        for state in [
            RunState::Done,
            RunState::Failed,
            RunState::Cancelled,
            RunState::Broken,
        ] {
            assert!(state.is_over(), "`{}` is over", state.as_str());
        }
    }

    /// A scripted run: hands out the events it was given, in order.
    struct Script(std::vec::IntoIter<Event>);

    impl Source for Script {
        fn next_event(&mut self) -> Option<Event> {
            self.0.next()
        }
    }

    /// Collects what the panel would have been told.
    #[derive(Default)]
    struct Collector(Mutex<Vec<Emission>>);

    impl Sink for Collector {
        fn emit(&self, emission: &Emission) {
            self.0.lock().unwrap().push(emission.clone());
        }
    }

    /// A workspace on disk, so the run loop can open its own connections the
    /// way it does in the application and the test can still read the rows.
    fn on_disk() -> (tempfile::TempDir, std::path::PathBuf, Connection, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspace.db");
        let conn = db::open(&path).unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (dir, path, conn, profile_id)
    }

    /// Run `events` through the loop against a real workspace.
    fn pumped(
        script: Vec<Event>,
        before: impl Fn(&Arc<Runs>, &Run),
    ) -> (Run, Vec<Emission>, Connection, tempfile::TempDir) {
        let (dir, path, conn, profile_id) = on_disk();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());

        let run_id = record(&conn, &chat_id, "running");
        let run = get(&conn, &run_id).unwrap().unwrap();

        // Registered as `start` would have, so cancellation has something to
        // find. A scripted source has no process to kill, so stopping it only
        // has to mark the run.
        runs.insert(run_id.clone(), chat_id.clone(), Arc::new(|| {}), None);
        before(&runs, &run);

        let collector: Arc<Collector> = Arc::new(Collector::default());
        let sink: Arc<dyn Sink> = collector.clone();
        let source = Arc::new(Mutex::new(Script(script.into_iter())));

        pump(&runs, &sink, &run, &source, || Connection::open(&path).ok());

        let emitted = collector.0.lock().unwrap().clone();
        (get(&conn, &run_id).unwrap().unwrap(), emitted, conn, dir)
    }

    #[test]
    fn a_finished_run_records_the_answer_and_ends_as_done() {
        let (run, emitted, conn, _dir) = pumped(
            vec![
                Event::Started {
                    session_id: "sess-1".into(),
                },
                Event::Tool {
                    name: "Read".into(),
                    detail: "notes.md".into(),
                },
                Event::Text {
                    body: "A verse.".into(),
                },
                Event::Finished {
                    body: "A verse.".into(),
                    cost_usd: Some(0.12),
                    duration_ms: Some(900),
                },
            ],
            |_, _| {},
        );

        assert_eq!(run.state, RunState::Done);
        assert!(run.ended_at.is_some());
        assert_eq!(run.events.len(), 4, "every event is kept for the replay");
        assert_eq!(emitted.len(), 4, "and every one reached the panel");

        let (body, meta): (String, String) = conn
            .query_row(
                "SELECT body, meta FROM chat_message WHERE role = 'assistant'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(body, "A verse.");

        let meta: Value = serde_json::from_str(&meta).unwrap();
        assert_eq!(
            meta["run_id"],
            json!(run.id),
            "the answer must name the run that wrote it"
        );
    }

    #[test]
    fn what_was_asked_is_tied_to_its_run() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);

        record_asked(&conn, &chat_id, "hello", "run-9").unwrap();

        let meta: String = conn
            .query_row(
                "SELECT meta FROM chat_message WHERE role = 'user'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let meta: Value = serde_json::from_str(&meta).unwrap();
        assert_eq!(meta["run_id"], json!("run-9"));
    }

    #[test]
    fn every_event_is_stored_before_the_panel_hears_about_it() {
        // A panel told about an event must be able to find it in the replay.
        // The check runs from inside the sink, mid-run: by then the row must
        // already carry what is being emitted.
        let (dir, path, conn, profile_id) = on_disk();
        let chat_id = chat(&conn, &profile_id);
        let run_id = record(&conn, &chat_id, "running");
        let run = get(&conn, &run_id).unwrap().unwrap();
        let runs = Arc::new(Runs::new());
        runs.insert(run_id.clone(), chat_id, Arc::new(|| {}), None);

        struct Watcher {
            path: std::path::PathBuf,
            missed: Mutex<Vec<String>>,
        }

        impl Sink for Watcher {
            fn emit(&self, emission: &Emission) {
                let conn = Connection::open(&self.path).unwrap();
                let stored = get(&conn, &emission.run_id).unwrap().unwrap();
                if !stored.events.contains(&emission.event) {
                    self.missed
                        .lock()
                        .unwrap()
                        .push(format!("{:?}", emission.event));
                }
            }
        }

        let watcher = Arc::new(Watcher {
            path: path.clone(),
            missed: Mutex::new(Vec::new()),
        });
        let sink: Arc<dyn Sink> = watcher.clone();
        let source = Arc::new(Mutex::new(Script(
            vec![
                Event::Started {
                    session_id: "s".into(),
                },
                Event::Text { body: "one".into() },
                Event::Finished {
                    body: "one".into(),
                    cost_usd: None,
                    duration_ms: None,
                },
            ]
            .into_iter(),
        )));

        pump(&runs, &sink, &run, &source, || Connection::open(&path).ok());

        let missed = watcher.missed.lock().unwrap().clone();
        assert!(missed.is_empty(), "emitted before storing: {missed:?}");
        drop(dir);
    }

    #[test]
    fn the_session_is_remembered_as_soon_as_the_cli_names_it() {
        let (run, _, conn, _dir) = pumped(
            vec![Event::Started {
                session_id: "sess-2".into(),
            }],
            |_, _| {},
        );

        let session: Option<String> = conn
            .query_row(
                "SELECT session_id FROM chat WHERE id = ?1",
                params![run.chat_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            session.as_deref(),
            Some("sess-2"),
            "a run cut short must still leave the chat resumable"
        );
    }

    #[test]
    fn a_run_whose_output_just_stops_is_a_failure_that_says_so() {
        let (run, emitted, _conn, _dir) = pumped(
            vec![Event::Started {
                session_id: "sess-3".into(),
            }],
            |_, _| {},
        );

        assert_eq!(run.state, RunState::Failed);
        assert!(run.detail.is_some(), "silence must be explained");
        assert!(
            matches!(emitted.last().map(|e| &e.event), Some(Event::Failed { .. })),
            "and the panel must be told rather than left waiting"
        );
    }

    #[test]
    fn a_reported_failure_is_not_repeated_to_the_panel_twice() {
        let (run, emitted, _conn, _dir) = pumped(
            vec![Event::Failed {
                message: "Not logged in".into(),
            }],
            |_, _| {},
        );

        assert_eq!(run.state, RunState::Failed);
        assert_eq!(run.detail.as_deref(), Some("Not logged in"));
        assert_eq!(emitted.len(), 1, "the CLI already said it");
    }

    #[test]
    fn a_cancelled_run_ends_as_cancelled_and_leaves_no_answer() {
        let (run, emitted, conn, _dir) = pumped(
            vec![Event::Text {
                body: "half a th".into(),
            }],
            |runs, run| {
                runs.cancel(&run.id);
            },
        );

        assert_eq!(run.state, RunState::Cancelled);
        assert!(
            matches!(emitted.last().map(|e| &e.event), Some(Event::Stopped)),
            "the panel must be told it is over, and told without an English sentence"
        );

        let answers: i64 = conn
            .query_row(
                "SELECT count(*) FROM chat_message WHERE role = 'assistant'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(answers, 0);
    }

    #[test]
    fn an_answer_that_arrived_before_the_kill_landed_still_counts() {
        // Killing a process is not instant: the CLI can finish in the gap.
        // Calling that "cancelled" would contradict the reply sitting in the chat.
        let (run, _, conn, _dir) = pumped(
            vec![Event::Finished {
                body: "done in time".into(),
                cost_usd: None,
                duration_ms: None,
            }],
            |runs, run| {
                runs.cancel(&run.id);
            },
        );

        assert_eq!(run.state, RunState::Done);
        let answers: i64 = conn
            .query_row(
                "SELECT count(*) FROM chat_message WHERE role = 'assistant'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(answers, 1);
    }

    #[test]
    fn a_finished_run_gives_its_slot_back() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());
        let run_id = record(&conn, &chat_id, "running");
        let run = get(&conn, &run_id).unwrap().unwrap();
        runs.insert(run_id, chat_id, Arc::new(|| {}), None);

        assert_eq!(runs.active(), 1);
        assert_eq!(runs.active_chats().len(), 1);

        let sink: Arc<dyn Sink> = Arc::new(Collector::default());
        let source = Arc::new(Mutex::new(Script(Vec::new().into_iter())));
        pump(&runs, &sink, &run, &source, || None);

        assert_eq!(runs.active(), 0, "the slot must go back to the limit");
        assert!(runs.active_chats().is_empty());
    }

    #[test]
    fn a_run_survives_a_database_that_will_not_open() {
        // Storage failing mid-run must not take the process down: the answer is
        // still worth showing, and the panel is the only place left to show it.
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());
        let run_id = record(&conn, &chat_id, "running");
        let run = get(&conn, &run_id).unwrap().unwrap();
        runs.insert(run_id, chat_id, Arc::new(|| {}), None);

        let collector: Arc<Collector> = Arc::new(Collector::default());
        let sink: Arc<dyn Sink> = collector.clone();
        let source = Arc::new(Mutex::new(Script(
            vec![Event::Finished {
                body: "kept".into(),
                cost_usd: None,
                duration_ms: None,
            }]
            .into_iter(),
        )));

        pump(&runs, &sink, &run, &source, || None);

        assert_eq!(collector.0.lock().unwrap().len(), 1);
    }

    #[test]
    fn the_limit_refuses_a_fourth_run_rather_than_spawning_it() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());

        for index in 0..PARALLEL_LIMIT {
            runs.insert(
                format!("run-{index}"),
                chat_id.clone(),
                Arc::new(|| {}),
                None,
            );
        }

        let Err(refused) = start(&conn, &runs, &chat_id, "one more", None) else {
            panic!("the limit must refuse a fourth run");
        };

        assert!(
            matches!(refused, Error::Assistant(_)),
            "the panel needs a sentence, not a database error"
        );
        // Nothing was written: no message, no row, no process.
        let asked: i64 = conn
            .query_row("SELECT count(*) FROM chat_message", [], |row| row.get(0))
            .unwrap();
        assert_eq!(asked, 0);
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM chat_run", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 0);
    }

    #[test]
    fn every_event_of_a_task_carries_its_key() {
        let (dir, path, conn, profile_id) = on_disk();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());

        let run_id = record(&conn, &chat_id, "running");
        let mut run = get(&conn, &run_id).unwrap().unwrap();
        run.task = Some("critique:w1".into());
        runs.insert(run_id, chat_id, Arc::new(|| {}), Some("critique:w1".into()));

        let collector: Arc<Collector> = Arc::new(Collector::default());
        let sink: Arc<dyn Sink> = collector.clone();
        let source = Arc::new(Mutex::new(Script(
            vec![
                Event::Text {
                    body: "half".into(),
                },
                Event::Finished {
                    body: "done".into(),
                    cost_usd: None,
                    duration_ms: None,
                },
            ]
            .into_iter(),
        )));

        pump(&runs, &sink, &run, &source, || Connection::open(&path).ok());

        let emitted = collector.0.lock().unwrap().clone();
        assert!(!emitted.is_empty());
        assert!(
            emitted
                .iter()
                .all(|emission| emission.task.as_deref() == Some("critique:w1")),
            "a listener far from the chat recognises the run only by its key"
        );
        drop(dir);
    }

    #[test]
    fn a_typed_prompt_emits_without_a_task_key() {
        let (run, emitted, _conn, _dir) = pumped(
            vec![Event::Finished {
                body: "done".into(),
                cost_usd: None,
                duration_ms: None,
            }],
            |_, _| {},
        );

        assert_eq!(run.state, RunState::Done);
        assert!(
            emitted.iter().all(|emission| emission.task.is_none()),
            "nothing typed by hand is a task, and no button is waiting on it"
        );
    }

    /// A finished task that asked something leaves the chat marked, so a
    /// question nobody was watching for is still found.
    #[test]
    fn a_task_that_ends_by_asking_marks_its_chat() {
        let (dir, path, conn, profile_id) = on_disk();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());

        let run_id = record(&conn, &chat_id, "running");
        let mut run = get(&conn, &run_id).unwrap().unwrap();
        run.task = Some("critique:w1".into());
        runs.insert(
            run_id,
            chat_id.clone(),
            Arc::new(|| {}),
            Some("critique:w1".into()),
        );

        let collector: Arc<Collector> = Arc::new(Collector::default());
        let sink: Arc<dyn Sink> = collector.clone();
        let source = Arc::new(Mutex::new(Script(
            vec![Event::Finished {
                body: "Two readings work here.

Which one do you want?"
                    .into(),
                cost_usd: None,
                duration_ms: None,
            }]
            .into_iter(),
        )));

        pump(&runs, &sink, &run, &source, || Connection::open(&path).ok());

        let chat = super::super::get(&conn, &chat_id).unwrap().unwrap();
        assert!(
            chat.waiting_since.is_some(),
            "a task that ended by asking must leave the question findable"
        );
        drop(dir);
    }

    #[test]
    fn a_task_that_reports_leaves_the_chat_alone() {
        let (dir, path, conn, profile_id) = on_disk();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());

        let run_id = record(&conn, &chat_id, "running");
        let mut run = get(&conn, &run_id).unwrap().unwrap();
        run.task = Some("critique:w1".into());
        runs.insert(
            run_id,
            chat_id.clone(),
            Arc::new(|| {}),
            Some("critique:w1".into()),
        );

        let collector: Arc<Collector> = Arc::new(Collector::default());
        let sink: Arc<dyn Sink> = collector.clone();
        let source = Arc::new(Mutex::new(Script(
            vec![Event::Finished {
                body: "I tightened the second verse and left the chorus alone.".into(),
                cost_usd: None,
                duration_ms: None,
            }]
            .into_iter(),
        )));

        pump(&runs, &sink, &run, &source, || Connection::open(&path).ok());

        let chat = super::super::get(&conn, &chat_id).unwrap().unwrap();
        assert!(
            chat.waiting_since.is_none(),
            "a banner on finished work is noise"
        );
        drop(dir);
    }

    /// A question asked in the panel is already on the asker's screen.
    #[test]
    fn a_typed_prompt_that_ends_by_asking_marks_nothing() {
        let (dir, path, conn, profile_id) = on_disk();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());

        let run_id = record(&conn, &chat_id, "running");
        let run = get(&conn, &run_id).unwrap().unwrap();
        runs.insert(run_id, chat_id.clone(), Arc::new(|| {}), None);

        let collector: Arc<Collector> = Arc::new(Collector::default());
        let sink: Arc<dyn Sink> = collector.clone();
        let source = Arc::new(Mutex::new(Script(
            vec![Event::Finished {
                body: "Which verse should go?".into(),
                cost_usd: None,
                duration_ms: None,
            }]
            .into_iter(),
        )));

        pump(&runs, &sink, &run, &source, || Connection::open(&path).ok());

        let chat = super::super::get(&conn, &chat_id).unwrap().unwrap();
        assert!(
            chat.waiting_since.is_none(),
            "the person who typed it is looking at the reply"
        );
        drop(dir);
    }

    /// A scoring action's answer arrives with the proposal already read, so
    /// the panel offers the same thing live and on replay.
    #[test]
    fn a_scoring_task_attaches_what_it_proposed() {
        let (dir, path, conn, profile_id) = on_disk();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());

        let config = crate::profile::active(&conn).unwrap().unwrap().config;
        let axis = config.axes[0].key.clone();

        let run_id = record(&conn, &chat_id, "running");
        let mut run = get(&conn, &run_id).unwrap().unwrap();
        run.task = Some(format!("score:{chat_id}"));
        runs.insert(run_id, chat_id.clone(), Arc::new(|| {}), run.task.clone());

        let collector: Arc<Collector> = Arc::new(Collector::default());
        let sink: Arc<dyn Sink> = collector.clone();
        let source = Arc::new(Mutex::new(Script(
            vec![Event::Finished {
                body: format!("It holds up.\n\n```json\n{{\"axes\": {{\"{axis}\": 8}}}}\n```"),
                cost_usd: None,
                duration_ms: None,
            }]
            .into_iter(),
        )));

        pump(&runs, &sink, &run, &source, || Connection::open(&path).ok());

        let transcript = super::super::transcript(&conn, &chat_id).unwrap().unwrap();
        let answer = transcript
            .messages
            .iter()
            .find(|message| message.role == super::super::ASSISTANT)
            .expect("the run must leave an answer");

        let proposal = answer
            .meta
            .get("proposal")
            .expect("a scoring action must arrive with its proposal read");
        assert_eq!(proposal.get("kind").and_then(|k| k.as_str()), Some("score"));
        assert_eq!(
            proposal
                .get("axes")
                .and_then(|axes| axes.get(&axis))
                .and_then(serde_json::Value::as_f64),
            Some(8.0)
        );
        drop(dir);
    }

    /// An ordinary action answers in prose, and nothing is proposed.
    #[test]
    fn a_plain_task_attaches_no_proposal() {
        let (dir, path, conn, profile_id) = on_disk();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());

        let run_id = record(&conn, &chat_id, "running");
        let mut run = get(&conn, &run_id).unwrap().unwrap();
        run.task = Some(format!("critique:{chat_id}"));
        runs.insert(run_id, chat_id.clone(), Arc::new(|| {}), run.task.clone());

        let collector: Arc<Collector> = Arc::new(Collector::default());
        let sink: Arc<dyn Sink> = collector.clone();
        let source = Arc::new(Mutex::new(Script(
            vec![Event::Finished {
                body: "The second verse is the weak one.".into(),
                cost_usd: None,
                duration_ms: None,
            }]
            .into_iter(),
        )));

        pump(&runs, &sink, &run, &source, || Connection::open(&path).ok());

        let transcript = super::super::transcript(&conn, &chat_id).unwrap().unwrap();
        let answer = transcript
            .messages
            .iter()
            .find(|message| message.role == super::super::ASSISTANT)
            .unwrap();

        assert!(answer.meta.get("proposal").is_none());
        drop(dir);
    }

    #[test]
    fn a_task_already_going_is_refused_a_second_time() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());

        runs.insert(
            "live".into(),
            chat_id.clone(),
            Arc::new(|| {}),
            Some("critique:w1".into()),
        );

        let Err(refused) = start_as(
            &conn,
            &runs,
            &chat_id,
            "again",
            None,
            Some("critique:w1".into()),
        ) else {
            panic!("the same task must not run twice at once");
        };

        assert!(matches!(refused, Error::Assistant(_)));
        // Refused before anything was written: a duplicate must not leave a
        // question in the transcript that no run will ever answer.
        let asked: i64 = conn
            .query_row("SELECT count(*) FROM chat_message", [], |row| row.get(0))
            .unwrap();
        assert_eq!(asked, 0);
    }

    #[test]
    fn the_duplicate_check_is_by_task_not_by_chat_or_work() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());

        runs.insert(
            "live".into(),
            chat_id,
            Arc::new(|| {}),
            Some("critique:w1".into()),
        );

        assert!(runs.task_running("critique:w1"));
        assert!(
            !runs.task_running("critique:w2"),
            "the same action on another work is not the same task"
        );
        assert!(
            !runs.task_running("score:w1"),
            "another action on the same work is not the same task"
        );
    }

    #[test]
    fn a_finished_task_frees_its_key() {
        let runs = Arc::new(Runs::new());
        runs.insert(
            "live".into(),
            "chat".into(),
            Arc::new(|| {}),
            Some("critique:w1".into()),
        );

        runs.remove("live");

        assert!(
            !runs.task_running("critique:w1"),
            "a key held past the run would lock the button forever"
        );
        assert!(runs.active_tasks().is_empty());
    }

    #[test]
    fn typed_prompts_never_count_as_duplicates_of_each_other() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());

        runs.insert("live".into(), chat_id.clone(), Arc::new(|| {}), None);
        runs.insert("other".into(), chat_id, Arc::new(|| {}), None);

        assert!(runs.active_tasks().is_empty());
        let _ = conn;
    }

    #[test]
    fn active_tasks_lists_each_key_once() {
        let runs = Arc::new(Runs::new());
        runs.insert(
            "a".into(),
            "chat".into(),
            Arc::new(|| {}),
            Some("score:w1".into()),
        );
        runs.insert(
            "b".into(),
            "chat".into(),
            Arc::new(|| {}),
            Some("critique:w1".into()),
        );
        runs.insert("c".into(), "chat".into(), Arc::new(|| {}), None);

        assert_eq!(runs.active_tasks(), vec!["critique:w1", "score:w1"]);
    }

    /// A run that never started answered nothing, so the question stands.
    #[test]
    fn a_refused_run_leaves_the_question_standing() {
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());
        super::super::mark_waiting(&conn, &chat_id).unwrap();

        // The limit refuses this before any process is spawned, which is all
        // this test needs: clearing happens before that check is reached.
        for index in 0..PARALLEL_LIMIT {
            runs.insert(
                format!("run-{index}"),
                chat_id.clone(),
                Arc::new(|| {}),
                None,
            );
        }
        let _ = start(&conn, &runs, &chat_id, "here is your answer", None);

        assert!(
            super::super::get(&conn, &chat_id)
                .unwrap()
                .unwrap()
                .waiting_since
                .is_some(),
            "a refused run answered nothing, so the question stands"
        );
    }

    #[test]
    fn starting_a_run_in_a_chat_that_is_gone_touches_nothing() {
        let (conn, _) = workspace();
        let runs = Arc::new(Runs::new());

        assert!(matches!(
            start(&conn, &runs, "no-such-chat", "hello", None),
            Err(Error::NotFound { .. })
        ));
        assert_eq!(runs.active(), 0);
    }

    #[test]
    fn cancelling_reaches_the_process_while_the_reader_is_still_blocked() {
        // The defect this guards against was found live, not here: the kill
        // used to live on the stream, so cancelling had to take the lock the
        // reading loop holds while it waits for the next line — which is the
        // whole time a run is going. Cancellation blocked until the CLI
        // finished by itself, and a run cancelled after two seconds went on to
        // write for another minute.
        //
        // What is asserted is the thing that was broken: the stop reaches the
        // process while the reader is inside `next_event`, not after it.
        let (conn, profile_id) = workspace();
        let chat_id = chat(&conn, &profile_id);
        let runs = Arc::new(Runs::new());
        let run_id = record(&conn, &chat_id, "running");
        let run = get(&conn, &run_id).unwrap().unwrap();

        /// A source that stays inside `next_event` until it is stopped, the way
        /// a CLI that is thinking keeps the reader waiting on a line.
        struct Blocking {
            stopped: Arc<std::sync::atomic::AtomicBool>,
            reading: Arc<std::sync::atomic::AtomicBool>,
        }

        impl Source for Blocking {
            fn next_event(&mut self) -> Option<Event> {
                self.reading
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                while !self.stopped.load(std::sync::atomic::Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                None
            }
        }

        let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let reading = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let by_cancel = Arc::clone(&stopped);
        runs.insert(
            run_id.clone(),
            chat_id,
            Arc::new(move || {
                by_cancel.store(true, std::sync::atomic::Ordering::SeqCst);
            }),
            None,
        );

        let source = Arc::new(Mutex::new(Blocking {
            stopped: Arc::clone(&stopped),
            reading: Arc::clone(&reading),
        }));
        let pumping = {
            let runs = Arc::clone(&runs);
            let source = Arc::clone(&source);
            std::thread::spawn(move || {
                let sink: Arc<dyn Sink> = Arc::new(Silent);
                pump(&runs, &sink, &run, &source, || None);
            })
        };

        // Wait until the reader is genuinely inside `next_event`, so the
        // cancellation below happens against a blocked reader rather than
        // before the loop has started.
        let entered = std::time::Instant::now();
        while !reading.load(std::sync::atomic::Ordering::SeqCst) {
            assert!(
                entered.elapsed() < std::time::Duration::from_secs(5),
                "the reading loop never started"
            );
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        // Cancelled on a thread of its own with a deadline, because the way
        // this failed was a wait with no end: asserting on elapsed time after
        // the call would never be reached.
        let cancelling = {
            let runs = Arc::clone(&runs);
            let run_id = run_id.clone();
            std::thread::spawn(move || runs.cancel(&run_id))
        };

        let deadline = std::time::Instant::now();
        while !cancelling.is_finished() {
            assert!(
                deadline.elapsed() < std::time::Duration::from_secs(5),
                "cancelling never returned — it queued behind the reader"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        assert!(cancelling.join().unwrap());
        assert!(
            stopped.load(std::sync::atomic::Ordering::SeqCst),
            "cancelling must reach the process, not just mark the run"
        );

        pumping.join().unwrap();
    }

    /// A sink for tests that only care that the loop finished.
    struct Silent;

    impl Sink for Silent {
        fn emit(&self, _: &Emission) {}
    }

    #[test]
    fn closing_the_application_stops_every_run_it_was_carrying() {
        // Runs are separate processes and outlive the window unless stopped —
        // measured: three runs killed with the window were still working
        // afterwards, spending tokens on answers nothing would read.
        let runs = Arc::new(Runs::new());
        let stopped = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        for index in 0..3 {
            let counted = Arc::clone(&stopped);
            runs.insert(
                format!("run-{index}"),
                format!("chat-{index}"),
                Arc::new(move || {
                    counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }),
                None,
            );
        }

        assert_eq!(runs.stop_all(), 3);
        assert_eq!(stopped.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[test]
    fn closing_with_nothing_running_stops_nothing() {
        assert_eq!(Runs::new().stop_all(), 0);
    }

    #[test]
    fn cancelling_something_that_is_not_running_is_not_an_error() {
        let runs = Arc::new(Runs::new());

        assert!(!runs.cancel("nothing"));
    }

    #[test]
    fn an_empty_registry_carries_nothing() {
        let runs = Runs::new();

        assert_eq!(runs.active(), 0);
        assert!(runs.active_chats().is_empty());
    }
}
