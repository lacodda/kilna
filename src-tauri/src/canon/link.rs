//! A relation between two cards of the canon: a graph, not a paragraph.
//!
//! One row per pair, whichever way it was drawn - "Otto is Wren's neighbour"
//! and "Wren is Otto's neighbour" are one relation. The words differ by side,
//! so the row carries both: `label` is what the second card is to the first,
//! `back_label` what the first is to the second.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use super::Layer;
use crate::error::{Error, Result};
use crate::minted::Minted;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct CanonLink {
    pub id: String,
    pub profile_id: String,
    pub from_id: String,
    pub to_id: String,
    /// A key of the profile's `relation_kinds`.
    pub kind: Option<String>,
    /// What `to` is to `from`: "neighbour, first listener".
    pub label: Option<String>,
    /// What `from` is to `to`.
    pub back_label: Option<String>,
    pub layer: Layer,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct NewCanonLink {
    pub from_id: String,
    pub to_id: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub back_label: Option<String>,
    #[serde(default)]
    pub layer: Option<Layer>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct CanonLinkPatch {
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub kind: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub label: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub back_label: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<Layer>,
}

const SELECT: &str = "SELECT id, profile_id, from_id, to_id, kind, label, back_label, layer, \
     created_at, updated_at FROM canon_link";

/// Draw a relation between two cards.
pub fn create_minted(conn: &Connection, new: NewCanonLink, minted: Minted) -> Result<CanonLink> {
    if new.from_id == new.to_id {
        return Err(Error::refused("canon.relationToItself"));
    }
    let from = card(conn, &new.from_id)?;
    let to = card(conn, &new.to_id)?;
    if from.profile_id != to.profile_id {
        return Err(Error::refused("canon.relationAcrossWorkspaces"));
    }
    let kind = clean(new.kind);
    if let Some(kind) = kind.as_deref() {
        check_kind(conn, &from.profile_id, kind)?;
    }
    if between(conn, &new.from_id, &new.to_id)?.is_some() {
        return Err(Error::refused("canon.alreadyRelated")
            .param("from", from.title.unwrap_or_default())
            .param("to", to.title.unwrap_or_default()));
    }

    conn.execute(
        "INSERT INTO canon_link (id, profile_id, from_id, to_id, kind, label, back_label, layer, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![
            minted.id(),
            from.profile_id,
            new.from_id,
            new.to_id,
            kind,
            clean(new.label),
            clean(new.back_label),
            new.layer.unwrap_or_default().as_str(),
            minted.at(),
        ],
    )?;
    get(conn, minted.id())?
        .ok_or_else(|| Error::Internal("the relation vanished after insert".into()))
}

pub fn update_at(
    conn: &Connection,
    id: &str,
    patch: CanonLinkPatch,
    at: &str,
) -> Result<CanonLink> {
    let found = get(conn, id)?.ok_or_else(|| Error::not_found("relation", id))?;
    let kind = match patch.kind {
        Some(kind) => clean(kind),
        None => found.kind.clone(),
    };
    if let Some(kind) = kind.as_deref() {
        if found.kind.as_deref() != Some(kind) {
            check_kind(conn, &found.profile_id, kind)?;
        }
    }
    let label = match patch.label {
        Some(label) => clean(label),
        None => found.label.clone(),
    };
    let back_label = match patch.back_label {
        Some(label) => clean(label),
        None => found.back_label.clone(),
    };
    let layer = patch.layer.unwrap_or(found.layer);
    if kind == found.kind
        && label == found.label
        && back_label == found.back_label
        && layer == found.layer
    {
        return Ok(found);
    }
    conn.execute(
        "UPDATE canon_link SET kind = ?2, label = ?3, back_label = ?4, layer = ?5, updated_at = ?6 WHERE id = ?1",
        params![id, kind, label, back_label, layer.as_str(), at],
    )?;
    get(conn, id)?.ok_or_else(|| Error::not_found("relation", id))
}

/// Remove a relation. It has no body of its own to keep in the trash: the
/// undo puts the same row back from the copy the operation recorded, the way
/// a link between works comes back (ADR 0019).
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    if conn.execute("DELETE FROM canon_link WHERE id = ?1", params![id])? == 0 {
        return Err(Error::not_found("relation", id));
    }
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<CanonLink>> {
    let raw = conn
        .query_row(&format!("{SELECT} WHERE id = ?1"), params![id], read)
        .optional()?;
    raw.map(RawLink::into_link).transpose()
}

/// The relation between two cards, drawn either way.
pub fn between(conn: &Connection, one: &str, other: &str) -> Result<Option<CanonLink>> {
    let raw = conn
        .query_row(
            &format!(
                "{SELECT} WHERE (from_id = ?1 AND to_id = ?2) OR (from_id = ?2 AND to_id = ?1)"
            ),
            params![one, other],
            read,
        )
        .optional()?;
    raw.map(RawLink::into_link).transpose()
}

/// Every relation a card stands in, from either end, oldest first.
pub fn for_card(conn: &Connection, note_id: &str) -> Result<Vec<CanonLink>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE from_id = ?1 OR to_id = ?1 ORDER BY created_at, rowid"
    ))?;
    let raw = statement
        .query_map(params![note_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    raw.into_iter().map(RawLink::into_link).collect()
}

/// Every relation of a profile - the graph.
pub fn for_profile(conn: &Connection, profile_id: &str) -> Result<Vec<CanonLink>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE profile_id = ?1 ORDER BY created_at, rowid"
    ))?;
    let raw = statement
        .query_map(params![profile_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    raw.into_iter().map(RawLink::into_link).collect()
}

impl CanonLink {
    /// The other card, seen from `note_id`.
    pub fn other(&self, note_id: &str) -> &str {
        if self.from_id == note_id {
            &self.to_id
        } else {
            &self.from_id
        }
    }

    /// What the other card is to `note_id`, in the words of this side.
    pub fn label_from(&self, note_id: &str) -> Option<&str> {
        if self.from_id == note_id {
            self.label.as_deref()
        } else {
            self.back_label.as_deref()
        }
    }
}

/// A card a relation may be drawn to.
fn card(conn: &Connection, note_id: &str) -> Result<crate::note::Note> {
    let note = crate::note::get(conn, note_id)?.ok_or_else(|| Error::not_found("note", note_id))?;
    let config = crate::profile::config_for(conn, &note.profile_id)?;
    if config.card_kind(&note.kind).is_none() {
        return Err(
            Error::refused("canon.notACard").param("title", note.title.clone().unwrap_or_default())
        );
    }
    Ok(note)
}

/// A kind of relation the profile names. A profile that names none has not
/// decided, and any word goes - the leniency a kind of note has.
fn check_kind(conn: &Connection, profile_id: &str, kind: &str) -> Result<()> {
    let config = crate::profile::config_for(conn, profile_id)?;
    if config.relation_kinds.is_empty() || config.relation_kinds.iter().any(|k| k.key == kind) {
        return Ok(());
    }
    Err(Error::refused("canon.unknownRelationKind")
        .param("kind", kind)
        .param(
            "known",
            config
                .relation_kinds
                .iter()
                .map(|k| k.key.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        ))
}

fn clean(text: Option<String>) -> Option<String> {
    text.map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

struct RawLink {
    id: String,
    profile_id: String,
    from_id: String,
    to_id: String,
    kind: Option<String>,
    label: Option<String>,
    back_label: Option<String>,
    layer: String,
    created_at: String,
    updated_at: String,
}

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawLink> {
    Ok(RawLink {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        from_id: row.get(2)?,
        to_id: row.get(3)?,
        kind: row.get(4)?,
        label: row.get(5)?,
        back_label: row.get(6)?,
        layer: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

impl RawLink {
    fn into_link(self) -> Result<CanonLink> {
        Ok(CanonLink {
            layer: Layer::parse(&self.layer)?,
            id: self.id,
            profile_id: self.profile_id,
            from_id: self.from_id,
            to_id: self.to_id,
            kind: self.kind,
            label: self.label,
            back_label: self.back_label,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
