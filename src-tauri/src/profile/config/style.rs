//! The style dictionary: its types, forms, families and compositions.

use serde::{Deserialize, Serialize};

use super::*;

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
    /// What a brick of this type is made of beside its description, and so
    /// how its card and its editor are drawn. The application knows the forms,
    /// never the types: `typography` is the craft's word, `lettering` is ours.
    #[serde(default, skip_serializing_if = "StyleForm::is_picture")]
    pub form: StyleForm,
    /// The families a brick of this type is filed under - "tattoo", "classic"
    /// for an image style. Empty for a type the craft does not file.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub families: Vec<StyleFamily>,
    /// Set when bricks of this type are no longer made: what does their work
    /// now, in a sentence the dictionary shows in their place. The type stays
    /// in the document so the bricks written under it still read with their
    /// word; new ones are refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retired: Option<Label>,
    /// A kind of note whose cards stand in for bricks of this type: a hero
    /// with a card of the canon is described from its facts, and needs no
    /// character brick. The dictionary says so above the type's bricks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canon_kind: Option<String>,
}

impl StyleType {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            hint: None,
            icon: None,
            form: StyleForm::Picture,
            families: Vec::new(),
            retired: None,
            canon_kind: None,
        }
    }
}

/// What a style brick carries beside its name and its description.
///
/// Five shapes, because a brick is shown and edited by what it is made of:
/// pictures to recognise a look by, a live sample of a typeface, slots filled
/// with the captions of the work and the channel, and colours - a ground's
/// or an accent's, one or a gradient.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum StyleForm {
    /// Reference pictures, and a palette while there are none.
    #[default]
    Picture,
    /// A sample of the lettering drawn with a typeface of the window.
    Lettering,
    /// A description with `{slots}`, filled from captions when the brick goes
    /// into a prompt (see `style_set::fill`).
    Dressing,
    /// A ground: one colour, or a gradient of up to four in their order.
    Colour,
    /// A colour the picture leans on - one, or a gradient of up to four.
    /// Drawn and edited as a colour; a cover takes it as its accent, never
    /// as its ground (v0.90.3).
    Accent,
    /// A phrase written into a text word for word - `noise guitar bursts` in
    /// a style prompt - with nothing to show beside it but what it means
    /// (v0.94). A text is read against the phrases of the types a
    /// composition names, and assembled out of them.
    Phrase,
}

impl StyleForm {
    pub(super) fn is_picture(&self) -> bool {
        *self == Self::Picture
    }

    /// Whether a brick of this form is its colours: a ground or an accent.
    pub fn is_colour(&self) -> bool {
        matches!(self, Self::Colour | Self::Accent)
    }
}

/// A family bricks of one type are filed under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct StyleFamily {
    pub key: String,
    pub label: Label,
}

/// A text a role is written out of the dictionary (v0.94): a song's style
/// prompt, picked block by block from the types of sound.
///
/// The craft says what the text is made of and kilna writes it: the picked
/// phrases in the order they were picked - a generator that weighs its words
/// from the left reads the first as the most important - and then the work's
/// own fields as the templates write them. The same composition says which
/// texts are read against which phrases: a version of `role` is checked
/// against the bricks of `parts`, and a phrase it does not know is said to be
/// unknown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Composition {
    pub key: String,
    /// What the dictionary calls the half these types stand in: "Sound".
    pub label: Label,
    /// The version role the text is a version of.
    pub role: String,
    /// The kinds of work it writes for; every one has `role`.
    pub kinds: Vec<String>,
    /// The blocks a text is picked from, in the order they are offered.
    pub parts: Vec<CompositionPart>,
    /// The work's own fields the text closes with.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<CompositionField>,
    /// What stands between two phrases. Absent: a comma and a space.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
    /// How many characters the generator reads. A longer text is written
    /// all the same, and said to be longer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl Composition {
    /// What stands between two phrases of the text.
    pub fn separator(&self) -> &str {
        self.separator.as_deref().unwrap_or(", ")
    }

    /// Whether the text of a work of `kind` in `role` is this one.
    pub fn writes(&self, kind: &str, role: &str) -> bool {
        self.role == role && self.kinds.iter().any(|k| k == kind)
    }

    /// The types it is picked from, in their order.
    pub fn types(&self) -> impl Iterator<Item = &str> {
        self.parts.iter().map(|part| part.type_key.as_str())
    }
}

/// One block of a composition: a type of brick, and how many of it a text
/// takes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct CompositionPart {
    /// A key of `style_types`, of the form `phrase`.
    #[serde(rename = "type")]
    pub type_key: String,
    /// How many a text needs; fewer is said, not refused.
    #[serde(default)]
    pub min: u8,
    /// How many a text takes at most; more is said, not refused. Absent: any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u8>,
    /// The rule of the block in a sentence - "one mode of delivery", "the
    /// first is the lead" - shown where it is picked from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<Label>,
}

/// A field of the work a composition closes with: "92 bpm" from `bpm`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct CompositionField {
    /// A key of `work_meta_fields`.
    pub field: String,
    /// How the value is written into the text: `{value} bpm`.
    pub template: String,
}

impl CompositionField {
    /// The value as the text says it.
    pub fn write(&self, value: &str) -> String {
        self.template.replace("{value}", value)
    }
}
