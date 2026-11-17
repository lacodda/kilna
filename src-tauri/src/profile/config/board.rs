//! The boards a kind's conception and lab are worked on: cards, fields, rules.

use serde::{Deserialize, Serialize};

use super::*;

/// What a kind's Conception tab holds (v0.99, ADR 0065).
///
/// A work is prepared before it is written: the person says what it is
/// about and how they see it (the brief), the brief is refined into one
/// idea, and boards gather candidates - scenarios to write it by, sources
/// to draw on - each starred or turned down. What comes out is the order a
/// writer works from, the same whether the window's assistant writes, an
/// agent reads it, or the person copies it elsewhere.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Conception {
    /// The version role the brief is written in. Its versions are the
    /// brief's revisions and never a draft of the work: the role does not
    /// count as a version (`counts_as_version: false`), which the profile's
    /// validation holds it to, and a version of it is never made current.
    pub brief: String,
    /// The field the brief is refined into: a song's premise. A text, one
    /// the kind carries. Absent: "Refine" has nowhere to write the idea, and
    /// rewrites only the brief.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idea: Option<String>,
    /// The roles whose current text goes into the order beside the brief:
    /// what the writer must keep to, as a song's style prompt sets the
    /// sound its lyrics are sung over.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with: Vec<String>,
    /// The boards the tab lays out, in order, by key (`boards` on the
    /// profile).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boards: Vec<String>,
}

/// A board of a work's conception (v0.99, ADR 0065): the kinds of card it
/// holds, each with its fields, and how a writer reads it.
///
/// One mechanism for every board the craft names - cards placed and dragged
/// among groups, starred or turned down, the person's own or the
/// assistant's - so a board of scenarios and a board of sources are the
/// same thing with different words, as a song and a chapter are the same
/// work with different roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Board {
    pub key: String,
    pub label: Label,
    /// What the board is for, said under its name. Absent says nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Label>,
    /// How the order reads the board. See [`BoardRule`].
    #[serde(default)]
    pub rule: BoardRule,
    /// The kinds of card it holds, in the order the board offers them. At
    /// least one.
    pub cards: Vec<CardKind>,
}

/// How a writer reads a board (v0.99, ADR 0065).
///
/// Two boards, two readings. Scenarios are rivals: a text is written by
/// one, so a star narrows the choice - one starred is the one, several are
/// what to choose or combine from, none leaves every card not turned down
/// to the writer. Sources add up: every card not turned down goes in, the
/// starred ones first.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum BoardRule {
    /// The text is written by one card.
    #[default]
    One,
    /// Every card is drawn on.
    All,
}

/// A kind of card on a board: a scenario, a work of culture, a concept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct CardKind {
    pub key: String,
    pub label: Label,
    /// What a card of this kind says, field by field. The first is its
    /// headline - what the card is called on the board and in the order -
    /// unless the card links to the library, where it is called what its
    /// entry is called. At least one.
    pub fields: Vec<CardField>,
    /// A card of this kind links to an entry of the library (v0.100, ADR
    /// 0066): what the work of culture is - its title, its author, when it
    /// was made, its medium - is the entry's, one record for the whole
    /// workspace, and the card says only what it is to this work. Needs a
    /// note kind marked `library`. Added in v0.100 - a document without it
    /// is the same document.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub library: bool,
}

/// A field of a kind of card: a text with a name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct CardField {
    pub key: String,
    pub label: Label,
    /// What the empty field says, as a hint of what goes in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<Label>,
    /// Whether it holds a paragraph rather than a line.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub multiline: bool,
    /// What the text is, where it is more than words. Absent: words.
    #[serde(default, skip_serializing_if = "FieldShape::is_text")]
    pub shape: FieldShape,
}

/// What a field of a card holds (v0.100, ADR 0066).
///
/// Most fields are words a person reads. Three are read by the application
/// as well: a quote is somebody else's words, which reach a writer word for
/// word only once they were checked; where they were checked is a link; and
/// chips are the images, names and places a card brings, each sent to the
/// work's phrases with one click.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum FieldShape {
    #[default]
    Text,
    /// Somebody else's words, as the work writes them. In the order a
    /// writer works from only when the card's proof says where they were
    /// checked, and never from a work of a medium that is not quoted (a
    /// song's lyrics). A translation beside it is the card's own words, not
    /// a quote: the proof checks the original.
    Quote,
    /// Where the card's quotes were checked: a link, `http(s)://`. Empty is
    /// "from memory".
    Proof,
    /// Short things to keep - images, names, places, terms - one to a line
    /// or parted by semicolons; each can become a phrase of the work.
    Chips,
}

impl FieldShape {
    pub fn is_text(&self) -> bool {
        *self == FieldShape::Text
    }
}

impl CardKind {
    /// The field of this kind that holds where its quotes were checked.
    pub fn proof(&self) -> Option<&CardField> {
        self.fields
            .iter()
            .find(|field| field.shape == FieldShape::Proof)
    }

    /// Whether `key` is a field of this kind of the shape given.
    pub fn is(&self, key: &str, shape: FieldShape) -> bool {
        self.fields
            .iter()
            .any(|field| field.key == key && field.shape == shape)
    }
}

impl Board {
    /// The kind of card by its key.
    pub fn card(&self, key: &str) -> Option<&CardKind> {
        self.cards.iter().find(|kind| kind.key == key)
    }

    /// Whether any kind of card of the board links to the library.
    pub fn links_the_library(&self) -> bool {
        self.cards.iter().any(|kind| kind.library)
    }
}

/// What a kind's board of trials is for (v0.95, ADR 0061).
///
/// A trial is a text. What a kept one becomes is a version of the role named
/// here, in a work of any kind that has the role - a song's style, a
/// chapter's text - written beside the work's current version rather than
/// over it; or a brick of the dictionary. The role says how a trial is read
/// too: a role a composition writes (`compose`) is picked from the
/// dictionary, and its trials are read against it, phrase by phrase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Lab {
    /// The version role a kept trial goes into. Absent: a kept trial goes
    /// only into the dictionary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harvest: Option<String>,
    /// The field of the experiment that holds its anchors - a list, or a
    /// text read one to a line as every experiment's was before v0.112:
    /// what every trial must keep, word for word. A trial whose text has
    /// lost one is marked on the board. Absent: an experiment has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchors: Option<String>,
}
