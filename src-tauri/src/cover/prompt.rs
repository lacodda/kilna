//! A picture's prompt, written from what is chosen.
//!
//! Pure: everything the workspace holds - the bricks' descriptions, the
//! hero's, the channel's details, bans and captions - arrives resolved in
//! [`Ingredients`] (see [`super::read`]), so the sentences can be tested on
//! their own and the same function writes the prompt of a cover, of the
//! frame a track plays under, and of a scene of a clip.

use serde::Serialize;

use super::framing::Shape;
use super::{Cover, MarkPlace};
use crate::style_set::{self, SlotValues};

/// What a prompt is written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A publication's cover: the only picture with words on it.
    Cover,
    /// The still a track plays under, built from its cover without the words.
    Frame,
    /// A scene of a clip.
    Scene,
}

impl Target {
    pub fn lettering(self) -> bool {
        self == Target::Cover
    }

    /// The word a signature detail names the pictures it belongs in by
    /// (`canon::fact::DETAIL_PLACES`).
    pub fn place(self) -> &'static str {
        match self {
            Target::Cover => "cover",
            Target::Frame => "frame",
            Target::Scene => "scene",
        }
    }
}

/// What a prompt needs from beyond the cover, read from the workspace.
#[derive(Debug, Clone, Default)]
pub struct Ingredients {
    /// The descriptions of the chosen bricks, slots not yet filled.
    pub style: Option<String>,
    pub typography: Option<String>,
    pub dressing: Option<String>,
    pub background: Option<String>,
    /// The background's colour: what the ink is chosen against.
    pub background_colour: Option<String>,
    pub hero: Option<HeroText>,
    /// The signature details that are on for this picture, their cards'
    /// descriptions written in.
    pub details: Vec<String>,
    /// The chosen variant of the mark, when it is to be drawn.
    pub mark: Option<MarkText>,
    /// What must not appear: the channel's bans for pictures, and the hero's.
    pub bans: Vec<String>,
    /// The captions the channel's card gives a cover.
    pub channel_slots: SlotValues,
    /// The title as lettered; empty for none.
    pub title: String,
}

/// The hero, as a generator is told about them.
#[derive(Debug, Clone, Default)]
pub struct HeroText {
    /// The card's description for a generator. Absent until it is written:
    /// a model does not know who a name is, so then the hero is not said.
    pub description: Option<String>,
    /// Reference pictures go with the prompt.
    pub references: bool,
}

/// The variant of the mark the generator is asked to draw.
#[derive(Debug, Clone, Default)]
pub struct MarkText {
    pub prompt: String,
    /// A file of it goes with the prompt as a reference.
    pub has_file: bool,
}

/// The blocks of a picture's prompt, each pasted into its own box.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, ts_rs::TS)]
#[ts(rename = "CoverPrompts")]
pub struct Prompts {
    /// What to draw.
    pub picture: String,
    /// What to keep out.
    pub negative: String,
    /// The title as a second prompt, an edit of the finished picture -
    /// present when the lettering goes apart.
    pub typography: Option<String>,
}

/// The sentence the picture closes with when its words come in a second
/// prompt.
const NO_TEXT_YET: &str =
    "No text, letters or numbers in this picture, except what the scene itself asks for.";

/// The sentence a picture with no words at all closes with.
const NO_TEXT: &str = "No text, letters or numbers in this picture.";

/// Write the prompt of `cover` for `target`, in `shape`.
pub fn build(cover: &Cover, target: Target, shape: Shape, parts: &Ingredients) -> Prompts {
    // A cover written by hand is its words, and nothing is added to them.
    if target == Target::Cover && !cover.is_built() {
        return Prompts {
            picture: cover.picture.trim().to_owned(),
            negative: cover.negative.trim().to_owned(),
            typography: Some(cover.typography.trim().to_owned()).filter(|text| !text.is_empty()),
        };
    }

    let values = slot_values(cover, parts);
    let fill = |text: &str| style_set::fill(text, &values);
    let lettering = target.lettering();

    let mut picture: Vec<String> = vec![match target {
        Target::Cover => shape.sentence("cover"),
        Target::Frame => format!(
            "{} No title, no text, no logos.",
            shape.sentence("video frame that stays on screen for the whole track")
        ),
        Target::Scene => format!("{} No title, no text.", shape.sentence("video frame")),
    }];
    if let Some(framing) = &cover.framing {
        picture.push(framing.placement(lettering));
    }
    picture.extend(parts.style.as_deref().map(fill));
    if let Some(framing) = &cover.framing {
        picture.push(fill(framing.layout.composition(lettering)));
    }
    if let Some(hero) = &parts.hero
        && let Some(description) = hero.description.as_deref()
    {
        let mut line = format!("HERO: {}", sentence(description));
        if hero.references {
            line.push_str(
                " Use the attached reference pictures of the hero only for the likeness; render everything in the chosen style.",
            );
        }
        picture.push(line);
    }
    if !cover.scene.trim().is_empty() {
        picture.push(format!("SCENE: {}", sentence(&cover.scene)));
    }
    picture.extend(parts.background.as_deref().map(fill));
    picture.extend(
        parts
            .details
            .iter()
            .map(|detail| format!("DETAIL: {}", sentence(detail))),
    );
    if let Some(mark) = &parts.mark {
        picture.push(mark_line(cover, mark));
    }

    let mut typography = None;
    if lettering {
        let words = lettering_paragraph(cover, parts, &fill);
        let own = cover.typography.trim();
        if cover.lettering.apart {
            picture.push(NO_TEXT_YET.to_owned());
            let edit: Vec<String> = words
                .into_iter()
                .chain(Some(own.to_owned()).filter(|own| !own.is_empty()))
                .collect();
            if !edit.is_empty() {
                typography = Some(format!(
                    "EDIT THE IMAGE: keep the picture exactly as it is. {}",
                    edit.join(" ")
                ));
            }
        } else {
            match words {
                Some(words) => picture.push(words),
                None if own.is_empty() => picture.push(NO_TEXT.to_owned()),
                None => {}
            }
            if !own.is_empty() {
                picture.push(own.to_owned());
            }
        }
    }
    if target == Target::Cover && !cover.picture.trim().is_empty() {
        picture.push(cover.picture.trim().to_owned());
    }

    Prompts {
        picture: picture
            .into_iter()
            .map(|paragraph| paragraph.trim().to_owned())
            .filter(|paragraph| !paragraph.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n"),
        negative: negative(cover, target, parts),
        typography,
    }
}

/// The values a slot is filled from: the channel's captions, the cover's own
/// over them, and what the cover itself says - its accent, the ink its
/// background wants, its title.
fn slot_values(cover: &Cover, parts: &Ingredients) -> SlotValues {
    let mut values = parts.channel_slots.clone();
    for (slot, lines) in &cover.lettering.captions {
        let lines: Vec<String> = lines
            .iter()
            .map(|line| line.trim().to_owned())
            .filter(|line| !line.is_empty())
            .collect();
        if !lines.is_empty() {
            values.insert(slot.clone(), lines);
        }
    }
    if let Some(accent) = &cover.accent {
        values.insert("accent".into(), vec![accent.said()]);
    }
    if let Some(ground) = parts.background_colour.as_deref() {
        let ink = if super::framing::is_dark(ground) {
            "off-white"
        } else {
            "black"
        };
        values.insert("ink".into(), vec![ink.to_owned()]);
    }
    if !parts.title.trim().is_empty() {
        values.insert("title".into(), vec![parts.title.trim().to_owned()]);
    }
    values
}

/// The title, the lettering it is drawn in, and the captions around it -
/// or nothing, when the cover has no title, no lettering and no captions.
fn lettering_paragraph(
    cover: &Cover,
    parts: &Ingredients,
    fill: &dyn Fn(&str) -> String,
) -> Option<String> {
    let mut words: Vec<String> = Vec::new();
    let title = parts.title.trim();
    if !title.is_empty() {
        let script = if title.chars().any(is_cyrillic) {
            ", in Cyrillic letters"
        } else {
            ""
        };
        let setting = cover
            .framing
            .map(|framing| format!(", {}", framing.place.setting(framing.column)))
            .unwrap_or_default();
        words.push(format!(
            "TITLE: the words «{title}», spelled exactly{script}{setting}."
        ));
    }
    if let Some(typography) = parts.typography.as_deref() {
        words.push(fill(typography));
        if !title.is_empty() {
            words.push("The lettering matches the picture's style and colours.".to_owned());
        }
    }
    if let Some(dressing) = parts.dressing.as_deref() {
        words.push(fill(dressing));
    }
    if words.is_empty() {
        return None;
    }
    words.push("No other text.".to_owned());
    Some(
        words
            .into_iter()
            .filter(|word| !word.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn mark_line(cover: &Cover, mark: &MarkText) -> String {
    let place = match cover.mark.place {
        MarkPlace::Hidden => "hidden somewhere in the picture, small".to_owned(),
        _ => format!("small, in the {} corner", cover.mark.corner.phrase()),
    };
    let mut line = format!(
        "CHANNEL MARK: {}, {place}.",
        mark.prompt.trim().trim_end_matches('.')
    );
    if mark.has_file {
        line.push_str(" Match the attached mark file exactly in shape.");
    }
    line
}

/// What to keep out: what every picture of the target keeps out, the bans,
/// and the person's own words.
fn negative(cover: &Cover, target: Target, parts: &Ingredients) -> String {
    let mut out: Vec<String> = Vec::new();
    out.push(match target {
        Target::Cover => {
            let mut base = "No watermark, no signature, no frame or border.".to_owned();
            let words = !parts.title.trim().is_empty() || parts.dressing.is_some();
            if cover.lettering.apart || !words {
                base.push_str(" No text, letters or numbers.");
            } else if parts.dressing.is_some() {
                base.push_str(" No readable text except the title and the captions asked for.");
            } else {
                base.push_str(" No readable text except the title.");
            }
            base
        }
        Target::Frame => {
            "No text, no title, no logos, no watermark, no frame or border.".to_owned()
        }
        Target::Scene => "No text, no title, no watermark, no frame or border.".to_owned(),
    });
    out.extend(parts.bans.iter().map(|ban| sentence(ban)));
    if target == Target::Cover && !cover.negative.trim().is_empty() {
        out.push(cover.negative.trim().to_owned());
    }
    out.join(" ")
}

/// A piece of text as a sentence: trimmed, ending in a full stop unless it
/// ends in a mark of its own.
fn sentence(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() || text.ends_with(['.', '!', '?', '»', '"', ')']) {
        text.to_owned()
    } else {
        format!("{text}.")
    }
}

fn is_cyrillic(c: char) -> bool {
    matches!(c, '\u{0400}'..='\u{04FF}' | '\u{0500}'..='\u{052F}')
}

#[cfg(test)]
mod tests {
    use super::super::framing::{Corner, Layout, Place};
    use super::super::{Accent, Bricks, Hero, Lettering, MarkChoice, MarkWay};
    use super::*;

    fn wide() -> Shape {
        Shape::parse("16:9").unwrap()
    }

    fn built() -> Cover {
        Cover {
            framing: Some(Layout::EmblemRight.defaults()),
            scene: "a lighthouse keeper polishes the lamp".into(),
            ..Cover::default()
        }
    }

    fn parts() -> Ingredients {
        Ingredients {
            title: "Northern Light".into(),
            ..Ingredients::default()
        }
    }

    #[test]
    fn a_cover_written_by_hand_is_its_words_exactly() {
        let cover = Cover {
            picture: "  a lantern on dark water ".into(),
            negative: "no people".into(),
            typography: String::new(),
            ..Cover::default()
        };
        let prompts = build(&cover, Target::Cover, wide(), &parts());
        assert_eq!(prompts.picture, "a lantern on dark water");
        assert_eq!(prompts.negative, "no people");
        assert_eq!(prompts.typography, None);

        let lettered = Cover {
            typography: "the title in white".into(),
            ..cover
        };
        assert_eq!(
            build(&lettered, Target::Cover, wide(), &parts()).typography,
            Some("the title in white".into())
        );
    }

    /// The details of the channel do not reach a cover written by hand: it
    /// keeps the prompt it was written with.
    #[test]
    fn nothing_is_added_to_a_cover_written_by_hand() {
        let cover = Cover {
            picture: "a lantern".into(),
            ..Cover::default()
        };
        let with_details = Ingredients {
            details: vec!["a hidden cat".into()],
            bans: vec!["no skulls".into()],
            ..parts()
        };
        let prompts = build(&cover, Target::Cover, wide(), &with_details);
        assert_eq!(prompts.picture, "a lantern");
        assert_eq!(prompts.negative, "");
    }

    #[test]
    fn a_built_cover_says_its_frame_placement_scene_and_title_in_order() {
        let prompts = build(&built(), Target::Cover, wide(), &parts());
        let picture = &prompts.picture;
        let order = [
            "FRAME: a wide 16:9 cover.",
            "PLACEMENT: place the hero in the right third of the frame, vertically centred.",
            "COMPOSITION:",
            "SCENE: a lighthouse keeper polishes the lamp.",
            "TITLE: the words «Northern Light», spelled exactly, set as a tall left-aligned block",
            "No other text.",
        ];
        let mut from = 0;
        for part in order {
            let at = picture[from..]
                .find(part)
                .unwrap_or_else(|| panic!("`{part}` after {from} in:\n{picture}"));
            from += at + part.len();
        }
        assert!(
            !picture.contains("in Cyrillic"),
            "a Latin title says no script"
        );
        assert_eq!(
            prompts.typography, None,
            "the title is inside the picture by default"
        );
    }

    #[test]
    fn a_cyrillic_title_is_said_to_be_one() {
        let cyrillic = Ingredients {
            title: "Северный свет".into(),
            ..parts()
        };
        let prompts = build(&built(), Target::Cover, wide(), &cyrillic);
        assert!(
            prompts
                .picture
                .contains("«Северный свет», spelled exactly, in Cyrillic letters")
        );
    }

    #[test]
    fn the_title_apart_is_a_second_prompt_and_the_picture_stays_clean() {
        let cover = Cover {
            lettering: Lettering {
                apart: true,
                ..Lettering::default()
            },
            typography: "tight letter spacing".into(),
            ..built()
        };
        let prompts = build(&cover, Target::Cover, wide(), &parts());
        assert!(!prompts.picture.contains("TITLE:"));
        assert!(
            prompts.picture.ends_with(NO_TEXT_YET),
            "{}",
            prompts.picture
        );
        let edit = prompts.typography.expect("a second prompt");
        assert!(edit.starts_with("EDIT THE IMAGE: keep the picture exactly as it is."));
        assert!(edit.contains("«Northern Light»"));
        assert!(
            edit.ends_with("tight letter spacing"),
            "own words close it: {edit}"
        );
        assert!(prompts.negative.contains("No text, letters or numbers."));
    }

    #[test]
    fn a_cover_with_no_title_and_no_captions_asks_for_no_text() {
        let untitled = Ingredients {
            title: String::new(),
            ..parts()
        };
        let prompts = build(&built(), Target::Cover, wide(), &untitled);
        assert!(prompts.picture.ends_with(NO_TEXT), "{}", prompts.picture);
        assert!(!prompts.picture.contains("TITLE:"));
    }

    /// The bricks fill their slots from the cover: the accent it chose, the
    /// ink its background wants, its own captions over the channel's.
    #[test]
    fn slots_are_filled_from_the_cover_before_the_channel() {
        let cover = Cover {
            accent: Some(Accent {
                name: "hot pink".into(),
                color: "#FF2E63".into(),
            }),
            lettering: Lettering {
                captions: [("brand".to_owned(), vec!["OWN".to_owned()])].into(),
                ..Lettering::default()
            },
            bricks: Bricks {
                typography: Some("t".into()),
                dressing: Some("d".into()),
                ..Bricks::default()
            },
            ..built()
        };
        let with_bricks = Ingredients {
            typography: Some(
                "TYPOGRAPHY: brush letters[, in {ink}][ with a {accent} echo].".into(),
            ),
            dressing: Some("Add a line[ \"{brand}\"][ and \"{motto}\"].".into()),
            background_colour: Some("#0E0D10".into()),
            channel_slots: [
                ("brand".to_owned(), vec!["CHANNEL".to_owned()]),
                ("motto".to_owned(), vec!["keep going".to_owned()]),
            ]
            .into(),
            ..parts()
        };
        let picture = build(&cover, Target::Cover, wide(), &with_bricks).picture;
        assert!(
            picture.contains("brush letters, in off-white with a hot pink (#FF2E63) echo."),
            "{picture}"
        );
        assert!(
            picture.contains("Add a line \"OWN\" and \"keep going\"."),
            "{picture}"
        );
        assert!(picture.contains("A large flat circle in hot pink (#FF2E63)"));
    }

    #[test]
    fn without_an_accent_its_phrases_drop_out() {
        let picture = build(&built(), Target::Cover, wide(), &parts()).picture;
        assert!(!picture.contains("circle"), "{picture}");
        assert!(!picture.contains("{accent}"));
    }

    #[test]
    fn a_hero_without_a_description_is_not_named() {
        let cover = Cover {
            hero: Some(Hero {
                card: "c".into(),
                references: true,
            }),
            ..built()
        };
        let undescribed = Ingredients {
            hero: Some(HeroText {
                description: None,
                references: true,
            }),
            ..parts()
        };
        assert!(
            !build(&cover, Target::Cover, wide(), &undescribed)
                .picture
                .contains("HERO:")
        );

        let described = Ingredients {
            hero: Some(HeroText {
                description: Some("a grey cat with one green eye".into()),
                references: true,
            }),
            ..parts()
        };
        let picture = build(&cover, Target::Cover, wide(), &described).picture;
        assert!(
            picture.contains("HERO: a grey cat with one green eye. Use the attached reference")
        );
    }

    #[test]
    fn a_drawn_mark_is_asked_for_where_it_goes() {
        let cover = Cover {
            mark: MarkChoice {
                variant: Some("m".into()),
                place: MarkPlace::Corner,
                corner: Corner::TopLeft,
                way: MarkWay::Drawn,
            },
            ..built()
        };
        let with_mark = Ingredients {
            mark: Some(MarkText {
                prompt: "a cat head with a crossed-out eye.".into(),
                has_file: true,
            }),
            ..parts()
        };
        let picture = build(&cover, Target::Cover, wide(), &with_mark).picture;
        assert!(
            picture.contains("CHANNEL MARK: a cat head with a crossed-out eye, small, in the upper left corner. Match the attached mark file exactly in shape."),
            "{picture}"
        );
    }

    #[test]
    fn bans_and_own_words_close_the_negative() {
        let cover = Cover {
            negative: "no blood".into(),
            ..built()
        };
        let banned = Ingredients {
            bans: vec!["no skulls".into(), "No crowns.".into()],
            ..parts()
        };
        let negative = build(&cover, Target::Cover, wide(), &banned).negative;
        assert!(negative.starts_with("No watermark"));
        assert!(
            negative.ends_with("no skulls. No crowns. no blood"),
            "{negative}"
        );
    }

    /// A frame and a scene carry no words: no title, no part of the frame
    /// kept for one, no lettering - and none of the cover's own words, which
    /// were written for a picture with a title on it.
    #[test]
    fn a_frame_and_a_scene_carry_no_words() {
        let cover = Cover {
            framing: Some(Framing {
                place: Place::Left,
                ..Layout::Split.defaults()
            }),
            picture: "the title in gold".into(),
            negative: "no gold".into(),
            ..built()
        };
        for target in [Target::Frame, Target::Scene] {
            let prompts = build(&cover, target, wide(), &parts());
            assert!(!prompts.picture.contains("TITLE"), "{target:?}");
            assert!(!prompts.picture.contains("title goes there"), "{target:?}");
            assert!(!prompts.picture.contains("vertical title"), "{target:?}");
            assert!(!prompts.picture.contains("in gold"), "{target:?}");
            assert!(!prompts.negative.contains("no gold"), "{target:?}");
            assert_eq!(prompts.typography, None);
            assert!(
                prompts
                    .picture
                    .starts_with("FRAME: a wide 16:9 video frame")
            );
        }
    }

    use super::super::framing::Framing;
}
