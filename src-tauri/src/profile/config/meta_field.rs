//! The overview fields a work keeps, and their types.

use serde::{Deserialize, Serialize};

use super::*;

/// A typed field inside `work.meta`.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct MetaField {
    pub key: String,
    pub label: Label,
    #[serde(rename = "type")]
    pub field_type: MetaFieldType,
    /// The kinds of work that have this field; empty means every kind. A
    /// song's tempo is a fact of the clip cut to it as well, while which
    /// variant of the track an audio release plays - the original, the
    /// instrumental, a slowed one - is a fact of the audio release and of
    /// nothing else. Added in v0.86: a document without it gives every field
    /// to every kind, as it always did.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<String>,
    /// The answers a `choice` field offers, stored by key. Meaningless, and
    /// required to be empty, for the other types.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<Kind>,
    /// What a new work of a kind that has the field starts with: the key of
    /// an option for a choice, the value itself otherwise. Absent means the
    /// field starts empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub default: Option<serde_json::Value>,
    /// Whether the field is a fact of the work's own media, never taken from
    /// the work it is made from: a short cut from a clip is not as long as
    /// the clip (v0.90). Absent is false - a mood, a tempo, an idea flow into
    /// what is made from them once, at creation.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub own: bool,
    /// The first code a work is numbered by, as an example of them all:
    /// `CAT-001` gives a new work of a kind that has the field CAT-001, then
    /// CAT-002, CAT-003 - the next after the greatest code of that shape the
    /// workspace has given, its trash included, padded to the example's
    /// width. Written into the work when it is made, so a code stays what it
    /// was whatever happens to the works around it, and one once given is not
    /// given again (ADR 0059). Only a text field is numbered, and only by an
    /// example that ends in digits. Added in v0.93.1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub numbered_from: Option<String>,
    /// The author's key (v0.146, FR-10 10.1): what the field holds - a
    /// song's anchors, its hooks and devices - goes with every action on the
    /// work, and the model is told to keep it as it is unless the author asks.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub protected: bool,
}

impl MetaField {
    pub fn new(key: &str, label: &str, field_type: MetaFieldType) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            field_type,
            kinds: Vec::new(),
            options: Vec::new(),
            default: None,
            own: false,
            numbered_from: None,
            protected: false,
        }
    }

    /// Whether a work of `kind` has this field.
    pub fn applies_to(&self, kind: &str) -> bool {
        self.kinds.is_empty() || self.kinds.iter().any(|named| named == kind)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum MetaFieldType {
    Text,
    /// Text that runs to paragraphs rather than a line: a premise, a note on
    /// where a piece came from. `Text` in a single-line box turned a paragraph
    /// into something you scroll sideways through.
    Multiline,
    Number,
    Date,
    Boolean,
    /// One of the answers the field lists in `options`, stored by its key: a
    /// short closed list a person picks from rather than types into (v0.86).
    Choice,
    /// Several short texts, each an entry of its own, in the order they
    /// were given: the anchors of an experiment, the guests of an episode
    /// (v0.112). Stored as a list of strings; a text stored before the field
    /// became a list reads as its lines.
    List,
    /// How the feeling of a work moves, section by section (v0.146, FR-10
    /// 10.1): a list of `{section, emotion, intensity}`, intensity from 1 to
    /// 10 - drawn as a table and a curve, read by the assistant in order.
    Arc,
}

impl MetaFieldType {
    /// The entries of a list field's stored value: the strings of a list,
    /// or the lines of a text - what the field held while it was one. Each
    /// trimmed, the empty ones left out.
    pub fn entries(stored: &serde_json::Value) -> Vec<String> {
        let kept = |text: &str| {
            let text = text.trim();
            (!text.is_empty()).then(|| text.to_owned())
        };
        match stored {
            serde_json::Value::Array(items) => items
                .iter()
                .filter_map(|item| item.as_str().and_then(kept))
                .collect(),
            serde_json::Value::String(text) => text.lines().filter_map(kept).collect(),
            _ => Vec::new(),
        }
    }
}
