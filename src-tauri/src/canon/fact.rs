//! A fact of a card: one short statement, filed under a section, with the
//! layer it may be told in, how settled it is, where it came from and when it
//! happened in the world.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{FactStatus, Layer};
use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::note::Note;
use crate::profile::config::{NoteKind, ProfileConfig, SectionShape};

/// Where a fact came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    /// A work - a song and the line it was read off.
    Work,
    /// A decision the author made.
    Decision,
    /// A document: a bible, a card, a brief.
    Document,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::Work => "work",
            SourceKind::Decision => "decision",
            SourceKind::Document => "document",
        }
    }

    fn parse(raw: &str) -> Result<Self> {
        match raw {
            "work" => Ok(SourceKind::Work),
            "decision" => Ok(SourceKind::Decision),
            "document" => Ok(SourceKind::Document),
            other => Err(Error::Internal(format!("a stored source reads `{other}`"))),
        }
    }
}

/// Where a fact came from, as it was said.
///
/// A work is named by id and kept softly: it may go to the trash and come
/// back, and the fact should still say where it was read. `label` is what is
/// shown when the work is not there, and all there is for a decision or a
/// document ("Bible 2.2", "decision of 24.09").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct Source {
    pub kind: SourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_id: Option<String>,
    /// The version of the work the line was read in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    /// The line itself, quoted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// When a fact happened inside the world: the words and the place on the
/// line (see [`super::when`]).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct When {
    /// As it is told: "winter 2022/23".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Where it sorts: a year, a month or a day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
}

/// A fact as it is stored and shown.
///
/// The names of the fields are the names a [`FactPatch`] uses, which is what
/// lets the log record the inverse of a patch off the row (ADR 0014).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Fact {
    pub id: String,
    pub profile_id: String,
    /// The card it is a fact of.
    pub note_id: String,
    /// A key of the card kind's `sections`.
    pub section: String,
    pub body: String,
    pub layer: Layer,
    pub status: FactStatus,
    /// Why it was retired. Present exactly when it is.
    pub retired_reason: Option<String>,
    pub source: Option<Source>,
    pub when: Option<When>,
    /// The one work it holds for, when it holds for one: an outfit for a
    /// single release.
    pub scope_work_id: Option<String>,
    /// What a section of another shape carries beside the words: a colour, a
    /// slot, a template, a code, a brick.
    pub data: Map<String, Value>,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// A fact to write.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct NewFact {
    pub note_id: String,
    pub section: String,
    pub body: String,
    #[serde(default)]
    pub layer: Option<Layer>,
    #[serde(default)]
    pub status: Option<FactStatus>,
    #[serde(default)]
    pub retired_reason: Option<String>,
    #[serde(default)]
    pub source: Option<Source>,
    #[serde(default)]
    pub when: Option<When>,
    #[serde(default)]
    pub scope_work_id: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub data: Map<String, Value>,
}

/// A change to a fact. A field left out is kept; a clearable field sent as
/// `null` is cleared.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct FactPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<Layer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<FactStatus>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub retired_reason: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub source: Option<Option<Source>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub when: Option<Option<When>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub scope_work_id: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Map<String, Value>>,
    /// Where it stands among the facts of its section. Written by a move and
    /// by an undo that puts one back; the window reorders a whole section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
}

/// Where the places of a detail may be: the pictures a signature detail
/// belongs in. The readers are the cover and frame constructors (v0.88).
pub const DETAIL_PLACES: [&str; 3] = ["cover", "frame", "scene"];

/// The roles a picture of a card can have.
pub const PICTURE_ROLES: [&str; 6] = ["portrait", "reference", "outfit", "mood", "still", "mark"];

const SELECT: &str = "SELECT id, profile_id, note_id, section, body, layer, status, retired_reason, \
     source_kind, source_work_id, source_version_id, source_line, source_label, when_label, \
     when_sort, scope_work_id, data, position, created_at, updated_at FROM canon_fact";

/// The card a fact is written onto, with the kind that says what it may hold.
///
/// A note of a plain kind is refused: its body is its knowledge, and a fact
/// filed under a section it does not have would be a row no screen shows.
pub fn card_of<'c>(
    conn: &Connection,
    config: &'c ProfileConfig,
    note_id: &str,
) -> Result<(Note, &'c NoteKind)> {
    let note = crate::note::get(conn, note_id)?.ok_or_else(|| Error::not_found("note", note_id))?;
    let kind = config.card_kind(&note.kind).ok_or_else(|| {
        Error::refused("canon.notACard").param("title", note.title.clone().unwrap_or_default())
    })?;
    Ok((note, kind))
}

/// Write a fact at the end of its section.
pub fn create_minted(conn: &Connection, new: NewFact, minted: Minted) -> Result<Fact> {
    let profile_id = note_profile(conn, &new.note_id)?;
    let config = crate::profile::config_for(conn, &profile_id)?;
    let (_, kind) = card_of(conn, &config, &new.note_id)?;
    let shape = section_shape(kind, &new.section)?;

    let body = clean_body(&new.body)?;
    let status = new.status.unwrap_or_default();
    let reason = clean(new.retired_reason);
    check_retirement(status, reason.as_deref())?;
    let source = checked_source(conn, new.source, true)?;
    let when = checked_when(new.when)?;
    if let Some(id) = new.scope_work_id.as_deref() {
        require_work(conn, id)?;
    }
    let data = checked_data(conn, shape, new.data)?;

    let position: i64 = conn.query_row(
        "SELECT coalesce(max(position), 0) + 1 FROM canon_fact WHERE note_id = ?1 AND section = ?2",
        params![new.note_id, new.section],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT INTO canon_fact (id, profile_id, note_id, section, body, layer, status, retired_reason,
             source_kind, source_work_id, source_version_id, source_line, source_label,
             when_label, when_sort, scope_work_id, data, position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?19)",
        params![
            minted.id(),
            profile_id,
            new.note_id,
            new.section,
            body,
            new.layer.unwrap_or_default().as_str(),
            status.as_str(),
            reason,
            source.as_ref().map(|s| s.kind.as_str()),
            source.as_ref().and_then(|s| s.work_id.clone()),
            source.as_ref().and_then(|s| s.version_id.clone()),
            source.as_ref().and_then(|s| s.line.clone()),
            source.as_ref().and_then(|s| s.label.clone()),
            when.as_ref().and_then(|w| w.label.clone()),
            when.as_ref().and_then(|w| w.sort.clone()),
            new.scope_work_id,
            Value::Object(data).to_string(),
            position,
            minted.at(),
        ],
    )?;

    get(conn, minted.id())?.ok_or_else(|| Error::Internal("the fact vanished after insert".into()))
}

/// Change a fact, stamped with the moment of the gesture.
///
/// A fact moved to another section goes to the end of it; one whose status
/// leaves `retired` loses its reason, and one retired must say why.
pub fn update_at(conn: &Connection, id: &str, patch: FactPatch, at: &str) -> Result<Fact> {
    let found = get(conn, id)?.ok_or_else(|| Error::not_found("fact", id))?;
    let config = crate::profile::config_for(conn, &found.profile_id)?;
    let (_, kind) = card_of(conn, &config, &found.note_id)?;

    let section = patch
        .section
        .clone()
        .unwrap_or_else(|| found.section.clone());
    let moved = section != found.section;
    // A fact filed under a section the profile has since dropped can still be
    // corrected where it stands; only a move has to land somewhere real.
    let shape = if moved || patch.data.is_some() {
        Some(section_shape(kind, &section)?)
    } else {
        kind.section(&section).map(|s| s.shape)
    };

    let body = match &patch.body {
        Some(body) => clean_body(body)?,
        None => found.body.clone(),
    };
    let status = patch.status.unwrap_or(found.status);
    let reason = match &patch.retired_reason {
        Some(reason) => clean(reason.clone()),
        // Leaving `retired` takes the reason with it: it explained a
        // retirement that is no longer so.
        None if status != FactStatus::Retired => None,
        None => found.retired_reason.clone(),
    };
    check_retirement(status, reason.as_deref())?;
    let source = match patch.source.clone() {
        Some(source) => checked_source(conn, source, false)?,
        None => found.source.clone(),
    };
    let when = match patch.when.clone() {
        Some(when) => checked_when(when)?,
        None => found.when.clone(),
    };
    let scope = match patch.scope_work_id.clone() {
        Some(scope) => scope,
        None => found.scope_work_id.clone(),
    };
    let data = match patch.data.clone() {
        Some(data) => match shape {
            Some(shape) => checked_data(conn, shape, data)?,
            None => data,
        },
        None => found.data.clone(),
    };
    let position = match (patch.position, moved) {
        (Some(position), _) => position,
        (None, true) => conn.query_row(
            "SELECT coalesce(max(position), 0) + 1 FROM canon_fact WHERE note_id = ?1 AND section = ?2",
            params![found.note_id, section],
            |row| row.get(0),
        )?,
        (None, false) => found.position,
    };

    let unchanged = section == found.section
        && body == found.body
        && patch.layer.is_none_or(|layer| layer == found.layer)
        && status == found.status
        && reason == found.retired_reason
        && source == found.source
        && when == found.when
        && scope == found.scope_work_id
        && data == found.data
        && position == found.position;
    if unchanged {
        return Ok(found);
    }

    conn.execute(
        "UPDATE canon_fact SET section = ?2, body = ?3, layer = ?4, status = ?5, retired_reason = ?6,
             source_kind = ?7, source_work_id = ?8, source_version_id = ?9, source_line = ?10,
             source_label = ?11, when_label = ?12, when_sort = ?13, scope_work_id = ?14, data = ?15,
             position = ?16, updated_at = ?17
         WHERE id = ?1",
        params![
            id,
            section,
            body,
            patch.layer.unwrap_or(found.layer).as_str(),
            status.as_str(),
            reason,
            source.as_ref().map(|s| s.kind.as_str()),
            source.as_ref().and_then(|s| s.work_id.clone()),
            source.as_ref().and_then(|s| s.version_id.clone()),
            source.as_ref().and_then(|s| s.line.clone()),
            source.as_ref().and_then(|s| s.label.clone()),
            when.as_ref().and_then(|w| w.label.clone()),
            when.as_ref().and_then(|w| w.sort.clone()),
            scope,
            Value::Object(data).to_string(),
            position,
            at,
        ],
    )?;

    get(conn, id)?.ok_or_else(|| Error::not_found("fact", id))
}

/// Put the facts of one section in the order given.
///
/// The whole section, every fact of it once: an order that leaves one out
/// says nothing about where it goes, and guessing would move a fact nobody
/// touched.
pub fn reorder(
    conn: &Connection,
    note_id: &str,
    section: &str,
    ids: &[String],
    at: &str,
) -> Result<()> {
    let standing = in_section(conn, note_id, section)?;
    let mut given: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut known: Vec<&str> = standing.iter().map(|f| f.id.as_str()).collect();
    given.sort_unstable();
    known.sort_unstable();
    if given != known {
        return Err(Error::refused("canon.reorderNotTheSection"));
    }
    for (index, id) in ids.iter().enumerate() {
        let position = i64::try_from(index).unwrap_or(i64::MAX) + 1;
        conn.execute(
            "UPDATE canon_fact SET position = ?2, updated_at = ?3 WHERE id = ?1 AND position IS NOT ?2",
            params![id, position, at],
        )?;
    }
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Fact>> {
    let raw = conn
        .query_row(&format!("{SELECT} WHERE id = ?1"), params![id], read)
        .optional()?;
    raw.map(RawFact::into_fact).transpose()
}

/// Every fact of a card, section by section, in the order the person set.
pub fn for_card(conn: &Connection, note_id: &str) -> Result<Vec<Fact>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE note_id = ?1 ORDER BY section, position, rowid"
    ))?;
    let raw = statement
        .query_map(params![note_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    raw.into_iter().map(RawFact::into_fact).collect()
}

/// The facts of one section of a card, in order.
pub fn in_section(conn: &Connection, note_id: &str, section: &str) -> Result<Vec<Fact>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE note_id = ?1 AND section = ?2 ORDER BY position, rowid"
    ))?;
    let raw = statement
        .query_map(params![note_id, section], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    raw.into_iter().map(RawFact::into_fact).collect()
}

/// Every fact of a profile, card by card - what the export and a whole-canon
/// prompt read.
pub fn for_profile(conn: &Connection, profile_id: &str) -> Result<Vec<Fact>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE profile_id = ?1 ORDER BY note_id, section, position, rowid"
    ))?;
    let raw = statement
        .query_map(params![profile_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    raw.into_iter().map(RawFact::into_fact).collect()
}

/// The facts dated inside the world, earliest first; those told in words the
/// line cannot place come last, in the order they were written.
pub fn dated(conn: &Connection, profile_id: &str, note_id: Option<&str>) -> Result<Vec<Fact>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE profile_id = ?1 AND (?2 IS NULL OR note_id = ?2)
           AND status <> 'retired' AND (when_sort IS NOT NULL OR when_label IS NOT NULL)
         ORDER BY when_sort IS NULL, when_sort, position, rowid"
    ))?;
    let raw = statement
        .query_map(params![profile_id, note_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    raw.into_iter().map(RawFact::into_fact).collect()
}

/// The profile a card belongs to.
fn note_profile(conn: &Connection, note_id: &str) -> Result<String> {
    conn.query_row(
        "SELECT profile_id FROM note WHERE id = ?1",
        params![note_id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| Error::not_found("note", note_id))
}

/// The shape of a section a fact may be written into.
fn section_shape(kind: &NoteKind, section: &str) -> Result<SectionShape> {
    let found = kind.section(section).ok_or_else(|| {
        Error::refused("canon.unknownSection")
            .param("section", section)
            .param(
                "kind",
                serde_json::to_value(&kind.label).unwrap_or_default(),
            )
    })?;
    if !found.shape.holds_facts() {
        return Err(Error::refused("canon.sectionHoldsNoFacts").param(
            "section",
            serde_json::to_value(&found.label).unwrap_or_default(),
        ));
    }
    Ok(found.shape)
}

fn clean(text: Option<String>) -> Option<String> {
    text.map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn clean_body(body: &str) -> Result<String> {
    let body = body.trim();
    if body.is_empty() {
        return Err(Error::refused("canon.factNeedsWords"));
    }
    Ok(body.to_owned())
}

/// A retirement says why; nothing else carries a reason.
fn check_retirement(status: FactStatus, reason: Option<&str>) -> Result<()> {
    match (status, reason) {
        (FactStatus::Retired, None) => Err(Error::refused("canon.retireNeedsReason")),
        (FactStatus::Retired, Some(_)) | (_, None) => Ok(()),
        (_, Some(_)) => Err(Error::refused("canon.reasonWithoutRetirement")),
    }
}

/// A source as it is kept: its words trimmed, a work that exists when it is
/// first named (`fresh`), a label where there is no work to name it by.
fn checked_source(
    conn: &Connection,
    source: Option<Source>,
    fresh: bool,
) -> Result<Option<Source>> {
    let Some(mut source) = source else {
        return Ok(None);
    };
    source.work_id = clean(source.work_id);
    source.version_id = clean(source.version_id);
    source.line = clean(source.line);
    source.label = clean(source.label);
    match source.kind {
        SourceKind::Work => {
            let work_id = source
                .work_id
                .clone()
                .ok_or_else(|| Error::refused("canon.sourceNeedsWork"))?;
            // Named by id, kept softly: checked when it is first said, not on
            // every edit after it may have gone to the trash.
            if fresh || source.label.is_none() {
                let work = require_work(conn, &work_id)?;
                if source.label.is_none() {
                    source.label = Some(work.title);
                }
            }
            if let Some(version_id) = source.version_id.as_deref()
                && fresh
            {
                let version = crate::work::version::get(conn, version_id)?
                    .ok_or_else(|| Error::not_found("version", version_id))?;
                if version.work_id != work_id {
                    return Err(Error::refused("canon.sourceVersionOfAnotherWork"));
                }
            }
        }
        SourceKind::Decision | SourceKind::Document => {
            if source.label.is_none() {
                return Err(Error::refused("canon.sourceNeedsLabel"));
            }
            source.work_id = None;
            source.version_id = None;
            source.line = None;
        }
    }
    Ok(Some(source))
}

/// A time as it is kept: the sort key read off the words when none was given,
/// and refused when one was given and is not a date.
fn checked_when(when: Option<When>) -> Result<Option<When>> {
    let Some(when) = when else {
        return Ok(None);
    };
    let label = clean(when.label);
    let sort = match clean(when.sort) {
        Some(sort) if super::when::is_sort_key(&sort) => Some(sort),
        Some(sort) => {
            return Err(Error::refused("canon.badWorldDate").param("value", sort));
        }
        None => label.as_deref().and_then(super::when::sort_key_of),
    };
    if label.is_none() && sort.is_none() {
        return Ok(None);
    }
    Ok(Some(When { label, sort }))
}

fn require_work(conn: &Connection, id: &str) -> Result<crate::work::Work> {
    crate::work::get(conn, id)?.ok_or_else(|| Error::not_found("work", id))
}

/// The keys a fact of a section of `shape` may carry in its data, beside its
/// words. The one list: what is checked on write and what the assistant is
/// told to write.
pub fn data_keys(shape: SectionShape) -> &'static [&'static str] {
    match shape {
        SectionShape::Facts | SectionShape::Relations | SectionShape::Appearances => &[],
        SectionShape::Slots => &["slot"],
        SectionShape::Details => &["template", "places", "on"],
        SectionShape::Palette => &["color"],
        SectionShape::Marks => &["code", "prompt"],
        SectionShape::Styles => &["styleId"],
    }
}

/// What a section of `shape` carries beside the words, checked.
///
/// A key the shape does not read is refused rather than kept: a value under
/// it would sit in the row where no screen shows it - a second schema hidden
/// inside the first (ADR 0013).
pub fn checked_data(
    conn: &Connection,
    shape: SectionShape,
    data: Map<String, Value>,
) -> Result<Map<String, Value>> {
    let allowed = data_keys(shape);
    if let Some(stray) = data.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(Error::refused("canon.dataUnknownKey").param("key", stray.clone()));
    }
    let text = |key: &str| {
        data.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    let mut out = Map::new();
    match shape {
        SectionShape::Slots => {
            let slot = text("slot").ok_or_else(|| Error::refused("canon.slotNeedsName"))?;
            out.insert("slot".into(), Value::String(slot));
        }
        SectionShape::Details => {
            let template =
                text("template").ok_or_else(|| Error::refused("canon.detailNeedsTemplate"))?;
            out.insert("template".into(), Value::String(template));
            let places: Vec<String> = match data.get("places") {
                None | Some(Value::Null) => DETAIL_PLACES.iter().map(|p| (*p).to_owned()).collect(),
                Some(Value::Array(values)) => {
                    let mut places = Vec::new();
                    for value in values {
                        let place = value.as_str().unwrap_or_default().trim();
                        if !DETAIL_PLACES.contains(&place) {
                            return Err(Error::refused("canon.unknownPlace").param("place", place));
                        }
                        if !places.iter().any(|p: &String| p == place) {
                            places.push(place.to_owned());
                        }
                    }
                    places
                }
                Some(_) => return Err(Error::refused("canon.unknownPlace").param("place", "")),
            };
            out.insert(
                "places".into(),
                Value::Array(places.into_iter().map(Value::String).collect()),
            );
            let on = data.get("on").and_then(Value::as_bool).unwrap_or(true);
            out.insert("on".into(), Value::Bool(on));
        }
        SectionShape::Palette => {
            let color = text("color").ok_or_else(|| Error::refused("canon.colorNeedsHex"))?;
            if !is_hex_colour(&color) {
                return Err(Error::refused("canon.colorNeedsHex").param("value", color));
            }
            out.insert("color".into(), Value::String(color.to_uppercase()));
        }
        SectionShape::Marks => {
            for key in ["code", "prompt"] {
                if let Some(value) = text(key) {
                    out.insert(key.into(), Value::String(value));
                }
            }
        }
        SectionShape::Styles => {
            let style_id =
                text("styleId").ok_or_else(|| Error::refused("canon.styleNeedsBrick"))?;
            if crate::style_brick::get(conn, &style_id)?.is_none() {
                return Err(Error::not_found("style", style_id));
            }
            out.insert("styleId".into(), Value::String(style_id));
        }
        SectionShape::Facts | SectionShape::Relations | SectionShape::Appearances => {}
    }
    Ok(out)
}

/// `#RRGGBB`.
fn is_hex_colour(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

struct RawFact {
    id: String,
    profile_id: String,
    note_id: String,
    section: String,
    body: String,
    layer: String,
    status: String,
    retired_reason: Option<String>,
    source_kind: Option<String>,
    source_work_id: Option<String>,
    source_version_id: Option<String>,
    source_line: Option<String>,
    source_label: Option<String>,
    when_label: Option<String>,
    when_sort: Option<String>,
    scope_work_id: Option<String>,
    data: String,
    position: i64,
    created_at: String,
    updated_at: String,
}

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawFact> {
    Ok(RawFact {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        note_id: row.get(2)?,
        section: row.get(3)?,
        body: row.get(4)?,
        layer: row.get(5)?,
        status: row.get(6)?,
        retired_reason: row.get(7)?,
        source_kind: row.get(8)?,
        source_work_id: row.get(9)?,
        source_version_id: row.get(10)?,
        source_line: row.get(11)?,
        source_label: row.get(12)?,
        when_label: row.get(13)?,
        when_sort: row.get(14)?,
        scope_work_id: row.get(15)?,
        data: row.get(16)?,
        position: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

impl RawFact {
    fn into_fact(self) -> Result<Fact> {
        let source = match self.source_kind.as_deref() {
            Some(kind) => Some(Source {
                kind: SourceKind::parse(kind)?,
                work_id: self.source_work_id,
                version_id: self.source_version_id,
                line: self.source_line,
                label: self.source_label,
            }),
            None => None,
        };
        let when = match (&self.when_label, &self.when_sort) {
            (None, None) => None,
            _ => Some(When {
                label: self.when_label,
                sort: self.when_sort,
            }),
        };
        Ok(Fact {
            layer: Layer::parse(&self.layer)?,
            status: FactStatus::parse(&self.status)?,
            data: serde_json::from_str(&self.data)?,
            source,
            when,
            id: self.id,
            profile_id: self.profile_id,
            note_id: self.note_id,
            section: self.section,
            body: self.body,
            retired_reason: self.retired_reason,
            scope_work_id: self.scope_work_id,
            position: self.position,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
