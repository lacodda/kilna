//! Where a work stands, and what moves it there.

use serde::{Deserialize, Serialize};

use super::*;

/// A status a work can hold.
///
/// The label is the owner's word — `Released`, `Published`, `Выпущено` — so the
/// automation cannot recognise a status by its key. `derive` is what it reads
/// instead: the meaning behind the word, stated once by whoever wrote the
/// profile.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Status {
    pub key: String,
    pub label: Label,
    /// What this status means to the automation. Absent is the same as
    /// `Manual`: a profile written before this field existed keeps its statuses
    /// under the owner's hand rather than having meaning guessed for it.
    #[serde(default)]
    pub derive: Derive,
    /// The palette role the status badge takes — emphasis, never the message:
    /// the word is always there beside it. Absent draws the badge in outline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<MarkColour>,
}

impl Status {
    pub fn new(key: &str, label: &str, derive: Derive) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            derive,
            colour: None,
        }
    }
}

/// The meaning of a status, in descending finality.
///
/// The order of the variants is the order the automation checks them in, and
/// `Ord` is derived from it deliberately: "which of these two is further along"
/// is the whole question the automation asks.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default, ts_rs::TS,
)]
#[serde(rename_all = "lowercase")]
pub enum Derive {
    /// Never derived. A decision someone made, with no fact in the data that
    /// could imply it — shelved, on hold, abandoned.
    #[default]
    Manual,
    /// Nothing has happened to it yet.
    Draft,
    /// It has been judged at least once.
    Scored,
    /// A release holds a calendar slot for it.
    Scheduled,
    /// It has gone out.
    Released,
}
