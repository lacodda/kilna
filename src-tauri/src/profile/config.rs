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
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
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
    /// The stops along the way from an idea to a finished work. A craft names
    /// its own - a novel's "second draft" is not a song's "mixed" - and an
    /// empty list means the craft has nothing to say, in which case
    /// [`stages`] answers with the line's own stops rather than nothing.
    #[serde(default)]
    pub stages: Vec<Stage>,
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
    ///
    /// A kind that names `sections` is a kind of card of the canon (v0.84,
    /// ADR 0043): a note of it is a card whose knowledge is facts, and it
    /// lives on the Canon screen rather than among the notes.
    #[serde(default)]
    pub note_kinds: Vec<NoteKind>,
    /// The kinds a relation between two cards of the canon can be: family, a
    /// neighbour, a pet, a partner. The craft's words, like the kinds of note;
    /// the words each side uses for the other are the relation's own. Added
    /// in v0.84 - a document without it is the same document.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relation_kinds: Vec<Kind>,
    /// The types a style brick can be — an image style, a character, an
    /// environment, a camera angle. The workspace's one dictionary of the
    /// parts a picture prompt is built from (ADR 0031).
    ///
    /// On the profile rather than on a kind of work, because a brick is not
    /// judged or shipped: the same character stands in the videos and in the
    /// shorts. A craft that names none has no style dictionary, and the screen
    /// that keeps one does not appear. Added in v0.75 — a document without it
    /// is the same document.
    #[serde(default)]
    pub style_types: Vec<StyleType>,
    /// How a work's overview is laid out, and where each widget stands on it.
    /// Absent means the owner's choice - the lead layout with the placement
    /// the window ships - which is why nothing here spells a default out: a
    /// profile that never chose keeps following the shipped board as it
    /// changes. On the profile for the catalogue columns' reason: a craft
    /// looks at its works through its own board. Added in v0.82 - a document
    /// without it is the same document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overview: Option<OverviewConfig>,
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
    pub stages: Vec<Stage>,
    #[serde(default)]
    pub prompts: Vec<crate::assistant::prompt::PromptTemplate>,
    #[serde(default)]
    pub rhythm: Option<Rhythm>,
    #[serde(default)]
    pub catalogue_columns: Option<Vec<String>>,
    #[serde(default)]
    pub catalogue_columns_by_kind: Option<BTreeMap<String, Vec<String>>>,
    #[serde(default)]
    pub note_kinds: Vec<NoteKind>,
    #[serde(default)]
    pub relation_kinds: Vec<Kind>,
    #[serde(default)]
    pub style_types: Vec<StyleType>,
    #[serde(default)]
    pub overview: Option<OverviewConfig>,
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
            stages: raw.stages,
            prompts: raw.prompts,
            rhythm: raw.rhythm,
            catalogue_columns: raw.catalogue_columns,
            catalogue_columns_by_kind: raw.catalogue_columns_by_kind,
            note_kinds: raw.note_kinds,
            relation_kinds: raw.relation_kinds,
            style_types: raw.style_types,
            overview: raw.overview,
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
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct WorkKind {
    pub key: String,
    pub label: Label,
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
    /// The parts of the prompt a work's cover picture is drawn from — what to
    /// draw, what to keep out, what words go on it. The same shape as
    /// [`scene_blocks`] and for the same reason: the craft names the parts,
    /// the code does not know them (ADR 0001). A kind that names none (a song
    /// whose cover is the album's) has no cover prompt, and the tab that
    /// edits one does not appear. Added in v0.73 — a document without it is
    /// the same document.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cover_blocks: Vec<SceneBlock>,
    /// Whether a work of this kind plays under one picture for the whole of
    /// its length - a track on a video platform - and so has a frame: the
    /// still, and the loop of what moves in it. Unlike the cover's parts the
    /// frame's are the application's, not the craft's: the loop is written
    /// from its settings (length, a still camera, a seamless join), the way
    /// the cover's layout is (decision of 2026-09-27, ADR 0046). Absent is
    /// no frame. Added in v0.86 - a document without it is the same document.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub frame: bool,
    /// What a work of this kind is called when it is made from another:
    /// `{title}` is the source's title and `{n}` the new work's number among
    /// the works of this kind made from the same source - "{title} · short
    /// {n}". A template without `{n}` numbers only the second and later ones,
    /// so the first clip of a song is "the clip" and the next "the clip 2".
    /// Absent means the source's title as it is. Added in v0.86.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub made_title: Option<Label>,
}

impl WorkKind {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            axes: Vec::new(),
            tiers: Vec::new(),
            version_roles: Vec::new(),
            release_kinds: Vec::new(),
            statuses: Vec::new(),
            shot_types: Vec::new(),
            scene_blocks: Vec::new(),
            cover_blocks: Vec::new(),
            frame: false,
            made_title: None,
        }
    }

    /// Whether a work of this kind goes out itself - names a kind of
    /// release - or only through the works made from it: a song ships as
    /// its clip, its audio and its shorts, never as the song (v0.86).
    pub fn has_doors(&self) -> bool {
        !self.release_kinds.is_empty()
    }

    /// The title a work of this kind takes when it is made from a work
    /// called `source`, in `locale`, as the `number`-th of its kind made from
    /// that source. See [`made_title`](Self::made_title).
    pub fn title_made_from(&self, source: &str, locale: &str, number: usize) -> String {
        let Some(template) = &self.made_title else {
            return source.to_owned();
        };
        let template = template.in_locale(locale);
        let mut title = template.replace("{title}", source);
        if template.contains("{n}") {
            title = title.replace("{n}", &number.to_string());
        } else if number > 1 {
            title = format!("{title} {number}");
        }
        title.trim().to_owned()
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

        // The title of a work made from another is written from the source's
        // title and a number, and from nothing else: a template that lost
        // `{title}` names every clip of every song the same.
        if let Some(template) = &self.made_title {
            for word in template.words() {
                if !word.contains("{title}") {
                    problems.push(format!(
                        "{place}: the title of a work made from another, `{word}`, never reads `{{title}}`"
                    ));
                }
                for name in crate::assistant::prompt::placeholders(word) {
                    if name != "title" && name != "n" {
                        problems.push(format!(
                            "{place}: the title of a work made from another reads `{{{name}}}`; it reads `{{title}}` and `{{n}}`"
                        ));
                    }
                }
            }
        }

        for (index, kind) in self.release_kinds.iter().enumerate() {
            if let Some(format) = &kind.cover_format
                && !is_cover_format(format)
            {
                problems.push(format!(
                    "{place}: release kind {} (`{}`) gives its cover the shape `{format}`; a shape is width to height, `16:9`",
                    index + 1,
                    kind.key
                ));
            }
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

            // What a release of this kind says about itself. Checked here
            // rather than beside the actions, because a release field is read
            // against ONE kind of work - the kind that owns this release kind
            // - while an action may be offered on many. That makes the check
            // sharper than the one an action gets: `{role:lyrics}` in a
            // video's release title is wrong here even though some other kind
            // has lyrics.
            unique(
                problems,
                &format!("{place}: field of release kind `{}`", kind.key),
                kind.fields.iter().map(|field| field.key.clone()),
            );
            for (number, field) in kind.fields.iter().enumerate() {
                let at = format!(
                    "{place}: release kind {} (`{}`), field {} (`{}`)",
                    index + 1,
                    kind.key,
                    number + 1,
                    field.key
                );
                if field.label.is_blank() {
                    problems.push(format!("{at} has no label"));
                }
                if field.limit == Some(0) {
                    problems.push(format!(
                        "{at} is limited to no characters at all; leave the limit out instead"
                    ));
                }
                let Some(template) = field.template() else {
                    continue;
                };
                for name in crate::assistant::prompt::placeholders(template) {
                    if !crate::assistant::prompt::is_known_placeholder(&name) {
                        problems.push(format!("{at} reads `{{{name}}}`, which nothing fills"));
                        continue;
                    }
                    if let Some(role) = name.strip_prefix("role:")
                        && !roles.contains(role)
                    {
                        problems.push(format!(
                            "{at} reads `{{{name}}}`, but this kind has no `{role}` role"
                        ));
                    }
                    if (name == "scenes" || name == "scene")
                        && self.shot_types.is_empty()
                        && self.scene_blocks.is_empty()
                    {
                        problems.push(format!(
                            "{at} reads `{{{name}}}`, but this kind has no storyboard"
                        ));
                    }
                    if crate::assistant::prompt::canon_lens(&name)
                        .is_some_and(|lens| lens != Lens::Public)
                    {
                        problems.push(format!(
                            "{at} reads `{{{name}}}`, but a release goes out in public: it reads `{{canon:public}}`, the canon a public text may see"
                        ));
                    }
                    if name == "selection" {
                        problems.push(format!(
                            "{at} reads `{{selection}}`, which only an action started on selected lines is given"
                        ));
                    }
                    if name == "scene" {
                        problems.push(format!(
                            "{at} reads `{{scene}}`, which is one row of a board; a release is about the whole work, so it reads `{{scenes}}`"
                        ));
                    }
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

/// The names [`OverviewConfig::layout`] may take: an even grid, a lead
/// column with a rail beside it, full-width bands, a mosaic, a sheet of rows.
pub const OVERVIEW_LAYOUTS: [&str; 5] = ["grid", "lead", "bands", "mosaic", "sheet"];

/// The sizes a widget may take: one cell, two across, two by two.
pub const WIDGET_SIZES: [&str; 3] = ["s", "m", "l"];

/// A work's overview: which of the layouts it is drawn in, and the widgets on it.
///
/// The layout and the sizes are words rather than enums on this side, and
/// checked by [`ProfileConfig::validate`] instead of by the parser. The two
/// ends of the document are held to different rules: a save that names a
/// layout nobody draws is a mistake to refuse with its name, while a stored
/// document written by a later build - a sixth layout, a new size - must
/// still read, or the whole profile would fail to load over one word the
/// window can fall back from. Nothing in the backend draws a board, so a word
/// costs it nothing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
pub struct OverviewConfig {
    /// One of [`OVERVIEW_LAYOUTS`].
    pub layout: String,
    /// The widgets, each with its size and place. Empty means the placement
    /// the window ships, so choosing a layout never has to invent one.
    #[serde(default)]
    pub widgets: Vec<WidgetPlacement>,
}

/// Where one widget stands on the overview, and how large it is.
///
/// The id is the window's widget catalogue's - `score`, `text`, `releases`
/// and the rest - and it is not checked against a list here: a widget this
/// build does not know, written by a later one, is kept on save and simply
/// not drawn, the way an unknown catalogue column is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
pub struct WidgetPlacement {
    pub id: String,
    /// One of [`WIDGET_SIZES`].
    pub size: String,
    /// The widget's place in the layout's reading order, first at 0.
    pub position: u32,
}

impl OverviewConfig {
    /// What is wrong with the overview as written, in the words of
    /// [`ProfileConfig::validate`].
    fn validate_into(&self, problems: &mut Vec<String>) {
        if !OVERVIEW_LAYOUTS.contains(&self.layout.as_str()) {
            problems.push(format!(
                "the overview's layout is {}, not `{}`",
                one_of(&OVERVIEW_LAYOUTS),
                self.layout
            ));
        }
        // A widget named twice would be drawn twice, or once at a place
        // nobody chose - the corruption `unique` exists to catch.
        unique(
            problems,
            "overview widget",
            self.widgets.iter().map(|widget| widget.id.clone()),
        );
        for (index, widget) in self.widgets.iter().enumerate() {
            if !WIDGET_SIZES.contains(&widget.size.as_str()) {
                problems.push(format!(
                    "overview widget {} (`{}`): the size is {}, not `{}`",
                    index + 1,
                    widget.id,
                    one_of(&WIDGET_SIZES),
                    widget.size
                ));
            }
        }
    }
}

/// The words a field may take, the way a person would list them:
/// "`s`, `m` or `l`".
fn one_of(words: &[&str]) -> String {
    let quoted: Vec<String> = words.iter().map(|word| format!("`{word}`")).collect();
    match quoted.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        _ => quoted.concat(),
    }
}

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
}

/// A word of the craft's vocabulary, in the languages the profile carries it in.
///
/// A profile's words are the author's data, not the interface's strings: they
/// are edited in Settings and they survive an upgrade untouched. But a
/// *shipped* profile is written by us, and writing it in English only meant a
/// Russian window read "Scored · Song" — the interface translated around a
/// hole in its own vocabulary.
///
/// So a label is either one string, as every profile written before this said
/// it, or a map from locale to string. Both shapes parse and both round-trip:
/// a word the author retyped comes back as the plain string they typed, and is
/// never silently promoted to a map on their behalf.
///
/// Rust resolves to English and only English. The window picks the language —
/// it is the only side that knows which one is showing — while this side uses
/// labels for prompts and exports, where English is what we want anyway: a
/// system prompt is written against a model's English, not the author's
/// window, and translating one changes what the model does.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[serde(untagged)]
pub enum Label {
    /// One word, in whatever language it was typed in.
    One(String),
    /// The word per locale, as a shipped profile carries it.
    PerLocale(BTreeMap<String, String>),
}

/// The locale a shipped profile is authored in, and what Rust reads a label as.
pub const SOURCE_LOCALE: &str = "en";

impl Label {
    /// The word in the source language: what a prompt and an export say.
    ///
    /// A map that somehow lacks English falls back to any word it does hold
    /// rather than to emptiness — a label is a name, and a nameless entry in
    /// a prompt is worse than one named in the wrong language.
    pub fn as_str(&self) -> &str {
        match self {
            Self::One(word) => word,
            Self::PerLocale(words) => words
                .get(SOURCE_LOCALE)
                .or_else(|| words.values().next())
                .map_or("", String::as_str),
        }
    }

    /// The word in `locale`, or in the source language when the label does
    /// not carry that one. For the few words the application writes into a
    /// person's own data - the title of a work made from another - where the
    /// window says which language it is showing.
    pub fn in_locale(&self, locale: &str) -> &str {
        match self {
            Self::One(word) => word,
            Self::PerLocale(words) => words
                .get(locale)
                .map_or_else(|| self.as_str(), String::as_str),
        }
    }

    /// Every word the label carries, whatever the language.
    pub fn words(&self) -> Vec<&str> {
        match self {
            Self::One(word) => vec![word.as_str()],
            Self::PerLocale(words) => words.values().map(String::as_str).collect(),
        }
    }

    /// Whether the label says nothing at all — what validation rejects.
    pub fn is_blank(&self) -> bool {
        match self {
            Self::One(word) => word.trim().is_empty(),
            Self::PerLocale(words) => {
                words.is_empty() || words.values().all(|word| word.trim().is_empty())
            }
        }
    }
}

/// Comparison against a plain string, so a test and a lookup can both ask
/// "is this the word?" without unwrapping the shape first. Compares the
/// source-language word, which is the one Rust works in.
impl PartialEq<str> for Label {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Label {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for Label {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl std::fmt::Display for Label {
    /// Through `f.pad`, so `{:<12}` lines a column up rather than being
    /// silently ignored the way `write_str` would.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(self.as_str())
    }
}

impl From<&str> for Label {
    fn from(word: &str) -> Self {
        Self::One(word.to_owned())
    }
}

impl From<String> for Label {
    fn from(word: String) -> Self {
        Self::One(word)
    }
}

/// A vocabulary entry: a stable key with a label the user may rename.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Kind {
    pub key: String,
    pub label: Label,
}

impl Kind {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
        }
    }
}

/// A kind a note can take, and - when it names sections - a kind of card of
/// the canon (ADR 0043).
///
/// A plain kind is what every kind was before v0.84: a key and a word. A kind
/// with sections is a character, a place, the channel: its notes are cards
/// whose knowledge is facts filed under those sections, read by the Canon
/// screen, by the assistant and by the tasks that draw on the world. The body
/// of such a note stays - as the card's free note.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct NoteKind {
    pub key: String,
    pub label: Label,
    /// Name of the glyph the kind is drawn with, from the fixed set the
    /// window knows. Carried, never read here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// What a card of this kind knows, section by section, in the order the
    /// card reads. Empty: a plain note, not a card.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<CanonSection>,
    /// One card of this kind per workspace - the channel, the root of the
    /// world everything else hangs from. A second is refused.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub root: bool,
    /// The sections the generator's description of the card is written
    /// from: a person's looks, not their biography. The description says it
    /// is stale when a fact of these sections changes. Empty: the card is
    /// described by hand and never goes stale.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub describe_from: Vec<String>,
    /// Notes of this kind are what works are made from, and are spent by
    /// them: an idea, a phrase. Such a note carries a state - fresh, used,
    /// parked, dropped - and going to a work leaves it in the bank, used
    /// (ADR 0045).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub material: bool,
    /// A note of this kind is one line: kept as a row of its own table, not
    /// as a page among the notes (ADR 0045).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub line: bool,
}

impl NoteKind {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            icon: None,
            sections: Vec::new(),
            root: false,
            describe_from: Vec::new(),
            material: false,
            line: false,
        }
    }

    /// Whether a note of this kind is a card of the canon.
    pub fn is_card(&self) -> bool {
        !self.sections.is_empty()
    }

    pub fn section(&self, key: &str) -> Option<&CanonSection> {
        self.sections.iter().find(|section| section.key == key)
    }

    /// Everything wrong with this kind's sections, each line naming its place.
    fn validate_into(&self, problems: &mut Vec<String>, place: &str, kinds: &[NoteKind]) {
        unique(
            problems,
            &format!("{place}: section"),
            self.sections.iter().map(|s| s.key.clone()),
        );
        if self.root && !self.is_card() {
            problems.push(format!(
                "{place} is the root of the canon but names no sections"
            ));
        }
        // A card is known by its facts and lives on the canon: it is neither
        // spent by a work nor a single line.
        if self.is_card() && (self.material || self.line) {
            problems.push(format!(
                "{place} names sections, and a card of the canon is neither material nor a line"
            ));
        }
        for key in &self.describe_from {
            match self.section(key) {
                None => problems.push(format!(
                    "{place} is described from a section `{key}` it does not have"
                )),
                Some(section) if !section.shape.holds_words() => problems.push(format!(
                    "{place} is described from `{key}`, a section that holds no statements"
                )),
                Some(_) => {}
            }
        }
        let mut gathers_the_rest = 0;
        for section in &self.sections {
            let at = format!("{place} section `{}`", section.key);
            if section.shape != SectionShape::Relations && !section.kinds.is_empty() {
                problems.push(format!(
                    "{at} names kinds of card, which only a section of relations gathers by"
                ));
            }
            if section.shape == SectionShape::Relations {
                if section.kinds.is_empty() {
                    gathers_the_rest += 1;
                }
                for kind in &section.kinds {
                    if !kinds.iter().any(|one| &one.key == kind && one.is_card()) {
                        problems.push(format!(
                            "{at} gathers relations to `{kind}`, which is not a kind of card"
                        ));
                    }
                }
            }
        }
        if gathers_the_rest > 1 {
            problems.push(format!(
                "{place} has {gathers_the_rest} sections of relations that name no kinds; one gathers the rest"
            ));
        }
    }
}

/// One section of a card: what it is called, what shape its entries take, and
/// which outward tasks may read it.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct CanonSection {
    pub key: String,
    pub label: Label,
    /// A line under the section's name: what goes here, and what reads it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<Label>,
    /// What an entry of the section is. Statements when absent.
    #[serde(default, skip_serializing_if = "SectionShape::is_facts")]
    pub shape: SectionShape,
    /// The outward tasks that read this section - a cover, a public text. A
    /// work itself reads every section, so `work` need not be named; a
    /// section naming none is read by the work alone.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lenses: Vec<Lens>,
    /// For a section of relations: the kinds of card it gathers - the events
    /// a person took part in apart from the people around them. Empty
    /// gathers every relation no other section of the card claims.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<String>,
}

impl CanonSection {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            hint: None,
            shape: SectionShape::Facts,
            lenses: Vec::new(),
            kinds: Vec::new(),
        }
    }

    /// Whether `lens` reads this section.
    pub fn read_by(&self, lens: Lens) -> bool {
        lens == Lens::Work || self.lenses.contains(&lens)
    }
}

/// What an entry of a section is.
///
/// Every shape but the last two is rows of facts; the shape says what a fact
/// carries beside its words and how the card draws it. Relations and
/// appearances are not written into at all: the first is the graph of the
/// card's relations, the second is counted from the works.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum SectionShape {
    /// Short statements.
    #[default]
    Facts,
    /// Named values: a caption's slot and its words, a template's parts.
    Slots,
    /// Signature details: a name, a prompt template, where it acts, and
    /// whether it is on by default.
    Details,
    /// Colours, each with a name.
    Palette,
    /// Variants of a mark: a code, a meaning, a description, the files.
    Marks,
    /// House styles: bricks of the style dictionary.
    Styles,
    /// The card's relations to other cards.
    Relations,
    /// Where the card appears, counted from scenes, texts and sources.
    Appearances,
}

impl SectionShape {
    fn is_facts(&self) -> bool {
        *self == SectionShape::Facts
    }

    /// Whether a fact can be written into a section of this shape.
    pub fn holds_facts(self) -> bool {
        !matches!(self, SectionShape::Relations | SectionShape::Appearances)
    }

    /// Whether its entries are statements a description can be written from.
    pub fn holds_words(self) -> bool {
        matches!(self, SectionShape::Facts | SectionShape::Slots)
    }
}

/// An outward task a card is read for, and what it may see of it.
///
/// Fixed here rather than named by the profile, because each has a reader in
/// the code - the cover constructor, the actions over a work's text, the text
/// a release goes out under - and the rule of the layers is the same for
/// every craft: a cover and a public text see the public layer only, a work
/// sees everything. What a craft decides is which sections a cover and a
/// public text read (`CanonSection::lenses`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum Lens {
    /// A picture: a cover, a frame.
    Cover,
    /// The work itself: a lyric, a chapter, a script.
    Work,
    /// What is said in public: a release's text, a reply to a comment.
    Public,
}

impl Lens {
    pub const ALL: [Lens; 3] = [Lens::Cover, Lens::Work, Lens::Public];

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "cover" => Some(Lens::Cover),
            "work" => Some(Lens::Work),
            "public" => Some(Lens::Public),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Lens::Cover => "cover",
            Lens::Work => "work",
            Lens::Public => "public",
        }
    }
}

/// One prompt block a scene carries: a still frame, an animation, a
/// negative. The key is what the scene stores its text under and what a
/// template (v0.62) will read; the hint is a line under the box saying what
/// goes in it.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct SceneBlock {
    pub key: String,
    pub label: Label,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<Label>,
}

impl SceneBlock {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            hint: None,
        }
    }
}

/// One type of style brick: an image style, a character, an environment.
///
/// The type is what tells the assistant which question a brick answers. Given
/// the same photograph, `image-style` asks for the render technique and
/// `character` asks for the person — so the `hint` is not decoration, it is
/// the difference between one rich dictionary and several flat ones. It reaches
/// the assistant when a brick is described; it never reaches a generator, being
/// an instruction about the description rather than part of it.
///
/// The same shape as [`SceneBlock`], with a glyph: both are a word of the craft
/// that carries a line saying what goes under it. It lives on the profile
/// rather than on a kind of work because a brick is not judged, shipped or
/// storyboarded — one character serves the videos, the shorts and the covers.
/// See ADR 0031.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct StyleType {
    pub key: String,
    pub label: Label,
    /// What to describe for a brick of this type. Absent means the craft has
    /// nothing particular to say, and the assistant is told only the label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<Label>,
    /// Name of the glyph the type is drawn with, from the fixed set the window
    /// knows. Carried, never read here — which picture goes with a word is a
    /// question for the screen, as it is for [`Mark`] and [`ReleaseKind`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl StyleType {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            hint: None,
            icon: None,
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
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct ReleaseKind {
    pub key: String,
    pub label: Label,
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
    /// What has to be written *about* a release of this kind before it can go
    /// out: a title, a description, tags, a comment to pin under it. The
    /// craft's own list, because the fields a video ships with and the fields
    /// a paperback ships with have nothing in common but being text someone
    /// has to write.
    ///
    /// Each field may carry a template, so the writing starts from the work
    /// rather than from an empty box. Empty — the state of every profile
    /// written before the field existed — means a release of this kind says
    /// nothing about itself, and the tab shows no metadata for it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ReleaseField>,
    /// The shape of the picture a release of this kind goes out with, as
    /// width to height: `16:9` for a video platform's preview, `9:16` for a
    /// vertical short, `1:1` for a streaming cover. The place decides the
    /// shape, so the shape is the door's - a work that goes out in two
    /// places needs two covers (v0.86). Absent means the door shows no
    /// picture of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_format: Option<String>,
}

/// Whether `format` is a shape a cover can take: two whole numbers above
/// zero with a colon between them, `16:9`.
pub fn is_cover_format(format: &str) -> bool {
    let Some((width, height)) = format.split_once(':') else {
        return false;
    };
    let whole = |side: &str| {
        !side.is_empty()
            && side.bytes().all(|byte| byte.is_ascii_digit())
            && side.parse::<u32>().is_ok_and(|value| value > 0)
    };
    whole(width) && whole(height)
}

/// One thing written about a release: the box it is typed in, and the
/// template it starts from.
///
/// The key is what the value is stored under in `release.meta`, so renaming
/// the label never loses what was written. The template is in the same
/// language as an AI action's prompt — `{title}`, `{role:lyrics}`, `{scenes}`
/// — because a placeholder that meant one thing in one box and another thing
/// in the next box would be two vocabularies wearing one syntax. See
/// [`crate::assistant::prompt`].
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct ReleaseField {
    pub key: String,
    pub label: Label,
    /// The shape of the box. Absent is a line: the state a field written
    /// before the type existed reads as.
    #[serde(default, rename = "type")]
    pub field_type: ReleaseFieldType,
    /// What the field is filled with when it is generated, in the prompt
    /// language. Absent means the field is only ever typed by hand — which is
    /// the honest answer for a link, and a bad one for a description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// A line under the box saying what goes in it — the platform's limit,
    /// the house style. The same role `hint` plays on a scene block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<Label>,
    /// How many characters the place this is going will accept. Absent means
    /// nobody is counting. Shown as a count beside the box rather than
    /// enforced: kilna is not the authority on what YouTube accepts this
    /// month, and a field it refuses to hold is a field typed somewhere else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// The shape of a release field's box.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseFieldType {
    /// One line: a title.
    #[default]
    Line,
    /// Paragraphs: a description, a comment to pin.
    Text,
    /// A list of words, kept as text with a comma between them — the form
    /// every platform's box takes, and the form that survives a copy.
    Tags,
}

impl ReleaseField {
    pub fn new(key: &str, label: &str, field_type: ReleaseFieldType) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            field_type,
            template: None,
            hint: None,
            limit: None,
        }
    }

    /// The same field, filled from a template.
    pub fn from_template(mut self, template: &str) -> Self {
        self.template = Some(template.to_owned());
        self
    }

    /// The template, when the field states one worth rendering: blank is none.
    pub fn template(&self) -> Option<&str> {
        self.template
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
    }
}

impl ReleaseKind {
    pub fn new(key: &str, label: &str, requires: &[&str]) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            requires: requires.iter().map(|role| (*role).to_owned()).collect(),
            icon: None,
            axis_weights: BTreeMap::new(),
            fields: Vec::new(),
            cover_format: None,
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

/// An independent body a work carries.
///
/// Most roles stand alone — lyrics and style advance separately, and showing
/// them interleaved would suggest otherwise. A role that names `comments_on`
/// does not: it is written *about* another role, and reading it away from what
/// it discusses is reading half of it. That is the whole reason the field
/// exists rather than the code knowing which keys are commentary: the craft
/// says what comments on what, the same way it says everything else (ADR 0001).
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct VersionRole {
    pub key: String,
    pub label: Label,
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
    /// Whether a body in this role is a time the WORK was written, and so
    /// belongs in the count the catalogue shows.
    ///
    /// A critique answers this by commenting on another role, and always did.
    /// A style prompt does not: it stands on its own, is written about the
    /// song rather than being a draft of it, and counting it made a song with
    /// four texts read six. The craft says which of its roles are the work
    /// itself, because the code cannot tell a lyric sheet from a production
    /// note by looking at it — the same reason `body` exists (ADR 0001).
    ///
    /// Absent reads as "yes, unless it comments on something", which is what
    /// every role meant before the field existed: a profile that never sets
    /// it counts exactly what it counted before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counts_as_version: Option<bool>,
}

/// The two ways a body is read. See [`VersionRole::body`].
pub const BODY_KINDS: [&str; 2] = ["plain", "markdown"];

impl VersionRole {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            comments_on: None,
            body: None,
            counts_as_version: None,
        }
    }

    /// Whether bodies in this role are drawn as markdown.
    pub fn reads_as_markdown(&self) -> bool {
        self.body.as_deref() == Some("markdown")
    }

    /// Whether a body in this role counts as a time the work was written.
    ///
    /// The profile's explicit answer wins; without one, anything that
    /// comments on another role is not a draft and everything else is.
    pub fn counts_as_a_version(&self) -> bool {
        self.counts_as_version
            .unwrap_or_else(|| self.comments_on.is_none())
    }
}

/// One stop on the way from an idea to a finished work.
///
/// The third thing a work carries about itself, beside the derived status and
/// the hand-raised marks, and it overlaps neither. A status says where the work
/// stands in the *process* and is worked out from facts — it was scored, it was
/// booked, it shipped. A stage says how finished the *work itself* is, which no
/// fact can answer: only the author knows that a song with a full lyric is
/// still three verses of placeholder.
///
/// `percent` is what is stored on the work, and what the dial draws. The key
/// and the label are how a person speaks about it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
pub struct Stage {
    pub key: String,
    pub label: Label,
    /// Where this stop sits, 0..=100.
    pub percent: i64,
    /// One of the palette's own roles, so a stage reads correctly in both
    /// themes.
    #[serde(default)]
    pub colour: MarkColour,
}

/// The stops a craft gets when it names none of its own.
///
/// A dial with nothing to snap to is not a dial, so this is not an empty list:
/// seven stops are enough to say something useful and few enough to click
/// through without reading. A profile that means something else says so and
/// this is never consulted.
///
/// The last stop is "Final" rather than "Finished", because those are two
/// different claims: a work can be finished and still unreleased, and the
/// owner wanted to see at a glance which ones were out. Both are set by hand,
/// like every other stop - the stage stays a judgement, and the fact of a
/// release is what the status already carries.
pub fn default_stages() -> Vec<Stage> {
    [
        ("idea", "Idea", 0, MarkColour::Plain),
        ("raw", "Rough draft", 17, MarkColour::Plain),
        ("half", "Half there", 33, MarkColour::Warn),
        ("nearly", "Nearly there", 50, MarkColour::Warn),
        ("polish", "Polishing", 67, MarkColour::Warn),
        ("done", "Finished", 83, MarkColour::Accent),
        ("final", "Final", 100, MarkColour::Good),
    ]
    .into_iter()
    .map(|(key, label, percent, colour)| Stage {
        key: key.to_owned(),
        label: Label::from(label),
        percent,
        colour,
    })
    .collect()
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
}

impl ProfileConfig {
    /// The kind a key names, if the profile has it.
    pub fn kind(&self, key: &str) -> Option<&WorkKind> {
        self.work_kinds.iter().find(|kind| kind.key == key)
    }

    /// The kind a key names, or the refusal a window, an agent and a package
    /// all get for a kind the profile does not have.
    ///
    /// The vocabulary is checked here, in the domain, rather than by each
    /// caller: the MCP server, the proposal applier and the commands each
    /// wrote their own check, in their own words, and a caller that forgot
    /// wrote a work of a kind no screen can show.
    pub fn require_kind(&self, key: &str) -> crate::Result<&WorkKind> {
        self.kind(key).ok_or_else(|| {
            crate::Error::refused("work.unknownKind")
                .param("kind", key)
                .param(
                    "known",
                    Self::keys(self.work_kinds.iter().map(|kind| kind.key.as_str())),
                )
        })
    }

    /// A version role of a kind, or the refusal for one it does not have.
    pub fn require_role(&self, kind: &str, role: &str) -> crate::Result<()> {
        let roles = &self.vocabulary(kind).version_roles;
        if roles.iter().any(|known| known.key == role) {
            Ok(())
        } else {
            Err(crate::Error::refused("version.unknownRole")
                .param("role", role)
                .param("kind", self.kind_named(kind))
                .param(
                    "known",
                    Self::keys(roles.iter().map(|known| known.key.as_str())),
                ))
        }
    }

    /// A kind of release a kind of work ships as, or the refusal for one it
    /// does not: a release nobody could plan by hand is not one a package or
    /// an agent may plan either.
    pub fn require_release_kind(&self, kind: &str, release_kind: &str) -> crate::Result<()> {
        let kinds = &self.vocabulary(kind).release_kinds;
        // A kind with no door at all goes out through what is made from it: a
        // song ships as its audio, its clip, its shorts (v0.86). Said as that,
        // rather than as an empty list of doors to choose from.
        if kinds.is_empty() {
            return Err(
                crate::Error::refused("release.noDoors").param("kind", self.kind_named(kind))
            );
        }
        if kinds.iter().any(|known| known.key == release_kind) {
            Ok(())
        } else {
            Err(crate::Error::refused("release.unknownKind")
                .param("releaseKind", release_kind)
                .param("kind", self.kind_named(kind))
                .param(
                    "known",
                    Self::keys(kinds.iter().map(|known| known.key.as_str())),
                ))
        }
    }

    /// A kind that has a storyboard - names kinds of shot or prompt blocks -
    /// or the refusal for one that takes no scenes.
    pub fn require_storyboard(&self, kind: &str) -> crate::Result<()> {
        let vocabulary = self.vocabulary(kind);
        if vocabulary.shot_types.is_empty() && vocabulary.scene_blocks.is_empty() {
            Err(crate::Error::refused("scene.noStoryboard").param("kind", self.kind_named(kind)))
        } else {
            Ok(())
        }
    }

    /// What a refusal calls a kind: its label - a word the window says in
    /// its own language - or its key when the profile does not know it.
    fn kind_named(&self, kind: &str) -> serde_json::Value {
        self.kind(kind)
            .and_then(|known| serde_json::to_value(&known.label).ok())
            .unwrap_or_else(|| serde_json::Value::String(kind.to_owned()))
    }

    /// Keys, listed for a refusal: `song, video, short`.
    fn keys<'a>(keys: impl Iterator<Item = &'a str>) -> String {
        keys.collect::<Vec<_>>().join(", ")
    }

    /// The vocabulary of a kind: its own, or — for a kind the profile does
    /// not know — an empty one, so nothing downstream has to ask twice.
    pub fn vocabulary(&self, kind: &str) -> &WorkKind {
        self.kind(kind).unwrap_or(&NO_KIND)
    }

    /// The overview fields a work of `kind` has, in the profile's order.
    pub fn fields_of(&self, kind: &str) -> Vec<&MetaField> {
        self.work_meta_fields
            .iter()
            .filter(|field| field.applies_to(kind))
            .collect()
    }

    /// The fields a new work of `kind` starts with: every field of the kind
    /// that names a default, at it.
    pub fn defaults_of(&self, kind: &str) -> serde_json::Map<String, serde_json::Value> {
        self.fields_of(kind)
            .into_iter()
            .filter_map(|field| Some((field.key.clone(), field.default.clone()?)))
            .collect()
    }

    /// The type a style brick is of, by key. Absent when the craft does not
    /// name it — a brick written under a type later dropped from the document
    /// still reads, and shows its key (ADR 0031).
    pub fn style_type(&self, key: &str) -> Option<&StyleType> {
        self.style_types.iter().find(|kind| kind.key == key)
    }

    /// The kind a note is of, by key.
    pub fn note_kind(&self, key: &str) -> Option<&NoteKind> {
        self.note_kinds.iter().find(|kind| kind.key == key)
    }

    /// The kind of card a note of `key` is, when it is one: a kind the
    /// profile names with sections (ADR 0043).
    pub fn card_kind(&self, key: &str) -> Option<&NoteKind> {
        self.note_kind(key).filter(|kind| kind.is_card())
    }

    /// The keys of every kind of card, in the order the profile lists them.
    pub fn card_kinds(&self) -> Vec<&str> {
        self.note_kinds
            .iter()
            .filter(|kind| kind.is_card())
            .map(|kind| kind.key.as_str())
            .collect()
    }

    /// The keys of every kind kept as one line (ADR 0045).
    pub fn line_kinds(&self) -> Vec<&str> {
        self.note_kinds
            .iter()
            .filter(|kind| kind.line)
            .map(|kind| kind.key.as_str())
            .collect()
    }

    /// A kind of card, or the refusal that says the kind is not one.
    pub fn require_card_kind(&self, key: &str) -> crate::Result<&NoteKind> {
        self.card_kind(key)
            .ok_or_else(|| crate::Error::refused("canon.unknownKind").param("kind", key))
    }

    /// The stops of the dial: the craft's own, or the line's when it names none.
    ///
    /// Answered here rather than at each caller, so a screen and an exporter
    /// cannot disagree about what an empty list means.
    pub fn stages(&self) -> Vec<Stage> {
        if self.stages.is_empty() {
            default_stages()
        } else {
            self.stages.clone()
        }
    }

    /// The stop a percentage belongs to: the last one it has reached.
    ///
    /// Not the nearest: a work at 79 is still polishing, not "finished", and
    /// rounding up would tell the author their song is done.
    pub fn stage_at(&self, percent: i64) -> Option<Stage> {
        self.stages()
            .into_iter()
            .rfind(|stage| stage.percent <= percent)
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
        unique(
            &mut problems,
            "style type",
            self.style_types.iter().map(|t| t.key.clone()),
        );
        if self.work_kinds.is_empty() {
            problems.push("the profile names no work kinds".into());
        }

        for (index, field) in self.work_meta_fields.iter().enumerate() {
            let place = format!("meta field {} (`{}`)", index + 1, field.key);
            for kind in &field.kinds {
                if self.kind(kind).is_none() {
                    problems.push(format!(
                        "{place} names a work kind `{kind}` the profile does not have"
                    ));
                }
            }
            if field.field_type == MetaFieldType::Choice {
                if field.options.is_empty() {
                    problems.push(format!("{place} is a choice with nothing to choose from"));
                }
                unique(
                    &mut problems,
                    &format!("{place}: option"),
                    field.options.iter().map(|option| option.key.clone()),
                );
                if let Some(default) = &field.default
                    && !default
                        .as_str()
                        .is_some_and(|key| field.options.iter().any(|option| option.key == key))
                {
                    problems.push(format!(
                        "{place} starts at {default}, which is not one of its options"
                    ));
                }
            } else if !field.options.is_empty() {
                problems.push(format!(
                    "{place} lists options but is not a choice; set `type` to `choice` or drop them"
                ));
            }
        }

        for (index, kind) in self.work_kinds.iter().enumerate() {
            let place = format!("work kind {} (`{}`)", index + 1, kind.key);
            kind.validate_into(&mut problems, &place);
        }

        unique(
            &mut problems,
            "note kind",
            self.note_kinds.iter().map(|k| k.key.clone()),
        );
        unique(
            &mut problems,
            "relation kind",
            self.relation_kinds.iter().map(|k| k.key.clone()),
        );
        let roots: Vec<&str> = self
            .note_kinds
            .iter()
            .filter(|kind| kind.root)
            .map(|kind| kind.key.as_str())
            .collect();
        if roots.len() > 1 {
            problems.push(format!(
                "the canon has one root, and {} are named: {}",
                roots.len(),
                roots.join(", ")
            ));
        }
        for (index, kind) in self.note_kinds.iter().enumerate() {
            let place = format!("note kind {} (`{}`)", index + 1, kind.key);
            kind.validate_into(&mut problems, &place, &self.note_kinds);
        }

        if let Some(rhythm) = &self.rhythm {
            if rhythm.every_days == 0 {
                problems.push("the rhythm must be at least one day".into());
            }
            if let Some(time) = &rhythm.default_time
                && !crate::time::is_clock_time(time)
            {
                problems.push(format!(
                    "the rhythm's default time `{time}` is not a time of day (HH:MM)"
                ));
            }
        }

        if let Some(overview) = &self.overview {
            overview.validate_into(&mut problems);
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
        use crate::assistant::prompt::{
            CANON_SCOPE, COMMENT_SCOPE, Produces, RELEASE_SCOPE, SCENE_SCOPE, SELECTION_SCOPE,
            STYLE_SCOPE, Scope, is_known_placeholder,
        };

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
            if let Some(scope) = prompt.scope.as_deref().map(str::trim)
                && scope != SCENE_SCOPE
                && scope != STYLE_SCOPE
                && scope != COMMENT_SCOPE
                && scope != CANON_SCOPE
                && scope != SELECTION_SCOPE
                && scope != RELEASE_SCOPE
                && scope != "work"
            {
                problems.push(format!(
                        "{place}: `scope` is `work`, `scene`, `selection`, `style`, `comment`, `canon` or `release`, not `{scope}`"
                    ));
            }
            if !prompt.produces_is_known() {
                problems.push(format!(
                    "{place}: `produces` is `score`, `version:<role>`, `scenes`, `scenes:add`, `scenes:revise`, `comment`, `reply`, `description`, `canon`, `card-prompt` or `release`, not `{}`",
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
                if name == "styles" && self.style_types.is_empty() {
                    problems.push(format!(
                        "{place} reads `{{styles}}`, but the profile names no style types"
                    ));
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
                // A card is given whole, ahead of the template, and the
                // template goes as written: a placeholder in it would reach
                // the model as braces.
                Scope::Canon if !placeholders.is_empty() => {
                    problems.push(format!(
                        "{place} is about a card, which it is given whole: it reads no placeholders, and `{{{}}}` would be sent as written",
                        placeholders[0]
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
                Produces::Comment | Produces::Reply if prompt.scope() != Scope::Comment => {
                    problems.push(format!(
                        "{place} produces `{}`, which only an action about a comment can: give it `\"scope\": \"comment\"`",
                        prompt.produces.as_deref().unwrap_or_default().trim()
                    ));
                }
                Produces::Description if prompt.scope() != Scope::Style => {
                    problems.push(format!(
                        "{place} produces `description`, which only an action about a style can: give it `\"scope\": \"style\"`"
                    ));
                }
                Produces::CardPrompt if prompt.scope() != Scope::Canon => {
                    problems.push(format!(
                        "{place} produces `card-prompt`, which only an action about a card can: give it `\"scope\": \"canon\"`"
                    ));
                }
                Produces::Canon
                    if matches!(prompt.scope(), Scope::Scene | Scope::Style | Scope::Comment) =>
                {
                    problems.push(format!(
                        "{place} produces `canon`, which an action about a work, a selection or a card can"
                    ));
                }
                Produces::Release if prompt.scope() != Scope::Release => {
                    problems.push(format!(
                        "{place} produces `release`, which only an action about a release can: give it `\"scope\": \"release\"`"
                    ));
                }
                Produces::Score
                | Produces::Prose
                | Produces::Comment
                | Produces::Reply
                | Produces::Description
                | Produces::Canon
                | Produces::CardPrompt
                | Produces::Release => {}
            }
            // An action about a release writes what it goes out under, and is
            // offered only where there is a release: on a kind that goes out.
            if prompt.scope() == Scope::Release {
                if prompt.produces() != Produces::Release {
                    problems.push(format!(
                        "{place} is about a release and must produce `release`"
                    ));
                }
                let missing = lacking(&|kind: &WorkKind| kind.has_doors());
                if !missing.is_empty() {
                    problems.push(format!(
                        "{place} is about a release, but {} {} no kind of release",
                        missing.join(", "),
                        if missing.len() == 1 { "has" } else { "have" }
                    ));
                }
            }
            // `{release}` and `{releases}` are filled only for an action
            // about a release; anywhere else they would be sent as written.
            if prompt.scope() != Scope::Release
                && let Some(name) = placeholders
                    .iter()
                    .find(|name| *name == "release" || *name == "releases")
            {
                problems.push(format!(
                    "{place} reads `{{{name}}}` but is not about a release: give it `\"scope\": \"release\"`"
                ));
            }
            // An action about a card gathers facts or describes it; anything
            // else it answered would have nowhere to go.
            if prompt.scope() == Scope::Canon
                && !matches!(prompt.produces(), Produces::Canon | Produces::CardPrompt)
            {
                problems.push(format!(
                    "{place} is about a card and must produce `canon` or `card-prompt`"
                ));
            }
            // An action about a selection reads it; one that does not would
            // send the same prompt whatever was selected.
            let reads_selection = placeholders.iter().any(|name| name == "selection");
            if prompt.scope() == Scope::Selection && !reads_selection {
                problems.push(format!(
                    "{place} is about a selection but never reads `{{selection}}`"
                ));
            }
            if prompt.scope() != Scope::Selection && reads_selection {
                problems.push(format!(
                    "{place} reads `{{selection}}` but is not about one: give it `\"scope\": \"selection\"`"
                ));
            }
            // An action about a style brick describes it; an answer of any
            // other shape has nowhere to go - the rule an action about a
            // comment follows below.
            if prompt.scope() == Scope::Style && prompt.produces() != Produces::Description {
                problems.push(format!(
                    "{place} is about a style and must produce `description`"
                ));
            }
            // An action about a comment either reads one off a screenshot or
            // drafts its reply; an answer of any other shape has nowhere to
            // go, and a button whose answer cannot be kept is a dead end.
            if prompt.scope() == Scope::Comment
                && !matches!(prompt.produces(), Produces::Comment | Produces::Reply)
            {
                problems.push(format!(
                    "{place} is about a comment and must produce `comment` or `reply`"
                ));
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

    /// Which roles are a time the work itself was written.
    ///
    /// Three answers, and the third is the one that was wrong: a style prompt
    /// comments on nothing, so the old rule counted it, and a song written
    /// twice read as having many versions. The profile now says outright.
    #[test]
    fn a_role_says_whether_it_is_a_draft_of_the_work() {
        let draft = VersionRole::new("lyrics", "Lyrics");
        assert!(
            draft.counts_as_a_version(),
            "a plain body is a time the work was written"
        );

        let mut critique = VersionRole::new("critique", "Critique");
        critique.comments_on = Some("lyrics".into());
        assert!(
            !critique.counts_as_a_version(),
            "something written ABOUT another role is not a draft of the work"
        );

        let mut style = VersionRole::new("style", "Style prompt");
        style.counts_as_version = Some(false);
        assert!(
            !style.counts_as_a_version(),
            "the profile's explicit `false` wins over commenting on nothing"
        );

        let mut counted = VersionRole::new("counted", "Counted");
        counted.comments_on = Some("lyrics".into());
        counted.counts_as_version = Some(true);
        assert!(
            counted.counts_as_a_version(),
            "the profile's explicit `true` wins over commenting on something"
        );
    }

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

    // Nor an overview - and a document that never chose one must not be
    // given one on the way out, or the shipped board could never change
    // under a profile that is following it.
    #[test]
    fn a_profile_without_an_overview_reads_and_writes_as_having_none() {
        let config = config();
        assert!(config.overview.is_none());
        let written = serde_json::to_value(&config).unwrap();
        assert!(
            written.get("overview").is_none(),
            "an absent overview is left out, not written as null: {written}"
        );
    }

    #[test]
    fn an_overview_round_trips_with_a_widget_this_build_does_not_know() {
        let mut document = serde_json::to_value(config()).unwrap();
        document["overview"] = json!({
            "layout": "mosaic",
            "widgets": [
                { "id": "score", "size": "l", "position": 0 },
                { "id": "from-a-later-build", "size": "s", "position": 1 },
            ],
        });
        let read: ProfileConfig = serde_json::from_value(document).unwrap();
        let overview = read.overview.as_ref().expect("the overview is read");
        assert_eq!(overview.layout, "mosaic");
        assert_eq!(overview.widgets.len(), 2, "the unknown widget is kept");
        assert!(read.validate().is_empty(), "{:?}", read.validate());

        let again: ProfileConfig =
            serde_json::from_value(serde_json::to_value(&read).unwrap()).unwrap();
        assert_eq!(
            again.overview, read.overview,
            "it is written back as it was read"
        );
    }

    #[test]
    fn an_overview_that_only_names_a_layout_has_the_shipped_placement() {
        let overview: OverviewConfig = serde_json::from_value(json!({ "layout": "lead" })).unwrap();
        assert!(overview.widgets.is_empty());
    }

    /// A layout nobody draws is refused on save, by name - but read, so a
    /// profile a later build wrote with a sixth layout still loads.
    #[test]
    fn an_unknown_layout_or_size_reads_but_is_refused_on_save() {
        let mut document = serde_json::to_value(config()).unwrap();
        document["overview"] = json!({
            "layout": "carousel",
            "widgets": [
                { "id": "score", "size": "xl", "position": 0 },
                { "id": "score", "size": "s", "position": 1 },
            ],
        });
        let read: ProfileConfig =
            serde_json::from_value(document).expect("an unknown layout still reads");
        let problems = read.validate();
        assert!(
            problems.iter().any(|p| p
                == "the overview's layout is `grid`, `lead`, `bands`, `mosaic` or `sheet`, not `carousel`"),
            "{problems:?}"
        );
        assert!(
            problems
                .iter()
                .any(|p| p == "overview widget 1 (`score`): the size is `s`, `m` or `l`, not `xl`"),
            "{problems:?}"
        );
        assert!(
            problems
                .iter()
                .any(|p| p.contains("overview widget 2 repeats the key `score`")),
            "{problems:?}"
        );
    }

    #[test]
    fn every_layout_the_window_draws_is_accepted() {
        let mut config = config();
        for layout in OVERVIEW_LAYOUTS {
            config.overview = Some(OverviewConfig {
                layout: layout.into(),
                widgets: WIDGET_SIZES
                    .iter()
                    .enumerate()
                    .map(|(position, size)| WidgetPlacement {
                        id: format!("w{position}"),
                        size: (*size).into(),
                        position: position as u32,
                    })
                    .collect(),
            });
            assert!(
                config.validate().is_empty(),
                "{layout}: {:?}",
                config.validate()
            );
        }
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
            counts_as_version: None,
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
            icon: None,
            produces: None,
            method: None,
            kinds: Vec::new(),
            scope: None,
        }
    }

    /// The shipped Studio profile, with the audio release's YouTube door -
    /// the audio kind's own since v0.86 - given the fields named here.
    fn audio_fields(fields: Vec<ReleaseField>) -> ProfileConfig {
        let mut config = studio();
        for kind in &mut config.work_kinds {
            if kind.key != "audio" {
                continue;
            }
            for release_kind in &mut kind.release_kinds {
                if release_kind.key == "youtube" {
                    release_kind.fields = fields.clone();
                }
            }
        }
        config
    }

    #[test]
    fn the_shipped_profiles_say_what_a_release_goes_out_as() {
        let config = studio();
        let door_of = |kind: &str, door: &str| {
            config
                .vocabulary(kind)
                .release_kinds
                .iter()
                .find(|release_kind| release_kind.key == door)
                .unwrap_or_else(|| panic!("the studio profile ships `{door}` for a {kind}"))
                .clone()
        };
        let keys_of = |release_kind: &ReleaseKind| -> Vec<String> {
            release_kind
                .fields
                .iter()
                .map(|field| field.key.clone())
                .collect()
        };

        // A song goes out as what is made from it, never itself (v0.86).
        assert!(
            config.vocabulary("song").release_kinds.is_empty(),
            "a song has no door of its own"
        );
        // The audio release and the clip both go out on YouTube, each with a
        // comment pinned under it, and each in the shape of a video's preview.
        let audio = door_of("audio", "youtube");
        let clip = door_of("video", "youtube");
        for door in [&audio, &clip] {
            assert_eq!(
                keys_of(door),
                vec!["title", "description", "tags", "pinned"]
            );
            assert_eq!(door.cover_format.as_deref(), Some("16:9"));
        }
        assert_eq!(
            door_of("short", "short").cover_format.as_deref(),
            Some("9:16")
        );
        assert_eq!(
            door_of("audio", "streaming").cover_format.as_deref(),
            Some("1:1")
        );
        assert!(
            clip.fields[0].template().is_some(),
            "a clip's title is filled from the work rather than typed every time"
        );
        assert_eq!(
            audio.fields[1].template(),
            Some("{donor:lyrics}"),
            "an audio release is described by the song's words, as the song's own door was"
        );
    }

    /// An action about a release is offered where there is one, reads
    /// `{release}` only there, and answers with fields.
    #[test]
    fn an_action_about_a_release_is_held_to_releases() {
        let mut config = studio();
        let mut about = action("Write {release} for {title}, after {releases}");
        about.scope = Some("release".into());
        about.produces = Some("release".into());
        about.kinds = vec!["video".into(), "audio".into()];
        config.prompts = vec![about.clone()];
        assert!(config.validate().is_empty(), "{:?}", config.validate());

        let mut on_a_song = about.clone();
        on_a_song.kinds = vec!["song".into()];
        config.prompts = vec![on_a_song];
        assert!(
            config
                .validate()
                .iter()
                .any(|p| p.contains("is about a release, but `song` has no kind of release")),
            "{:?}",
            config.validate()
        );

        let mut loose = action("Write {release}");
        loose.produces = Some("release".into());
        config.prompts = vec![loose];
        let problems = config.validate().join("\n");
        assert!(
            problems.contains("which only an action about a release can"),
            "{problems}"
        );
        assert!(
            problems.contains("reads `{release}` but is not about a release"),
            "{problems}"
        );

        let mut prose = about;
        prose.produces = None;
        config.prompts = vec![prose];
        assert!(
            config
                .validate()
                .iter()
                .any(|p| p.contains("is about a release and must produce `release`"))
        );
    }

    /// A choice lists what can be chosen and starts at one of those; a field
    /// of kinds names kinds the profile has.
    #[test]
    fn a_choice_field_offers_its_options_and_starts_at_one_of_them() {
        let mut config = studio();
        let variant = config
            .work_meta_fields
            .iter()
            .find(|field| field.key == "variant")
            .expect("the studio profile ships the audio's variant")
            .clone();
        assert_eq!(variant.field_type, MetaFieldType::Choice);
        assert!(variant.applies_to("audio") && !variant.applies_to("song"));
        assert_eq!(
            config.defaults_of("audio").get("variant"),
            Some(&serde_json::json!("original"))
        );
        assert!(config.defaults_of("song").is_empty());

        let mut broken = variant.clone();
        broken.default = Some(serde_json::json!("slowed-to-a-halt"));
        broken.kinds.push("opera".into());
        let mut empty = MetaField::new("mood-of-the-day", "Mood", MetaFieldType::Choice);
        empty.key = "empty".into();
        let mut stray = MetaField::new("tempo-word", "Tempo", MetaFieldType::Text);
        stray.options = variant.options.clone();
        config.work_meta_fields = vec![broken, empty, stray];
        let problems = config.validate().join("\n");
        assert!(
            problems.contains("which is not one of its options"),
            "{problems}"
        );
        assert!(problems.contains("names a work kind `opera`"), "{problems}");
        assert!(
            problems.contains("is a choice with nothing to choose from"),
            "{problems}"
        );
        assert!(
            problems.contains("lists options but is not a choice"),
            "{problems}"
        );
    }

    /// The title of a work made from another reads the source's title and a
    /// number, and a cover is a shape.
    #[test]
    fn a_made_title_and_a_cover_shape_are_checked() {
        let short = studio().kind("short").unwrap().clone();
        assert_eq!(short.title_made_from("Tide", "ru", 3), "Tide · шортс 3");
        assert_eq!(short.title_made_from("Tide", "de", 1), "Tide · short 1");
        let clip = studio().kind("video").unwrap().clone();
        assert_eq!(clip.title_made_from("Tide", "en", 1), "Tide — clip");
        assert_eq!(
            clip.title_made_from("Tide", "en", 2),
            "Tide — clip 2",
            "a template without a number numbers the second one on"
        );

        let mut config = studio();
        for kind in &mut config.work_kinds {
            if kind.key == "video" {
                kind.made_title = Some("{name} ({n})".into());
                kind.release_kinds[0].cover_format = Some("wide".into());
            }
        }
        let problems = config.validate().join("\n");
        assert!(problems.contains("never reads `{title}`"), "{problems}");
        assert!(
            problems.contains("reads `{name}`; it reads `{title}` and `{n}`"),
            "{problems}"
        );
        assert!(
            problems.contains("gives its cover the shape `wide`"),
            "{problems}"
        );
        assert!(is_cover_format("16:9") && is_cover_format("1:1"));
        assert!(!is_cover_format("16:0") && !is_cover_format("16x9") && !is_cover_format(":9"));
    }

    #[test]
    fn a_release_field_with_a_hole_in_it_is_refused_at_save() {
        let config = audio_fields(vec![
            ReleaseField::new("title", "Title", ReleaseFieldType::Line).from_template("{typo}"),
        ]);

        let problems = config.validate();

        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("reads `{typo}`, which nothing fills")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_release_field_reads_the_canon_only_as_a_public_text_may() {
        let config = audio_fields(vec![
            ReleaseField::new("title", "Title", ReleaseFieldType::Line).from_template("{title}"),
            ReleaseField::new("description", "Description", ReleaseFieldType::Text)
                .from_template("{canon}"),
            ReleaseField::new("tags", "Tags", ReleaseFieldType::Tags)
                .from_template("{canon:cover} {selection}"),
            ReleaseField::new("pinned", "Pinned", ReleaseFieldType::Text)
                .from_template("{canon:public}"),
        ]);

        let problems = config.validate().join("\n");

        assert!(
            problems.contains("reads `{canon}`, but a release goes out in public"),
            "{problems}"
        );
        assert!(
            problems.contains("reads `{canon:cover}`, but a release goes out in public"),
            "{problems}"
        );
        assert!(
            problems.contains("reads `{selection}`, which only an action"),
            "{problems}"
        );
        assert!(!problems.contains("`{canon:public}`, but"), "{problems}");
    }

    #[test]
    fn a_release_field_is_held_to_the_roles_its_own_kind_has() {
        // `lyrics` is a role of the song, not of the audio release made from
        // it - which reads the song's words as `{donor:lyrics}`. An action
        // naming no kinds would be judged against every kind and could pass
        // on the strength of the song's roles; a release field cannot,
        // because it is read against exactly one kind.
        let config = audio_fields(vec![
            ReleaseField::new("description", "Description", ReleaseFieldType::Text)
                .from_template("{role:lyrics}"),
        ]);

        let problems = config.validate();

        assert!(
            problems.iter().any(|problem| problem
                .contains("reads `{role:lyrics}`, but this kind has no `lyrics` role")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_release_field_reads_the_board_not_a_row_of_it() {
        // `{scene}` is one row, filled from the scene an action was started
        // on. A release is about the whole work and is started from no row at
        // all, so the placeholder would render empty forever.
        let config = audio_fields(vec![
            ReleaseField::new("description", "Description", ReleaseFieldType::Text)
                .from_template("{scene}"),
        ]);

        let problems = config.validate();

        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("it reads `{scenes}`")),
            "{problems:?}"
        );
    }

    #[test]
    fn two_release_fields_cannot_share_a_key() {
        let config = audio_fields(vec![
            ReleaseField::new("title", "Title", ReleaseFieldType::Line),
            ReleaseField::new("title", "Headline", ReleaseFieldType::Line),
        ]);

        let problems = config.validate();

        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("repeats the key `title`")),
            "one key is one box: the second would overwrite the first in `release.meta`. {problems:?}"
        );
    }

    #[test]
    fn a_release_field_needs_a_label_and_a_limit_worth_having() {
        let config = audio_fields(vec![ReleaseField {
            key: "title".into(),
            label: "  ".into(),
            field_type: ReleaseFieldType::Line,
            template: None,
            hint: None,
            limit: Some(0),
        }]);

        let problems = config.validate();

        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("has no label")),
            "{problems:?}"
        );
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("limited to no characters at all")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_release_field_reading_a_role_its_kind_has_is_accepted() {
        let config = audio_fields(vec![
            ReleaseField::new("description", "Description", ReleaseFieldType::Text)
                .from_template("{title}\n\n{role:plot}"),
        ]);

        assert!(config.validate().is_empty(), "{:?}", config.validate());
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
            problems.iter().any(|p| p.contains(
                "reads `{role:lyrics}`, but `video`, `audio`, `short` have no `lyrics` role"
            )),
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
    fn a_style_action_describes_its_brick_and_nothing_else_produces_a_description() {
        let mut config = studio();
        let mut stray = action("Describe it.");
        stray.produces = Some("description".into());
        config.prompts = vec![stray];
        assert!(
            config
                .validate()
                .iter()
                .any(|p| p.contains("only an action about a style can")),
            "{:?}",
            config.validate()
        );

        let mut chatty = action("Say something about it.");
        chatty.scope = Some("style".into());
        config.prompts = vec![chatty];
        assert!(
            config
                .validate()
                .iter()
                .any(|p| p.contains("is about a style and must produce `description`")),
            "{:?}",
            config.validate()
        );

        let mut kept = action("Describe it.");
        kept.scope = Some("style".into());
        kept.produces = Some("description".into());
        config.prompts = vec![kept];
        assert!(
            !config.validate().iter().any(|p| p.contains("style")),
            "{:?}",
            config.validate()
        );
    }

    #[test]
    fn a_comment_action_is_about_a_comment_and_nothing_else_produces_one() {
        let mut config = studio();
        let mut stray = action("Write the reply.");
        stray.produces = Some("reply".into());
        config.prompts = vec![stray];
        assert!(
            config
                .validate()
                .iter()
                .any(|p| p.contains("only an action about a comment can")),
            "{:?}",
            config.validate()
        );

        let mut chatty = action("Say something about it.");
        chatty.scope = Some("comment".into());
        config.prompts = vec![chatty];
        assert!(
            config
                .validate()
                .iter()
                .any(|p| p.contains("must produce `comment` or `reply`")),
            "{:?}",
            config.validate()
        );

        let mut reader = action("Read it.");
        reader.scope = Some("comment".into());
        reader.produces = Some("comment".into());
        config.prompts = vec![reader];
        assert!(config.validate().is_empty(), "{:?}", config.validate());
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

    #[test]
    fn the_canon_of_a_profile_is_checked_at_save() {
        let mut config = studio();
        assert!(config.validate().is_empty(), "{:?}", config.validate());

        let mut second_root = NoteKind::new("brand", "Brand");
        second_root.root = true;
        second_root.sections = vec![CanonSection::new("identity", "General")];
        config.note_kinds.push(second_root);
        let mut crooked = NoteKind::new("prop", "Prop");
        crooked.sections = vec![{
            let mut relations = CanonSection::new("owners", "Owners");
            relations.shape = SectionShape::Relations;
            relations.kinds = vec!["note".into()];
            relations
        }];
        crooked.describe_from = vec!["looks".into()];
        config.note_kinds.push(crooked);

        let problems = config.validate().join("\n");
        assert!(problems.contains("one root"), "{problems}");
        assert!(
            problems.contains("`note`, which is not a kind of card"),
            "{problems}"
        );
        assert!(
            problems.contains("described from a section `looks`"),
            "{problems}"
        );
    }

    #[test]
    fn an_action_about_a_card_or_a_selection_is_held_to_its_shape() {
        let mut config = studio();
        let action = |key: &str, scope: Option<&str>, produces: Option<&str>, template: &str| {
            let mut prompt = config.prompts[0].clone();
            prompt.key = key.into();
            prompt.scope = scope.map(str::to_owned);
            prompt.produces = produces.map(str::to_owned);
            prompt.template = template.into();
            prompt.kinds = Vec::new();
            prompt
        };
        let added = vec![
            action("card-scores", Some("canon"), Some("score"), "Judge it."),
            action(
                "loose-selection",
                Some("selection"),
                Some("canon"),
                "{title}",
            ),
            action("stray-selection", None, None, "{selection}"),
            action("stray-describe", None, Some("card-prompt"), "Describe."),
            action(
                "card-by-title",
                Some("canon"),
                Some("canon"),
                "About {title}.",
            ),
        ];
        config.prompts.extend(added);

        let problems = config.validate().join("\n");
        assert!(
            problems.contains("`card-scores`) is about a card"),
            "{problems}"
        );
        assert!(
            problems.contains("`loose-selection`) is about a selection but never reads"),
            "{problems}"
        );
        assert!(
            problems.contains("`stray-selection`) reads `{selection}` but is not about one"),
            "{problems}"
        );
        assert!(
            problems.contains("`stray-describe`) produces `card-prompt`"),
            "{problems}"
        );
        assert!(
            problems.contains("`card-by-title`) is about a card, which it is given whole"),
            "{problems}"
        );
    }
}
