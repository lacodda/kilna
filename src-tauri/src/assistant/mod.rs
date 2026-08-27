pub mod cli;
pub mod prompt;
pub mod proposal;
pub mod run;
pub mod stream;
pub mod task;
pub mod waiting;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::error::{Error, Result};
use crate::time::now;

/// One conversation, optionally about a particular work.
#[derive(Debug, Clone, Serialize)]
pub struct Chat {
    pub id: String,
    pub profile_id: String,
    pub work_id: Option<String>,
    pub title: Option<String>,
    pub session_id: Option<String>,
    /// When a background task in this chat stopped to ask something, and the
    /// question is still unanswered. Null when nothing is pending.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_since: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub id: String,
    pub chat_id: String,
    pub role: String,
    pub body: String,
    pub meta: Map<String, Value>,
    pub created_at: String,
}

/// A chat with everything said in it.
#[derive(Debug, Clone, Serialize)]
pub struct Transcript {
    pub chat: Chat,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewChat {
    #[serde(default)]
    pub work_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

/// A chat as the list draws it: named, priced, tied to its work.
#[derive(Debug, Clone, Serialize)]
pub struct ChatSummary {
    pub id: String,
    pub work_id: Option<String>,
    /// Title of the work the chat is about, so the list can say so.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The first thing asked, cut to a caption — the name of a chat nobody
    /// named.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_prompt: Option<String>,
    /// What the answers in this chat have cost so far, as the CLI reported
    /// it. Turns that died before reporting are not in the sum.
    pub cost_usd: f64,
    /// Set while this chat holds an unanswered question.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_since: Option<String>,
    pub updated_at: String,
}

pub const USER: &str = "user";
pub const ASSISTANT: &str = "assistant";

const SELECT_CHAT: &str = "SELECT id, profile_id, work_id, title, session_id, waiting_since,                            created_at, updated_at FROM chat";

pub fn create(conn: &Connection, profile_id: &str, new: NewChat) -> Result<Chat> {
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now();

    conn.execute(
        "INSERT INTO chat (id, profile_id, work_id, title, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![id, profile_id, new.work_id, new.title, timestamp],
    )?;

    get(conn, &id)?.ok_or_else(|| Error::Other("the chat vanished after insert".into()))
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
                 ORDER BY m.created_at, m.id LIMIT 1),
                (SELECT coalesce(sum(json_extract(m.meta, '$.cost_usd')), 0)
                 FROM chat_message m WHERE m.chat_id = c.id),
                c.waiting_since,
                c.updated_at
         FROM chat c LEFT JOIN work w ON w.id = c.work_id
         WHERE c.profile_id = ?1",
    );
    if work_id.is_some() {
        sql.push_str(" AND c.work_id = ?2");
    }
    sql.push_str(" ORDER BY c.updated_at DESC");

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
    const LIMIT: usize = 80;

    let flat: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= LIMIT {
        return flat;
    }

    let cut: String = flat.chars().take(LIMIT).collect();
    format!("{cut}…")
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

    let mut statement = conn.prepare(
        "SELECT id, chat_id, role, body, meta, created_at
         FROM chat_message WHERE chat_id = ?1 ORDER BY created_at, id",
    )?;
    let raw = statement
        .query_map(params![chat_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let messages = raw
        .into_iter()
        .map(|(id, chat_id, role, body, meta, created_at)| {
            Ok(Message {
                meta: serde_json::from_str(&meta)?,
                id,
                chat_id,
                role,
                body,
                created_at,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Some(Transcript { chat, messages }))
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    if conn.execute("DELETE FROM chat WHERE id = ?1", params![id])? == 0 {
        return Err(Error::not_found("chat", id));
    }
    Ok(())
}

/// Send a prompt in a chat and record both sides of the exchange.
///
/// The user's message is stored before the CLI is called, so a failed or slow
/// turn still leaves a record of what was asked.
pub fn ask(
    conn: &mut Connection,
    chat_id: &str,
    prompt: &str,
    workdir: Option<&std::path::Path>,
) -> Result<Message> {
    let chat = get(conn, chat_id)?.ok_or_else(|| Error::not_found("chat", chat_id))?;

    append(conn, chat_id, USER, prompt, Map::new())?;

    let turn = cli::ask(prompt, chat.session_id.as_deref(), workdir)?;

    let mut meta = Map::new();
    if let Some(cost) = turn.cost_usd {
        meta.insert("cost_usd".into(), json!(cost));
    }
    if let Some(duration) = turn.duration_ms {
        meta.insert("duration_ms".into(), json!(duration));
    }

    let message = append(conn, chat_id, ASSISTANT, &turn.body, meta)?;

    // Keep the session id so the next turn continues this conversation.
    if let Some(session_id) = turn.session_id {
        conn.execute(
            "UPDATE chat SET session_id = ?2, updated_at = ?3 WHERE id = ?1",
            params![chat_id, session_id, now()],
        )?;
    }

    Ok(message)
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::work::{self, NewWork};

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    #[test]
    fn a_new_chat_has_no_session_until_a_turn_happens() {
        let (conn, profile_id) = workspace();

        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: Some("Scratch".into()),
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
    fn messages_come_back_in_the_order_they_were_said() {
        let (conn, profile_id) = workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
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

    #[test]
    fn deleting_a_work_takes_its_chats_with_it() {
        let (conn, profile_id) = workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Doomed".into(),
                status: None,
                collection_id: None,
                meta: None,
            },
        )
        .unwrap();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: Some(work.id.clone()),
                title: None,
            },
        )
        .unwrap();

        work::delete(&conn, &work.id).unwrap();

        assert!(get(&conn, &chat.id).unwrap().is_none());
    }

    #[test]
    fn deleting_a_chat_takes_its_messages_with_it() {
        let (conn, profile_id) = workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
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
        let (conn, profile_id) = workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
            },
        )
        .unwrap();

        assert!(append(&conn, &chat.id, "narrator", "hello", Map::new()).is_err());
    }

    #[test]
    fn a_summary_prices_the_chat_from_what_its_answers_cost() {
        let (conn, profile_id) = workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
            },
        )
        .unwrap();

        let mut priced = Map::new();
        priced.insert("cost_usd".into(), json!(0.12));
        append(&conn, &chat.id, ASSISTANT, "one", priced).unwrap();
        let mut priced = Map::new();
        priced.insert("cost_usd".into(), json!(0.25));
        append(&conn, &chat.id, ASSISTANT, "two", priced).unwrap();
        // A turn that died before reporting carries no cost, and must not
        // break the sum.
        append(&conn, &chat.id, USER, "free", Map::new()).unwrap();

        let summary = &summaries(&conn, &profile_id, None).unwrap()[0];
        assert!(
            (summary.cost_usd - 0.37).abs() < 1e-9,
            "{}",
            summary.cost_usd
        );
    }

    #[test]
    fn an_unnamed_chat_borrows_its_first_question() {
        let (conn, profile_id) = workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
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
        let (conn, profile_id) = workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
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
        let (conn, profile_id) = workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Subject".into(),
                status: None,
                collection_id: None,
                meta: None,
            },
        )
        .unwrap();
        create(
            &conn,
            &profile_id,
            NewChat {
                work_id: Some(work.id.clone()),
                title: None,
            },
        )
        .unwrap();
        create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
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
        let (conn, profile_id) = workspace();
        let older = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
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
        let (conn, profile_id) = workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: Some("Named".into()),
            },
        )
        .unwrap();

        rename(&conn, &chat.id, Some("   ")).unwrap();

        assert_eq!(get(&conn, &chat.id).unwrap().unwrap().title, None);
    }

    #[test]
    fn renaming_a_missing_chat_fails() {
        let (conn, _) = workspace();

        assert!(rename(&conn, "nope", Some("Named")).is_err());
    }

    #[test]
    fn marking_a_waiting_chat_twice_keeps_the_first_moment() {
        let (conn, profile_id) = workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
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
        let (conn, profile_id) = workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
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
        let (conn, profile_id) = workspace();
        let older = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: Some("Asked first".into()),
            },
        )
        .unwrap();
        let newer = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: Some("Asked later".into()),
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

    #[test]
    fn a_chat_with_nothing_pending_is_not_listed_as_waiting() {
        let (conn, profile_id) = workspace();
        let chat = create(
            &conn,
            &profile_id,
            NewChat {
                work_id: None,
                title: None,
            },
        )
        .unwrap();
        mark_waiting(&conn, &chat.id).unwrap();
        clear_waiting(&conn, &chat.id).unwrap();

        assert!(waiting(&conn, &profile_id).unwrap().is_empty());
    }

    #[test]
    fn asking_in_an_unknown_chat_fails_before_the_cli_is_called() {
        let (mut conn, _) = workspace();

        assert!(ask(&mut conn, "nope", "hello", None).is_err());
    }
}
