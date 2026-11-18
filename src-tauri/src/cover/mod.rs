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
//! Before it is settled, a cover is looked for on a board of ideas - each
//! a concept of this same shape, kept in [`idea`] (ADR 0050).
//!
//! One record per publication, in `work.cover`. Its parts that are words -
//! the idea, the scene, what it keeps out and the person's own words for
//! each block - stand at the top of the JSON, so the search finds them as it
//! found the blocks written before v0.88; everything else is nested.
//!
//! The blocks a cover had before v0.88 (`picture`, `negative`, `typography`)
//! are its own words now, written after whatever is built. A cover with
//! nothing chosen in the constructor is one written by hand, and its prompt
//! is those words exactly: the 132 covers written before the constructor
//! read as they did.

#[cfg(test)]
mod agreement;
pub mod contract;
pub mod framing;
pub mod idea;
pub mod prompt;
pub mod read;

use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::profile::config::StyleForm;
pub use framing::{Corner, Framing, Scheme, Shape, Subject};

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
    /// What this picture must keep out beyond the channel's bans, in the
    /// generator's English: what the scene risks - a filter's trigger, a
    /// detail the song's lines invite and the channel does not show. Part of
    /// the idea: it goes into the constructor with the scene and into the
    /// negative after the bans, where the person's own words come last.
    pub avoid: String,
    /// The hero from the canon. Absent, the picture is of what
    /// [`subject`](Self::subject) says.
    pub hero: Option<Hero>,
    /// How the picture is staged: what it is of when no hero is chosen, and
    /// what the accent and the background are to it (v0.115). Nested, like
    /// everything that is not words: the search reads the words at the top
    /// of the record.
    pub staging: Staging,
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
    /// The layers of a picture above its style (v0.134, FR-07 7.2), each a
    /// brick of the type of its own name: how the frame is composed, where
    /// it is, what the hero wears, how they stand, the angle, what is over
    /// what, the light, the lens, the grade.
    #[ts(optional)]
    pub composition: Option<String>,
    #[ts(optional)]
    pub look: Option<String>,
    #[ts(optional)]
    pub environment: Option<String>,
    #[ts(optional)]
    pub angle: Option<String>,
    #[ts(optional)]
    pub pose: Option<String>,
    #[ts(optional)]
    pub layering: Option<String>,
    #[ts(optional)]
    pub lighting: Option<String>,
    #[ts(optional)]
    pub optics: Option<String>,
    #[ts(optional)]
    pub grading: Option<String>,
}

/// The layers, in the order a prompt says them - and the contracts are read
/// in, after the style and the ground: the frame and the place before the
/// hero, the hero before the light, the light before the lens and the grade.
pub const LAYERS: [&str; 9] = [
    "composition",
    "environment",
    "look",
    "pose",
    "angle",
    "layering",
    "lighting",
    "optics",
    "grading",
];

impl Bricks {
    /// These bricks over `base`: each place this set leaves empty is the
    /// base's.
    pub fn over(self, base: &Bricks) -> Bricks {
        Bricks {
            style: self.style.or_else(|| base.style.clone()),
            typography: self.typography.or_else(|| base.typography.clone()),
            dressing: self.dressing.or_else(|| base.dressing.clone()),
            background: self.background.or_else(|| base.background.clone()),
            composition: self.composition.or_else(|| base.composition.clone()),
            look: self.look.or_else(|| base.look.clone()),
            environment: self.environment.or_else(|| base.environment.clone()),
            angle: self.angle.or_else(|| base.angle.clone()),
            pose: self.pose.or_else(|| base.pose.clone()),
            layering: self.layering.or_else(|| base.layering.clone()),
            lighting: self.lighting.or_else(|| base.lighting.clone()),
            optics: self.optics.or_else(|| base.optics.clone()),
            grading: self.grading.or_else(|| base.grading.clone()),
        }
    }

    /// The chosen layers, each with its place, in [`LAYERS`]' order.
    pub fn layers(&self) -> impl Iterator<Item = (&'static str, &str)> {
        [
            self.composition.as_deref(),
            self.environment.as_deref(),
            self.look.as_deref(),
            self.pose.as_deref(),
            self.angle.as_deref(),
            self.layering.as_deref(),
            self.lighting.as_deref(),
            self.optics.as_deref(),
            self.grading.as_deref(),
        ]
        .into_iter()
        .zip(LAYERS)
        .filter_map(|(id, place)| id.map(|id| (place, id)))
    }

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
        self.chosen().next().is_some() || self.layers().next().is_some()
    }
}

/// An accent colour and the words a generator is given for it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
#[ts(rename = "CoverAccent")]
pub struct Accent {
    /// "hot pink". May be empty: the colour alone is then said.
    pub name: String,
    /// `#RRGGBB`; the first stop of a gradient, which is what a swatch or a
    /// scheme that can hold one colour shows.
    pub color: String,
    /// A gradient's colours in their order, from an accent of the dictionary
    /// (v0.90.3). Empty, or one colour, for a flat accent.
    pub stops: Vec<String>,
}

impl Accent {
    /// An accent of the dictionary, as a cover keeps it: its name and its
    /// colours, copied, the way a colour of the channel's palette is.
    pub fn of_brick(brick: &crate::style_brick::StyleBrick) -> Option<Self> {
        let color = brick.colours.first()?.clone();
        Some(Self {
            name: brick.name.clone(),
            color,
            stops: if brick.colours.len() > 1 {
                brick.colours.clone()
            } else {
                Vec::new()
            },
        })
    }

    /// As a prompt says it: "hot pink (#FF2E63)", "Sunset (a gradient of
    /// #FA8072 to #7A5CFF)", or the colour alone.
    pub fn said(&self) -> String {
        let colour = if self.stops.len() > 1 {
            format!("a gradient of {}", self.stops.join(" to "))
        } else {
            self.color.clone()
        };
        let name = self.name.trim();
        if name.is_empty() {
            colour
        } else {
            format!("{name} ({colour})")
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
    /// With the title apart, the picture still keeps the title's part of the
    /// frame clean for it (v0.115). Off, the first prompt says nothing of a
    /// title at all and the second finds its own room.
    pub reserve: bool,
    /// The cover's own captions for the dressing's slots, by slot name. A
    /// slot not named here is filled from the channel's card.
    pub captions: BTreeMap<String, Vec<String>>,
}

/// How a picture is staged.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
#[ts(rename = "CoverStaging")]
pub struct Staging {
    /// What the picture is of when no hero is chosen: a person the scene
    /// describes, a thing, or nobody. Absent, a thing: the frame then says
    /// "the main subject" and never a head, a chest or eyes. A cover with a
    /// hero is of a person whatever this says.
    pub subject: Option<Subject>,
    /// The accent is a flat disc behind the subject, rather than a colour
    /// for highlights and edges. Off unless asked for: until v0.115 five
    /// layouts of the eight put a moon behind every picture with an accent.
    pub disc: bool,
    /// What the chosen background is to the picture.
    pub ground: Ground,
}

/// What a chosen background is to the picture (v0.115).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "CoverGround")]
pub enum Ground {
    /// Its colours and its mood: the scene keeps the place it describes.
    #[default]
    Palette,
    /// The whole ground, flat: a studio backdrop with no scenery - what a
    /// background's description says, as it is written.
    Flat,
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
            || !self.avoid.trim().is_empty()
            || self.accent.is_some()
            || self.staging != Staging::default()
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

    /// What the picture is of: a person when a hero is chosen, else what the
    /// cover says, else a thing.
    pub fn subject_of(&self) -> Subject {
        if self.hero_card().is_some() {
            Subject::Person
        } else {
            self.staging.subject.unwrap_or_default()
        }
    }

    /// The card it names as its hero, if any.
    pub fn hero_card(&self) -> Option<&str> {
        self.hero
            .as_ref()
            .map(|hero| hero.card.as_str())
            .filter(|id| !id.is_empty())
    }
}

/// Refuse layers or movements that name what is not there or a brick of the
/// wrong kind (v0.134): each layer a brick of the type of its own name, each
/// movement a brick of the form `motion`. For a frame's and a scene's own
/// bricks as for a cover's.
pub fn check_layers(
    conn: &Connection,
    profile_id: &str,
    bricks: Option<&Bricks>,
    movements: &[String],
) -> Result<()> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let named = |id: &str| -> Result<crate::style_brick::StyleBrick> {
        crate::style_brick::get(conn, id)?
            .filter(|brick| brick.profile_id == profile_id)
            .ok_or_else(|| Error::refused("cover.unknownBrick").param("id", id))
    };
    if let Some(bricks) = bricks {
        for (place, id) in bricks.layers() {
            let brick = named(id)?;
            if brick.type_key != place {
                return Err(Error::refused("cover.brickOfOtherForm").param("name", brick.name));
            }
        }
    }
    if movements.len() > crate::work::frame::MOST_MOVEMENTS {
        return Err(Error::refused("cover.tooManyMovements")
            .param("most", crate::work::frame::MOST_MOVEMENTS as i64));
    }
    for id in movements {
        let brick = named(id)?;
        let form = config.style_type(&brick.type_key).map(|kind| kind.form);
        if form != Some(StyleForm::Motion) {
            return Err(Error::refused("cover.notAMovement").param("name", brick.name));
        }
    }
    Ok(())
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
    // A layer takes a brick of the type of its own name: a pose is a pose.
    check_layers(conn, profile_id, Some(&cover.bricks), &[])?;
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
                avoid: "a drowned body".into(),
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
        // In the keys' order: avoid, idea, negative, picture, scene, typography.
        assert_eq!(texts, ["", "an idea", "", "", "", ""]);
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
