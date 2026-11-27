//! The canon: the world the works share, kept as facts on cards.

use rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::assistant::apply;
use crate::assistant::proposal::Proposal;
use crate::error::{Error, Result};
use crate::mcp::call::{Call, Page, arg, find_work, pretty};
use crate::mcp::deliver::deliver;
use crate::mcp::schema::section_schema;
use crate::profile::config::SectionShape;
use crate::speech::Speech;
use crate::{profile, search};

/// `canon`: the world the works share, card by card.
pub(super) fn canon(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        config,
        args,
        ..
    } = *call;
    read_canon(conn, &profile.id, config, args)
}

/// The `canon` tool: the list of cards, one card through a lens, a search of
/// the facts, or the timeline.
fn read_canon(
    conn: &Connection,
    profile_id: &str,
    config: &profile::config::ProfileConfig,
    args: &Map<String, Value>,
) -> Result<String> {
    use crate::canon::{proposal::CardIndex, view};

    let lens = match arg(args, "lens") {
        Some(word) => Some(
            profile::config::Lens::parse(word)
                .ok_or_else(|| Error::refused("mcp.unknownLens").param("lens", word))?,
        ),
        None => None,
    };
    let index = CardIndex::of(conn, profile_id)?;
    let card = match arg(args, "card") {
        Some(named) => Some(
            index
                .find_or_root(named, config)
                .cloned()
                .ok_or_else(|| Error::refused("canon.unknownCard").param("card", named))?,
        ),
        None => None,
    };

    if args.get("timeline").and_then(Value::as_bool) == Some(true) {
        let dated = view::timeline(conn, profile_id, card.as_ref().map(|c| c.id.as_str()))?;
        let facts = dated
            .into_iter()
            .map(|one| {
                json!({
                    "card": one.fact.note_id, "card_title": one.card_title, "fact": one.fact.id,
                    "text": one.fact.body, "when": one.fact.when, "layer": one.fact.layer,
                    "status": one.fact.status, "read_by": one.lenses,
                })
            })
            .collect::<Vec<_>>();
        return pretty(&Page::of(args, 200).cut("facts", facts));
    }

    if let Some(query) = arg(args, "query") {
        let needle = search::fold(query);
        let mut hits = Vec::new();
        for one in crate::canon::fact::for_profile(conn, profile_id)? {
            if !search::matches(&one.body, &needle) {
                continue;
            }
            let title = crate::note::get(conn, &one.note_id)?.and_then(|c| c.title);
            hits.push(json!({
                "fact": one.id, "card": one.note_id, "card_title": title, "section": one.section,
                "text": one.body, "layer": one.layer, "status": one.status, "when": one.when,
                "source": one.source,
            }));
        }
        return pretty(&Page::of(args, 100).cut("facts", hits));
    }

    if let Some(card) = card {
        let seen = view::card(conn, &card.id)?;
        let kind = config.card_kind(&seen.card.kind);
        // A long card is read in parts (v0.144, FR-18 18.5): its head and
        // counts alone, one section, one state, a page of facts.
        if args.get("summary").and_then(Value::as_bool) == Some(true) {
            let counts: Vec<Value> = kind
                .map(|k| {
                    k.sections
                        .iter()
                        .map(|s| {
                            let held = seen.facts.iter().filter(|r| r.fact.section == s.key);
                            json!({
                                "key": s.key, "label": s.label, "shape": s.shape,
                                "facts": held.clone().count(),
                                "drafts": held.filter(|r| r.fact.status == crate::canon::FactStatus::Draft).count(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            return pretty(&json!({
                "id": seen.card.id, "kind": seen.card.kind, "title": seen.card.title,
                "aliases": seen.card.aliases, "layer": seen.card.layer,
                "lives_at_work": seen.card.work_id, "note": seen.card.body,
                "total_facts": seen.facts.len(), "relations": seen.relations.len(),
                "sections": counts,
            }));
        }
        let wanted_section = arg(args, "section");
        let wanted_status = match arg(args, "status") {
            Some(word) => Some(
                crate::canon::FactStatus::from_word(word)
                    .ok_or_else(|| Error::refused("canon.unknownStatus").param("status", word))?,
            ),
            None => None,
        };
        let narrowed: Vec<&view::ReadFact> = seen
            .facts
            .iter()
            .filter(|read| lens.is_none_or(|lens| read.lenses.contains(&lens)))
            .filter(|read| wanted_section.is_none_or(|key| read.fact.section == key))
            .filter(|read| wanted_status.is_none_or(|status| read.fact.status == status))
            .collect();
        let total_facts = narrowed.len();
        let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .map(|n| n as usize);
        let page: Vec<&&view::ReadFact> = narrowed
            .iter()
            .skip(offset)
            .take(limit.unwrap_or(usize::MAX))
            .collect();
        let has_more = offset + page.len() < total_facts;
        let facts: Vec<Value> = page
            .into_iter()
            .map(|read| {
                let one = &read.fact;
                json!({
                    "id": one.id, "section": one.section, "text": one.body, "layer": one.layer,
                    "status": one.status, "retired_because": one.retired_reason,
                    "source": one.source, "when": one.when, "only_for_work": one.scope_work_id,
                    "data": one.data, "read_by": read.lenses,
                })
            })
            .collect();
        let relations: Vec<Value> = seen
            .relations
            .iter()
            .filter(|relation| lens.is_none_or(|lens| relation.lenses.contains(&lens)))
            .map(|relation| {
                json!({
                    "card": relation.other_id, "title": relation.other_title,
                    "kind": relation.link.kind, "is": relation.label, "layer": relation.link.layer,
                    "section": relation.section,
                })
            })
            .collect();
        if lens.is_some_and(|lens| !seen.lenses.contains(&lens)) {
            return Ok(
                "This card is not seen through that lens: publicly it does not exist.".into(),
            );
        }
        return pretty(&json!({
            "id": seen.card.id, "kind": seen.card.kind, "title": seen.card.title,
            "aliases": seen.card.aliases, "layer": seen.card.layer,
            "lives_at_work": seen.card.work_id, "note": seen.card.body,
            "description_for_pictures": seen.card.prompt, "description_stale": seen.prompt_stale,
            "sections": kind.map(|k| k.sections.iter().map(|s| json!({
                "key": s.key, "label": s.label, "shape": s.shape, "read_by": s.lenses,
            })).collect::<Vec<_>>()),
            "facts": facts,
            "total_facts": total_facts,
            "has_more": has_more,
            "relations": relations,
            "pictures": seen.pictures.iter().map(|p| json!({
                "role": p.kind, "path": p.path, "of_fact": p.canon_fact_id,
            })).collect::<Vec<_>>(),
            "appears_in": seen.appearances,
        }));
    }

    let cards = view::cards(
        conn,
        profile_id,
        &view::CardFilter {
            kind: arg(args, "kind").map(str::to_owned),
            ..view::CardFilter::default()
        },
    )?;
    // The cards a page at a time (v0.171): a world of five hundred cards is
    // walked, not swallowed. Short, they are their names alone - and the
    // vocabulary of kinds and shapes, said once, is left out.
    let page = Page::of(args, 200);
    let root = index.root(config);
    if page.summary {
        let mut answer = page.cut(
            "cards",
            cards
                .iter()
                .map(|c| json!({ "id": c.id, "kind": c.kind, "title": c.title, "layer": c.layer }))
                .collect(),
        );
        answer.insert("has_root_card".into(), json!(root.is_some()));
        answer.insert(
            "root_card_id".into(),
            json!(root.map(|card| card.id.clone())),
        );
        return pretty(&answer);
    }
    // Whether the space has its passport, and what each shape of section
    // carries beside an entry's words (v0.144).
    let schemas: Map<String, Value> = [
        SectionShape::Facts,
        SectionShape::Slots,
        SectionShape::Details,
        SectionShape::Palette,
        SectionShape::Marks,
        SectionShape::Styles,
        SectionShape::Bans,
        SectionShape::Branding,
        SectionShape::Quotas,
        SectionShape::Bank,
    ]
    .into_iter()
    .map(|shape| {
        let said = serde_json::to_value(shape).unwrap_or_default();
        let key = said.as_str().unwrap_or_default().to_owned();
        (key, section_schema(shape))
    })
    .collect();
    let listed = page.cut(
        "cards",
        cards
            .iter()
            .map(|c| {
                json!({
                    "id": c.id, "kind": c.kind, "title": c.title, "aliases": c.aliases,
                    "layer": c.layer, "lives_at_work": c.work_title, "facts": c.facts,
                    "drafts": c.drafts, "open": c.open,
                })
            })
            .collect(),
    );
    let mut answer = json!({
        "has_root_card": root.is_some(),
        "root_card_id": root.map(|card| card.id.clone()),
        "section_schemas": schemas,
        "kinds": config.note_kinds.iter().filter(|k| k.is_card()).map(|k| json!({
            "key": k.key, "label": k.label, "root": k.root,
            "sections": k.sections.iter().map(|s| json!({
                "key": s.key, "label": s.label, "shape": s.shape, "read_by": s.lenses,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "relation_kinds": config.relation_kinds,
    });
    if let Some(object) = answer.as_object_mut() {
        object.extend(listed);
    }
    pretty(&answer)
}

/// `propose_canon`: cards, facts, relations, pictures and descriptions for
/// the canon.
pub(super) fn propose_canon(call: &Call) -> Result<String> {
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
    let defaults = crate::canon::proposal::Defaults {
        card_id: None,
        work_id: found.as_ref().map(|w| w.id.clone()),
        version_id: found.as_ref().and_then(|w| w.current_version_id.clone()),
    };
    let package =
        crate::canon::proposal::read(conn, &profile.id, &Value::Object(args.clone()), &defaults)?;
    let body = apply::render_canon(conn, &package, Speech::ENGLISH);
    let summary = format!(
        "{} card(s), {} card update(s), {} fact(s), {} relation(s), {} picture(s), {} description(s)",
        package.cards.len(),
        package.card_updates.len(),
        package.facts.len(),
        package.links.len(),
        package.pictures.len(),
        package.descriptions.len()
    );
    // A new fact the card nearly holds is said back to the agent
    // (v0.144), beside what it proposed.
    let similar: Vec<String> = package
        .facts
        .iter()
        .enumerate()
        .filter_map(|(index, fact)| {
            fact.similar_to.as_ref().map(|held| {
                format!(
                    "fact {} is similar_to {} ({}): {}",
                    index + 1,
                    held.fact_id,
                    held.section,
                    held.text
                )
            })
        })
        .collect();
    let dropped: Vec<String> = package
        .dropped
        .iter()
        .map(|reason| {
            let code = reason.key.strip_prefix("refusal.").unwrap_or(&reason.key);
            crate::error::english(code, &reason.params)
        })
        .collect();
    deliver(
        conn,
        &profile.id,
        session,
        found.as_ref(),
        Proposal::Canon { package },
        &body,
        None,
    )?;
    let mut answer = format!(
        "Proposed {summary} for the canon. It waits in the chat {}; the person applies it whole or item by item.",
        if found.is_some() {
            "on the work"
        } else {
            "named after you"
        }
    );
    if !dropped.is_empty() {
        answer.push_str(&format!(" Left out: {}.", dropped.join("; ")));
    }
    if !similar.is_empty() {
        answer.push_str(&format!(" Marked: {}.", similar.join("; ")));
    }
    Ok(answer)
}
