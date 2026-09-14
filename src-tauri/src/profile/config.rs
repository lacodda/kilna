use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// The version of the document's shape. See [`ProfileConfig::format`].
pub const FORMAT: u32 = 2;

/// A craft scenario. Everything that differs between music, prose and podcasting
/// is described here rather than in the schema.
///
/// **Format 2** (v0.57): the vocabulary a work is judged and shipped by — its
/// axes, tiers, version roles, kinds of release, statuses — belongs to the
/// *kind* of work, not to the profile. A song and a video are one craft with
/// two vocabularies, and a video judged on "hook" and "lyrics" is nonsense.
/// Format 1 laid all of it flat on the profile; a format 1 document is still
/// read — see [`RawProfileConfig`] — and comes out of the parser in format 2,
/// with the flat vocabulary handed to every kind that declared none of its own.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawProfileConfig")]
pub struct ProfileConfig {
    /// Which shape this document has. Written as [`FORMAT`]; read only to
    /// know whether a stored copy still needs rewriting.
    pub format: u32,
    /// Kinds a work can take — song, chapter, episode, video — each with the
    /// vocabulary it is judged and shipped by.
    pub work_kinds: Vec<WorkKind>,
    /// Kinds a collection can take: album, book, season.
    pub collection_kinds: Vec<Kind>,
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
    /// The columns the catalogue shows while it is narrowed to one kind of
    /// work, by kind key. A video is read down other columns than a song;
    /// a kind with no entry reads down `catalogue_columns`. An optional key
    /// added in format 2 — a document without it is the same document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalogue_columns_by_kind: Option<BTreeMap<String, Vec<String>>>,
    /// Kinds a note can take: a character, a location, a piece of lore, a
    /// plain note. One table with a kind rather than a table each (ADR 0001),
    /// and the kinds are the craft's words rather than the application's: a
    /// novel has characters and places, a podcast has guests and segments.
    ///
    /// A scene points at notes of these kinds, which is what makes "every
    /// scene with her in it" a question the board can answer. An optional key
    /// added in format 2 — a document without it is the same document, and a
    /// note keeps taking any kind a person writes.
    #[serde(default)]
    pub note_kinds: Vec<Kind>,
}

/// A profile document as it is written, in either format.
///
/// Format 1 laid the vocabulary flat on the profile; format 2 puts it under
/// each kind. Both are read: the flat fields, when any is present, are given
/// to every kind that declares nothing of its own — all of it or none of it,
/// so a kind that names even one list is taken to have named its vocabulary
/// on purpose — and then dropped. The document that comes out is format 2
/// whichever went in, which is what lets the stored copies, the shipped
/// files, an imported JSON and every test share one parser.
#[derive(Debug, Clone, Deserialize)]
pub struct RawProfileConfig {
    #[serde(default)]
    pub format: u32,
    pub work_kinds: Vec<WorkKind>,
    #[serde(default)]
    pub collection_kinds: Vec<Kind>,
    #[serde(default)]
    pub work_meta_fields: Vec<MetaField>,
    #[serde(default)]
    pub marks: Vec<Mark>,
    #[serde(default)]
    pub prompts: Vec<crate::assistant::prompt::PromptTemplate>,
    #[serde(default)]
    pub rhythm: Option<Rhythm>,
    #[serde(default)]
    pub catalogue_columns: Option<Vec<String>>,
    #[serde(default)]
    pub catalogue_columns_by_kind: Option<BTreeMap<String, Vec<String>>>,
    #[serde(default)]
    pub note_kinds: Vec<Kind>,
    // Format 1: the vocabulary, flat on the profile.
    #[serde(default)]
    pub release_kinds: Vec<ReleaseKind>,
    #[serde(default)]
    pub version_roles: Vec<VersionRole>,
    #[serde(default)]
    pub statuses: Vec<Status>,
    #[serde(default)]
    pub axes: Vec<Axis>,
    #[serde(default)]
    pub tiers: Vec<Tier>,
}

impl From<RawProfileConfig> for ProfileConfig {
    fn from(raw: RawProfileConfig) -> Self {
        let flat_present = !raw.axes.is_empty()
            || !raw.tiers.is_empty()
            || !raw.version_roles.is_empty()
            || !raw.release_kinds.is_empty()
            || !raw.statuses.is_empty();

        let mut work_kinds = raw.work_kinds;
        if flat_present {
            for kind in &mut work_kinds {
                if kind.declares_nothing() {
                    kind.axes = raw.axes.clone();
                    kind.tiers = raw.tiers.clone();
                    kind.version_roles = raw.version_roles.clone();
                    kind.release_kinds = raw.release_kinds.clone();
                    kind.statuses = raw.statuses.clone();
                }
            }
        }

        Self {
            format: FORMAT,
            work_kinds,
            collection_kinds: raw.collection_kinds,
            work_meta_fields: raw.work_meta_fields,
            marks: raw.marks,
            prompts: raw.prompts,
            rhythm: raw.rhythm,
            catalogue_columns: raw.catalogue_columns,
            catalogue_columns_by_kind: raw.catalogue_columns_by_kind,
            note_kinds: raw.note_kinds,
        }
    }
}

/// A kind of work, with everything the craft says about works of that kind.
///
/// The vocabulary lives here rather than on the profile because a craft ships
/// more than one thing — a studio makes songs and the videos for them — and
/// the two are judged on different axes, carry different bodies and go out
/// through different doors. A kind that declares no axes is scored empty
/// rather than on someone else's; a kind that declares no statuses cannot
/// hold a work, and validation says so.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkKind {
    pub key: String,
    pub label: String,
    /// What a work of this kind is judged on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub axes: Vec<Axis>,
    /// Score thresholds, on the 0–100 total.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tiers: Vec<Tier>,
    /// Independent bodies a work of this kind carries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub version_roles: Vec<VersionRole>,
    /// Kinds of release a work of this kind goes out as.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub release_kinds: Vec<ReleaseKind>,
    /// Statuses a work of this kind moves through, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub statuses: Vec<Status>,
    /// Kinds of shot a scene of this kind's storyboard can be — wide, close,
    /// detail. A kind that names none (a song) has no storyboard. An optional
    /// key added in v0.60 — a document without it is the same document.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shot_types: Vec<Kind>,
    /// The prompt blocks a scene carries — a still frame, an animation, a
    /// negative — each edited and copied on its own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scene_blocks: Vec<SceneBlock>,
}

impl WorkKind {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
            axes: Vec::new(),
            tiers: Vec::new(),
            version_roles: Vec::new(),
            release_kinds: Vec::new(),
            statuses: Vec::new(),
            shot_types: Vec::new(),
            scene_blocks: Vec::new(),
        }
    }

    /// Whether the kind states no vocabulary at all — the state a format 1
    /// kind is in, and the one the flat vocabulary fills.
    pub fn declares_nothing(&self) -> bool {
        self.axes.is_empty()
            && self.tiers.is_empty()
            && self.version_roles.is_empty()
            && self.release_kinds.is_empty()
            && self.statuses.is_empty()
    }

    /// The same kind without its judgement: axes and tiers dropped, the
    /// structural vocabulary kept. What a kind newly shipped into an existing
    /// workspace arrives as — a stranger's axes must not appear silently
    /// beside the owner's own (see `profile::carry_forward`).
    pub fn without_judgement(&self) -> Self {
        Self {
            axes: Vec::new(),
            tiers: Vec::new(),
            ..self.clone()
        }
    }

    /// Combine axis values into a 0–100 total.
    ///
    /// Axes missing from `values` are skipped rather than counted as zero: a
    /// half-filled score card should not read as a bad work. An answer the
    /// axis cannot read — a choice key the profile no longer offers — is
    /// skipped the same way. A kind with no axes totals zero.
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

    /// The highest tier whose threshold the total reaches.
    pub fn tier_for(&self, total: f64) -> Option<&Tier> {
        self.tiers
            .iter()
            .filter(|tier| total >= tier.min)
            .max_by(|a, b| a.min.total_cmp(&b.min))
    }

    /// The status a work of this kind starts in: the one that means draft,
    /// or the first listed for a kind that names no draft.
    pub fn starting_status(&self) -> Option<&Status> {
        self.statuses
            .iter()
            .find(|status| status.derive == Derive::Draft)
            .or_else(|| self.statuses.first())
    }

    /// The status that means `meaning` to the automation, if the kind has one.
    pub fn status_meaning(&self, meaning: Derive) -> Option<&Status> {
        if meaning == Derive::Manual {
            return None;
        }
        self.statuses.iter().find(|status| status.derive == meaning)
    }

    /// Everything wrong with this kind's vocabulary, each line naming its place.
    fn validate_into(&self, problems: &mut Vec<String>, place: &str) {
        unique(
            problems,
            &format!("{place}: release kind"),
            self.release_kinds.iter().map(|k| k.key.clone()),
        );
        unique(
            problems,
            &format!("{place}: version role"),
            self.version_roles.iter().map(|r| r.key.clone()),
        );
        unique(
            problems,
            &format!("{place}: status"),
            self.statuses.iter().map(|s| s.key.clone()),
        );
        unique(
            problems,
            &format!("{place}: axis"),
            self.axes.iter().map(|a| a.key.clone()),
        );
        unique(
            problems,
            &format!("{place}: tier"),
            self.tiers.iter().map(|t| t.key.clone()),
        );
        unique(
            problems,
            &format!("{place}: kind of shot"),
            self.shot_types.iter().map(|s| s.key.clone()),
        );
        unique(
            problems,
            &format!("{place}: scene block"),
            self.scene_blocks.iter().map(|b| b.key.clone()),
        );

        if self.statuses.is_empty() {
            problems.push(format!(
                "{place} names no statuses; a work has to start somewhere"
            ));
        }

        let roles: BTreeSet<&str> = self.version_roles.iter().map(|r| r.key.as_str()).collect();
        for (index, role) in self.version_roles.iter().enumerate() {
            // Not a let-chain: the MSRV is older than they are.
            let Some(target) = &role.comments_on else {
                continue;
            };
            if !roles.contains(target.as_str()) {
                problems.push(format!(
                    "{place}: version role {} (`{}`) comments on `{target}`, which no role is",
                    index + 1,
                    role.key
                ));
            }
        }
        for (index, role) in self.version_roles.iter().enumerate() {
            let Some(body) = &role.body else {
                continue;
            };
            if !BODY_KINDS.contains(&body.as_str()) {
                problems.push(format!(
                    "{place}: version role {} (`{}`) reads its body as `{body}`; it can only be `plain` or `markdown`",
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
                        "{place}: release kind {} (`{}`) requires `{required}`, which no version role is",
                        index + 1,
                        kind.key
                    ));
                }
            }
            for (axis, weight) in &kind.axis_weights {
                if !axes.contains(axis.as_str()) {
                    problems.push(format!(
                        "{place}: release kind {} (`{}`) weights `{axis}`, which no axis is",
                        index + 1,
                        kind.key
                    ));
                }
                if !(weight.is_finite() && *weight >= 0.0) {
                    problems.push(format!(
                        "{place}: release kind {} (`{}`) gives `{axis}` the weight {weight}; it must be zero or above",
                        index + 1,
                        kind.key
                    ));
                }
            }
        }

        for (index, axis) in self.axes.iter().enumerate() {
            let place = format!("{place}: axis {} (`{}`)", index + 1, axis.key);
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
                        problems,
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

            // A rubric is checked like the options are: a mark off the scale
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
                    "{place}: tier {} (`{}`) starts at {}; the total runs 0–100",
                    index + 1,
                    tier.key,
                    tier.min
                ));
            }
        }
    }
}

/// Keys that repeat or are blank, reported with their position.
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

/// The vocabulary of a kind the profile does not know: nothing, so that a
/// work whose kind was removed from the profile reads as unscorable and
/// roleless rather than crashing the screen it is on.
static NO_KIND: std::sync::LazyLock<WorkKind> = std::sync::LazyLock::new(|| WorkKind::new("", ""));

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

/// One prompt block a scene carries: a still frame, an animation, a
/// negative. The key is what the scene stores its text under and what a
/// template (v0.62) will read; the hint is a line under the box saying what
/// goes in it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneBlock {
    pub key: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl SceneBlock {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
            hint: None,
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
    /// The palette role the status badge takes — emphasis, never the message:
    /// the word is always there beside it. Absent draws the badge in outline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<MarkColour>,
}

impl Status {
    pub fn new(key: &str, label: &str, derive: Derive) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
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
    /// How a body in this role reads: `plain` (a monospace column, exactly as
    /// typed — lyrics, a style prompt) or `markdown` (headings, quotes and
    /// tables drawn — a review, a chapter). The craft says, because the code
    /// cannot tell a lyric sheet from an essay by looking at it (ADR 0001).
    /// Absent reads as plain, which is what every role was before the field
    /// existed; a value outside the two reads as plain too, so a typo costs
    /// headings rather than a screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// The two ways a body is read. See [`VersionRole::body`].
pub const BODY_KINDS: [&str; 2] = ["plain", "markdown"];

impl VersionRole {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
            comments_on: None,
            body: None,
        }
    }

    /// Whether bodies in this role are drawn as markdown.
    pub fn reads_as_markdown(&self) -> bool {
        self.body.as_deref() == Some("markdown")
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
    /// A glyph beside the word, from the short list the screen knows —
    /// `wrench`, `circle-help`, `thumbs-up`… (see the profile reference). A
    /// name the screen does not know draws the default glyph rather than
    /// nothing, and the word is always there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
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
    Info,
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
    /// The kind a key names, if the profile has it.
    pub fn kind(&self, key: &str) -> Option<&WorkKind> {
        self.work_kinds.iter().find(|kind| kind.key == key)
    }

    /// The vocabulary of a kind: its own, or — for a kind the profile does
    /// not know — an empty one, so nothing downstream has to ask twice.
    pub fn vocabulary(&self, kind: &str) -> &WorkKind {
        self.kind(kind).unwrap_or(&NO_KIND)
    }

    /// Combine axis values into a 0–100 total, as `kind` weighs them.
    pub fn total(&self, kind: &str, values: &serde_json::Map<String, serde_json::Value>) -> f64 {
        self.vocabulary(kind).total(values)
    }

    /// The highest tier of `kind` whose threshold the total reaches.
    pub fn tier_for(&self, kind: &str, total: f64) -> Option<&Tier> {
        self.vocabulary(kind).tier_for(total)
    }

    /// What each release kind of `kind` makes of the same answers.
    pub fn verdicts(
        &self,
        kind: &str,
        values: &serde_json::Map<String, serde_json::Value>,
    ) -> Vec<KindVerdict> {
        self.vocabulary(kind).verdicts(values)
    }

    /// Every status any kind names, once per key, first label wins. For the
    /// screens that have no work in hand — the catalogue's filter, the batch
    /// that moves many works — and for nothing that judges a single work.
    pub fn all_statuses(&self) -> Vec<Status> {
        union(
            self.work_kinds.iter().flat_map(|kind| kind.statuses.iter()),
            |s| &s.key,
        )
    }

    /// Every tier any kind names, once per key.
    pub fn all_tiers(&self) -> Vec<Tier> {
        union(
            self.work_kinds.iter().flat_map(|kind| kind.tiers.iter()),
            |t| &t.key,
        )
    }

    /// Every axis any kind names, once per key.
    pub fn all_axes(&self) -> Vec<Axis> {
        union(
            self.work_kinds.iter().flat_map(|kind| kind.axes.iter()),
            |a| &a.key,
        )
    }

    /// Every version role any kind names, once per key.
    pub fn all_version_roles(&self) -> Vec<VersionRole> {
        union(
            self.work_kinds
                .iter()
                .flat_map(|kind| kind.version_roles.iter()),
            |r| &r.key,
        )
    }

    /// Every kind of release any kind names, once per key.
    pub fn all_release_kinds(&self) -> Vec<ReleaseKind> {
        union(
            self.work_kinds
                .iter()
                .flat_map(|kind| kind.release_kinds.iter()),
            |k| &k.key,
        )
    }

    /// Everything wrong with the document, in the words a person can act on.
    ///
    /// Empty means the profile is sound. Each line names the place — "work
    /// kind `song`: axis 3 (`hook`)" — and the rule it breaks, because a
    /// profile is edited by hand and "invalid config" sends the person back
    /// to guess. Only what would make the app misbehave is refused: an empty
    /// label is a taste, a duplicate key is a corruption waiting for the next
    /// score.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();

        unique(
            &mut problems,
            "work kind",
            self.work_kinds.iter().map(|k| k.key.clone()),
        );
        unique(
            &mut problems,
            "collection kind",
            self.collection_kinds.iter().map(|k| k.key.clone()),
        );
        unique(
            &mut problems,
            "meta field",
            self.work_meta_fields.iter().map(|f| f.key.clone()),
        );
        if self.work_kinds.is_empty() {
            problems.push("the profile names no work kinds".into());
        }

        for (index, kind) in self.work_kinds.iter().enumerate() {
            let place = format!("work kind {} (`{}`)", index + 1, kind.key);
            kind.validate_into(&mut problems, &place);
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

        self.validate_prompts(&mut problems);

        problems
    }

    /// The actions, checked against the vocabulary they read: a placeholder
    /// no kind of the action fills, a role the answer cannot be kept in, a
    /// storyboard a kind does not have. Refused at save rather than found
    /// as a hole in a prompt — the predecessor let an edited template lose
    /// its text placeholder and sent critiques of nothing for a month.
    fn validate_prompts(&self, problems: &mut Vec<String>) {
        use crate::assistant::prompt::{Produces, SCENE_SCOPE, Scope, is_known_placeholder};

        unique(
            problems,
            "action",
            self.prompts.iter().map(|prompt| prompt.key.clone()),
        );

        for (index, prompt) in self.prompts.iter().enumerate() {
            let place = format!("action {} (`{}`)", index + 1, prompt.key);
            if prompt.template.trim().is_empty() {
                problems.push(format!("{place} has no message"));
            }
            for kind in &prompt.kinds {
                if self.kind(kind).is_none() {
                    problems.push(format!(
                        "{place} names a work kind `{kind}` the profile does not have"
                    ));
                }
            }
            if let Some(scope) = prompt.scope.as_deref().map(str::trim) {
                if scope != SCENE_SCOPE && scope != "work" {
                    problems.push(format!(
                        "{place}: `scope` is `work` or `scene`, not `{scope}`"
                    ));
                }
            }
            if !prompt.produces_is_known() {
                problems.push(format!(
                    "{place}: `produces` is `score`, `version:<role>`, `scenes`, `scenes:add` or `scenes:revise`, not `{}`",
                    prompt.produces.as_deref().unwrap_or_default().trim()
                ));
            }

            // The kinds the action is offered on: the ones it names, or all.
            let kinds: Vec<&WorkKind> = if prompt.kinds.is_empty() {
                self.work_kinds.iter().collect()
            } else {
                prompt.kinds.iter().filter_map(|k| self.kind(k)).collect()
            };
            let lacking = |has: &dyn Fn(&WorkKind) -> bool| -> Vec<String> {
                kinds
                    .iter()
                    .filter(|kind| !has(kind))
                    .map(|kind| format!("`{}`", kind.key))
                    .collect()
            };
            fn has_role<'a>(role: &'a str) -> impl Fn(&WorkKind) -> bool + 'a {
                move |kind: &WorkKind| kind.version_roles.iter().any(|r| r.key == role)
            }
            let has_board =
                |kind: &WorkKind| !kind.shot_types.is_empty() || !kind.scene_blocks.is_empty();

            let placeholders = prompt.placeholders();
            for name in &placeholders {
                if !is_known_placeholder(name) {
                    problems.push(format!("{place} reads `{{{name}}}`, which nothing fills"));
                    continue;
                }
                if let Some(role) = name.strip_prefix("role:") {
                    let missing = lacking(&has_role(role));
                    if !missing.is_empty() {
                        problems.push(format!(
                            "{place} reads `{{{name}}}`, but {} {} no `{role}` role",
                            missing.join(", "),
                            if missing.len() == 1 { "has" } else { "have" }
                        ));
                    }
                }
                if name == "scenes" || name == "scene" {
                    let missing = lacking(&has_board);
                    if !missing.is_empty() {
                        problems.push(format!(
                            "{place} reads `{{{name}}}`, but {} {} no storyboard",
                            missing.join(", "),
                            if missing.len() == 1 { "has" } else { "have" }
                        ));
                    }
                }
            }
            let reads_scene = placeholders.iter().any(|name| name == "scene");
            match prompt.scope() {
                Scope::Scene if !reads_scene => {
                    problems.push(format!(
                        "{place} is about a scene but never reads `{{scene}}`"
                    ));
                }
                Scope::Work if reads_scene => {
                    problems.push(format!(
                        "{place} reads `{{scene}}` but is not about a scene: give it `\"scope\": \"scene\"`"
                    ));
                }
                _ => {}
            }

            match prompt.produces() {
                Produces::Version(role) => {
                    let missing = lacking(&has_role(&role));
                    if !missing.is_empty() {
                        problems.push(format!(
                            "{place} produces `version:{role}`, but {} {} no `{role}` role",
                            missing.join(", "),
                            if missing.len() == 1 { "has" } else { "have" }
                        ));
                    }
                }
                Produces::Scenes(change) => {
                    let missing = lacking(&has_board);
                    if !missing.is_empty() {
                        problems.push(format!(
                            "{place} produces scenes, but {} {} no storyboard",
                            missing.join(", "),
                            if missing.len() == 1 { "has" } else { "have" }
                        ));
                    }
                    if prompt.scope() == Scope::Scene
                        && change != crate::assistant::proposal::BoardChange::Revise
                    {
                        problems.push(format!(
                            "{place} is about a scene and must produce `scenes:revise`, not the whole board"
                        ));
                    }
                }
                Produces::Score | Produces::Prose => {}
            }
        }
    }
}

/// Entries once per key, first seen first: the union of a vocabulary across
/// kinds, for the screens that speak to every kind at once.
fn union<'a, T: Clone + 'a>(
    entries: impl Iterator<Item = &'a T>,
    key: impl Fn(&T) -> &String,
) -> Vec<T> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for entry in entries {
        if seen.insert(key(entry).clone()) {
            out.push(entry.clone());
        }
    }
    out
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
        assert!(config().work_kinds[0].axes[0].rubric.is_empty());
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
        config.work_kinds[0].axes[0].rubric = vec![AxisMark {
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
        config.work_kinds[0].axes[0].rubric = vec![
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
        config.work_kinds[0].axes[0].rubric = vec![
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
        assert!(config().work_kinds[0].release_kinds[0].requires.is_empty());
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
        let total = config().total("song", values.as_object().unwrap());
        assert!((total - 80.0).abs() < 1e-9, "got {total}");
    }

    #[test]
    fn a_missing_axis_does_not_count_as_zero() {
        let values = json!({ "hook": 8.0 });
        let total = config().total("song", values.as_object().unwrap());
        assert!((total - 80.0).abs() < 1e-9, "got {total}");
    }

    #[test]
    fn an_empty_score_card_totals_zero_rather_than_dividing_by_zero() {
        let values = json!({});
        assert_eq!(config().total("song", values.as_object().unwrap()), 0.0);
    }

    #[test]
    fn tier_picks_the_highest_threshold_reached() {
        let config = config();
        assert_eq!(config.tier_for("song", 80.0).unwrap().key, "clip");
        assert_eq!(config.tier_for("song", 74.9).unwrap().key, "hold");
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
        assert_eq!(config().work_kinds[0].axes[0].kind, AxisKind::Scale);
        assert!(config().work_kinds[0].axes[0].options.is_empty());
    }

    #[test]
    fn a_flag_is_worth_the_scale_or_nothing_and_a_choice_its_option() {
        let config = typed();
        let values = json!({ "hook": 5.0, "chorus": true, "length": "short" });
        // (0.5 + 1.0 + 0.4) / 3
        let total = config.total("song", values.as_object().unwrap());
        assert!((total - 63.333_333).abs() < 1e-3, "got {total}");

        let values = json!({ "hook": 5.0, "chorus": false, "length": "short" });
        let total = config.total("song", values.as_object().unwrap());
        assert!((total - 30.0).abs() < 1e-9, "got {total}");
    }

    #[test]
    fn an_answer_the_axis_cannot_read_is_skipped_like_a_missing_one() {
        let config = typed();
        let values = json!({ "hook": 5.0, "length": "epic" });
        let total = config.total("song", values.as_object().unwrap());
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
        let total = config.total("song", values.as_object().unwrap());
        assert!((total - 100.0).abs() < 1e-9, "got {total}");
    }

    #[test]
    fn a_release_kind_reweighs_the_axes_it_names_and_keeps_the_rest() {
        let config = typed();
        let values = json!({ "hook": 10.0, "chorus": false, "length": "short" });
        let values = values.as_object().unwrap();
        // Plain: (1.0 + 0 + 0.4) / 3
        assert!((config.total("song", values) - 46.666_666).abs() < 1e-3);
        // As a clip: hook ×4, chorus ×0, length keeps 1: (4.0 + 0 + 0.4) / 5
        let clip = config.vocabulary("song").total_for(values, "clip");
        assert!((clip - 88.0).abs() < 1e-9, "got {clip}");
        // A kind that reweights nothing, or none at all, is the plain total.
        assert_eq!(
            config.vocabulary("song").total_for(values, "audio"),
            config.total("song", values)
        );
        assert_eq!(
            config.vocabulary("song").total_for(values, "no-such-kind"),
            config.total("song", values)
        );
    }

    #[test]
    fn every_release_kind_gets_a_verdict_even_when_it_reweighs_nothing() {
        let config = typed();
        let values = json!({ "hook": 10.0, "chorus": false, "length": "short" });
        let verdicts = config.verdicts("song", values.as_object().unwrap());

        // One row per kind: a kind missing from the comparison would read as
        // "not applicable" rather than "weighs them the same way".
        assert_eq!(verdicts.len(), config.work_kinds[0].release_kinds.len());

        let clip = verdicts.iter().find(|v| v.kind == "clip").unwrap();
        let audio = verdicts.iter().find(|v| v.kind == "audio").unwrap();

        assert!(clip.reweighed, "clip names its own weights");
        assert!(!audio.reweighed, "audio names none");

        // The verdicts are the per-kind totals, not the flat one repeated.
        assert!((clip.total - 88.0).abs() < 1e-9, "got {}", clip.total);
        assert_eq!(
            audio.total,
            config.total("song", values.as_object().unwrap())
        );
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
        let verdicts = config.verdicts("song", values.as_object().unwrap());

        for verdict in &verdicts {
            // The claim is checkable: each tier is the one the profile itself
            // returns for that verdict's own total, not for the flat total.
            let expected = config
                .tier_for("song", verdict.total)
                .map(|t| t.key.clone());
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
        let first_axis = config.work_kinds[0].axes[0].clone();
        config.work_kinds[0].axes.push(first_axis);
        config.work_kinds[0].axes[1].scale = 0.0;
        config.work_kinds[0].axes[2].options.clear();
        config.work_kinds[0].tiers.push(Tier {
            key: "top".into(),
            label: "Top".into(),
            min: 140.0,
        });
        config.work_kinds[0].release_kinds[0]
            .requires
            .push("melody".into());
        config.work_kinds[0].release_kinds[0]
            .axis_weights
            .insert("ghost".into(), 1.0);
        config.work_kinds[0].version_roles.push(VersionRole {
            key: "review".into(),
            label: "Review".into(),
            comments_on: Some("prose".into()),
            body: None,
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
        config.work_kinds[0].axes[0].options.push(AxisOption {
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

    // ---- format 2: the vocabulary belongs to the kind -----------------------

    fn flat_document() -> serde_json::Value {
        json!({
            "work_kinds": [{ "key": "song", "label": "Song" }, { "key": "instrumental", "label": "Instrumental" }],
            "release_kinds": [{ "key": "clip", "label": "Clip" }],
            "collection_kinds": [],
            "version_roles": [{ "key": "lyrics", "label": "Lyrics" }],
            "statuses": [{ "key": "draft", "label": "Draft", "derive": "draft" }],
            "axes": [{ "key": "hook", "label": "Hook", "weight": 1.0, "scale": 10.0 }],
            "tiers": [{ "key": "hold", "label": "Hold", "min": 0.0 }],
            "work_meta_fields": []
        })
    }

    #[test]
    fn a_flat_document_hands_its_vocabulary_to_every_kind_that_declares_none() {
        let config: ProfileConfig = serde_json::from_value(flat_document()).unwrap();

        assert_eq!(config.format, FORMAT);
        for kind in &config.work_kinds {
            assert_eq!(kind.axes.len(), 1, "{}", kind.key);
            assert_eq!(kind.tiers.len(), 1, "{}", kind.key);
            assert_eq!(kind.version_roles.len(), 1, "{}", kind.key);
            assert_eq!(kind.release_kinds.len(), 1, "{}", kind.key);
            assert_eq!(kind.statuses.len(), 1, "{}", kind.key);
        }
    }

    #[test]
    fn a_kind_that_declares_anything_keeps_only_what_it_declared() {
        let mut document = flat_document();
        document["work_kinds"].as_array_mut().unwrap().push(json!({
            "key": "video", "label": "Video",
            "statuses": [{ "key": "cut", "label": "Cut" }]
        }));
        let config: ProfileConfig = serde_json::from_value(document).unwrap();

        let video = config.kind("video").unwrap();
        assert!(
            video.axes.is_empty(),
            "a video must not inherit the song's axes"
        );
        assert!(video.tiers.is_empty());
        assert!(video.version_roles.is_empty());
        assert_eq!(video.statuses[0].key, "cut");
        assert_eq!(config.kind("song").unwrap().axes.len(), 1);
    }

    #[test]
    fn the_written_document_has_no_flat_vocabulary_left() {
        let config: ProfileConfig = serde_json::from_value(flat_document()).unwrap();
        let written = serde_json::to_value(&config).unwrap();

        assert_eq!(written["format"], FORMAT);
        for flat in [
            "axes",
            "tiers",
            "version_roles",
            "release_kinds",
            "statuses",
        ] {
            assert!(
                written.get(flat).is_none(),
                "`{flat}` must live under the kinds now"
            );
        }
        assert_eq!(written["work_kinds"][0]["axes"][0]["key"], "hook");

        // And it reads back as the same thing: format 2 in, format 2 out.
        let again: ProfileConfig = serde_json::from_value(written).unwrap();
        assert_eq!(again.kind("song").unwrap().axes.len(), 1);
        assert_eq!(again.format, FORMAT);
    }

    #[test]
    fn an_unknown_kind_reads_as_an_empty_vocabulary_rather_than_a_panic() {
        let config: ProfileConfig = serde_json::from_value(flat_document()).unwrap();
        let values = json!({ "hook": 9.0 });
        let values = values.as_object().unwrap();

        assert!(config.kind("poem").is_none());
        assert!(config.vocabulary("poem").axes.is_empty());
        assert_eq!(config.total("poem", values), 0.0);
        assert!(config.tier_for("poem", 50.0).is_none());
        assert!(config.verdicts("poem", values).is_empty());
        assert!(config.vocabulary("poem").starting_status().is_none());
    }

    #[test]
    fn the_unions_name_each_key_once_with_the_first_label() {
        let mut document = flat_document();
        document["work_kinds"].as_array_mut().unwrap().push(json!({
            "key": "video", "label": "Video",
            "statuses": [{ "key": "draft", "label": "Rough cut" }, { "key": "posted", "label": "Posted" }],
            "tiers": [{ "key": "post", "label": "Post", "min": 70.0 }]
        }));
        let config: ProfileConfig = serde_json::from_value(document).unwrap();

        let statuses = config.all_statuses();
        assert_eq!(
            statuses.iter().map(|s| s.key.as_str()).collect::<Vec<_>>(),
            vec!["draft", "posted"]
        );
        assert_eq!(statuses[0].label, "Draft", "the first kind's label wins");
        assert_eq!(
            config
                .all_tiers()
                .iter()
                .map(|t| t.key.as_str())
                .collect::<Vec<_>>(),
            vec!["hold", "post"]
        );
    }

    #[test]
    fn a_kind_without_statuses_is_a_problem_named_by_its_place() {
        let mut document = flat_document();
        document["work_kinds"].as_array_mut().unwrap().push(json!({
            "key": "video", "label": "Video",
            "axes": [{ "key": "cut", "label": "Cut", "weight": 1.0, "scale": 10.0 }]
        }));
        let config: ProfileConfig = serde_json::from_value(document).unwrap();
        let problems = config.validate();
        assert!(
            problems
                .iter()
                .any(|p| p.contains("work kind 3 (`video`) names no statuses")),
            "{problems:?}"
        );
    }

    #[test]
    fn without_judgement_keeps_the_doors_and_drops_the_axes() {
        let config: ProfileConfig = serde_json::from_value(flat_document()).unwrap();
        let bare = config.kind("song").unwrap().without_judgement();
        assert!(bare.axes.is_empty() && bare.tiers.is_empty());
        assert_eq!(bare.statuses.len(), 1);
        assert_eq!(bare.version_roles.len(), 1);
        assert_eq!(bare.release_kinds.len(), 1);
    }

    fn studio() -> ProfileConfig {
        let conn = crate::db::open_in_memory().unwrap();
        crate::profile::seed(&conn).unwrap();
        crate::profile::active(&conn).unwrap().unwrap().config
    }

    fn action(template: &str) -> crate::assistant::prompt::PromptTemplate {
        crate::assistant::prompt::PromptTemplate {
            key: "x".into(),
            label: "X".into(),
            template: template.into(),
            description: None,
            produces: None,
            method: None,
            kinds: Vec::new(),
            scope: None,
        }
    }

    #[test]
    fn an_action_with_a_hole_in_it_is_refused_at_save() {
        let mut config = studio();
        config.prompts = vec![action("Check {typo} of {title}")];
        let problems = config.validate();
        assert!(
            problems
                .iter()
                .any(|p| p.contains("reads `{typo}`, which nothing fills")),
            "{problems:?}"
        );

        // A role not every kind of the action has: the shipped Studio
        // kinds without lyrics are named, and naming the kinds fixes it.
        let mut config = studio();
        config.prompts = vec![action("{role:lyrics}")];
        let problems = config.validate();
        assert!(
            problems
                .iter()
                .any(|p| p
                    .contains("reads `{role:lyrics}`, but `video`, `short` have no `lyrics` role")),
            "{problems:?}"
        );
        config.prompts[0].kinds = vec!["song".into()];
        assert!(config.validate().is_empty(), "{:?}", config.validate());

        let mut config = studio();
        let mut a = action("{role:plot}");
        a.kinds = vec!["video".into()];
        a.produces = Some("version:critique".into());
        config.prompts = vec![a];
        let problems = config.validate();
        assert!(
            problems
                .iter()
                .any(|p| p
                    .contains("produces `version:critique`, but `video` has no `critique` role")),
            "{problems:?}"
        );

        let mut config = studio();
        let mut a = action("{scenes}");
        a.kinds = vec!["song".into()];
        config.prompts = vec![a];
        assert!(
            config
                .validate()
                .iter()
                .any(|p| p.contains("reads `{scenes}`, but `song` has no storyboard")),
            "{:?}",
            config.validate()
        );
    }

    #[test]
    fn a_scene_action_is_held_to_its_shape() {
        let mut config = studio();
        let mut a = action("{scene}");
        a.kinds = vec!["video".into()];
        config.prompts = vec![a.clone()];
        assert!(
            config
                .validate()
                .iter()
                .any(|p| p.contains("give it `\"scope\": \"scene\"`")),
            "{:?}",
            config.validate()
        );

        a.scope = Some("scene".into());
        a.produces = Some("scenes".into());
        config.prompts = vec![a.clone()];
        assert!(
            config
                .validate()
                .iter()
                .any(|p| p.contains("must produce `scenes:revise`")),
            "{:?}",
            config.validate()
        );

        a.produces = Some("scenes:revise".into());
        config.prompts = vec![a];
        assert!(config.validate().is_empty(), "{:?}", config.validate());

        let mut config = studio();
        let mut a = action("{title}");
        a.scope = Some("scene".into());
        a.produces = Some("something-later".into());
        a.kinds = vec!["poem".into()];
        config.prompts = vec![a];
        let problems = config.validate();
        assert!(
            problems.iter().any(|p| p.contains("never reads `{scene}`")),
            "{problems:?}"
        );
        assert!(
            problems.iter().any(|p| p.contains("`produces` is `score`")),
            "{problems:?}"
        );
        assert!(
            problems
                .iter()
                .any(|p| p.contains("work kind `poem` the profile does not have")),
            "{problems:?}"
        );
    }
}
