//! A kind of work and what it is made of: its vocabulary, its scenes, its folders.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::error::Reason;

use super::*;

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
    /// The layers a scene of this kind's storyboard can belong to - the
    /// story, the performance, the texture cut between them - each a visual
    /// style of its own: a second dictionary beside the kinds of shot, which
    /// say how a scene is framed, where a layer says which strand of the
    /// piece it is. A kind that names none has no layers, and its board
    /// draws no column for them. Added in v0.112 - a document without it is
    /// the same document.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<Kind>,
    /// The prompt blocks a scene carries — a still frame, an animation, a
    /// negative — each edited and copied on its own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scene_blocks: Vec<SceneBlock>,
    /// Whether a work of this kind goes out under a cover it builds itself - a
    /// clip, an audio release, a short - and so has the Cover tab. The
    /// cover's parts are the application's since v0.88 (ADR 0049): the
    /// constructor writes a picture, a negative and a lettering from what is
    /// chosen, as the frame writes its loop. A kind that says nothing (a song,
    /// which goes out as what is made from it) has no cover. Format 2 named
    /// the parts here as `cover_blocks`; a document that still does is read as
    /// saying `cover: true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cover: bool,
    /// Format 2's `cover_blocks`, read so a stored document keeps its covers
    /// and never written: [`ProfileConfig`]'s parser turns a list that names
    /// anything into `cover: true`.
    #[serde(default, rename = "cover_blocks", skip_serializing)]
    #[ts(skip)]
    pub(crate) blocks_before_v088: Vec<serde_json::Value>,
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
    /// `{title}` is the title of what it is all made from - the song, for a
    /// short cut from its clip - and `{n}` the new work's number among the
    /// works of this kind made from it. A template without `{n}` numbers only
    /// the second and later ones, inside a closing bracket when it ends with
    /// one: the first short of a song is "Song (short)" and the next "Song
    /// (short 2)". One string for every language since v0.90 - "(video)" is
    /// a name, not a word to translate - though a document may still give one
    /// per language. Absent means the source's title as it is. Added in
    /// v0.86.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub made_title: Option<Label>,
    /// The tab a work of this kind opens on when the address names none:
    /// `versions` for a song lived in through its text, `scenes` for a short
    /// made on its board. The word is the window's tab id and is not checked
    /// against a list here, for the reason a widget id is not: a tab this
    /// build does not draw, or one a work of the kind has not got, opens the
    /// overview instead. Absent is the overview. On the profile rather than
    /// the machine since v0.90.1 - where a card opens is read per kind, and
    /// the kinds are the craft's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_on: Option<String>,
    /// Where a work of this kind keeps its files on disk, under the media
    /// folder this machine names for the workspace: `songs/{title}`.
    /// `{title}` reads the work's title and `{key}` one of its fields;
    /// `{origin.title}` and `{origin.key}` read the same of what it is all
    /// made from, so a clip can look into its song's folder (ADR 0057).
    /// The folder is found by this name, never recorded: kilna looks at it
    /// and writes nothing there but the folder itself, when asked. Absent is
    /// no folder. Added in v0.93 - a document without it is the same
    /// document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// Where a work of this kind keeps the file it is finished as - the clip
    /// a short is cut from, the short a cutter made - inside its folder: a
    /// pattern whose steps may hold `*` for any run of characters and `?`
    /// for one, as `video/*.mp4`. The newest file it matches is the work's,
    /// unless the person chose one in the Files tab (ADR 0070). Absent is no
    /// rule: only a file chosen by hand. Added in v0.101 - a document without
    /// it is the same document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// What a work's files are called when its folder is shared with other
    /// works - a month's folder of a channel, `Music/{month}/CN - {month}` -
    /// read with the same placeholders as `folder`: `Celestia Nomad –
    /// {title}`. A file is the work's when it is called so, or so with more
    /// after it in brackets; a file attached to it is named so; the Files
    /// tab and the finished file look at nothing else (v0.164). Absent: the
    /// folder is the work's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files_named: Option<String>,
    /// Whether a work of this kind is an experiment - a board of trials,
    /// each tried out somewhere outside, heard, and kept or thrown away -
    /// and where what is kept goes (v0.95, ADR 0061). A cover gives a
    /// publication its constructor; a lab gives an experiment its board. The
    /// craft's word for it is the kind's own: Studio calls it an experiment
    /// with sound. Absent is no board. Added in v0.95 - a document without
    /// it is the same document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lab: Option<Lab>,
    /// What a work of this kind is prepared from before it is written - its
    /// brief, the idea the brief is refined into, the boards of scenarios
    /// and sources - and so whether it has those on its Conception tab
    /// (v0.99, ADR 0065). Absent: the tab holds only what is attached to
    /// the work. Added in v0.99 - a document without it is the same
    /// document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conception: Option<Conception>,
    /// Whether a work of this kind is a piece of what it is made from - a
    /// short cut from a clip - rather than the whole of it. An excerpt never
    /// opens a song: a teaser out a day before the track leaves the track the
    /// premiere (v0.100.1, ADR 0067). Absent is a whole work.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub excerpt: bool,
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
            layers: Vec::new(),
            scene_blocks: Vec::new(),
            cover: false,
            blocks_before_v088: Vec::new(),
            frame: false,
            made_title: None,
            open_on: None,
            file: None,
            files_named: None,
            folder: None,
            lab: None,
            conception: None,
            excerpt: false,
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
            title = match title.trim_end().strip_suffix(')') {
                Some(open) if template.trim_end().ends_with(')') => format!("{open} {number})"),
                _ => format!("{title} {number}"),
            };
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

    pub(super) fn weigh(
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

    /// Everything wrong with this kind's vocabulary, each problem naming its place.
    pub(super) fn validate_into(&self, problems: &mut Vec<Reason>, place: &Reason) {
        unique(
            problems,
            Reason::of("place.releaseKinds").param("in", place.clone()),
            self.release_kinds.iter().map(|k| k.key.clone()),
        );
        unique(
            problems,
            Reason::of("place.versionRoles").param("in", place.clone()),
            self.version_roles.iter().map(|r| r.key.clone()),
        );
        unique(
            problems,
            Reason::of("place.statuses").param("in", place.clone()),
            self.statuses.iter().map(|s| s.key.clone()),
        );
        unique(
            problems,
            Reason::of("place.axes").param("in", place.clone()),
            self.axes.iter().map(|a| a.key.clone()),
        );
        unique(
            problems,
            Reason::of("place.tiers").param("in", place.clone()),
            self.tiers.iter().map(|t| t.key.clone()),
        );
        unique(
            problems,
            Reason::of("place.shots").param("in", place.clone()),
            self.shot_types.iter().map(|s| s.key.clone()),
        );
        unique(
            problems,
            Reason::of("place.layers").param("in", place.clone()),
            self.layers.iter().map(|l| l.key.clone()),
        );
        unique(
            problems,
            Reason::of("place.sceneBlocks").param("in", place.clone()),
            self.scene_blocks.iter().map(|b| b.key.clone()),
        );

        if self.statuses.is_empty() {
            problems.push(Reason::of("problem.noStatuses").param("place", place.clone()));
        }

        // The title of a work made from another is written from the source's
        // title and a number, and from nothing else: a template that lost
        // `{title}` names every clip of every song the same.
        if let Some(template) = &self.made_title {
            for word in template.words() {
                if !word.contains("{title}") {
                    problems.push(
                        Reason::of("problem.madeTitleLostTitle")
                            .param("place", place.clone())
                            .param("word", word),
                    );
                }
                for name in crate::assistant::prompt::placeholders(word) {
                    if name != "title" && name != "n" {
                        problems.push(
                            Reason::of("problem.madeTitleForeignPlaceholder")
                                .param("place", place.clone())
                                .param("placeholder", format!("{{{name}}}")),
                        );
                    }
                }
            }
        }

        // A folder template names a place under the media folder and nothing
        // else: one that climbs out of it, or names no folder at all, would
        // list somebody's home in a work's Files tab. The reason it gives is
        // said by the folder's own rules, already in English.
        if let Some(template) = &self.folder
            && let Some(problem) = crate::folder::problem(template)
        {
            problems.push(
                Reason::of("problem.folderTemplate")
                    .param("place", place.clone())
                    .param("template", template.as_str())
                    .param("problem", problem.to_string()),
            );
        }
        // The name a work's files take in a shared folder is one step of a
        // template: the same placeholders, no slash (v0.164).
        if let Some(template) = &self.files_named
            && let Some(problem) = crate::folder::problem(template).or_else(|| {
                template
                    .contains(['/', '\\'])
                    .then(|| "names a folder; it is what a file is called".to_owned())
            })
        {
            problems.push(
                Reason::of("problem.folderTemplate")
                    .param("place", place.clone())
                    .param("template", template.as_str())
                    .param("problem", problem),
            );
        }
        // A rule for the finished file names a file inside the work's folder
        // and nothing else, for the folder's reason.
        if let Some(rule) = &self.file
            && let Some(problem) = crate::folder::rule_problem(rule)
        {
            problems.push(
                Reason::of("problem.fileRule")
                    .param("place", place.clone())
                    .param("rule", rule.as_str())
                    .param("problem", problem.to_string()),
            );
        }

        for (index, kind) in self.release_kinds.iter().enumerate() {
            let at = release_kind_at(place, index, &kind.key);
            if let Some(format) = &kind.cover_format
                && !is_cover_format(format)
            {
                problems.push(
                    Reason::of("problem.coverShape")
                        .param("place", at.clone())
                        .param("format", format.as_str()),
                );
            }
            for pattern in &kind.links {
                if !is_link_pattern(pattern) {
                    problems.push(
                        Reason::of("problem.linkPattern")
                            .param("place", at.clone())
                            .param("pattern", pattern.as_str()),
                    );
                }
            }
        }

        let roles: BTreeSet<&str> = self.version_roles.iter().map(|r| r.key.as_str()).collect();
        for (index, role) in self.version_roles.iter().enumerate() {
            // Not a let-chain: the MSRV is older than they are.
            let Some(target) = &role.comments_on else {
                continue;
            };
            if !roles.contains(target.as_str()) {
                problems.push(
                    Reason::of("problem.roleCommentsOnNothing")
                        .param("place", version_role_at(place, index, &role.key))
                        .param("target", target.as_str()),
                );
            }
        }
        for (index, role) in self.version_roles.iter().enumerate() {
            let Some(body) = &role.body else {
                continue;
            };
            if !BODY_KINDS.contains(&body.as_str()) {
                problems.push(
                    Reason::of("problem.roleBody")
                        .param("place", version_role_at(place, index, &role.key))
                        .param("body", body.as_str()),
                );
            }
        }

        let axes: BTreeSet<&str> = self.axes.iter().map(|a| a.key.as_str()).collect();
        for (index, kind) in self.release_kinds.iter().enumerate() {
            let at = release_kind_at(place, index, &kind.key);
            for required in &kind.requires {
                if !roles.contains(required.as_str()) {
                    problems.push(
                        Reason::of("problem.releaseRequiresNoRole")
                            .param("place", at.clone())
                            .param("required", required.as_str()),
                    );
                }
            }
            for (axis, weight) in &kind.axis_weights {
                if !axes.contains(axis.as_str()) {
                    problems.push(
                        Reason::of("problem.releaseWeighsNoAxis")
                            .param("place", at.clone())
                            .param("axis", axis.as_str()),
                    );
                }
                if !(weight.is_finite() && *weight >= 0.0) {
                    problems.push(
                        Reason::of("problem.releaseWeight")
                            .param("place", at.clone())
                            .param("axis", axis.as_str())
                            .param("weight", weight.to_string()),
                    );
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
                Reason::of("place.releaseFields")
                    .param("in", place.clone())
                    .param("key", kind.key.as_str()),
                kind.fields.iter().map(|field| field.key.clone()),
            );
            for (number, field) in kind.fields.iter().enumerate() {
                let at = release_field_at(&at, number, &field.key);
                if field.label.is_blank() {
                    problems.push(Reason::of("problem.fieldNoLabel").param("place", at.clone()));
                }
                if field.limit == Some(0) {
                    problems.push(Reason::of("problem.fieldNoRoom").param("place", at.clone()));
                }
                if let (Some(tail), Some(limit)) = (field.suffix(), field.limit)
                    && tail.chars().count() >= limit as usize
                {
                    problems.push(
                        Reason::of("problem.fieldTailTooLong")
                            .param("place", at.clone())
                            .param("tail", tail)
                            .param("limit", limit),
                    );
                }
                if let Some(premiere) = &field.premiere {
                    for word in premiere.words() {
                        if let Some(limit) = field.limit
                            && word.chars().count() >= limit as usize
                        {
                            problems.push(
                                Reason::of("problem.fieldPremiereTooLong")
                                    .param("place", at.clone())
                                    .param("word", word)
                                    .param("limit", limit),
                            );
                        }
                    }
                }
                let Some(template) = field.template() else {
                    continue;
                };
                for name in crate::assistant::prompt::placeholders(template) {
                    let placeholder = format!("{{{name}}}");
                    if !crate::assistant::prompt::is_known_placeholder(&name) {
                        problems.push(
                            Reason::of("problem.readsNothing")
                                .param("place", at.clone())
                                .param("placeholder", placeholder),
                        );
                        continue;
                    }
                    if let Some(role) = name.strip_prefix("role:")
                        && !roles.contains(role)
                    {
                        problems.push(
                            Reason::of("problem.fieldNoRole")
                                .param("place", at.clone())
                                .param("placeholder", placeholder.clone())
                                .param("role", role),
                        );
                    }
                    if (name == "scenes" || name == "scene")
                        && self.shot_types.is_empty()
                        && self.scene_blocks.is_empty()
                    {
                        problems.push(
                            Reason::of("problem.fieldNoStoryboard")
                                .param("place", at.clone())
                                .param("placeholder", placeholder.clone()),
                        );
                    }
                    if crate::assistant::prompt::canon_lens(&name)
                        .is_some_and(|lens| lens != Lens::Public)
                    {
                        problems.push(
                            Reason::of("problem.fieldCanonNotPublic")
                                .param("place", at.clone())
                                .param("placeholder", placeholder.clone()),
                        );
                    }
                    if name == "selection" {
                        problems
                            .push(Reason::of("problem.fieldSelection").param("place", at.clone()));
                    }
                    if name == crate::assistant::prompt::WEEK {
                        problems.push(Reason::of("problem.fieldWeek").param("place", at.clone()));
                    }
                    if name == "scene" {
                        problems.push(Reason::of("problem.fieldScene").param("place", at.clone()));
                    }
                }
            }
        }

        for (index, axis) in self.axes.iter().enumerate() {
            let place = axis_at(place, index, &axis.key);
            if !(axis.scale.is_finite() && axis.scale > 0.0) {
                problems.push(
                    Reason::of("problem.axisScale")
                        .param("place", place.clone())
                        .param("scale", axis.scale.to_string()),
                );
            }
            if !(axis.weight.is_finite() && axis.weight >= 0.0) {
                problems.push(
                    Reason::of("problem.axisWeight")
                        .param("place", place.clone())
                        .param("weight", axis.weight.to_string()),
                );
            }
            match axis.kind {
                AxisKind::Choice => {
                    if axis.options.is_empty() {
                        problems
                            .push(Reason::of("problem.choiceEmpty").param("place", place.clone()));
                    }
                    unique(
                        problems,
                        Reason::of("place.options").param("in", place.clone()),
                        axis.options.iter().map(|o| o.key.clone()),
                    );
                    for option in &axis.options {
                        if !(option.value.is_finite()
                            && option.value >= 0.0
                            && option.value <= axis.scale)
                        {
                            problems.push(
                                Reason::of("problem.optionRange")
                                    .param("place", place.clone())
                                    .param("key", option.key.as_str())
                                    .param("value", option.value.to_string())
                                    .param("scale", axis.scale.to_string()),
                            );
                        }
                    }
                }
                AxisKind::Scale | AxisKind::Flag => {
                    if !axis.options.is_empty() {
                        problems.push(
                            Reason::of("problem.axisOptionsNotChoice")
                                .param("place", place.clone()),
                        );
                    }
                }
            }

            // A rubric is checked like the options are: a mark off the scale
            // describes nothing, and two sentences about the same mark leave
            // the app to pick one of them.
            let mut named = BTreeSet::new();
            for mark in &axis.rubric {
                if !(mark.at.is_finite() && mark.at >= 0.0 && mark.at <= axis.scale) {
                    problems.push(
                        Reason::of("problem.rubricOffScale")
                            .param("place", place.clone())
                            .param("at", mark.at.to_string())
                            .param("scale", axis.scale.to_string()),
                    );
                    continue;
                }
                if !named.insert(mark.at.to_bits()) {
                    problems.push(
                        Reason::of("problem.rubricTwice")
                            .param("place", place.clone())
                            .param("at", mark.at.to_string()),
                    );
                }
            }
        }

        for (index, tier) in self.tiers.iter().enumerate() {
            if !(tier.min.is_finite() && (0.0..=100.0).contains(&tier.min)) {
                problems.push(
                    Reason::of("problem.tierStart")
                        .param("place", tier_at(place, index, &tier.key))
                        .param("min", tier.min.to_string()),
                );
            }
        }
    }
}

/// The vocabulary of a kind the profile does not know: nothing, so that a
/// work whose kind was removed from the profile reads as unscorable and
/// roleless rather than crashing the screen it is on.
pub(super) static NO_KIND: std::sync::LazyLock<WorkKind> =
    std::sync::LazyLock::new(|| WorkKind::new("", ""));

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
    /// The block that says what the picture shows: a scene's frame, built
    /// in the constructor, is written around it (v0.88). One per kind; a
    /// kind that marks none has scenes without a built frame.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub picture: bool,
}

impl SceneBlock {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            hint: None,
            picture: false,
        }
    }
}
