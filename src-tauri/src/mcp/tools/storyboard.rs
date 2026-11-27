//! The storyboard of a video or a short.

use serde_json::{Value, json};

use crate::assistant::apply;
use crate::assistant::proposal::{self, BoardChange, Proposal};
use crate::error::{Error, Result};
use crate::mcp::call::{Call, Page, arg, find_work, length_of, pretty, required};
use crate::mcp::deliver::deliver;
use crate::scene;
use crate::speech::Speech;

/// `scenes`: the storyboard of a work, in order - a page at a time, short
/// or whole, or one scene by its number or id (v0.171).
pub(super) fn scenes(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        args,
        ..
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    let board = scene::for_work(conn, &found.id)?;
    // One scene whole: what a summary of the board is read down to.
    let named = arg(args, "scene").map(str::to_owned).or_else(|| {
        args.get("scene")
            .and_then(Value::as_u64)
            .map(|number| number.to_string())
    });
    if let Some(named) = named {
        let one = board
            .iter()
            .find(|scene| scene.id == named || scene.position.to_string() == named)
            .ok_or_else(|| Error::refused("mcp.unknownScene").param("scene", named))?;
        return pretty(one);
    }
    let page = Page::of(args, 100);
    let mut scenes = Vec::with_capacity(board.len());
    for one in &board {
        let whole = serde_json::to_value(one)?;
        scenes.push(if page.summary {
            scene_summary(&whole)
        } else {
            whole
        });
    }
    pretty(&page.cut("scenes", scenes))
}

/// A scene as a summary gives it: where it stands, what it is and the first
/// words of what happens in it, without its prompt blocks.
fn scene_summary(scene: &Value) -> Value {
    let description = scene["description"].as_str().unwrap_or_default();
    let written: usize = scene["blocks"]
        .as_object()
        .map(|blocks| {
            blocks
                .values()
                .filter_map(Value::as_str)
                .map(length_of)
                .sum()
        })
        .unwrap_or(0);
    json!({
        "id": scene["id"], "position": scene["position"], "section": scene["section"],
        "starts_at": scene["starts_at"], "ends_at": scene["ends_at"],
        "shot_type": scene["shot_type"], "layer": scene["layer"],
        "description": description.chars().take(120).collect::<String>(),
        "length": length_of(description) + written,
    })
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
