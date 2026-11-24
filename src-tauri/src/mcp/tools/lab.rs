//! An experiment's board of trials.

use serde_json::{Value, json};

use crate::assistant::apply;
use crate::assistant::proposal::Proposal;
use crate::error::{Error, Result};
use crate::mcp::call::{Call, Page, arg, find_work, length_of, pretty, required};
use crate::mcp::deliver::deliver;
use crate::speech::Speech;

/// `trials`: the board of an experiment, a page at a time, or one trial
/// whole.
pub(super) fn trials(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        args,
        ..
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    let board = crate::lab::trial::board(conn, &found.id)?;
    let page = Page::of(args, 50);
    let one = arg(args, "trial");
    let summary = page.summary && one.is_none();
    let trials: Vec<Value> = board
        .trials
        .iter()
        .filter(|card| one.is_none_or(|id| card.trial.id == id))
        .map(|card| {
            if summary {
                return json!({
                    "id": card.trial.id,
                    "series": card.trial.series,
                    "parent": card.trial.parent_id,
                    "angle": card.trial.angle,
                    "verdict": card.trial.verdict,
                    "length": length_of(&card.trial.body),
                    "takes": card.takes.len(),
                });
            }
            json!({
                "id": card.trial.id,
                "series": card.trial.series,
                "parent": card.trial.parent_id,
                "angle": card.trial.angle,
                "body": card.trial.body,
                "bricks": card.trial.bricks,
                "reference": card.trial.reference,
                "outcome": card.trial.outcome,
                "verdict": card.trial.verdict,
                "run_first": card.trial.run_first,
                "source": card.source.as_ref().map(|source| json!({
                    "work": source.work_id, "title": source.title,
                    "version": source.version_id, "revision": source.revision,
                })),
                "lost_anchors": card.lost_anchors,
                "takes": card.takes.len(),
                "harvest": card.harvest,
            })
        })
        .collect();
    // One trial, by its id: whole, and nothing of the board around it.
    if let Some(id) = one {
        let whole = trials
            .into_iter()
            .next()
            .ok_or_else(|| Error::refused("mcp.unknownTrial").param("trial", id))?;
        return pretty(&whole);
    }
    let mut answer = page.cut("trials", trials);
    answer.insert("work".into(), json!(found.id));
    answer.insert("title".into(), json!(found.title));
    answer.insert("anchors".into(), json!(board.anchors));
    answer.insert("harvest_role".into(), json!(board.harvest_role));
    answer.insert("harvest_kinds".into(), json!(board.harvest_kinds));
    answer.insert("series".into(), json!(board.series));
    // The phrases a trial is written from are the long half of the
    // board: left out of a summary, like the texts.
    if !page.summary {
        answer.insert(
            "choices".into(),
            serde_json::to_value(crate::lab::answer::choices_sheet(conn, &found)?)?,
        );
    }
    pretty(&answer)
}

/// `propose_trials`: trials for an experiment's board.
pub(super) fn propose_trials(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        config,
        args,
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    if config.lab(&found.kind).is_none() {
        return Err(Error::refused("trial.noLab").param("title", found.title.clone()));
    }
    let raw = args
        .get("trials")
        .filter(|raw| raw.as_array().is_some_and(|list| !list.is_empty()))
        .ok_or_else(|| Error::refused("mcp.trialsNotArray"))?;
    let read = crate::lab::answer::read(
        conn,
        config,
        crate::lab::answer::For::work(&found),
        raw,
        None,
    )?;
    let count = read.trials.len();
    let left_out: Vec<String> = read
        .dropped
        .iter()
        .map(|one| format!("trial {}: {} “{}”", one.trial, one.part, one.value))
        .collect();
    let body = apply::render_trials(&read.trials, &read.dropped, Speech::ENGLISH);
    let proposal = Proposal::Trials {
        work_id: found.id.clone(),
        trials: read.trials,
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
        "Proposed {count} trial{plural} for the board of “{}”. They wait in the chat on the work and above its board; one click puts them on the board.",
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
