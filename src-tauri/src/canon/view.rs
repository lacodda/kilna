//! Reading the canon: the list of cards, one card whole, a card read for a
//! task, the timeline - and the text a prompt is given.

use std::collections::BTreeMap;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use super::appearances::{self, Appearance};
use super::{CanonLink, Fact, FactStatus, Layer, fact, fingerprint, lenses_of, link, seen_by};
use crate::asset::Asset;
use crate::error::{Error, Result};
use crate::note::Note;
use crate::profile::config::{Lens, NoteKind, SectionShape};

/// A card as the list shows it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct CardSummary {
    pub id: String,
    pub kind: String,
    pub title: Option<String>,
    /// The work the card lives at - the hero of one song - when it does.
    pub work_id: Option<String>,
    pub work_title: Option<String>,
    pub layer: Layer,
    pub aliases: Vec<String>,
    /// Facts that are not retired.
    pub facts: usize,
    pub drafts: usize,
    pub open: usize,
    /// The newest portrait, for the list's avatar.
    pub portrait: Option<String>,
    pub updated_at: String,
}

/// Narrowing the list of cards. Every field may be left out.
#[derive(Debug, Clone, Default, Deserialize, ts_rs::TS)]
#[ts(optional_fields = nullable)]
pub struct CardFilter {
    pub kind: Option<String>,
    /// A case-insensitive piece of a name, an alias or a fact.
    pub search: Option<String>,
    /// Only the cards living at this work.
    pub work_id: Option<String>,
    /// Only cards holding a fact in this state: the drafts to settle, the
    /// live zones to build on.
    pub status: Option<FactStatus>,
}

/// Every card of a profile: the root first, then kind by kind in the order
/// the profile lists them, then by name.
pub fn cards(conn: &Connection, profile_id: &str, filter: &CardFilter) -> Result<Vec<CardSummary>> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let notes = crate::note::list(
        conn,
        profile_id,
        &crate::note::NoteFilter {
            canon: Some(true),
            kind: filter.kind.clone(),
            work_id: filter.work_id.clone(),
            ..crate::note::NoteFilter::default()
        },
    )?;
    let facts = fact::for_profile(conn, profile_id)?;
    let mut by_card: BTreeMap<&str, Vec<&Fact>> = BTreeMap::new();
    for one in &facts {
        by_card.entry(one.note_id.as_str()).or_default().push(one);
    }
    let portraits = portraits(conn, profile_id)?;
    let needle = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(crate::search::fold);

    let mut out = Vec::new();
    for note in notes {
        let own = by_card.get(note.id.as_str()).cloned().unwrap_or_default();
        if let Some(needle) = &needle {
            let hit = note
                .title
                .as_deref()
                .is_some_and(|t| crate::search::matches(t, needle))
                || note
                    .aliases
                    .iter()
                    .any(|a| crate::search::matches(a, needle))
                || crate::search::matches(&note.body, needle)
                || own.iter().any(|f| crate::search::matches(&f.body, needle));
            if !hit {
                continue;
            }
        }
        let count = |status: FactStatus| own.iter().filter(|f| f.status == status).count();
        if let Some(status) = filter.status {
            if count(status) == 0 {
                continue;
            }
        }
        let work_title = match note.work_id.as_deref() {
            Some(id) => crate::work::get(conn, id)?.map(|w| w.title),
            None => None,
        };
        out.push(CardSummary {
            facts: own
                .iter()
                .filter(|f| f.status != FactStatus::Retired)
                .count(),
            drafts: count(FactStatus::Draft),
            open: count(FactStatus::Open),
            portrait: portraits.get(&note.id).cloned(),
            id: note.id,
            kind: note.kind,
            title: note.title,
            work_id: note.work_id,
            work_title,
            layer: note.layer,
            aliases: note.aliases,
            updated_at: note.updated_at,
        });
    }

    let order = |kind: &str| {
        config
            .note_kinds
            .iter()
            .position(|k| k.key == kind)
            .map_or(usize::MAX, |at| {
                if config.note_kinds[at].root {
                    0
                } else {
                    at + 1
                }
            })
    };
    out.sort_by(|a, b| {
        order(&a.kind).cmp(&order(&b.kind)).then_with(|| {
            let name = |c: &CardSummary| c.title.clone().unwrap_or_default().to_lowercase();
            name(a).cmp(&name(b))
        })
    });
    Ok(out)
}

/// The newest portrait of every card that has one.
fn portraits(conn: &Connection, profile_id: &str) -> Result<BTreeMap<String, String>> {
    let mut statement = conn.prepare(
        "SELECT note_id, path FROM asset
          WHERE profile_id = ?1 AND note_id IS NOT NULL AND kind = 'portrait'
          ORDER BY created_at, rowid",
    )?;
    let rows = statement
        .query_map(params![profile_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    // Oldest first, so the newest of each card is the one left standing.
    Ok(rows.into_iter().collect())
}

/// A fact, with the tasks that may read it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct ReadFact {
    pub fact: Fact,
    pub lenses: Vec<Lens>,
}

/// A relation seen from one card: the other card, the words this side uses,
/// and the section of the card that gathers it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Relation {
    pub link: CanonLink,
    pub other_id: String,
    pub other_title: Option<String>,
    pub other_kind: String,
    pub other_layer: Layer,
    /// What the other card is to this one.
    pub label: Option<String>,
    /// The section of relations that gathers it, when the card has one.
    pub section: Option<String>,
    pub lenses: Vec<Lens>,
}

/// One card whole: the note, its facts by section, its relations, its
/// pictures, where it appears, and whether its description is stale.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct CardView {
    pub card: Note,
    pub work_title: Option<String>,
    pub facts: Vec<ReadFact>,
    pub relations: Vec<Relation>,
    pub pictures: Vec<Asset>,
    pub appearances: Vec<Appearance>,
    /// What the description would be written from now, as a fingerprint.
    /// Absent for a kind described by hand.
    pub basis: Option<String>,
    /// The description no longer matches the facts it was written from.
    pub prompt_stale: bool,
    /// Which lenses see the card at all.
    pub lenses: Vec<Lens>,
}

/// One card, read whole.
pub fn card(conn: &Connection, note_id: &str) -> Result<CardView> {
    let note = crate::note::get(conn, note_id)?.ok_or_else(|| Error::not_found("note", note_id))?;
    let config = crate::profile::config_for(conn, &note.profile_id)?;
    let (_, kind) = fact::card_of(conn, &config, note_id)?;

    let facts = fact::for_card(conn, note_id)?
        .into_iter()
        .map(|one| ReadFact {
            lenses: lenses_of(
                note.layer,
                kind.section(&one.section),
                one.layer,
                one.status,
            ),
            fact: one,
        })
        .collect::<Vec<_>>();
    let relations = relations_of(conn, &note, kind)?;
    let pictures = crate::asset::for_card(conn, note_id)?;
    let appearances = appearances::of_card(conn, &note)?;
    let basis = basis_of(conn, &note, kind)?;
    let prompt_stale = note.prompt.is_some()
        && basis.is_some()
        && note.prompt_basis.as_deref() != basis.as_deref();
    let work_title = match note.work_id.as_deref() {
        Some(id) => crate::work::get(conn, id)?.map(|w| w.title),
        None => None,
    };
    let lenses = Lens::ALL
        .into_iter()
        .filter(|lens| *lens == Lens::Work || note.layer == Layer::Public)
        .collect();

    Ok(CardView {
        card: note,
        work_title,
        facts,
        relations,
        pictures,
        appearances,
        basis,
        prompt_stale,
        lenses,
    })
}

/// The relations of a card, each placed in the section that gathers it.
fn relations_of(conn: &Connection, note: &Note, kind: &NoteKind) -> Result<Vec<Relation>> {
    let mut out = Vec::new();
    for one in link::for_card(conn, &note.id)? {
        let other_id = one.other(&note.id).to_owned();
        let Some(other) = crate::note::get(conn, &other_id)? else {
            continue;
        };
        let section = section_for(kind, &other.kind);
        let lenses = Lens::ALL
            .into_iter()
            .filter(|lens| {
                *lens == Lens::Work
                    || (other.layer == Layer::Public
                        && seen_by(*lens, note.layer, section, one.layer, FactStatus::Canon))
            })
            .collect();
        out.push(Relation {
            label: one.label_from(&note.id).map(str::to_owned),
            section: section.map(|s| s.key.clone()),
            other_title: other.title.clone(),
            other_kind: other.kind.clone(),
            other_layer: other.layer,
            other_id,
            link: one,
            lenses,
        });
    }
    Ok(out)
}

/// The section of relations that gathers a relation to a card of `other`:
/// the first that names the kind, or else the one that names none.
fn section_for<'k>(
    kind: &'k NoteKind,
    other: &str,
) -> Option<&'k crate::profile::config::CanonSection> {
    let relations = || {
        kind.sections
            .iter()
            .filter(|s| s.shape == SectionShape::Relations)
    };
    relations()
        .find(|s| s.kinds.iter().any(|k| k == other))
        .or_else(|| relations().find(|s| s.kinds.is_empty()))
}

/// The fingerprint of what a card's description is written from: the settled
/// public facts of the kind's `describe_from` sections, in order.
///
/// Absent for a kind that names none - its description is written by hand
/// and cannot go stale.
pub fn basis_of(conn: &Connection, note: &Note, kind: &NoteKind) -> Result<Option<String>> {
    if kind.describe_from.is_empty() {
        return Ok(None);
    }
    let mut text = String::new();
    for section in &kind.describe_from {
        for one in fact::in_section(conn, &note.id, section)? {
            if one.status == FactStatus::Canon && one.layer == Layer::Public {
                text.push_str(section);
                text.push('\u{1f}');
                text.push_str(&one.body);
                text.push('\n');
            }
        }
    }
    Ok(Some(fingerprint(&text)))
}

/// A fact on the timeline, with the card it belongs to.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Dated {
    pub fact: Fact,
    pub card_title: Option<String>,
    pub card_kind: String,
    pub card_layer: Layer,
    /// The tasks that may read it, by the one rule (`seen_by`).
    pub lenses: Vec<Lens>,
}

/// The facts dated inside the world, earliest first - of one card, or of the
/// whole canon.
pub fn timeline(conn: &Connection, profile_id: &str, note_id: Option<&str>) -> Result<Vec<Dated>> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let mut cards: BTreeMap<String, Note> = BTreeMap::new();
    let mut out = Vec::new();
    for one in fact::dated(conn, profile_id, note_id)? {
        if !cards.contains_key(&one.note_id) {
            if let Some(card) = crate::note::get(conn, &one.note_id)? {
                cards.insert(card.id.clone(), card);
            }
        }
        let Some(card) = cards.get(&one.note_id) else {
            continue;
        };
        let section = config
            .card_kind(&card.kind)
            .and_then(|kind| kind.section(&one.section));
        out.push(Dated {
            card_title: card.title.clone(),
            card_kind: card.kind.clone(),
            card_layer: card.layer,
            lenses: lenses_of(card.layer, section, one.layer, one.status),
            fact: one,
        });
    }
    Ok(out)
}

/// Replace every `[[card:id]]` in `text` with what a generator is given for
/// that card: its description, or its name while it has none.
///
/// A template of a signature detail says "hidden detail: [[card:…]] hides in
/// the picture", and the picture needs the cat described, not named - a model
/// does not know who Tabby is.
pub fn expand_cards(conn: &Connection, text: &str) -> Result<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("[[card:") {
        out.push_str(&rest[..start]);
        let after = &rest[start + "[[card:".len()..];
        let Some(end) = after.find("]]") else {
            out.push_str(&rest[start..]);
            return Ok(out);
        };
        let id = &after[..end];
        match crate::note::get(conn, id)? {
            Some(card) => out.push_str(
                card.prompt
                    .as_deref()
                    .or(card.title.as_deref())
                    .unwrap_or_default(),
            ),
            None => out.push_str(&rest[start..start + "[[card:".len() + end + 2]),
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// A card as a prompt reads it, through `lens`: the header, then every
/// section the lens reads with the facts it may see, then the relations.
///
/// Read by the lens function and nothing else, so what the assistant is
/// handed is what the screen shows through the same lens. Through the work's
/// lens the layers are said beside each fact, with what each allows: the
/// internal layer gives details without their addresses, the last layer
/// stays inside the work.
pub fn render(conn: &Connection, note_id: &str, lens: Lens) -> Result<String> {
    let view = card(conn, note_id)?;
    let config = crate::profile::config_for(conn, &view.card.profile_id)?;
    let kind = config.card_kind(&view.card.kind).ok_or_else(|| {
        Error::refused("canon.notACard").param("title", view.card.title.clone().unwrap_or_default())
    })?;
    if lens != Lens::Work && view.card.layer != Layer::Public {
        return Ok(String::new());
    }

    let title = view.card.title.clone().unwrap_or_default();
    let mut out = format!("## {title} — {} (card `{}`)\n", kind.label, view.card.id);
    if lens == Lens::Work && view.card.layer != Layer::Public {
        out.push_str(&format!(
            "The card itself: {}\n",
            layer_rule(view.card.layer)
        ));
    }
    if !view.card.aliases.is_empty() {
        out.push_str(&format!("Also called: {}\n", view.card.aliases.join(", ")));
    }
    if let Some(prompt) = view.card.prompt.as_deref() {
        out.push_str(&format!("For a picture: {prompt}\n"));
    }

    for section in &kind.sections {
        if !section.read_by(lens) {
            continue;
        }
        let mut lines = Vec::new();
        match section.shape {
            SectionShape::Relations => {
                for relation in view
                    .relations
                    .iter()
                    .filter(|r| r.section.as_deref() == Some(section.key.as_str()))
                    .filter(|r| r.lenses.contains(&lens))
                {
                    let mut line = format!(
                        "- {} (card `{}`)",
                        relation.other_title.clone().unwrap_or_default(),
                        relation.other_id
                    );
                    if let Some(label) = &relation.label {
                        line.push_str(&format!(" — {label}"));
                    }
                    if lens == Lens::Work && relation.link.layer != Layer::Public {
                        line.push_str(&format!(" [{}]", layer_rule(relation.link.layer)));
                    }
                    lines.push(line);
                }
            }
            SectionShape::Appearances => {}
            shape => {
                for read in view
                    .facts
                    .iter()
                    .filter(|f| f.fact.section == section.key && f.lenses.contains(&lens))
                {
                    lines.push(fact_line(&read.fact, shape, lens));
                }
            }
        }
        if lines.is_empty() {
            continue;
        }
        out.push_str(&format!("### {}\n{}\n", section.label, lines.join("\n")));
    }
    Ok(out.trim_end().to_owned())
}

/// One fact as a line of a prompt.
fn fact_line(one: &Fact, shape: SectionShape, lens: Lens) -> String {
    let text = |key: &str| {
        one.data
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
    };
    let mut line = match shape {
        SectionShape::Slots => format!("- {}: {}", text("slot"), one.body),
        SectionShape::Palette => format!("- {} {}", text("color"), one.body),
        SectionShape::Details => format!("- {}: {}", one.body, text("template")),
        SectionShape::Marks => {
            let mut line = format!("- {}", one.body);
            if !text("code").is_empty() {
                line = format!("- {} {}", text("code"), one.body);
            }
            if !text("prompt").is_empty() {
                line.push_str(&format!(" ({})", text("prompt")));
            }
            line
        }
        _ => format!("- {}", one.body),
    };
    if let Some(when) = &one.when {
        if let Some(label) = when.label.as_deref().or(when.sort.as_deref()) {
            line.push_str(&format!(" ({label})"));
        }
    }
    if lens == Lens::Work {
        if one.layer != Layer::Public {
            line.push_str(&format!(" [{}]", layer_rule(one.layer)));
        }
        match one.status {
            FactStatus::Draft => line.push_str(" [a draft, not settled]"),
            FactStatus::Open => line.push_str(" [open: free to build on]"),
            FactStatus::Canon | FactStatus::Retired => {}
        }
    }
    line
}

/// What a layer allows, in the words a prompt gives the assistant.
fn layer_rule(layer: Layer) -> &'static str {
    match layer {
        Layer::Public => "public",
        Layer::Internal => "internal: take the detail, never the name, the date or the address",
        Layer::InWorks => "only inside the work itself: never outside it",
    }
}

/// The fingerprint a card's description would answer to now - [`basis_of`]
/// for a card named by id.
pub fn basis_for(conn: &Connection, note_id: &str) -> Result<Option<String>> {
    let note = crate::note::get(conn, note_id)?.ok_or_else(|| Error::not_found("note", note_id))?;
    let config = crate::profile::config_for(conn, &note.profile_id)?;
    let (_, kind) = fact::card_of(conn, &config, note_id)?;
    basis_of(conn, &note, kind)
}
