pub mod apply;
pub mod cli;
pub mod prompt;
pub mod proposal;
pub mod queue;
pub mod run;
pub mod spend;
pub mod stream;
pub mod task;
pub mod waiting;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::time::now;

/// One conversation, optionally about a particular work.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Chat {
    pub id: String,
    pub profile_id: String,
    pub work_id: Option<String>,
    pub title: Option<String>,
    pub session_id: Option<String>,
    /// When a background task in this chat stopped to ask something, and the
    /// question is still unanswered. Null when nothing is pending.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_since: Option<String>,
    /// The profile action that opened this chat, when one did. Read on
    /// every turn for the action's method (ADR 0021).
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// The version the action was started on, when it was started on one
    /// rather than on the work's current version. What the chat proposes
    /// is about it, and applying binds to it.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Message {
    pub id: String,
    pub chat_id: String,
    pub role: String,
    pub body: String,
    pub meta: Map<String, Value>,
    pub created_at: String,
    /// When the person starred this answer to come back to it (v0.106).
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred_at: Option<String>,
}

/// A starred answer as the list of them draws it: what it says, cut to a
/// caption, and the chat and the work it was said in (v0.106).
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct StarredAnswer {
    pub message_id: String,
    pub chat_id: String,
    /// The chat's name, when it has one.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_title: Option<String>,
    /// The chat's first question, cut to a caption - the name of a chat
    /// nobody named, as the list of chats calls it.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_prompt: Option<String>,
    pub work_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_title: Option<String>,
    /// The answer flattened and cut: enough to tell one from the next.
    pub excerpt: String,
    pub created_at: String,
    pub starred_at: String,
}

/// A chat with everything said in it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Transcript {
    pub chat: Chat,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Default, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct NewChat {
    #[serde(default)]
    pub work_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
}

/// A chat as the list draws it: named, priced, tied to its work.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct ChatSummary {
    pub id: String,
    pub work_id: Option<String>,
    /// Title of the work the chat is about, so the list can say so.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_title: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The first thing asked, cut to a caption — the name of a chat nobody
    /// named.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_prompt: Option<String>,
    /// What the answers in this chat have cost so far, as the CLI reported
    /// it. Turns that died before reporting are not in the sum.
    pub cost_usd: f64,
    /// Set while this chat holds an unanswered question.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_since: Option<String>,
    pub updated_at: String,
}

pub const USER: &str = "user";
pub const ASSISTANT: &str = "assistant";

const SELECT_CHAT: &str = "SELECT id, profile_id, work_id, title, session_id, waiting_since, \
                            created_at, updated_at, action, version_id FROM chat";

pub fn create(conn: &Connection, profile_id: &str, new: NewChat) -> Result<Chat> {
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now();

    conn.execute(
        "INSERT INTO chat (id, profile_id, work_id, title, action, version_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            id,
            profile_id,
            new.work_id,
            new.title,
            new.action,
            new.version_id,
            timestamp
        ],
    )?;

    get(conn, &id)?.ok_or_else(|| Error::Internal("the chat vanished after insert".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Chat>> {
    Ok(conn
        .query_row(
            &format!("{SELECT_CHAT} WHERE id = ?1"),
            params![id],
            read_chat,
        )
        .optional()?)
}

/// Chats of a profile as the list shows them, most recently used first.
/// `work_id` narrows to one work.
pub fn summaries(
    conn: &Connection,
    profile_id: &str,
    work_id: Option<&str>,
) -> Result<Vec<ChatSummary>> {
    let mut sql = String::from(
        "SELECT c.id, c.work_id, w.title, c.title,
                (SELECT m.body FROM chat_message m
                 WHERE m.chat_id = c.id AND m.role = 'user'
                 ORDER BY m.created_at, m.rowid LIMIT 1),
                (SELECT coalesce(sum(r.cost_usd), 0)
                 FROM chat_run r WHERE r.chat_id = c.id),
                c.waiting_since,
                c.updated_at
         FROM chat c LEFT JOIN work w ON w.id = c.work_id
         WHERE c.profile_id = ?1",
    );
    if work_id.is_some() {
        sql.push_str(" AND c.work_id = ?2");
    }
    sql.push_str(" ORDER BY c.updated_at DESC, c.rowid DESC");

    let read = |row: &rusqlite::Row<'_>| {
        Ok(ChatSummary {
            id: row.get(0)?,
            work_id: row.get(1)?,
            work_title: row.get(2)?,
            title: row.get(3)?,
            first_prompt: row.get::<_, Option<String>>(4)?.map(|body| caption(&body)),
            cost_usd: row.get(5)?,
            waiting_since: row.get(6)?,
            updated_at: row.get(7)?,
        })
    };

    let mut statement = conn.prepare(&sql)?;
    let rows = match work_id {
        Some(work_id) => statement.query_map(params![profile_id, work_id], read)?,
        None => statement.query_map(params![profile_id], read)?,
    };

    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// A prompt flattened and cut to fit a list row. Cut by characters, not bytes:
/// a byte slice through a multibyte character panics.
fn caption(body: &str) -> String {
    cut_to(body, 80)
}

/// A body flattened and cut to `limit` characters, with an ellipsis when
/// anything was cut.
fn cut_to(body: &str, limit: usize) -> String {
    let flat: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= limit {
        return flat;
    }

    let cut: String = flat.chars().take(limit).collect();
    format!("{cut}…")
}

/// Star an answer, or take its star away (v0.106). Only an answer takes
/// one: a question is the person's own, and the list is of what was said
/// back. Starring a starred answer keeps its first moment - the list is in
/// the order things were starred, and a second click must not reorder it.
pub fn star(conn: &Connection, message_id: &str, starred: bool) -> Result<Option<String>> {
    let role: String = conn
        .query_row(
            "SELECT role FROM chat_message WHERE id = ?1",
            params![message_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| Error::not_found("message", message_id))?;
    if role != ASSISTANT {
        return Err(Error::refused("assistant.starOnlyAnswer"));
    }
    if starred {
        conn.execute(
            "UPDATE chat_message SET starred_at = ?2 WHERE id = ?1 AND starred_at IS NULL",
            params![message_id, now()],
        )?;
    } else {
        conn.execute(
            "UPDATE chat_message SET starred_at = NULL WHERE id = ?1",
            params![message_id],
        )?;
    }
    Ok(conn.query_row(
        "SELECT starred_at FROM chat_message WHERE id = ?1",
        params![message_id],
        |row| row.get(0),
    )?)
}

/// The starred answers of a profile, newest star first; `work_id` narrows to
/// the chats about one work.
pub fn starred(
    conn: &Connection,
    profile_id: &str,
    work_id: Option<&str>,
) -> Result<Vec<StarredAnswer>> {
    let mut statement = conn.prepare(
        "SELECT m.id, c.id, c.title,
                (SELECT q.body FROM chat_message q
                 WHERE q.chat_id = c.id AND q.role = 'user'
                 ORDER BY q.created_at, q.rowid LIMIT 1),
                c.work_id, w.title, m.body, m.created_at, m.starred_at
         FROM chat_message m
         JOIN chat c ON c.id = m.chat_id
         LEFT JOIN work w ON w.id = c.work_id
         WHERE m.starred_at IS NOT NULL AND c.profile_id = ?1
           AND (?2 IS NULL OR c.work_id = ?2)
         ORDER BY m.starred_at DESC, m.rowid DESC",
    )?;
    let rows = statement.query_map(params![profile_id, work_id], |row| {
        Ok(StarredAnswer {
            message_id: row.get(0)?,
            chat_id: row.get(1)?,
            chat_title: row.get(2)?,
            first_prompt: row.get::<_, Option<String>>(3)?.map(|body| caption(&body)),
            work_id: row.get(4)?,
            work_title: row.get(5)?,
            excerpt: cut_to(&row.get::<_, String>(6)?, 160),
            created_at: row.get(7)?,
            starred_at: row.get(8)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Name a chat, or clear the name so it borrows its first question again.
///
/// `updated_at` is left alone on purpose: the list orders by use, and
/// renaming is housekeeping, not use.
pub fn rename(conn: &Connection, id: &str, title: Option<&str>) -> Result<()> {
    let title = title.map(str::trim).filter(|title| !title.is_empty());
    if conn.execute(
        "UPDATE chat SET title = ?2 WHERE id = ?1",
        params![id, title],
    )? == 0
    {
        return Err(Error::not_found("chat", id));
    }
    Ok(())
}

pub fn transcript(conn: &Connection, chat_id: &str) -> Result<Option<Transcript>> {
    let Some(chat) = get(conn, chat_id)? else {
        return Ok(None);
    };

    let mut statement = conn.prepare(&format!(
        "{SELECT_MESSAGE} WHERE chat_id = ?1 ORDER BY created_at, rowid"
    ))?;
    let raw = statement
        .query_map(params![chat_id], read_message)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let messages = raw
        .into_iter()
        .map(RawMessage::into_message)
        .collect::<Result<Vec<_>>>()?;

    Ok(Some(Transcript { chat, messages }))
}

const SELECT_MESSAGE: &str =
    "SELECT id, chat_id, role, body, meta, created_at, starred_at FROM chat_message";

/// A message as it comes out of the row, its meta still text.
struct RawMessage {
    id: String,
    chat_id: String,
    role: String,
    body: String,
    meta: String,
    created_at: String,
    starred_at: Option<String>,
}

impl RawMessage {
    fn into_message(self) -> Result<Message> {
        Ok(Message {
            meta: meta_of(&self.meta)?,
            id: self.id,
            chat_id: self.chat_id,
            role: self.role,
            body: self.body,
            created_at: self.created_at,
            starred_at: self.starred_at,
        })
    }
}

fn read_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawMessage> {
    Ok(RawMessage {
        id: row.get(0)?,
        chat_id: row.get(1)?,
        role: row.get(2)?,
        body: row.get(3)?,
        meta: row.get(4)?,
        created_at: row.get(5)?,
        starred_at: row.get(6)?,
    })
}

/// One message by id.
pub fn message(conn: &Connection, id: &str) -> Result<Option<Message>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_MESSAGE} WHERE id = ?1"),
            params![id],
            read_message,
        )
        .optional()?;
    raw.map(RawMessage::into_message).transpose()
}

/// A message's meta as it is read back.
///
/// A proposal is stored as it was proposed, and the shapes grow: a canon
/// package gained `pictures` and `descriptions` in v0.89.2, while packages
/// proposed before that still waited in the owner's chat. Read raw, such a
/// package lacked both lists, and the whole chat it stood in stopped
/// rendering - the experiments proposed after it with it (v0.95.2). So a
/// proposal is read as it was meant - [`proposal::read_stored`], the reader
/// applying uses too - and goes out in today's shape, what was added since
/// taking its default and what was renamed put back; one that cannot be
/// read is passed on as it is, for the window's own checks to turn down.
fn meta_of(raw: &str) -> Result<Map<String, Value>> {
    let mut meta: Map<String, Value> = serde_json::from_str(raw)?;
    if let Some(stored) = meta.get_mut("proposal")
        && let Ok(proposal) = proposal::read_stored(stored.clone())
    {
        *stored = serde_json::to_value(proposal)?;
    }
    Ok(meta)
}

/// Replace a message's meta — how a proposal is marked applied. The body
/// and the moment stay: what was said is not what changed.
pub fn set_meta(conn: &Connection, id: &str, meta: &Map<String, Value>) -> Result<()> {
    if conn.execute(
        "UPDATE chat_message SET meta = ?2 WHERE id = ?1",
        params![id, Value::Object(meta.clone()).to_string()],
    )? == 0
    {
        return Err(Error::not_found("message", id));
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    if conn.execute("DELETE FROM chat WHERE id = ?1", params![id])? == 0 {
        return Err(Error::not_found("chat", id));
    }
    Ok(())
}

/// Mark a chat as holding an unanswered question.
///
/// Idempotent by design: re-marking a chat that is already waiting keeps the
/// original moment. The banner says how long something has been sitting there,
/// and a second task landing in the same chat must not make an old question
/// look new.
pub fn mark_waiting(conn: &Connection, chat_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE chat SET waiting_since = ?2 WHERE id = ?1 AND waiting_since IS NULL",
        params![chat_id, now()],
    )?;
    Ok(())
}

/// Clear the question — answered, or dismissed by hand.
///
/// Not an error when nothing was waiting: the button that clears it can be
/// clicked twice, and two people looking at the same banner is not a failure.
pub fn clear_waiting(conn: &Connection, chat_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE chat SET waiting_since = NULL WHERE id = ?1",
        params![chat_id],
    )?;
    Ok(())
}

/// Chats of a profile with an unanswered question, oldest first.
///
/// Oldest first because that is the order they should be dealt with: the
/// question that has been sitting longest is the one holding work up.
pub fn waiting(conn: &Connection, profile_id: &str) -> Result<Vec<ChatSummary>> {
    let mut waiting: Vec<ChatSummary> = summaries(conn, profile_id, None)?
        .into_iter()
        .filter(|summary| summary.waiting_since.is_some())
        .collect();

    // By when the question was asked, not by when the chat was last touched:
    // `summaries` orders by use, and a question's age is what the banner is
    // about.
    waiting.sort_by(|left, right| left.waiting_since.cmp(&right.waiting_since));
    Ok(waiting)
}

/// Record a message without calling the CLI.
pub fn append(
    conn: &Connection,
    chat_id: &str,
    role: &str,
    body: &str,
    meta: Map<String, Value>,
) -> Result<Message> {
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now();

    conn.execute(
        "INSERT INTO chat_message (id, chat_id, role, body, meta, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            chat_id,
            role,
            body,
            Value::Object(meta.clone()).to_string(),
            timestamp,
        ],
    )?;

    conn.execute(
        "UPDATE chat SET updated_at = ?2 WHERE id = ?1",
        params![chat_id, timestamp],
    )?;

    Ok(Message {
        id,
        chat_id: chat_id.to_owned(),
        role: role.to_owned(),
        body: body.to_owned(),
        meta,
        created_at: timestamp,
        starred_at: None,
    })
}

fn read_chat(row: &rusqlite::Row<'_>) -> rusqlite::Result<Chat> {
    Ok(Chat {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        work_id: row.get(2)?,
        title: row.get(3)?,
        session_id: row.get(4)?,
        waiting_since: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        action: row.get(8)?,
        version_id: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use serde_json::json;

    use crate::work::{self, NewWork};

    #[test]
    fn a_new_chat_has_no_session_until_a_turn_happens() {
        let (conn, profile_id) = fixtures::workspace();

        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: Some("Scratch".into()),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(chat.session_id.is_none());
        assert_eq!(
            transcript(&conn, &chat.id).unwrap().unwrap().messages.len(),
            0
        );
    }

    #[test]
    fn a_proposal_stored_in_an_older_shape_reads_back_in_todays() {
        let (conn, profile_id) = fixtures::workspace();
        let chat = create(&conn, &profile_id, NewChat::default()).unwrap();
        // A canon package as proposed before v0.89.2: no pictures, no
        // descriptions.
        let older = json!({
            "proposal": {
                "kind": "canon",
                "package": { "cards": [], "facts": [], "links": [], "dropped": [] }
            },
            "source": "mcp"
        });
        let unknown = json!({ "proposal": { "kind": "somethingLater", "items": 3 } });
        // A storyboard as v0.62 stored it: the window is told it replaces.
        let renamed = json!({
            "proposal": { "kind": "scenes", "replace": true, "scenes": [] }
        });
        for meta in [&older, &unknown, &renamed] {
            append(
                &conn,
                &chat.id,
                ASSISTANT,
                "proposed",
                meta.as_object().unwrap().clone(),
            )
            .unwrap();
        }

        let messages = transcript(&conn, &chat.id).unwrap().unwrap().messages;
        let package = &messages[0].meta["proposal"]["package"];
        assert_eq!(package["pictures"], json!([]));
        assert_eq!(package["descriptions"], json!([]));
        assert_eq!(
            messages[0].meta["source"], "mcp",
            "the rest of the meta stays"
        );
        assert_eq!(
            message(&conn, &messages[0].id).unwrap().unwrap().meta["proposal"]["package"]["pictures"],
            json!([]),
            "one message read alone reads the same"
        );
        assert_eq!(
            messages[1].meta,
            *unknown.as_object().unwrap(),
            "a proposal today's type cannot read is passed on as it is"
        );
        assert_eq!(messages[2].meta["proposal"]["change"], "replace");
    }

    #[test]
    fn messages_come_back_in_the_order_they_were_said() {
        let (conn, profile_id) = fixtures::workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
                ..Default::default()
            },
        )
        .unwrap();

        append(&conn, &chat.id, USER, "first", Map::new()).unwrap();
        append(&conn, &chat.id, ASSISTANT, "second", Map::new()).unwrap();
        append(&conn, &chat.id, USER, "third", Map::new()).unwrap();

        let transcript = transcript(&conn, &chat.id).unwrap().unwrap();

        let bodies: Vec<_> = transcript
            .messages
            .iter()
            .map(|m| m.body.as_str())
            .collect();
        assert_eq!(bodies, vec!["first", "second", "third"]);
    }

    /// Messages written inside the same second keep the order they were said.
    ///
    /// The timestamp is RFC3339 with second resolution, so three quick turns
    /// can share one. The tie-break used to be the message id — a random uuid,
    /// which is no order at all: the transcript came back shuffled. macOS in CI
    /// was fast enough to hit it while every other runner was not, which is the
    /// only reason it surfaced at all. `rowid` is the order of insertion, and
    /// SQLite keeps it for us.
    #[test]
    fn messages_sharing_a_timestamp_keep_the_order_they_were_written() {
        let (conn, profile_id) = fixtures::workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
                ..Default::default()
            },
        )
        .unwrap();

        // Written straight to the table: `append` stamps its own time, and the
        // collision this guards against cannot be arranged through it.
        for body in ["first", "second", "third"] {
            conn.execute(
                "INSERT INTO chat_message (id, chat_id, role, body, meta, created_at)
                 VALUES (?1, ?2, 'user', ?3, '{}', '2026-08-28T08:48:32Z')",
                rusqlite::params![uuid::Uuid::new_v4().to_string(), chat.id, body],
            )
            .unwrap();
        }

        let transcript = transcript(&conn, &chat.id).unwrap().unwrap();
        let bodies: Vec<_> = transcript
            .messages
            .iter()
            .map(|m| m.body.as_str())
            .collect();

        assert_eq!(
            bodies,
            vec!["first", "second", "third"],
            "a shared timestamp must fall back to insertion order, not to a random id"
        );
    }

    #[test]
    fn deleting_a_work_takes_its_chats_with_it() {
        let (conn, profile_id) = fixtures::workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Doomed".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: Some(work.id.clone()),
                title: None,
                ..Default::default()
            },
        )
        .unwrap();

        work::delete(&conn, &work.id).unwrap();

        assert!(get(&conn, &chat.id).unwrap().is_none());
    }

    #[test]
    fn deleting_a_chat_takes_its_messages_with_it() {
        let (conn, profile_id) = fixtures::workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
                ..Default::default()
            },
        )
        .unwrap();
        append(&conn, &chat.id, USER, "something", Map::new()).unwrap();

        delete(&conn, &chat.id).unwrap();

        let orphans: i64 = conn
            .query_row("SELECT count(*) FROM chat_message", [], |row| row.get(0))
            .unwrap();
        assert_eq!(orphans, 0);
    }

    #[test]
    fn the_schema_refuses_an_invented_role() {
        let (conn, profile_id) = fixtures::workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(append(&conn, &chat.id, "narrator", "hello", Map::new()).is_err());
    }

    /// The price of a chat is what its runs cost - the failed ones too, which
    /// wrote no answer to carry a price (v0.107).
    #[test]
    fn a_summary_prices_the_chat_from_what_its_runs_cost() {
        let (conn, profile_id) = fixtures::workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
                ..Default::default()
            },
        )
        .unwrap();

        for (state, cost) in [
            ("done", Some(0.12)),
            ("failed", Some(0.25)),
            ("cancelled", None),
        ] {
            conn.execute(
                "INSERT INTO chat_run (id, chat_id, prompt, state, events, started_at, cost_usd)
                 VALUES (?1, ?2, 'q', ?3, '[]', '2026-11-01T00:00:00.000Z', ?4)",
                rusqlite::params![uuid::Uuid::new_v4().to_string(), chat.id, state, cost],
            )
            .unwrap();
        }
        // A price left in an answer's meta is not read: the run's is the one.
        let mut stray = Map::new();
        stray.insert("cost_usd".into(), json!(9.0));
        append(&conn, &chat.id, ASSISTANT, "old", stray).unwrap();

        let summary = &summaries(&conn, &profile_id, None).unwrap()[0];
        assert!(
            (summary.cost_usd - 0.37).abs() < 1e-9,
            "{}",
            summary.cost_usd
        );
    }

    #[test]
    fn an_unnamed_chat_borrows_its_first_question() {
        let (conn, profile_id) = fixtures::workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
                ..Default::default()
            },
        )
        .unwrap();
        append(&conn, &chat.id, ASSISTANT, "a greeting", Map::new()).unwrap();
        append(
            &conn,
            &chat.id,
            USER,
            "shorten   the\nsecond verse",
            Map::new(),
        )
        .unwrap();
        append(&conn, &chat.id, USER, "and the third", Map::new()).unwrap();

        let summary = &summaries(&conn, &profile_id, None).unwrap()[0];

        assert_eq!(summary.title, None);
        assert_eq!(
            summary.first_prompt.as_deref(),
            Some("shorten the second verse"),
            "the first *question*, flattened — not whatever spoke first"
        );
    }

    #[test]
    fn a_long_first_question_is_cut_by_characters_not_bytes() {
        let (conn, profile_id) = fixtures::workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
                ..Default::default()
            },
        )
        .unwrap();
        append(&conn, &chat.id, USER, &"é".repeat(100), Map::new()).unwrap();

        let summary = &summaries(&conn, &profile_id, None).unwrap()[0];

        let caption = summary.first_prompt.as_deref().unwrap();
        assert_eq!(caption.chars().count(), 81, "80 kept plus the ellipsis");
        assert!(caption.ends_with('…'));
    }

    #[test]
    fn summaries_narrow_to_a_work_and_name_it() {
        let (conn, profile_id) = fixtures::workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Subject".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        create(
            &conn,
            &profile_id,
            NewChat {
                work_id: Some(work.id.clone()),
                title: None,
                ..Default::default()
            },
        )
        .unwrap();
        create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
                ..Default::default()
            },
        )
        .unwrap();

        let narrowed = summaries(&conn, &profile_id, Some(&work.id)).unwrap();
        assert_eq!(narrowed.len(), 1);
        assert_eq!(narrowed[0].work_title.as_deref(), Some("Subject"));

        assert_eq!(summaries(&conn, &profile_id, None).unwrap().len(), 2);
    }

    #[test]
    fn renaming_a_chat_does_not_reorder_the_list() {
        let (conn, profile_id) = fixtures::workspace();
        let older = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
                ..Default::default()
            },
        )
        .unwrap();
        // Same-instant chats order by nothing useful; make the second later.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let newer = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
                ..Default::default()
            },
        )
        .unwrap();

        rename(&conn, &older.id, Some("Named")).unwrap();

        let ids: Vec<_> = summaries(&conn, &profile_id, None)
            .unwrap()
            .into_iter()
            .map(|summary| summary.id)
            .collect();
        assert_eq!(
            ids,
            vec![newer.id, older.id.clone()],
            "renaming is housekeeping, not use"
        );
        assert_eq!(
            get(&conn, &older.id).unwrap().unwrap().title.as_deref(),
            Some("Named")
        );
    }

    #[test]
    fn a_blank_rename_clears_the_title() {
        let (conn, profile_id) = fixtures::workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: Some("Named".into()),
                ..Default::default()
            },
        )
        .unwrap();

        rename(&conn, &chat.id, Some("   ")).unwrap();

        assert_eq!(get(&conn, &chat.id).unwrap().unwrap().title, None);
    }

    #[test]
    fn renaming_a_missing_chat_fails() {
        let (conn, _) = fixtures::workspace();

        assert!(rename(&conn, "nope", Some("Named")).is_err());
    }

    #[test]
    fn marking_a_waiting_chat_twice_keeps_the_first_moment() {
        let (conn, profile_id) = fixtures::workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
                ..Default::default()
            },
        )
        .unwrap();

        mark_waiting(&conn, &chat.id).unwrap();
        let first = get(&conn, &chat.id).unwrap().unwrap().waiting_since;
        std::thread::sleep(std::time::Duration::from_millis(1100));
        mark_waiting(&conn, &chat.id).unwrap();

        assert_eq!(
            get(&conn, &chat.id).unwrap().unwrap().waiting_since,
            first,
            "a second question must not make an old one look new"
        );
    }

    #[test]
    fn clearing_a_chat_that_was_not_waiting_is_not_an_error() {
        let (conn, profile_id) = fixtures::workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(clear_waiting(&conn, &chat.id).is_ok());
        assert!(
            get(&conn, &chat.id)
                .unwrap()
                .unwrap()
                .waiting_since
                .is_none()
        );
    }

    #[test]
    fn waiting_chats_come_back_oldest_question_first() {
        let (conn, profile_id) = fixtures::workspace();
        let older = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: Some("Asked first".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let newer = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: Some("Asked later".into()),
                ..Default::default()
            },
        )
        .unwrap();

        mark_waiting(&conn, &older.id).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        mark_waiting(&conn, &newer.id).unwrap();
        // Touching the newer chat makes it the most recently used, which is the
        // order `summaries` would give. The question's age is what matters.
        append(&conn, &newer.id, USER, "poke", Map::new()).unwrap();

        let ids: Vec<_> = waiting(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .map(|summary| summary.id)
            .collect();

        assert_eq!(
            ids,
            vec![older.id, newer.id],
            "the question that has waited longest is the one holding work up"
        );
    }

    /// A star goes on an answer and comes off it; a second star keeps the
    /// first moment, and a question takes none.
    #[test]
    fn an_answer_is_starred_and_unstarred_and_a_question_is_not() {
        let (conn, profile_id) = fixtures::workspace();
        let chat = create(&conn, &profile_id, NewChat::default()).unwrap();
        let question = append(&conn, &chat.id, USER, "why", Map::new()).unwrap();
        let answer = append(&conn, &chat.id, ASSISTANT, "because", Map::new()).unwrap();

        let first = star(&conn, &answer.id, true).unwrap();
        assert!(first.is_some());
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert_eq!(
            star(&conn, &answer.id, true).unwrap(),
            first,
            "a second star must not move it up the list"
        );
        assert_eq!(
            message(&conn, &answer.id).unwrap().unwrap().starred_at,
            first
        );

        let refused = star(&conn, &question.id, true).unwrap_err();
        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("assistant.starOnlyAnswer")
        );
        assert!(star(&conn, "nope", true).is_err());

        assert_eq!(star(&conn, &answer.id, false).unwrap(), None);
        assert!(starred(&conn, &profile_id, None).unwrap().is_empty());
    }

    /// The starred answers come newest star first, with their chat's name
    /// and work; a work narrows them to its own chats.
    #[test]
    fn starred_answers_are_listed_newest_first_and_narrow_to_a_work() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        let about_song = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: Some(song.id.clone()),
                ..NewChat::default()
            },
        )
        .unwrap();
        let loose = create(&conn, &profile_id, NewChat::default()).unwrap();
        append(&conn, &about_song.id, USER, "the chorus?", Map::new()).unwrap();
        let older = append(
            &conn,
            &about_song.id,
            ASSISTANT,
            &"long ".repeat(60),
            Map::new(),
        )
        .unwrap();
        let newer = append(&conn, &loose.id, ASSISTANT, "a loose thought", Map::new()).unwrap();
        star(&conn, &older.id, true).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        star(&conn, &newer.id, true).unwrap();

        let all = starred(&conn, &profile_id, None).unwrap();
        let ids: Vec<_> = all.iter().map(|a| a.message_id.as_str()).collect();
        assert_eq!(ids, vec![newer.id.as_str(), older.id.as_str()]);
        assert_eq!(all[1].work_title.as_deref(), Some("Tide"));
        assert_eq!(all[1].first_prompt.as_deref(), Some("the chorus?"));
        assert!(all[1].excerpt.ends_with('…'), "{}", all[1].excerpt);
        assert_eq!(all[1].excerpt.chars().count(), 161);

        let narrowed = starred(&conn, &profile_id, Some(&song.id)).unwrap();
        assert_eq!(narrowed.len(), 1);
        assert_eq!(narrowed[0].message_id, older.id);
    }

    #[test]
    fn a_chat_with_nothing_pending_is_not_listed_as_waiting() {
        let (conn, profile_id) = fixtures::workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
                ..Default::default()
            },
        )
        .unwrap();
        mark_waiting(&conn, &chat.id).unwrap();
        clear_waiting(&conn, &chat.id).unwrap();

        assert!(waiting(&conn, &profile_id).unwrap().is_empty());
    }
}
