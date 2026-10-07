//! The starter set of the style dictionary, and the slots a dressing is filled
//! through.
//!
//! A shipped profile carries a set of bricks beside its document - image
//! styles, typography, dressings, backgrounds - so a new workspace does not
//! open on an empty dictionary (ADR 0048). The set is seeded into
//! `style_brick` every time the workspace opens, on terms that keep the owner's
//! word:
//!
//! - an entry with no brick yet becomes one, unless the owner deleted it: a
//!   brick's id is derived from the profile and the entry, so its tombstone is
//!   found and it is not brought back;
//! - a brick nobody touched since it was seeded takes a newer wording of its
//!   entry; one the owner changed keeps what they wrote, and reads "changed",
//!   with the set's version one click away ([`restoring`]).
//!
//! "Nobody touched" is a fingerprint, not a flag: the row's content against the
//! fingerprint of the entry last written into it (`Content::digest`).

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::profile::config::{Label, Lens, SectionShape};
use crate::style_brick::{self, Content, StyleBrick, StyleBrickPatch};
use crate::time::now;

/// One brick of a shipped starter set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetBrick {
    /// Unique within the set; with the profile's key it makes the brick's id.
    pub key: String,
    /// A key of the profile's `style_types`.
    #[serde(rename = "type")]
    pub type_key: String,
    /// The name per language. The English word is the brick's `name`.
    pub label: Label,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    /// The words that go into a prompt, in English.
    pub description: String,
    /// When to reach for it, in English: read by whoever picks bricks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colours: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<String>,
}

impl SetBrick {
    /// The English name, which the dictionary is unique on.
    pub fn name(&self) -> &str {
        self.label.as_str()
    }

    pub fn content(&self) -> Content<'_> {
        Content {
            type_key: &self.type_key,
            name: self.name(),
            label: Some(&self.label),
            family: self.family.as_deref(),
            description: Some(&self.description),
            when_to_use: self.when.as_deref(),
            colours: &self.colours,
            sample: self.sample.as_deref(),
        }
    }

    /// The brick's id in every workspace: the same entry of the same profile
    /// is the same brick, so a deletion is recognised as one and two devices
    /// seeding the same set do not make two bricks of it.
    pub fn id(&self, profile_key: &str) -> String {
        const NAMESPACE: uuid::Uuid = uuid::uuid!("6f2b8a3c-1d4e-4c8b-9a57-2e0f6b1c7d90");
        uuid::Uuid::new_v5(
            &NAMESPACE,
            format!("style-set:{profile_key}:{}", self.key).as_bytes(),
        )
        .to_string()
    }
}

/// What a seeding did, for the log and the tests.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Seeded {
    pub added: usize,
    pub rewritten: usize,
    /// Entries left out because a brick of the owner's own already has the
    /// name in that type.
    pub clashes: Vec<String>,
}

/// Seed every shipped profile's set into the workspace's copy of it.
pub fn seed(conn: &Connection) -> Result<()> {
    for profile in crate::profile::builtin()? {
        if profile.style_set.is_empty() {
            continue;
        }
        let seeded = seed_profile(conn, &profile.key, &profile.style_set)?;
        if seeded.added + seeded.rewritten > 0 || !seeded.clashes.is_empty() {
            crate::log::info(
                "style",
                &format!(
                    "the `{}` set: {} added, {} rewritten, {} left for a name the owner took",
                    profile.key,
                    seeded.added,
                    seeded.rewritten,
                    seeded.clashes.len()
                ),
            );
        }
    }
    Ok(())
}

/// Seed `set` into the profile keyed `profile_key`, if the workspace has it.
pub fn seed_profile(conn: &Connection, profile_key: &str, set: &[SetBrick]) -> Result<Seeded> {
    let mut seeded = Seeded::default();
    let Some(profile_id): Option<String> = conn
        .query_row(
            "SELECT id FROM profile WHERE key = ?1",
            params![profile_key],
            |row| row.get(0),
        )
        .optional()?
    else {
        return Ok(seeded);
    };
    let at = now();

    for entry in set {
        let id = entry.id(profile_key);
        let digest = entry.content().digest();
        if let Some(brick) = style_brick::get(conn, &id)? {
            let untouched = brick.set_digest.as_deref() == Some(brick.content().digest().as_str());
            if untouched && brick.set_digest.as_deref() != Some(digest.as_str()) {
                write_entry(conn, &id, entry, &digest, &at)?;
                seeded.rewritten += 1;
            }
            continue;
        }
        let deleted: bool = conn
            .query_row(
                "SELECT 1 FROM tombstone WHERE entity = 'style_brick' AND entity_id = ?1 AND restored_at IS NULL",
                params![id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if deleted {
            continue;
        }
        let inserted = conn.execute(
            "INSERT INTO style_brick (id, profile_id, type_key, name, description, status, created_at, updated_at,
                                      label, family, when_to_use, colours, sample, set_key, set_digest)
             VALUES (?1, ?2, ?3, ?4, ?5, 'ready', ?6, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT DO NOTHING",
            params![
                id,
                profile_id,
                entry.type_key,
                entry.name(),
                entry.description,
                at,
                serde_json::to_string(&entry.label)?,
                entry.family,
                entry.when,
                serde_json::to_string(&entry.colours)?,
                entry.sample,
                entry.key,
                digest,
            ],
        )?;
        if inserted == 1 {
            seeded.added += 1;
        } else {
            seeded.clashes.push(entry.key.clone());
        }
    }
    Ok(seeded)
}

/// A newer wording of an entry, into a brick nobody touched. Not a gesture: no
/// person made it, and the log records what people do.
fn write_entry(
    conn: &Connection,
    id: &str,
    entry: &SetBrick,
    digest: &str,
    at: &str,
) -> Result<()> {
    let result = conn.execute(
        "UPDATE style_brick SET type_key = ?2, name = ?3, description = ?4, label = ?5, family = ?6,
                when_to_use = ?7, colours = ?8, sample = ?9, set_digest = ?10, updated_at = ?11
         WHERE id = ?1",
        params![
            id,
            entry.type_key,
            entry.name(),
            entry.description,
            serde_json::to_string(&entry.label)?,
            entry.family,
            entry.when,
            serde_json::to_string(&entry.colours)?,
            entry.sample,
            digest,
            at,
        ],
    );
    match result {
        Ok(_) => Ok(()),
        // The newer name is one the owner has since given a brick of their
        // own: the old wording stays rather than taking theirs.
        Err(rusqlite::Error::SqliteFailure(failure, _))
            if failure.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

/// The entry of the shipped set a brick came from, if it still ships.
pub fn entry_of(conn: &Connection, brick: &StyleBrick) -> Result<Option<SetBrick>> {
    let Some(set_key) = &brick.set_key else {
        return Ok(None);
    };
    let key: Option<String> = conn
        .query_row(
            "SELECT key FROM profile WHERE id = ?1",
            params![brick.profile_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(key) = key else {
        return Ok(None);
    };
    Ok(crate::profile::builtin()?
        .into_iter()
        .find(|profile| profile.key == key)
        .and_then(|profile| profile.style_set.into_iter().find(|e| e.key == *set_key)))
}

/// The edit that puts a brick back the way the set has it - every field the
/// set speaks for, and the fingerprint, so it reads untouched again. Status
/// and the steer stay: they are about the brick, not what it says.
pub fn restoring(conn: &Connection, brick: &StyleBrick) -> Result<StyleBrickPatch> {
    let entry = entry_of(conn, brick)?
        .ok_or_else(|| Error::refused("style.notFromTheSet").param("name", brick.name.clone()))?;
    Ok(restoring_from(&entry))
}

/// [`restoring`], from the entry itself.
pub fn restoring_from(entry: &SetBrick) -> StyleBrickPatch {
    StyleBrickPatch {
        type_key: Some(entry.type_key.clone()),
        name: Some(entry.name().to_owned()),
        description: Some(Some(entry.description.clone())),
        label: Some(Some(entry.label.clone())),
        family: Some(entry.family.clone()),
        when_to_use: Some(entry.when.clone()),
        colours: Some(entry.colours.clone()),
        sample: Some(entry.sample.clone()),
        set_digest: Some(Some(entry.content().digest())),
        ..StyleBrickPatch::default()
    }
}

/// Values of a dressing's slots, by slot name. A slot may hold several lines
/// - "micro" is a few short phrases - and `{micro.0}` reads one of them.
pub type SlotValues = BTreeMap<String, Vec<String>>;

/// The captions the channel's card gives a cover: every fact of a `slots`
/// section of a root card that a cover may read, under its slot's name.
pub fn channel_slots(conn: &Connection, profile_id: &str) -> Result<SlotValues> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let mut values = SlotValues::new();
    let roots: Vec<_> =
        crate::canon::view::cards(conn, profile_id, &crate::canon::view::CardFilter::default())?
            .into_iter()
            .filter(|card| config.card_kind(&card.kind).is_some_and(|kind| kind.root))
            .collect();
    for root in roots {
        let Some(kind) = config.card_kind(&root.kind) else {
            continue;
        };
        let view = crate::canon::view::card(conn, &root.id)?;
        for read in &view.facts {
            let slotted = kind
                .sections
                .iter()
                .any(|s| s.key == read.fact.section && s.shape == SectionShape::Slots);
            if !slotted || !read.lenses.contains(&Lens::Cover) {
                continue;
            }
            let Some(slot) = read.fact.data.get("slot").and_then(|v| v.as_str()) else {
                continue;
            };
            let body = read.fact.body.trim();
            if !body.is_empty() {
                values
                    .entry(slot.trim().to_owned())
                    .or_default()
                    .push(body.to_owned());
            }
        }
    }
    Ok(values)
}

/// The slot names a text asks for, in the order they first appear.
pub fn slots(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for piece in parse(text) {
        let names: Vec<&Slot> = match &piece {
            Piece::Slot(slot) => vec![slot],
            Piece::Group(inner) => inner
                .iter()
                .filter_map(|p| match p {
                    Piece::Slot(slot) => Some(slot),
                    _ => None,
                })
                .collect(),
            Piece::Text(_) => Vec::new(),
        };
        for slot in names {
            if !out.contains(&slot.name) {
                out.push(slot.name.clone());
            }
        }
    }
    out
}

/// A dressing's description with its slots filled.
///
/// A slot with nothing to say drops out of the prompt with the phrase it
/// stands in: a phrase in `[square brackets]` is dropped whole, and a slot
/// outside any brackets takes its clause with it - up to the nearest `;`,
/// sentence end or line break. "A tiny line "{brand}" in a corner" with no
/// brand is not "a tiny line "" in a corner": that would be an instruction to
/// draw an empty label.
pub fn fill(text: &str, values: &SlotValues) -> String {
    let mut clauses: Vec<Vec<Resolved>> = vec![Vec::new()];
    for piece in parse(text) {
        match piece {
            Piece::Text(text) => {
                // Split the text at clause ends; the end stays with its clause.
                let mut rest = text.as_str();
                while let Some(end) = clause_end(rest) {
                    let (head, tail) = rest.split_at(end);
                    push_text(&mut clauses, head);
                    clauses.push(Vec::new());
                    rest = tail;
                }
                push_text(&mut clauses, rest);
            }
            Piece::Slot(slot) => {
                let value = value_of(&slot, values);
                last(&mut clauses).push(match value {
                    Some(value) => Resolved::Text(value),
                    None => Resolved::Empty,
                });
            }
            Piece::Group(inner) => {
                let mut out = String::new();
                let mut whole = true;
                for piece in inner {
                    match piece {
                        Piece::Text(text) => out.push_str(&text),
                        Piece::Slot(slot) => match value_of(&slot, values) {
                            Some(value) => out.push_str(&value),
                            None => whole = false,
                        },
                        Piece::Group(_) => {}
                    }
                }
                if whole {
                    last(&mut clauses).push(Resolved::Text(out));
                }
            }
        }
    }

    let mut out = String::new();
    for clause in clauses {
        if clause.iter().any(|piece| matches!(piece, Resolved::Empty)) {
            continue;
        }
        for piece in clause {
            if let Resolved::Text(text) = piece {
                out.push_str(&text);
            }
        }
    }
    tidy(&out)
}

#[derive(Debug, Clone, PartialEq)]
struct Slot {
    name: String,
    index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
enum Piece {
    Text(String),
    Slot(Slot),
    /// A `[...]` phrase: kept whole or dropped whole. Groups do not nest.
    Group(Vec<Piece>),
}

enum Resolved {
    Text(String),
    Empty,
}

fn last(clauses: &mut [Vec<Resolved>]) -> &mut Vec<Resolved> {
    clauses.last_mut().expect("there is always a clause")
}

fn push_text(clauses: &mut [Vec<Resolved>], text: &str) {
    if !text.is_empty() {
        last(clauses).push(Resolved::Text(text.to_owned()));
    }
}

/// Where the first clause of `text` ends, just past its end mark: a `;`, a
/// line break, or a `.` `!` `?` followed by white space.
fn clause_end(text: &str) -> Option<usize> {
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        let next = chars.peek().map(|(_, n)| *n);
        match c {
            ';' | '\n' => return Some(at + c.len_utf8()),
            '.' | '!' | '?' if next.is_some_and(char::is_whitespace) => {
                return Some(at + c.len_utf8());
            }
            _ => {}
        }
    }
    None
}

fn value_of(slot: &Slot, values: &SlotValues) -> Option<String> {
    let lines: Vec<&str> = values
        .get(&slot.name)
        .map(|lines| {
            lines
                .iter()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let value = match slot.index {
        Some(index) => lines.get(index).map(|line| (*line).to_owned()),
        None if lines.is_empty() => None,
        None => Some(lines.join(" / ")),
    };
    value.filter(|value| !value.is_empty())
}

fn parse(text: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(inner) = rest.strip_prefix('[')
            && let Some(close) = inner.find(']')
            && !inner[..close].contains('[')
        {
            out.push(Piece::Group(parse_flat(&inner[..close])));
            rest = &inner[close + 1..];
            continue;
        }
        let next = rest[1..].find('[').map_or(rest.len(), |at| at + 1);
        out.extend(parse_flat(&rest[..next]));
        rest = &rest[next..];
    }
    merge_text(out)
}

/// Text and slots, no groups.
fn parse_flat(text: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            break;
        };
        match slot_named(&after[..close]) {
            Some(slot) => {
                if open > 0 {
                    out.push(Piece::Text(rest[..open].to_owned()));
                }
                out.push(Piece::Slot(slot));
                rest = &after[close + 1..];
            }
            None => {
                out.push(Piece::Text(rest[..=open].to_owned()));
                rest = after;
            }
        }
    }
    if !rest.is_empty() {
        out.push(Piece::Text(rest.to_owned()));
    }
    out
}

/// `micro` or `micro.0`: a letter first, then letters, digits, `_` and `-`.
fn slot_named(inside: &str) -> Option<Slot> {
    let (name, index) = match inside.split_once('.') {
        Some((name, index)) => (name, Some(index.parse::<usize>().ok()?)),
        None => (inside, None),
    };
    let mut chars = name.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic()
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    Some(Slot {
        name: name.to_owned(),
        index,
    })
}

fn merge_text(pieces: Vec<Piece>) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::with_capacity(pieces.len());
    for piece in pieces {
        if let (Piece::Text(text), Some(Piece::Text(before))) = (&piece, out.last_mut()) {
            before.push_str(text);
            continue;
        }
        out.push(piece);
    }
    out
}

/// What dropping phrases leaves behind: doubled spaces, a space before a
/// comma or a full stop, a leading comma.
fn tidy(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split('\n') {
        let mut line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        for mark in [",", ".", ";", ":"] {
            line = line.replace(&format!(" {mark}"), mark);
        }
        while line.contains(",,") {
            line = line.replace(",,", ",");
        }
        let line = line.trim_start_matches([',', ';', ' ']).trim_end();
        if !line.is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(line);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::profile::config::StyleForm;

    fn values(pairs: &[(&str, &[&str])]) -> SlotValues {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.iter().map(|s| (*s).to_owned()).collect()))
            .collect()
    }

    #[test]
    fn a_filled_slot_reads_its_value_and_a_list_reads_as_one_line() {
        let v = values(&[("brand", &["NORTHWIND"]), ("micro", &["ONE", "TWO"])]);
        assert_eq!(
            fill(
                "A label \"{brand}\". Lines \"{micro}\", first \"{micro.0}\".",
                &v
            ),
            "A label \"NORTHWIND\". Lines \"ONE / TWO\", first \"ONE\"."
        );
    }

    #[test]
    fn an_empty_slot_in_brackets_drops_its_phrase_and_nothing_else() {
        let text = "Under the title add[ a small subtitle \"{micro.0}\",][ a tiny line \"{brand} · No. {num}\"] and a small barcode.";
        let v = values(&[("micro", &["BY THE SEA"])]);
        assert_eq!(
            fill(text, &v),
            "Under the title add a small subtitle \"BY THE SEA\", and a small barcode."
        );
        let all = values(&[
            ("micro", &["BY THE SEA"]),
            ("brand", &["NW"]),
            ("num", &["7"]),
        ]);
        assert_eq!(
            fill(text, &all),
            "Under the title add a small subtitle \"BY THE SEA\", a tiny line \"NW · No. 7\" and a small barcode."
        );
    }

    #[test]
    fn an_empty_slot_outside_brackets_takes_its_clause_with_it() {
        let text =
            "Keep the frame calm; add a tiny line \"{brand}\" in a corner. Leave the edges empty.";
        assert_eq!(
            fill(text, &SlotValues::new()),
            "Keep the frame calm; Leave the edges empty."
        );
    }

    #[test]
    fn braces_that_are_not_a_slot_stay_as_they_are() {
        let v = SlotValues::new();
        assert_eq!(fill("A set {of 3} marks {}.", &v), "A set {of 3} marks {}.");
        assert_eq!(
            slots("A {brand} and [{micro.1}] and {brand}"),
            vec!["brand", "micro"]
        );
    }

    #[test]
    fn an_index_past_the_list_is_an_empty_slot() {
        let v = values(&[("micro", &["ONE"])]);
        assert_eq!(
            fill("First[ \"{micro.0}\"][ then \"{micro.1}\"].", &v),
            "First \"ONE\"."
        );
    }

    fn a_set() -> Vec<SetBrick> {
        serde_json::from_value(serde_json::json!([
            {"key": "ink", "type": "image-style", "label": {"en": "Ink", "ru": "Тушь"},
             "description": "Black ink.", "when": "Quiet songs."},
            {"key": "paper", "type": "image-style", "label": {"en": "Paper", "ru": "Бумага"},
             "description": "Off-white paper."}
        ]))
        .unwrap()
    }

    #[test]
    fn a_set_is_seeded_once_and_reads_from_the_set() {
        let (conn, profile_id) = fixtures::workspace();
        let set = a_set();

        let first = seed_profile(&conn, "music", &set).unwrap();
        assert_eq!(first.added, 2);
        let again = seed_profile(&conn, "music", &set).unwrap();
        assert_eq!(again, Seeded::default(), "seeding is idempotent");

        let bricks = style_brick::list(&conn, &profile_id, &Default::default()).unwrap();
        assert_eq!(bricks.len(), 2);
        assert!(bricks.iter().all(|b| b.origin == style_brick::Origin::Set));
        let ink = bricks.iter().find(|b| b.name == "Ink").unwrap();
        assert_eq!(ink.status, style_brick::READY);
        assert_eq!(ink.label.as_ref().map(|l| l.in_locale("ru")), Some("Тушь"));
        assert_eq!(ink.when_to_use.as_deref(), Some("Quiet songs."));
    }

    #[test]
    fn a_newer_set_rewrites_an_untouched_brick_and_leaves_a_changed_one() {
        let (conn, profile_id) = fixtures::workspace();
        let mut set = a_set();
        seed_profile(&conn, "music", &set).unwrap();

        let paper_id = set[1].id("music");
        crate::actions::style::update(
            &conn,
            &paper_id,
            StyleBrickPatch {
                description: Some(Some("My paper.".into())),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            style_brick::get(&conn, &paper_id).unwrap().unwrap().origin,
            style_brick::Origin::Changed
        );

        set[0].description = "Black ink, wet.".into();
        set[1].description = "Warm paper.".into();
        let seeded = seed_profile(&conn, "music", &set).unwrap();
        assert_eq!(seeded.rewritten, 1);

        let ink = style_brick::get(&conn, &set[0].id("music"))
            .unwrap()
            .unwrap();
        assert_eq!(ink.description.as_deref(), Some("Black ink, wet."));
        assert_eq!(ink.origin, style_brick::Origin::Set, "still untouched");
        let paper = style_brick::get(&conn, &paper_id).unwrap().unwrap();
        assert_eq!(
            paper.description.as_deref(),
            Some("My paper."),
            "the owner's word stays"
        );
        let _ = profile_id;
    }

    #[test]
    fn an_edit_typed_back_is_no_edit() {
        let (conn, _) = fixtures::workspace();
        let set = a_set();
        seed_profile(&conn, "music", &set).unwrap();
        let id = set[0].id("music");
        for text in ["Changed.", "Black ink."] {
            crate::actions::style::update(
                &conn,
                &id,
                StyleBrickPatch {
                    description: Some(Some(text.into())),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        assert_eq!(
            style_brick::get(&conn, &id).unwrap().unwrap().origin,
            style_brick::Origin::Set
        );
    }

    #[test]
    fn a_deleted_set_brick_does_not_come_back() {
        let (conn, _) = fixtures::workspace();
        let set = a_set();
        seed_profile(&conn, "music", &set).unwrap();
        let id = set[0].id("music");
        crate::trash::discard(&conn, crate::trash::Entity::Style, &id).unwrap();

        let seeded = seed_profile(&conn, "music", &set).unwrap();
        assert_eq!(seeded.added, 0);
        assert!(style_brick::get(&conn, &id).unwrap().is_none());
    }

    #[test]
    fn a_name_the_owner_took_keeps_the_set_brick_out() {
        let (conn, profile_id) = fixtures::workspace();
        style_brick::create(
            &conn,
            &profile_id,
            style_brick::NewStyleBrick {
                type_key: "image-style".into(),
                name: "Ink".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let seeded = seed_profile(&conn, "music", &a_set()).unwrap();
        assert_eq!(seeded.added, 1);
        assert_eq!(seeded.clashes, vec!["ink".to_owned()]);
    }

    #[test]
    fn renaming_a_set_brick_makes_the_name_the_owners() {
        let (conn, _) = fixtures::workspace();
        let set = a_set();
        seed_profile(&conn, "music", &set).unwrap();
        let id = set[0].id("music");
        let renamed = crate::actions::style::update(
            &conn,
            &id,
            StyleBrickPatch {
                name: Some("Моя тушь".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            renamed.label.is_none(),
            "the shipped word per language goes"
        );
        assert_eq!(renamed.origin, style_brick::Origin::Changed);
    }

    #[test]
    fn restoring_puts_a_changed_brick_back_as_the_set_has_it() {
        let (conn, _) = fixtures::workspace();
        let set = a_set();
        seed_profile(&conn, "music", &set).unwrap();
        let id = set[0].id("music");
        crate::actions::style::update(
            &conn,
            &id,
            StyleBrickPatch {
                name: Some("Mine".into()),
                description: Some(Some("Mine.".into())),
                status: Some(style_brick::DROPPED.into()),
                ..Default::default()
            },
        )
        .unwrap();

        crate::actions::style::update(&conn, &id, restoring_from(&set[0])).unwrap();

        let back = style_brick::get(&conn, &id).unwrap().unwrap();
        assert_eq!(back.name, "Ink");
        assert_eq!(back.description.as_deref(), Some("Black ink."));
        assert_eq!(back.origin, style_brick::Origin::Set);
        assert_eq!(
            back.status,
            style_brick::DROPPED,
            "the status is the owner's"
        );
    }

    #[test]
    fn the_shipped_set_is_one_the_shipped_profile_can_hold() {
        for profile in crate::profile::builtin().unwrap() {
            let config = &profile.config;
            let mut keys: Vec<&str> = Vec::new();
            let mut names: Vec<(String, String)> = Vec::new();
            for entry in &profile.style_set {
                let at = format!("`{}` set entry `{}`", profile.key, entry.key);
                assert!(!keys.contains(&entry.key.as_str()), "{at} is listed twice");
                keys.push(&entry.key);

                let style = config
                    .style_type(&entry.type_key)
                    .unwrap_or_else(|| panic!("{at} is of a type the profile does not name"));
                assert!(style.retired.is_none(), "{at} is of a retired type");
                match &entry.family {
                    Some(family) => assert!(
                        style.families.iter().any(|f| f.key == *family),
                        "{at} is filed under `{family}`, which its type does not have"
                    ),
                    None => assert!(style.families.is_empty(), "{at} names no family"),
                }

                let name = (entry.type_key.clone(), entry.name().to_owned());
                assert!(!names.contains(&name), "{at} repeats a name in its type");
                names.push(name);
                for locale in ["en", "ru"] {
                    assert!(
                        matches!(&entry.label, Label::PerLocale(words)
                            if words.get(locale).is_some_and(|w| !w.trim().is_empty())),
                        "{at} has no `{locale}` name"
                    );
                }

                // What goes to a model is English, as every prompt is.
                assert!(entry.when.is_some(), "{at} does not say when to take it");
                for text in [Some(entry.description.as_str()), entry.when.as_deref()]
                    .into_iter()
                    .flatten()
                {
                    assert!(!text.trim().is_empty(), "{at} says nothing");
                    assert!(
                        !text.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)),
                        "{at} is not in English: {text}"
                    );
                }

                // A slot outside brackets would take a whole sentence with it
                // when its caption is empty; the set brackets every one.
                assert!(
                    parse(&entry.description)
                        .iter()
                        .all(|piece| !matches!(piece, Piece::Slot(_))),
                    "{at} has a slot outside [brackets]: {}",
                    entry.description
                );

                for colour in &entry.colours {
                    let hex = colour.strip_prefix('#').unwrap_or("");
                    assert!(
                        hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()),
                        "{at} has a colour `{colour}` that is not #RRGGBB"
                    );
                }
                match style.form {
                    StyleForm::Lettering => {
                        assert!(entry.sample.is_some(), "{at} has no sample to show");
                        assert_eq!(
                            entry.colours.len(),
                            1,
                            "{at} names the ground of its sample"
                        );
                    }
                    StyleForm::Colour => assert_eq!(entry.colours.len(), 1, "{at} is one colour"),
                    StyleForm::Picture => {
                        assert!(!entry.colours.is_empty(), "{at} has no palette to show");
                    }
                    StyleForm::Dressing => assert!(
                        !slots(&entry.description).is_empty(),
                        "{at} is a dressing with no slot"
                    ),
                }
            }
        }
    }

    /// The quoted words after each `marker` in `text`: the family names of
    /// `font-family: 'A'` and the file of `url('./a.woff2')`.
    fn quoted_after<'a>(text: &'a str, marker: &str) -> Vec<&'a str> {
        text.match_indices(marker)
            .filter_map(|(at, _)| {
                let rest = text[at + marker.len()..].trim_start();
                let quote = rest.chars().next().filter(|c| *c == '\'' || *c == '"')?;
                let inner = &rest[1..];
                inner.find(quote).map(|end| &inner[..end])
            })
            .collect()
    }

    /// A lettering sample is drawn with a typeface the window carries. One it
    /// names and the window lacks falls back to the system font, and the card
    /// shows nothing of the style - without a word, which is how it would go
    /// unnoticed. Every file the faces point at ships, and none ships unused.
    #[test]
    fn every_sample_is_drawn_with_a_typeface_the_window_ships() {
        let fonts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/assets/fonts");
        let css = std::fs::read_to_string(fonts.join("samples.css")).unwrap();
        let declared = quoted_after(&css, "font-family:");
        let files = quoted_after(&css, "url(");
        assert!(
            !declared.is_empty() && !files.is_empty(),
            "the faces are read"
        );

        for file in &files {
            assert!(
                fonts.join(file).is_file(),
                "samples.css points at {file}, which is not there"
            );
        }
        for found in std::fs::read_dir(&fonts).unwrap() {
            let name = found.unwrap().file_name().to_string_lossy().into_owned();
            if name.ends_with(".woff2") {
                assert!(
                    files
                        .iter()
                        .any(|file| file.trim_start_matches("./") == name),
                    "{name} ships but no face uses it"
                );
            }
        }

        for profile in crate::profile::builtin().unwrap() {
            for entry in &profile.style_set {
                let Some(sample) = &entry.sample else {
                    continue;
                };
                let families = quoted_after(sample, "font-family:");
                assert!(
                    !families.is_empty(),
                    "`{}` set entry `{}` names no typeface",
                    profile.key,
                    entry.key
                );
                for family in families {
                    assert!(
                        declared.contains(&family),
                        "`{}` set entry `{}` is drawn with '{family}', which the window does not ship",
                        profile.key,
                        entry.key
                    );
                }
            }
        }
    }

    #[test]
    fn the_shipped_set_seeds_whole_into_a_new_workspace() {
        let (conn, profile_id) = fixtures::workspace();
        seed(&conn).unwrap();
        let music = crate::profile::builtin()
            .unwrap()
            .into_iter()
            .find(|p| p.key == "music")
            .unwrap();
        let bricks = style_brick::list(&conn, &profile_id, &Default::default()).unwrap();
        assert_eq!(bricks.len(), music.style_set.len());
        assert!(bricks.iter().all(|b| b.origin == style_brick::Origin::Set));
        assert!(bricks.iter().all(|b| b.status == style_brick::READY));
    }

    fn a_channel_caption(conn: &Connection, profile_id: &str, slot: &str, body: &str) {
        let channel = match crate::note::list(
            conn,
            profile_id,
            &crate::note::NoteFilter {
                canon: Some(true),
                ..Default::default()
            },
        )
        .unwrap()
        .into_iter()
        .find(|n| n.kind == "channel")
        {
            Some(card) => card,
            None => fixtures::card(conn, profile_id, "channel", "The channel"),
        };
        let mut data = serde_json::Map::new();
        data.insert("slot".into(), slot.into());
        crate::canon::fact::create_minted(
            conn,
            crate::canon::NewFact {
                note_id: channel.id,
                section: "captions".into(),
                body: body.into(),
                data,
                ..Default::default()
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap();
    }

    #[test]
    fn the_channel_card_gives_a_dressing_its_captions() {
        let (conn, profile_id) = fixtures::workspace();
        a_channel_caption(&conn, &profile_id, "brand", "NORTHWIND");
        a_channel_caption(&conn, &profile_id, "micro", "ONE ROOM");
        a_channel_caption(&conn, &profile_id, "micro", "NO SHORE");

        let slots = channel_slots(&conn, &profile_id).unwrap();
        assert_eq!(slots.get("brand"), Some(&vec!["NORTHWIND".to_owned()]));
        assert_eq!(
            slots.get("micro"),
            Some(&vec!["ONE ROOM".to_owned(), "NO SHORE".to_owned()])
        );
    }

    #[test]
    fn a_dressing_in_a_prompt_is_filled_and_an_empty_caption_drops_its_phrase() {
        let (conn, profile_id) = fixtures::workspace();
        seed(&conn).unwrap();
        a_channel_caption(&conn, &profile_id, "brand", "NORTHWIND");
        let work = fixtures::video(&conn, &profile_id, "Tide");
        let minimal = style_brick::list(&conn, &profile_id, &Default::default())
            .unwrap()
            .into_iter()
            .find(|b| b.set_key.as_deref() == Some("minimal"))
            .unwrap();

        let prompt = crate::assistant::prompt::for_work(
            &conn,
            &work.id,
            "{styles}",
            crate::assistant::prompt::Context {
                style_brick_ids: std::slice::from_ref(&minimal.id),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(prompt.contains("NORTHWIND"), "{prompt}");
        assert!(
            !prompt.contains('{') && !prompt.contains('['),
            "no slot and no bracket reaches a prompt: {prompt}"
        );
        assert!(
            !prompt.contains("subtitle"),
            "the subtitle had no caption and went: {prompt}"
        );
    }

    #[test]
    fn the_library_lists_when_to_take_each_brick_and_leaves_retired_types_out() {
        let (conn, profile_id) = fixtures::workspace();
        seed(&conn).unwrap();
        let old = style_brick::create(
            &conn,
            &profile_id,
            style_brick::NewStyleBrick {
                type_key: "image-style".into(),
                name: "Old layers".into(),
                description: Some("Text over the figure.".into()),
                ..Default::default()
            },
        )
        .unwrap();
        // Written before the type retired, the way a real one was.
        conn.execute(
            "UPDATE style_brick SET type_key = 'layering' WHERE id = ?1",
            params![old.id],
        )
        .unwrap();
        let work = fixtures::video(&conn, &profile_id, "Tide");

        let library = crate::assistant::prompt::for_work(
            &conn,
            &work.id,
            "{style_library}",
            crate::assistant::prompt::Context::default(),
        )
        .unwrap();
        assert!(library.contains("## Image style"), "{library}");
        assert!(library.contains("Neo-traditional"), "{library}");
        assert!(
            library.contains(", Tattoo: Heroic"),
            "family and when: {library}"
        );
        assert!(
            !library.contains("Old layers"),
            "a retired type builds nothing: {library}"
        );
    }

    #[test]
    fn a_retired_type_reads_but_makes_no_new_brick() {
        let (conn, profile_id) = fixtures::workspace();
        let refused = style_brick::create(
            &conn,
            &profile_id,
            style_brick::NewStyleBrick {
                type_key: "composition".into(),
                name: "Centre".into(),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(refused.refusal().map(|r| r.code), Some("style.retiredType"));

        let own = style_brick::create(
            &conn,
            &profile_id,
            style_brick::NewStyleBrick {
                type_key: "image-style".into(),
                name: "Moved".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let moved = crate::actions::style::update(
            &conn,
            &own.id,
            StyleBrickPatch {
                type_key: Some("layering".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(moved.refusal().map(|r| r.code), Some("style.retiredType"));
    }

    #[test]
    fn a_family_is_one_its_type_files_under_and_goes_when_the_type_changes() {
        let (conn, profile_id) = fixtures::workspace();
        let wrong = style_brick::create(
            &conn,
            &profile_id,
            style_brick::NewStyleBrick {
                type_key: "image-style".into(),
                name: "Filed".into(),
                family: Some("nowhere".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(wrong.refusal().map(|r| r.code), Some("style.unknownFamily"));

        let filed = style_brick::create(
            &conn,
            &profile_id,
            style_brick::NewStyleBrick {
                type_key: "image-style".into(),
                name: "Filed".into(),
                family: Some("tattoo".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let moved = crate::actions::style::update(
            &conn,
            &filed.id,
            StyleBrickPatch {
                type_key: Some("environment".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(moved.family, None, "an environment files nothing");
    }

    #[test]
    fn colours_are_kept_as_upper_case_hex_and_anything_else_is_refused() {
        let (conn, profile_id) = fixtures::workspace();
        let bg = style_brick::create(
            &conn,
            &profile_id,
            style_brick::NewStyleBrick {
                type_key: "background".into(),
                name: "Sea".into(),
                colours: Some(vec!["#1e9e95".into()]),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(bg.colours, vec!["#1E9E95".to_owned()]);
        let refused = crate::actions::style::update(
            &conn,
            &bg.id,
            StyleBrickPatch {
                colours: Some(vec!["teal".into()]),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(refused.refusal().map(|r| r.code), Some("style.badColour"));
    }

    #[test]
    fn restoring_refuses_a_brick_of_the_owners_own() {
        let (conn, profile_id) = fixtures::workspace();
        let own = style_brick::create(
            &conn,
            &profile_id,
            style_brick::NewStyleBrick {
                type_key: "image-style".into(),
                name: "Own".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let refused = restoring(&conn, &own).unwrap_err();
        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("style.notFromTheSet")
        );
    }
}
