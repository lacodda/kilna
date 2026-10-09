//! The board of ideas for a publication's cover (v0.89, ADR 0050).
//!
//! Before a person settles on a cover they look at several: their own idea,
//! the same idea worked out by the assistant, a handful the assistant
//! proposes from other angles, the cover a neighbouring publication of the
//! same song already has. Each is a concept of the cover's own shape, judged
//! on its own - starred onto the shortlist, turned down, taken into the
//! constructor - and kept between sessions: a row of `cover_idea`
//! (migration 0034).
//!
//! The neighbours' covers are not stored until they are judged here: they
//! are read live from the other publications of the same song, fitted to
//! this publication's shape, and a star copies one onto the board.
//!
//! What the assistant answers - and what an agent outside the window
//! proposes - is read by [`read`]: every brick, card and variant it names is
//! looked up in the workspace by id or by name, and what is not there is
//! left out and said, rather than refusing a whole answer for one misspelt
//! name.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::framing::{Colours, Column, Crop, Drawing, Framing, Layout, Place, Row, Scheme, Size};
use super::read::{Channel, MarkOption};
use super::{Accent, Cover, Hero, Lettering, MarkChoice};
use crate::asset::Asset;
use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::profile::config::{Label, ProfileConfig, StyleForm};
use crate::style_brick::StyleBrick;
use crate::work::Work;

/// Where an idea came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename = "IdeaSource")]
pub enum Source {
    /// The person's own idea, in their words.
    Own,
    /// The person's idea worked out by the assistant.
    Refined,
    /// The assistant's own.
    Ai,
    /// A neighbouring publication's cover, copied when it was starred.
    Sibling,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Own => "own",
            Source::Refined => "refined",
            Source::Ai => "ai",
            Source::Sibling => "sibling",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "own" => Some(Source::Own),
            "refined" => Some(Source::Refined),
            "ai" => Some(Source::Ai),
            "sibling" => Some(Source::Sibling),
            _ => None,
        }
    }
}

/// What the person said about an idea. None of them is "not judged yet".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename = "IdeaVerdict")]
pub enum Verdict {
    /// On the shortlist: an example of what is wanted.
    Star,
    /// "Not that": an example of what is not.
    Rejected,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Star => "star",
            Verdict::Rejected => "rejected",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "star" => Some(Verdict::Star),
            "rejected" => Some(Verdict::Rejected),
            _ => None,
        }
    }
}

/// One idea on a publication's board.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct CoverIdea {
    pub id: String,
    pub profile_id: String,
    /// The publication whose board it is on.
    pub work_id: String,
    pub source: Source,
    /// The publication a neighbour's idea was copied from, while it exists.
    pub from_work_id: Option<String>,
    /// What sets it apart from the others on the board.
    pub angle: String,
    /// What the card is called.
    pub headline: String,
    pub concept: Cover,
    pub verdict: Option<Verdict>,
    pub created_at: String,
    pub updated_at: String,
}

/// An idea to put on a board - what the operations log carries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewIdea {
    pub work_id: String,
    pub source: Source,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_work_id: Option<String>,
    #[serde(default)]
    pub angle: String,
    #[serde(default)]
    pub headline: String,
    #[serde(default)]
    pub concept: Cover,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Verdict>,
}

/// What may change about an idea; `None` leaves a part alone, and
/// `Some(None)` takes the verdict back.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IdeaPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concept: Option<Cover>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub verdict: Option<Option<Verdict>>,
}

const SELECT: &str = "SELECT id, profile_id, work_id, source, from_work_id, angle, headline, \
                      concept, verdict, created_at, updated_at FROM cover_idea";

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CoverIdea> {
    let source: String = row.get(3)?;
    let concept: String = row.get(7)?;
    let verdict: Option<String> = row.get(8)?;
    Ok(CoverIdea {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        work_id: row.get(2)?,
        // The schema's CHECK holds both words to the four and the two; a
        // word this build does not know reads as the assistant's own rather
        // than failing the whole board.
        source: Source::parse(&source).unwrap_or(Source::Ai),
        from_work_id: row.get(4)?,
        angle: row.get(5)?,
        headline: row.get(6)?,
        concept: serde_json::from_str(&concept).unwrap_or_default(),
        verdict: verdict.as_deref().and_then(Verdict::parse),
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

/// The publication an idea is for: a work of the profile whose kind has a
/// cover.
fn a_board(conn: &Connection, profile_id: &str, work_id: &str) -> Result<Work> {
    let work = crate::work::get(conn, work_id)?
        .filter(|work| work.profile_id == profile_id)
        .ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, profile_id)?;
    if !config.vocabulary(&work.kind).cover {
        return Err(Error::refused("idea.noCover").param("title", work.title.clone()));
    }
    Ok(work)
}

/// Put an idea on a publication's board.
///
/// Refused when the publication has no cover, when the idea says nothing at
/// all, when a neighbour's idea names no neighbour (or another's does), and
/// when its concept names a brick, a card or a variant that is not there -
/// the same door the cover itself goes through.
pub fn create_minted(
    conn: &Connection,
    profile_id: &str,
    new: NewIdea,
    minted: Minted,
) -> Result<CoverIdea> {
    a_board(conn, profile_id, &new.work_id)?;
    if !new.concept.holds_anything() && new.headline.trim().is_empty() {
        return Err(Error::refused("idea.empty"));
    }
    match (new.source, new.from_work_id.as_deref()) {
        (Source::Sibling, Some(from)) => {
            crate::work::get(conn, from)?.ok_or_else(|| Error::not_found("work", from))?;
        }
        (Source::Sibling, None) => return Err(Error::refused("idea.siblingWithoutWork")),
        (_, Some(_)) => return Err(Error::refused("idea.workOfNoSibling")),
        (_, None) => {}
    }
    super::check(conn, profile_id, &new.concept)?;

    conn.execute(
        "INSERT INTO cover_idea (id, profile_id, work_id, source, from_work_id, angle, headline,
                                 concept, verdict, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![
            minted.id(),
            profile_id,
            new.work_id,
            new.source.as_str(),
            new.from_work_id,
            new.angle.trim(),
            new.headline.trim(),
            serde_json::to_string(&new.concept)?,
            new.verdict.map(Verdict::as_str),
            minted.at(),
        ],
    )?;
    get(conn, minted.id())?.ok_or_else(|| Error::Internal("the idea vanished after insert".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<CoverIdea>> {
    Ok(conn
        .query_row(&format!("{SELECT} WHERE id = ?1"), params![id], read_row)
        .optional()?)
}

/// A publication's board, oldest first: the order the ideas arrived in.
pub fn for_work(conn: &Connection, work_id: &str) -> Result<Vec<CoverIdea>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE work_id = ?1 ORDER BY created_at, rowid"
    ))?;
    let rows = statement
        .query_map(params![work_id], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Change an idea: its words, its concept, the person's verdict on it.
pub fn update_at(conn: &Connection, id: &str, patch: IdeaPatch, at: &str) -> Result<CoverIdea> {
    let current = get(conn, id)?.ok_or_else(|| Error::not_found("idea", id))?;
    if let Some(concept) = &patch.concept {
        super::check(conn, &current.profile_id, concept)?;
    }
    let angle = patch.angle.unwrap_or(current.angle);
    let headline = patch.headline.unwrap_or(current.headline);
    let concept = patch.concept.unwrap_or(current.concept);
    let verdict = patch.verdict.unwrap_or(current.verdict);
    conn.execute(
        "UPDATE cover_idea SET angle = ?2, headline = ?3, concept = ?4, verdict = ?5,
                               updated_at = ?6
         WHERE id = ?1",
        params![
            id,
            angle.trim(),
            headline.trim(),
            serde_json::to_string(&concept)?,
            verdict.map(Verdict::as_str),
            at,
        ],
    )?;
    get(conn, id)?.ok_or_else(|| Error::not_found("idea", id))
}

/// The cover a publication has once `idea` is taken into its constructor.
///
/// An idea decides the picture: its idea and scene and what the scene keeps
/// out, the hero, the built frame, the four bricks, the accent, which variant
/// of the mark, the captions - each as the idea has it, an absent one absent. What belongs to
/// the publication stays: the lettered title and whether it goes apart,
/// where and how the mark goes, the switches of the channel's details, the
/// person's own words and the prompt as it was last copied.
///
/// An idea of words alone - the person's own, never worked out - is an idea
/// for the cover being built, not a cover to start from: it changes only the
/// words of the idea.
pub fn taken_into(cover: &Cover, idea: &Cover) -> Cover {
    if !idea.is_built() {
        return Cover {
            idea: idea.idea.clone(),
            ..cover.clone()
        };
    }
    Cover {
        idea: idea.idea.clone(),
        scene: idea.scene.clone(),
        avoid: idea.avoid.clone(),
        hero: idea.hero.clone(),
        framing: idea.framing,
        bricks: idea.bricks.clone(),
        accent: idea.accent.clone(),
        mark: MarkChoice {
            variant: idea.mark.variant.clone(),
            ..cover.mark.clone()
        },
        lettering: Lettering {
            captions: idea.lettering.captions.clone(),
            ..cover.lettering.clone()
        },
        ..cover.clone()
    }
}

/// What an idea decides, and nothing else - what two ideas are compared by.
fn decided(concept: &Cover) -> Cover {
    taken_into(&Cover::default(), concept)
}

/// `concept` in a picture of `shape`. A tall picture has no room beside the
/// hero: a layout built around a side becomes the centred one, and a title
/// kept to a side goes to the top. Everything else reads the same in any
/// shape - the frame is shares of the picture, not pixels.
pub fn fitted(concept: &Cover, shape: super::framing::Shape) -> Cover {
    let Some(framing) = concept.framing else {
        return concept.clone();
    };
    if shape.width >= shape.height {
        return concept.clone();
    }
    let mut fitted = framing;
    if matches!(
        framing.layout,
        Layout::EmblemLeft | Layout::EmblemRight | Layout::Split
    ) {
        fitted = Layout::Centre.defaults();
        fitted.crop = framing.crop;
    }
    fitted.column = Column::Centre;
    if matches!(fitted.place, Place::Left | Place::Right | Place::Vertical) {
        fitted.place = Place::Top;
    }
    Cover {
        framing: Some(fitted),
        ..concept.clone()
    }
}

/// A brick as a card of the board names it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(rename = "IdeaBrick")]
pub struct Named {
    pub id: String,
    pub name: String,
    /// The name per language, while it is the one the set shipped.
    pub label: Option<Label>,
    /// The colour of a ground.
    pub colour: Option<String>,
    /// The first reference picture of an image style.
    pub picture: Option<Asset>,
}

/// What a card of the board shows of a concept.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(rename = "IdeaLook")]
pub struct Look {
    /// The built frame drawn in the board's shape.
    pub scheme: Option<Scheme>,
    pub layout: Option<Layout>,
    pub style: Option<Named>,
    pub background: Option<Named>,
    /// The variant of the mark: its status and file.
    pub mark: Option<MarkOption>,
    /// The hero's card, by name.
    pub hero: Option<String>,
}

/// An idea as the board shows it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(rename = "IdeaCard")]
pub struct Card {
    pub idea: CoverIdea,
    pub look: Look,
    /// The title of the neighbour a copied idea came from, while it exists.
    pub from_title: Option<String>,
}

/// A neighbouring publication's cover, offered on the board.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(rename = "SiblingCover")]
pub struct Sibling {
    pub work_id: String,
    pub title: String,
    pub kind: String,
    /// Its cover, fitted to this publication's shape.
    pub concept: Cover,
    pub look: Look,
    /// Its final picture, when it has one.
    pub picture: Option<Asset>,
}

/// A publication's board of ideas.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct CoverBoard {
    /// The shape the schemes are drawn in.
    pub format: String,
    pub ideas: Vec<Card>,
    /// The neighbours' covers not yet copied onto this board as they stand.
    pub siblings: Vec<Sibling>,
}

/// What a board is drawn from: the bricks, cards and the channel, read once.
struct Lookup {
    channel: Channel,
    shape: super::framing::Shape,
}

impl Lookup {
    fn look(&self, conn: &Connection, concept: &Cover) -> Result<Look> {
        let brick = |id: Option<&str>| -> Result<Option<StyleBrick>> {
            match id {
                Some(id) => crate::style_brick::get(conn, id),
                None => Ok(None),
            }
        };
        let style = match brick(concept.bricks.style.as_deref())? {
            Some(found) => {
                let picture = crate::asset::for_style_brick(conn, &found.id)?
                    .into_iter()
                    .next();
                Some(Named {
                    id: found.id,
                    name: found.name,
                    label: found.label,
                    colour: found.colours.first().cloned(),
                    picture,
                })
            }
            None => None,
        };
        let background = brick(concept.bricks.background.as_deref())?.map(|found| Named {
            id: found.id,
            name: found.name,
            label: found.label,
            colour: found.colours.first().cloned(),
            picture: None,
        });
        let colours = Colours::on(
            background
                .as_ref()
                .and_then(|named| named.colour.as_deref()),
            concept.accent.as_ref().map(|accent| accent.color.as_str()),
        );
        let scheme = concept.framing.map(|framing| {
            framing.scheme(Drawing {
                shape: self.shape,
                lettering: true,
                colours: &colours,
                accent: concept.accent.is_some(),
                mark: concept.mark.in_corner(),
            })
        });
        let mark = concept.mark.variant.as_deref().and_then(|variant| {
            self.channel
                .marks
                .iter()
                .find(|option| option.id == variant)
                .cloned()
        });
        let hero = match concept.hero_card() {
            Some(card) => crate::note::get(conn, card)?.and_then(|note| note.title),
            None => None,
        };
        Ok(Look {
            scheme,
            layout: concept.framing.map(|framing| framing.layout),
            style,
            background,
            mark,
            hero,
        })
    }
}

/// The neighbouring publications of `work`: everything else made from the
/// song it is made from, whose kind has a cover. The song is the nearest
/// work up the chain that never goes out itself - a short cut from a clip
/// has the clip and the audio for neighbours, not only the clip.
fn neighbours(conn: &Connection, config: &ProfileConfig, work: &Work) -> Result<Vec<Work>> {
    let Some(root) = song_of(conn, config, work)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (id, _) in crate::link::descendants(conn, &root.id)? {
        if id == work.id {
            continue;
        }
        if let Some(found) = crate::work::get(conn, &id)?
            && config.vocabulary(&found.kind).cover
        {
            out.push(found);
        }
    }
    Ok(out)
}

/// What `work` is made from, all the way up (`publication::origin`).
fn song_of(conn: &Connection, config: &ProfileConfig, work: &Work) -> Result<Option<Work>> {
    crate::publication::origin(conn, config, &work.id)
}

/// The board of `work`, drawn in the shape its cover is written for.
pub fn board(conn: &Connection, work_id: &str) -> Result<CoverBoard> {
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let formats = super::read::formats_of(conn, &config, &work)?;
    let shape = super::read::shape_for(&formats, None);
    let lookup = Lookup {
        channel: Channel::read(conn, &work.profile_id, &config)?,
        shape,
    };

    let stored = for_work(conn, &work.id)?;
    let mut ideas = Vec::with_capacity(stored.len());
    for idea in stored {
        let look = lookup.look(conn, &idea.concept)?;
        let from_title = match idea.from_work_id.as_deref() {
            Some(from) => crate::work::get(conn, from)?.map(|found| found.title),
            None => None,
        };
        ideas.push(Card {
            idea,
            look,
            from_title,
        });
    }

    let mut siblings = Vec::new();
    for neighbour in neighbours(conn, &config, &work)? {
        if !neighbour.cover.is_built() {
            continue;
        }
        let concept = fitted(&neighbour.cover, shape);
        let copied = ideas.iter().any(|card| {
            card.idea.from_work_id.as_deref() == Some(neighbour.id.as_str())
                && decided(&card.idea.concept) == decided(&concept)
        });
        if copied {
            continue;
        }
        siblings.push(Sibling {
            look: lookup.look(conn, &concept)?,
            picture: crate::asset::cover_of(conn, &neighbour.id)?,
            work_id: neighbour.id,
            title: neighbour.title,
            kind: neighbour.kind,
            concept,
        });
    }

    Ok(CoverBoard {
        format: shape.name(),
        ideas,
        siblings,
    })
}

/// A neighbour's cover as an idea for `work`'s board, fitted to its shape.
pub fn from_sibling(conn: &Connection, work_id: &str, sibling_id: &str) -> Result<NewIdea> {
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let sibling = neighbours(conn, &config, &work)?
        .into_iter()
        .find(|neighbour| neighbour.id == sibling_id)
        .ok_or_else(|| Error::refused("idea.notASibling").param("title", work.title.clone()))?;
    if !sibling.cover.is_built() {
        return Err(Error::refused("idea.siblingHasNoCover").param("title", sibling.title));
    }
    let formats = super::read::formats_of(conn, &config, &work)?;
    let shape = super::read::shape_for(&formats, None);
    Ok(NewIdea {
        work_id: work.id,
        source: Source::Sibling,
        from_work_id: Some(sibling.id),
        angle: String::new(),
        headline: sibling.title,
        concept: decided(&fitted(&sibling.cover, shape)),
        verdict: None,
    })
}

// ---------------------------------------------------------------------------
// What the assistant is asked, and what it answered.

/// What a person asked the board for.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct IdeaRequest {
    /// How many ideas of the assistant's own: none to five.
    pub count: u8,
    /// The person's own idea, to be worked out as one more.
    pub refine: Option<String>,
    /// "More in this direction": the shortlist is what is wanted.
    pub more: bool,
}

/// The most ideas one run is asked for.
pub const MOST: u8 = 5;

/// The number of ideas a "Make…" asks for when the profile does not say.
pub const ON_MAKE: u8 = 3;

impl IdeaRequest {
    /// How many ideas the answer should hold.
    pub fn total(&self) -> usize {
        usize::from(self.count) + usize::from(self.refined().is_some())
    }

    /// The idea to work out, when there is one with words in it.
    pub fn refined(&self) -> Option<&str> {
        self.refine
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
    }

    /// Refuse a request that asks for nothing, or for more than one run gives.
    pub fn check(&self) -> Result<()> {
        if self.count > MOST {
            return Err(Error::refused("idea.tooMany").param("most", MOST));
        }
        if self.total() == 0 {
            return Err(Error::refused("idea.nothingAsked"));
        }
        Ok(())
    }
}

/// An idea inside a proposal: what it is, already checked against the
/// workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(rename = "PackagedIdea")]
pub struct Packaged {
    pub source: Source,
    #[serde(default)]
    pub angle: String,
    #[serde(default)]
    pub headline: String,
    pub concept: Cover,
}

/// A part of an answer's idea left out because the workspace has nothing by
/// that name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(rename = "IdeaDropped")]
pub struct Dropped {
    /// The idea's number in the answer, from 1.
    pub idea: usize,
    /// `style`, `typography`, `dressing`, `background`, `hero`, `mark`,
    /// `accent`, `layout`, `column`, `row`, `size`, `crop`, `place`, or
    /// `idea` for one that said nothing and was left out whole.
    pub part: String,
    pub value: String,
}

/// What an answer's block came to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Read {
    pub ideas: Vec<Packaged>,
    pub dropped: Vec<Dropped>,
}

/// The names an answer may use, read once.
struct Names {
    bricks: Vec<(StyleBrick, StyleForm)>,
    cards: Vec<crate::canon::view::CardSummary>,
    marks: Vec<MarkOption>,
    palette: Vec<super::read::PaletteColour>,
}

impl Names {
    fn read(conn: &Connection, profile_id: &str, config: &ProfileConfig) -> Result<Self> {
        let bricks = crate::style_brick::list(
            conn,
            profile_id,
            &crate::style_brick::StyleBrickFilter {
                ready_only: true,
                ..Default::default()
            },
        )?
        .into_iter()
        .filter_map(|brick| {
            let form = config
                .style_types
                .iter()
                .find(|kind| kind.key == brick.type_key)
                .filter(|kind| kind.retired.is_none() && kind.canon_kind.is_none())?
                .form;
            Some((brick, form))
        })
        .collect();
        let cards = crate::canon::view::cards(
            conn,
            profile_id,
            &crate::canon::view::CardFilter::default(),
        )?
        .into_iter()
        .filter(|card| config.card_kind(&card.kind).is_some_and(|kind| !kind.root))
        .collect();
        let channel = Channel::read(conn, profile_id, config)?;
        Ok(Self {
            bricks,
            cards,
            marks: channel.marks,
            palette: channel.palette,
        })
    }

    /// A brick of `form` named by its id, its name or a word of its label.
    fn brick(&self, named: &str, form: StyleForm) -> Option<&StyleBrick> {
        let named = named.trim();
        let of_form = || {
            self.bricks
                .iter()
                .filter(move |(_, f)| *f == form)
                .map(|(brick, _)| brick)
        };
        of_form().find(|brick| brick.id == named).or_else(|| {
            of_form().find(|brick| {
                same(&brick.name, named)
                    || brick.label.as_ref().is_some_and(|label| match label {
                        Label::One(word) => same(word, named),
                        Label::PerLocale(words) => words.values().any(|word| same(word, named)),
                    })
            })
        })
    }

    fn card(&self, named: &str) -> Option<&crate::canon::view::CardSummary> {
        let named = named.trim();
        self.cards.iter().find(|card| card.id == named).or_else(|| {
            self.cards.iter().find(|card| {
                card.title
                    .as_deref()
                    .is_some_and(|title| same(title, named))
                    || card.aliases.iter().any(|alias| same(alias, named))
            })
        })
    }

    fn mark(&self, named: &str) -> Option<&MarkOption> {
        let named = named.trim();
        self.marks.iter().find(|mark| mark.id == named).or_else(|| {
            self.marks.iter().find(|mark| {
                mark.code.as_deref().is_some_and(|code| same(code, named))
                    || same(&mark.name, named)
            })
        })
    }
}

fn same(one: &str, other: &str) -> bool {
    one.trim().to_lowercase() == other.trim().to_lowercase()
}

/// A text field of an idea's object.
fn text(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// One of the frame's words, as the code spells it.
fn word<T: serde::de::DeserializeOwned>(raw: &str) -> Option<T> {
    serde_json::from_value(Value::String(raw.trim().to_owned())).ok()
}

/// Read the ideas of an answer's block - or of an agent's proposal - against
/// the workspace.
///
/// The block is `{"ideas": [...]}` or the list alone. Every name is looked up
/// by id first, then by name; what is not found is left out of its idea and
/// said in `dropped`. An idea with neither words nor a scene is left out
/// whole. Refused only when nothing at all could be read.
pub fn read(
    conn: &Connection,
    profile_id: &str,
    config: &ProfileConfig,
    raw: &Value,
    request: Option<&IdeaRequest>,
) -> Result<Read> {
    let list = match raw {
        Value::Array(list) => list,
        Value::Object(object) => object
            .get("ideas")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::refused("idea.noIdeas"))?,
        _ => return Err(Error::refused("idea.noIdeas")),
    };
    let names = Names::read(conn, profile_id, config)?;
    let mut out = Read::default();
    let refining = request.and_then(IdeaRequest::refined).is_some();
    let mut refined_seen = false;

    for (index, raw) in list.iter().enumerate() {
        let number = index + 1;
        let mut leave = |part: &str, value: &str| {
            out.dropped.push(Dropped {
                idea: number,
                part: part.to_owned(),
                value: value.to_owned(),
            });
        };
        let Some(object) = raw.as_object() else {
            leave("idea", &raw.to_string());
            continue;
        };
        let idea = text(object, "idea").unwrap_or_default();
        let scene = text(object, "scene").unwrap_or_default();
        if idea.is_empty() && scene.is_empty() {
            leave("idea", &text(object, "headline").unwrap_or_default());
            continue;
        }

        // The one worked out from the person's idea says so; anything else is
        // the assistant's own. Only one can be, and only when one was asked.
        let source = match text(object, "source").as_deref() {
            Some("refined") if (refining || request.is_none()) && !refined_seen => {
                refined_seen = true;
                Source::Refined
            }
            _ => Source::Ai,
        };

        let mut concept = Cover {
            idea,
            scene,
            avoid: text(object, "avoid").unwrap_or_default(),
            ..Cover::default()
        };

        if let Some(named) = text(object, "hero") {
            match names.card(&named) {
                Some(card) => {
                    concept.hero = Some(Hero {
                        card: card.id.clone(),
                        references: true,
                    });
                }
                None => leave("hero", &named),
            }
        }

        if let Some(raw_layout) = text(object, "layout") {
            match word::<Layout>(&raw_layout) {
                Some(layout) => {
                    let mut framing: Framing = layout.defaults();
                    if let Some(raw) = text(object, "column") {
                        match word::<Column>(&raw) {
                            Some(value) => framing.column = value,
                            None => leave("column", &raw),
                        }
                    }
                    if let Some(raw) = text(object, "row") {
                        match word::<Row>(&raw) {
                            Some(value) => framing.row = value,
                            None => leave("row", &raw),
                        }
                    }
                    if let Some(raw) = text(object, "size") {
                        match word::<Size>(&raw) {
                            Some(value) => framing.size = value,
                            None => leave("size", &raw),
                        }
                    }
                    if let Some(raw) = text(object, "crop") {
                        match word::<Crop>(&raw) {
                            Some(value) => framing.crop = value,
                            None => leave("crop", &raw),
                        }
                    }
                    if let Some(raw) = text(object, "place") {
                        match word::<Place>(&raw) {
                            Some(value) => framing.place = value,
                            None => leave("place", &raw),
                        }
                    }
                    concept.framing = Some(framing);
                }
                None => leave("layout", &raw_layout),
            }
        }

        for (key, form) in [
            ("style", StyleForm::Picture),
            ("typography", StyleForm::Lettering),
            ("dressing", StyleForm::Dressing),
            ("background", StyleForm::Colour),
        ] {
            let Some(named) = text(object, key) else {
                continue;
            };
            let Some(brick) = names.brick(&named, form) else {
                leave(key, &named);
                continue;
            };
            let place = match form {
                StyleForm::Picture => &mut concept.bricks.style,
                StyleForm::Lettering => &mut concept.bricks.typography,
                StyleForm::Dressing => &mut concept.bricks.dressing,
                StyleForm::Colour => &mut concept.bricks.background,
                // An accent is a value the cover keeps, not a brick it is
                // built from: it is read below. A phrase is never a part of
                // a picture: it is written into a text.
                StyleForm::Accent | StyleForm::Phrase => continue,
            };
            *place = Some(brick.id.clone());
        }

        match object.get("accent") {
            Some(Value::String(raw)) if !raw.trim().is_empty() => {
                match accent_of(raw, None, &names) {
                    Some(accent) => concept.accent = Some(accent),
                    None => leave("accent", raw),
                }
            }
            Some(Value::Object(accent)) => {
                let colour = text(accent, "color").or_else(|| text(accent, "colour"));
                let name = text(accent, "name");
                match accent_of(
                    colour.as_deref().or(name.as_deref()).unwrap_or_default(),
                    name.as_deref(),
                    &names,
                ) {
                    Some(found) => concept.accent = Some(found),
                    None => leave("accent", &colour.or(name).unwrap_or_default()),
                }
            }
            _ => {}
        }

        if let Some(named) = text(object, "mark") {
            match names.mark(&named) {
                Some(mark) => concept.mark.variant = Some(mark.id.clone()),
                None => leave("mark", &named),
            }
        }

        if let Some(Value::Object(captions)) = object.get("captions") {
            let mut kept: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for (slot, lines) in captions {
                let lines: Vec<String> = match lines {
                    Value::String(line) => vec![line.trim().to_owned()],
                    Value::Array(lines) => lines
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|line| line.trim().to_owned())
                        .collect(),
                    _ => Vec::new(),
                };
                let lines: Vec<String> =
                    lines.into_iter().filter(|line| !line.is_empty()).collect();
                if !lines.is_empty() {
                    kept.insert(slot.trim().to_owned(), lines);
                }
            }
            concept.lettering.captions = kept;
        }

        out.ideas.push(Packaged {
            source,
            angle: text(object, "angle").unwrap_or_default(),
            headline: text(object, "headline").unwrap_or_default(),
            concept,
        });
    }

    if out.ideas.is_empty() {
        return Err(Error::refused("idea.noIdeas"));
    }
    Ok(out)
}

/// An accent named by its colour, by a colour of the channel's palette, or
/// by an accent of the dictionary - its id or its name, a gradient whole.
fn accent_of(raw: &str, name: Option<&str>, names: &Names) -> Option<Accent> {
    let raw = raw.trim();
    if crate::canon::fact::is_hex_colour(raw) {
        let name = name
            .map(str::to_owned)
            .or_else(|| {
                names
                    .palette
                    .iter()
                    .find(|colour| colour.color.eq_ignore_ascii_case(raw))
                    .map(|colour| colour.name.clone())
            })
            .unwrap_or_default();
        return Some(Accent {
            name,
            color: raw.to_uppercase(),
            stops: Vec::new(),
        });
    }
    names
        .palette
        .iter()
        .find(|colour| same(&colour.name, raw))
        .map(|colour| Accent {
            name: colour.name.clone(),
            color: colour.color.clone(),
            stops: Vec::new(),
        })
        .or_else(|| {
            names
                .brick(raw, StyleForm::Accent)
                .and_then(Accent::of_brick)
        })
}

/// What the board asks for, and what stands on it: the `{ideas}` of an action
/// about a cover.
pub fn request_sheet(conn: &Connection, work: &Work, request: &IdeaRequest) -> Result<String> {
    let mut out = String::new();
    let count = usize::from(request.count);
    match (count, request.refined()) {
        (0, Some(own)) => out.push_str(&format!(
            "Asked for: the person's own idea, worked out into one cover - nothing else.\n\nTheir idea, in their words: “{own}”\n"
        )),
        (_, Some(own)) => out.push_str(&format!(
            "Asked for: {count} new idea{} of your own, and the person's own idea worked out into one more.\n\nTheir idea, in their words: “{own}”\n",
            if count == 1 { "" } else { "s" }
        )),
        (_, None) => out.push_str(&format!(
            "Asked for: {count} new idea{} of your own.\n",
            if count == 1 { "" } else { "s" }
        )),
    }

    let ideas = for_work(conn, &work.id)?;
    let starred: Vec<&CoverIdea> = ideas
        .iter()
        .filter(|idea| idea.verdict == Some(Verdict::Star))
        .collect();
    let rejected: Vec<&CoverIdea> = ideas
        .iter()
        .filter(|idea| idea.verdict == Some(Verdict::Rejected))
        .collect();
    let open: Vec<&CoverIdea> = ideas.iter().filter(|idea| idea.verdict.is_none()).collect();

    if request.more && !starred.is_empty() {
        out.push_str(
            "\nMore in this direction: the shortlist below is what the person likes. Go further its way - its mood, its hero, its kind of picture - from angles it has not taken yet.\n",
        );
    }
    let mut list = |heading: &str, ideas: &[&CoverIdea]| {
        if ideas.is_empty() {
            return;
        }
        out.push_str(&format!("\n{heading}\n"));
        for idea in ideas {
            out.push_str(&format!("- {}\n", idea_line(conn, idea)));
        }
    };
    list(
        if request.more {
            "The shortlist - examples of what is wanted:"
        } else {
            "On the shortlist already - do not repeat these:"
        },
        &starred,
    );
    list(
        "Turned down - examples of what is NOT wanted; stay away from them:",
        &rejected,
    );
    list("On the board, not judged yet - do not repeat these:", &open);

    if work.cover.holds_anything() {
        out.push_str(&format!(
            "\nThe cover as it stands: {}\n",
            concept_line(conn, &work.cover)
        ));
    }
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    // A short is made from the clip, and the clip from the song: what the
    // action reads as the source is the clip, and the hero lives in the
    // song's lines. So the song is given here whenever it is further up.
    let donor = crate::link::sources(conn, &work.id)?
        .into_iter()
        .find(|source| source.role == crate::link::DONOR)
        .map(|source| source.source_id);
    if let Some(song) = song_of(conn, &config, work)?
        && donor.as_deref() != Some(song.id.as_str())
    {
        let text = match &song.current_version_id {
            Some(id) => crate::work::version::get(conn, id)?.map(|version| version.body),
            None => None,
        }
        .unwrap_or_default();
        out.push_str(&format!("\nThe song it all comes from: “{}”\n", song.title));
        let text = text.trim();
        if !text.is_empty() {
            let excerpt: String = text.chars().take(SONG_EXCERPT).collect();
            let cut = if excerpt.len() < text.len() {
                "\n[…]"
            } else {
                ""
            };
            out.push_str(&format!("\n{excerpt}{cut}\n"));
        }
    }
    let built: Vec<Work> = neighbours(conn, &config, work)?
        .into_iter()
        .filter(|neighbour| neighbour.cover.is_built())
        .collect();
    if !built.is_empty() {
        out.push_str("\nThe covers of the neighbouring publications of the same song - the same world, a cover of its own:\n");
        for neighbour in built {
            out.push_str(&format!(
                "- {} ({}): {}\n",
                neighbour.title,
                neighbour.kind,
                concept_line(conn, &neighbour.cover)
            ));
        }
    }
    Ok(out.trim_end().to_owned())
}

/// How much of the song's text a request quotes when the song is not the
/// publication's own source.
const SONG_EXCERPT: usize = 3000;

/// An idea on one line, for the request.
fn idea_line(conn: &Connection, idea: &CoverIdea) -> String {
    let mut parts = Vec::new();
    if !idea.headline.is_empty() {
        parts.push(format!("“{}”", idea.headline));
    }
    if !idea.angle.is_empty() {
        parts.push(idea.angle.clone());
    }
    parts.push(concept_line(conn, &idea.concept));
    parts.join(" — ")
}

/// What a concept decides, on one line.
fn concept_line(conn: &Connection, concept: &Cover) -> String {
    let mut parts = Vec::new();
    if !concept.idea.trim().is_empty() {
        parts.push(one_line(&concept.idea));
    }
    if !concept.scene.trim().is_empty() {
        parts.push(format!("scene: {}", one_line(&concept.scene)));
    }
    if let Some(framing) = concept.framing {
        parts.push(format!(
            "layout {}",
            serde_json::to_value(framing.layout)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .unwrap_or_default()
        ));
    }
    let brick = |id: Option<&str>| {
        id.and_then(|id| crate::style_brick::get(conn, id).ok().flatten())
            .map(|brick| brick.name)
    };
    if let Some(style) = brick(concept.bricks.style.as_deref()) {
        parts.push(format!("style {style}"));
    }
    if let Some(card) = concept.hero_card()
        && let Ok(Some(note)) = crate::note::get(conn, card)
    {
        parts.push(format!("hero {}", note.title.unwrap_or_default()));
    }
    if parts.is_empty() {
        "(nothing chosen)".to_owned()
    } else {
        parts.join("; ")
    }
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Everything an idea may choose from, with the names to choose it by: the
/// `{choices}` of an action about a cover.
pub fn choices_sheet(conn: &Connection, work: &Work) -> Result<String> {
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let names = Names::read(conn, &work.profile_id, &config)?;
    let channel = Channel::read(conn, &work.profile_id, &config)?;
    let formats = super::read::formats_of(conn, &config, work)?;
    let mut out = String::new();

    if !formats.is_empty() {
        let shapes: Vec<String> = formats
            .iter()
            .map(|format| format!("{} ({})", format.format, format.label.as_str()))
            .collect();
        out.push_str(&format!(
            "The shapes this publication's cover goes out in: {}.\n",
            shapes.join(", ")
        ));
    }

    out.push_str("\nLayouts (`layout`) - each starts its own frame, which the five settings below can move:\n");
    for layout in Layout::ALL {
        let framing = layout.defaults();
        out.push_str(&format!(
            "- `{}`: {} (starts at column {}, row {}, size {}, crop {}, title {})\n",
            spelt(&layout),
            layout_meaning(layout),
            spelt(&framing.column),
            spelt(&framing.row),
            spelt(&framing.size),
            spelt(&framing.crop),
            spelt(&framing.place),
        ));
    }
    out.push_str(&format!(
        "Settings, each optional: `column` {}; `row` {}; `size` {}; `crop` {}; `place` (where the title goes) {}.\n",
        all(&Column::ALL),
        all(&Row::ALL),
        all(&Size::ALL),
        all(&Crop::ALL),
        all(&Place::ALL),
    ));

    let house = &channel.house_styles;
    for (key, form, heading) in [
        (
            "style",
            StyleForm::Picture,
            "Image styles (`style`) - the channel's house styles marked ★, reach for them more often",
        ),
        (
            "typography",
            StyleForm::Lettering,
            "Lettering (`typography`)",
        ),
        (
            "dressing",
            StyleForm::Dressing,
            "Dressing of small captions around the title (`dressing`) - its {slots} are filled from `captions`",
        ),
        ("background", StyleForm::Colour, "Grounds (`background`)"),
        (
            "accent",
            StyleForm::Accent,
            "Accents (`accent`) - an id here, or a colour of the channel's palette below; several colours are a gradient",
        ),
    ] {
        let mut bricks: Vec<&StyleBrick> = names
            .bricks
            .iter()
            .filter(|(_, f)| *f == form)
            .map(|(brick, _)| brick)
            .collect();
        if bricks.is_empty() {
            continue;
        }
        bricks.sort_by_key(|brick| !house.contains(&brick.id));
        out.push_str(&format!("\n{heading}:\n"));
        for brick in bricks {
            let mut line = format!("- `{}` {}", brick.id, brick.name);
            if house.contains(&brick.id) {
                line.push_str(" ★");
            }
            if form.is_colour() && !brick.colours.is_empty() {
                line.push_str(&format!(" {}", brick.colours.join(" to ")));
            }
            if let Some(when) = brick
                .when_to_use
                .as_deref()
                .filter(|when| !when.trim().is_empty())
            {
                line.push_str(&format!(" - when: {}", one_line(when)));
            }
            if key == "dressing" {
                let slots = brick
                    .description
                    .as_deref()
                    .map(crate::style_set::slots)
                    .unwrap_or_default();
                if !slots.is_empty() {
                    line.push_str(&format!(" - slots: {}", slots.join(", ")));
                }
            }
            out.push_str(&line);
            out.push('\n');
        }
    }

    if !channel.palette.is_empty() {
        out.push_str(
            "\nThe channel's palette, for the `accent` (a colour #RRGGBB, or a name below):\n",
        );
        for colour in &channel.palette {
            out.push_str(&format!("- {} {}\n", colour.name, colour.color));
        }
    }

    if !names.marks.is_empty() {
        out.push_str("\nThe variants of the channel's mark (`mark`) - choose the one whose meaning fits the song:\n");
        for mark in &names.marks {
            let code = mark.code.as_deref().unwrap_or("");
            out.push_str(&format!(
                "- `{}` {} - {}\n",
                mark.id,
                code,
                one_line(&mark.name)
            ));
        }
    }

    if !names.cards.is_empty() {
        out.push_str("\nHeroes from the canon (`hero`) - a card the picture is drawn from; leave it out to let the scene describe the hero:\n");
        for card in &names.cards {
            let described = crate::note::get(conn, &card.id)?
                .and_then(|note| note.prompt)
                .is_some_and(|prompt| !prompt.trim().is_empty());
            out.push_str(&format!(
                "- `{}` {} ({}){}\n",
                card.id,
                card.title.as_deref().unwrap_or(""),
                card.kind,
                if described {
                    ""
                } else {
                    " - not described for a generator yet"
                }
            ));
        }
    }

    if !channel.bans.is_empty() {
        out.push_str("\nWhat a picture of this channel never shows - never propose any of it:\n");
        for ban in &channel.bans {
            out.push_str(&format!("- {}\n", one_line(ban)));
        }
    }
    Ok(out.trim().to_owned())
}

/// A word of the frame as the code spells it: `emblemRight`.
fn spelt<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn all<T: Serialize>(values: &[T]) -> String {
    values
        .iter()
        .map(|value| format!("`{}`", spelt(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn layout_meaning(layout: Layout) -> &'static str {
    match layout {
        Layout::EmblemRight => "an emblem in the right third, the title on the left",
        Layout::EmblemLeft => "an emblem in the left third, the title on the right",
        Layout::Centre => "an emblem in the middle, the whole height of the frame",
        Layout::Poster => "a loud full-bleed poster, the hero bigger than the frame",
        Layout::Masthead => "a magazine cover, the title behind the hero's head",
        Layout::CloseUp => "the face, or the key object, fills the frame",
        Layout::Figure => "a small lonely figure in a vast space",
        Layout::Split => "the hero in one half, a vertical title in the other",
    }
}

/// What an action about a cover appends to its prompt: the block to answer
/// with, spelled out, because "reply with JSON" makes a different shape every
/// time.
pub fn instruction(request: &IdeaRequest) -> String {
    let total = request.total();
    let refined = if request.refined().is_some() {
        "\n- The idea worked out from the person's own says `\"source\": \"refined\"`; every other one `\"source\": \"ai\"`."
    } else {
        ""
    };
    format!(
        "\n\nEnd your answer with one fenced ```json block holding exactly {total} idea{plural}:\n\n\
```json\n\
{{\"ideas\": [\n  {{\n    \"source\": \"ai\",\n    \"angle\": \"what sets this one apart, a few words\",\n    \"headline\": \"what the card is called\",\n    \"idea\": \"what the cover says, one or two sentences in the language of the work\",\n    \"scene\": \"what the picture shows, in English, for the generator: the hero, what they do, the objects around\",\n    \"avoid\": \"what this scene must keep out beyond the channel's bans, in English, or leave it out\",\n    \"hero\":\"a card id from the heroes, or leave it out\",\n    \"layout\": \"one of the layouts\",\n    \"size\": \"optional, moves the layout's frame; so do column, row, crop and place\",\n    \"style\": \"a style id\",\n    \"typography\": \"a lettering id\",\n    \"dressing\": \"a dressing id\",\n    \"background\": \"a ground id\",\n    \"accent\": \"an accent id, or #RRGGBB\",\n    \"mark\": \"a variant id of the mark\",\n    \"captions\": {{\"slot\": [\"a line\"]}}\n  }}\n]}}\n```\n\n\
- Use the ids from the lists exactly; a name that is not in a list is left out of the idea.\n\
- `idea` and `scene` are required; every other key may be left out.{refined}",
        plural = if total == 1 { "" } else { "s" },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cover::framing::Shape;
    use crate::fixtures;

    /// A workspace with the shipped profile and its starter set of styles.
    fn studio() -> (Connection, String) {
        let (conn, profile_id) = fixtures::workspace();
        crate::style_set::seed(&conn).unwrap();
        (conn, profile_id)
    }

    /// The first brick of the starter set of a type.
    fn set_brick(conn: &Connection, profile_id: &str, type_key: &str) -> StyleBrick {
        crate::style_brick::list(conn, profile_id, &Default::default())
            .unwrap()
            .into_iter()
            .find(|brick| brick.type_key == type_key && brick.status == "ready")
            .unwrap_or_else(|| panic!("the set has a ready `{type_key}`"))
    }

    fn built() -> Cover {
        Cover {
            idea: "a keeper's last night".into(),
            scene: "a lighthouse keeper at the window".into(),
            hero: Some(Hero {
                card: "card".into(),
                references: false,
            }),
            framing: Some(Layout::EmblemRight.defaults()),
            accent: Some(Accent {
                name: "teal".into(),
                color: "#1E9E95".into(),
                ..Accent::default()
            }),
            mark: MarkChoice {
                variant: Some("variant".into()),
                ..MarkChoice::default()
            },
            lettering: Lettering {
                captions: BTreeMap::from([("tagline".into(), vec!["by the sea".into()])]),
                ..Lettering::default()
            },
            ..Cover::default()
        }
    }

    /// Taking an idea replaces what it decides and keeps what belongs to the
    /// publication - and an idea absent a part takes the cover's part away.
    #[test]
    fn an_idea_taken_in_decides_the_picture_and_keeps_the_publication() {
        let cover = Cover {
            idea: "old".into(),
            scene: "an old scene".into(),
            hero: Some(Hero {
                card: "old-card".into(),
                references: true,
            }),
            mark: MarkChoice {
                variant: Some("old-variant".into()),
                place: super::super::MarkPlace::Hidden,
                ..MarkChoice::default()
            },
            lettering: Lettering {
                title: Some("Own title".into()),
                apart: true,
                ..Lettering::default()
            },
            details: BTreeMap::from([("detail".into(), false)]),
            picture: "own words".into(),
            ..Cover::default()
        };
        let idea = Cover {
            hero: None,
            avoid: "a drowned body".into(),
            negative: "the idea's own words".into(),
            ..built()
        };
        let taken = taken_into(&cover, &idea);
        assert_eq!(taken.idea, idea.idea);
        assert_eq!(taken.scene, idea.scene);
        assert_eq!(
            taken.avoid, "a drowned body",
            "what the scene keeps out goes with it"
        );
        assert_eq!(taken.negative, "", "the publication's own words stay");
        assert_eq!(
            taken.hero, None,
            "an idea without a card takes the card away"
        );
        assert_eq!(taken.framing, idea.framing);
        assert_eq!(taken.accent, idea.accent);
        assert_eq!(taken.mark.variant.as_deref(), Some("variant"));
        assert_eq!(taken.mark.place, super::super::MarkPlace::Hidden);
        assert_eq!(taken.lettering.title.as_deref(), Some("Own title"));
        assert!(taken.lettering.apart);
        assert_eq!(taken.lettering.captions, idea.lettering.captions);
        assert_eq!(taken.details, cover.details);
        assert_eq!(taken.picture, "own words");
    }

    /// An idea of words alone changes only the words of the idea.
    #[test]
    fn an_idea_of_words_changes_only_the_idea() {
        let cover = built();
        let words = Cover {
            idea: "the sea outside the window".into(),
            ..Cover::default()
        };
        let taken = taken_into(&cover, &words);
        assert_eq!(taken.idea, "the sea outside the window");
        assert_eq!(
            Cover {
                idea: cover.idea.clone(),
                ..taken
            },
            cover
        );
    }

    /// In a tall picture a side layout is centred and a side title goes up;
    /// a wide picture keeps the frame as it is.
    #[test]
    fn a_neighbours_frame_is_fitted_to_a_tall_picture() {
        let tall = Shape::parse("9:16").unwrap();
        let wide = Shape::parse("16:9").unwrap();
        let cover = built();
        let fitted_tall = fitted(&cover, tall).framing.unwrap();
        assert_eq!(fitted_tall.layout, Layout::Centre);
        assert_eq!(fitted_tall.column, Column::Centre);
        assert_eq!(fitted_tall.place, Place::Top);
        assert_eq!(fitted_tall.crop, Layout::EmblemRight.defaults().crop);
        assert_eq!(fitted(&cover, wide), cover);

        for layout in Layout::ALL {
            let framing = fitted(
                &Cover {
                    framing: Some(layout.defaults()),
                    ..Cover::default()
                },
                tall,
            )
            .framing
            .unwrap();
            assert_ne!(framing.layout, Layout::EmblemLeft);
            assert_ne!(framing.layout, Layout::EmblemRight);
            assert_ne!(framing.layout, Layout::Split);
            assert!(!matches!(
                framing.place,
                Place::Left | Place::Right | Place::Vertical
            ));
        }
    }

    #[test]
    fn a_request_asks_for_something_and_not_too_much() {
        assert!(IdeaRequest::default().check().is_err());
        assert!(
            IdeaRequest {
                count: MOST + 1,
                ..IdeaRequest::default()
            }
            .check()
            .is_err()
        );
        let refine_only = IdeaRequest {
            refine: Some("  the sea in a room  ".into()),
            ..IdeaRequest::default()
        };
        assert!(refine_only.check().is_ok());
        assert_eq!(refine_only.total(), 1);
        let blank = IdeaRequest {
            refine: Some("   ".into()),
            ..IdeaRequest::default()
        };
        assert!(blank.check().is_err(), "blank words are not an idea");
        assert_eq!(
            IdeaRequest {
                count: 3,
                refine: Some("x".into()),
                more: false
            }
            .total(),
            4
        );
    }

    /// Names are looked up by id or by name; what is not there is left out
    /// and said, and the rest of the idea lands.
    #[test]
    fn an_answer_is_read_against_the_workspace_and_what_is_missing_is_said() {
        let (conn, profile_id) = studio();
        let config = crate::profile::config_for(&conn, &profile_id).unwrap();
        let style = set_brick(&conn, &profile_id, "image-style");
        let ground = set_brick(&conn, &profile_id, "background");

        let raw = serde_json::json!({ "ideas": [
            {
                "source": "refined",
                "angle": "from below",
                "headline": "The keeper",
                "idea": "Смотритель маяка",
                "scene": "a keeper on a rock",
                "avoid": "  a drowned body  ",
                "layout": "poster",
                "size": "huge",
                "style": style.id,
                "background": ground.name.to_uppercase(),
                "typography": "No such lettering",
                "accent": "#1e9e95",
                "mark": "nothing",
                "hero": "Nobody",
                "captions": { "tagline": "by the sea", "empty": [""] }
            },
            { "headline": "only a name" },
            { "idea": "a second", "layout": "diagonal" }
        ]});
        let read = read(&conn, &profile_id, &config, &raw, None).unwrap();
        assert_eq!(read.ideas.len(), 2);
        let first = &read.ideas[0];
        assert_eq!(first.source, Source::Refined);
        assert_eq!(
            first.concept.bricks.style.as_deref(),
            Some(style.id.as_str())
        );
        assert_eq!(
            first.concept.bricks.background.as_deref(),
            Some(ground.id.as_str())
        );
        assert_eq!(first.concept.bricks.typography, None);
        assert_eq!(first.concept.framing.unwrap().layout, Layout::Poster);
        assert_eq!(
            first.concept.framing.unwrap().size,
            Layout::Poster.defaults().size,
            "an unknown size leaves the layout's own"
        );
        assert_eq!(first.concept.accent.as_ref().unwrap().color, "#1E9E95");
        assert_eq!(first.concept.lettering.captions.len(), 1);
        assert_eq!(first.concept.avoid, "a drowned body");
        assert_eq!(read.ideas[1].concept.avoid, "");
        assert_eq!(read.ideas[1].concept.framing, None);
        assert_eq!(read.ideas[1].source, Source::Ai);

        let said: Vec<(usize, &str)> = read
            .dropped
            .iter()
            .map(|dropped| (dropped.idea, dropped.part.as_str()))
            .collect();
        assert_eq!(
            said,
            [
                (1, "hero"),
                (1, "size"),
                (1, "typography"),
                (1, "mark"),
                (2, "idea"),
                (3, "layout"),
            ]
        );
    }

    /// An accent of the dictionary is named by its name or its id and lands
    /// the way the cover keeps it - a gradient with every stop, said as one -
    /// and an accent offered as the ground is not taken for one.
    #[test]
    fn an_accent_of_the_dictionary_lands_whole_and_is_no_ground() {
        let (conn, profile_id) = studio();
        let config = crate::profile::config_for(&conn, &profile_id).unwrap();
        let bricks = crate::style_brick::list(&conn, &profile_id, &Default::default()).unwrap();
        let gradient = bricks
            .iter()
            .find(|b| b.type_key == "accent" && b.colours.len() > 2)
            .expect("the set has a gradient accent of three stops");
        let flat = bricks
            .iter()
            .find(|b| b.type_key == "accent" && b.colours.len() == 1)
            .expect("the set has a flat accent");

        let raw = serde_json::json!({ "ideas": [
            { "idea": "one", "accent": gradient.name.to_uppercase() },
            { "idea": "two", "accent": { "name": flat.id } },
            { "idea": "three", "background": gradient.id },
        ]});
        let read = read(&conn, &profile_id, &config, &raw, None).unwrap();

        let first = read.ideas[0].concept.accent.clone().unwrap();
        assert_eq!(first.stops, gradient.colours);
        assert_eq!(first.color, gradient.colours[0]);
        assert_eq!(
            first.said(),
            format!(
                "{} (a gradient of {})",
                gradient.name,
                gradient.colours.join(" to ")
            )
        );
        let second = read.ideas[1].concept.accent.clone().unwrap();
        assert_eq!(second.color, flat.colours[0]);
        assert!(second.stops.is_empty(), "a flat accent has no stops");
        assert_eq!(
            second.said(),
            format!("{} ({})", flat.name, flat.colours[0])
        );
        assert_eq!(read.ideas[2].concept.bricks.background, None);
        assert!(
            read.dropped
                .iter()
                .any(|dropped| dropped.idea == 3 && dropped.part == "background"),
            "an accent is no ground"
        );

        let work = fixtures::video(&conn, &profile_id, "Tide");
        let sheet = choices_sheet(&conn, &work).unwrap();
        assert!(sheet.contains("Accents (`accent`)"), "{sheet}");
        assert!(
            sheet.contains(&format!(
                "`{}` {} {}",
                gradient.id,
                gradient.name,
                gradient.colours.join(" to ")
            )),
            "a gradient accent is offered with its stops: {sheet}"
        );
    }

    /// Only one idea of an answer can be the person's worked out, and only
    /// when one was asked for.
    #[test]
    fn refined_is_claimed_once_and_only_when_asked() {
        let (conn, profile_id) = studio();
        let config = crate::profile::config_for(&conn, &profile_id).unwrap();
        let raw = serde_json::json!([
            { "source": "refined", "idea": "one" },
            { "source": "refined", "idea": "two" }
        ]);
        let asked = IdeaRequest {
            count: 1,
            refine: Some("mine".into()),
            more: false,
        };
        let read_asked = read(&conn, &profile_id, &config, &raw, Some(&asked)).unwrap();
        assert_eq!(read_asked.ideas[0].source, Source::Refined);
        assert_eq!(read_asked.ideas[1].source, Source::Ai);
        let not_asked = IdeaRequest {
            count: 2,
            ..IdeaRequest::default()
        };
        let read_plain = read(&conn, &profile_id, &config, &raw, Some(&not_asked)).unwrap();
        assert!(
            read_plain
                .ideas
                .iter()
                .all(|idea| idea.source == Source::Ai)
        );
    }

    #[test]
    fn an_answer_with_nothing_readable_is_refused() {
        let (conn, profile_id) = studio();
        let config = crate::profile::config_for(&conn, &profile_id).unwrap();
        for raw in [
            serde_json::json!({}),
            serde_json::json!({ "ideas": [] }),
            serde_json::json!({ "ideas": [{ "angle": "x" }] }),
            serde_json::json!("text"),
        ] {
            assert!(
                read(&conn, &profile_id, &config, &raw, None).is_err(),
                "{raw}"
            );
        }
    }
}
