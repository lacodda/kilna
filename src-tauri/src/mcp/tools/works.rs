//! Works: the catalogue, a card, a text, the scores - and what is proposed
//! for a work or as a new one.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::assistant::apply;
use crate::assistant::proposal::{self, PackagedNote, PackagedVersion, Proposal};
use crate::error::{Error, Result};
use crate::mcp::call::{Call, Page, arg, find_work, pretty, required};
use crate::mcp::deliver::deliver;
use crate::speech::Speech;
use crate::work::version;
use crate::{link, release, scene, score};

/// `catalogue`: every work with its verdict, a page at a time, narrowed by
/// title, kind, status or what it feels like.
pub(super) fn catalogue(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        args,
        ..
    } = *call;
    let query = arg(args, "query").map(|q| q.to_lowercase());
    let kind = arg(args, "kind");
    let status = arg(args, "status");
    let felt = [
        ("mood", arg(args, "mood")),
        ("character", arg(args, "character")),
        ("complexity", arg(args, "complexity")),
    ];
    let page = Page::of(args, 100);
    // The guard's mark, by work (ADR 0054): what a song that has not
    // gone out shares with songs that have.
    let today = crate::time::now();
    let marks: std::collections::HashMap<String, crate::register::guard::RepeatMark> =
        crate::register::guard::marks(conn, &profile.id, &today[..10])?
            .into_iter()
            .map(|mark| (mark.work_id.clone(), mark))
            .collect();
    let rows: Vec<Value> = score::catalogue(conn, &profile.id)?
        .into_iter()
        .filter(|row| {
            query
                .as_ref()
                .is_none_or(|q| row.title.to_lowercase().contains(q))
        })
        .filter(|row| kind.is_none_or(|k| row.kind == k))
        .filter(|row| status.is_none_or(|s| row.status == s))
        .filter(|row| {
            felt.iter().all(|(line, wanted)| {
                wanted.is_none_or(|wanted| {
                    let shown = row.vibe.as_ref().map(|shown| &shown.vibe);
                    let said = shown.and_then(|vibe| match *line {
                        "mood" => vibe.mood.as_deref(),
                        "character" => vibe.character.as_deref(),
                        _ => vibe.complexity.as_deref(),
                    });
                    said == Some(wanted)
                })
            })
        })
        .map(|row| {
            if page.summary {
                json!({
                    "id": row.work_id, "title": row.title, "kind": row.kind, "status": row.status,
                    "updated_at": row.updated_at,
                })
            } else {
                json!({
                    "id": row.work_id, "title": row.title, "kind": row.kind, "status": row.status,
                    "total": row.total, "tier": row.tier, "tier_pinned": row.tier_pinned,
                    "stale": row.stale, "released": row.released, "scheduled": row.scheduled,
                    "scored_at": row.scored_at, "updated_at": row.updated_at,
                    "stage": row.stage,
                    // What it feels like, its own or taken from what it was
                    // made from (`from`), when anybody has said (FR-02).
                    "vibe": row.vibe,
                    "repeats": marks.get(&row.work_id).map(|mark| json!({
                        "level": mark.level, "count": mark.count, "word": mark.top.word,
                        "with": mark.top.neighbour_title, "day": mark.top.day,
                    })),
                })
            }
        })
        .collect();
    pretty(&page.cut("works", rows))
}

/// `work`: one work as its card shows it - everything but the bodies.
pub(super) fn work(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        config,
        args,
        ..
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    let versions = version::list(conn, &found.id)?;
    let mut by_role: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for v in versions {
        by_role.entry(v.role.clone()).or_default().push(json!({
            "id": v.id, "revision": v.revision, "label": v.label, "length": v.length,
            "is_current": v.is_current, "is_main": v.is_main,
            "created_at": v.created_at, "parent": v.parent_version_id,
        }));
    }
    let latest = score::latest(conn, &found.id)?;
    let releases = release::for_work(conn, &profile.id, &found.id)?;
    let links = link::for_work(conn, &found.id)?;
    let notes: i64 = conn.query_row(
        "SELECT count(*) FROM note WHERE work_id = ?1",
        [&found.id],
        |row| row.get(0),
    )?;
    let scenes = scene::count(conn, &found.id)?;
    let vocabulary = config.vocabulary(&found.kind);
    let frame = if vocabulary.frame {
        let view = crate::cover::read::frame_view(conn, &found.id)?;
        Some(json!({
            "still": view.prompts.still, "loop": view.prompts.loop_,
            "negative": view.prompts.negative, "built_from_cover": view.from_cover,
            "motion": found.frame.motion, "seconds": found.frame.seconds(),
            "still_camera": found.frame.still_camera, "seamless": found.frame.seamless,
        }))
    } else {
        None
    };
    // The cover as a generator gets it - the prompt written from the
    // concept - beside the concept itself.
    let cover = if vocabulary.cover || found.cover.holds_anything() {
        let view = crate::cover::read::view(conn, &found.id, None)?;
        Some(json!({
            "prompt": view.prompts, "format": view.format, "title": view.title,
            "concept": found.cover,
        }))
    } else {
        None
    };
    let lab = config.lab(&found.kind).is_some();
    let publications = if vocabulary.has_doors() || lab {
        None
    } else {
        Some(crate::publication::of(conn, config, &found.id)?)
    };
    let trials = if lab {
        Some(crate::lab::trial::counts(conn, &found.id)?)
    } else {
        None
    };
    pretty(&json!({
        "id": found.id, "title": found.title, "kind": found.kind, "status": found.status,
        "meta": found.meta, "tags": found.tags, "marks": found.marks,
        // How finished the author says it is, 0..=100. Worth telling an
        // agent: a full lyric at stage 20 is a sketch to build on, not
        // a work to polish, and nothing else in this shape says so.
        "stage": found.stage,
        // What it feels like (FR-02): its own, as said for it, and the
        // one it shows - its own, or that of what it was made from.
        "vibe": found.vibe,
        "shown_vibe": crate::work::shown_vibes(conn, &profile.id)?.remove(&found.id),
        "tier_pinned": found.tier_pinned, "tier_pin_reason": found.tier_pin_reason,
        // The day it is planned for before anything of it can go out
        // (v0.138), when it is.
        "work_plan": crate::work_plan::of_work(conn, &found.id)?
            .filter(|plan| !crate::work_plan::closed(conn, plan, config.plan_window()).unwrap_or(true))
            .map(|plan| json!({ "id": plan.id, "scheduled_at": plan.scheduled_at, "note": plan.note })),
        "created_at": found.created_at, "updated_at": found.updated_at,
        "versions": by_role,
        "latest_score": latest,
        "releases": releases,
        "notes": notes,
        "scenes": scenes,
        "cover": cover,
        "frame": frame,
        "publications": publications,
        "trials": trials,
        "sources": links.sources.iter().map(|l| json!({
            "work": l.source_id, "title": l.source_title, "kind": l.source_kind, "role": l.role,
            "taken_at_version": l.source_version_id, "drifted": l.drifted,
        })).collect::<Vec<_>>(),
        "derived": links.derived.iter().map(|d| json!({
            "work": d.work_id, "title": d.title, "kind": d.kind, "status": d.status, "role": d.role,
        })).collect::<Vec<_>>(),
    }))
}

/// `text`: the body of a version - the one that stands for a role, or the
/// one named by id, which must be the work's own.
pub(super) fn text(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        config,
        args,
        ..
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    let body = match arg(args, "version") {
        Some(id) => {
            let v = version::get(conn, id)?.ok_or_else(|| Error::not_found("version", id))?;
            if v.work_id != found.id {
                return Err(Error::refused("mcp.versionOtherWork").param("version", id));
            }
            v
        }
        None => {
            let vocabulary = config.vocabulary(&found.kind);
            let role = match arg(args, "role") {
                Some(role) => role.to_owned(),
                None => vocabulary
                    .version_roles
                    .first()
                    .map(|r| r.key.clone())
                    .ok_or_else(|| {
                        Error::refused("mcp.kindNoRoles").param("kind", found.kind.clone())
                    })?,
            };
            if !vocabulary.version_roles.iter().any(|r| r.key == role) {
                return Err(Error::refused("mcp.unknownVersionRole")
                    .param("role", role.clone())
                    .param("kind", found.kind.clone()));
            }
            // The version that stands for the role: the current one
            // when it is of this role, the one marked main, otherwise
            // the newest - the same answer the panel gives.
            version::for_role(conn, &found.id, &role)?.ok_or_else(|| {
                Error::refused("mcp.noVersionYet")
                    .param("title", found.title.clone())
                    .param("role", role.clone())
            })?
        }
    };
    let reads = config
        .vocabulary(&found.kind)
        .version_roles
        .iter()
        .find(|r| r.key == body.role)
        .map_or("plain", |r| {
            if r.reads_as_markdown() {
                "markdown"
            } else {
                "plain"
            }
        });
    pretty(&json!({
        "version": body.id, "role": body.role, "revision": body.revision, "label": body.label,
        "reads_as": reads, "created_at": body.created_at, "body": body.body,
    }))
}

/// `scores`: the score history of a work, newest first.
pub(super) fn scores(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        args,
        ..
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    pretty(&score::history(conn, &found.id)?)
}

/// `propose_version`: a new text in a role, given whole or as the bricks a
/// composition writes it from.
pub(super) fn propose_version(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        config,
        args,
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    let role = required(args, "role")?;
    if !config
        .vocabulary(&found.kind)
        .version_roles
        .iter()
        .any(|r| r.key == role)
    {
        return Err(Error::refused("mcp.unknownVersionRole")
            .param("role", role)
            .param("kind", found.kind.clone()));
    }
    // A text written out of the dictionary is written by kilna, from
    // the bricks named - the same function the constructor uses - so
    // an agent and a person picking the same phrases get one text.
    let named: Vec<String> = args
        .get("bricks")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let written;
    let mut said = String::new();
    let body = if named.is_empty() {
        required(args, "body")?
    } else {
        if arg(args, "body").is_some() {
            return Err(Error::refused("mcp.bodyAndBricks"));
        }
        let composition = config.composition_for(&found.kind, role).ok_or_else(|| {
            Error::refused("mcp.roleNotComposed")
                .param("role", role)
                .param("kind", found.kind.clone())
        })?;
        let bricks = super::styles::brick_ids(conn, &profile.id, composition, &named)?;
        let composed = crate::phrase::compose::write(
            conn,
            &profile.id,
            &crate::phrase::compose::ComposeRequest {
                composition: composition.key.clone(),
                bricks,
                work_id: Some(found.id.clone()),
            },
        )?;
        for problem in &composed.problems {
            said.push_str(&format!(
                " `{}`: {} picked, the composition takes {}{}.",
                problem.type_key,
                problem.picked,
                problem.min,
                problem
                    .max
                    .map(|max| format!("-{max}"))
                    .unwrap_or_else(|| "+".into())
            ));
        }
        if let Some(limit) = composed.limit
            && composed.length > limit as usize
        {
            said.push_str(&format!(
                " The text is {} characters, past the limit of {limit}.",
                composed.length
            ));
        }
        written = composed.text;
        written.as_str()
    };
    // The version rewritten, said now rather than lost at apply time:
    // lineage stays inside a work and a role (ADR 0055).
    let from = arg(args, "from").map(str::to_owned);
    if let Some(from) = &from {
        version::check_line(conn, &found.id, role, from)?;
    }
    let proposal = Proposal::Version {
        role: role.to_owned(),
        label: arg(args, "label").map(str::to_owned),
        from,
    };
    deliver(
        conn,
        &profile.id,
        session,
        Some(&found),
        proposal,
        body,
        arg(args, "note"),
    )?;
    Ok(format!(
        "Proposed a `{role}` version for “{}”. It is in the chat on the work, waiting to be inserted — or not.{said}",
        found.title
    ))
}

/// `propose_score`: marks along the axes of the work's kind.
pub(super) fn propose_score(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        config,
        args,
    } = *call;
    let found = find_work(conn, &profile.id, required(args, "work")?)?;
    let axes = args
        .get("axes")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| Error::refused("mcp.axesNotObject"))?;
    let note = arg(args, "note").map(str::to_owned);
    let vocabulary = config.vocabulary(&found.kind);
    let proposal = proposal::score_from(axes, note.clone(), vocabulary).ok_or_else(|| {
        Error::refused("mcp.noKnownAxes")
            .param("kind", found.kind.clone())
            .param(
                "known",
                vocabulary
                    .axes
                    .iter()
                    .map(|a| a.key.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
    })?;
    let summary = match &proposal {
        Proposal::Score {
            axes,
            unknown,
            missing,
            ..
        } => {
            let mut s = format!(
                "Proposed a score for “{}”: {}.",
                found.title,
                axes.iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if !unknown.is_empty() {
                s.push_str(&format!(
                    " Ignored axes the profile does not have: {}.",
                    unknown.join(", ")
                ));
            }
            if !missing.is_empty() {
                s.push_str(&format!(" Left unjudged: {}.", missing.join(", ")));
            }
            s
        }
        _ => unreachable!("score_from makes scores"),
    };
    let body = note
        .clone()
        .unwrap_or_else(|| "A score, proposed from outside kilna.".into());
    deliver(
        conn,
        &profile.id,
        session,
        Some(&found),
        proposal,
        &body,
        None,
    )?;
    Ok(summary)
}

/// `propose_work`: a whole new work, or a package of changes to one, applied
/// together.
pub(super) fn propose_work(call: &Call) -> Result<String> {
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
    let (title, kind) = match &found {
        Some(w) => (None, w.kind.clone()),
        None => {
            let title = required(args, "title")
                .map_err(|_| Error::refused("proposal.newWorkNeedsTitle"))?;
            let kind =
                required(args, "kind").map_err(|_| Error::refused("proposal.newWorkNeedsKind"))?;
            (Some(title.to_owned()), kind.to_owned())
        }
    };
    config.require_kind(&kind)?;
    let vocabulary = config.vocabulary(&kind);

    let (fields, unknown_fields) = proposal::fields_from(
        args.get("fields")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default(),
        config,
    );

    // What the work feels like (FR-02), in the profile's words: a
    // word it does not have is refused with the ones it has.
    let vibe = match args.get("vibe") {
        None | Some(Value::Null) => None,
        Some(raw) => {
            let said: crate::work::Vibe =
                serde_json::from_value(raw.clone()).map_err(|_| Error::refused("mcp.badVibe"))?;
            let said = said.kept();
            if let Some(said) = &said {
                config.vibe().check(said)?;
            }
            said
        }
    };

    let mut versions = Vec::new();
    for item in args
        .get("versions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let item = item.as_object().cloned().unwrap_or_default();
        let role = required(&item, "role")?;
        if !vocabulary.version_roles.iter().any(|r| r.key == role) {
            return Err(Error::refused("mcp.unknownVersionRole")
                .param("role", role)
                .param("kind", kind.clone()));
        }
        // Only a work that exists has versions to have been written
        // from; a new work's text is written from nothing.
        let from = arg(&item, "from").map(str::to_owned);
        if let Some(from) = &from {
            match &found {
                Some(work) => version::check_line(conn, &work.id, role, from)?,
                None => {
                    return Err(Error::refused("version.parentNotSameRole")
                        .param("parent", from.clone())
                        .param("role", role));
                }
            }
        }
        versions.push(PackagedVersion {
            role: role.to_owned(),
            body: required(&item, "body")?.to_owned(),
            label: arg(&item, "label").map(str::to_owned),
            from,
        });
    }

    let score = match args.get("score").and_then(Value::as_object) {
        Some(raw) => {
            let axes = raw
                .get("axes")
                .and_then(Value::as_object)
                .cloned()
                .ok_or_else(|| Error::refused("mcp.scoreAxesNotObject"))?;
            let note = arg(raw, "note").map(str::to_owned);
            Some(proposal::marks_from(axes, note, vocabulary).ok_or_else(|| {
                Error::refused("mcp.noKnownScoreAxes")
                    .param("kind", kind.clone())
                    .param(
                        "known",
                        vocabulary
                            .axes
                            .iter()
                            .map(|a| a.key.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    )
            })?)
        }
        None => None,
    };

    let mut notes = Vec::new();
    for item in args
        .get("notes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let item = item.as_object().cloned().unwrap_or_default();
        notes.push(PackagedNote {
            title: arg(&item, "title").map(str::to_owned),
            body: required(&item, "body")?.to_owned(),
            kind: arg(&item, "kind").map(str::to_owned),
            attribution: arg(&item, "attribution").map(str::to_owned),
        });
    }

    let scenes = match args.get("scenes").and_then(Value::as_array) {
        Some(raw) if !raw.is_empty() => proposal::scenes_from(raw, vocabulary)?,
        _ => Vec::new(),
    };

    let mut releases = Vec::new();
    for item in args
        .get("releases")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let item = item.as_object().cloned().unwrap_or_default();
        let release_kind = required(&item, "kind")?;
        config.require_release_kind(&kind, release_kind)?;
        let defined = vocabulary
            .release_kinds
            .iter()
            .find(|k| k.key == release_kind)
            .expect("just checked by require_release_kind");
        let scheduled_at = arg(&item, "scheduled_at").map(str::to_owned);
        if let Some(date) = &scheduled_at
            && !crate::time::is_date(date)
        {
            return Err(Error::refused("mcp.badReleaseDate").param("value", date.clone()));
        }
        let (fields, unknown_fields) = proposal::release_fields_from(
            item.get("fields")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default(),
            defined,
        );
        releases.push(proposal::PackagedRelease {
            kind: release_kind.to_owned(),
            scheduled_at,
            fields,
            unknown_fields,
        });
    }

    let mut trials_left_out = Vec::new();
    let trials = match args.get("trials") {
        Some(raw) if raw.as_array().is_some_and(|list| !list.is_empty()) => {
            if config.lab(&kind).is_none() {
                return Err(Error::refused("trial.noLabKind").param("kind", kind.clone()));
            }
            let read = crate::lab::answer::read(
                conn,
                config,
                crate::lab::answer::For {
                    profile_id: &profile.id,
                    kind: &kind,
                    work_id: found.as_ref().map(|w| w.id.as_str()),
                },
                raw,
                None,
            )?;
            trials_left_out = read
                .dropped
                .iter()
                .map(|one| format!("trial {}: {} “{}”", one.trial, one.part, one.value))
                .collect();
            read.trials
        }
        _ => Vec::new(),
    };

    // Cards for the boards of the work's conception, read as
    // `propose_cards` reads them: a song arrives with its research.
    let mut cards_left_out = Vec::new();
    let mut boards = Vec::new();
    for item in args
        .get("boards")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let item = item.as_object().cloned().unwrap_or_default();
        let laid = config.require_board(&kind, required(&item, "board")?)?;
        let raw = json!({ "cards": item.get("cards").cloned().unwrap_or_else(|| json!([])) });
        let read = crate::board::answer::read(laid, config.library(), &raw, None)?;
        cards_left_out.extend(read.dropped.iter().map(|one| {
            format!(
                "{} card {}: {} “{}”",
                laid.label, one.card, one.part, one.value
            )
        }));
        boards.push(crate::assistant::proposal::PackagedBoard {
            board: laid.key.clone(),
            cards: read.cards,
        });
    }

    if found.is_some()
        && fields.is_empty()
        && vibe.is_none()
        && versions.is_empty()
        && score.is_none()
        && notes.is_empty()
        && scenes.is_empty()
        && releases.is_empty()
        && trials.is_empty()
        && boards.is_empty()
    {
        return Err(Error::refused("mcp.emptyPackage"));
    }

    let proposal = Proposal::Work {
        vibe: vibe.map(Box::new),
        title,
        work_kind: Some(kind.clone()),
        fields,
        unknown_fields,
        versions,
        score,
        notes,
        scenes,
        releases,
        trials,
        boards,
    };
    let body = apply::render_package(&proposal, config, &kind, Speech::ENGLISH);
    let mut summary = package_summary(&proposal);
    if !trials_left_out.is_empty() {
        summary.push_str(&format!(
            ", left out of trials (not in the workspace): {}",
            trials_left_out.join("; ")
        ));
    }
    if !cards_left_out.is_empty() {
        summary.push_str(&format!(
            ", left out of cards: {}",
            cards_left_out.join("; ")
        ));
    }
    deliver(
        conn,
        &profile.id,
        session,
        found.as_ref(),
        proposal,
        &body,
        None,
    )?;
    Ok(match found {
        Some(w) => format!(
            "Proposed a package for “{}” — {summary}. It waits in the chat on the work; one click applies all of it.",
            w.title
        ),
        None => format!(
            "Proposed a new work — {summary}. It waits in the chat named after you; one click creates it with everything in it."
        ),
    })
}

/// What a package holds, in one clause for the agent's answer.
fn package_summary(proposal: &Proposal) -> String {
    let Proposal::Work {
        title,
        fields,
        unknown_fields,
        versions,
        score,
        notes,
        scenes,
        releases,
        trials,
        boards,
        ..
    } = proposal
    else {
        return String::new();
    };
    let mut parts = Vec::new();
    if let Some(title) = title {
        parts.push(format!("“{title}”"));
    }
    if !versions.is_empty() {
        parts.push(format!(
            "{} version{} ({})",
            versions.len(),
            if versions.len() == 1 { "" } else { "s" },
            versions
                .iter()
                .map(|v| v.role.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !fields.is_empty() {
        parts.push(format!(
            "fields {}",
            fields.keys().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    if let Some(marks) = score {
        parts.push(format!("a score on {} axes", marks.axes.len()));
        if !marks.unknown.is_empty() {
            parts.push(format!("axes ignored: {}", marks.unknown.join(", ")));
        }
    }
    if !notes.is_empty() {
        parts.push(format!(
            "{} note{}",
            notes.len(),
            if notes.len() == 1 { "" } else { "s" }
        ));
    }
    if !scenes.is_empty() {
        parts.push(format!(
            "{} scene{}",
            scenes.len(),
            if scenes.len() == 1 { "" } else { "s" }
        ));
    }
    if !trials.is_empty() {
        parts.push(format!(
            "{} trial{}",
            trials.len(),
            if trials.len() == 1 { "" } else { "s" }
        ));
    }
    for packaged in boards {
        parts.push(format!(
            "{} card{} for `{}`",
            packaged.cards.len(),
            if packaged.cards.len() == 1 { "" } else { "s" },
            packaged.board
        ));
    }
    if !releases.is_empty() {
        parts.push(format!(
            "{} release{} ({})",
            releases.len(),
            if releases.len() == 1 { "" } else { "s" },
            releases
                .iter()
                .map(|r| r.kind.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !unknown_fields.is_empty() {
        parts.push(format!(
            "fields ignored (not in the profile): {}",
            unknown_fields.join(", ")
        ));
    }
    // Named per release rather than pooled: two releases may each ignore a
    // different key, and one merged list would not say which.
    for release in releases {
        if !release.unknown_fields.is_empty() {
            parts.push(format!(
                "`{}` fields ignored (not of that kind of release): {}",
                release.kind,
                release.unknown_fields.join(", ")
            ));
        }
    }
    parts.join(", ")
}
