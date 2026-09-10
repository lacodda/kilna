use std::collections::{BTreeMap, BTreeSet};

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
    /// Which columns the catalogue shows, by column id, in order. A novel and a
    /// record are read down different columns, which is what makes this a fact
    /// about the craft rather than about the machine. Absent means the
    /// catalogue's own default; the ids are the frontend's, and one it no
    /// longer knows is dropped on read rather than refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalogue_columns: Option<Vec<String>>,
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
    /// Axis weights that apply when a work is judged *for this kind* of
    /// release, keyed by axis key. A clip lives or dies on its hook and its
    /// visuals; the same song as an audio release is carried by its lyrics.
    /// An axis not named here keeps the weight the axis itself declares, so a
    /// kind may reweight one axis and say nothing about the rest. Empty — the
    /// state of every profile written before the field — means the axes'
    /// own weights, and one tier for every kind.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub axis_weights: BTreeMap<String, f64>,
}

impl ReleaseKind {
    pub fn new(key: &str, label: &str, requires: &[&str]) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
            requires: requires.iter().map(|role| (*role).to_owned()).collect(),
            icon: None,
            axis_weights: BTreeMap::new(),
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
    /// Highest value the axis accepts; scores are normalised against it. For
    /// a flag it is the worth of "yes"; for a choice it is the value the
    /// options are read against.
    pub scale: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisMark {
    pub at: f64,
    pub label: String,
}

/// One answer a `choice` axis offers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisOption {
    /// Stored in the score snapshot; never renamed once scores hold it.
    pub key: String,
    pub label: String,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tier {
    pub key: String,
    pub label: String,
    pub min: f64,
}

/// What one release kind makes of a set of answers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KindVerdict {
    pub kind: String,
    pub total: f64,
    pub tier: Option<String>,
    /// False when the kind weighs the axes exactly as the profile does, so a
    /// reader can tell a real second opinion from an echo of the first.
    pub reweighed: bool,
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
    /// half-filled score card should not read as a bad work. An answer the
    /// axis cannot read — a choice key the profile no longer offers — is
    /// skipped the same way.
    pub fn total(&self, values: &serde_json::Map<String, serde_json::Value>) -> f64 {
        self.weigh(values, |axis| axis.weight)
    }

    /// The same values, weighed as a release of `kind` would weigh them.
    ///
    /// A kind that reweights nothing — or a key that names no kind — gives
    /// exactly [`total`](Self::total): one verdict, not a second opinion.
    pub fn total_for(
        &self,
        values: &serde_json::Map<String, serde_json::Value>,
        release_kind: &str,
    ) -> f64 {
        let weights = self
            .release_kinds
            .iter()
            .find(|kind| kind.key == release_kind)
            .map(|kind| &kind.axis_weights);
        self.weigh(values, |axis| {
            weights
                .and_then(|weights| weights.get(&axis.key))
                .copied()
                .unwrap_or(axis.weight)
        })
    }

    /// What each release kind makes of the same answers.
    ///
    /// One score, read down every channel the craft ships to: a song that is
    /// a clip and a merely adequate audio release is one work with two honest
    /// verdicts, not a work whose number is wrong. Kinds that reweigh nothing
    /// still appear - the point is the comparison, and a row missing from it
    /// would read as "not applicable" rather than "same as the others".
    pub fn verdicts(
        &self,
        values: &serde_json::Map<String, serde_json::Value>,
    ) -> Vec<KindVerdict> {
        self.release_kinds
            .iter()
            .map(|kind| {
                let total = self.total_for(values, &kind.key);
                KindVerdict {
                    kind: kind.key.clone(),
                    total,
                    tier: self.tier_for(total).map(|tier| tier.key.clone()),
                    reweighed: !kind.axis_weights.is_empty(),
                }
            })
            .collect()
    }

    fn weigh(
        &self,
        values: &serde_json::Map<String, serde_json::Value>,
        weight_of: impl Fn(&Axis) -> f64,
    ) -> f64 {
        let mut weighted = 0.0;
        let mut weight_sum = 0.0;

        for axis in &self.axes {
            let Some(value) = values
                .get(&axis.key)
                .and_then(|stored| axis.value_of(stored))
            else {
                continue;
            };
            if axis.scale <= 0.0 {
                continue;
            }
            let weight = weight_of(axis);
            weighted += (value / axis.scale) * weight;
            weight_sum += weight;
        }

        if weight_sum == 0.0 {
            return 0.0;
        }
        (weighted / weight_sum) * 100.0
    }

    /// Everything wrong with the document, in the words a person can act on.
    ///
    /// Empty means the profile is sound. Each line names the place — "axis 3
    /// (`hook`)" — and the rule it breaks, because a profile is edited by
    /// hand and "invalid config" sends the person back to guess. Only what
    /// would make the app misbehave is refused: an empty label is a taste,
    /// a duplicate key is a corruption waiting for the next score.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();

        fn unique(problems: &mut Vec<String>, what: &str, keys: impl Iterator<Item = String>) {
            let mut seen = BTreeSet::new();
            for (index, key) in keys.enumerate() {
                if key.trim().is_empty() {
                    problems.push(format!("{what} {} has no key", index + 1));
                } else if !seen.insert(key.clone()) {
                    problems.push(format!("{what} {} repeats the key `{key}`", index + 1));
                }
            }
        }

        unique(
            &mut problems,
            "work kind",
            self.work_kinds.iter().map(|k| k.key.clone()),
        );
        unique(
            &mut problems,
            "release kind",
            self.release_kinds.iter().map(|k| k.key.clone()),
        );
        unique(
            &mut problems,
            "collection kind",
            self.collection_kinds.iter().map(|k| k.key.clone()),
        );
        unique(
            &mut problems,
            "version role",
            self.version_roles.iter().map(|r| r.key.clone()),
        );
        unique(
            &mut problems,
            "status",
            self.statuses.iter().map(|s| s.key.clone()),
        );
        unique(
            &mut problems,
            "axis",
            self.axes.iter().map(|a| a.key.clone()),
        );
        unique(
            &mut problems,
            "tier",
            self.tiers.iter().map(|t| t.key.clone()),
        );
        unique(
            &mut problems,
            "meta field",
            self.work_meta_fields.iter().map(|f| f.key.clone()),
        );
        unique(
            &mut problems,
            "mark",
            self.marks.iter().map(|m| m.key.clone()),
        );
        unique(
            &mut problems,
            "prompt",
            self.prompts.iter().map(|p| p.key.clone()),
        );

        if self.statuses.is_empty() {
            problems.push("the profile names no statuses; a work has to start somewhere".into());
        }
        if self.work_kinds.is_empty() {
            problems.push("the profile names no work kinds".into());
        }

        let roles: BTreeSet<&str> = self.version_roles.iter().map(|r| r.key.as_str()).collect();
        for (index, role) in self.version_roles.iter().enumerate() {
            // Not a let-chain: the MSRV is older than they are.
            let Some(target) = &role.comments_on else {
                continue;
            };
            if !roles.contains(target.as_str()) {
                problems.push(format!(
                    "version role {} (`{}`) comments on `{target}`, which no role is",
                    index + 1,
                    role.key
                ));
            }
        }

        let axes: BTreeSet<&str> = self.axes.iter().map(|a| a.key.as_str()).collect();
        for (index, kind) in self.release_kinds.iter().enumerate() {
            for required in &kind.requires {
                if !roles.contains(required.as_str()) {
                    problems.push(format!(
                        "release kind {} (`{}`) requires `{required}`, which no version role is",
                        index + 1,
                        kind.key
                    ));
                }
            }
            for (axis, weight) in &kind.axis_weights {
                if !axes.contains(axis.as_str()) {
                    problems.push(format!(
                        "release kind {} (`{}`) weights `{axis}`, which no axis is",
                        index + 1,
                        kind.key
                    ));
                }
                if !(weight.is_finite() && *weight >= 0.0) {
                    problems.push(format!(
                        "release kind {} (`{}`) gives `{axis}` the weight {weight}; it must be zero or above",
                        index + 1,
                        kind.key
                    ));
                }
            }
        }

        for (index, axis) in self.axes.iter().enumerate() {
            let place = format!("axis {} (`{}`)", index + 1, axis.key);
            if !(axis.scale.is_finite() && axis.scale > 0.0) {
                problems.push(format!(
                    "{place} has the scale {}; it must be above zero",
                    axis.scale
                ));
            }
            if !(axis.weight.is_finite() && axis.weight >= 0.0) {
                problems.push(format!(
                    "{place} has the weight {}; it must be zero or above",
                    axis.weight
                ));
            }
            match axis.kind {
                AxisKind::Choice => {
                    if axis.options.is_empty() {
                        problems.push(format!("{place} is a choice with nothing to choose from"));
                    }
                    unique(
                        &mut problems,
                        &format!("{place}: option"),
                        axis.options.iter().map(|o| o.key.clone()),
                    );
                    for option in &axis.options {
                        if !(option.value.is_finite()
                            && option.value >= 0.0
                            && option.value <= axis.scale)
                        {
                            problems.push(format!(
                                "{place}: option `{}` is worth {}, outside 0–{}",
                                option.key, option.value, axis.scale
                            ));
                        }
                    }
                }
                AxisKind::Scale | AxisKind::Flag => {
                    if !axis.options.is_empty() {
                        problems.push(format!(
                            "{place} lists options but is not a choice; set `kind` to `choice` or drop them"
                        ));
                    }
                }
            }

            // A rubric describes marks on this axis, so a mark off the scale
            // describes nothing, and two sentences about the same mark leave
            // the app to pick one of them.
            let mut named = BTreeSet::new();
            for mark in &axis.rubric {
                if !(mark.at.is_finite() && mark.at >= 0.0 && mark.at <= axis.scale) {
                    problems.push(format!(
                        "{place}: the rubric names {}, outside 0-{}",
                        mark.at, axis.scale
                    ));
                    continue;
                }
                if !named.insert(mark.at.to_bits()) {
                    problems.push(format!("{place}: the rubric names {} twice", mark.at));
                }
            }
        }

        for (index, tier) in self.tiers.iter().enumerate() {
            if !(tier.min.is_finite() && (0.0..=100.0).contains(&tier.min)) {
                problems.push(format!(
                    "tier {} (`{}`) starts at {}; the total runs 0–100",
                    index + 1,
                    tier.key,
                    tier.min
                ));
            }
        }

        if let Some(rhythm) = &self.rhythm {
            if rhythm.every_days == 0 {
                problems.push("the rhythm must be at least one day".into());
            }
            if let Some(time) = &rhythm.default_time {
                if !crate::time::is_clock_time(time) {
                    problems.push(format!(
                        "the rhythm's default time `{time}` is not a time of day (HH:MM)"
                    ));
                }
            }
        }

        problems
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

    // A profile from before rubrics existed must load, with the axis simply
    // saying nothing about what its marks mean.
    #[test]
    fn an_axis_without_a_rubric_parses_as_naming_no_marks() {
        assert!(config().axes[0].rubric.is_empty());
    }

    #[test]
    fn a_rubric_names_marks_on_the_axis_scale() {
        let axis: Axis = serde_json::from_value(json!({
            "key": "hook", "label": "Hook", "weight": 2.0, "scale": 10.0,
            "rubric": [
                { "at": 3.0, "label": "audible, but it does not catch" },
                { "at": 7.0, "label": "the chorus sticks on the first listen" }
            ]
        }))
        .unwrap();

        assert_eq!(axis.rubric.len(), 2);
        assert_eq!(axis.rubric[1].at, 7.0);
    }

    #[test]
    fn a_rubric_mark_off_the_scale_is_refused() {
        let mut config = config();
        config.axes[0].rubric = vec![AxisMark {
            at: 12.0,
            label: "beyond the scale".into(),
        }];

        let problems = config.validate();
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("outside 0-10")),
            "got {problems:?}"
        );
    }

    #[test]
    fn a_rubric_naming_the_same_mark_twice_is_refused() {
        let mut config = config();
        config.axes[0].rubric = vec![
            AxisMark {
                at: 7.0,
                label: "one sentence".into(),
            },
            AxisMark {
                at: 7.0,
                label: "and another".into(),
            },
        ];

        let problems = config.validate();
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("names 7 twice")),
            "got {problems:?}"
        );
    }

    #[test]
    fn a_sound_rubric_is_not_refused() {
        let mut config = config();
        config.axes[0].rubric = vec![
            AxisMark {
                at: 0.0,
                label: "the bottom of the scale is a verdict too".into(),
            },
            AxisMark {
                at: 10.0,
                label: "and so is the top".into(),
            },
        ];

        assert!(config.validate().is_empty(), "got {:?}", config.validate());
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

    fn typed() -> ProfileConfig {
        serde_json::from_value(json!({
            "work_kinds": [{ "key": "song", "label": "Song" }],
            "release_kinds": [
                { "key": "clip", "label": "Clip", "axis_weights": { "hook": 4.0, "chorus": 0.0 } },
                { "key": "audio", "label": "Audio" }
            ],
            "collection_kinds": [],
            "version_roles": [{ "key": "lyrics", "label": "Lyrics" }],
            "statuses": [{ "key": "draft", "label": "Draft" }],
            "axes": [
                { "key": "hook", "label": "Hook", "weight": 1.0, "scale": 10.0 },
                { "key": "chorus", "label": "Has a chorus", "weight": 1.0, "scale": 10.0, "kind": "flag" },
                { "key": "length", "label": "Length", "weight": 1.0, "scale": 10.0, "kind": "choice",
                  "options": [
                      { "key": "short", "label": "Short", "value": 4.0 },
                      { "key": "right", "label": "Right", "value": 10.0 }
                  ] }
            ],
            "tiers": [],
            "work_meta_fields": []
        }))
        .unwrap()
    }

    #[test]
    fn an_axis_written_before_kinds_existed_is_a_scale() {
        assert_eq!(config().axes[0].kind, AxisKind::Scale);
        assert!(config().axes[0].options.is_empty());
    }

    #[test]
    fn a_flag_is_worth_the_scale_or_nothing_and_a_choice_its_option() {
        let config = typed();
        let values = json!({ "hook": 5.0, "chorus": true, "length": "short" });
        // (0.5 + 1.0 + 0.4) / 3
        let total = config.total(values.as_object().unwrap());
        assert!((total - 63.333_333).abs() < 1e-3, "got {total}");

        let values = json!({ "hook": 5.0, "chorus": false, "length": "short" });
        let total = config.total(values.as_object().unwrap());
        assert!((total - 30.0).abs() < 1e-9, "got {total}");
    }

    #[test]
    fn an_answer_the_axis_cannot_read_is_skipped_like_a_missing_one() {
        let config = typed();
        let values = json!({ "hook": 5.0, "length": "epic" });
        let total = config.total(values.as_object().unwrap());
        assert!(
            (total - 50.0).abs() < 1e-9,
            "an unknown option must not count: {total}"
        );
    }

    #[test]
    fn a_number_still_reads_on_a_flag_or_a_choice() {
        // A snapshot taken while the axis was a scale keeps its value.
        let config = typed();
        let values = json!({ "chorus": 10.0, "length": 10.0 });
        let total = config.total(values.as_object().unwrap());
        assert!((total - 100.0).abs() < 1e-9, "got {total}");
    }

    #[test]
    fn a_release_kind_reweighs_the_axes_it_names_and_keeps_the_rest() {
        let config = typed();
        let values = json!({ "hook": 10.0, "chorus": false, "length": "short" });
        let values = values.as_object().unwrap();
        // Plain: (1.0 + 0 + 0.4) / 3
        assert!((config.total(values) - 46.666_666).abs() < 1e-3);
        // As a clip: hook ×4, chorus ×0, length keeps 1: (4.0 + 0 + 0.4) / 5
        let clip = config.total_for(values, "clip");
        assert!((clip - 88.0).abs() < 1e-9, "got {clip}");
        // A kind that reweights nothing, or none at all, is the plain total.
        assert_eq!(config.total_for(values, "audio"), config.total(values));
        assert_eq!(
            config.total_for(values, "no-such-kind"),
            config.total(values)
        );
    }

    #[test]
    fn every_release_kind_gets_a_verdict_even_when_it_reweighs_nothing() {
        let config = typed();
        let values = json!({ "hook": 10.0, "chorus": false, "length": "short" });
        let verdicts = config.verdicts(values.as_object().unwrap());

        // One row per kind: a kind missing from the comparison would read as
        // "not applicable" rather than "weighs them the same way".
        assert_eq!(verdicts.len(), config.release_kinds.len());

        let clip = verdicts.iter().find(|v| v.kind == "clip").unwrap();
        let audio = verdicts.iter().find(|v| v.kind == "audio").unwrap();

        assert!(clip.reweighed, "clip names its own weights");
        assert!(!audio.reweighed, "audio names none");

        // The verdicts are the per-kind totals, not the flat one repeated.
        assert!((clip.total - 88.0).abs() < 1e-9, "got {}", clip.total);
        assert_eq!(audio.total, config.total(values.as_object().unwrap()));
        assert!(
            clip.total > audio.total,
            "the same answers read better as a clip: {} vs {}",
            clip.total,
            audio.total
        );
    }

    #[test]
    fn a_verdict_carries_the_tier_its_own_total_reaches() {
        let config = typed();
        let values = json!({ "hook": 10.0, "chorus": false, "length": "short" });
        let verdicts = config.verdicts(values.as_object().unwrap());

        for verdict in &verdicts {
            // The claim is checkable: each tier is the one the profile itself
            // returns for that verdict's own total, not for the flat total.
            let expected = config.tier_for(verdict.total).map(|t| t.key.clone());
            assert_eq!(verdict.tier, expected, "kind {}", verdict.kind);
        }
    }

    #[test]
    fn a_sound_profile_has_no_problems() {
        assert_eq!(typed().validate(), Vec::<String>::new());
        assert_eq!(config().validate(), Vec::<String>::new());
    }

    #[test]
    fn every_problem_names_its_place() {
        let mut config = typed();
        config.axes.push(config.axes[0].clone());
        config.axes[1].scale = 0.0;
        config.axes[2].options.clear();
        config.tiers.push(Tier {
            key: "top".into(),
            label: "Top".into(),
            min: 140.0,
        });
        config.release_kinds[0].requires.push("melody".into());
        config.release_kinds[0]
            .axis_weights
            .insert("ghost".into(), 1.0);
        config.version_roles.push(VersionRole {
            key: "review".into(),
            label: "Review".into(),
            comments_on: Some("prose".into()),
        });
        config.rhythm = Some(Rhythm {
            every_days: 0,
            default_time: Some("noon".into()),
        });

        let problems = config.validate();
        let expect = |needle: &str| {
            assert!(
                problems.iter().any(|p| p.contains(needle)),
                "no problem mentions `{needle}`:
{}",
                problems.join(
                    "
"
                )
            );
        };
        expect("axis 4 repeats the key `hook`");
        expect("axis 2 (`chorus`) has the scale 0");
        expect("axis 3 (`length`) is a choice with nothing to choose from");
        expect("tier 1 (`top`) starts at 140");
        expect("release kind 1 (`clip`) requires `melody`");
        expect("release kind 1 (`clip`) weights `ghost`");
        expect("version role 2 (`review`) comments on `prose`");
        expect("the rhythm must be at least one day");
        expect("default time `noon`");
    }

    #[test]
    fn options_on_a_scale_are_refused_rather_than_ignored() {
        let mut config = typed();
        config.axes[0].options.push(AxisOption {
            key: "x".into(),
            label: "X".into(),
            value: 1.0,
        });
        let problems = config.validate();
        assert!(
            problems
                .iter()
                .any(|p| p.contains("lists options but is not a choice")),
            "{problems:?}"
        );
    }
}
