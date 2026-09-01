use serde::{Deserialize, Serialize};

/// A craft scenario. Everything that differs between music, prose and podcasting
/// is described here rather than in the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    /// Kinds a work can take: song, chapter, episode.
    pub work_kinds: Vec<Kind>,
    /// Kinds a release can take: clip, short, audio release.
    pub release_kinds: Vec<ReleaseKind>,
    /// Kinds a collection can take: album, book, season.
    pub collection_kinds: Vec<Kind>,
    /// Independent bodies a work carries: lyrics and style, or text and outline.
    pub version_roles: Vec<VersionRole>,
    /// Statuses a work moves through, in order.
    pub statuses: Vec<Status>,
    /// What a work is judged on.
    pub axes: Vec<Axis>,
    /// Score thresholds, highest `min` first when evaluated.
    pub tiers: Vec<Tier>,
    /// Craft-specific fields stored in `work.meta`.
    pub work_meta_fields: Vec<MetaField>,
    /// Flags a work can be given by hand, beside the derived status. Defaulted
    /// so a profile written before they existed still loads.
    #[serde(default)]
    pub marks: Vec<Mark>,
    /// Actions the AI panel offers. Defaulted so a profile written before the
    /// panel existed still loads.
    #[serde(default)]
    pub prompts: Vec<crate::assistant::prompt::PromptTemplate>,
    /// How often the craft aims to ship. Absent in a profile written before the
    /// field existed; without it the auto-layout has nothing to pace by and
    /// refuses rather than inventing a cadence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rhythm: Option<Rhythm>,
}

/// The pace releases go out at.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rhythm {
    /// Days the auto-layout keeps between releases. 1 is daily.
    pub every_days: u32,
    /// Time of day a release usually ships (HH:MM), shown beside the date when
    /// editing a release. Slots themselves stay dates: the contest is per day,
    /// and a time would split it.
    #[serde(default)]
    pub default_time: Option<String>,
}

/// A vocabulary entry: a stable key with a label the user may rename.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kind {
    pub key: String,
    pub label: String,
}

impl Kind {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
        }
    }
}

/// A kind of release, and what a release of it cannot ship without.
///
/// `requires` names version roles: a clip needs lyrics and a style prompt, a
/// beta read needs the text. An empty list means the kind states no
/// requirements — readiness is then judged on the universal facts alone, and
/// every role mark reads as "not applicable" rather than "missing". A profile
/// written before this field existed loads that way.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseKind {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub requires: Vec<String>,
    /// The glyph the calendar draws this kind with, named from a fixed set the
    /// frontend knows. The profile names it because the code is not allowed to
    /// know which kinds exist (ADR 0001) -- a clip and a beta read have nothing
    /// in common but the shape of the row they sit in. Absent, or naming a
    /// glyph the set does not hold, falls back to a neutral one: a profile
    /// written before this field existed still loads, and a typo costs an
    /// icon rather than a screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl ReleaseKind {
    pub fn new(key: &str, label: &str, requires: &[&str]) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
            requires: requires.iter().map(|role| (*role).to_owned()).collect(),
            icon: None,
        }
    }

    /// The same kind, drawn with a named glyph.
    pub fn with_icon(mut self, icon: &str) -> Self {
        self.icon = Some(icon.to_owned());
        self
    }
}

/// A status a work can hold.
///
/// The label is the owner's word — `Released`, `Published`, `Выпущено` — so the
/// automation cannot recognise a status by its key. `derive` is what it reads
/// instead: the meaning behind the word, stated once by whoever wrote the
/// profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub key: String,
    pub label: String,
    /// What this status means to the automation. Absent is the same as
    /// `Manual`: a profile written before this field existed keeps its statuses
    /// under the owner's hand rather than having meaning guessed for it.
    #[serde(default)]
    pub derive: Derive,
}

impl Status {
    pub fn new(key: &str, label: &str, derive: Derive) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
            derive,
        }
    }
}

/// The meaning of a status, in descending finality.
///
/// The order of the variants is the order the automation checks them in, and
/// `Ord` is derived from it deliberately: "which of these two is further along"
/// is the whole question the automation asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
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

/// One dimension a work is scored along.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Axis {
    pub key: String,
    pub label: String,
    /// Relative importance when the axes are combined into a total.
    pub weight: f64,
    /// Highest value the axis accepts; scores are normalised against it.
    pub scale: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A band a total score falls into. `min` is on the normalised 0–100 total.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tier {
    pub key: String,
    pub label: String,
    pub min: f64,
}

/// An independent body a work carries.
///
/// Most roles stand alone — lyrics and style advance separately, and showing
/// them interleaved would suggest otherwise. A role that names `comments_on`
/// does not: it is written *about* another role, and reading it away from what
/// it discusses is reading half of it. That is the whole reason the field
/// exists rather than the code knowing which keys are commentary: the craft
/// says what comments on what, the same way it says everything else (ADR 0001).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRole {
    pub key: String,
    pub label: String,
    /// The role this one discusses, if any. A key no role defines is ignored,
    /// which keeps a half-edited profile from breaking the card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comments_on: Option<String>,
}

impl VersionRole {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
            comments_on: None,
        }
    }
}

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mark {
    pub key: String,
    pub label: String,
    /// One of the palette's own roles, so a mark reads correctly in both
    /// themes. A free-form colour would be a colour nobody guaranteed contrast
    /// for.
    #[serde(default)]
    pub colour: MarkColour,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MarkColour {
    /// The neutral one: a flag that carries no urgency of its own.
    #[default]
    Plain,
    Accent,
    Good,
    Warn,
    Bad,
}

/// A typed field inside `work.meta`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: MetaFieldType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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
}

impl ProfileConfig {
    /// Combine axis values into a 0–100 total.
    ///
    /// Axes missing from `values` are skipped rather than counted as zero: a
    /// half-filled score card should not read as a bad work.
    pub fn total(&self, values: &serde_json::Map<String, serde_json::Value>) -> f64 {
        let mut weighted = 0.0;
        let mut weight_sum = 0.0;

        for axis in &self.axes {
            let Some(value) = values.get(&axis.key).and_then(serde_json::Value::as_f64) else {
                continue;
            };
            if axis.scale <= 0.0 {
                continue;
            }
            weighted += (value / axis.scale) * axis.weight;
            weight_sum += axis.weight;
        }

        if weight_sum == 0.0 {
            return 0.0;
        }
        (weighted / weight_sum) * 100.0
    }

    /// The highest tier whose threshold the total reaches.
    pub fn tier_for(&self, total: f64) -> Option<&Tier> {
        self.tiers
            .iter()
            .filter(|tier| total >= tier.min)
            .max_by(|a, b| a.min.total_cmp(&b.min))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn config() -> ProfileConfig {
        serde_json::from_value(json!({
            "work_kinds": [{ "key": "song", "label": "Song" }],
            "release_kinds": [{ "key": "clip", "label": "Clip" }],
            "collection_kinds": [{ "key": "album", "label": "Album" }],
            "version_roles": [{ "key": "lyrics", "label": "Lyrics" }],
            "statuses": [{ "key": "draft", "label": "Draft" }],
            "axes": [
                { "key": "hook", "label": "Hook", "weight": 2.0, "scale": 10.0 },
                { "key": "text", "label": "Text", "weight": 1.0, "scale": 10.0 }
            ],
            "tiers": [
                { "key": "hold", "label": "Hold", "min": 0.0 },
                { "key": "clip", "label": "Clip", "min": 75.0 }
            ],
            "work_meta_fields": []
        }))
        .unwrap()
    }

    // The fixture's release kind carries no `requires`, as every profile
    // written before the field existed does.
    #[test]
    fn a_release_kind_without_requirements_parses_as_requiring_nothing() {
        assert!(config().release_kinds[0].requires.is_empty());
    }

    // The fixture states no rhythm either — a profile from before the field
    // must load, and the absence must read as "no rhythm" rather than a guess.
    #[test]
    fn a_profile_without_a_rhythm_parses_as_having_none() {
        assert!(config().rhythm.is_none());
    }

    #[test]
    fn a_rhythm_needs_no_default_time() {
        let rhythm: Rhythm = serde_json::from_value(json!({ "every_days": 3 })).unwrap();
        assert_eq!(rhythm.every_days, 3);
        assert!(rhythm.default_time.is_none());
    }

    #[test]
    fn total_weighs_the_axes() {
        let values = json!({ "hook": 10.0, "text": 4.0 });
        // (1.0 * 2 + 0.4 * 1) / 3 = 0.8
        let total = config().total(values.as_object().unwrap());
        assert!((total - 80.0).abs() < 1e-9, "got {total}");
    }

    #[test]
    fn a_missing_axis_does_not_count_as_zero() {
        let values = json!({ "hook": 8.0 });
        let total = config().total(values.as_object().unwrap());
        assert!((total - 80.0).abs() < 1e-9, "got {total}");
    }

    #[test]
    fn an_empty_score_card_totals_zero_rather_than_dividing_by_zero() {
        let values = json!({});
        assert_eq!(config().total(values.as_object().unwrap()), 0.0);
    }

    #[test]
    fn tier_picks_the_highest_threshold_reached() {
        let config = config();
        assert_eq!(config.tier_for(80.0).unwrap().key, "clip");
        assert_eq!(config.tier_for(74.9).unwrap().key, "hold");
    }
}
