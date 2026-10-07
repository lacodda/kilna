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
    Size,
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
        ..Cover::default()
    }
}

fn parts() -> Ingredients {
    Ingredients {
        title: "Northern Light".into(),
        background_colour: Some("#EFEBE3".into()),
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
        accent,
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
    let from = picture.find("PLACEMENT:").expect("a placement");
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
        Crop::Detail => "only the eyes or the key object",
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
                    accent: true,
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
                    accent: false,
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
