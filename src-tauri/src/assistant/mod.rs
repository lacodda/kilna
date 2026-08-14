pub mod cli;
pub mod prompt;

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

pub const USER: &str = "user";
pub const ASSISTANT: &str = "assistant";

const SELECT_CHAT: &str =
    "SELECT id, profile_id, work_id, title, session_id, created_at, updated_at FROM chat";

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

/// Chats in a profile, most recently used first. `work_id` narrows to one work.
pub fn list(conn: &Connection, profile_id: &str, work_id: Option<&str>) -> Result<Vec<Chat>> {
    let mut sql = format!("{SELECT_CHAT} WHERE profile_id = ?1");
    if work_id.is_some() {
        sql.push_str(" AND work_id = ?2");
    }
    sql.push_str(" ORDER BY updated_at DESC");

    let mut statement = conn.prepare(&sql)?;
    let rows = match work_id {
        Some(work_id) => statement.query_map(params![profile_id, work_id], read_chat)?,
        None => statement.query_map(params![profile_id], read_chat)?,
    };

    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
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
pub fn ask(conn: &mut Connection, chat_id: &str, prompt: &str) -> Result<Message> {
    let chat = get(conn, chat_id)?.ok_or_else(|| Error::not_found("chat", chat_id))?;

    append(conn, chat_id, USER, prompt, Map::new())?;

    let turn = cli::ask(prompt, chat.session_id.as_deref())?;

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
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
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
    fn a_chat_can_be_attached_to_a_work_and_listed_by_it() {
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

        assert_eq!(list(&conn, &profile_id, Some(&work.id)).unwrap().len(), 1);
        assert_eq!(list(&conn, &profile_id, None).unwrap().len(), 2);
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
    fn asking_in_an_unknown_chat_fails_before_the_cli_is_called() {
        let (mut conn, _) = workspace();

        assert!(ask(&mut conn, "nope", "hello").is_err());
    }
}
