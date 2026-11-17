//! What a work is judged on: axes, their marks and options, tiers, verdicts.

use serde::{Deserialize, Serialize};

use super::*;

/// One dimension a work is scored along.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Axis {
    pub key: String,
    pub label: Label,
    /// Relative importance when the axes are combined into a total.
    pub weight: f64,
    /// Highest value the axis accepts; scores are normalised against it. For
    /// a flag it is the worth of "yes"; for a choice it is the value the
    /// options are read against.
    pub scale: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Label>,
    /// What kind of answer the axis takes. Absent is a scale — the state of
    /// every axis written before the field existed.
    #[serde(default)]
    pub kind: AxisKind,
    /// The answers a `choice` axis offers, each worth a value on the scale.
    /// Meaningless, and required to be empty, for the other kinds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<AxisOption>,
    /// What the marks on this axis mean, for the marks worth naming.
    ///
    /// A rubric turns "is this a seven" from a feeling into a question with an
    /// answer: the craft says what a seven is, once, and every scoring after
    /// that is measured against the same sentence. Only the landmarks are
    /// named - three or so on a scale of ten - and a mark with nothing of its
    /// own reads the nearest named mark *below* it, so the whole scale is
    /// covered without the profile having to describe every step of it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rubric: Vec<AxisMark>,
}

/// The shape of an answer along an axis.
///
/// A scale is a number up to `scale`. A flag is yes or no — "has a chorus",
/// "explicit" — stored as a boolean and worth the whole scale or nothing. A
/// choice is one option from a short list, stored by its key, with the value
/// the option declares. All three land in the same 0–100 total, so a score
/// snapshot never needs to know which kind an axis was when it was taken.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum AxisKind {
    #[default]
    Scale,
    Flag,
    Choice,
}

/// What one mark on an axis means.
///
/// `at` is a mark on the axis's own scale, not on the 0-100 total: the person
/// scoring is looking at this axis, and a rubric written in totals would be
/// about a number they cannot see from here.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct AxisMark {
    pub at: f64,
    pub label: Label,
}

/// One answer a `choice` axis offers.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct AxisOption {
    /// Stored in the score snapshot; never renamed once scores hold it.
    pub key: String,
    pub label: Label,
    /// What the answer is worth, on the axis's scale.
    pub value: f64,
}

impl Axis {
    /// The number a stored answer is worth on this axis, if it is readable.
    ///
    /// A number is taken as-is for every kind: a snapshot that predates the
    /// axis becoming a flag or a choice still reads. A boolean reads as the
    /// scale or zero, and a string names a choice option.
    pub fn value_of(&self, stored: &serde_json::Value) -> Option<f64> {
        match stored {
            serde_json::Value::Number(number) => number.as_f64(),
            serde_json::Value::Bool(yes) => Some(if *yes { self.scale } else { 0.0 }),
            serde_json::Value::String(key) => self
                .options
                .iter()
                .find(|option| &option.key == key)
                .map(|option| option.value),
            _ => None,
        }
    }
}

/// A band a total score falls into. `min` is on the normalised 0–100 total.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Tier {
    pub key: String,
    pub label: Label,
    pub min: f64,
}

/// What one release kind makes of a set of answers.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct KindVerdict {
    pub kind: String,
    pub total: f64,
    pub tier: Option<String>,
    /// False when the kind weighs the axes exactly as the profile does, so a
    /// reader can tell a real second opinion from an echo of the first.
    pub reweighed: bool,
}
