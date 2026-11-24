//! The storyboard of a video or a short.

use serde_json::Value;

use crate::assistant::apply;
use crate::assistant::proposal::{self, BoardChange, Proposal};
use crate::error::{Error, Result};
use crate::mcp::call::{Call, find_work, pretty, required};
use crate::mcp::deliver::deliver;
use crate::scene;
use crate::speech::Speech;

/// `scenes`: the storyboard of a work, in order.
pub(super) fn scenes(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        args,
        ..
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    pretty(&scene::for_work(conn, &found.id)?)
}

/// `propose_scenes`: a storyboard added to the board, replacing it or
/// revising the scenes named.
pub(super) fn propose_scenes(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        config,
        args,
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    let vocabulary = config.vocabulary(&found.kind);
    let raw = args
        .get("scenes")
        .and_then(Value::as_array)
        .filter(|raw| !raw.is_empty())
        .ok_or_else(|| Error::refused("mcp.scenesNotArray"))?;
    let scenes = proposal::scenes_from(raw, vocabulary)?;
    let change = match args.get("change").and_then(Value::as_str) {
        None => BoardChange::Add,
        Some(value) => BoardChange::parse(value)
            .ok_or_else(|| Error::refused("mcp.badBoardChange").param("value", value))?,
    };
    if change == BoardChange::Revise
        && let Some(missing) = scenes.iter().position(|scene| scene.position.is_none())
    {
        return Err(Error::refused("mcp.reviseNeedsPosition").param("index", missing + 1));
    }
    let count = scenes.len();
    let standing = scene::count(conn, &found.id)?;
    let body = apply::render_board(&scenes, vocabulary, change, Speech::ENGLISH);
    let proposal = Proposal::Scenes { scenes, change };
    deliver(
        conn,
        &profile.id,
        session,
        Some(&found),
        proposal,
        &body,
        None,
    )?;
    let plural = if count == 1 { "" } else { "s" };
    Ok(match change {
        BoardChange::Replace => format!(
            "Proposed a storyboard of {count} scene{plural} to replace the {standing} on the board of “{}”. It waits in the chat on the work; one click replaces the board.",
            found.title
        ),
        BoardChange::Revise => format!(
            "Proposed a revision of {count} scene{plural} on the board of “{}”. It waits in the chat on the work; one click revises them in place.",
            found.title
        ),
        BoardChange::Add => format!(
            "Proposed {count} scene{plural} to add after the {standing} on the board of “{}”. It waits in the chat on the work; one click adds them.",
            found.title
        ),
    })
}
