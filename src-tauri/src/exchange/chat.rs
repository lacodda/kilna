//! A conversation as Markdown (v0.106).
//!
//! One writer for a chat going out of kilna, used twice: "Save as Markdown…"
//! on one chat, and the workspace's export, which carries every chat in
//! `chats/` beside the works (format 7). The export promised that nothing
//! stays locked in, and the conversations were the one thing it left behind.
//!
//! A page is front matter - what the chat is called, the work it is about,
//! when it began and was last used, what its answers cost - then each turn
//! under its heading, the question and the answer, a starred answer marked,
//! and under an answer what it proposed and what became of the proposal.
//! The words are the export's, in English, as every exported page's are.

use std::path::Path;

use rusqlite::Connection;
use serde_json::Value;

use crate::assistant::{self, ASSISTANT, USER};
use crate::error::{Error, Result};

use super::export::{push_field, slug};

/// What a chat's page is called: the chat's name, or its first question
/// cut to a caption, or "Chat" for a chat with neither.
pub fn title_of(transcript: &assistant::Transcript) -> String {
    if let Some(title) = transcript
        .chat
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
    {
        return title.to_owned();
    }
    transcript
        .messages
        .iter()
        .find(|message| message.role == USER)
        .map(|message| {
            let flat = message
                .body
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if flat.chars().count() > 80 {
                format!("{}…", flat.chars().take(80).collect::<String>())
            } else {
                flat
            }
        })
        .filter(|caption| !caption.is_empty())
        .unwrap_or_else(|| "Chat".to_owned())
}

/// A file name for a chat's page: its title made safe, with the chat's id
/// so two chats of one name do not overwrite each other.
pub fn file_name(transcript: &assistant::Transcript) -> String {
    format!("{}.md", slug(&title_of(transcript), &transcript.chat.id))
}

/// One chat, whole, as Markdown.
pub fn markdown(conn: &Connection, chat_id: &str) -> Result<String> {
    let transcript =
        assistant::transcript(conn, chat_id)?.ok_or_else(|| Error::not_found("chat", chat_id))?;
    let work_title = match &transcript.chat.work_id {
        Some(id) => crate::work::get(conn, id)?.map(|work| work.title),
        None => None,
    };
    Ok(page(&transcript, work_title.as_deref()))
}

/// The page of a transcript, the work's title given.
fn page(transcript: &assistant::Transcript, work_title: Option<&str>) -> String {
    let title = title_of(transcript);
    let cost: f64 = transcript
        .messages
        .iter()
        .filter_map(|message| message.meta.get("cost_usd").and_then(Value::as_f64))
        .sum();

    let mut out = String::from("---\n");
    out.push_str(&format!("format: {}\n", super::export::FORMAT));
    push_field(&mut out, "chat", &title);
    if let Some(work) = work_title {
        push_field(&mut out, "work", work);
    }
    push_field(&mut out, "created", &transcript.chat.created_at);
    push_field(&mut out, "updated", &transcript.chat.updated_at);
    if cost > 0.0 {
        out.push_str(&format!("cost_usd: {cost:.4}\n"));
    }
    out.push_str("---\n\n");
    out.push_str(&format!("# {title}\n"));

    for message in &transcript.messages {
        let at = moment(&message.created_at);
        if message.role == ASSISTANT {
            let mut heading = format!("## Answer · {at}");
            if message.starred_at.is_some() {
                heading.push_str(" · starred");
            }
            if let Some(client) = message.meta.get("client").and_then(Value::as_str) {
                heading.push_str(&format!(" · from {client}"));
            }
            out.push_str(&format!("\n{heading}\n\n{}\n", message.body.trim_end()));
            if let Some(line) = proposal_line(&message.meta) {
                out.push_str(&format!("\n_{line}_\n"));
            }
        } else {
            out.push_str(&format!(
                "\n## Question · {at}\n\n{}\n",
                message.body.trim_end()
            ));
        }
    }
    out
}

/// What an answer proposed and what became of it, in one line, when it
/// proposed anything.
fn proposal_line(meta: &serde_json::Map<String, Value>) -> Option<String> {
    let kind = meta
        .get("proposal")
        .and_then(|proposal| proposal.get("kind"))
        .and_then(Value::as_str)?;
    let fate = if meta.get("applied").is_some_and(Value::is_object) {
        "applied"
    } else if meta.get("dismissed").is_some_and(Value::is_string) {
        "dismissed"
    } else {
        "not applied"
    };
    Some(format!("Proposed: {kind} - {fate}"))
}

/// A stored moment as a person reads it in a file: the day and the minute,
/// in UTC as stored.
fn moment(stamp: &str) -> String {
    match (stamp.get(..10), stamp.get(11..16)) {
        (Some(day), Some(time)) => format!("{day} {time} UTC"),
        _ => stamp.to_owned(),
    }
}

/// Every chat of a profile, a page each, into `directory`. How many pages
/// were written.
pub fn write_all(conn: &Connection, profile_id: &str, directory: &Path) -> Result<usize> {
    let chats = assistant::summaries(conn, profile_id, None)?;
    if chats.is_empty() {
        return Ok(0);
    }
    std::fs::create_dir_all(directory)?;
    let mut written = 0;
    for summary in &chats {
        let Some(transcript) = assistant::transcript(conn, &summary.id)? else {
            continue;
        };
        std::fs::write(
            directory.join(file_name(&transcript)),
            page(&transcript, summary.work_title.as_deref()),
        )?;
        written += 1;
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::NewChat;
    use crate::fixtures;
    use serde_json::{Map, json};

    fn meta(value: Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap()
    }

    /// A chat goes out with its work, its turns in order, the starred answer
    /// marked, what an answer proposed and its fate, and what it cost.
    #[test]
    fn a_chat_reads_as_its_turns_with_what_they_proposed() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        let chat = assistant::create(
            &conn,
            &profile_id,
            NewChat {
                work_id: Some(song.id.clone()),
                ..NewChat::default()
            },
        )
        .unwrap();
        assistant::append(
            &conn,
            &chat.id,
            USER,
            "Shorten the second verse",
            Map::new(),
        )
        .unwrap();
        let answer = assistant::append(
            &conn,
            &chat.id,
            ASSISTANT,
            "Here it is, shorter.\n",
            meta(
                json!({ "cost_usd": 0.0125, "proposal": { "kind": "version", "role": "lyrics" } }),
            ),
        )
        .unwrap();
        assistant::star(&conn, &answer.id, true).unwrap();
        assistant::append(&conn, &chat.id, USER, "And the third?", Map::new()).unwrap();
        assistant::append(
            &conn,
            &chat.id,
            ASSISTANT,
            "Leave it.",
            meta(json!({ "proposal": { "kind": "note" }, "applied": { "at": "2026-10-31T10:00:00.000Z" } })),
        )
        .unwrap();

        let page = markdown(&conn, &chat.id).unwrap();

        assert!(page.starts_with("---\nformat: 7\n"), "{page}");
        assert!(
            page.contains("chat: \"Shorten the second verse\""),
            "{page}"
        );
        assert!(page.contains("work: \"Tide\""), "{page}");
        assert!(page.contains("cost_usd: 0.0125"), "{page}");
        assert!(page.contains("# Shorten the second verse\n"), "{page}");
        let first = page.find("## Question").unwrap();
        let starred = page.find(" · starred\n\nHere it is, shorter.\n").unwrap();
        let second = page.find("And the third?").unwrap();
        assert!(first < starred && starred < second, "{page}");
        assert!(page.contains("_Proposed: version - not applied_"), "{page}");
        assert!(page.contains("_Proposed: note - applied_"), "{page}");
    }

    #[test]
    fn a_missing_chat_is_not_found() {
        let (conn, _) = fixtures::workspace();
        assert!(markdown(&conn, "nope").is_err());
    }

    /// Every chat of the workspace goes out, a page each, named so two chats
    /// of one name keep their own pages.
    #[test]
    fn every_chat_goes_out_a_page_each() {
        let (conn, profile_id) = fixtures::workspace();
        for _ in 0..2 {
            let chat = assistant::create(
                &conn,
                &profile_id,
                NewChat {
                    title: Some("Same name".into()),
                    ..NewChat::default()
                },
            )
            .unwrap();
            assistant::append(&conn, &chat.id, USER, "hello", Map::new()).unwrap();
        }
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(write_all(&conn, &profile_id, dir.path()).unwrap(), 2);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }
}
