//! The frame a picture is built in: where its subject stands, how big, how
//! much of it shows, and where the title goes.
//!
//! The subject is a person, an object, or nobody (v0.115, [`Subject`]): a
//! frame says "the hero" only of a person, and of a picture with no subject
//! it places nothing and frames the scene. Until then every sentence said
//! "the hero" - its head, its chest, its eyes - whatever the picture showed,
//! and a cover of an empty street was told where to put the hero's face.
//!
//! A frame is geometry and nothing else: where, how big, how much shows. The
//! sentences that told every picture to be an emblem of interlocked symbolic
//! objects with splatters around it went with v0.115 - what is in the
//! picture is the scene's to say, and how it is drawn is the style's.
//!
//! Built into the code rather than kept as bricks (the owner's decision of
//! 2026-09-27: "the composition is built in and not edited"). Eight layouts,
//! a position on a three by three grid, six steps of size, five ways to frame
//! the hero and nine places for the title. Each of them says its sentence of
//! the prompt AND draws its part of the scheme from the same numbers - the
//! zone a title is kept to is one rectangle that both the blocks of the scheme
//! and the "keep the left 40%" of the prompt are written from - so the scheme
//! cannot show a picture the prompt does not ask for (ADR 0049). The window
//! draws the shapes it is sent and computes none of them.

use serde::{Deserialize, Serialize};

/// The built frame of a picture: a layout and the five settings it starts
/// from, each of which can then be moved on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
pub struct Framing {
    pub layout: Layout,
    /// Which third of the frame the hero stands in, left to right.
    pub column: Column,
    /// How the hero sits in the frame, top to bottom.
    pub row: Row,
    pub size: Size,
    /// How much of the hero shows.
    pub crop: Crop,
    /// Where the title goes. Read only where a picture carries words: a
    /// frame under a track and a scene of a clip carry none.
    pub place: Place,
}

/// What a picture is of, which is what a frame places.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "CoverSubject")]
pub enum Subject {
    /// A person, or a creature drawn as one: the hero.
    Person,
    /// A thing: the main subject, with no head, chest or eyes to speak of.
    #[default]
    Object,
    /// Nobody and nothing in particular: the scene itself.
    None,
}

impl Subject {
    pub const ALL: [Subject; 3] = [Subject::Person, Subject::Object, Subject::None];

    /// What the frame's sentences call the subject. A picture with none has
    /// no word for it: its sentences are written apart.
    fn noun(self) -> &'static str {
        match self {
            Subject::Person => "the hero",
            Subject::Object | Subject::None => "the main subject",
        }
    }
}

/// The eight layouts. Picking one sets the five settings to its own; each can
/// then be moved, and the layout still says how the objects are composed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "CoverLayout")]
pub enum Layout {
    /// An emblem in the right third, the title on the left.
    EmblemRight,
    /// An emblem in the left third, the title on the right.
    EmblemLeft,
    /// An emblem in the middle, the whole height of the frame.
    Centre,
    /// A loud full-bleed poster, the hero bigger than the frame.
    Poster,
    /// A magazine cover: the title behind the hero's head.
    Masthead,
    /// The face, or the key object, fills the frame.
    CloseUp,
    /// A small lonely figure in a vast space.
    Figure,
    /// The hero in one half, a vertical title in the other.
    Split,
}

impl Layout {
    pub const ALL: [Layout; 8] = [
        Layout::EmblemRight,
        Layout::EmblemLeft,
        Layout::Centre,
        Layout::Poster,
        Layout::Masthead,
        Layout::CloseUp,
        Layout::Figure,
        Layout::Split,
    ];

    /// The frame a layout starts from.
    pub fn defaults(self) -> Framing {
        let (column, row, size, crop, place) = match self {
            Layout::EmblemRight => (
                Column::Right,
                Row::Middle,
                Size::Emblem,
                Crop::Bust,
                Place::Left,
            ),
            Layout::EmblemLeft => (
                Column::Left,
                Row::Middle,
                Size::Emblem,
                Crop::Bust,
                Place::Right,
            ),
            Layout::Centre => (
                Column::Centre,
                Row::Bottom,
                Size::Full,
                Crop::Waist,
                Place::Top,
            ),
            Layout::Poster => (
                Column::Centre,
                Row::Bottom,
                Size::Over,
                Crop::Bust,
                Place::Poster,
            ),
            Layout::Masthead => (
                Column::Centre,
                Row::Bottom,
                Size::Full,
                Crop::Bust,
                Place::Behind,
            ),
            Layout::CloseUp => (
                Column::Centre,
                Row::Middle,
                Size::Macro,
                Crop::Head,
                Place::Corner,
            ),
            Layout::Figure => (
                Column::Centre,
                Row::Bottom,
                Size::Small,
                Crop::Full,
                Place::Top,
            ),
            Layout::Split => (
                Column::Left,
                Row::Bottom,
                Size::Full,
                Crop::Waist,
                Place::Vertical,
            ),
        };
        Framing {
            layout: self,
            column,
            row,
            size,
            crop,
            place,
        }
    }

    /// How the frame is composed: its geometry, and nothing about what is in
    /// it. `open` is what a split frame's other half is left to.
    fn composition(self, subject: Subject, lettering: bool) -> String {
        let n = subject.noun();
        let other_half = if lettering {
            "is left to a huge vertical title"
        } else {
            "stays open"
        };
        if subject == Subject::None {
            return match self {
                Layout::EmblemRight | Layout::EmblemLeft | Layout::Centre => {
                    "One compact composition: the elements of the scene are grouped together; nothing floats separately.".to_owned()
                }
                Layout::Poster => {
                    "A dense full-bleed composition that runs to the frame edges.".to_owned()
                }
                Layout::Masthead => {
                    "A magazine-cover layout: the scene is centred and fills the frame.".to_owned()
                }
                Layout::CloseUp => {
                    "An extreme close-up of a part of the scene, cropped by the frame edges.".to_owned()
                }
                Layout::Figure => "A vast, open view with a lot of empty space.".to_owned(),
                Layout::Split => format!(
                    "A split frame: the scene fills one half of the frame; the other half {other_half}."
                ),
            };
        }
        match self {
            Layout::EmblemRight | Layout::EmblemLeft | Layout::Centre => format!(
                "One compact group: {n} and what surrounds it overlap and frame each other; nothing floats separately."
            ),
            Layout::Poster => format!(
                "A dense full-bleed poster: {n} dominates the frame, and what surrounds it overlaps the frame edges."
            ),
            Layout::Masthead => {
                format!("A magazine-cover layout: {n} stands in front, large and centred.")
            }
            Layout::CloseUp => {
                format!("An extreme close-up: {n} fills the frame and is cropped by its edges.")
            }
            Layout::Figure => {
                format!("A vast space: {n} is small in it, surrounded by a lot of background.")
            }
            Layout::Split => format!(
                "A split frame: {n} fills one half of the frame; the other half {other_half}."
            ),
        }
    }
}

/// Which third of the frame the hero stands in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "CoverColumn")]
pub enum Column {
    Left,
    Centre,
    Right,
}

impl Column {
    pub const ALL: [Column; 3] = [Column::Left, Column::Centre, Column::Right];

    /// Where the middle of the hero stands, as a share of the width.
    fn centre(self) -> f64 {
        match self {
            Column::Left => 0.26,
            Column::Centre => 0.5,
            Column::Right => 0.74,
        }
    }

    fn phrase(self) -> &'static str {
        match self {
            Column::Left => "in the left third of the frame",
            Column::Centre => "in the centre of the frame",
            Column::Right => "in the right third of the frame",
        }
    }
}

/// How the hero sits in the frame, top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "CoverRow")]
pub enum Row {
    Top,
    Middle,
    Bottom,
}

impl Row {
    pub const ALL: [Row; 3] = [Row::Top, Row::Middle, Row::Bottom];

    fn phrase(self) -> &'static str {
        match self {
            Row::Top => "near the top",
            Row::Middle => "vertically centred",
            Row::Bottom => "anchored to the bottom edge",
        }
    }
}

/// How big the hero is, as a share of the frame's height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "CoverSize")]
pub enum Size {
    Small,
    Emblem,
    Large,
    Full,
    /// Bigger than the frame: cropped by its edges.
    Over,
    /// The face or the key object fills the frame.
    Macro,
}

impl Size {
    pub const ALL: [Size; 6] = [
        Size::Small,
        Size::Emblem,
        Size::Large,
        Size::Full,
        Size::Over,
        Size::Macro,
    ];

    /// The hero's height as a share of the frame's.
    pub fn share(self) -> f64 {
        match self {
            Size::Small => 0.36,
            Size::Emblem => 0.58,
            Size::Large => 0.8,
            Size::Full => 1.0,
            Size::Over => 1.25,
            Size::Macro => 1.9,
        }
    }

    /// The sentence. Every size but the last names its share as a percent,
    /// which is what the test holds the scheme's subject to.
    fn sentence(self, subject: Subject) -> String {
        let percent = percent(self.share());
        let name = subject.noun();
        // The sentence opens with the name.
        let n = format!("T{}", &name[1..]);
        match self {
            Size::Small => format!(
                "{n} is small, about {percent}% of the frame height, with a lot of space around."
            ),
            Size::Emblem => {
                format!("{n} with what surrounds it takes about {percent}% of the frame height.")
            }
            Size::Large => format!("{n} is large, about {percent}% of the frame height."),
            Size::Full => format!(
                "{n} fills {percent}% of the frame height, from the bottom edge almost to the top."
            ),
            Size::Over => format!(
                "{n} is bigger than the frame, about {percent}% of its height, and is cropped by the frame edges."
            ),
            Size::Macro => format!(
                "An extreme close-up: {name} fills the whole frame and is cropped on all sides."
            ),
        }
    }

    /// How much of a scene with no subject the frame takes in.
    fn view(self) -> &'static str {
        match self {
            Size::Small | Size::Emblem => "A wide view of the entire scene.",
            Size::Large | Size::Full => "A medium view of the scene.",
            Size::Over => "A close view of the scene, cropped by the frame edges.",
            Size::Macro => "An extreme close-up of a detail of the scene.",
        }
    }
}

/// How much of the hero shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "CoverCrop")]
pub enum Crop {
    /// Head to feet.
    Full,
    /// From the waist up.
    Waist,
    /// Head and shoulders.
    Bust,
    /// The head alone.
    Head,
    /// The eyes, or the key object.
    Detail,
}

impl Crop {
    pub const ALL: [Crop; 5] = [
        Crop::Full,
        Crop::Waist,
        Crop::Bust,
        Crop::Head,
        Crop::Detail,
    ];

    /// The width of the hero's box against its height.
    fn aspect(self) -> f64 {
        match self {
            Crop::Full => 0.42,
            Crop::Waist => 0.78,
            Crop::Bust => 1.0,
            Crop::Head => 0.82,
            Crop::Detail => 2.0,
        }
    }

    /// How much shows. A person is said by their body; a thing has none, and
    /// is said by how much of it is in the frame. A picture with no subject
    /// has nothing to crop.
    fn sentence(self, subject: Subject) -> &'static str {
        match (subject, self) {
            (Subject::None, _) => "",
            (Subject::Person, Crop::Full) => "Show the hero in full figure, head to feet.",
            (Subject::Person, Crop::Waist) => "Show the hero from the waist up.",
            (Subject::Person, Crop::Bust) => "Show the hero as a head-and-shoulders bust.",
            (Subject::Person, Crop::Head) => "Show only the hero's head, very large.",
            (Subject::Person, Crop::Detail) => "Show only the hero's eyes, extremely close.",
            (Subject::Object, Crop::Full) => "Show the main subject whole.",
            (Subject::Object, Crop::Waist) => "Show most of the main subject.",
            (Subject::Object, Crop::Bust) => "Show the upper part of the main subject.",
            (Subject::Object, Crop::Head) => "Show one part of the main subject, very large.",
            (Subject::Object, Crop::Detail) => {
                "Show only a key detail of the main subject, extremely close."
            }
        }
    }

    /// Where the head is in the hero's box, and how big against the box's
    /// height - where the accent's disc sits behind it.
    fn head(self) -> (f64, f64) {
        match self {
            Crop::Full => (0.085, 0.16),
            Crop::Waist => (0.2, 0.3),
            Crop::Bust => (0.3, 0.36),
            Crop::Head => (0.45, 0.5),
            Crop::Detail => (0.5, 0.4),
        }
    }
}

/// The nine places a title can go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "CoverPlace")]
pub enum Place {
    Left,
    Right,
    Top,
    Bottom,
    /// Huge, behind the hero's head.
    Behind,
    /// Across the hero's chest, in front of the figure.
    Overlap,
    /// A vertical title in the side the hero leaves open.
    Vertical,
    /// Small, in the lower left corner.
    Corner,
    /// Huge across the top and a second line at the bottom.
    Poster,
}

impl Place {
    pub const ALL: [Place; 9] = [
        Place::Left,
        Place::Right,
        Place::Top,
        Place::Bottom,
        Place::Behind,
        Place::Overlap,
        Place::Vertical,
        Place::Corner,
        Place::Poster,
    ];

    /// The parts of the frame the title is kept to, as shares of its width
    /// and height. A vertical title takes the side the hero leaves.
    pub fn zones(self, hero: Column) -> Vec<Rect> {
        match self {
            Place::Left => vec![Rect::new(0.0, 0.0, 0.4, 1.0)],
            Place::Right => vec![Rect::new(0.6, 0.0, 0.4, 1.0)],
            Place::Top => vec![Rect::new(0.0, 0.0, 1.0, 0.25)],
            Place::Bottom => vec![Rect::new(0.0, 0.75, 1.0, 0.25)],
            Place::Behind => vec![Rect::new(0.0, 0.0, 1.0, 0.4)],
            Place::Overlap => vec![Rect::new(0.0, 0.5, 1.0, 0.3)],
            Place::Vertical => match vertical_side(hero) {
                Side::Right => vec![Rect::new(0.6, 0.0, 0.4, 1.0)],
                Side::Left => vec![Rect::new(0.0, 0.0, 0.4, 1.0)],
            },
            Place::Corner => vec![Rect::new(0.0, 0.78, 0.35, 0.22)],
            Place::Poster => vec![
                Rect::new(0.0, 0.0, 1.0, 0.25),
                Rect::new(0.0, 0.8, 1.0, 0.2),
            ],
        }
    }

    /// The blocks the scheme draws for the title, as shares of the frame.
    /// Each lies inside a zone of [`zones`](Self::zones) - the test says so.
    fn blocks(self, hero: Column) -> Vec<Rect> {
        match self {
            Place::Left => vec![
                Rect::new(0.05, 0.16, 0.34, 0.13),
                Rect::new(0.05, 0.32, 0.28, 0.13),
                Rect::new(0.05, 0.51, 0.2, 0.025),
            ],
            Place::Right => vec![
                Rect::new(0.61, 0.16, 0.34, 0.13),
                Rect::new(0.67, 0.32, 0.28, 0.13),
                Rect::new(0.75, 0.51, 0.2, 0.025),
            ],
            Place::Top => vec![
                Rect::new(0.1, 0.05, 0.8, 0.15),
                Rect::new(0.3, 0.215, 0.4, 0.025),
            ],
            Place::Bottom => vec![
                Rect::new(0.1, 0.77, 0.8, 0.13),
                Rect::new(0.3, 0.92, 0.4, 0.025),
            ],
            Place::Behind => vec![Rect::new(0.03, 0.1, 0.94, 0.28)],
            Place::Overlap => vec![Rect::new(0.08, 0.56, 0.84, 0.2)],
            Place::Vertical => {
                let left = match vertical_side(hero) {
                    Side::Right => 0.64,
                    Side::Left => 0.06,
                };
                (0..3)
                    .map(|bar| Rect::new(left + f64::from(bar) * 0.1, 0.08, 0.07, 0.84))
                    .collect()
            }
            Place::Corner => vec![Rect::new(0.05, 0.84, 0.26, 0.08)],
            Place::Poster => vec![
                Rect::new(0.06, 0.03, 0.88, 0.2),
                Rect::new(0.18, 0.85, 0.64, 0.09),
            ],
        }
    }

    /// Whether the title is drawn behind the hero - the head overlaps it.
    fn behind_hero(self) -> bool {
        matches!(self, Place::Behind | Place::Poster)
    }

    /// The sentence that keeps the title's part of the frame for it, written
    /// from the zone's own numbers.
    fn space(self, hero: Column, subject: Subject) -> String {
        let zones = self.zones(hero);
        let first = zones[0];
        let n = subject.noun();
        let some = subject != Subject::None;
        match self {
            Place::Left | Place::Right => format!(
                "Keep the {} {}% of the frame {}: the title goes there.",
                if self == Place::Left { "left" } else { "right" },
                percent(first.w),
                if some {
                    format!("free of {n} and what surrounds it")
                } else {
                    "calm and empty".to_owned()
                }
            ),
            Place::Top => format!(
                "Keep the top {}% of the frame calm: the title goes there.",
                percent(first.h)
            ),
            Place::Bottom => format!(
                "Keep the bottom {}% of the frame calm: the title goes there.",
                percent(first.h)
            ),
            Place::Behind => format!(
                "Keep a wide calm band across the top {}% of the frame{} for a giant title.",
                percent(first.h),
                match subject {
                    Subject::Person => ", behind the hero's head,",
                    Subject::Object => ", behind the main subject,",
                    Subject::None => "",
                }
            ),
            Place::Overlap => format!(
                "The title crosses {} between {}% and {}% of the frame height, so keep that band simple.",
                if some { n } else { "the picture" },
                percent(first.y),
                percent(first.y + first.h)
            ),
            Place::Vertical => format!(
                "Keep the {} {}% of the frame free for a vertical title running top to bottom.",
                vertical_side(hero).word(),
                percent(first.w)
            ),
            Place::Corner => format!(
                "Keep the lower left corner, {}% of the width and the bottom {}% of the height, free for a small title.",
                percent(first.w),
                percent(first.h)
            ),
            Place::Poster => format!(
                "Leave the top {}% and the bottom {}% of the frame to the lettering{}.",
                percent(zones[0].h),
                percent(zones[1].h),
                match subject {
                    Subject::Person => "; the head may overlap the top lettering",
                    Subject::Object => "; the main subject may overlap the top lettering",
                    Subject::None => "",
                }
            ),
        }
    }

    /// How the title is set there - the end of the lettering's sentence.
    pub fn setting(self, hero: Column, subject: Subject) -> String {
        match self {
            Place::Left => {
                "set as a tall left-aligned block in the left part of the frame".to_owned()
            }
            Place::Right => {
                "set as a tall right-aligned block in the right part of the frame".to_owned()
            }
            Place::Top => "set as a wide line across the top of the frame".to_owned(),
            Place::Bottom => "set as a wide line across the bottom of the frame".to_owned(),
            Place::Behind => match subject {
                Subject::Person => "set huge across the full width behind the hero's head; the head overlaps and hides part of the letters",
                Subject::Object => "set huge across the full width behind the main subject, which overlaps and hides part of the letters",
                Subject::None => "set huge across the full width in the upper part of the frame",
            }
            .to_owned(),
            Place::Overlap => match subject {
                Subject::Person => "set huge across the hero's chest, in front of the figure, slightly overlapping it",
                Subject::Object => "set huge across the main subject, in front of it, slightly overlapping it",
                Subject::None => "set huge across the middle of the frame, in front of the scene",
            }
            .to_owned(),
            Place::Vertical => format!(
                "set as a huge vertical title running top to bottom along the {} side",
                vertical_side(hero).word()
            ),
            Place::Corner => "set small in the lower left corner, like a label".to_owned(),
            Place::Poster => match subject {
                Subject::Person => "set huge across the top, partly hidden behind the hero's head, with a second smaller line at the bottom",
                Subject::Object => "set huge across the top, partly hidden behind the main subject, with a second smaller line at the bottom",
                Subject::None => "set huge across the top, with a second smaller line at the bottom",
            }
            .to_owned(),
        }
    }
}

/// The side a vertical title runs along: the one the hero leaves open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

impl Side {
    fn word(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }
}

fn vertical_side(hero: Column) -> Side {
    if hero == Column::Left {
        Side::Right
    } else {
        Side::Left
    }
}

/// Where a mark goes on a picture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "CoverCorner")]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    #[default]
    BottomRight,
}

impl Corner {
    pub const ALL: [Corner; 4] = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ];

    pub fn phrase(self) -> &'static str {
        match self {
            Corner::TopLeft => "upper left",
            Corner::TopRight => "upper right",
            Corner::BottomLeft => "lower left",
            Corner::BottomRight => "lower right",
        }
    }
}

/// The shape of a picture: `16:9`, `9:16`, `1:1`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    pub width: u32,
    pub height: u32,
}

impl Shape {
    pub const SQUARE: Shape = Shape {
        width: 1,
        height: 1,
    };

    /// Read a profile's `16:9`. Anything that is not two positive numbers
    /// around a colon is none.
    pub fn parse(raw: &str) -> Option<Self> {
        let (width, height) = raw.trim().split_once(':')?;
        let width: u32 = width.trim().parse().ok()?;
        let height: u32 = height.trim().parse().ok()?;
        (width > 0 && height > 0).then_some(Self { width, height })
    }

    /// As the profile writes it.
    pub fn name(self) -> String {
        format!("{}:{}", self.width, self.height)
    }

    /// The opening sentence of a picture's prompt.
    pub fn sentence(self, what: &str) -> String {
        let name = self.name();
        if self.width > self.height {
            format!("FRAME: a wide {name} {what}.")
        } else if self.width < self.height {
            format!("FRAME: a tall {name} {what}.")
        } else {
            format!("FRAME: a square {name} {what}.")
        }
    }

    /// The scheme's width, its height being [`SCHEME_HEIGHT`].
    fn scheme_width(self) -> f64 {
        round(SCHEME_HEIGHT * f64::from(self.width) / f64::from(self.height))
    }
}

/// Every scheme is this tall; its width follows the shape.
const SCHEME_HEIGHT: f64 = 100.0;

/// A rectangle, in the scheme's units or as shares of the frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, ts_rs::TS)]
#[ts(rename = "SchemeRect")]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    /// The same rectangle in a frame `width` by `height`.
    fn scaled(self, width: f64, height: f64) -> Self {
        Self {
            x: round(self.x * width),
            y: round(self.y * height),
            w: round(self.w * width),
            h: round(self.h * height),
        }
    }

    /// Whether `other` lies inside this one, allowing for the rounding.
    pub fn holds(&self, other: &Rect) -> bool {
        const SLACK: f64 = 0.051;
        other.x >= self.x - SLACK
            && other.y >= self.y - SLACK
            && other.x + other.w <= self.x + self.w + SLACK
            && other.y + other.h <= self.y + self.h + SLACK
    }
}

/// What a shape of the scheme is painted with. The colours themselves travel
/// once, on the scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "SchemePaint")]
pub enum Paint {
    Background,
    /// The hero.
    Figure,
    /// The title, and the pupils of a close look.
    Ink,
    Accent,
}

/// One shape of a scheme.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
#[serde(tag = "shape", rename_all = "camelCase")]
#[ts(rename = "SchemeShape")]
pub enum SchemeShape {
    Rect {
        rect: Rect,
        paint: Paint,
    },
    Circle {
        cx: f64,
        cy: f64,
        r: f64,
        paint: Paint,
    },
    Ellipse {
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        paint: Paint,
    },
    /// A closed outline, as SVG path data.
    Path {
        d: String,
        paint: Paint,
    },
}

/// The colours a scheme is painted in: the picture's own, not the window's
/// theme - a scheme on a light background is light in a dark window too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
#[ts(rename = "SchemeColours")]
pub struct Colours {
    pub background: String,
    pub figure: String,
    pub ink: String,
    pub accent: String,
}

impl Colours {
    /// The colours of a picture on `background` with `accent`: dark ink and a
    /// dark figure on a light ground, light ones on a dark ground.
    pub fn on(background: Option<&str>, accent: Option<&str>) -> Self {
        let background = background.unwrap_or(NEUTRAL_GROUND).to_owned();
        let dark = is_dark(&background);
        Self {
            figure: if dark { "#8F8C97" } else { "#57525F" }.to_owned(),
            ink: if dark { "#EFEBE3" } else { "#121114" }.to_owned(),
            accent: accent.unwrap_or(NEUTRAL_ACCENT).to_owned(),
            background,
        }
    }
}

/// The ground a scheme is drawn on before a background is chosen.
const NEUTRAL_GROUND: &str = "#D9D6CE";

/// The disc's colour in a scheme before an accent is chosen. The disc is
/// only drawn with an accent, so this paints nothing a person sees today; it
/// keeps the colours whole.
const NEUTRAL_ACCENT: &str = "#9D9A94";

/// Whether a `#RRGGBB` colour is dark enough to want light ink on it.
pub fn is_dark(hex: &str) -> bool {
    let Some(digits) = hex.trim().strip_prefix('#') else {
        return false;
    };
    let Ok(value) = u32::from_str_radix(digits, 16) else {
        return false;
    };
    if digits.len() != 6 {
        return false;
    }
    let channel = |shift: u32| f64::from((value >> shift) & 0xff);
    let luma = (0.299 * channel(16) + 0.587 * channel(8) + 0.114 * channel(0)) / 255.0;
    luma < 0.45
}

/// A picture's built frame, drawn: what the window shows beside the prompt.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Scheme {
    /// In the scheme's units: every scheme is 100 tall.
    pub width: f64,
    pub height: f64,
    pub colours: Colours,
    /// In the order they are painted.
    pub shapes: Vec<SchemeShape>,
    /// The box the hero stands in. May reach past the frame: a hero bigger
    /// than the frame is cropped by it.
    pub hero: Rect,
    /// The parts of the frame the title is kept to. Empty where the picture
    /// carries no words.
    pub zones: Vec<Rect>,
    /// Where the mark goes when it sits in a corner - the same box the
    /// exported picture puts it in.
    pub mark: Option<Rect>,
}

/// What a scheme is drawn for.
#[derive(Debug, Clone, Copy)]
pub struct Drawing<'a> {
    pub shape: Shape,
    /// Whether the picture carries a title.
    pub lettering: bool,
    pub colours: &'a Colours,
    /// What the picture is of: a person is drawn as one, a thing as a
    /// shape, and a picture with no subject draws none.
    pub subject: Subject,
    /// Whether a disc of the accent stands behind the subject: asked for by
    /// its own switch, and only with an accent to paint it in.
    pub disc: bool,
    /// The corner a mark sits in, when it sits in one.
    pub mark: Option<Corner>,
}

/// The side of a mark's square against the shorter side of the frame, and
/// its margin from the edges.
const MARK_SIDE: f64 = 0.22;
const MARK_MARGIN: f64 = 0.04;

/// Where a mark goes in a frame of `shape`, as shares of its width and
/// height - what the scheme draws and what an export composites.
pub fn mark_box(shape: Shape, corner: Corner) -> Rect {
    let width = f64::from(shape.width);
    let height = f64::from(shape.height);
    let shorter = width.min(height);
    let (side_x, side_y) = (MARK_SIDE * shorter / width, MARK_SIDE * shorter / height);
    let (margin_x, margin_y) = (
        MARK_MARGIN * shorter / width,
        MARK_MARGIN * shorter / height,
    );
    let x = match corner {
        Corner::TopLeft | Corner::BottomLeft => margin_x,
        Corner::TopRight | Corner::BottomRight => 1.0 - margin_x - side_x,
    };
    let y = match corner {
        Corner::TopLeft | Corner::TopRight => margin_y,
        Corner::BottomLeft | Corner::BottomRight => 1.0 - margin_y - side_y,
    };
    Rect::new(round4(x), round4(y), round4(side_x), round4(side_y))
}

/// What a frame's paragraph is said for.
#[derive(Debug, Clone, Copy)]
pub struct Said<'a> {
    pub subject: Subject,
    /// Whether the picture carries words at all: a split frame's other half
    /// is left to a title only where there is one.
    pub lettering: bool,
    /// Whether the title's part of the frame is kept for it in this prompt.
    pub room_for_title: bool,
    /// The accent, as a prompt says it.
    pub accent: Option<&'a str>,
    /// The accent is a disc behind the subject rather than a colour.
    pub disc: bool,
}

impl Framing {
    /// The frame's paragraph: where the subject stands, how big, how much of
    /// it shows, how the frame is composed, what the accent is for, and - on
    /// a picture that keeps room for words - the part kept for them.
    ///
    /// One block (v0.115). Until then a PLACEMENT and a COMPOSITION said the
    /// same frame twice, three paragraphs apart, and could disagree.
    pub fn paragraph(&self, said: &Said<'_>) -> String {
        let subject = said.subject;
        let mut out: Vec<String> = Vec::new();
        if subject == Subject::None {
            out.push(format!(
                "FRAMING: no single subject. {} The focal point sits {}, {}.",
                self.size.view(),
                self.column.phrase(),
                self.row.phrase()
            ));
        } else {
            out.push(format!(
                "FRAMING: place {} {}, {}. {} {}",
                subject.noun(),
                self.column.phrase(),
                self.row.phrase(),
                self.size.sentence(subject),
                self.crop.sentence(subject)
            ));
        }
        out.push(self.layout.composition(subject, said.lettering));
        if let Some(accent) = said.accent {
            out.push(if !said.disc {
                format!(
                    "Accent colour: {accent}, used for highlights, light edges and key accents."
                )
            } else if subject == Subject::None {
                format!("A large flat circle in {accent} sits in the background.")
            } else {
                format!(
                    "A large flat circle in {accent} sits behind {}.",
                    subject.noun()
                )
            });
        }
        if said.room_for_title {
            out.push(self.place.space(self.column, subject));
        }
        out.into_iter()
            .map(|sentence| sentence.trim().to_owned())
            .filter(|sentence| !sentence.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The hero's box, in the scheme's units.
    fn hero_box(&self, width: f64, height: f64) -> Rect {
        let tall = height * self.size.share();
        let wide = tall * self.crop.aspect();
        let x = width * self.column.centre() - wide / 2.0;
        let y = match self.row {
            Row::Top => height * 0.06,
            Row::Middle => (height - tall) / 2.0,
            Row::Bottom => height - tall,
        };
        Rect::new(round(x), round(y), round(wide), round(tall))
    }

    /// The scheme, drawn from the same settings the placement sentence is
    /// written from.
    pub fn scheme(&self, drawing: Drawing<'_>) -> Scheme {
        let width = drawing.shape.scheme_width();
        let height = SCHEME_HEIGHT;
        let hero = self.hero_box(width, height);
        let mut shapes = vec![SchemeShape::Rect {
            rect: Rect::new(0.0, 0.0, width, height),
            paint: Paint::Background,
        }];

        // The subject as the frame shows it: the part of its box that is in
        // the frame. A subject bigger than the frame is cropped by it, and
        // what is drawn of it is drawn in what is left - a close look at a
        // pair of eyes drew them at twice the frame's height until v0.115.
        let shown = Rect::new(
            hero.x.max(0.0),
            hero.y.max(0.0),
            round(hero.x.max(0.0).mul_add(-1.0, (hero.x + hero.w).min(width))),
            round(hero.y.max(0.0).mul_add(-1.0, (hero.y + hero.h).min(height))),
        );

        if drawing.disc {
            let (head_at, reach) = self.crop.head();
            let (cy, r) = match drawing.subject {
                Subject::Person => (hero.y + head_at * hero.h, reach * hero.h),
                Subject::Object => (shown.y + shown.h / 2.0, 0.5 * shown.h.min(shown.w)),
                Subject::None => (height * 0.5, height * 0.3),
            };
            shapes.push(SchemeShape::Circle {
                cx: round(shown.x + shown.w / 2.0),
                cy: round(cy.clamp(0.0, height)),
                r: round(r.min(height * 0.6)),
                paint: Paint::Accent,
            });
        }

        let zones: Vec<Rect> = if drawing.lettering {
            self.place
                .zones(self.column)
                .into_iter()
                .map(|zone| zone.scaled(width, height))
                .collect()
        } else {
            Vec::new()
        };
        let blocks: Vec<Rect> = if drawing.lettering {
            self.place
                .blocks(self.column)
                .into_iter()
                .map(|block| block.scaled(width, height))
                .collect()
        } else {
            Vec::new()
        };
        // A title behind the hero is painted first, so the head covers it;
        // of a poster's two lines only the top one is behind.
        let (behind, front): (Vec<Rect>, Vec<Rect>) = if self.place.behind_hero() {
            let mut blocks = blocks.into_iter();
            let first = blocks.next();
            (first.into_iter().collect(), blocks.collect())
        } else {
            (Vec::new(), blocks)
        };
        shapes.extend(behind.into_iter().map(|rect| SchemeShape::Rect {
            rect,
            paint: Paint::Ink,
        }));
        match drawing.subject {
            Subject::Person if self.crop == Crop::Detail => {
                shapes.extend(figure(self.crop, shown));
            }
            Subject::Person => shapes.extend(figure(self.crop, hero)),
            // A thing is a shape that fills its box, not a body.
            Subject::Object => shapes.push(SchemeShape::Ellipse {
                cx: round(shown.x + shown.w / 2.0),
                cy: round(shown.y + shown.h / 2.0),
                rx: round(shown.w * 0.42),
                ry: round(shown.h * 0.42),
                paint: Paint::Figure,
            }),
            Subject::None => {}
        }
        shapes.extend(front.into_iter().map(|rect| SchemeShape::Rect {
            rect,
            paint: Paint::Ink,
        }));

        Scheme {
            width,
            height,
            colours: drawing.colours.clone(),
            shapes,
            hero,
            zones,
            mark: drawing
                .mark
                .map(|corner| mark_box(drawing.shape, corner).scaled(width, height)),
        }
    }
}

/// The hero, drawn as the crop shows them, inside their box.
fn figure(crop: Crop, hero: Rect) -> Vec<SchemeShape> {
    let at = |u: f64, v: f64| (round(hero.x + u * hero.w), round(hero.y + v * hero.h));
    let point = |u: f64, v: f64| {
        let (x, y) = at(u, v);
        format!("{x},{y}")
    };
    let paint = Paint::Figure;
    match crop {
        Crop::Full => {
            let (cx, cy) = at(0.5, 0.085);
            let outline = [
                (0.3, 0.17),
                (0.7, 0.17),
                (0.78, 0.55),
                (0.64, 0.58),
                (0.62, 1.0),
                (0.53, 1.0),
                (0.5, 0.66),
                (0.47, 1.0),
                (0.38, 1.0),
                (0.36, 0.58),
                (0.22, 0.55),
            ]
            .iter()
            .map(|(u, v)| point(*u, *v))
            .collect::<Vec<_>>()
            .join(" L");
            vec![
                SchemeShape::Circle {
                    cx,
                    cy,
                    r: round(0.075 * hero.h),
                    paint,
                },
                SchemeShape::Path {
                    d: format!("M{outline} Z"),
                    paint,
                },
            ]
        }
        Crop::Waist => {
            let (cx, cy) = at(0.5, 0.2);
            vec![
                SchemeShape::Circle {
                    cx,
                    cy,
                    r: round(0.15 * hero.h),
                    paint,
                },
                SchemeShape::Path {
                    d: format!(
                        "M{} L{} Q{} {} L{} Z",
                        point(0.14, 1.0),
                        point(0.2, 0.52),
                        point(0.5, 0.34),
                        point(0.8, 0.52),
                        point(0.86, 1.0)
                    ),
                    paint,
                },
            ]
        }
        Crop::Bust => {
            let (cx, cy) = at(0.5, 0.3);
            vec![
                SchemeShape::Circle {
                    cx,
                    cy,
                    r: round(0.21 * hero.h),
                    paint,
                },
                SchemeShape::Path {
                    d: format!(
                        "M{} L{} Q{} {} L{} Z",
                        point(0.0, 1.0),
                        point(0.06, 0.8),
                        point(0.5, 0.52),
                        point(0.94, 0.8),
                        point(1.0, 1.0)
                    ),
                    paint,
                },
            ]
        }
        Crop::Head => {
            let (nx, ny) = at(0.38, 0.78);
            let (cx, cy) = at(0.5, 0.45);
            vec![
                SchemeShape::Rect {
                    rect: Rect::new(nx, ny, round(0.24 * hero.w), round(0.22 * hero.h)),
                    paint,
                },
                SchemeShape::Ellipse {
                    cx,
                    cy,
                    rx: round(0.38 * hero.w),
                    ry: round(0.42 * hero.h),
                    paint,
                },
            ]
        }
        Crop::Detail => [0.28, 0.72]
            .into_iter()
            .flat_map(|u| {
                let (cx, cy) = at(u, 0.5);
                [
                    SchemeShape::Ellipse {
                        cx,
                        cy,
                        rx: round(0.2 * hero.w),
                        ry: round(0.16 * hero.h),
                        paint,
                    },
                    SchemeShape::Circle {
                        cx,
                        cy,
                        r: round(0.1 * hero.h),
                        paint: Paint::Ink,
                    },
                ]
            })
            .collect(),
    }
}

/// A share as a whole percent: 0.4 is 40.
fn percent(share: f64) -> i64 {
    (share * 100.0).round() as i64
}

fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colours() -> Colours {
        Colours::on(Some("#EFEBE3"), Some("#FF2E63"))
    }

    #[test]
    fn a_layout_starts_from_its_own_frame() {
        let right = Layout::EmblemRight.defaults();
        assert_eq!(right.column, Column::Right);
        assert_eq!(right.place, Place::Left);
        let split = Layout::Split.defaults();
        assert_eq!(split.place, Place::Vertical);
        assert_eq!(split.column, Column::Left);
        for layout in Layout::ALL {
            assert_eq!(layout.defaults().layout, layout);
        }
    }

    #[test]
    fn a_shape_is_read_from_the_profiles_words() {
        assert_eq!(
            Shape::parse("16:9"),
            Some(Shape {
                width: 16,
                height: 9
            })
        );
        assert_eq!(
            Shape::parse(" 9 : 16 ").map(Shape::name),
            Some("9:16".into())
        );
        assert_eq!(Shape::parse("16x9"), None);
        assert_eq!(Shape::parse("0:9"), None);
        assert!(
            Shape::parse("16:9")
                .unwrap()
                .sentence("cover")
                .contains("wide 16:9")
        );
        assert!(
            Shape::parse("9:16")
                .unwrap()
                .sentence("cover")
                .contains("tall 9:16")
        );
        assert!(Shape::SQUARE.sentence("cover").contains("square 1:1"));
    }

    #[test]
    fn dark_grounds_want_light_ink() {
        assert!(is_dark("#0E0D10"));
        assert!(is_dark("#16203A"));
        assert!(!is_dark("#EFEBE3"));
        assert!(!is_dark("#D3F03B"));
        assert!(!is_dark("not a colour"));
        assert_eq!(Colours::on(Some("#0E0D10"), None).ink, "#EFEBE3");
        assert_eq!(Colours::on(Some("#EFEBE3"), None).ink, "#121114");
    }

    #[test]
    fn a_mark_sits_in_its_corner_inside_the_frame() {
        for shape in ["16:9", "9:16", "1:1"].map(|raw| Shape::parse(raw).unwrap()) {
            for corner in Corner::ALL {
                let mark = mark_box(shape, corner);
                assert!(
                    Rect::new(0.0, 0.0, 1.0, 1.0).holds(&mark),
                    "{corner:?} {mark:?}"
                );
                let right = mark.x > 0.5;
                let low = mark.y > 0.5;
                assert_eq!(
                    right,
                    matches!(corner, Corner::TopRight | Corner::BottomRight),
                    "{corner:?}"
                );
                assert_eq!(
                    low,
                    matches!(corner, Corner::BottomLeft | Corner::BottomRight),
                    "{corner:?}"
                );
                // Square on the picture, whatever its shape.
                let wide = mark.w * f64::from(shape.width);
                let tall = mark.h * f64::from(shape.height);
                assert!((wide - tall).abs() < 0.01, "{shape:?} {mark:?}");
            }
        }
    }

    #[test]
    fn a_picture_without_words_keeps_no_part_of_the_frame_for_them() {
        let framing = Layout::EmblemRight.defaults();
        let colours = colours();
        let scheme = framing.scheme(Drawing {
            shape: Shape::parse("16:9").unwrap(),
            lettering: false,
            colours: &colours,
            subject: Subject::Person,
            disc: true,
            mark: None,
        });
        assert!(scheme.zones.is_empty());
        assert!(
            !scheme.shapes.iter().any(|shape| matches!(
                shape,
                SchemeShape::Rect {
                    paint: Paint::Ink,
                    ..
                }
            )),
            "no title is drawn"
        );
        let said = |room_for_title: bool| {
            framing.paragraph(&Said {
                subject: Subject::Person,
                lettering: room_for_title,
                room_for_title,
                accent: None,
                disc: false,
            })
        };
        assert!(!said(false).contains("title"));
        assert!(said(true).contains("left 40%"));
    }
}
