//! Reading what a cover is built from out of the workspace: the bricks, the
//! hero, the channel's card - and the views the window draws from them.
//!
//! What the channel's card gives a picture is read through the cover's lens
//! (`canon::seen_by`): only settled public facts of the sections a cover may
//! read. Each section's shape says who reads it - signature details are the
//! switches, marks the family a mark is picked from, a palette the accents,
//! bans the negative, house styles the bricks offered first, slots the
//! captions a dressing is filled with.

use rusqlite::Connection;
use serde::Serialize;

use super::framing::{self, Colours, Drawing, Layout, Scheme, Shape};
use super::prompt::{self, HeroText, Ingredients, MarkText, Prompts, Target};
use super::{Cover, MarkWay};
use crate::asset::Asset;
use crate::canon::view::CardFilter;
use crate::error::{Error, Result};
use crate::profile::config::{Label, Lens, ProfileConfig, SectionShape};
use crate::style_brick::StyleBrick;
use crate::style_set::SlotValues;
use crate::work::Work;

/// The roles of a card's pictures that are handed to a generator as the
/// hero's likeness.
const LIKENESS: [&str; 3] = ["portrait", "reference", "outfit"];

/// The slots a cover fills itself, which the captions editor does not offer.
const OWN_SLOTS: [&str; 3] = ["accent", "ink", "title"];

/// A signature detail of the channel, as a picture's switch.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(rename = "CoverDetail")]
pub struct DetailState {
    /// The fact's id: what the cover remembers its switch by.
    pub id: String,
    pub name: String,
    /// What goes into the prompt, the cards it names written in.
    pub template: String,
    /// The pictures it belongs in: `cover`, `frame`, `scene`.
    pub places: Vec<String>,
    /// As the channel says.
    pub default_on: bool,
    /// As this picture has it.
    pub on: bool,
}

/// A variant of the channel's mark.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(rename = "CoverMarkOption")]
pub struct MarkOption {
    pub id: String,
    /// The variant's code: a status, a number.
    pub code: Option<String>,
    /// What it means.
    pub name: String,
    /// How a generator is told to draw it.
    pub prompt: String,
    /// Its file, the newest picture attached to it.
    pub file: Option<Asset>,
}

/// A colour of the channel's palette.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(rename = "CoverPaletteColour")]
pub struct PaletteColour {
    pub id: String,
    /// The words a generator is given for it: the fact up to its first dash
    /// or full stop.
    pub name: String,
    pub color: String,
}

/// A slot of the chosen lettering and dressing, and what fills it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(rename = "CoverSlot")]
pub struct SlotState {
    pub name: String,
    /// The cover's own lines.
    pub own: Vec<String>,
    /// The channel's lines - what fills it while the cover has none.
    pub channel: Vec<String>,
}

/// A shape a work's covers take: one per door of its kind that states one.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct CoverFormat {
    pub door: String,
    pub label: Label,
    pub format: String,
    /// The work already goes out through that door.
    pub held: bool,
}

/// A layout, with its starting frame drawn small.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(rename = "CoverLayoutOption")]
pub struct LayoutOption {
    pub framing: framing::Framing,
    pub scheme: Scheme,
}

/// The chosen hero.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(rename = "CoverHeroState")]
pub struct HeroState {
    pub id: String,
    pub title: Option<String>,
    pub kind: String,
    /// The card's description for a generator, when it has one.
    pub description: Option<String>,
    /// The pictures of the hero's likeness.
    pub pictures: Vec<Asset>,
}

/// A file to hand the generator with the prompt.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(rename = "CoverReference")]
pub struct Reference {
    pub asset: Asset,
    /// What it is for: `hero`, `mark`, `style`.
    pub role: String,
}

/// Something the window should say about a cover: a part that is chosen and
/// cannot be used as it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(rename = "CoverProblem")]
pub enum Problem {
    /// A chosen brick is not in the dictionary any more.
    BrickGone { place: String },
    /// The hero's card is gone.
    HeroGone,
    /// The hero's card has no description for a generator yet, so the
    /// prompt does not name the hero.
    HeroUndescribed { title: String },
    /// The chosen variant of the mark is gone from the channel.
    MarkGone,
    /// The mark is to be laid over the picture, and the variant has no file.
    MarkWithoutFile { code: String },
    /// A slot of the chosen lettering or dressing has nothing to say: its
    /// phrase drops out of the prompt.
    SlotEmpty { slot: String },
}

/// What the Cover tab draws: the prompt, the scheme, and everything the
/// constructor offers from the workspace.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct CoverView {
    /// Anything is chosen in the constructor; a cover that is not is its
    /// own words exactly.
    pub built: bool,
    pub prompts: Prompts,
    /// The shape the prompt and the scheme are written for.
    pub format: String,
    pub formats: Vec<CoverFormat>,
    pub scheme: Option<Scheme>,
    pub layouts: Vec<LayoutOption>,
    /// The title as lettered.
    pub title: String,
    /// The title of what the publication is made from: the default.
    pub source_title: String,
    pub slots: Vec<SlotState>,
    pub details: Vec<DetailState>,
    pub marks: Vec<MarkOption>,
    pub palette: Vec<PaletteColour>,
    /// The channel's house styles, by brick id: offered first.
    pub house_styles: Vec<String>,
    pub hero: Option<HeroState>,
    pub references: Vec<Reference>,
    pub problems: Vec<Problem>,
    /// The prompt was copied out before and has changed since.
    pub sent_differs: bool,
    /// Where the mark's file is laid over the picture at export, as shares
    /// of its width and height - the box the scheme draws it in. Absent
    /// unless a variant is laid over in a corner.
    pub mark_box: Option<framing::Rect>,
}

/// What the channel's card gives a picture.
#[derive(Debug, Clone, Default)]
pub struct Channel {
    pub details: Vec<DetailState>,
    pub marks: Vec<MarkOption>,
    pub palette: Vec<PaletteColour>,
    pub bans: Vec<String>,
    pub house_styles: Vec<String>,
    pub slots: SlotValues,
}

impl Channel {
    /// Read every root card of the profile through the cover's lens.
    pub fn read(conn: &Connection, profile_id: &str, config: &ProfileConfig) -> Result<Self> {
        let mut channel = Channel::default();
        let roots = crate::canon::view::cards(conn, profile_id, &CardFilter::default())?
            .into_iter()
            .filter(|card| config.card_kind(&card.kind).is_some_and(|kind| kind.root));
        for root in roots {
            let Some(kind) = config.card_kind(&root.kind) else {
                continue;
            };
            let view = crate::canon::view::card(conn, &root.id)?;
            for read in &view.facts {
                if !read.lenses.contains(&Lens::Cover) {
                    continue;
                }
                let fact = &read.fact;
                let Some(section) = kind.section(&fact.section) else {
                    continue;
                };
                let text = |key: &str| {
                    fact.data
                        .get(key)
                        .and_then(|value| value.as_str())
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned)
                };
                let body = fact.body.trim().to_owned();
                match section.shape {
                    SectionShape::Details => {
                        let template = crate::canon::view::expand_cards(
                            conn,
                            &text("template").unwrap_or_else(|| body.clone()),
                        )?;
                        let places = fact
                            .data
                            .get("places")
                            .and_then(|value| value.as_array())
                            .map(|places| {
                                places
                                    .iter()
                                    .filter_map(|place| place.as_str().map(str::to_owned))
                                    .collect()
                            })
                            .unwrap_or_else(|| {
                                crate::canon::fact::DETAIL_PLACES
                                    .iter()
                                    .map(|place| (*place).to_owned())
                                    .collect()
                            });
                        let default_on = fact
                            .data
                            .get("on")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(true);
                        channel.details.push(DetailState {
                            id: fact.id.clone(),
                            name: body,
                            template,
                            places,
                            default_on,
                            on: default_on,
                        });
                    }
                    SectionShape::Marks => {
                        let file = view
                            .pictures
                            .iter()
                            .rfind(|picture| picture.canon_fact_id.as_deref() == Some(&fact.id))
                            .cloned();
                        channel.marks.push(MarkOption {
                            id: fact.id.clone(),
                            code: text("code"),
                            prompt: text("prompt").unwrap_or_else(|| body.clone()),
                            name: body,
                            file,
                        });
                    }
                    SectionShape::Palette => {
                        if let Some(color) = text("color") {
                            channel.palette.push(PaletteColour {
                                id: fact.id.clone(),
                                name: colour_name(&body),
                                color,
                            });
                        }
                    }
                    SectionShape::Bans => {
                        if !body.is_empty() {
                            channel.bans.push(body);
                        }
                    }
                    SectionShape::Styles => {
                        if let Some(id) = text("styleId") {
                            channel.house_styles.push(id);
                        }
                    }
                    SectionShape::Slots => {
                        if let Some(slot) = text("slot")
                            && !body.is_empty()
                        {
                            channel.slots.entry(slot).or_default().push(body);
                        }
                    }
                    SectionShape::Facts | SectionShape::Relations | SectionShape::Appearances => {}
                }
            }
        }
        Ok(channel)
    }

    /// The details that belong in `target`, each switched as `cover` has it.
    pub fn details_for(&self, cover: &Cover, target: Target) -> Vec<DetailState> {
        self.details
            .iter()
            .filter(|detail| detail.places.iter().any(|place| place == target.place()))
            .map(|detail| DetailState {
                on: cover
                    .details
                    .get(&detail.id)
                    .copied()
                    .unwrap_or(detail.default_on),
                ..detail.clone()
            })
            .collect()
    }
}

/// The words a generator is given for a palette's colour: "Lime - the left
/// eye of the mark" is "Lime".
fn colour_name(body: &str) -> String {
    let cut = [" - ", " — ", " – ", ". ", ": ", ", "]
        .iter()
        .filter_map(|mark| body.find(mark))
        .min()
        .unwrap_or(body.len());
    body[..cut].trim().trim_end_matches('.').to_owned()
}

/// What a cover's chosen parts read as, and what is wrong with them.
struct Resolved {
    parts: Ingredients,
    hero: Option<HeroState>,
    references: Vec<Reference>,
    problems: Vec<Problem>,
}

/// The title a cover letters: the song's, for its clip, its audio and its
/// shorts - a short is often cut from the clip, and "the song - clip" is a
/// name for the catalogue. The nearest work up the chain it was made from
/// that never goes out itself (a kind with no doors), or else the furthest
/// one up, or else the work's own.
fn source_title(conn: &Connection, config: &ProfileConfig, work: &Work) -> Result<String> {
    crate::publication::origin_title(conn, config, work)
}

/// Read what `cover` names, for `target`.
fn resolve(
    conn: &Connection,
    config: &ProfileConfig,
    channel: &Channel,
    cover: &Cover,
    target: Target,
    title: String,
) -> Result<Resolved> {
    let mut problems = Vec::new();
    let mut references = Vec::new();
    let mut brick = |id: Option<&str>, place: &str| -> Result<Option<StyleBrick>> {
        let Some(id) = id else {
            return Ok(None);
        };
        let found = crate::style_brick::get(conn, id)?;
        if found.is_none() {
            problems.push(Problem::BrickGone {
                place: place.to_owned(),
            });
        }
        Ok(found)
    };
    let style = brick(cover.bricks.style.as_deref(), "style")?;
    let lettering = target.lettering();
    let typography = if lettering {
        brick(cover.bricks.typography.as_deref(), "typography")?
    } else {
        None
    };
    let dressing = if lettering {
        brick(cover.bricks.dressing.as_deref(), "dressing")?
    } else {
        None
    };
    let background = if target == Target::Scene {
        None
    } else {
        brick(cover.bricks.background.as_deref(), "background")?
    };

    if let Some(style) = &style {
        for picture in crate::asset::for_style_brick(conn, &style.id)? {
            references.push(Reference {
                asset: picture,
                role: "style".into(),
            });
        }
    }

    let mut bans = channel.bans.clone();
    let mut hero_state = None;
    let mut hero_text = None;
    if let Some(card) = cover.hero_card() {
        match crate::note::get(conn, card)? {
            None => problems.push(Problem::HeroGone),
            Some(note) => {
                let pictures: Vec<Asset> = crate::asset::for_card(conn, &note.id)?
                    .into_iter()
                    .filter(|picture| {
                        picture.canon_fact_id.is_none() && LIKENESS.contains(&picture.kind.as_str())
                    })
                    .collect();
                let description = note
                    .prompt
                    .as_deref()
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .map(str::to_owned);
                if description.is_none() {
                    problems.push(Problem::HeroUndescribed {
                        title: note.title.clone().unwrap_or_default(),
                    });
                }
                let handed =
                    cover.hero.as_ref().is_some_and(|hero| hero.references) && !pictures.is_empty();
                if handed {
                    references.extend(pictures.iter().map(|asset| Reference {
                        asset: asset.clone(),
                        role: "hero".into(),
                    }));
                }
                bans.extend(card_bans(conn, config, &note.id)?);
                hero_text = Some(HeroText {
                    description: description.clone(),
                    references: handed,
                });
                hero_state = Some(HeroState {
                    id: note.id,
                    title: note.title,
                    kind: note.kind,
                    description,
                    pictures,
                });
            }
        }
    }

    let mut mark = None;
    if target != Target::Scene
        && let Some(variant) = cover.mark.variant.as_deref()
    {
        match channel.marks.iter().find(|option| option.id == variant) {
            None => problems.push(Problem::MarkGone),
            Some(option) => {
                if cover.mark.drawn() {
                    mark = Some(MarkText {
                        prompt: option.prompt.clone(),
                        has_file: option.file.is_some(),
                    });
                    if let Some(file) = &option.file {
                        references.push(Reference {
                            asset: file.clone(),
                            role: "mark".into(),
                        });
                    }
                } else if cover.mark.laid_over().is_some()
                    && cover.mark.way == MarkWay::Overlay
                    && option.file.is_none()
                {
                    problems.push(Problem::MarkWithoutFile {
                        code: option.code.clone().unwrap_or_else(|| option.name.clone()),
                    });
                }
            }
        }
    }

    let details = channel
        .details_for(cover, target)
        .into_iter()
        .filter(|detail| detail.on)
        .map(|detail| detail.template)
        .collect();

    let parts = Ingredients {
        style: style.as_ref().and_then(|brick| brick.description.clone()),
        typography: typography
            .as_ref()
            .and_then(|brick| brick.description.clone()),
        dressing: dressing
            .as_ref()
            .and_then(|brick| brick.description.clone()),
        background_colour: background
            .as_ref()
            .and_then(|brick| brick.colours.first().cloned()),
        background: background.and_then(|brick| brick.description),
        hero: hero_text,
        details,
        mark,
        bans,
        channel_slots: channel.slots.clone(),
        title,
    };
    if lettering {
        for slot in slots_of(&parts) {
            let filled = cover
                .lettering
                .captions
                .get(&slot)
                .is_some_and(|lines| lines.iter().any(|line| !line.trim().is_empty()))
                || channel.slots.contains_key(&slot);
            if !filled {
                problems.push(Problem::SlotEmpty { slot });
            }
        }
    }
    Ok(Resolved {
        parts,
        hero: hero_state,
        references,
        problems,
    })
}

/// The bans a picture of a card keeps out: its sections of bans the cover
/// may read.
fn card_bans(conn: &Connection, config: &ProfileConfig, note_id: &str) -> Result<Vec<String>> {
    let Ok((note, kind)) = crate::canon::fact::card_of(conn, config, note_id) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for one in crate::canon::fact::for_card(conn, &note.id)? {
        let section = kind.section(&one.section);
        if section.is_some_and(|section| section.shape == SectionShape::Bans)
            && crate::canon::seen_by(Lens::Cover, note.layer, section, one.layer, one.status)
        {
            out.push(one.body.trim().to_owned());
        }
    }
    Ok(out)
}

/// The slots the chosen lettering and dressing ask the cover for.
fn slots_of(parts: &Ingredients) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for text in [parts.typography.as_deref(), parts.dressing.as_deref()]
        .into_iter()
        .flatten()
    {
        for slot in crate::style_set::slots(text) {
            if !OWN_SLOTS.contains(&slot.as_str()) && !out.contains(&slot) {
                out.push(slot);
            }
        }
    }
    out
}

/// The shapes a work's covers take, held or not.
pub(super) fn formats_of(
    conn: &Connection,
    config: &ProfileConfig,
    work: &Work,
) -> Result<Vec<CoverFormat>> {
    let doors: Vec<String> = crate::release::for_work(conn, &work.profile_id, &work.id)?
        .into_iter()
        .map(|scheduled| scheduled.release.kind)
        .collect();
    Ok(config
        .vocabulary(&work.kind)
        .release_kinds
        .iter()
        .filter_map(|door| {
            let format = door.cover_format.as_deref()?.trim();
            Shape::parse(format)?;
            Some(CoverFormat {
                door: door.key.clone(),
                label: door.label.clone(),
                format: format.to_owned(),
                held: doors.contains(&door.key),
            })
        })
        .collect())
}

/// The shape asked for, when the work has a door of it; else the first door
/// it goes out through; else its kind's first; else a square.
pub(super) fn shape_for(formats: &[CoverFormat], asked: Option<&str>) -> Shape {
    let pick = asked
        .and_then(|asked| formats.iter().find(|format| format.format == asked.trim()))
        .or_else(|| formats.iter().find(|format| format.held))
        .or_else(|| formats.first());
    pick.and_then(|format| Shape::parse(&format.format))
        .unwrap_or(Shape::SQUARE)
}

/// The eight layouts, drawn small in the cover's colours.
fn layouts(shape: Shape, lettering: bool, colours: &Colours, accent: bool) -> Vec<LayoutOption> {
    Layout::ALL
        .into_iter()
        .map(|layout| {
            let framing = layout.defaults();
            LayoutOption {
                scheme: framing.scheme(Drawing {
                    shape,
                    lettering,
                    colours,
                    accent,
                    mark: None,
                }),
                framing,
            }
        })
        .collect()
}

/// The Cover tab of a work, written for `format` - one of its doors' shapes.
pub fn view(conn: &Connection, work_id: &str, format: Option<&str>) -> Result<CoverView> {
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let channel = Channel::read(conn, &work.profile_id, &config)?;
    let cover = &work.cover;
    let formats = formats_of(conn, &config, &work)?;
    let shape = shape_for(&formats, format);
    let source_title = source_title(conn, &config, &work)?;
    let title = cover
        .lettering
        .title
        .clone()
        .unwrap_or_else(|| source_title.clone());

    let resolved = resolve(conn, &config, &channel, cover, Target::Cover, title.clone())?;
    let prompts = prompt::build(cover, Target::Cover, shape, &resolved.parts);
    let colours = Colours::on(
        resolved.parts.background_colour.as_deref(),
        cover.accent.as_ref().map(|accent| accent.color.as_str()),
    );
    let accent = cover.accent.is_some();
    let scheme = cover.framing.map(|framing| {
        framing.scheme(Drawing {
            shape,
            lettering: true,
            colours: &colours,
            accent,
            mark: cover.mark.in_corner(),
        })
    });
    let slots = slots_of(&resolved.parts)
        .into_iter()
        .map(|name| SlotState {
            own: cover
                .lettering
                .captions
                .get(&name)
                .cloned()
                .unwrap_or_default(),
            channel: channel.slots.get(&name).cloned().unwrap_or_default(),
            name,
        })
        .collect();
    let sent_differs = cover.sent.as_ref().is_some_and(|sent| {
        sent.picture != prompts.picture
            || sent.negative != prompts.negative
            || sent.typography != prompts.typography
    });

    Ok(CoverView {
        mark_box: cover
            .mark
            .laid_over()
            .map(|corner| framing::mark_box(shape, corner)),
        built: cover.is_built(),
        format: shape.name(),
        layouts: layouts(Shape::SQUARE, true, &colours, accent),
        details: channel.details_for(cover, Target::Cover),
        marks: channel.marks.clone(),
        palette: channel.palette.clone(),
        house_styles: channel.house_styles.clone(),
        hero: resolved.hero,
        references: resolved.references,
        problems: resolved.problems,
        prompts,
        formats,
        scheme,
        title,
        source_title,
        slots,
        sent_differs,
    })
}

/// The Frame tab of a work that plays under one picture.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct FrameView {
    pub prompts: crate::work::frame::FramePrompts,
    /// The still is built from the cover, rather than written whole.
    pub from_cover: bool,
    pub format: String,
    pub scheme: Option<Scheme>,
    pub layouts: Vec<LayoutOption>,
    /// The channel's details that belong in a frame, switched as the cover
    /// has them.
    pub details: Vec<DetailState>,
    pub problems: Vec<Problem>,
}

/// A work's frame: the still built from its cover without the words, or
/// written whole, and the loop written from its settings.
pub fn frame_view(conn: &Connection, work_id: &str) -> Result<FrameView> {
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let formats = formats_of(conn, &config, &work)?;
    // A video frame takes the shape of the first door that shows one - the
    // video platform's, before the square of a streaming cover.
    let shape = formats
        .first()
        .and_then(|format| Shape::parse(&format.format))
        .unwrap_or(Shape {
            width: 16,
            height: 9,
        });
    let frame = &work.frame;
    let written = frame.prompts();
    if !frame.from_cover() {
        return Ok(FrameView {
            prompts: written,
            from_cover: false,
            format: shape.name(),
            scheme: None,
            layouts: Vec::new(),
            details: Vec::new(),
            problems: Vec::new(),
        });
    }

    let channel = Channel::read(conn, &work.profile_id, &config)?;
    let mut cover = work.cover.clone();
    if let Some(framing) = frame.framing {
        cover.framing = Some(framing);
    }
    let resolved = resolve(
        conn,
        &config,
        &channel,
        &cover,
        Target::Frame,
        String::new(),
    )?;
    let built = prompt::build(&cover, Target::Frame, shape, &resolved.parts);
    let colours = Colours::on(
        resolved.parts.background_colour.as_deref(),
        cover.accent.as_ref().map(|accent| accent.color.as_str()),
    );
    let accent = cover.accent.is_some();
    let scheme = cover.framing.map(|framing| {
        framing.scheme(Drawing {
            shape,
            lettering: false,
            colours: &colours,
            accent,
            mark: None,
        })
    });
    let join = |built: String, own: &str| {
        let own = own.trim();
        if own.is_empty() {
            built
        } else if built.is_empty() {
            own.to_owned()
        } else {
            format!("{built}\n\n{own}")
        }
    };
    Ok(FrameView {
        prompts: crate::work::frame::FramePrompts {
            still: join(built.picture, &frame.still),
            loop_: written.loop_,
            negative: [built.negative, frame.negative.trim().to_owned()]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
        },
        from_cover: true,
        format: shape.name(),
        scheme,
        layouts: layouts(shape, false, &colours, accent),
        details: channel.details_for(&cover, Target::Frame),
        problems: resolved.problems,
    })
}

/// A scene's frame, built: what the drawer shows and copies.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct SceneFrameView {
    /// The still, built around the scene's picture block. Absent while the
    /// scene has no built frame.
    pub still: Option<String>,
    pub negative: Option<String>,
    pub format: String,
    pub scheme: Option<Scheme>,
    pub layouts: Vec<LayoutOption>,
    /// The block of the scene the still is built around.
    pub block: Option<String>,
    pub problems: Vec<Problem>,
}

/// A scene's frame: the clip's cover without the words, its style and
/// details, the scene's own picture block as the scene, and the characters
/// it is about as its heroes.
pub fn scene_view(conn: &Connection, scene_id: &str) -> Result<SceneFrameView> {
    let scene =
        crate::scene::get(conn, scene_id)?.ok_or_else(|| Error::not_found("scene", scene_id))?;
    let work = crate::work::get(conn, &scene.work_id)?
        .ok_or_else(|| Error::not_found("work", &scene.work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let kind = config.vocabulary(&work.kind);
    let formats = formats_of(conn, &config, &work)?;
    let shape = formats
        .first()
        .and_then(|format| Shape::parse(&format.format))
        .unwrap_or(Shape {
            width: 16,
            height: 9,
        });
    let block = kind
        .scene_blocks
        .iter()
        .find(|block| block.picture)
        .map(|block| block.key.clone());
    let colours = Colours::on(None, work.cover.accent.as_ref().map(|a| a.color.as_str()));
    let accent = work.cover.accent.is_some();
    let layouts = layouts(shape, false, &colours, accent);
    let Some(framing) = scene.framing else {
        return Ok(SceneFrameView {
            still: None,
            negative: None,
            format: shape.name(),
            scheme: None,
            layouts,
            block,
            problems: Vec::new(),
        });
    };

    let channel = Channel::read(conn, &work.profile_id, &config)?;
    let scene_text = block
        .as_deref()
        .and_then(|key| scene.blocks.get(key))
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_owned();
    // The clip's cover gives the style, the accent and the switches; the
    // scene gives its frame and what it shows. Its characters are its heroes.
    let cover = Cover {
        scene: scene_text,
        framing: Some(framing),
        bricks: super::Bricks {
            style: work.cover.bricks.style.clone(),
            ..super::Bricks::default()
        },
        accent: work.cover.accent.clone(),
        details: work.cover.details.clone(),
        ..Cover::default()
    };
    let mut resolved = resolve(
        conn,
        &config,
        &channel,
        &cover,
        Target::Scene,
        String::new(),
    )?;
    let mut heroes: Vec<String> = Vec::new();
    for link in crate::scene_note::for_scene(conn, &scene.id)? {
        if config.card_kind(&link.note_kind).is_none() {
            continue;
        }
        if let Some(note) = crate::note::get(conn, &link.note_id)? {
            match note
                .prompt
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty())
            {
                Some(description) => heroes.push(description.to_owned()),
                None => resolved.problems.push(Problem::HeroUndescribed {
                    title: note.title.unwrap_or_default(),
                }),
            }
            resolved
                .parts
                .bans
                .extend(card_bans(conn, &config, &link.note_id)?);
        }
    }
    if !heroes.is_empty() {
        resolved.parts.hero = Some(HeroText {
            description: Some(heroes.join(" Also in the picture: ")),
            references: false,
        });
    }
    let built = prompt::build(&cover, Target::Scene, shape, &resolved.parts);
    Ok(SceneFrameView {
        still: Some(built.picture),
        negative: Some(built.negative),
        format: shape.name(),
        scheme: Some(framing.scheme(Drawing {
            shape,
            lettering: false,
            colours: &colours,
            accent,
            mark: None,
        })),
        layouts,
        block,
        problems: resolved.problems,
    })
}

#[cfg(test)]
mod tests {
    use super::super::framing::Layout;
    use super::super::{Accent, Bricks, Hero, MarkChoice, MarkPlace};
    use super::*;
    use crate::fixtures;
    use crate::work::WorkPatch;
    use serde_json::{Map, Value, json};

    /// A fact of a card with the data its section's shape carries.
    fn fact_with(conn: &Connection, card: &str, section: &str, body: &str, data: Value) -> String {
        crate::canon::fact::create_minted(
            conn,
            crate::canon::NewFact {
                note_id: card.into(),
                section: section.into(),
                body: body.into(),
                data: data.as_object().cloned().unwrap_or_else(Map::new),
                ..crate::canon::NewFact::default()
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap()
        .id
    }

    /// The id of the set brick `key`.
    fn set_brick(conn: &Connection, profile_id: &str, key: &str) -> String {
        crate::style_brick::list(conn, profile_id, &Default::default())
            .unwrap()
            .into_iter()
            .find(|brick| brick.set_key.as_deref() == Some(key))
            .unwrap_or_else(|| panic!("the set has `{key}`"))
            .id
    }

    struct Stage {
        conn: Connection,
        profile_id: String,
        clip: Work,
        hero: String,
        mark: String,
        cat: String,
    }

    /// A song and its clip, the channel's card with details of each place, a
    /// mark, a colour and bans of each kind, and a described hero.
    fn stage() -> Stage {
        let (conn, profile_id) = fixtures::workspace();
        crate::style_set::seed(&conn).unwrap();
        let song = fixtures::song(&conn, &profile_id, "Northern Light");
        let clip = fixtures::work(&conn, &profile_id, "video", "Northern Light — clip");
        crate::link::create(
            &conn,
            &profile_id,
            crate::link::NewLink {
                work_id: clip.id.clone(),
                source_id: song.id.clone(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();

        let channel = fixtures::card(&conn, &profile_id, "channel", "The channel");
        let cat = fact_with(
            &conn,
            &channel.id,
            "details",
            "A cat hides",
            json!({ "template": "a small black cat hides somewhere", "places": ["cover", "frame"], "on": true }),
        );
        fact_with(
            &conn,
            &channel.id,
            "details",
            "A stamp",
            json!({ "template": "a red stamp in a corner", "places": ["cover"], "on": false }),
        );
        fact_with(
            &conn,
            &channel.id,
            "details",
            "A leaf falls",
            json!({ "template": "a leaf falls into a tin box", "places": ["scene"], "on": true }),
        );
        let mark = fact_with(
            &conn,
            &channel.id,
            "mark",
            "Tear - for elegies",
            json!({ "code": "410", "prompt": "a cat head with a tear in its eye" }),
        );
        fact_with(
            &conn,
            &channel.id,
            "palette",
            "Hot pink - the accent",
            json!({ "color": "#FF2E63" }),
        );
        fixtures::fact(&conn, &channel.id, "picture_bans", "no skulls");
        fixtures::fact(&conn, &channel.id, "bans", "never call it content");

        let hero = fixtures::card(&conn, &profile_id, "character", "Wren");
        crate::note::update(
            &conn,
            &hero.id,
            crate::note::NotePatch {
                prompt: Some(Some("a small woman with a red scarf".into())),
                ..Default::default()
            },
        )
        .unwrap();
        fixtures::fact(&conn, &hero.id, "picture_bans", "no tattoos on her");
        Stage {
            conn,
            profile_id,
            clip,
            hero: hero.id,
            mark,
            cat,
        }
    }

    fn cover_of(stage: &Stage) -> Cover {
        let brick = |key: &str| Some(set_brick(&stage.conn, &stage.profile_id, key));
        Cover {
            scene: "she waits at the end of a pier".into(),
            hero: Some(Hero {
                card: stage.hero.clone(),
                references: true,
            }),
            framing: Some(Layout::Masthead.defaults()),
            bricks: Bricks {
                style: brick("neo-traditional"),
                typography: brick("dry-brush"),
                dressing: brick("minimal"),
                background: brick("bg-ink"),
            },
            accent: Some(Accent {
                name: "hot pink".into(),
                color: "#FF2E63".into(),
            }),
            mark: MarkChoice {
                variant: Some(stage.mark.clone()),
                place: MarkPlace::Hidden,
                ..MarkChoice::default()
            },
            ..Cover::default()
        }
    }

    fn write(stage: &Stage, cover: Cover) {
        crate::work::update(
            &stage.conn,
            &stage.clip.id,
            WorkPatch {
                cover: Some(cover),
                ..WorkPatch::default()
            },
        )
        .unwrap();
    }

    /// Everything a cover names reaches its prompt from where it lives: the
    /// song's title, the hero's description, the bricks' words with the slots
    /// filled from the cover, the channel's details as switched, the mark,
    /// and the bans a picture keeps out - and only those.
    #[test]
    fn a_built_cover_is_written_from_the_workspace() {
        let stage = stage();
        write(&stage, cover_of(&stage));

        let view = view(&stage.conn, &stage.clip.id, None).unwrap();
        let picture = &view.prompts.picture;
        assert!(view.built);
        assert_eq!(view.format, "16:9");
        assert_eq!(
            view.title, "Northern Light",
            "the song's title, not the clip's"
        );
        for part in [
            "FRAME: a wide 16:9 cover.",
            "STYLE: neo-traditional tattoo art",
            "HERO: a small woman with a red scarf.",
            "SCENE: she waits at the end of a pier.",
            "BACKGROUND: perfectly flat solid ink black",
            "DETAIL: a small black cat hides somewhere.",
            "CHANNEL MARK: a cat head with a tear in its eye, hidden somewhere in the picture, small.",
            "TITLE: the words «Northern Light», spelled exactly, set huge across the full width behind the hero's head",
            "dry-brush hand lettering with rough bristle texture, fast strokes and ink splatter, in off-white.",
            "A large flat circle in hot pink (#FF2E63)",
        ] {
            assert!(picture.contains(part), "`{part}` in: {picture}");
        }
        assert!(
            !picture.contains("attached reference"),
            "a hero with no pictures hands none over"
        );
        assert!(
            !picture.contains("red stamp"),
            "a detail the channel keeps off stays off"
        );
        assert!(
            !picture.contains("leaf falls"),
            "a detail of scenes is not a cover's"
        );
        assert!(view.prompts.negative.contains("no skulls."));
        assert!(
            view.prompts.negative.contains("no tattoos on her."),
            "the hero's own bans"
        );
        assert!(
            !view.prompts.negative.contains("content"),
            "a ban of texts does not reach a picture: {}",
            view.prompts.negative
        );
        assert_eq!(view.details.len(), 2, "the details that belong on a cover");
        assert_eq!(view.marks.len(), 1);
        assert_eq!(view.palette[0].name, "Hot pink");
        assert!(view.problems.contains(&Problem::SlotEmpty {
            slot: "brand".into()
        }));
        assert!(view.scheme.is_some());
        assert!(view.mark_box.is_none(), "a mark drawn in is not laid over");
    }

    #[test]
    fn a_detail_switched_on_this_cover_goes_against_the_channel() {
        let stage = stage();
        let mut cover = cover_of(&stage);
        cover.details.insert(stage.cat.clone(), false);
        write(&stage, cover);

        let view = view(&stage.conn, &stage.clip.id, None).unwrap();
        assert!(!view.prompts.picture.contains("cat hides"));
        let cat = view.details.iter().find(|d| d.id == stage.cat).unwrap();
        assert!(!cat.on && cat.default_on);
    }

    #[test]
    fn a_mark_laid_over_is_not_asked_of_the_generator_and_has_its_box() {
        let stage = stage();
        let mut cover = cover_of(&stage);
        cover.mark.place = MarkPlace::Corner;
        write(&stage, cover);

        let view = view(&stage.conn, &stage.clip.id, None).unwrap();
        assert!(!view.prompts.picture.contains("CHANNEL MARK"));
        assert!(view.mark_box.is_some());
        assert!(
            view.problems
                .contains(&Problem::MarkWithoutFile { code: "410".into() }),
            "the variant has no file to lay over yet"
        );
    }

    /// A cover that names what is not there is refused at the door: a brick
    /// of another form in a place, a hero that is no card.
    #[test]
    fn a_cover_naming_what_is_not_there_is_refused() {
        let stage = stage();
        let mut wrong = cover_of(&stage);
        wrong.bricks.typography = wrong.bricks.background.clone();
        let refused = crate::work::update(
            &stage.conn,
            &stage.clip.id,
            WorkPatch {
                cover: Some(wrong),
                ..WorkPatch::default()
            },
        );
        assert!(
            matches!(refused, Err(crate::error::Error::Refused(ref r)) if r.code == "cover.brickOfOtherForm"),
            "{refused:?}"
        );
        let mut nobody = cover_of(&stage);
        nobody.hero = Some(Hero {
            card: "no-such-card".into(),
            references: true,
        });
        let refused = crate::work::update(
            &stage.conn,
            &stage.clip.id,
            WorkPatch {
                cover: Some(nobody),
                ..WorkPatch::default()
            },
        );
        assert!(
            matches!(refused, Err(crate::error::Error::Refused(ref r)) if r.code == "cover.unknownHero"),
            "{refused:?}"
        );
    }

    /// The frame of an audio release is its cover without the words: the
    /// same hero and style, the details of frames, no title, no dressing.
    #[test]
    fn a_frame_is_built_from_the_cover_without_its_words() {
        let stage = stage();
        let audio = fixtures::work(
            &stage.conn,
            &stage.profile_id,
            "audio",
            "Northern Light — audio",
        );
        crate::work::update(
            &stage.conn,
            &audio.id,
            WorkPatch {
                cover: Some(cover_of(&stage)),
                ..WorkPatch::default()
            },
        )
        .unwrap();

        let frame = frame_view(&stage.conn, &audio.id).unwrap();
        assert!(frame.from_cover, "a frame nobody wrote is built");
        let still = &frame.prompts.still;
        assert!(still.starts_with("FRAME: a wide 16:9 video frame that stays on screen"));
        assert!(still.contains("HERO: a small woman with a red scarf"));
        assert!(still.contains("DETAIL: a small black cat hides somewhere."));
        assert!(!still.contains("TITLE"), "{still}");
        assert!(!still.contains("barcode"), "no dressing: {still}");
        assert!(frame.prompts.loop_.starts_with("LOOP (6 s)"));
        assert!(frame.scheme.is_some());
    }

    /// A scene with a built frame is written around its picture block, in
    /// the clip's style, with the scene's characters as its heroes.
    #[test]
    fn a_scene_is_built_around_its_picture_block() {
        let stage = stage();
        write(&stage, cover_of(&stage));
        let scene = crate::scene::create(
            &stage.conn,
            &stage.profile_id,
            crate::scene::NewScene {
                work_id: stage.clip.id.clone(),
                blocks: json!({ "still": "a brass spyglass on a windowsill" })
                    .as_object()
                    .cloned(),
                ..Default::default()
            },
        )
        .unwrap();
        let before = scene_view(&stage.conn, &scene.id).unwrap();
        assert_eq!(before.still, None, "no built frame until one is picked");
        assert_eq!(before.block.as_deref(), Some("still"));

        crate::scene_note::attach(&stage.conn, &scene.id, &stage.hero).unwrap();
        crate::scene::update(
            &stage.conn,
            &scene.id,
            crate::scene::ScenePatch {
                framing: Some(Some(Layout::CloseUp.defaults())),
                ..Default::default()
            },
        )
        .unwrap();

        let built = scene_view(&stage.conn, &scene.id).unwrap();
        let still = built.still.unwrap();
        assert!(
            still.contains("SCENE: a brass spyglass on a windowsill."),
            "{still}"
        );
        assert!(
            still.contains("HERO: a small woman with a red scarf."),
            "{still}"
        );
        assert!(
            still.contains("STYLE: neo-traditional tattoo art"),
            "the clip's style"
        );
        assert!(
            still.contains("DETAIL: a leaf falls into a tin box."),
            "a detail of scenes"
        );
        assert!(
            !still.contains("cat hides"),
            "a detail of covers and frames only"
        );
        assert!(
            !still.contains("BACKGROUND"),
            "a scene keeps its own surroundings"
        );
        assert!(built.negative.unwrap().contains("no tattoos on her."));
    }

    /// A short cut from the clip letters the song's title, not "the song -
    /// clip": the title is read up the chain to what never goes out itself.
    #[test]
    fn a_short_cut_from_the_clip_letters_the_songs_title() {
        let stage = stage();
        let short = fixtures::work(
            &stage.conn,
            &stage.profile_id,
            "short",
            "Northern Light · short 1",
        );
        crate::link::create(
            &stage.conn,
            &stage.profile_id,
            crate::link::NewLink {
                work_id: short.id.clone(),
                source_id: stage.clip.id.clone(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();
        let view = view(&stage.conn, &short.id, None).unwrap();
        assert_eq!(view.source_title, "Northern Light");
        assert_eq!(view.format, "9:16");
    }

    #[test]
    fn a_colour_is_named_by_its_first_words() {
        assert_eq!(colour_name("Lime - the left eye of the mark."), "Lime");
        assert_eq!(colour_name("Paper. The ground of the print."), "Paper");
        assert_eq!(colour_name("Hot pink"), "Hot pink");
        assert_eq!(colour_name("Ink black, the second paint"), "Ink black");
    }
}
