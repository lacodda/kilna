//! The register of repeats and the words proposed for the record.

use serde_json::{Value, json};

use crate::assistant::proposal::Proposal;
use crate::error::{Error, Result};
use crate::mcp::call::{Call, Page, arg, find_work, pretty};
use crate::mcp::deliver::deliver;
use crate::work::version;

/// `register`: the register of repeats (ADR 0044) - the whole of it, or a
/// text checked against it. The marks are the window's; an agent is given
/// what was found.
pub(super) fn register(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        args,
        ..
    } = *call;
    let text = match (arg(args, "text"), arg(args, "work")) {
        (Some(text), _) => Some(text.to_owned()),
        (None, Some(named)) => {
            let found = find_work(conn, &profile.id, named)?;
            let current = match found.current_version_id.as_deref() {
                Some(id) => version::get(conn, id)?,
                None => None,
            };
            Some(current.map(|v| v.body).unwrap_or_default())
        }
        (None, None) => None,
    };
    match text {
        Some(text) => {
            let checked = crate::register::check::text(conn, &profile.id, &text, false)?;
            // For a work, what the guard says too: the songs out or
            // booked it shares a spent term or a rare word with.
            let guard = match arg(args, "work") {
                Some(named) => {
                    let found = find_work(conn, &profile.id, named)?;
                    let today = crate::time::now();
                    crate::register::guard::of_work(conn, &found.id, &today[..10])?
                }
                None => None,
            };
            pretty(&json!({
                "terms": checked.terms,
                "repeats": checked.repeats,
                "guard": guard,
            }))
        }
        None => {
            let entries = crate::register::entries(conn, &profile.id)?;
            // One word whole, with the works that carry it.
            if let Some(named) = arg(args, "term") {
                let entry = entries
                    .iter()
                    .find(|entry| entry.term.id == named)
                    .or_else(|| {
                        entries.iter().find(|entry| {
                            entry.term.word.to_lowercase() == named.trim().to_lowercase()
                        })
                    })
                    .ok_or_else(|| Error::refused("mcp.unknownTerm").param("term", named))?;
                let mut whole = serde_json::to_value(entry)?;
                if let Some(object) = whole.as_object_mut() {
                    object.insert(
                        "works".into(),
                        serde_json::to_value(crate::register::uses(conn, &entry.term.id)?)?,
                    );
                }
                return pretty(&whole);
            }
            let page = Page::of(args, 100);
            let terms: Vec<Value> = entries
                .iter()
                .map(|entry| {
                    if page.summary {
                        Ok(json!({
                            "id": entry.term.id, "word": entry.term.word,
                            "kind": entry.term.kind,
                            "strictness": entry.term.strictness, "uses": entry.uses,
                        }))
                    } else {
                        serde_json::to_value(entry)
                    }
                })
                .collect::<std::result::Result<_, _>>()?;
            pretty(&page.cut("terms", terms))
        }
    }
}

/// `propose_words`: words for the record - phrases to keep, ways of singing,
/// terms for the register.
pub(super) fn propose_words(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        args,
        ..
    } = *call;
    let found = match arg(args, "work") {
        Some(named) => Some(find_work(conn, &profile.id, named)?),
        None => None,
    };
    let raw = args
        .get("words")
        .cloned()
        .ok_or_else(|| Error::refused("mcp.missingArg").param("key", "words"))?;
    let package = crate::register::proposal::read(conn, &profile.id, &raw)?;
    if package.is_empty() {
        return Err(Error::refused("words.noWords"));
    }
    let body = crate::register::proposal::render(&package);
    let count = package.words.len();
    let dropped = package.dropped.len();
    let changes = package.words.iter().filter(|word| word.known).count();
    let warned = package.warnings.len();
    deliver(
        conn,
        &profile.id,
        session,
        found.as_ref(),
        Proposal::Words { package },
        &body,
        arg(args, "note"),
    )?;
    let mut answer = format!(
        "Proposed {count} word{} for the record. It waits in the chat {}; the person keeps it whole or word by word.",
        if count == 1 { "" } else { "s" },
        if found.is_some() {
            "on the work"
        } else {
            "named after you"
        }
    );
    if changes > 0 {
        answer.push_str(&format!(
            " {changes} of them the record already keeps: the chat says what changes."
        ));
    }
    if warned > 0 {
        answer.push_str(&format!(
            " {warned} word(s) read as several, written with a comma or a slash: one \
             `word` is one thing - put synonyms in `forms`, or split them."
        ));
    }
    if dropped > 0 {
        answer.push_str(&format!(" {dropped} item(s) were left out: read the chat."));
    }
    Ok(answer)
}
