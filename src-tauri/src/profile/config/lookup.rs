//! Reading a profile: the kind, the vocabulary, the boards a key names.

use std::collections::BTreeSet;

use super::*;

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
    pub(super) fn kind_named(&self, kind: &str) -> serde_json::Value {
        self.kind(kind)
            .and_then(|known| serde_json::to_value(&known.label).ok())
            .unwrap_or_else(|| serde_json::Value::String(kind.to_owned()))
    }

    /// Keys, listed for a refusal: `song, video, short`.
    pub(super) fn keys<'a>(keys: impl Iterator<Item = &'a str>) -> String {
        keys.collect::<Vec<_>>().join(", ")
    }

    /// The vocabulary of a kind: its own, or — for a kind the profile does
    /// not know — an empty one, so nothing downstream has to ask twice.
    pub fn vocabulary(&self, kind: &str) -> &WorkKind {
        self.kind(kind).unwrap_or(&NO_KIND)
    }

    /// How many ideas for its cover a publication is given when it is made.
    pub fn ideas_on_make(&self) -> u8 {
        self.cover_ideas
            .unwrap_or(crate::cover::idea::ON_MAKE)
            .min(crate::cover::idea::MOST)
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

    /// The fields a work of `kind` is numbered by: a new one is given the
    /// next code of each (ADR 0059).
    pub fn numbered_of(&self, kind: &str) -> Vec<&MetaField> {
        self.fields_of(kind)
            .into_iter()
            .filter(|field| field.numbered_from.is_some())
            .collect()
    }

    /// The composition a version of `role` of a work of `kind` is written
    /// and read by, if the craft names one.
    pub fn composition_for(&self, kind: &str, role: &str) -> Option<&Composition> {
        self.compose.iter().find(|c| c.writes(kind, role))
    }

    /// The composition by its key.
    pub fn composition(&self, key: &str) -> Option<&Composition> {
        self.compose.iter().find(|c| c.key == key)
    }

    /// The lab of a kind, when its works are experiments (ADR 0061).
    pub fn lab(&self, kind: &str) -> Option<&Lab> {
        self.kind(kind).and_then(|found| found.lab.as_ref())
    }

    /// The conception of a kind, when its works are prepared on a
    /// Conception tab (ADR 0065).
    pub fn conception(&self, kind: &str) -> Option<&Conception> {
        self.kind(kind).and_then(|found| found.conception.as_ref())
    }

    /// The conception of a kind, or the refusal that says works of the kind
    /// are not prepared on one.
    pub fn require_conception(&self, kind: &str) -> crate::Result<&Conception> {
        self.conception(kind).ok_or_else(|| {
            crate::Error::refused("conception.noConception").param("kind", self.kind_named(kind))
        })
    }

    /// The board by its key.
    pub fn board(&self, key: &str) -> Option<&Board> {
        self.boards.iter().find(|board| board.key == key)
    }

    /// The boards a work of `kind` lays out, in its conception's order. A
    /// key the profile does not name is left out - validation says so.
    pub fn boards_of(&self, kind: &str) -> Vec<&Board> {
        self.conception(kind)
            .map(|conception| {
                conception
                    .boards
                    .iter()
                    .filter_map(|key| self.board(key))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The board of a work of `kind` by its key, or the refusal that says
    /// the kind lays out no such board.
    pub fn require_board(&self, kind: &str, key: &str) -> crate::Result<&Board> {
        self.boards_of(kind)
            .into_iter()
            .find(|board| board.key == key)
            .ok_or_else(|| {
                crate::Error::refused("board.notOfKind")
                    .param("board", key)
                    .param("kind", self.kind_named(kind))
            })
    }

    /// The kinds a kept trial of `lab_kind` may go into: every other kind
    /// that has the `role` it keeps its trials in.
    pub fn harvest_kinds<'a>(
        &'a self,
        role: &'a str,
        lab_kind: &'a str,
    ) -> impl Iterator<Item = &'a WorkKind> + 'a {
        self.work_kinds.iter().filter(move |kind| {
            kind.key != lab_kind && kind.version_roles.iter().any(|r| r.key == role)
        })
    }

    /// The composition a trial of an experiment of `lab_kind` is written and
    /// read by: the one that writes the role its trials are kept in, for any
    /// kind that keeps them. A trial is a text of that role before it lands.
    pub fn trial_composition(&self, lab_kind: &str) -> Option<&Composition> {
        let role = self.lab(lab_kind)?.harvest.as_deref()?;
        self.harvest_kinds(role, lab_kind)
            .find_map(|kind| self.composition_for(&kind.key, role))
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

    /// The note kind that is the library, when the profile has one (ADR
    /// 0066): the works of culture the boards link to.
    pub fn library(&self) -> Option<&NoteKind> {
        self.note_kinds.iter().find(|kind| kind.library)
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

    /// The lines a work's feel is said along: the profile's, or the line's
    /// own when it names none.
    pub fn vibe(&self) -> VibeAxes {
        self.vibe.clone().unwrap_or_else(default_vibe)
    }

    /// The stop a stage stands at. A profile that names only some of the
    /// five answers for the rest with the line's own.
    pub fn stage_at(&self, percent: i64) -> Option<Stage> {
        let at = normalize_stage(Some(percent));
        self.stages()
            .into_iter()
            .find(|stage| stage.percent == at)
            .or_else(|| default_stages().into_iter().find(|s| s.percent == at))
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
