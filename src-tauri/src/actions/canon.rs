//! Gestures on the canon: cards, their facts, the relations between them, and
//! the description a picture generator is given.
//!
//! A card is a note (ADR 0043), so making one, renaming it or raising it from
//! a song to the channel is the note's own gesture; what is the canon's own is
//! here.

use rusqlite::Connection;

use super::gesture;
use crate::canon::{
    self, CanonLink, CanonLinkPatch, Fact, FactPatch, FactStatus, NewCanonLink, NewFact,
};
use crate::error::{Error, Result};
use crate::note::{self, NewNote, Note, NotePatch};

/// Make a card: a note of a kind the profile names with sections.
///
/// The note's own gesture, `note.create`, so a replay and an undo treat it as
/// the note it is. What is added is the check that the kind is a card's: a
/// card of a kind with no sections would be a note the Canon screen never
/// shows.
pub fn create_card(conn: &Connection, new: NewNote) -> Result<Note> {
    gesture(conn, "note.create", |act| {
        let config = crate::profile::config_for(act, act.profile_id())?;
        let kind = new.kind.clone().unwrap_or_default();
        config.require_card_kind(&kind)?;
        if new
            .title
            .as_deref()
            .map(str::trim)
            .is_none_or(str::is_empty)
        {
            return Err(Error::refused("canon.cardNeedsName"));
        }
        act.json("note", &new)?;
        let minted = act.mint();
        note::create_minted(act, act.profile_id(), new, minted)
    })
}

/// Write the description a generator is given for a card, or clear it.
///
/// The fingerprint of the facts it answers to is taken here, at the moment it
/// is written - by hand, or kept from the assistant, which passes the one it
/// was composed against so that facts changed while it was writing still show
/// the description as stale. The window never sets the fingerprint itself.
pub fn describe(
    conn: &Connection,
    id: &str,
    text: Option<String>,
    basis: Option<String>,
) -> Result<Note> {
    let found = note::get(conn, id)?.ok_or_else(|| Error::not_found("note", id))?;
    let config = crate::profile::config_for(conn, &found.profile_id)?;
    let (_, kind) = canon::fact::card_of(conn, &config, id)?;
    let text = text.map(|t| t.trim().to_owned()).filter(|t| !t.is_empty());
    let basis = match (&text, basis) {
        (None, _) => None,
        (Some(_), Some(basis)) => Some(basis),
        (Some(_), None) => canon::view::basis_of(conn, &found, kind)?,
    };
    super::note::update(
        conn,
        id,
        NotePatch {
            prompt: Some(text),
            prompt_basis: Some(basis),
            ..NotePatch::default()
        },
    )
}

/// Write a fact onto a card, at the end of its section.
pub fn add_fact(conn: &Connection, new: NewFact) -> Result<Fact> {
    gesture(conn, "fact.create", |act| {
        act.param("title", excerpt(&new.body));
        act.json("fact", &new)?;
        let minted = act.mint();
        canon::fact::create_minted(act, new, minted)
    })
}

/// Change a fact.
///
/// A fact moved to another section carries the place it lands in, so that a
/// replay puts it in the same place and an undo puts it back where it stood.
pub fn update_fact(conn: &Connection, id: &str, patch: FactPatch) -> Result<Fact> {
    gesture(conn, "fact.update", |act| {
        let before = canon::fact::get(act, id)?.ok_or_else(|| Error::not_found("fact", id))?;
        let mut patch = patch;
        if patch
            .section
            .as_deref()
            .is_some_and(|section| section != before.section)
            && patch.position.is_none()
        {
            let section = patch.section.clone().unwrap_or_default();
            patch.position = Some(act.query_row(
                "SELECT coalesce(max(position), 0) + 1 FROM canon_fact WHERE note_id = ?1 AND section = ?2",
                rusqlite::params![before.note_id, section],
                |row| row.get(0),
            )?);
        }
        act.param("id", id);
        act.param("title", excerpt(&before.body));
        act.json("patch", &patch)?;
        act.before(Some(&before), &patch)?;
        act.stamped();
        let after = canon::fact::update_at(act, id, patch, act.at())?;
        if after == before {
            act.unchanged();
        }
        Ok(after)
    })
}

/// Retire a fact, saying why. Kept, not deleted: the reason is what stops the
/// next picture from bringing it back.
pub fn retire_fact(conn: &Connection, id: &str, reason: &str) -> Result<Fact> {
    update_fact(
        conn,
        id,
        FactPatch {
            status: Some(FactStatus::Retired),
            retired_reason: Some(Some(reason.to_owned())),
            ..FactPatch::default()
        },
    )
}

/// Put the facts of one section of a card in the order given.
pub fn reorder_facts(
    conn: &Connection,
    note_id: &str,
    section: &str,
    ids: &[String],
) -> Result<()> {
    gesture(conn, "fact.reorder", |act| {
        let before: Vec<String> = canon::fact::in_section(act, note_id, section)?
            .into_iter()
            .map(|fact| fact.id)
            .collect();
        if before == ids {
            act.unchanged();
            return Ok(());
        }
        act.param("noteId", note_id);
        act.param("section", section);
        act.param("title", card_title(act, note_id));
        act.json("order", &ids)?;
        act.json("before", &before)?;
        act.stamped();
        canon::fact::reorder(act, note_id, section, ids, act.at())
    })
}

/// Draw a relation between two cards.
pub fn relate(conn: &Connection, new: NewCanonLink) -> Result<CanonLink> {
    gesture(conn, "canonLink.create", |act| {
        act.param("title", pair_title(act, &new.from_id, &new.to_id));
        act.json("link", &new)?;
        let minted = act.mint();
        canon::link::create_minted(act, new, minted)
    })
}

/// Change what a relation says.
pub fn update_relation(conn: &Connection, id: &str, patch: CanonLinkPatch) -> Result<CanonLink> {
    gesture(conn, "canonLink.update", |act| {
        let before = canon::link::get(act, id)?.ok_or_else(|| Error::not_found("relation", id))?;
        act.param("id", id);
        act.param("title", pair_title(act, &before.from_id, &before.to_id));
        act.json("patch", &patch)?;
        act.before(Some(&before), &patch)?;
        act.stamped();
        let after = canon::link::update_at(act, id, patch, act.at())?;
        if after == before {
            act.unchanged();
        }
        Ok(after)
    })
}

/// Undraw a relation. The row goes outright, as a link between works does;
/// the log keeps a copy for the undo.
pub fn unrelate(conn: &Connection, id: &str) -> Result<()> {
    gesture(conn, "canonLink.delete", |act| {
        let before = canon::link::get(act, id)?.ok_or_else(|| Error::not_found("relation", id))?;
        act.param("id", id);
        act.param("title", pair_title(act, &before.from_id, &before.to_id));
        act.json("before", &before)?;
        canon::link::delete(act, id)
    })
}

/// The opening words of a fact, for the sentence an undo offers.
fn excerpt(body: &str) -> String {
    let body = body.trim();
    match body.char_indices().nth(60) {
        Some((at, _)) => format!("{}…", &body[..at]),
        None => body.to_owned(),
    }
}

/// What a card is called, for the sentence an undo offers.
fn card_title(conn: &Connection, note_id: &str) -> String {
    note::get(conn, note_id)
        .ok()
        .flatten()
        .and_then(|card| card.title)
        .unwrap_or_default()
}

/// The two cards a relation joins, for the sentence an undo offers.
fn pair_title(conn: &Connection, from: &str, to: &str) -> String {
    format!("{} — {}", card_title(conn, from), card_title(conn, to))
}

/// A picture pasted onto a card, or onto one of its facts - the arrival a
/// chosen file takes, as `asset.attach`, so the log reads one kind of arrival.
pub fn paste_picture(
    conn: &Connection,
    media: &std::path::Path,
    picture: crate::asset::NewAsset,
    bytes: &[u8],
    name: &str,
) -> Result<crate::asset::Asset> {
    gesture(conn, "asset.attach", |act| {
        act.param("source", format!("<pasted: {name}>"));
        act.json("asset", &picture)?;
        crate::asset::attach_bytes(act, act.profile_id(), media, bytes, name, picture)
    })
}

/// Give a picture of a card another role: the reference that turned out to
/// be the portrait.
pub fn set_picture_role(conn: &Connection, id: &str, role: &str) -> Result<crate::asset::Asset> {
    gesture(conn, "asset.setRole", |act| {
        let before = crate::asset::get(act, id)?.ok_or_else(|| Error::not_found("asset", id))?;
        act.param("id", id);
        act.param("role", role);
        act.param("before", before.kind.clone());
        act.param("title", before.original_name.clone().unwrap_or_default());
        let after = crate::asset::set_role(act, id, role)?;
        if after.kind == before.kind {
            act.unchanged();
        }
        Ok(after)
    })
}

/// Whether the person kept item `index` of `kind` - every item when they
/// chose none.
fn kept(items: Option<&[String]>, kind: &str, index: usize) -> bool {
    items.is_none_or(|items| items.iter().any(|item| item == &format!("{kind}:{index}")))
}

/// Everything wrong with keeping the chosen items of a proposal for the
/// canon, all at once: a card of a kind that is not a card's, a second root,
/// a fact on a card that was left out or is gone, a section the kind does not
/// have, a fact to change that nobody holds any more.
pub fn check_package(
    conn: &Connection,
    config: &crate::profile::config::ProfileConfig,
    package: &canon::proposal::Package,
    items: Option<&[String]>,
) -> Result<Vec<crate::error::Reason>> {
    let mut problems = Vec::new();
    let mut note = |outcome: Result<()>| {
        if let Err(cause) = outcome {
            problems.push(cause.reason());
        }
    };
    if items.is_some_and(<[String]>::is_empty) {
        note(Err(Error::refused("canon.nothingChosen")));
    }

    let mut kinds_by_handle = std::collections::BTreeMap::new();
    for (index, card) in package.cards.iter().enumerate() {
        if !kept(items, "card", index) {
            continue;
        }
        match config.require_card_kind(&card.kind) {
            Ok(kind) => {
                kinds_by_handle.insert(card.handle.clone(), kind.key.clone());
                if kind.root {
                    let standing: Option<String> = conn
                        .query_row(
                            "SELECT id FROM note WHERE kind = ?1 LIMIT 1",
                            rusqlite::params![kind.key],
                            |row| row.get(0),
                        )
                        .ok();
                    if standing.is_some() {
                        note(Err(Error::refused("canon.secondRoot").param(
                            "kind",
                            serde_json::to_value(&kind.label).unwrap_or_default(),
                        )));
                    }
                }
            }
            Err(cause) => note(Err(cause)),
        }
        if let Some(work_id) = card.work_id.as_deref() {
            if crate::work::get(conn, work_id)?.is_none() {
                note(Err(Error::not_found("work", work_id)));
            }
        }
    }
    let left_out: Vec<&str> = package
        .cards
        .iter()
        .enumerate()
        .filter(|(index, _)| !kept(items, "card", *index))
        .map(|(_, card)| card.handle.as_str())
        .collect();

    // The kind of the card an item names: a card of this proposal, or one of
    // the canon.
    let kind_of = |named: &str| -> Result<String> {
        if let Some(kind) = kinds_by_handle.get(named) {
            return Ok(kind.clone());
        }
        if left_out.contains(&named) {
            return Err(Error::refused("canon.onACardLeftOut"));
        }
        let (card, _) = canon::fact::card_of(conn, config, named)?;
        Ok(card.kind)
    };

    for (index, fact) in package.facts.iter().enumerate() {
        if !kept(items, "fact", index) {
            continue;
        }
        match fact.change {
            canon::proposal::FactChange::Add => {
                let card = fact.card.as_deref().unwrap_or_default();
                match kind_of(card) {
                    Ok(kind) => {
                        let section = fact.section.as_deref().unwrap_or_default();
                        let found = config
                            .card_kind(&kind)
                            .and_then(|kind| kind.section(section))
                            .filter(|section| section.shape.holds_facts());
                        if found.is_none() {
                            note(Err(Error::refused("canon.unknownSection")
                                .param("section", section)
                                .param("kind", kind)));
                        }
                    }
                    Err(cause) => note(Err(cause)),
                }
            }
            canon::proposal::FactChange::Refine | canon::proposal::FactChange::Retire => {
                let id = fact.fact_id.as_deref().unwrap_or_default();
                if canon::fact::get(conn, id)?.is_none() {
                    note(Err(Error::not_found("fact", id)));
                }
            }
        }
    }

    for (index, link) in package.links.iter().enumerate() {
        if !kept(items, "relation", index) {
            continue;
        }
        note(kind_of(&link.from).map(|_| ()));
        note(kind_of(&link.to).map(|_| ()));
    }
    Ok(problems)
}

/// Keep the chosen items of a checked proposal for the canon, through the
/// gestures a hand uses: the cards first, so the facts and relations naming
/// them by handle have somewhere to land.
pub fn keep_package(
    conn: &Connection,
    package: canon::proposal::Package,
    items: Option<&[String]>,
    outcome: &mut super::proposal::Outcome,
) -> Result<()> {
    let mut made = std::collections::BTreeMap::new();
    for (index, card) in package.cards.into_iter().enumerate() {
        if !kept(items, "card", index) {
            continue;
        }
        let created = create_card(
            conn,
            NewNote {
                body: card.note.unwrap_or_default(),
                kind: Some(card.kind),
                title: Some(card.title),
                work_id: card.work_id,
                tags: Vec::new(),
                layer: card.layer,
                aliases: card.aliases,
            },
        )?;
        made.insert(card.handle, created.id.clone());
        outcome.cards.push(created.id);
    }
    let resolve = |named: &str| made.get(named).cloned().unwrap_or_else(|| named.to_owned());

    for (index, fact) in package.facts.into_iter().enumerate() {
        if !kept(items, "fact", index) {
            continue;
        }
        let kept_fact = match fact.change {
            canon::proposal::FactChange::Add => add_fact(
                conn,
                NewFact {
                    note_id: resolve(fact.card.as_deref().unwrap_or_default()),
                    section: fact.section.unwrap_or_default(),
                    body: fact.body.unwrap_or_default(),
                    layer: fact.layer,
                    // What the assistant proposes is a draft until the person
                    // settles it, and a draft reaches no cover and no public
                    // text: keeping a package is not reading every line of
                    // it. A live zone stays one.
                    status: Some(match fact.status {
                        Some(FactStatus::Open) => FactStatus::Open,
                        _ => FactStatus::Draft,
                    }),
                    retired_reason: None,
                    source: fact.source,
                    when: fact.when,
                    scope_work_id: None,
                    data: fact.data,
                },
            )?,
            canon::proposal::FactChange::Refine => update_fact(
                conn,
                fact.fact_id.as_deref().unwrap_or_default(),
                FactPatch {
                    body: fact.body,
                    layer: fact.layer,
                    // Settling is the person's: a proposal may leave a fact
                    // open or send it back to draft, never make it canon.
                    status: fact.status.filter(|status| *status != FactStatus::Canon),
                    source: fact.source.map(Some),
                    when: fact.when.map(Some),
                    data: (!fact.data.is_empty()).then_some(fact.data),
                    ..FactPatch::default()
                },
            )?,
            canon::proposal::FactChange::Retire => retire_fact(
                conn,
                fact.fact_id.as_deref().unwrap_or_default(),
                fact.reason.as_deref().unwrap_or_default(),
            )?,
        };
        outcome.facts.push(kept_fact.id);
    }

    for (index, link) in package.links.into_iter().enumerate() {
        if !kept(items, "relation", index) {
            continue;
        }
        let from = resolve(&link.from);
        let to = resolve(&link.to);
        let kept_link = match canon::link::between(conn, &from, &to)? {
            // The pair already stands in a relation: it is redrawn in the
            // words proposed, each on the side it belongs to.
            Some(standing) => {
                let (label, back_label) = if standing.from_id == from {
                    (link.label, link.back_label)
                } else {
                    (link.back_label, link.label)
                };
                update_relation(
                    conn,
                    &standing.id,
                    CanonLinkPatch {
                        kind: link.kind.map(Some),
                        label: label.map(Some),
                        back_label: back_label.map(Some),
                        layer: link.layer,
                    },
                )?
            }
            None => relate(
                conn,
                NewCanonLink {
                    from_id: from,
                    to_id: to,
                    kind: link.kind,
                    label: link.label,
                    back_label: link.back_label,
                    layer: link.layer,
                },
            )?,
        };
        outcome.relations.push(kept_link.id);
    }
    Ok(())
}
