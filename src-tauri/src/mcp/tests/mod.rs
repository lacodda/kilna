//! What the tools answer and what a proposal leaves behind, called as a
//! client calls them: `run_tool` and `handle` against a workspace in memory.

use rusqlite::Connection;
use serde_json::{Map, Value, json};

use super::call::MOST;
use super::deliver::UNNAMED_CLIENT;
use super::schema::tools;
use super::{
    INVALID_PARAMS, METHOD_NOT_FOUND, PROTOCOL_VERSION, Session, SpaceInfo, handle, reads_a_space,
    registration_command, run_tool,
};
use crate::assistant::apply;
use crate::assistant::proposal::Proposal;
use crate::note::{self, NoteFilter};
use crate::speech::Speech;
use crate::work::version;
use crate::work::{self, WorkFilter};
use crate::{assistant, fixtures, profile, scene, score};

mod calendar;
mod canon;
mod conception;
mod cover;
mod deliver;
mod lab;
mod media;
mod notes;
mod protocol;
mod skills;
mod space;
mod storyboard;
mod styles;
mod works;

/// [`fixtures::workspace`], with a song and its lyrics already on it - most
/// tests here read the transcript or the board against a work in hand.
fn workspace() -> (Connection, String) {
    let (conn, profile_id) = fixtures::workspace();
    let work_id = fixtures::song(&conn, &profile_id, "Harbour lights").id;
    fixtures::version(&conn, &work_id, "lyrics", "one line\ntwo lines");
    (conn, work_id)
}

fn claude() -> Session {
    Session {
        client: Some("Claude Code".into()),
        space: None,
    }
}

/// A video in the workspace's profile — a kind with a storyboard.
fn video(conn: &Connection) -> String {
    let profile_id = profile::active(conn).unwrap().unwrap().id;
    fixtures::video(conn, &profile_id, "Harbour lights — the clip").id
}

fn first_message(conn: &Connection, work_id: &str) -> assistant::Message {
    let profile_id = profile::active(conn).unwrap().unwrap().id;
    let chats = assistant::summaries(conn, &profile_id, Some(work_id)).unwrap();
    assert_eq!(chats.len(), 1, "one chat, the client's");
    assistant::transcript(conn, &chats[0].id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0)
}

/// The first message of the one chat not on a work - where a proposal of
/// an agent that named no work waits.
fn first_message_anywhere(conn: &Connection) -> assistant::Message {
    let profile_id = profile::active(conn).unwrap().unwrap().id;
    let chats = assistant::summaries(conn, &profile_id, None).unwrap();
    let chat = chats.first().expect("a chat");
    assistant::transcript(conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0)
}

fn args(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// A tool's answer, read back as the JSON it is.
fn read(conn: &Connection, name: &str, with: Value) -> Value {
    serde_json::from_str(&run_tool(conn, &claude(), name, &args(with)).unwrap()).unwrap()
}
