//! How often a kind is meant to go out: the rhythm and its monthly goals.

use serde::{Deserialize, Serialize};

/// The pace releases go out at.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Rhythm {
    /// Days the auto-layout keeps between releases. 1 is daily.
    pub every_days: u32,
    /// Time of day a release usually ships (HH:MM), shown beside the date when
    /// editing a release. Slots themselves stay dates: the contest is per day,
    /// and a time would split it.
    #[serde(default)]
    pub default_time: Option<String>,
    /// What a month should hold, by tier or by kind of work (v0.111): "four of
    /// the top tier a month", "eight shorts a month". `every_days` paces the
    /// whole output and says nothing of what it is made of; a month can keep
    /// the pace on shorts alone. The window says which goal a month falls
    /// short of, and by how many. Empty for a profile that sets none.
    #[serde(default)]
    pub monthly: Vec<MonthlyGoal>,
    /// How many days before a planned slot's day it starts to remind
    /// (v0.138, FR-11), at least one.
    #[serde(default = "seven", skip_serializing_if = "is_seven")]
    #[ts(optional, as = "Option<u32>")]
    pub plan_lead_days: u32,
    /// How near a planned slot's day something made from its work must be
    /// booked to close it, in days either side (v0.138).
    #[serde(default = "seven", skip_serializing_if = "is_seven")]
    #[ts(optional, as = "Option<u32>")]
    pub plan_match_window_days: u32,
}

fn seven() -> u32 {
    7
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_seven(days: &u32) -> bool {
    *days == 7
}

/// How many releases of one sort a month should hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct MonthlyGoal {
    /// What is counted.
    #[serde(flatten)]
    pub of: GoalOf,
    /// How many a month, at least one.
    pub count: u32,
}

/// What a monthly goal counts: one or the other, never both and never
/// neither - a goal "of the top tier and of shorts" is two goals, and a goal
/// of nothing counts nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum GoalOf {
    /// Releases whose tier this is - the tier of what goes out, which a clip,
    /// a short and an audio release take from their song.
    Tier(String),
    /// Releases of works of this kind.
    Kind(String),
}
