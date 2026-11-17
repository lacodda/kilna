//! Marks a work carries this week, and their colours.

use serde::{Deserialize, Serialize};

use super::*;

/// A flag the author raises on a work by hand.
///
/// Not a status: a status says where the work stands in the process and is
/// worked out from what happened, while a mark says something the data cannot
/// know — that this one is being fought with, or that it is the good one. It
/// derives nothing and blocks nothing.
///
/// Not a tag either, although both are lists of strings the user sets: a tag is
/// the author's vocabulary for what a work *is* and stays with it, a mark is
/// about this week and comes off. Keeping them apart means clearing the flags
/// does not clear the vocabulary, and "on fire" does not offer itself while
/// someone is typing "winter".
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Mark {
    pub key: String,
    pub label: Label,
    /// One of the palette's own roles, so a mark reads correctly in both
    /// themes. A free-form colour would be a colour nobody guaranteed contrast
    /// for.
    #[serde(default)]
    pub colour: MarkColour,
    /// A glyph beside the word, from the short list the screen knows —
    /// `wrench`, `circle-help`, `thumbs-up`… (see the profile reference). A
    /// name the screen does not know draws the default glyph rather than
    /// nothing, and the word is always there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum MarkColour {
    /// The neutral one: a flag that carries no urgency of its own.
    #[default]
    Plain,
    Accent,
    Good,
    Warn,
    Bad,
    Info,
}
