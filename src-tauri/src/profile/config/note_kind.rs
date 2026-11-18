//! Kinds of notes, the canon's sections and what each section holds.

use serde::{Deserialize, Serialize};

use crate::error::Reason;

use super::*;

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
    /// A note of this kind is an entry of the library: a work of culture -
    /// a novel, a film, a painting, a gospel - kept once for the whole
    /// workspace and linked from the boards of the works it is drawn on
    /// (v0.100, ADR 0066). Its title is the work's, its attribution the
    /// author, its aliases its other titles; it is in one of `media`, and
    /// says when it was made. One kind of a profile at most. Added in v0.100
    /// - a document without it is the same document.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub library: bool,
    /// What an entry of the library can be in: literature, film, painting,
    /// music. Only on a library kind; in the order the board's "by kind"
    /// reads them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub media: Vec<Medium>,
    /// A note of this kind is a long document (v0.145, FR-09 9.1): read in a
    /// reading view of its own, with its headings as an outline, apart from
    /// the short working notes.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub document: bool,
    /// Turned off in the profile editor (v0.141, FR-18): a kind of card not
    /// offered for a new card, and no passport made of it with a space.
    /// Cards of it already made stay, and read as they did.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub off: bool,
}

/// What a work of culture is in - literature, film, music - as an entry of
/// the library says it (ADR 0066).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Medium {
    pub key: String,
    pub label: Label,
    /// Whether a work of this medium is ever quoted to a writer. A song's
    /// words are its author's to sing: a card of a song names it and says
    /// why, and its quotes never reach the order. Absent: quoted.
    #[serde(default = "quoted_by_default", skip_serializing_if = "is_quoted")]
    pub quoted: bool,
}

pub(super) fn quoted_by_default() -> bool {
    true
}

pub(super) fn is_quoted(quoted: &bool) -> bool {
    *quoted
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
            library: false,
            media: Vec::new(),
            document: false,
            off: false,
        }
    }

    /// Whether a note of this kind is a card of the canon.
    pub fn is_card(&self) -> bool {
        !self.sections.is_empty()
    }

    pub fn section(&self, key: &str) -> Option<&CanonSection> {
        self.sections.iter().find(|section| section.key == key)
    }

    /// A medium of the library by its key.
    pub fn medium(&self, key: &str) -> Option<&Medium> {
        self.media.iter().find(|medium| medium.key == key)
    }

    /// Everything wrong with this kind's sections, each problem naming its place.
    pub(super) fn validate_into(
        &self,
        problems: &mut Vec<Reason>,
        place: &Reason,
        kinds: &[NoteKind],
    ) {
        unique(
            problems,
            Reason::of("place.sections").param("in", place.clone()),
            self.sections.iter().map(|s| s.key.clone()),
        );
        unique(
            problems,
            Reason::of("place.media").param("in", place.clone()),
            self.media.iter().map(|m| m.key.clone()),
        );
        // An entry of the library is a page about a work of culture: not a
        // card of the canon, not spent by a work, not one line.
        if self.library && (self.is_card() || self.material || self.line) {
            problems.push(Reason::of("problem.libraryEntry").param("place", place.clone()));
        }
        if !self.library && !self.media.is_empty() {
            problems.push(Reason::of("problem.mediaOutsideLibrary").param("place", place.clone()));
        }
        if self.root && !self.is_card() {
            problems.push(Reason::of("problem.rootWithoutSections").param("place", place.clone()));
        }
        // A card is known by its facts and lives on the canon: it is neither
        // spent by a work nor a single line.
        if self.is_card() && (self.material || self.line) {
            problems.push(Reason::of("problem.cardIsNotMaterial").param("place", place.clone()));
        }
        for key in &self.describe_from {
            match self.section(key) {
                None => problems.push(
                    Reason::of("problem.describedFromMissing")
                        .param("place", place.clone())
                        .param("key", key.as_str()),
                ),
                Some(section) if !section.shape.holds_words() => problems.push(
                    Reason::of("problem.describedFromNoWords")
                        .param("place", place.clone())
                        .param("key", key.as_str()),
                ),
                Some(_) => {}
            }
        }
        let mut gathers_the_rest = 0;
        for section in &self.sections {
            let at = section_at(place, &section.key);
            if section.shape != SectionShape::Relations && !section.kinds.is_empty() {
                problems.push(Reason::of("problem.sectionKinds").param("place", at.clone()));
            }
            match section.draw {
                Some(0) => {
                    problems
                        .push(Reason::of("problem.sectionDrawsNone").param("place", at.clone()));
                }
                Some(_) if !section.shape.holds_words() => {
                    problems.push(
                        Reason::of("problem.sectionDrawsFromNoWords").param("place", at.clone()),
                    );
                }
                _ => {}
            }
            if section.shape == SectionShape::Relations {
                if section.kinds.is_empty() {
                    gathers_the_rest += 1;
                }
                for kind in &section.kinds {
                    if !kinds.iter().any(|one| &one.key == kind && one.is_card()) {
                        problems.push(
                            Reason::of("problem.sectionGathersNonCard")
                                .param("place", at.clone())
                                .param("kind", kind.as_str()),
                        );
                    }
                }
            }
        }
        if gathers_the_rest > 1 {
            problems.push(
                Reason::of("problem.sectionsGatherTheRest")
                    .param("place", place.clone())
                    .param("count", gathers_the_rest),
            );
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
    /// How many of the section's entries a prompt is handed, drawn at random
    /// each time one is written (v0.100.2): a pool a text is built from - a
    /// hundred moves of a description - reaches every text as a different
    /// handful. A model handed the whole pool reaches for the same few every
    /// time, which is the sameness the pool is there to break. Absent hands
    /// over every entry; the card itself always shows them all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draw: Option<u16>,
    /// A section an automation reads rather than a person (v0.141, FR-18):
    /// the templates a release's text is written from. The card gathers such
    /// sections under one folded group, out of the way of the passport.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub advanced: bool,
    /// Turned off in the profile editor (v0.141): not offered when a section
    /// is added to a card. One that holds facts still shows them.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub off: bool,
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
            draw: None,
            advanced: false,
            off: false,
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
    /// What must not appear: statements a picture's negative reads through
    /// the cover's lens, and a public text through the public one (v0.88).
    Bans,
    /// The card's relations to other cards.
    Relations,
    /// Where the card appears, counted from scenes, texts and sources.
    Appearances,
}

impl SectionShape {
    pub(super) fn is_facts(&self) -> bool {
        *self == SectionShape::Facts
    }

    /// Whether a fact can be written into a section of this shape.
    pub fn holds_facts(self) -> bool {
        !matches!(self, SectionShape::Relations | SectionShape::Appearances)
    }

    /// Whether its entries are statements a description can be written from.
    pub fn holds_words(self) -> bool {
        matches!(
            self,
            SectionShape::Facts | SectionShape::Slots | SectionShape::Bans
        )
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
