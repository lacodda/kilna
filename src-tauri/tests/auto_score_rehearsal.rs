//! A text scored by itself, start to end, against a CLI that is not Claude
//! Code (v0.170, FR-01).
//!
//! The parts were covered apart: who is scored, what the answer's block is
//! read as, how a proposal is kept as the assistant's. What was rehearsed by
//! hand once - and only with a CLI that fails - is the whole pass: a process
//! is spawned, its answer read off its output, kept as `auto`, and the
//! catalogue goes on reading the author's score. Here it is, with a stand-in
//! executable taken instead of the one on the PATH: one that answers with a
//! score block, and one that says nothing and fails.
//!
//! One test, because where the CLI is taken from is the process's: two
//! would race for it.

use std::path::Path;
use std::sync::Arc;

use kilna_lib::assistant::auto_score::{self, Not};
use kilna_lib::assistant::run::{self, Emission, Runs, Sink};
use kilna_lib::assistant::settings::AssistantSettings;
use kilna_lib::assistant::{self, cli, task};
use kilna_lib::score::{self, NewScore};
use kilna_lib::work::version::{self, NewVersion};
use kilna_lib::work::{self, NewWork, WorkPatch};
use kilna_lib::{actions, db, profile};
use rusqlite::Connection;

/// Nobody is listening: the window is not part of this.
struct Quiet;

impl Sink for Quiet {
    fn emit(&self, _: &Emission) {}
}

/// Put a stand-in for the CLI in `dir`: it reads the prompt to its end, then
/// prints `answer` - lines of `stream-json` - or, given none, fails.
fn stand_in(dir: &Path, answer: Option<&str>) {
    if let Some(answer) = answer {
        std::fs::write(dir.join("answer.jsonl"), answer).unwrap();
    }
    #[cfg(windows)]
    {
        let said = match answer {
            Some(_) => "type \"%~dp0answer.jsonl\"\r\nexit /b 0\r\n",
            None => "exit /b 1\r\n",
        };
        std::fs::write(
            dir.join("claude.cmd"),
            format!("@echo off\r\nfindstr \"^\" >nul\r\n{said}"),
        )
        .unwrap();
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let said = match answer {
            Some(_) => "cat \"$(dirname \"$0\")/answer.jsonl\"\n",
            None => "exit 1\n",
        };
        let path = dir.join("claude");
        std::fs::write(&path, format!("#!/bin/sh\ncat >/dev/null\n{said}")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// A song with a written text, far enough along to be scored by itself.
fn written(conn: &Connection, profile_id: &str, title: &str) -> work::Work {
    let song = work::create(
        conn,
        profile_id,
        NewWork {
            kind: "song".into(),
            title: title.into(),
            ..Default::default()
        },
    )
    .unwrap();
    version::create(
        conn,
        &song.id,
        NewVersion {
            role: "lyrics".into(),
            body: "The tide goes out past the harbour wall\nand the boats lie on their sides"
                .into(),
            make_current: true,
            ..NewVersion::default()
        },
    )
    .unwrap();
    work::update(
        conn,
        &song.id,
        WorkPatch {
            stage: Some(Some(50)),
            ..Default::default()
        },
    )
    .unwrap()
}

/// Score a work the way the window does when nobody asked: the task a click
/// starts, read to its end. Answers the chat it opened.
fn scored_alone(conn: &Connection, path: &Path, work_id: &str) -> String {
    let runs = Arc::new(Runs::new());
    let prepared =
        task::prepare(conn, work_id, auto_score::ACTION, task::About::default()).unwrap();
    let (started, stream) = run::start_as(
        conn,
        &AssistantSettings::default(),
        &runs,
        &prepared.chat_id,
        &prepared.prompt,
        None,
        Some(prepared.key),
        &prepared.attachments,
    )
    .unwrap();
    let sink: Arc<dyn Sink> = Arc::new(Quiet);
    let path = path.to_path_buf();
    run::pump(&runs, &sink, &started, &stream, move || {
        Connection::open(&path).ok()
    });
    prepared.chat_id
}

#[test]
fn a_text_is_scored_by_itself_from_the_process_to_the_catalogue() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workspace.db");
    let conn = db::open(&path).unwrap();
    profile::seed(&conn, profile::DEFAULT_KIND).unwrap();
    let active = profile::active(&conn).unwrap().unwrap();
    let (profile_id, config) = (active.id, active.config);
    let axis = config.vocabulary("song").axes[0].key.clone();
    let on = AssistantSettings {
        auto_score: true,
        ..AssistantSettings::default()
    };
    let standing = |work_id: &str| {
        let now = work::get(&conn, work_id).unwrap().unwrap();
        auto_score::standing(&conn, &config, &on, &now, None).unwrap()
    };

    // --- a CLI that answers ------------------------------------------------
    let answering = tempfile::tempdir().unwrap();
    let said = format!("It holds up.\n\n```json\n{{\"axes\": {{\"{axis}\": 4}}}}\n```");
    let lines = [
        serde_json::json!({"type": "system", "subtype": "init", "session_id": "rehearsal"}),
        serde_json::json!({
            "type": "result", "result": said, "total_cost_usd": 0.01, "duration_ms": 5
        }),
    ]
    .map(|line| line.to_string())
    .join("\n");
    stand_in(answering.path(), Some(&format!("{lines}\n")));
    cli::stand_in(Some(answering.path()));

    let song = written(&conn, &profile_id, "Tide");
    let text = song.current_version_id.clone().unwrap();
    // The author has judged this text themselves, and higher.
    let mut own = serde_json::Map::new();
    own.insert(axis.clone(), serde_json::json!(9));
    let authors = score::create(
        &conn,
        &song.id,
        NewScore {
            axes: own,
            version_id: Some(text.clone()),
            note: None,
            rater: None,
        },
    )
    .unwrap();

    let chat = scored_alone(&conn, &path, &song.id);
    assert!(
        !auto_score::forget_if_nothing_said(&conn, &chat).unwrap(),
        "a chat with an answer in it is kept"
    );
    let kept = actions::proposal::apply_pending_as(&conn, &chat, Some(auto_score::RATER)).unwrap();
    assert_eq!(kept.len(), 1, "the answer's block was read as one proposal");
    let opinion = score::get(&conn, kept[0].score.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(opinion.rater.as_deref(), Some(auto_score::RATER));
    assert_eq!(opinion.version_id.as_deref(), Some(text.as_str()));
    assert!(opinion.total < authors.total);

    // The catalogue still reads the author's score, later though the
    // opinion is.
    let row = score::catalogue(&conn, &profile_id)
        .unwrap()
        .into_iter()
        .find(|row| row.work_id == song.id)
        .unwrap();
    assert_eq!(row.total, Some(authors.total));

    // And the text is not asked about again - not for a typo either, in a
    // sitting that knows nothing of it - until it is really rewritten.
    assert!(standing(&song.id).is_err());
    let edit = |body: &str| {
        conn.execute(
            "UPDATE work_version SET body = ?2 WHERE id = ?1",
            rusqlite::params![text, body],
        )
        .unwrap();
        conn.execute(
            "UPDATE work SET updated_at = '2999-01-01T00:00:00.000Z' WHERE id = ?1",
            rusqlite::params![song.id],
        )
        .unwrap();
    };
    edit("The tide goes out past the harbour wall,\nand the boats lie on their sides");
    assert_eq!(standing(&song.id), Err(Not::BarelyChanged));
    edit("The tide goes out past the harbour wall\nand nobody is left to see it go");
    assert_eq!(standing(&song.id), Ok(text));

    // --- a CLI that fails without a word -----------------------------------
    let failing = tempfile::tempdir().unwrap();
    stand_in(failing.path(), None);
    cli::stand_in(Some(failing.path()));

    let other = written(&conn, &profile_id, "Ebb");
    let chat = scored_alone(&conn, &path, &other.id);
    let ended = run::list(&conn, &chat).unwrap();
    assert_eq!(ended.len(), 1);
    assert!(ended[0].state.is_over());
    assert!(
        auto_score::forget_if_nothing_said(&conn, &chat).unwrap(),
        "a run nobody asked for that said nothing leaves no chat"
    );
    assert!(assistant::get(&conn, &chat).unwrap().is_none());
    assert!(run::list(&conn, &chat).unwrap().is_empty());
    assert!(score::history(&conn, &other.id).unwrap().is_empty());
}
