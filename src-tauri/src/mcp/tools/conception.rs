//! A work prepared before it is written: the order and its boards.

use serde_json::{Value, json};

use crate::assistant::apply;
use crate::assistant::proposal::Proposal;
use crate::error::{Error, Result};
use crate::mcp::call::{Call, arg, find_work, pretty, required};
use crate::mcp::deliver::deliver;
use crate::profile;
use crate::speech::Speech;

/// `brief`: the order a writer works from, and the boards it was laid out
/// on.
pub(super) fn brief(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        config,
        args,
        ..
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    let conception = config.require_conception(&found.kind)?;
    // Nothing written yet is a conception still to be written, not a
    // mistake: the boards are there to read all the same.
    let order = match crate::conception::order(conn, &found) {
        Ok(order) => Value::from(order),
        Err(error)
            if error
                .refusal()
                .is_some_and(|r| r.code == "conception.empty") =>
        {
            Value::Null
        }
        Err(error) => return Err(error),
    };
    let boards: Vec<Value> = crate::board::boards(conn, &found)?
        .into_iter()
        .map(|view| {
            let laid = config.board(&view.board);
            json!({
                "key": view.board,
                "label": laid.map(|board| &board.label),
                "rule": laid.map(|board| board.rule),
                "cards": view.cards.iter().map(|card| json!({
                    "id": card.id,
                    "kind": card.kind,
                    "topic": card.topic,
                    "source": card.source,
                    "verdict": card.verdict,
                    "entry": card.entry,
                    "fields": card.fields,
                })).collect::<Vec<_>>(),
                "directions": view.directions,
                "elsewhere": view.elsewhere,
            })
        })
        .collect();
    // The author's key, entry by entry, with the rule it is kept by
    // (v0.146, FR-10 10.1).
    let anchors: Vec<String> = config
        .fields_of(&found.kind)
        .into_iter()
        .filter(|field| field.protected)
        .filter_map(|field| found.meta.get(&field.key))
        .flat_map(profile::config::MetaFieldType::entries)
        .collect();
    pretty(&json!({
        "work": found.id,
        "title": found.title,
        "kind": found.kind,
        "brief_role": conception.brief,
        "idea_field": conception.idea,
        "order": order,
        "anchors": anchors,
        "anchors_rule": "Keep every anchor as it is: do not change, move or drop one unless the author asks.",
        "boards": boards,
    }))
}

/// `propose_cards`: cards onto a board of a work's conception.
pub(super) fn propose_cards(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        config,
        args,
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    let board = config.require_board(&found.kind, required(args, "board")?)?;
    let cards = args
        .get("cards")
        .filter(|raw| raw.as_array().is_some_and(|list| !list.is_empty()))
        .ok_or_else(|| Error::refused("mcp.cardsNotArray"))?;
    let mut raw = json!({ "cards": cards });
    if let Some(directions) = args.get("directions") {
        raw["directions"] = directions.clone();
    }
    let read = crate::board::answer::read(board, config.library(), &raw, None)?;
    let count = read.cards.len();
    let left_out: Vec<String> = read
        .dropped
        .iter()
        .map(|one| format!("card {}: {} “{}”", one.card, one.part, one.value))
        .collect();
    let body = apply::render_cards(
        board,
        &read.cards,
        &read.directions,
        &read.dropped,
        Speech::ENGLISH,
    );
    let proposal = Proposal::Cards {
        work_id: found.id.clone(),
        board: board.key.clone(),
        cards: read.cards,
        directions: read.directions,
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
        "Proposed {count} card{plural} for the board “{}” of “{}”. They wait in the chat on the work and above its board; one click puts them on the board.",
        board.label, found.title
    );
    if !left_out.is_empty() {
        said.push_str(&format!(
            " Left out, not on the board: {}.",
            left_out.join("; ")
        ));
    }
    Ok(said)
}
