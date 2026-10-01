//! What the assistant proposes for the canon: new cards, facts to add, refine
//! or retire, and relations - read out of an answer's block or an agent's
//! arguments, said to the model as an instruction, and reviewed against the
//! canon before a person applies it (ADR 0043).
//!
//! Nothing here writes. A proposal is a message; applying it is
//! `actions::proposal`, through the same gestures a hand uses.

use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{Fact, FactStatus, Layer, Source, SourceKind, When, fact};
use crate::error::{Error, Reason, Result};
use crate::note::Note;
use crate::profile::config::{ProfileConfig, SectionShape};

/// A card the proposal would make.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct ProposedCard {
    /// How the facts and relations of the same proposal name it: `new-1`.
    pub handle: String,
    pub kind: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional = nullable)]
    pub aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<Layer>,
    /// The work it lives at - a hero of one song.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_id: Option<String>,
    /// Its free note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// What a proposed fact does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum FactChange {
    /// A new fact.
    #[default]
    Add,
    /// A sharper wording of a fact the canon holds.
    Refine,
    /// A fact that is no longer true, and why.
    Retire,
}

/// A fact the canon already holds that a proposed one contradicts, and why -
/// said by the assistant, shown beside both before anything is applied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Contradiction {
    pub fact_id: String,
    pub why: String,
}

/// A fact the proposal would add, refine or retire.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct ProposedFact {
    #[serde(default)]
    pub change: FactChange,
    /// The fact refined or retired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fact_id: Option<String>,
    /// The card a new fact goes onto: its id, or the handle of a card of this
    /// proposal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<Layer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<FactStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<When>,
    /// Why a fact is retired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    #[ts(optional = nullable)]
    pub data: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional = nullable)]
    pub contradicts: Vec<Contradiction>,
}

/// A relation the proposal would draw, or redraw when the two cards already
/// stand in one.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct ProposedLink {
    /// A card id or a handle.
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<Layer>,
}

/// A picture the proposal would attach to a card, or to one fact of it - a
/// file on this machine, copied into the workspace when the person keeps it
/// (v0.89.2).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct ProposedPicture {
    /// The file, as a path on this machine.
    pub path: String,
    /// The card it shows: an id, or the handle of a card of this proposal.
    pub card: String,
    /// The card's name, for the person reading the proposal.
    #[serde(default)]
    pub card_title: String,
    /// The fact of the card it shows - an outfit - when it shows one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fact_id: Option<String>,
    /// One of the picture roles; `reference` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// The description a generator is given for a card, proposed whole
/// (v0.89.2). The fingerprint of the facts it answers to is taken when the
/// person keeps it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct ProposedDescription {
    /// A card id.
    pub card: String,
    #[serde(default)]
    pub card_title: String,
    pub text: String,
}

/// A proposal for the canon, read and checked against the profile.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Package {
    pub cards: Vec<ProposedCard>,
    pub facts: Vec<ProposedFact>,
    pub links: Vec<ProposedLink>,
    #[serde(default)]
    pub pictures: Vec<ProposedPicture>,
    #[serde(default)]
    pub descriptions: Vec<ProposedDescription>,
    /// What the answer said that the canon has no place for, and why - shown
    /// rather than applied, and rather than silently dropped.
    pub dropped: Vec<Reason>,
}

impl Package {
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
            && self.facts.is_empty()
            && self.links.is_empty()
            && self.pictures.is_empty()
            && self.descriptions.is_empty()
    }
}

/// The files a proposed picture may be: what a generator takes as a
/// reference and the window can show.
const PICTURE_FILES: [&str; 6] = ["jpg", "jpeg", "png", "webp", "gif", "avif"];

/// Why a proposed picture's file cannot be one, if it cannot.
pub fn picture_file_problem(path: &str) -> Option<Error> {
    let file = std::path::Path::new(path);
    if !file.is_absolute() || !file.is_file() {
        return Some(Error::refused("canon.pictureNotAFile").param("path", path));
    }
    let picture = file
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| PICTURE_FILES.contains(&ext.to_lowercase().as_str()));
    (!picture).then(|| Error::refused("canon.pictureNotAPicture").param("path", path))
}

/// What a proposal falls back on where an item says nothing.
#[derive(Debug, Clone, Default)]
pub struct Defaults {
    /// The card a fact with no card is about: the card an action was started
    /// on.
    pub card_id: Option<String>,
    /// The work the facts were read from, and the version: a fact's source
    /// when it names none, the home of a card said to live on the work.
    pub work_id: Option<String>,
    pub version_id: Option<String>,
}

/// Read a proposal out of the words of an answer's block or an agent's
/// arguments.
///
/// The words are the ones the instruction teaches - `text`, `line`, `when`,
/// `fact` - and each item is checked against the profile and the canon as it
/// stands: a card of a kind that is not a card's, a section its kind does not
/// have, a fact nobody holds. An item that fails is named in `dropped` and
/// left out; the rest is proposed. A block that yields nothing at all is
/// refused, so a button that would apply nothing never appears.
pub fn read(
    conn: &Connection,
    profile_id: &str,
    raw: &Value,
    defaults: &Defaults,
) -> Result<Package> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let object = raw
        .as_object()
        .ok_or_else(|| Error::refused("canon.proposalNotAnObject"))?;
    let mut package = Package::default();
    // The kind of every card a fact may name: those of the canon, and those
    // this proposal makes, by handle.
    let mut kinds_by_handle: BTreeMap<String, String> = BTreeMap::new();

    for (index, item) in list(object, "cards").iter().enumerate() {
        let place = format!("card {}", index + 1);
        let item = item.as_object().cloned().unwrap_or_default();
        let kind = text(&item, "kind").unwrap_or_default();
        if config.card_kind(&kind).is_none() {
            package.dropped.push(
                Reason::of("refusal.canon.unknownKind")
                    .param("kind", kind)
                    .param("item", place),
            );
            continue;
        }
        let Some(title) = text(&item, "title").or_else(|| text(&item, "name")) else {
            package
                .dropped
                .push(Reason::of("refusal.canon.cardNeedsName").param("item", place));
            continue;
        };
        let handle = text(&item, "handle").unwrap_or_else(|| format!("new-{}", index + 1));
        let on_work = item
            .get("on_work")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        kinds_by_handle.insert(handle.clone(), kind.clone());
        package.cards.push(ProposedCard {
            handle,
            kind,
            title,
            aliases: strings(&item, "aliases"),
            layer: text(&item, "layer").and_then(|l| Layer::from_word(&l)),
            work_id: text(&item, "work_id").or(if on_work {
                defaults.work_id.clone()
            } else {
                None
            }),
            note: text(&item, "note"),
        });
    }

    let cards = CardIndex::of(conn, profile_id)?;
    for (index, item) in list(object, "facts").iter().enumerate() {
        let place = format!("fact {}", index + 1);
        let item = item.as_object().cloned().unwrap_or_default();
        match read_fact(conn, &config, &cards, &kinds_by_handle, &item, defaults) {
            Ok(fact) => package.facts.push(fact),
            Err(error) => package.dropped.push(error.reason().param("item", place)),
        }
    }

    for (index, item) in list(object, "relations")
        .iter()
        .chain(list(object, "links").iter())
        .enumerate()
    {
        let place = format!("relation {}", index + 1);
        let item = item.as_object().cloned().unwrap_or_default();
        let resolve = |key: &str| {
            text(&item, key).and_then(|named| {
                if kinds_by_handle.contains_key(&named) {
                    Some(named)
                } else {
                    cards.find(&named).map(|card| card.id.clone())
                }
            })
        };
        let (Some(from), Some(to)) = (resolve("from"), resolve("to")) else {
            package
                .dropped
                .push(Reason::of("refusal.canon.relationNamesNoCard").param("item", place));
            continue;
        };
        if from == to {
            package
                .dropped
                .push(Reason::of("refusal.canon.relationToItself").param("item", place));
            continue;
        }
        package.links.push(ProposedLink {
            from,
            to,
            kind: text(&item, "kind"),
            label: text(&item, "label"),
            back_label: text(&item, "back_label"),
            layer: text(&item, "layer").and_then(|l| Layer::from_word(&l)),
        });
    }

    // A card a picture or a description names: one of this proposal, by
    // handle, or one of the canon, with its name for the person.
    let proposed: Vec<(String, String)> = package
        .cards
        .iter()
        .map(|card| (card.handle.clone(), card.title.clone()))
        .collect();
    let card_named = |named: &str| -> Option<(String, String)> {
        if let Some(card) = proposed.iter().find(|(handle, _)| handle == named) {
            return Some(card.clone());
        }
        cards
            .find(named)
            .map(|card| (card.id.clone(), card.title.clone().unwrap_or_default()))
    };

    let mut pictures = Vec::new();
    for (index, item) in list(object, "pictures").iter().enumerate() {
        let place = format!("picture {}", index + 1);
        let item = item.as_object().cloned().unwrap_or_default();
        let path = text(&item, "path").unwrap_or_default();
        if let Some(problem) = picture_file_problem(&path) {
            package.dropped.push(problem.reason().param("item", place));
            continue;
        }
        let role = text(&item, "role");
        if let Some(role) = role.as_deref()
            && !fact::PICTURE_ROLES.contains(&role)
        {
            package.dropped.push(
                Reason::of("refusal.canon.pictureRole")
                    .param("role", role)
                    .param("item", place),
            );
            continue;
        }
        // A picture of a fact is a picture of its card.
        let fact_id = text(&item, "fact");
        let named = match fact_id.as_deref() {
            Some(id) => match fact::get(conn, id)? {
                Some(found) if found.profile_id == profile_id => Some(found.note_id),
                _ => {
                    package.dropped.push(
                        Reason::of("refusal.canon.pictureNamesNoFact")
                            .param("fact", id)
                            .param("item", place),
                    );
                    continue;
                }
            },
            None => text(&item, "card"),
        };
        let Some((card, card_title)) = named.as_deref().and_then(card_named) else {
            package
                .dropped
                .push(Reason::of("refusal.canon.pictureNamesNoCard").param("item", place));
            continue;
        };
        pictures.push(ProposedPicture {
            path,
            card,
            card_title,
            fact_id,
            role,
        });
    }
    package.pictures = pictures;

    let mut descriptions = Vec::new();
    for (index, item) in list(object, "descriptions").iter().enumerate() {
        let place = format!("description {}", index + 1);
        let item = item.as_object().cloned().unwrap_or_default();
        let Some(said) = text(&item, "text").or_else(|| text(&item, "description")) else {
            package
                .dropped
                .push(Reason::of("refusal.canon.descriptionEmpty").param("item", place));
            continue;
        };
        // A description answers to the facts a card already holds: a card of
        // this proposal has none to answer to yet.
        let found = text(&item, "card").and_then(|named| cards.find(&named).cloned());
        let Some(card) = found else {
            package
                .dropped
                .push(Reason::of("refusal.canon.descriptionNamesNoCard").param("item", place));
            continue;
        };
        descriptions.push(ProposedDescription {
            card: card.id,
            card_title: card.title.unwrap_or_default(),
            text: said,
        });
    }
    package.descriptions = descriptions;

    if package.is_empty() {
        return Err(Error::refused("canon.proposalEmpty"));
    }
    Ok(package)
}

/// One fact out of an item.
fn read_fact(
    conn: &Connection,
    config: &ProfileConfig,
    cards: &CardIndex,
    handles: &BTreeMap<String, String>,
    item: &Map<String, Value>,
    defaults: &Defaults,
) -> Result<ProposedFact> {
    let change = match text(item, "change").as_deref() {
        None | Some("add") => FactChange::Add,
        Some("refine") => FactChange::Refine,
        Some("retire") => FactChange::Retire,
        Some(other) => return Err(Error::refused("canon.unknownChange").param("change", other)),
    };
    let layer = match text(item, "layer") {
        Some(word) => Some(
            Layer::from_word(&word)
                .ok_or_else(|| Error::refused("canon.unknownLayer").param("layer", word))?,
        ),
        None => None,
    };
    let status = match text(item, "status") {
        Some(word) => Some(
            FactStatus::from_word(&word)
                .filter(|status| *status != FactStatus::Retired)
                .ok_or_else(|| Error::refused("canon.unknownStatus").param("status", word))?,
        ),
        None => None,
    };
    let when = match (text(item, "when"), text(item, "sort")) {
        (None, None) => None,
        (label, sort) => {
            if let Some(sort) = &sort
                && !super::when::is_sort_key(sort)
            {
                return Err(Error::refused("canon.badWorldDate").param("value", sort.clone()));
            }
            Some(When { label, sort })
        }
    };
    let contradicts = list(item, "contradicts")
        .iter()
        .filter_map(|one| {
            let one = one.as_object()?;
            let fact_id = text(one, "fact")?;
            fact::get(conn, &fact_id).ok().flatten()?;
            Some(Contradiction {
                fact_id,
                why: text(one, "why").unwrap_or_default(),
            })
        })
        .collect();
    let body = text(item, "text").or_else(|| text(item, "body"));
    let data = item
        .get("data")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    match change {
        FactChange::Add => {
            let named = text(item, "card").or_else(|| defaults.card_id.clone());
            let (card, kind) = match named {
                Some(named) if handles.contains_key(&named) => {
                    let kind = handles[&named].clone();
                    (named, kind)
                }
                Some(named) => {
                    let found = cards
                        .find(&named)
                        .ok_or_else(|| Error::refused("canon.unknownCard").param("card", named))?;
                    (found.id.clone(), found.kind.clone())
                }
                None => return Err(Error::refused("canon.factNamesNoCard")),
            };
            let section =
                text(item, "section").ok_or_else(|| Error::refused("canon.factNeedsSection"))?;
            let kind = config.require_card_kind(&kind)?;
            let shape = kind.section(&section).map(|s| s.shape).ok_or_else(|| {
                Error::refused("canon.unknownSection")
                    .param("section", section.clone())
                    .param(
                        "kind",
                        serde_json::to_value(&kind.label).unwrap_or_default(),
                    )
            })?;
            if !shape.holds_facts() {
                return Err(Error::refused("canon.sectionHoldsNoFacts").param("section", section));
            }
            let body = body.ok_or_else(|| Error::refused("canon.factNeedsWords"))?;
            Ok(ProposedFact {
                change,
                card: Some(card),
                section: Some(section),
                body: Some(body),
                layer,
                status,
                source: source_of(item, defaults),
                when,
                data: fact::checked_data(conn, shape, data)?,
                contradicts,
                ..ProposedFact::default()
            })
        }
        FactChange::Refine | FactChange::Retire => {
            let fact_id = text(item, "fact")
                .or_else(|| text(item, "id"))
                .ok_or_else(|| Error::refused("canon.changeNamesNoFact"))?;
            let found =
                fact::get(conn, &fact_id)?.ok_or_else(|| Error::not_found("fact", &fact_id))?;
            if change == FactChange::Retire {
                let reason = text(item, "reason")
                    .ok_or_else(|| Error::refused("canon.retireNeedsReason"))?;
                return Ok(ProposedFact {
                    change,
                    fact_id: Some(found.id),
                    card: Some(found.note_id),
                    reason: Some(reason),
                    contradicts,
                    ..ProposedFact::default()
                });
            }
            Ok(ProposedFact {
                change,
                fact_id: Some(found.id),
                card: Some(found.note_id),
                body,
                layer,
                status,
                source: source_of(item, defaults),
                when,
                data,
                contradicts,
                ..ProposedFact::default()
            })
        }
    }
}

/// Where a proposed fact came from: what the item says, or the work the task
/// read, with the line the item quotes.
fn source_of(item: &Map<String, Value>, defaults: &Defaults) -> Option<Source> {
    let line = text(item, "line");
    if let Some(document) = text(item, "document") {
        return Some(Source {
            kind: SourceKind::Document,
            work_id: None,
            version_id: None,
            line: None,
            label: Some(document),
        });
    }
    if let Some(decision) = text(item, "decision") {
        return Some(Source {
            kind: SourceKind::Decision,
            work_id: None,
            version_id: None,
            line: None,
            label: Some(decision),
        });
    }
    let work_id = defaults.work_id.clone()?;
    Some(Source {
        kind: SourceKind::Work,
        work_id: Some(work_id),
        version_id: defaults.version_id.clone(),
        line,
        label: None,
    })
}

/// The cards of a profile, found by id, name or alias.
pub struct CardIndex {
    cards: Vec<Note>,
}

impl CardIndex {
    pub fn of(conn: &Connection, profile_id: &str) -> Result<Self> {
        Ok(Self {
            cards: crate::note::list(
                conn,
                profile_id,
                &crate::note::NoteFilter {
                    canon: Some(true),
                    ..crate::note::NoteFilter::default()
                },
            )?,
        })
    }

    /// The card `named` means: by id, then by an exact name or alias in any
    /// case. A name two cards share names neither.
    pub fn find(&self, named: &str) -> Option<&Note> {
        if let Some(card) = self.cards.iter().find(|card| card.id == named) {
            return Some(card);
        }
        let folded = named.trim().to_lowercase();
        let mut matching = self.cards.iter().filter(|card| {
            card.title
                .iter()
                .chain(card.aliases.iter())
                .any(|name| name.trim().to_lowercase() == folded)
        });
        let first = matching.next()?;
        matching.next().is_none().then_some(first)
    }
}

/// What a person reads about one proposed fact before applying it: the card
/// and the section it lands in, the fact it changes, the facts it
/// contradicts, and whether the canon already says the same.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct FactReview {
    pub card_title: Option<String>,
    pub section_label: Option<String>,
    /// The fact a refinement or a retirement changes, as it stands.
    pub target: Option<Fact>,
    pub contradicted: Vec<Fact>,
    /// The card already holds a fact with the same words.
    pub duplicate: bool,
}

/// The review of every fact of a proposal, in order, against the canon as it
/// stands now - which may have moved since the proposal was written.
pub fn review(conn: &Connection, package: &Package) -> Result<Vec<FactReview>> {
    let mut out = Vec::with_capacity(package.facts.len());
    for proposed in &package.facts {
        let card_id = proposed.card.as_deref().unwrap_or_default();
        let (card_title, card) = match crate::note::get(conn, card_id)? {
            Some(card) => (card.title.clone(), Some(card)),
            None => (
                package
                    .cards
                    .iter()
                    .find(|c| c.handle == card_id)
                    .map(|c| c.title.clone()),
                None,
            ),
        };
        let target = match proposed.fact_id.as_deref() {
            Some(id) => fact::get(conn, id)?,
            None => None,
        };
        let section_key = proposed
            .section
            .clone()
            .or_else(|| target.as_ref().map(|t| t.section.clone()));
        let section_label = match (&card, &section_key) {
            (Some(card), Some(key)) => {
                let config = crate::profile::config_for(conn, &card.profile_id)?;
                config
                    .card_kind(&card.kind)
                    .and_then(|kind| kind.section(key))
                    .map(|section| section.label.as_str().to_owned())
            }
            _ => section_key.clone(),
        };
        let mut contradicted = Vec::new();
        for one in &proposed.contradicts {
            if let Some(found) = fact::get(conn, &one.fact_id)? {
                contradicted.push(found);
            }
        }
        let duplicate = match (&card, &proposed.body) {
            (Some(card), Some(body)) if proposed.change == FactChange::Add => {
                let words = normalised(body);
                fact::for_card(conn, &card.id)?.iter().any(|held| {
                    held.status != FactStatus::Retired && normalised(&held.body) == words
                })
            }
            _ => false,
        };
        out.push(FactReview {
            card_title,
            section_label,
            target,
            contradicted,
            duplicate,
        });
    }
    Ok(out)
}

fn normalised(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(['.', '!', '?', '…'])
        .to_lowercase()
}

/// What an action that proposes for the canon appends to its prompt: the
/// kinds of card with their sections, the layers and states, the kinds of
/// relation, and the block to answer in.
///
/// Spelled out from the profile, the way a score's axes are: a section the
/// kind does not have is left out of the proposal, and a model handed only
/// `"section": "<key>"` invents its own.
pub fn instruction(config: &ProfileConfig) -> String {
    let mut kinds = String::new();
    for kind in config.note_kinds.iter().filter(|k| k.is_card()) {
        let sections = kind
            .sections
            .iter()
            .filter(|s| s.shape.holds_facts())
            .map(|s| {
                let keys = super::fact::data_keys(s.shape);
                let extra = match s.shape {
                    SectionShape::Palette => " (data: color #RRGGBB)".to_owned(),
                    _ if keys.is_empty() => String::new(),
                    _ => format!(" (data: {})", keys.join(", ")),
                };
                format!("`{}` = {}{extra}", s.key, s.label)
            })
            .collect::<Vec<_>>()
            .join("; ");
        kinds.push_str(&format!("- `{}` ({}): {sections}\n", kind.key, kind.label));
    }
    let relations = if config.relation_kinds.is_empty() {
        String::new()
    } else {
        format!(
            "\nKinds of relation, by key: {}.\n",
            config
                .relation_kinds
                .iter()
                .map(|k| format!("`{}` = {}", k.key, k.label))
                .collect::<Vec<_>>()
                .join("; ")
        )
    };
    format!(
        "\n\nKinds of card and their sections, by key:\n{kinds}{relations}\n\
         Layers: `public` (may be said anywhere), `internal` (the works take the detail, never \
         the name, the date or the address), `inWorks` (only inside the works). What you propose is \
         kept as a draft until the person settles it; give `\"status\": \"open\"` only to a live \
         zone, a question left open on purpose.\n\n\
         End your reply with a fenced json block, exactly this shape and nothing else inside it:\n\n\
         ```json\n{{\n  \"cards\": [\n    {{ \"handle\": \"new-1\", \"kind\": \"<kind key>\", \"title\": \"<name>\", \
         \"aliases\": [\"<other forms of the name>\"], \"layer\": \"public\", \"on_work\": false }}\n  ],\n  \
         \"facts\": [\n    {{ \"change\": \"add\", \"card\": \"<card id, or a handle above>\", \"section\": \"<section key>\", \
         \"text\": \"<the fact>\", \"layer\": \"public\", \"line\": \"<the line it was read off, quoted>\", \
         \"when\": \"<when in the world, in words>\", \"sort\": \"<YYYY, YYYY-MM or YYYY-MM-DD>\", \
         \"contradicts\": [{{ \"fact\": \"<fact id>\", \"why\": \"<one line>\" }}] }},\n    \
         {{ \"change\": \"refine\", \"fact\": \"<fact id>\", \"text\": \"<the sharper wording>\" }},\n    \
         {{ \"change\": \"retire\", \"fact\": \"<fact id>\", \"reason\": \"<why it is no longer true>\" }}\n  ],\n  \
         \"relations\": [\n    {{ \"from\": \"<card>\", \"to\": \"<card>\", \"kind\": \"<relation kind>\", \
         \"label\": \"<what `to` is to `from`>\", \"back_label\": \"<what `from` is to `to`>\", \"layer\": \"public\" }}\n  ]\n}}\n```\n\n\
         Leave out what you have nothing for rather than inventing it; `on_work` is true for a card \
         that lives only in this work. Say whatever you like above the block."
    )
}

fn list<'a>(object: &'a Map<String, Value>, key: &str) -> &'a [Value] {
    object
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn text(object: &Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "null")
        .map(str::to_owned)
}

fn strings(object: &Map<String, Value>, key: &str) -> Vec<String> {
    list(object, key)
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}
