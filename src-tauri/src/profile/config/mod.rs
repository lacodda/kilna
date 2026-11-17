use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::Reason;

mod axis;
mod board;
mod label;
mod lookup;
mod mark;
mod meta_field;
mod note_kind;
mod overview;
mod release;
mod rhythm;
mod stage;
mod status;
mod style;
mod validate;
mod version_role;
mod vibe;
mod work_kind;

pub use axis::*;
pub use board::*;
pub use label::*;
pub use mark::*;
pub use meta_field::*;
pub use note_kind::*;
pub use overview::*;
pub use release::*;
pub use rhythm::*;
pub use stage::*;
pub use status::*;
pub use style::*;
pub use version_role::*;
pub use vibe::*;
pub use work_kind::*;

#[cfg(test)]
mod tests;

/// The version of the document's shape. See [`ProfileConfig::format`].
pub const FORMAT: u32 = 3;

/// A craft scenario. Everything that differs between music, prose and podcasting
/// is described here rather than in the schema.
///
/// **Format 2** (v0.57): the vocabulary a work is judged and shipped by — its
/// axes, tiers, version roles, kinds of release, statuses — belongs to the
/// *kind* of work, not to the profile. A song and a video are one craft with
/// two vocabularies, and a video judged on "hook" and "lyrics" is nonsense.
/// Format 1 laid all of it flat on the profile; a format 1 document is still
/// read — see [`RawProfileConfig`] — and comes out of the parser in the
/// current format, with the flat vocabulary handed to every kind that
/// declared none of its own.
///
/// **Format 3** (v0.88): a kind says whether it has a cover (`cover: true`)
/// rather than naming the parts of its prompt; a format 2 document's
/// `cover_blocks` is read as the flag.
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
    /// The three lines a work's feel is said along, and the words on each
    /// (FR-02): moods, characters, complexities. Absent, [`Self::vibe`]
    /// answers with the line's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub vibe: Option<VibeAxes>,
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
    /// The texts a role is written from the dictionary: a song's style
    /// prompt picked from the types of sound, block by block (v0.94). A role
    /// no composition names is written by hand only. Added in v0.94 - a
    /// document without it is the same document.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compose: Vec<Composition>,
    /// How a work's overview is laid out, and where each widget stands on it.
    /// Absent means the owner's choice - the lead layout with the placement
    /// the window ships - which is why nothing here spells a default out: a
    /// profile that never chose keeps following the shipped board as it
    /// changes. On the profile for the catalogue columns' reason: a craft
    /// looks at its works through its own board. Added in v0.82 - a document
    /// without it is the same document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overview: Option<OverviewConfig>,
    /// How many ideas for its cover a publication is given when it is made
    /// ("Make a clip"), 0 to 5; 0 asks for none. Absent means three. Added
    /// in v0.89 - a document without it is the same document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_ideas: Option<u8>,
    /// How the guard of repeats reads "recent" and "rare" (v0.90, ADR 0054).
    /// Absent means its defaults: 90 days, past the 20 000th stem of the
    /// language, in no more than two works of the owner's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<Guard>,
    /// The boards a work's conception is laid out on: scenarios, sources
    /// (v0.99, ADR 0065). Each names the kinds of card it holds and the
    /// fields of each; a kind of work lists the boards its Conception tab
    /// shows (`conception.boards`). On the profile rather than on a kind,
    /// as the kinds of note are: a song and a clip may share one. Added in
    /// v0.99 - a document without it is the same document.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boards: Vec<Board>,
}

/// How the guard of repeats reads its two words (ADR 0054).
///
/// A song that has not gone out is held against the songs that have, and
/// against the ones booked: a term of the register they share is spent
/// (orange); a rare word they share is red when the other song went out -
/// or goes out - within the window, orange when longer ago.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Guard {
    /// How close, in days, a song sharing a rare word is too close: red
    /// inside, orange outside.
    #[serde(default = "Guard::default_window")]
    pub window_days: u32,
    /// From which rank of the language's stems a word is rare: the stems
    /// past it, and the ones the list does not hold at all.
    #[serde(default = "Guard::default_rank")]
    pub rare_rank: u32,
    /// In how many of the owner's works a word may stand and still be
    /// rare: past it, the word is the owner's own and spent only if the
    /// register says so.
    #[serde(default = "Guard::default_works")]
    pub rare_in_works: u32,
}

impl Guard {
    fn default_window() -> u32 {
        90
    }
    /// Past it: "пульсар" (57 460), "сухогруз" (25 796) - words a listener
    /// carries from one song to the next. A word met more often than the
    /// 20 000th stem of the language passes unnoticed the second time.
    fn default_rank() -> u32 {
        20_000
    }
    fn default_works() -> u32 {
        2
    }
}

impl Default for Guard {
    fn default() -> Self {
        Self {
            window_days: Self::default_window(),
            rare_rank: Self::default_rank(),
            rare_in_works: Self::default_works(),
        }
    }
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
    pub vibe: Option<VibeAxes>,
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
    pub compose: Vec<Composition>,
    #[serde(default)]
    pub overview: Option<OverviewConfig>,
    #[serde(default)]
    pub cover_ideas: Option<u8>,
    #[serde(default)]
    pub guard: Option<Guard>,
    #[serde(default)]
    pub boards: Vec<Board>,
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
        // Format 2 named a cover's parts; naming any is having a cover.
        for kind in &mut work_kinds {
            if !kind.blocks_before_v088.is_empty() {
                kind.cover = true;
                kind.blocks_before_v088.clear();
            }
        }
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
            vibe: raw.vibe,
            rhythm: raw.rhythm,
            catalogue_columns: raw.catalogue_columns,
            catalogue_columns_by_kind: raw.catalogue_columns_by_kind,
            note_kinds: raw.note_kinds,
            relation_kinds: raw.relation_kinds,
            style_types: raw.style_types,
            compose: raw.compose,
            overview: raw.overview,
            cover_ideas: raw.cover_ideas,
            guard: raw.guard,
            boards: raw.boards,
        }
    }
}

/// Where a problem sits, said as a reason a window can translate and code can
/// walk: `place.<noun>` with the position (`n`, from 1) and key of the thing,
/// and, for a thing inside another, the place of the other under `in`. A key
/// is either always a whole place or always nested, never both.
fn work_kind_at(index: usize, key: &str) -> Reason {
    Reason::of("place.workKind")
        .param("n", index + 1)
        .param("key", key)
}

fn note_kind_at(index: usize, key: &str) -> Reason {
    Reason::of("place.noteKind")
        .param("n", index + 1)
        .param("key", key)
}

fn style_type_at(index: usize, key: &str) -> Reason {
    Reason::of("place.styleType")
        .param("n", index + 1)
        .param("key", key)
}

fn meta_field_at(index: usize, key: &str) -> Reason {
    Reason::of("place.metaField")
        .param("n", index + 1)
        .param("key", key)
}

fn board_at(index: usize, key: &str) -> Reason {
    Reason::of("place.board")
        .param("n", index + 1)
        .param("key", key)
}

fn composition_at(index: usize, key: &str) -> Reason {
    Reason::of("place.composition")
        .param("n", index + 1)
        .param("key", key)
}

fn action_at(index: usize, key: &str) -> Reason {
    Reason::of("place.action")
        .param("n", index + 1)
        .param("key", key)
}

fn widget_at(index: usize, key: &str) -> Reason {
    Reason::of("place.overviewWidget")
        .param("n", index + 1)
        .param("key", key)
}

fn axis_at(within: &Reason, index: usize, key: &str) -> Reason {
    Reason::of("place.axis")
        .param("in", within.clone())
        .param("n", index + 1)
        .param("key", key)
}

fn release_kind_at(within: &Reason, index: usize, key: &str) -> Reason {
    Reason::of("place.releaseKind")
        .param("in", within.clone())
        .param("n", index + 1)
        .param("key", key)
}

fn release_field_at(within: &Reason, index: usize, key: &str) -> Reason {
    Reason::of("place.releaseField")
        .param("in", within.clone())
        .param("n", index + 1)
        .param("key", key)
}

fn version_role_at(within: &Reason, index: usize, key: &str) -> Reason {
    Reason::of("place.versionRole")
        .param("in", within.clone())
        .param("n", index + 1)
        .param("key", key)
}

fn tier_at(within: &Reason, index: usize, key: &str) -> Reason {
    Reason::of("place.tier")
        .param("in", within.clone())
        .param("n", index + 1)
        .param("key", key)
}

fn card_kind_at(within: &Reason, index: usize, key: &str) -> Reason {
    Reason::of("place.cardKind")
        .param("in", within.clone())
        .param("n", index + 1)
        .param("key", key)
}

fn section_at(within: &Reason, key: &str) -> Reason {
    Reason::of("place.section")
        .param("in", within.clone())
        .param("key", key)
}

/// One of two reasons by how many things it names: the same sentence in the
/// singular and the plural, because a language that declines the verb needs
/// both written out.
fn by_count(count: usize, one: Reason, many: Reason) -> Reason {
    if count == 1 { one } else { many }
}

/// Keys that repeat or are blank, reported with their position. `what` says
/// which list the keys are of, as a reason (`place.workKinds`).
fn unique(problems: &mut Vec<Reason>, what: Reason, keys: impl Iterator<Item = String>) {
    let mut seen = BTreeSet::new();
    for (index, key) in keys.enumerate() {
        if key.trim().is_empty() {
            problems.push(
                Reason::of("problem.noKey")
                    .param("what", what.clone())
                    .param("n", index + 1),
            );
        } else if !seen.insert(key.clone()) {
            problems.push(
                Reason::of("problem.repeatedKey")
                    .param("what", what.clone())
                    .param("n", index + 1)
                    .param("key", key),
            );
        }
    }
}
