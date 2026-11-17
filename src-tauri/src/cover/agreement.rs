//! The scheme and the prompt do not disagree (v0.88).
//!
//! The scheme beside a cover's prompt is only worth looking at if what it
//! shows is what the prompt asks for. Both are written from the same
//! settings, and this test holds them to each other the hard way: for every
//! layout, place of the title, size, way of framing the hero, position and
//! shape - with an accent and without - it reads the numbers and the words
//! back out of the PROMPT TEXT and checks them against the SHAPES of the
//! scheme. A title block drawn outside the part of the frame the prompt keeps
//! for it, a hero drawn in another third than the prompt names, a size the
//! prompt states as 58% drawn at 80%, a disc of the accent the prompt never
//! asks for - each fails here.

use super::framing::{
    Column, Corner, Crop, Framing, Layout, Paint, Place, Rect, Row, Scheme, SchemeShape, Shape,
    Size, Subject,
};
use super::prompt::{self, Ingredients, MarkText, Target};
use super::{Accent, Cover, MarkChoice, MarkPlace, MarkWay};

const SHAPES: [&str; 3] = ["16:9", "9:16", "1:1"];

/// What the test knows about the words, stated apart from the code that
/// writes them: the phrase for each third, read back to a third.
fn third_named(prompt: &str) -> usize {
    let phrases = [
        "in the left third of the frame",
        "in the centre of the frame",
        "in the right third of the frame",
    ];
    let found: Vec<usize> = (0..3).filter(|i| prompt.contains(phrases[*i])).collect();
    assert_eq!(found.len(), 1, "one third is named: {prompt}");
    found[0]
}

/// The whole numbers that follow `lead` in `text`, up to the next `%`.
fn percent_after(text: &str, lead: &str) -> f64 {
    let at = text
        .find(lead)
        .unwrap_or_else(|| panic!("`{lead}` in: {text}"));
    let rest = &text[at + lead.len()..];
    let digits: String = rest
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    digits
        .parse::<f64>()
        .unwrap_or_else(|_| panic!("a number after `{lead}`: {text}"))
        / 100.0
}

/// Every whole percent in `text`, in order.
fn percents(text: &str) -> Vec<f64> {
    let mut out = Vec::new();
    let mut digits = String::new();
    for c in text.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else {
            if c == '%' && !digits.is_empty() {
                out.push(digits.parse::<f64>().unwrap() / 100.0);
            }
            digits.clear();
        }
    }
    out
}

/// The title's blocks the scheme draws.
fn title_blocks(scheme: &Scheme) -> Vec<Rect> {
    scheme
        .shapes
        .iter()
        .filter_map(|shape| match shape {
            SchemeShape::Rect {
                rect,
                paint: Paint::Ink,
            } => Some(*rect),
            _ => None,
        })
        .collect()
}

/// A box around everything painted as the hero.
fn figure_bounds(scheme: &Scheme) -> Rect {
    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    for shape in &scheme.shapes {
        match shape {
            SchemeShape::Rect {
                rect,
                paint: Paint::Figure,
            } => {
                xs.extend([rect.x, rect.x + rect.w]);
                ys.extend([rect.y, rect.y + rect.h]);
            }
            SchemeShape::Circle {
                cx,
                cy,
                r,
                paint: Paint::Figure,
            } => {
                xs.extend([cx - r, cx + r]);
                ys.extend([cy - r, cy + r]);
            }
            SchemeShape::Ellipse {
                cx,
                cy,
                rx,
                ry,
                paint: Paint::Figure,
            } => {
                xs.extend([cx - rx, cx + rx]);
                ys.extend([cy - ry, cy + ry]);
            }
            SchemeShape::Path {
                d,
                paint: Paint::Figure,
            } => {
                let numbers: Vec<f64> = d
                    .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
                    .filter(|piece| !piece.is_empty())
                    .map(|piece| piece.parse().unwrap())
                    .collect();
                for pair in numbers.chunks(2) {
                    xs.push(pair[0]);
                    ys.push(pair[1]);
                }
            }
            _ => {}
        }
    }
    assert!(!xs.is_empty(), "the hero is drawn");
    let min = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
    let max = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Rect {
        x: min(&xs),
        y: min(&ys),
        w: max(&xs) - min(&xs),
        h: max(&ys) - min(&ys),
    }
}

fn has_disc(scheme: &Scheme) -> bool {
    scheme.shapes.iter().any(|shape| {
        matches!(
            shape,
            SchemeShape::Circle {
                paint: Paint::Accent,
                ..
            }
        )
    })
}

/// The part of the frame the prompt keeps for the title, read back from its
/// words as shares of the width and the height - one rectangle, or two for
/// a poster's top and bottom lines.
fn zones_said(prompt: &str, place: Place) -> Vec<Rect> {
    let rect = |x: f64, y: f64, w: f64, h: f64| Rect { x, y, w, h };
    match place {
        Place::Left => {
            let share = percent_after(prompt, "Keep the left ");
            vec![rect(0.0, 0.0, share, 1.0)]
        }
        Place::Right => {
            let share = percent_after(prompt, "Keep the right ");
            vec![rect(1.0 - share, 0.0, share, 1.0)]
        }
        Place::Top => {
            let share = percent_after(prompt, "Keep the top ");
            vec![rect(0.0, 0.0, 1.0, share)]
        }
        Place::Bottom => {
            let share = percent_after(prompt, "Keep the bottom ");
            vec![rect(0.0, 1.0 - share, 1.0, share)]
        }
        Place::Behind => {
            let share = percent_after(prompt, "across the top ");
            vec![rect(0.0, 0.0, 1.0, share)]
        }
        Place::Overlap => {
            let at = prompt.find("The title crosses the hero").unwrap();
            let both = percents(&prompt[at..]);
            vec![rect(0.0, both[0], 1.0, both[1] - both[0])]
        }
        Place::Vertical => {
            if prompt.contains("Keep the right ") {
                let share = percent_after(prompt, "Keep the right ");
                vec![rect(1.0 - share, 0.0, share, 1.0)]
            } else {
                let share = percent_after(prompt, "Keep the left ");
                vec![rect(0.0, 0.0, share, 1.0)]
            }
        }
        Place::Corner => {
            let at = prompt.find("Keep the lower left corner").unwrap();
            let both = percents(&prompt[at..]);
            vec![rect(0.0, 1.0 - both[1], both[0], both[1])]
        }
        Place::Poster => {
            let at = prompt.find("Leave the top ").unwrap();
            let both = percents(&prompt[at..]);
            vec![
                rect(0.0, 0.0, 1.0, both[0]),
                rect(0.0, 1.0 - both[1], 1.0, both[1]),
            ]
        }
    }
}

fn cover_with(framing: Framing, accent: bool) -> Cover {
    Cover {
        scene: "a lighthouse keeper polishes the lamp".into(),
        framing: Some(framing),
        accent: accent.then(|| Accent {
            name: "hot pink".into(),
            color: "#FF2E63".into(),
            ..Accent::default()
        }),
        // The accent as a disc: the scheme draws one when, and only when,
        // the prompt asks for it.
        staging: super::Staging {
            disc: accent,
            ..super::Staging::default()
        },
        ..Cover::default()
    }
}

fn parts() -> Ingredients {
    Ingredients {
        title: "Northern Light".into(),
        background_colour: Some("#EFEBE3".into()),
        // A person: the subject the scheme draws as a figure, whose head and
        // shoulders the checks below measure.
        subject: Subject::Person,
        ..Ingredients::default()
    }
}

/// Every combination of the six settings, in every shape, with an accent
/// and without: the prompt's words and the scheme's shapes agree.
#[test]
fn the_scheme_and_the_prompt_agree_in_every_combination() {
    let mut checked = 0;
    for raw in SHAPES {
        let shape = Shape::parse(raw).unwrap();
        for layout in Layout::ALL {
            for place in Place::ALL {
                for size in Size::ALL {
                    for crop in Crop::ALL {
                        for column in Column::ALL {
                            for row in Row::ALL {
                                for accent in [false, true] {
                                    let framing = Framing {
                                        layout,
                                        column,
                                        row,
                                        size,
                                        crop,
                                        place,
                                    };
                                    agree(shape, framing, accent);
                                    checked += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(checked, 3 * 8 * 9 * 6 * 5 * 3 * 3 * 2);
}

fn agree(shape: Shape, framing: Framing, accent: bool) {
    let cover = cover_with(framing, accent);
    let parts = parts();
    let picture = prompt::build(&cover, Target::Cover, shape, &parts).picture;
    let colours = super::framing::Colours::on(Some("#EFEBE3"), Some("#FF2E63"));
    let scheme = framing.scheme(super::framing::Drawing {
        shape,
        lettering: true,
        colours: &colours,
        subject: parts.subject,
        disc: accent,
        mark: None,
    });
    let at = format!("{framing:?} in {} with accent {accent}", shape.name());
    let (width, height) = (scheme.width, scheme.height);

    // Where the hero stands, across.
    let centre = (scheme.hero.x + scheme.hero.w / 2.0) / width;
    let third = ((centre * 3.0).floor() as usize).min(2);
    assert_eq!(third_named(&picture), third, "{at}: the third");

    // Where the hero sits, down.
    let top = scheme.hero.y / height;
    let bottom = (scheme.hero.y + scheme.hero.h) / height;
    if picture.contains("near the top") {
        assert!(top < 0.1, "{at}: near the top, drawn at {top}");
    } else if picture.contains("vertically centred") {
        let middle = (top + bottom) / 2.0;
        assert!(
            (middle - 0.5).abs() < 0.01,
            "{at}: centred, drawn at {middle}"
        );
    } else if picture.contains("anchored to the bottom edge") {
        assert!(
            (bottom - 1.0).abs() < 0.01,
            "{at}: on the bottom, drawn to {bottom}"
        );
    } else {
        panic!("{at}: no row is named");
    }

    // How big: the sentence after where the hero stands, before how much of
    // them shows (the crop's sentence opens with "Show").
    let drawn = scheme.hero.h / height;
    let from = picture.find("FRAMING:").expect("a framing");
    let to = from + picture[from..].find(" Show ").expect("a crop");
    let sizing = &picture[from..to];
    if sizing.contains("An extreme close-up") {
        assert!(drawn > 1.5, "{at}: an extreme close-up drawn at {drawn}");
    } else {
        let said = ["% of the frame height", "% of its height"]
            .iter()
            .find_map(|tail| {
                let end = sizing.find(tail)?;
                let digits: String = sizing[..end]
                    .chars()
                    .rev()
                    .take_while(char::is_ascii_digit)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                digits.parse::<f64>().ok().map(|n| n / 100.0)
            })
            .unwrap_or_else(|| panic!("{at}: no size is said"));
        assert!(
            (said - drawn).abs() < 0.006,
            "{at}: said {said}, drawn {drawn}"
        );
    }

    // How much of the hero: the shapes painted as the hero stay in their box.
    let bounds = figure_bounds(&scheme);
    let slack = 0.06;
    assert!(
        bounds.x >= scheme.hero.x - slack
            && bounds.y >= scheme.hero.y - slack
            && bounds.x + bounds.w <= scheme.hero.x + scheme.hero.w + slack
            && bounds.y + bounds.h <= scheme.hero.y + scheme.hero.h + slack,
        "{at}: the hero drawn at {bounds:?} out of its box {:?}",
        scheme.hero
    );
    let crop_words = match framing.crop {
        Crop::Full => "full figure",
        Crop::Waist => "from the waist up",
        Crop::Bust => "head-and-shoulders",
        Crop::Head => "only the hero's head",
        Crop::Detail => "only the hero's eyes",
    };
    assert!(picture.contains(crop_words), "{at}: the crop is said");

    // Where the title goes: every block inside the part the prompt keeps.
    let zones: Vec<Rect> = zones_said(&picture, framing.place)
        .into_iter()
        .map(|zone| Rect {
            x: zone.x * width,
            y: zone.y * height,
            w: zone.w * width,
            h: zone.h * height,
        })
        .collect();
    let blocks = title_blocks(&scheme);
    assert!(!blocks.is_empty(), "{at}: a title is drawn");
    for block in &blocks {
        assert!(
            zones.iter().any(|zone| zone.holds(block)),
            "{at}: the title block {block:?} is outside {zones:?}"
        );
    }

    // The accent's disc: drawn when, and only when, the prompt asks for it.
    assert_eq!(
        has_disc(&scheme),
        picture.contains("circle in hot pink"),
        "{at}: the disc"
    );
}

/// A picture with no words - a frame under a track, a scene of a clip - has
/// no title drawn and none asked for, in every combination.
#[test]
fn a_picture_without_words_draws_and_asks_for_no_title() {
    let shape = Shape::parse("16:9").unwrap();
    for layout in Layout::ALL {
        for place in Place::ALL {
            for column in Column::ALL {
                let framing = Framing {
                    place,
                    column,
                    ..layout.defaults()
                };
                let cover = cover_with(framing, true);
                for target in [Target::Frame, Target::Scene] {
                    let picture = prompt::build(&cover, target, shape, &parts()).picture;
                    assert!(
                        !picture.contains("TITLE") && !picture.contains("title goes there"),
                        "{framing:?} {target:?}: {picture}"
                    );
                }
                let colours = super::framing::Colours::on(None, Some("#FF2E63"));
                let scheme = framing.scheme(super::framing::Drawing {
                    shape,
                    lettering: false,
                    colours: &colours,
                    subject: Subject::Person,
                    disc: true,
                    mark: None,
                });
                assert!(title_blocks(&scheme).is_empty(), "{framing:?}");
                assert!(scheme.zones.is_empty());
            }
        }
    }
}

/// The mark: drawn in the corner the prompt names, and laid over the picture
/// in the corner the scheme shows - in every shape and every corner.
#[test]
fn the_mark_sits_where_the_prompt_says() {
    for raw in SHAPES {
        let shape = Shape::parse(raw).unwrap();
        for corner in Corner::ALL {
            for way in [MarkWay::Drawn, MarkWay::Overlay] {
                let mark = MarkChoice {
                    variant: Some("variant".into()),
                    place: MarkPlace::Corner,
                    corner,
                    way,
                };
                let cover = Cover {
                    mark: mark.clone(),
                    ..cover_with(Layout::Centre.defaults(), false)
                };
                let drawn = Ingredients {
                    mark: mark.drawn().then(|| MarkText {
                        prompt: "a cat's head".into(),
                        has_file: true,
                    }),
                    ..parts()
                };
                let picture = prompt::build(&cover, Target::Cover, shape, &drawn).picture;
                let colours = super::framing::Colours::on(None, None);
                let scheme = Layout::Centre.defaults().scheme(super::framing::Drawing {
                    shape,
                    lettering: true,
                    colours: &colours,
                    subject: Subject::Person,
                    disc: false,
                    mark: cover.mark.in_corner(),
                });
                let boxed = scheme.mark.expect("the scheme shows where the mark goes");
                let right = boxed.x + boxed.w / 2.0 > scheme.width / 2.0;
                let low = boxed.y + boxed.h / 2.0 > scheme.height / 2.0;
                let at = format!("{corner:?} {way:?} in {raw}");
                match way {
                    MarkWay::Drawn => {
                        let words = format!(
                            "in the {} {} corner",
                            if low { "lower" } else { "upper" },
                            if right { "right" } else { "left" }
                        );
                        assert!(picture.contains(&words), "{at}: {picture}");
                    }
                    MarkWay::Overlay => {
                        assert!(
                            !picture.contains("CHANNEL MARK"),
                            "{at}: a mark laid over at export is not asked of the generator"
                        );
                        assert_eq!(cover.mark.laid_over(), Some(corner));
                    }
                }
            }
        }
    }
}

/// The words a frame may say only of a person (FR-07 7.1).
const OF_A_PERSON: [&str; 7] = ["hero", "head", "chest", "face", "figure", "eyes", "skin"];

/// The words of `text`, lower case: a word is said when it stands whole, so
/// "headline" does not say "head".
fn words_of(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphabetic())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// A picture with no hero never speaks of one (v0.115).
///
/// Until then every frame said "the hero" - its head behind the title, its
/// chest under it, its eyes in a close-up - whatever the picture was of, and
/// a cover of an empty street was told where the hero's face goes. For a
/// thing and for a picture with no subject, in every layout, place of the
/// title, size, crop and position, on a cover, a frame and a scene, with the
/// accent as a colour and as a disc, with the title in the picture and
/// apart: none of the words of a person, in any block of the prompt.
#[test]
fn a_picture_with_no_hero_never_speaks_of_one() {
    let shape = Shape::parse("16:9").unwrap();
    let mut checked = 0;
    for subject in [Subject::Object, Subject::None] {
        for layout in Layout::ALL {
            for place in Place::ALL {
                for size in Size::ALL {
                    for crop in Crop::ALL {
                        for column in Column::ALL {
                            for (disc, apart) in [(false, false), (true, true)] {
                                let framing = Framing {
                                    place,
                                    size,
                                    crop,
                                    column,
                                    ..layout.defaults()
                                };
                                let mut cover = cover_with(framing, true);
                                cover.staging.subject = Some(subject);
                                cover.staging.disc = disc;
                                cover.lettering.apart = apart;
                                cover.lettering.reserve = apart;
                                let parts = Ingredients {
                                    subject: cover.subject_of(),
                                    ..parts()
                                };
                                for target in [Target::Cover, Target::Frame, Target::Scene] {
                                    let built = prompt::build(&cover, target, shape, &parts);
                                    let said = format!(
                                        "{}\n{}",
                                        built.picture,
                                        built.typography.unwrap_or_default()
                                    );
                                    let words = words_of(&said);
                                    for banned in OF_A_PERSON {
                                        assert!(
                                            !words.iter().any(|word| word == banned),
                                            "{subject:?} {framing:?} {target:?} says `{banned}`:\n{said}"
                                        );
                                    }
                                    checked += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(checked, 2 * 8 * 9 * 6 * 5 * 3 * 2 * 3);
}

/// The other half: a person is still spoken of as one, so the check above
/// is about the subject and not about words that went missing everywhere.
#[test]
fn a_person_is_still_spoken_of_as_one() {
    let shape = Shape::parse("16:9").unwrap();
    let framing = Framing {
        place: Place::Behind,
        ..Layout::Masthead.defaults()
    };
    let picture =
        prompt::build(&cover_with(framing, false), Target::Cover, shape, &parts()).picture;
    assert!(picture.contains("place the hero"), "{picture}");
    assert!(picture.contains("behind the hero's head"), "{picture}");
}

/// The accent is a colour unless a disc is asked for, and the frame says
/// nothing of what the picture holds: no circle by default, and none of the
/// motifs every frame used to add.
#[test]
fn a_frame_is_geometry_and_the_accent_a_colour() {
    let shape = Shape::parse("16:9").unwrap();
    for layout in Layout::ALL {
        let mut cover = cover_with(layout.defaults(), true);
        cover.staging.disc = false;
        let picture = prompt::build(&cover, Target::Cover, shape, &parts()).picture;
        assert!(!picture.contains("circle"), "{layout:?}: {picture}");
        assert!(
            picture.contains("Accent colour: hot pink (#FF2E63), used for highlights"),
            "{layout:?}: {picture}"
        );
        for motif in [
            "symbolic",
            "splatters",
            "doodles",
            "moon",
            "marks on the skin",
        ] {
            assert!(
                !picture.contains(motif),
                "{layout:?} says `{motif}`: {picture}"
            );
        }
        assert!(!picture.contains("PLACEMENT:") && !picture.contains("COMPOSITION:"));
        assert_eq!(picture.matches("FRAMING:").count(), 1, "{picture}");

        // Asked for, the disc is said and drawn - in every layout, not five.
        cover.staging.disc = true;
        let picture = prompt::build(&cover, Target::Cover, shape, &parts()).picture;
        assert!(
            picture.contains("circle in hot pink"),
            "{layout:?}: {picture}"
        );
    }
}

/// A close look at a pair of eyes is drawn inside the frame: until v0.115
/// the largest size drew them at twice the frame's height.
#[test]
fn a_close_look_is_drawn_inside_the_frame() {
    let shape = Shape::parse("16:9").unwrap();
    let colours = super::framing::Colours::on(None, None);
    let framing = Framing {
        size: Size::Macro,
        crop: Crop::Detail,
        ..Layout::CloseUp.defaults()
    };
    let scheme = framing.scheme(super::framing::Drawing {
        shape,
        lettering: false,
        colours: &colours,
        subject: Subject::Person,
        disc: false,
        mark: None,
    });
    for drawn in &scheme.shapes {
        if let SchemeShape::Ellipse { cy, ry, cx, rx, .. } = drawn {
            assert!(
                cy - ry >= -0.01 && cy + ry <= scheme.height + 0.01,
                "an eye runs out of the frame: {drawn:?}"
            );
            assert!(
                cx - rx >= -0.01 && cx + rx <= scheme.width + 0.01,
                "{drawn:?}"
            );
        }
    }
}
