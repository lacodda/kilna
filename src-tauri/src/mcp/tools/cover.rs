//! The cover of a publication and the ideas for it.

use serde_json::{Value, json};

use crate::assistant::apply;
use crate::assistant::proposal::Proposal;
use crate::error::{Error, Result};
use crate::mcp::call::{Call, arg, find_work, pretty, required};
use crate::mcp::deliver::deliver;
use crate::speech::Speech;

/// `cover`: the board of ideas for a publication's cover and everything an
/// idea is built from.
pub(super) fn cover(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        config,
        args,
        ..
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    if !config.vocabulary(&found.kind).cover {
        return Err(Error::refused("idea.noCover").param("title", found.title.clone()));
    }
    let board = crate::cover::idea::board(conn, &found.id)?;
    let ideas: Vec<Value> = board
        .ideas
        .iter()
        .map(|card| {
            json!({
                "id": card.idea.id,
                "source": card.idea.source,
                "verdict": card.idea.verdict,
                "angle": card.idea.angle,
                "headline": card.idea.headline,
                "from": card.from_title,
                "concept": card.idea.concept,
            })
        })
        .collect();
    let siblings: Vec<Value> = board
        .siblings
        .iter()
        .map(|sibling| {
            json!({
                "work": sibling.work_id,
                "title": sibling.title,
                "kind": sibling.kind,
                "concept": sibling.concept,
            })
        })
        .collect();
    pretty(&json!({
        "work": found.id,
        "title": found.title,
        "format": board.format,
        "cover": found.cover,
        "ideas": ideas,
        "siblings": siblings,
        "choices": crate::cover::idea::choices_sheet(conn, &found)?,
    }))
}

/// `propose_cover`: ideas for the cover of a publication.
pub(super) fn propose_cover(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        config,
        args,
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    if !config.vocabulary(&found.kind).cover {
        return Err(Error::refused("idea.noCover").param("title", found.title.clone()));
    }
    let raw = args
        .get("ideas")
        .filter(|raw| raw.as_array().is_some_and(|list| !list.is_empty()))
        .ok_or_else(|| Error::refused("mcp.ideasNotArray"))?;
    let read = crate::cover::idea::read(conn, &profile.id, config, raw, None)?;
    let count = read.ideas.len();
    let left_out: Vec<String> = read
        .dropped
        .iter()
        .map(|one| format!("idea {}: {} “{}”", one.idea, one.part, one.value))
        .collect();
    let body = apply::render_ideas(conn, &read.ideas, &read.dropped, Speech::ENGLISH);
    let proposal = Proposal::CoverIdeas {
        work_id: found.id.clone(),
        ideas: read.ideas,
        dropped: read.dropped,
    };
    deliver(
        conn,
        &profile.id,
        session,
        Some(&found),
        proposal,
        &body,
        arg(args, "note"),
    )?;
    let plural = if count == 1 { "" } else { "s" };
    let mut said = format!(
        "Proposed {count} idea{plural} for the cover of “{}”. They wait in the chat on the work and above its board; one click puts them on the board.",
        found.title
    );
    if !left_out.is_empty() {
        said.push_str(&format!(
            " Left out, not in the workspace: {}.",
            left_out.join("; ")
        ));
    }
    Ok(said)
}
