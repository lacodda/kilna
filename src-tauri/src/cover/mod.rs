//! The cover of a publication: a concept the application builds a prompt from.
//!
//! A clip, an audio release or a short goes out under a picture, and the
//! picture is drawn by a generator from a prompt. The owner built those
//! prompts by hand from a page of parts - a style, a lettering, a dressing of
//! small captions, a background, a layout, the channel's signature details -
//! and lost the choices between sessions. Here the choices are the record:
//! the idea, the scene, the hero, the built frame, the bricks of the style
//! dictionary, the mark, the details, and the prompt is written from them in
//! code (ADR 0049).
//!
//! One record per publication, in `work.cover`. Its parts that are words -
//! the idea, the scene and the person's own words for each block - stand at
//! the top of the JSON, so the search finds them as it found the blocks
//! written before v0.88; everything else is nested.
//!
//! The blocks a cover had before v0.88 (`picture`, `negative`, `typography`)
//! are its own words now, written after whatever is built. A cover with
//! nothing chosen in the constructor is one written by hand, and its prompt
//! is those words exactly: the 132 covers written before the constructor
//! read as they did.

#[cfg(test)]
mod agreement;
pub mod framing;
pub mod prompt;
pub mod read;

use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::profile::config::StyleForm;
pub use framing::{Corner, Framing, Scheme, Shape};

/// A publication's cover, as the constructor holds it.
///
/// Read from `{}` it is a cover with nothing in it, and every part has a
/// value: the window sends the whole cover back with each change, so the
/// log's `before` holds it whole and an undo puts it back, as the frame does.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct Cover {
    /// What the cover says, in the person's own words and language. Never
    /// sent to a generator: it is what the scene is written from.
    pub idea: String,
    /// What the picture shows, in the generator's English.
    pub scene: String,
    /// The hero from the canon. Absent, the hero is the one the scene
    /// describes.
    pub hero: Option<Hero>,
    /// The built frame. Absent until a layout is picked.
    pub framing: Option<Framing>,
    pub bricks: Bricks,
    /// The colour the composition and the lettering lean on.
    pub accent: Option<Accent>,
    pub lettering: Lettering,
    pub mark: MarkChoice,
    /// The channel's signature details this cover turned on or off, by the
    /// fact's id. A detail not named here is as the channel says.
    pub details: BTreeMap<String, bool>,
    /// The person's own words, written after the built picture prompt.
    pub picture: String,
    /// The person's own words, written after the built negative.
    pub negative: String,
    /// The person's own words, written after the built lettering.
    pub typography: String,
    /// The prompt as it was last copied out - what the picture was drawn
    /// from, kept when the channel or a brick changes afterwards.
    pub sent: Option<Sent>,
}

/// The hero of a cover: a card of the canon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
#[ts(rename = "CoverHero")]
pub struct Hero {
    /// The card's id.
    pub card: String,
    /// Whether the card's reference pictures go to the generator with the
    /// prompt - and the prompt says what they are for.
    pub references: bool,
}

impl Default for Hero {
    fn default() -> Self {
        Self {
            card: String::new(),
            references: true,
        }
    }
}

/// The bricks of the style dictionary a cover is built from, by id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
#[ts(rename = "CoverBricks")]
pub struct Bricks {
    /// How the picture is drawn: a brick of a type made of pictures.
    pub style: Option<String>,
    /// How the title is lettered: a brick of a lettering type.
    pub typography: Option<String>,
    /// The small captions around the title: a brick of a dressing type.
    pub dressing: Option<String>,
    /// The ground: a brick of a colour type.
    pub background: Option<String>,
}

impl Bricks {
    /// Each brick with the form its place asks for.
    pub fn chosen(&self) -> impl Iterator<Item = (&str, StyleForm)> {
        [
            (self.style.as_deref(), StyleForm::Picture),
            (self.typography.as_deref(), StyleForm::Lettering),
            (self.dressing.as_deref(), StyleForm::Dressing),
            (self.background.as_deref(), StyleForm::Colour),
        ]
        .into_iter()
        .filter_map(|(id, form)| id.map(|id| (id, form)))
    }

    fn any(&self) -> bool {
        self.chosen().next().is_some()
    }
}

/// An accent colour and the words a generator is given for it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
#[ts(rename = "CoverAccent")]
pub struct Accent {
    /// "hot pink". May be empty: the colour alone is then said.
    pub name: String,
    /// `#RRGGBB`.
    pub color: String,
}

impl Accent {
    /// As a prompt says it: "hot pink (#FF2E63)", or the colour alone.
    pub fn said(&self) -> String {
        let name = self.name.trim();
        if name.is_empty() {
            self.color.clone()
        } else {
            format!("{name} ({})", self.color)
        }
    }
}

/// The words on a cover.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
#[ts(rename = "CoverLettering")]
pub struct Lettering {
    /// The title as it is lettered. Absent, it is the title of what the
    /// publication is made from - the song, not "the song - clip"; empty, the
    /// cover has no title.
    pub title: Option<String>,
    /// The title goes to the generator as a second prompt, an edit of the
    /// finished picture, rather than inside the picture's prompt.
    pub apart: bool,
    /// The cover's own captions for the dressing's slots, by slot name. A
    /// slot not named here is filled from the channel's card.
    pub captions: BTreeMap<String, Vec<String>>,
}

/// The channel's mark on a cover.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
#[ts(rename = "CoverMark")]
pub struct MarkChoice {
    /// The variant of the family, by its fact's id: the status that fits the
    /// song. Absent, no mark is chosen yet.
    pub variant: Option<String>,
    pub place: MarkPlace,
    /// Which corner, when it sits in one.
    pub corner: Corner,
    pub way: MarkWay,
}

impl MarkChoice {
    /// How the mark is put on: a mark hidden in the picture can only be
    /// drawn into it.
    pub fn way(&self) -> MarkWay {
        if self.place == MarkPlace::Hidden {
            MarkWay::Drawn
        } else {
            self.way
        }
    }

    /// The corner the mark sits in - drawn there or laid over - when a
    /// variant is chosen and it sits in one.
    pub fn in_corner(&self) -> Option<Corner> {
        (self.variant.is_some() && self.place == MarkPlace::Corner).then_some(self.corner)
    }

    /// The corner the exact file is laid over at export, when it is.
    pub fn laid_over(&self) -> Option<Corner> {
        self.in_corner().filter(|_| self.way() == MarkWay::Overlay)
    }

    /// Whether the generator is asked to draw the mark.
    pub fn drawn(&self) -> bool {
        self.variant.is_some() && self.place != MarkPlace::None && self.way() == MarkWay::Drawn
    }
}

/// Where a mark goes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "CoverMarkPlace")]
pub enum MarkPlace {
    #[default]
    Corner,
    /// Small, somewhere inside the picture.
    Hidden,
    /// No mark on this cover.
    None,
}

/// How a mark gets onto the picture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "CoverMarkWay")]
pub enum MarkWay {
    /// The exact file, laid over the finished picture when it is exported.
    /// Generators distort a logo; a mark has to be exact.
    #[default]
    Overlay,
    /// Described in the prompt and handed over as a reference, for the
    /// generator to draw in the picture's style.
    Drawn,
}

/// A prompt as it was copied out.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
#[ts(rename = "CoverSent")]
pub struct Sent {
    pub picture: String,
    pub negative: String,
    pub typography: Option<String>,
    pub at: String,
}

impl Cover {
    /// Whether anything is chosen in the constructor. A cover that only
    /// holds words was written by hand, and its prompt is those words: the
    /// channel's details are not added to it, nor anything else.
    pub fn is_built(&self) -> bool {
        self.framing.is_some()
            || self.bricks.any()
            || self.hero.is_some()
            || !self.scene.trim().is_empty()
            || self.accent.is_some()
            || self.mark.variant.is_some()
            || !self.details.is_empty()
            || self.lettering != Lettering::default()
    }

    /// Whether the cover holds anything at all - what makes a work show its
    /// Cover tab although its kind has none.
    pub fn holds_anything(&self) -> bool {
        self.is_built()
            || !self.idea.trim().is_empty()
            || !self.picture.trim().is_empty()
            || !self.negative.trim().is_empty()
            || !self.typography.trim().is_empty()
    }

    /// The card it names as its hero, if any.
    pub fn hero_card(&self) -> Option<&str> {
        self.hero
            .as_ref()
            .map(|hero| hero.card.as_str())
            .filter(|id| !id.is_empty())
    }
}

/// Refuse a cover that names what is not there, or a brick in a place its
/// form does not fill. A cover that names a brick deleted since is read
/// all the same - the prompt leaves it out and the window says so - but a
/// new write of a name that does not exist is a mistake caught at the door.
pub fn check(conn: &Connection, profile_id: &str, cover: &Cover) -> Result<()> {
    let config = crate::profile::config_for(conn, profile_id)?;
    for (id, form) in cover.bricks.chosen() {
        let brick = crate::style_brick::get(conn, id)?
            .filter(|brick| brick.profile_id == profile_id)
            .ok_or_else(|| Error::refused("cover.unknownBrick").param("id", id))?;
        let actual = config
            .style_types
            .iter()
            .find(|kind| kind.key == brick.type_key)
            .map(|kind| kind.form);
        if actual != Some(form) {
            return Err(Error::refused("cover.brickOfOtherForm").param("name", brick.name));
        }
    }
    if let Some(card) = cover.hero_card() {
        let note = crate::note::get(conn, card)?
            .filter(|note| note.profile_id == profile_id)
            .ok_or_else(|| Error::refused("cover.unknownHero").param("id", card))?;
        if config.card_kind(&note.kind).is_none() {
            return Err(
                Error::refused("cover.heroNotACard").param("title", note.title.unwrap_or_default())
            );
        }
    }
    if let Some(accent) = &cover.accent
        && !crate::canon::fact::is_hex_colour(accent.color.trim())
    {
        return Err(Error::refused("cover.accentNotHex").param("value", accent.color.clone()));
    }
    if let Some(variant) = cover.mark.variant.as_deref() {
        let fact = crate::canon::fact::get(conn, variant)?
            .filter(|fact| fact.profile_id == profile_id)
            .ok_or_else(|| Error::refused("cover.unknownMark").param("id", variant))?;
        let shape = crate::canon::fact::card_of(conn, &config, &fact.note_id)
            .ok()
            .and_then(|(_, kind)| kind.section(&fact.section).map(|section| section.shape));
        if shape != Some(crate::profile::config::SectionShape::Marks) {
            return Err(Error::refused("cover.markNotAVariant"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cover written before v0.88 was three blocks of text. It reads as a
    /// cover written by hand: the same words, nothing chosen, nothing built.
    #[test]
    fn a_cover_written_before_the_constructor_reads_as_its_own_words() {
        let cover: Cover = serde_json::from_str(
            r#"{"picture": "a lantern on dark water", "negative": "no people", "typography": "the title in white"}"#,
        )
        .unwrap();
        assert_eq!(cover.picture, "a lantern on dark water");
        assert_eq!(cover.negative, "no people");
        assert_eq!(cover.typography, "the title in white");
        assert!(!cover.is_built());
        assert!(cover.holds_anything());
    }

    #[test]
    fn a_cover_read_from_nothing_holds_nothing() {
        let cover: Cover = serde_json::from_str("{}").unwrap();
        assert_eq!(cover, Cover::default());
        assert!(!cover.is_built());
        assert!(!cover.holds_anything());
        assert_eq!(cover.mark.place, MarkPlace::Corner);
        assert_eq!(cover.mark.way, MarkWay::Overlay);
    }

    /// Each choice of the constructor makes it a built cover, and the words
    /// alone - the idea included - do not.
    #[test]
    fn any_choice_makes_a_cover_built() {
        let built = [
            Cover {
                framing: Some(framing::Layout::Poster.defaults()),
                ..Cover::default()
            },
            Cover {
                bricks: Bricks {
                    background: Some("b".into()),
                    ..Bricks::default()
                },
                ..Cover::default()
            },
            Cover {
                hero: Some(Hero {
                    card: "c".into(),
                    references: true,
                }),
                ..Cover::default()
            },
            Cover {
                scene: "a lighthouse".into(),
                ..Cover::default()
            },
            Cover {
                lettering: Lettering {
                    apart: true,
                    ..Lettering::default()
                },
                ..Cover::default()
            },
        ];
        for cover in built {
            assert!(cover.is_built(), "{cover:?}");
        }
        let written = Cover {
            idea: "a lighthouse keeper's last night".into(),
            picture: "a lighthouse".into(),
            ..Cover::default()
        };
        assert!(!written.is_built());
    }

    /// The words stand at the top of the JSON, everything else is nested -
    /// the search reads the top level's text and nothing below it.
    #[test]
    fn only_words_stand_at_the_top_of_the_record() {
        let cover = Cover {
            idea: "an idea".into(),
            hero: Some(Hero {
                card: "card-id".into(),
                references: true,
            }),
            framing: Some(framing::Layout::Figure.defaults()),
            bricks: Bricks {
                style: Some("brick-id".into()),
                ..Bricks::default()
            },
            ..Cover::default()
        };
        let value = serde_json::to_value(&cover).unwrap();
        let texts: Vec<&str> = value
            .as_object()
            .unwrap()
            .values()
            .filter_map(|value| value.as_str())
            .collect();
        assert_eq!(texts, ["an idea", "", "", "", ""]);
    }

    #[test]
    fn a_hidden_mark_can_only_be_drawn() {
        let mark = MarkChoice {
            variant: Some("v".into()),
            place: MarkPlace::Hidden,
            way: MarkWay::Overlay,
            ..MarkChoice::default()
        };
        assert_eq!(mark.way(), MarkWay::Drawn);
        assert!(mark.drawn());
        assert_eq!(mark.in_corner(), None);
        assert_eq!(mark.laid_over(), None);
        let corner = MarkChoice {
            variant: Some("v".into()),
            ..MarkChoice::default()
        };
        assert_eq!(corner.laid_over(), Some(Corner::BottomRight));
        assert!(!corner.drawn());
        let none = MarkChoice {
            variant: Some("v".into()),
            place: MarkPlace::None,
            way: MarkWay::Drawn,
            ..MarkChoice::default()
        };
        assert!(!none.drawn());
        assert_eq!(none.in_corner(), None);
    }
}
