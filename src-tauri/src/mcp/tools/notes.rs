//! The two shelves of material for works to come: phrases and notes.

use rusqlite::Connection;
use serde_json::{Value, json};

use crate::assistant::proposal::Proposal;
use crate::error::{Error, Result};
use crate::mcp::call::{Call, Page, arg, find_work, length_of, pretty, required};
use crate::mcp::deliver::deliver;
use crate::note::{self, NoteFilter};
use crate::profile;

/// `notes`: the notes and phrases, a page at a time, or one of them whole.
pub(super) fn notes(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        config,
        args,
        ..
    } = *call;
    let work_id = match arg(args, "work") {
        Some(named) => Some(find_work(conn, &profile.id, named)?.id),
        None => None,
    };
    let state = match arg(args, "state") {
        Some(word) => Some(
            note::NoteState::from_word(word)
                .ok_or_else(|| Error::refused("mcp.unknownNoteState").param("state", word))?,
        ),
        None => None,
    };
    let filter = NoteFilter {
        work_id,
        search: arg(args, "query").map(str::to_owned),
        kind: arg(args, "kind").map(str::to_owned),
        state,
        ..NoteFilter::default()
    };
    // One note whole, by its id or its exact title: what a summary
    // of the list is read down to.
    if let Some(named) = arg(args, "note") {
        let found = find_note(conn, &profile.id, named)?;
        let whole = read_notes(
            conn,
            &profile.id,
            config,
            &NoteFilter {
                kind: Some(found.kind.clone()),
                ..NoteFilter::default()
            },
            None,
        )?
        .into_iter()
        .find(|shown| shown["id"] == json!(found.id))
        .map_or_else(|| serde_json::to_value(&found), Ok)?;
        return pretty(&whole);
    }
    let page = Page::of(args, 50);
    let mut notes = read_notes(conn, &profile.id, config, &filter, arg(args, "shelf"))?;
    if page.summary {
        notes = notes.iter().map(note_summary).collect();
    }
    let mut answer = page.cut("notes", notes);
    // With no question asked, the shelves too: what there is to ask
    // for. Narrowing by a page is no question.
    let asked = args
        .keys()
        .any(|key| !["limit", "offset", "summary", "space"].contains(&key.as_str()));
    if !asked {
        let shelves: Vec<Value> = crate::shelf::views(conn, &profile.id)?
            .iter()
            .map(|view| {
                json!({
                    "id": view.shelf.id, "name": view.shelf.name,
                    "phrases": view.note_ids.len(),
                })
            })
            .collect();
        answer.insert("shelves".into(), Value::Array(shelves));
    }
    pretty(&answer)
}

/// The `notes` tool: the notes a filter leaves - a phrase with the shelves it
/// stands on, how many works say it and how it is sung (ADR 0064) - or the
/// phrases of one shelf; asked for nothing in particular, the shelves too.
fn read_notes(
    conn: &Connection,
    profile_id: &str,
    config: &profile::config::ProfileConfig,
    filter: &NoteFilter,
    shelf: Option<&str>,
) -> Result<Vec<Value>> {
    let shelves = crate::shelf::views(conn, profile_id)?;
    let only = match shelf {
        Some(named) => Some(
            crate::shelf::find(conn, profile_id, named)?
                .ok_or_else(|| Error::refused("shelf.unknown").param("shelf", named))?,
        ),
        None => None,
    };
    let names = |ids: &[String]| -> Vec<String> {
        ids.iter()
            .filter_map(|id| shelves.iter().find(|view| view.shelf.id == *id))
            .map(|view| view.shelf.name.clone())
            .collect()
    };
    let lines = crate::lines::rows(conn, profile_id, filter)?;
    let mut out: Vec<Value> = Vec::new();
    for found in note::list(conn, profile_id, filter)? {
        let line = config.note_kind(&found.kind).is_some_and(|kind| kind.line);
        if !line {
            if only.is_none() {
                out.push(serde_json::to_value(&found)?);
            }
            continue;
        }
        let Some(row) = lines.iter().find(|row| row.note.id == found.id) else {
            continue;
        };
        if only
            .as_ref()
            .is_some_and(|shelf| !row.shelf_ids.contains(&shelf.id))
        {
            continue;
        }
        let mut shown = serde_json::to_value(&found)?;
        if let Some(object) = shown.as_object_mut() {
            object.insert("shelves".into(), json!(names(&row.shelf_ids)));
            object.insert("sung_in".into(), json!(row.uses));
            if !row.sung.is_empty() {
                object.insert("sung".into(), json!(row.sung));
            }
        }
        out.push(shown);
    }
    Ok(out)
}

/// A note as a summary gives it: what it is and how much there is of it,
/// without its body. A note with no title is called by its first words.
fn note_summary(note: &Value) -> Value {
    let body = note["body"].as_str().unwrap_or_default();
    let title = match note["title"]
        .as_str()
        .filter(|title| !title.trim().is_empty())
    {
        Some(title) => title.to_owned(),
        None => {
            let first = body
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("");
            first.trim().chars().take(80).collect()
        }
    };
    json!({
        "id": note["id"], "title": title, "kind": note["kind"], "state": note["state"],
        "tags": note["tags"], "work_id": note["work_id"],
        "length": length_of(body), "updated_at": note["updated_at"],
    })
}

/// A note by its id, or by its exact title.
fn find_note(conn: &Connection, profile_id: &str, named: &str) -> Result<note::Note> {
    if let Some(found) = note::get(conn, named)?.filter(|found| found.profile_id == profile_id) {
        return Ok(found);
    }
    let mut titled = note::list(conn, profile_id, &NoteFilter::default())?
        .into_iter()
        .filter(|found| found.title.as_deref().map(str::trim) == Some(named.trim()));
    match (titled.next(), titled.next()) {
        (Some(found), None) => Ok(found),
        (Some(_), Some(_)) => Err(Error::refused("mcp.ambiguousNote").param("note", named)),
        (None, _) => Err(Error::refused("mcp.unknownNote").param("note", named)),
    }
}

/// `propose_note`: a note or a phrase, a change to one that is there, or
/// several at once.
pub(super) fn propose_note(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        config,
        args,
    } = *call;
    let found = match arg(args, "work") {
        Some(named) => Some(find_work(conn, &profile.id, named)?),
        None => None,
    };
    // Several at once, or one that says more than its words - tags,
    // a layer, a note it changes: read as notes, kept whole or one by
    // one, each change shown against what it changes (v0.116).
    let several = args.get("notes").is_some();
    if several
        || args.get("tags").is_some()
        || arg(args, "layer").is_some()
        || arg(args, "note").is_some()
        || arg(args, "origin_work").is_some()
    {
        let raw: Vec<Value> = match args.get("notes") {
            Some(Value::Array(list)) if !list.is_empty() => list.clone(),
            Some(_) => return Err(Error::refused("mcp.notesNotArray")),
            None => vec![Value::Object(args.clone())],
        };
        let mut notes = Vec::with_capacity(raw.len());
        for one in raw {
            notes.push(read_proposed_note(conn, &profile.id, config, &one)?);
        }
        let count = notes.len();
        let changes = notes.iter().filter(|one| one.note.is_some()).count();
        let body = render_notes(&notes);
        deliver(
            conn,
            &profile.id,
            session,
            found.as_ref(),
            Proposal::Notes { notes },
            &body,
            None,
        )?;
        return Ok(format!(
            "Proposed {count} note{} ({changes} of them a change to a note that is there). \
             It waits in the chat {}; the person keeps it whole or note by note.",
            if count == 1 { "" } else { "s" },
            if found.is_some() {
                "on the work"
            } else {
                "named after you"
            }
        ));
    }
    let body = required(args, "body")?;
    let kind = arg(args, "kind").map(str::to_owned);
    if let Some(kind) = kind.as_deref()
        && !config.note_kind(kind).is_some_and(|found| !found.is_card())
    {
        return Err(Error::refused("proposal.unknownNoteKind").param("kind", kind));
    }
    let proposal = Proposal::Note {
        title: arg(args, "title").map(str::to_owned),
        note_kind: kind,
        attribution: arg(args, "attribution").map(str::to_owned),
        shelf: arg(args, "shelf").map(str::to_owned),
    };
    deliver(
        conn,
        &profile.id,
        session,
        found.as_ref(),
        proposal,
        body,
        None,
    )?;
    Ok(match found {
        Some(w) => format!(
            "Proposed a note on “{}”; it waits in the chat on the work.",
            w.title
        ),
        None => "Proposed a note; it waits in the chat named after you.".into(),
    })
}

/// One note of a proposal, read from what an agent wrote: its kind checked
/// against the craft, the note it changes found, and what that note holds
/// now kept beside it for the person to read against.
fn read_proposed_note(
    conn: &Connection,
    profile_id: &str,
    config: &profile::config::ProfileConfig,
    raw: &Value,
) -> Result<crate::assistant::proposal::ProposedNote> {
    use crate::assistant::proposal::{NoteBefore, ProposedNote};

    let object = raw
        .as_object()
        .ok_or_else(|| Error::refused("mcp.notesNotArray"))?;
    let text = |key: &str| arg(object, key).map(str::to_owned);
    let kind = text("kind");
    if let Some(kind) = kind.as_deref()
        && !config.note_kind(kind).is_some_and(|found| !found.is_card())
    {
        return Err(Error::refused("proposal.unknownNoteKind").param("kind", kind));
    }
    let layer = match arg(object, "layer") {
        Some(word) => Some(
            serde_json::from_value::<crate::canon::Layer>(json!(word))
                .map_err(|_| Error::refused("mcp.unknownLayer").param("layer", word))?,
        ),
        None => None,
    };
    let tags: Vec<String> = match object.get("tags") {
        Some(Value::Array(list)) => list
            .iter()
            .filter_map(Value::as_str)
            .map(|tag| tag.trim().trim_start_matches('#').to_owned())
            .filter(|tag| !tag.is_empty())
            .collect(),
        _ => Vec::new(),
    };
    let body = text("body").unwrap_or_default();
    // The work a new note arose at (v0.145): a phrase that came while a song
    // was written remembers the song, and stays fresh for the next one.
    let origin_work_id = match arg(object, "origin_work") {
        Some(named) => Some(find_work(conn, profile_id, named)?.id),
        None => None,
    };
    let target = match arg(object, "note") {
        Some(named) => Some(find_note(conn, profile_id, named)?),
        None => None,
    };
    // A new note is its words; a change may leave them as they are.
    if target.is_none() && body.trim().is_empty() {
        return Err(Error::refused("mcp.missingArg").param("key", "body"));
    }
    Ok(ProposedNote {
        title: text("title"),
        body,
        kind,
        attribution: text("attribution"),
        shelf: text("shelf"),
        tags,
        layer,
        before: target.as_ref().map(|found| NoteBefore {
            title: found.title.clone(),
            body: found.body.clone(),
            tags: found.tags.clone(),
        }),
        note: target.map(|found| found.id),
        origin_work_id,
    })
}

/// Proposed notes as a person reads them before keeping them: each new one
/// with its words, each change with what it changes.
fn render_notes(notes: &[crate::assistant::proposal::ProposedNote]) -> String {
    let mut out = String::new();
    for one in notes {
        let name = one
            .title
            .clone()
            .or_else(|| one.before.as_ref().and_then(|before| before.title.clone()))
            .unwrap_or_else(|| {
                let source = if one.body.trim().is_empty() {
                    one.before
                        .as_ref()
                        .map_or("", |before| before.body.as_str())
                } else {
                    one.body.as_str()
                };
                source
                    .lines()
                    .find(|line| !line.trim().is_empty())
                    .unwrap_or("")
                    .trim()
                    .chars()
                    .take(80)
                    .collect()
            });
        match &one.before {
            None => {
                out.push_str(&format!("- **{name}**"));
                if !one.tags.is_empty() {
                    out.push_str(&format!(" _({})_", one.tags.join(", ")));
                }
                out.push('\n');
                if one.title.is_some() {
                    out.push_str(&format!("  {}\n", one.body.trim().replace('\n', "\n  ")));
                }
            }
            Some(before) => {
                out.push_str(&format!("- **{name}** — a change\n"));
                if let Some(title) = one.title.as_deref().filter(|title| {
                    before.title.as_deref() != Some(*title) && !title.trim().is_empty()
                }) {
                    out.push_str(&format!(
                        "  title: {} → {title}\n",
                        before.title.as_deref().unwrap_or("-")
                    ));
                }
                if !one.tags.is_empty() && one.tags != before.tags {
                    out.push_str(&format!(
                        "  tags: {} → {}\n",
                        if before.tags.is_empty() {
                            "-".to_owned()
                        } else {
                            before.tags.join(", ")
                        },
                        one.tags.join(", ")
                    ));
                }
                if !one.body.trim().is_empty() && one.body.trim() != before.body.trim() {
                    out.push_str(&format!(
                        "  the text is rewritten ({} → {} characters)\n",
                        length_of(&before.body),
                        length_of(&one.body)
                    ));
                }
            }
        }
    }
    out.trim_end().to_owned()
}
