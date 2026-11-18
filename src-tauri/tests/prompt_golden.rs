//! The prompts a picture is drawn from, held to the letter (v0.115).
//!
//! A prompt is written by code from what is chosen, and a generator draws
//! what it reads: one sentence reworded, one paragraph moved, and every cover
//! made afterwards comes out different. The unit tests hold the rules - no
//! hero where there is none, no disc unless asked - but a rule says nothing
//! of a sentence it does not name, and that is where a change slips through.
//!
//! So ten pictures, each chosen for a way a prompt goes - with a hero, with a
//! thing, with nobody; the accent a colour and a disc; the title in the
//! picture, apart, and absent; a frame under a track; a scene of a clip - are
//! written out whole into `tests/golden/prompts/`, and compared here. Any
//! change to how a prompt is built shows as a diff of these files in the
//! commit that makes it, to be read before it is agreed to:
//!
//!     KILNA_BLESS=1 cargo test --test prompt_golden
//!
//! rewrites them, and removes one no picture writes any more.

use std::collections::BTreeMap;
use std::path::PathBuf;

use kilna_lib::cover::framing::{Corner, Framing, Layout, Place, Shape, Subject};
use kilna_lib::cover::prompt::{self, HeroText, Ingredients, MarkText, Target};
use kilna_lib::cover::{Accent, Cover, Ground, Hero, Lettering, MarkChoice, MarkPlace, Staging};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/prompts")
}

fn pink() -> Option<Accent> {
    Some(Accent {
        name: "hot pink".into(),
        color: "#FF2E63".into(),
        ..Accent::default()
    })
}

/// What a cover of the channel is usually built from: a style, a lettering
/// with an offer in brackets, a dressing, a dark ground, two details, bans.
fn channel() -> Ingredients {
    Ingredients {
        style: Some(
            "STYLE: neo-traditional tattoo art rendered as a detailed digital painting, bold ink outlines[, with {accent} highlights]."
                .into(),
        ),
        typography: Some(
            "TYPOGRAPHY: huge dry-brush hand lettering[, in {ink}][ with a {accent} echo].".into(),
        ),
        dressing: Some("Under the title add a small line[ \"{brand}\"][ and a year \"{year}\"].".into()),
        background: Some("BACKGROUND: perfectly flat solid ink black, no scenery.".into()),
        background_name: Some("Ink black".into()),
        background_colour: Some("#0E0D10".into()),
        details: vec![
            "a small black cat hides somewhere in the picture".into(),
            "one object in the scene is cracked".into(),
        ],
        bans: vec!["no skulls".into(), "no weapons".into()],
        channel_slots: [("brand".to_owned(), vec!["NIGHT SHIFT".to_owned()])].into(),
        title: "Northern Light".into(),
        ..Ingredients::default()
    }
}

/// Every picture, by the name of its file.
fn pictures() -> BTreeMap<&'static str, (Cover, Target, &'static str, Ingredients)> {
    let mut all = BTreeMap::new();

    // 1. A hero from the canon, the title in the picture, the accent a colour.
    all.insert(
        "01-cover-hero",
        (
            Cover {
                scene: "she polishes the lamp of a lighthouse at dusk".into(),
                avoid: "no storm, no ships".into(),
                hero: Some(Hero {
                    card: "card".into(),
                    references: true,
                }),
                framing: Some(Layout::EmblemRight.defaults()),
                accent: pink(),
                ..Cover::default()
            },
            Target::Cover,
            "16:9",
            Ingredients {
                subject: Subject::Person,
                hero: Some(HeroText {
                    description: Some("a small woman in her thirties with a red scarf".into()),
                    references: true,
                }),
                ..channel()
            },
        ),
    );

    // 2. No hero: a thing. The case the phantom hero was found on.
    all.insert(
        "02-cover-no-hero-object",
        (
            Cover {
                scene: "an old tape recorder on a kitchen table, its reels turning".into(),
                framing: Some(Framing {
                    place: Place::Behind,
                    ..Layout::Masthead.defaults()
                }),
                accent: pink(),
                ..Cover::default()
            },
            Target::Cover,
            "16:9",
            Ingredients {
                subject: Subject::Object,
                ..channel()
            },
        ),
    );

    // 3. Nobody at all: a place, the background taken as its palette.
    all.insert(
        "03-cover-no-subject",
        (
            Cover {
                scene: "an empty night street after rain, one window lit".into(),
                staging: Staging {
                    subject: Some(Subject::None),
                    ..Staging::default()
                },
                framing: Some(Framing {
                    place: Place::Overlap,
                    ..Layout::Poster.defaults()
                }),
                ..Cover::default()
            },
            Target::Cover,
            "1:1",
            Ingredients {
                subject: Subject::None,
                ..channel()
            },
        ),
    );

    // 4. The accent as a disc, the ground flat, the mark drawn in a corner.
    all.insert(
        "04-cover-disc-flat-ground",
        (
            Cover {
                scene: "he stands with a suitcase, looking up".into(),
                hero: Some(Hero {
                    card: "card".into(),
                    references: false,
                }),
                staging: Staging {
                    disc: true,
                    ground: Ground::Flat,
                    ..Staging::default()
                },
                framing: Some(Layout::Centre.defaults()),
                accent: pink(),
                mark: MarkChoice {
                    variant: Some("variant".into()),
                    place: MarkPlace::Corner,
                    corner: Corner::TopRight,
                    way: kilna_lib::cover::MarkWay::Drawn,
                },
                ..Cover::default()
            },
            Target::Cover,
            "9:16",
            Ingredients {
                subject: Subject::Person,
                hero: Some(HeroText {
                    description: Some("a tall man in a grey coat".into()),
                    references: false,
                }),
                mark: Some(MarkText {
                    prompt: "a cat's head with a tear in its eye".into(),
                    has_file: true,
                }),
                ..channel()
            },
        ),
    );

    // 5. The title apart: the picture says nothing of it.
    let apart = Cover {
        scene: "a paper boat in a gutter".into(),
        framing: Some(Layout::EmblemLeft.defaults()),
        accent: pink(),
        lettering: Lettering {
            apart: true,
            ..Lettering::default()
        },
        typography: "Keep the letters rough.".into(),
        ..Cover::default()
    };
    all.insert(
        "05-cover-title-apart",
        (apart.clone(), Target::Cover, "16:9", channel()),
    );

    // 6. Apart, with its room kept in the picture.
    all.insert(
        "06-cover-title-apart-room-kept",
        (
            Cover {
                lettering: Lettering {
                    apart: true,
                    reserve: true,
                    ..Lettering::default()
                },
                ..apart
            },
            Target::Cover,
            "16:9",
            channel(),
        ),
    );

    // 7. No title, no captions: a picture with no words, and its own words
    //    after what is built.
    all.insert(
        "07-cover-no-words",
        (
            Cover {
                scene: "a moth on a lamp".into(),
                framing: Some(Layout::CloseUp.defaults()),
                picture: "Grain like old film.".into(),
                negative: "no neon".into(),
                ..Cover::default()
            },
            Target::Cover,
            "1:1",
            Ingredients {
                title: String::new(),
                typography: None,
                dressing: None,
                ..channel()
            },
        ),
    );

    // 8. The frame a track plays under: the cover without its words, the
    //    still's own words said with the scene.
    all.insert(
        "08-frame-under-a-track",
        (
            Cover {
                scene: "she polishes the lamp of a lighthouse at dusk".into(),
                hero: Some(Hero {
                    card: "card".into(),
                    references: true,
                }),
                framing: Some(Layout::Figure.defaults()),
                accent: pink(),
                ..Cover::default()
            },
            Target::Frame,
            "16:9",
            Ingredients {
                subject: Subject::Person,
                hero: Some(HeroText {
                    description: Some("a small woman in her thirties with a red scarf".into()),
                    references: false,
                }),
                scene_words: Some("the lamp's beam turns slowly through fog".into()),
                typography: None,
                dressing: None,
                title: String::new(),
                ..channel()
            },
        ),
    );

    // 9. A scene of a clip with nobody in it.
    all.insert(
        "09-scene-no-cast",
        (
            Cover {
                scene: "a kettle boils on a stove; steam fills the frame".into(),
                framing: Some(Layout::CloseUp.defaults()),
                accent: pink(),
                ..Cover::default()
            },
            Target::Scene,
            "16:9",
            Ingredients {
                subject: Subject::Object,
                typography: None,
                dressing: None,
                background: None,
                background_name: None,
                background_colour: None,
                title: String::new(),
                ..channel()
            },
        ),
    );

    // 10. A scene with its characters.
    all.insert(
        "10-scene-cast",
        (
            Cover {
                scene: "they argue across the table".into(),
                framing: Some(Layout::Split.defaults()),
                ..Cover::default()
            },
            Target::Scene,
            "16:9",
            Ingredients {
                subject: Subject::Person,
                hero: Some(HeroText {
                    description: Some(
                        "a small woman with a red scarf Also in the picture: a tall man in a grey coat"
                            .into(),
                    ),
                    references: false,
                }),
                typography: None,
                dressing: None,
                background: None,
                background_name: None,
                background_colour: None,
                title: String::new(),
                ..channel()
            },
        ),
    );

    // 11. The layers above the style the contracts let in (v0.134): the
    //     ones that name their own heading as written, the rest under the
    //     place they fill - a pose left out by its contract is simply not
    //     among them.
    all.insert(
        "11-cover-layers",
        (
            Cover {
                scene: "she waits at the end of a pier".into(),
                framing: Some(Layout::Masthead.defaults()),
                accent: pink(),
                ..Cover::default()
            },
            Target::Cover,
            "16:9",
            Ingredients {
                subject: Subject::Person,
                layers: vec![
                    ("environment".into(), "a wooden pier over black water".into()),
                    (
                        "lighting".into(),
                        "LIGHTING: soft window light from one side, a gentle falloff into shadow on the other."
                            .into(),
                    ),
                    (
                        "optics".into(),
                        "OPTICS: an 85 mm portrait lens, a shallow depth of field.".into(),
                    ),
                    (
                        "grading".into(),
                        "GRADING: faded film: lifted blacks, soft contrast.".into(),
                    ),
                ],
                ..channel()
            },
        ),
    );

    // 12. A monochrome style with an accent: selective colour - the accent
    //     is the one colour in the picture.
    all.insert(
        "12-cover-mono-accent",
        (
            Cover {
                scene: "a moth on a lamp".into(),
                framing: Some(Layout::CloseUp.defaults()),
                accent: pink(),
                ..Cover::default()
            },
            Target::Cover,
            "1:1",
            Ingredients {
                style: Some("STYLE: 1940s film-noir still: high-contrast black and white.".into()),
                mono: true,
                ..channel()
            },
        ),
    );

    all
}

/// A loop built of the movements the contracts let in, then the person's own
/// words for what moves (v0.134).
#[test]
fn a_loop_of_movements_reads_as_it_was_agreed_to() {
    let frame = kilna_lib::work::frame::Frame {
        motion: "a curtain stirs".into(),
        seconds: 8,
        ..Default::default()
    };
    let prompts = frame.prompts_with(&[
        "MOTION: steady rain falling straight down in thin streaks; the subject and the background stay completely still.".into(),
        "MOTION: the subject blinks once every four to five seconds; the head, the body and the background do not move at all.".into(),
    ]);
    // Beside the pictures, not among them: blessing the pictures rewrites
    // their folder whole.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/loops");
    let path = dir.join("movements.txt");
    if std::env::var_os("KILNA_BLESS").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, &prompts.loop_).unwrap();
        return;
    }
    let kept = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert_eq!(
        kept, prompts.loop_,
        "bless with KILNA_BLESS=1 after reading the diff"
    );
}

/// A prompt's blocks as one page: what is pasted into each box, under its
/// name.
fn page(cover: &Cover, target: Target, shape: &str, parts: &Ingredients) -> String {
    let shape = Shape::parse(shape).expect("a shape");
    let built = prompt::build(cover, target, shape, parts);
    let mut page = format!(
        "{}\n\n--- NEGATIVE ---\n{}\n",
        built.picture, built.negative
    );
    if let Some(typography) = built.typography {
        page.push_str(&format!("\n--- SECOND PROMPT ---\n{typography}\n"));
    }
    page
}

#[test]
fn every_prompt_reads_as_it_was_agreed_to() {
    let dir = golden_dir();
    let bless = std::env::var_os("KILNA_BLESS").is_some();
    let pictures = pictures();
    assert_eq!(pictures.len(), 12);

    if bless {
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
    }
    let mut changed = Vec::new();
    for (name, (cover, target, shape, parts)) in &pictures {
        let written = page(cover, *target, shape, parts);
        let path = dir.join(format!("{name}.txt"));
        if bless {
            std::fs::write(&path, &written).unwrap();
            continue;
        }
        // Compared without the line ends a checkout may have rewritten.
        let kept = std::fs::read_to_string(&path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        if kept != written {
            changed.push(format!(
                "{name}.txt\n--- kept ---\n{kept}\n--- written now ---\n{written}"
            ));
        }
    }
    assert!(
        changed.is_empty(),
        "the prompt of {} picture(s) changed. Read the difference; if it is meant, \
         rewrite the files with `KILNA_BLESS=1 cargo test --test prompt_golden` and commit \
         them with the change.\n\n{}",
        changed.len(),
        changed.join("\n\n")
    );

    // And no file is kept for a picture nothing writes any more.
    let kept: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !pictures.contains_key(name.trim_end_matches(".txt")))
        .collect();
    assert!(kept.is_empty(), "files no picture writes: {kept:?}");
}

/// What the files are for, said about the files themselves: the picture
/// with no hero is among them and says nothing of one.
#[test]
fn the_kept_prompt_of_a_cover_with_no_hero_names_none() {
    let all = pictures();
    for name in [
        "02-cover-no-hero-object",
        "03-cover-no-subject",
        "09-scene-no-cast",
    ] {
        let (cover, target, shape, parts) = &all[name];
        let written = page(cover, *target, shape, parts).to_lowercase();
        for word in ["hero", "head", "chest", "face", "figure", "eyes", "skin"] {
            assert!(
                !written
                    .split(|c: char| !c.is_alphabetic())
                    .any(|said| said == word),
                "{name} says `{word}`:\n{written}"
            );
        }
        assert!(!written.contains("circle"), "{name}:\n{written}");
    }
}
