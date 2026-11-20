//! Notes brought in from a vault as they were written, turned into what
//! kilna keeps them as (v0.147, FR-10): places into cards of the canon,
//! section by section; the channel's banners into kits of its dressing; the
//! reviews of its videos into documents on their works; one long note of
//! edits into a note on each work it names; a list of covers to redraw into
//! the mark the Catalogue filters by.
//!
//! Which notes and which headings go where is a file of rules, because it
//! is said in the owner's words - the titles and headings of their vault -
//! and kilna ships the engine, not the words (the reference has an example).
//! Nothing is written unless `--apply` is said: the plan and its report come
//! first; applied, a copy of the space is kept beside the report and all of
//! it lands as one gesture. The notes that were converted stay unless
//! `--remove-notes` is said too, and then go to the trash, from where they
//! come back.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::canon::{NewCanonLink, NewFact};
use crate::error::{Error, Result};
use crate::note::{NewNote, Note, NoteFilter, NotePatch};
use crate::profile::config::{ProfileConfig, SectionShape};

/// What to convert, in the owner's words.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    /// Notes turned into cards of a kind, heading by heading.
    #[serde(default)]
    pub cards: Vec<CardRule>,
    /// Notes turned into kits of the channel's dressing.
    #[serde(default)]
    pub kits: Option<KitRule>,
    /// Notes made documents of a kind, where they stand.
    #[serde(default)]
    pub documents: Vec<DocumentRule>,
    /// One note split into a note on each work a section of it names.
    #[serde(default)]
    pub splits: Vec<SplitRule>,
    /// One note naming works a section each: the works are marked.
    #[serde(default)]
    pub marks: Vec<MarkRule>,
}

/// Notes whose title starts with `title_prefix`, each made a card of `kind`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardRule {
    pub title_prefix: String,
    pub kind: String,
    /// The start of a heading, case ignored, and where its text goes:
    /// `section`, or `section:slot` for a section of named values.
    #[serde(default)]
    pub headings: BTreeMap<String, String>,
    /// Where the text under a heading the rule does not name goes.
    pub fallback: String,
    /// Titles, after the prefix, left as notes.
    #[serde(default)]
    pub skip: Vec<String>,
    /// A card named as another and a dash and more - "Cape-1960s" beside
    /// "Cape" - is drawn as that one's variant.
    #[serde(default)]
    pub variants: bool,
    /// Whether the card is named by the heading the note opens with, rather
    /// than by its title after the prefix. True when not said.
    #[serde(default = "named_by_heading")]
    pub name_from_heading: bool,
}

fn named_by_heading() -> bool {
    true
}

/// Notes whose title starts with `title_prefix`, each made a kit.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KitRule {
    pub title_prefix: String,
    /// A word of the title that says the kit is the look before a
    /// rebranding.
    #[serde(default)]
    pub archive_marker: Option<String>,
    /// A word of the title and the kind of kit it says: avatar, banner or
    /// watermark. A banner when none is found.
    #[serde(default)]
    pub kinds: BTreeMap<String, String>,
    /// The headings - or bold labels, `**Prompt:**` - the prompt is under.
    #[serde(default)]
    pub prompt: Vec<String>,
    /// Those of what a picture keeps out.
    #[serde(default)]
    pub negative: Vec<String>,
    /// Those of the idea of it; the first found is taken.
    #[serde(default)]
    pub about: Vec<String>,
    /// Titles, after the prefix, left as notes.
    #[serde(default)]
    pub skip: Vec<String>,
}

/// Notes whose title starts with `title_prefix`, made notes of `kind`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentRule {
    pub title_prefix: String,
    pub kind: String,
    /// The title it takes: a review of a video shown on the video needs no
    /// "Analytics · Video review · " in front of it. Kept when not said.
    #[serde(default)]
    pub title: Option<String>,
}

/// The note titled `title`, a `## N. Title` section a work: each section
/// becomes a note of `kind` titled `note_title` on the work it names.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SplitRule {
    pub title: String,
    pub kind: String,
    pub note_title: String,
    /// Only works of this kind are looked for.
    #[serde(default)]
    pub work_kind: Option<String>,
}

/// The note titled `title`, a section a work to mark with `mark`; each
/// section also kept as a note of `kind` titled `note_title`, when given.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkRule {
    pub title: String,
    pub mark: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub note_title: Option<String>,
    #[serde(default)]
    pub work_kind: Option<String>,
}

/// A part of a note: the heading it stands under and its text.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub heading: Option<String>,
    pub level: usize,
    pub text: String,
}

/// A note's parts, heading by heading. A line that looks like a heading
/// inside a fence of code is text.
pub fn blocks(body: &str) -> Vec<Block> {
    let mut out = vec![Block {
        heading: None,
        level: 0,
        text: String::new(),
    }];
    let mut fenced = false;
    // The front matter a vault writes on top - `---`, keys, `---` - is not
    // the note's text.
    let mut lines = body.lines().peekable();
    if lines.peek().is_some_and(|first| first.trim() == "---") {
        lines.next();
        for line in lines.by_ref() {
            if line.trim() == "---" {
                break;
            }
        }
    }
    for line in lines {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        let level = line.chars().take_while(|c| *c == '#').count();
        if !fenced && (1..=6).contains(&level) && line[level..].starts_with(' ') {
            out.push(Block {
                heading: Some(line[level..].trim().to_owned()),
                level,
                text: String::new(),
            });
            continue;
        }
        let last = out.last_mut().expect("the first block is always there");
        last.text.push_str(line);
        last.text.push('\n');
    }
    out.retain(|block| block.heading.is_some() || !block.text.trim().is_empty());
    out
}

/// A heading without its number: "3. The cover" is "The cover".
fn unnumbered(heading: &str) -> &str {
    let rest = heading.trim_start_matches(|c: char| c.is_ascii_digit());
    if rest.len() < heading.len()
        && let Some(after) = rest.strip_prefix(['.', ')'])
    {
        return after.trim_start();
    }
    heading
}

/// The rule a heading answers to: the longest key it starts with.
fn matched<'r>(rules: &'r BTreeMap<String, String>, heading: &str) -> Option<(&'r str, &'r str)> {
    let said = unnumbered(heading).to_lowercase();
    rules
        .iter()
        .filter(|(key, _)| said.starts_with(&key.to_lowercase()))
        .max_by_key(|(key, _)| key.chars().count())
        .map(|(key, to)| (key.as_str(), to.as_str()))
}

/// What a heading says after the rule's word, its dashes, bars, colons and
/// brackets taken off: "Prompt block - REALITY" after "Prompt block" is
/// "REALITY".
fn rest_of(heading: &str, key: &str) -> String {
    let said = unnumbered(heading);
    let skip = said
        .char_indices()
        .nth(key.chars().count())
        .map_or(said.len(), |(at, _)| at);
    said[skip..]
        .trim_matches(|c: char| {
            c.is_whitespace() || matches!(c, '-' | '—' | '–' | '|' | ':' | '(' | ')' | '.')
        })
        .to_owned()
}

/// Whether a heading names one of `labels`, case ignored.
fn names(labels: &[String], heading: &str) -> bool {
    let said = unnumbered(heading).to_lowercase();
    labels
        .iter()
        .any(|label| said.starts_with(&label.to_lowercase()))
}

/// Text as a fact: its fences of code taken off, its blank runs closed.
fn tidy(text: &str) -> String {
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("```"))
        .collect();
    let mut out = String::new();
    let mut blank = 0;
    for line in lines {
        if line.trim().is_empty() {
            blank += 1;
            continue;
        }
        if !out.is_empty() {
            out.push_str(if blank > 0 { "\n\n" } else { "\n" });
        }
        blank = 0;
        out.push_str(line.trim_end());
    }
    out.trim().to_owned()
}

/// Text as statements: a fact a bullet, a fact a paragraph.
fn statements(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let flush = |current: &mut String, out: &mut Vec<String>| {
        let said = current.trim();
        if !said.is_empty() {
            out.push(said.to_owned());
        }
        current.clear();
    };
    for line in tidy(text).lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush(&mut current, &mut out);
            continue;
        }
        let bullet = ["- ", "* ", "• "]
            .iter()
            .find_map(|marker| trimmed.strip_prefix(marker))
            .or_else(|| {
                let rest = trimmed.trim_start_matches(|c: char| c.is_ascii_digit());
                (rest.len() < trimmed.len())
                    .then(|| rest.strip_prefix(". "))
                    .flatten()
            });
        match bullet {
            Some(item) => {
                flush(&mut current, &mut out);
                current.push_str(item);
            }
            None => {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(trimmed);
            }
        }
    }
    flush(&mut current, &mut out);
    out
}

/// What a conversion would do.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Plan {
    pub cards: Vec<PlannedCard>,
    pub kits: Vec<PlannedKit>,
    pub documents: Vec<PlannedDocument>,
    pub notes: Vec<PlannedNote>,
    pub marks: Vec<PlannedMark>,
    /// Notes and parts passed over: held already, or left by the rules.
    pub passed_over: Vec<String>,
    /// Sections that name no work of the space.
    pub unmatched: Vec<String>,
}

/// A card to make from a note.
#[derive(Debug, Clone, Serialize)]
pub struct PlannedCard {
    pub note_id: String,
    pub kind: String,
    /// The note's name, after the prefix: what variants are told by.
    pub key: String,
    pub title: String,
    pub facts: Vec<PlannedFact>,
    /// The key of the card it is a variant of.
    pub variant_of: Option<String>,
    /// The card of the canon the note adds to, when it holds one by that
    /// name already: nothing new is made.
    pub existing: Option<String>,
    /// The base it is a variant of, when the canon holds that card already.
    pub variant_of_id: Option<String>,
}

/// A fact of a card to make.
#[derive(Debug, Clone, Serialize)]
pub struct PlannedFact {
    pub section: String,
    pub slot: Option<String>,
    pub body: String,
}

/// A kit of the dressing to make from a note.
#[derive(Debug, Clone, Serialize)]
pub struct PlannedKit {
    pub note_id: String,
    pub name: String,
    pub kind: String,
    pub prompt: Option<String>,
    pub negative: Option<String>,
    pub about: Option<String>,
    pub archived: bool,
}

/// A note made a note of another kind, where it stands.
#[derive(Debug, Clone, Serialize)]
pub struct PlannedDocument {
    pub note_id: String,
    pub title: String,
    pub kind: String,
    /// The title it takes, when the rule gives one.
    pub retitled: Option<String>,
}

/// A note to make on a work, from a section of another.
#[derive(Debug, Clone, Serialize)]
pub struct PlannedNote {
    pub from_note_id: String,
    pub work_id: String,
    pub work_title: String,
    pub kind: String,
    pub title: String,
    pub body: String,
}

/// A work to mark.
#[derive(Debug, Clone, Serialize)]
pub struct PlannedMark {
    pub from_note_id: String,
    pub work_id: String,
    pub work_title: String,
    pub mark: String,
}

/// The title of a note without the rule's prefix, when it has it.
fn after_prefix<'n>(note: &'n Note, prefix: &str) -> Option<&'n str> {
    note.title
        .as_deref()
        .and_then(|title| title.strip_prefix(prefix))
        .map(str::trim)
}

/// The name a note gives itself: the first heading, when it opens the note
/// at the top level, else `fallback`.
fn own_name(found: &[Block], fallback: &str) -> String {
    match found.first() {
        Some(Block {
            heading: Some(heading),
            level: 1,
            ..
        }) => heading.clone(),
        _ => fallback.to_owned(),
    }
}

/// What converting the notes of this space by `rules` would make.
pub fn plan(conn: &Connection, profile_id: &str, rules: &Rules) -> Result<Plan> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let notes = crate::note::list(conn, profile_id, &NoteFilter::default())?;
    let mut out = Plan::default();
    plan_cards(conn, &config, &notes, rules, &mut out)?;
    plan_kits(conn, &config, &notes, rules, &mut out)?;
    for rule in &rules.documents {
        config
            .note_kind(&rule.kind)
            .ok_or_else(|| Error::not_found("note kind", rule.kind.clone()))?;
        for note in &notes {
            if after_prefix(note, &rule.title_prefix).is_none() {
                continue;
            }
            if note.kind == rule.kind {
                out.passed_over.push(note.title.clone().unwrap_or_default());
                continue;
            }
            out.documents.push(PlannedDocument {
                note_id: note.id.clone(),
                title: note.title.clone().unwrap_or_default(),
                kind: rule.kind.clone(),
                retitled: rule.title.clone(),
            });
        }
    }
    let works = crate::work::list(conn, profile_id, &crate::work::WorkFilter::default())?;
    for rule in &rules.splits {
        config
            .note_kind(&rule.kind)
            .ok_or_else(|| Error::not_found("note kind", rule.kind.clone()))?;
        let Some(source) = notes
            .iter()
            .find(|n| n.title.as_deref() == Some(&rule.title))
        else {
            continue;
        };
        for (work, text) in by_work(source, &works, rule.work_kind.as_deref(), &mut out) {
            let held = notes.iter().any(|n| {
                n.work_id.as_deref() == Some(&work.id)
                    && n.kind == rule.kind
                    && n.title.as_deref() == Some(&rule.note_title)
            });
            if held {
                out.passed_over
                    .push(format!("{} · {}", rule.note_title, work.title));
                continue;
            }
            out.notes.push(PlannedNote {
                from_note_id: source.id.clone(),
                work_id: work.id.clone(),
                work_title: work.title.clone(),
                kind: rule.kind.clone(),
                title: rule.note_title.clone(),
                body: text,
            });
        }
    }
    for rule in &rules.marks {
        if !config.marks.iter().any(|mark| mark.key == rule.mark) {
            return Err(Error::refused("work.unknownMark").param("mark", rule.mark.clone()));
        }
        let Some(source) = notes
            .iter()
            .find(|n| n.title.as_deref() == Some(&rule.title))
        else {
            continue;
        };
        for (work, text) in by_work(source, &works, rule.work_kind.as_deref(), &mut out) {
            if work.marks.contains(&rule.mark) {
                out.passed_over
                    .push(format!("{} · {}", rule.mark, work.title));
            } else {
                out.marks.push(PlannedMark {
                    from_note_id: source.id.clone(),
                    work_id: work.id.clone(),
                    work_title: work.title.clone(),
                    mark: rule.mark.clone(),
                });
            }
            if let (Some(kind), Some(title)) = (&rule.kind, &rule.note_title) {
                let held = notes.iter().any(|n| {
                    n.work_id.as_deref() == Some(&work.id)
                        && &n.kind == kind
                        && n.title.as_deref() == Some(title)
                });
                if !held {
                    out.notes.push(PlannedNote {
                        from_note_id: source.id.clone(),
                        work_id: work.id.clone(),
                        work_title: work.title.clone(),
                        kind: kind.clone(),
                        title: title.clone(),
                        body: text,
                    });
                }
            }
        }
    }
    Ok(out)
}

/// A title without a trailing word in brackets: "Harbour lights (video)"
/// is "Harbour lights".
fn bare_title(title: &str) -> &str {
    let trimmed = title.trim_end();
    if trimmed.ends_with(')')
        && let Some(open) = trimmed.rfind(" (")
    {
        return trimmed[..open].trim_end();
    }
    trimmed
}

/// The top sections of `source` and the works they name: a work answers to
/// a section whose heading, its number taken off, starts with the work's
/// title - the longest such title. A section nobody answers to is listed.
fn by_work<'w>(
    source: &Note,
    works: &'w [crate::work::Work],
    kind: Option<&str>,
    out: &mut Plan,
) -> Vec<(&'w crate::work::Work, String)> {
    let found = blocks(&source.body);
    let top = found
        .iter()
        .filter_map(|block| (block.level > 0).then_some(block.level))
        .filter(|level| *level > 1)
        .min()
        .unwrap_or(2);
    let mut sections: Vec<(String, String)> = Vec::new();
    for block in &found {
        match (&block.heading, block.level) {
            (Some(heading), level) if level == top => {
                sections.push((heading.clone(), block.text.clone()));
            }
            (Some(heading), level) if level > top => {
                if let Some((_, text)) = sections.last_mut() {
                    // A sub-heading stays one, a level up under the note.
                    text.push_str(&format!("{} {heading}\n", "#".repeat(level - 1)));
                    text.push_str(&block.text);
                }
            }
            _ => {}
        }
    }
    let mut paired = Vec::new();
    for (heading, text) in sections {
        let said = unnumbered(&heading).to_lowercase();
        let work = works
            .iter()
            .filter(|work| kind.is_none_or(|kind| work.kind == kind))
            .filter(|work| {
                // A work made from another is often titled as it with a word
                // in brackets - "Harbour lights (video)" - and a section names
                // it without.
                let full = work.title.trim().to_lowercase();
                let bare = bare_title(&full);
                [full.as_str(), bare].into_iter().any(|title| {
                    !title.is_empty()
                        && said.starts_with(title)
                        && said[title.len()..]
                            .chars()
                            .next()
                            .is_none_or(|c| !c.is_alphanumeric())
                })
            })
            .max_by_key(|work| work.title.chars().count());
        match work {
            Some(work) if !tidy(&text).is_empty() => paired.push((work, tidy(&text))),
            Some(_) => {}
            None => out.unmatched.push(heading),
        }
    }
    paired
}

fn plan_cards(
    conn: &Connection,
    config: &ProfileConfig,
    notes: &[Note],
    rules: &Rules,
    out: &mut Plan,
) -> Result<()> {
    for rule in &rules.cards {
        let kind = config.require_card_kind(&rule.kind)?;
        // Where a target sends its text, checked against the kind once.
        let target =
            |to: &str, heading: Option<&str>| -> Result<(String, Option<String>, SectionShape)> {
                let (section, slot) = match to.split_once(':') {
                    Some((section, slot)) => (section.trim(), Some(slot.trim().to_owned())),
                    None => (to.trim(), None),
                };
                let shape = kind
                    .section(section)
                    .map(|found| found.shape)
                    .filter(|shape| shape.holds_facts())
                    .ok_or_else(|| {
                        Error::refused("convert.unknownSection")
                            .param("kind", rule.kind.clone())
                            .param("section", section)
                    })?;
                let slot = match shape {
                    SectionShape::Slots => Some(slot.unwrap_or_else(|| {
                        heading
                            .map(|heading| unnumbered(heading).trim().to_lowercase())
                            .unwrap_or_else(|| "description".to_owned())
                    })),
                    _ => None,
                };
                Ok((section.to_owned(), slot, shape))
            };
        target(&rule.fallback, None)?;
        for to in rule.headings.values() {
            target(to, None)?;
        }
        // The cards of the kind already made, by every name they answer to:
        // a note of a place the canon holds adds to that card - what the
        // card says already is not said twice.
        let mut held: BTreeMap<String, &Note> = BTreeMap::new();
        for card in notes.iter().filter(|note| note.kind == rule.kind) {
            for name in card.title.iter().chain(card.aliases.iter()) {
                held.entry(name.trim().to_lowercase()).or_insert(card);
            }
        }
        let mut made: Vec<PlannedCard> = Vec::new();
        for note in notes {
            let Some(key) = after_prefix(note, &rule.title_prefix) else {
                continue;
            };
            if note.kind == rule.kind || rule.skip.iter().any(|skip| skip == key) {
                continue;
            }
            let found = blocks(&note.body);
            let title = if rule.name_from_heading {
                own_name(&found, key)
            } else {
                key.to_owned()
            };
            let existing = held
                .get(&key.to_lowercase())
                .or_else(|| held.get(&title.to_lowercase()))
                .copied();
            let said_already: BTreeSet<(String, String)> = match existing {
                Some(card) => crate::canon::fact::for_card(conn, &card.id)?
                    .into_iter()
                    .map(|one| (one.section, one.body.trim().to_lowercase()))
                    .collect(),
                None => BTreeSet::new(),
            };
            let mut facts = Vec::new();
            for block in &found {
                // A heading with nothing under it - the one that names the
                // note, most often - gives no fact.
                if block.text.trim().is_empty() {
                    continue;
                }
                let heading = block.heading.as_deref();
                let found_rule = heading.and_then(|heading| matched(&rule.headings, heading));
                let to = found_rule.map_or(rule.fallback.as_str(), |(_, to)| to);
                let (section, slot, shape) = target(to, heading)?;
                // What the heading says beyond the rule's word stays with the
                // text - "Prompt block - REALITY" is the REALITY block - and a
                // heading the rules do not know keeps all its words. The
                // place's own name said again in a heading is not kept.
                let said = match (heading, found_rule) {
                    (Some(heading), Some((key, _))) => rest_of(heading, key),
                    (Some(heading), None) => unnumbered(heading).trim().to_owned(),
                    (None, _) => String::new(),
                };
                let lower = said.to_lowercase();
                let said = if said.is_empty()
                    || lower == key.to_lowercase()
                    || title.to_lowercase().starts_with(&lower)
                {
                    None
                } else {
                    Some(said)
                };
                let bodies = match shape {
                    SectionShape::Facts => statements(&block.text),
                    _ => vec![tidy(&block.text)],
                };
                for body in bodies.into_iter().filter(|body| !body.is_empty()) {
                    let body = match &said {
                        Some(said) => format!("{said}: {body}"),
                        None => body,
                    };
                    if said_already.contains(&(section.clone(), body.trim().to_lowercase())) {
                        continue;
                    }
                    facts.push(PlannedFact {
                        section: section.clone(),
                        slot: slot.clone(),
                        body,
                    });
                }
            }
            made.push(PlannedCard {
                note_id: note.id.clone(),
                kind: rule.kind.clone(),
                key: key.to_owned(),
                title: existing
                    .and_then(|card| card.title.clone())
                    .unwrap_or(title),
                facts,
                variant_of: None,
                existing: existing.map(|card| card.id.clone()),
                variant_of_id: None,
            });
        }
        if rule.variants {
            let bases: Vec<(String, Option<String>)> = made
                .iter()
                .map(|card| (card.key.clone(), card.existing.clone()))
                .collect();
            for card in &mut made {
                let base = bases
                    .iter()
                    .filter(|(other, _)| {
                        card.key.len() > other.len()
                            && card.key.starts_with(other.as_str())
                            && card.key[other.len()..].starts_with('-')
                    })
                    .max_by_key(|(other, _)| other.len());
                let Some((base, base_id)) = base else {
                    continue;
                };
                // Two cards already related are left as they are.
                if let (Some(from), Some(to)) = (&card.existing, base_id)
                    && crate::canon::link::for_card(conn, from)?
                        .iter()
                        .any(|one| one.other(from) == to)
                {
                    continue;
                }
                card.variant_of = Some(base.clone());
                card.variant_of_id = base_id.clone();
            }
        }
        for card in made {
            if card.existing.is_some() && card.facts.is_empty() && card.variant_of.is_none() {
                out.passed_over.push(card.title);
            } else {
                out.cards.push(card);
            }
        }
    }
    Ok(())
}

fn plan_kits(
    conn: &Connection,
    config: &ProfileConfig,
    notes: &[Note],
    rules: &Rules,
    out: &mut Plan,
) -> Result<()> {
    let Some(rule) = &rules.kits else {
        return Ok(());
    };
    let root = config
        .note_kinds
        .iter()
        .find(|kind| {
            kind.root
                && kind
                    .sections
                    .iter()
                    .any(|s| s.shape == SectionShape::Branding)
        })
        .ok_or_else(|| Error::refused("convert.noDressing"))?;
    // The kits the root card holds already, by name: a second run makes none
    // of them again.
    let mut held: BTreeSet<String> = BTreeSet::new();
    for card in notes.iter().filter(|note| note.kind == root.key) {
        for one in crate::canon::fact::for_card(conn, &card.id)? {
            if root
                .section(&one.section)
                .is_some_and(|section| section.shape == SectionShape::Branding)
            {
                held.insert(one.body.trim().to_lowercase());
            }
        }
    }
    for note in notes {
        let Some(key) = after_prefix(note, &rule.title_prefix) else {
            continue;
        };
        if rule.skip.iter().any(|skip| skip == key) {
            out.passed_over.push(note.title.clone().unwrap_or_default());
            continue;
        }
        let found = blocks(&note.body);
        let name = own_name(&found, key);
        if held.contains(&name.trim().to_lowercase()) {
            out.passed_over.push(note.title.clone().unwrap_or_default());
            continue;
        }
        let lower = key.to_lowercase();
        let kind = rule
            .kinds
            .iter()
            .find(|(word, _)| lower.contains(&word.to_lowercase()))
            .map(|(_, kind)| kind.clone())
            .unwrap_or_else(|| "banner".to_owned());
        if !crate::canon::fact::BRANDING_KINDS.contains(&kind.as_str()) {
            return Err(Error::refused("canon.brandingNeedsKind").param("kind", kind));
        }
        let archived = rule
            .archive_marker
            .as_deref()
            .is_some_and(|marker| lower.contains(&marker.to_lowercase()));
        let under = |labels: &[String]| -> Option<String> {
            for block in &found {
                if let Some(heading) = &block.heading
                    && names(labels, heading)
                {
                    let text = tidy(&block.text);
                    if !text.is_empty() {
                        return Some(text);
                    }
                }
            }
            labelled(&note.body, labels)
        };
        out.kits.push(PlannedKit {
            note_id: note.id.clone(),
            name,
            kind,
            prompt: under(&rule.prompt),
            negative: under(&rule.negative),
            about: rule
                .about
                .iter()
                .find_map(|label| under(std::slice::from_ref(label))),
            archived,
        });
    }
    Ok(())
}

/// The text after a bold label - `**Prompt:** ...` - on its line, or the
/// lines after it up to a blank one.
fn labelled(body: &str, labels: &[String]) -> Option<String> {
    let lines: Vec<&str> = body.lines().collect();
    for (at, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let Some(inner) = trimmed.strip_prefix("**") else {
            continue;
        };
        let Some((label, rest)) = inner.split_once("**") else {
            continue;
        };
        let label = label.trim().trim_end_matches(':').trim();
        if !labels.iter().any(|wanted| {
            label.eq_ignore_ascii_case(wanted.trim())
                || label.to_lowercase() == wanted.trim().to_lowercase()
        }) {
            continue;
        }
        let rest = rest.trim().trim_start_matches(':').trim();
        if !rest.is_empty() {
            return Some(rest.to_owned());
        }
        let mut text = String::new();
        for next in &lines[at + 1..] {
            if next.trim().is_empty() && !tidy(&text).is_empty() {
                break;
            }
            text.push_str(next);
            text.push('\n');
        }
        let text = tidy(&text);
        if !text.is_empty() {
            return Some(text);
        }
    }
    None
}

/// What a conversion made.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Applied {
    pub cards: usize,
    pub facts: usize,
    pub variants: usize,
    pub kits: usize,
    pub documents: usize,
    pub notes: usize,
    pub marks: usize,
    pub removed: usize,
}

/// Make what the plan says, as one gesture; with `remove`, the notes made
/// into cards and kits, and the notes split, go to the trash.
pub fn apply(conn: &Connection, profile_id: &str, plan: &Plan, remove: bool) -> Result<Applied> {
    let config = crate::profile::config_for(conn, profile_id)?;
    crate::db::unit::atomically(conn, |tx| {
        let mut done = Applied::default();
        let mut made: BTreeMap<String, String> = BTreeMap::new();
        for card in &plan.cards {
            // A card the canon holds is added to; another is made.
            let id = match &card.existing {
                Some(id) => id.clone(),
                None => {
                    let source = crate::note::get(tx, &card.note_id)?;
                    let aliases = if card.key != card.title {
                        vec![card.key.clone()]
                    } else {
                        Vec::new()
                    };
                    let new = crate::actions::canon::create_card(
                        tx,
                        NewNote {
                            kind: Some(card.kind.clone()),
                            title: Some(card.title.clone()),
                            aliases,
                            layer: source.as_ref().map(|note| note.layer),
                            ..NewNote::default()
                        },
                    )?;
                    done.cards += 1;
                    new.id
                }
            };
            made.insert(card.key.clone(), id.clone());
            for fact in &card.facts {
                let mut data = Map::new();
                if let Some(slot) = &fact.slot {
                    data.insert("slot".into(), Value::String(slot.clone()));
                }
                crate::actions::canon::add_fact(
                    tx,
                    NewFact {
                        note_id: id.clone(),
                        section: fact.section.clone(),
                        body: fact.body.clone(),
                        data,
                        ..NewFact::default()
                    },
                )?;
                done.facts += 1;
            }
        }
        let variant = crate::canon::view::VARIANT_OF;
        if config.relation_kinds.iter().any(|kind| kind.key == variant) {
            for card in &plan.cards {
                let base = card
                    .variant_of
                    .as_ref()
                    .and_then(|key| made.get(key))
                    .or(card.variant_of_id.as_ref());
                let (Some(base), Some(from)) = (base, made.get(&card.key)) else {
                    continue;
                };
                crate::actions::canon::relate(
                    tx,
                    NewCanonLink {
                        from_id: from.clone(),
                        to_id: base.clone(),
                        kind: Some(variant.to_owned()),
                        ..NewCanonLink::default()
                    },
                )?;
                done.variants += 1;
            }
        }
        if !plan.kits.is_empty() {
            let (root, section) = dressing(tx, profile_id, &config)?;
            // The look in use first, by kind and name; the archive after it.
            let mut kits: Vec<&PlannedKit> = plan.kits.iter().collect();
            kits.sort_by(|a, b| {
                (a.archived, &a.kind, &a.name).cmp(&(b.archived, &b.kind, &b.name))
            });
            for kit in kits {
                let mut data = Map::new();
                data.insert("kind".into(), json!(kit.kind));
                for (key, value) in [
                    ("prompt", &kit.prompt),
                    ("negative", &kit.negative),
                    ("about", &kit.about),
                ] {
                    if let Some(value) = value {
                        data.insert(key.into(), json!(value));
                    }
                }
                if kit.archived {
                    data.insert("archived".into(), json!(true));
                }
                crate::actions::canon::add_fact(
                    tx,
                    NewFact {
                        note_id: root.clone(),
                        section: section.clone(),
                        body: kit.name.clone(),
                        data,
                        ..NewFact::default()
                    },
                )?;
                done.kits += 1;
            }
        }
        for document in &plan.documents {
            crate::actions::note::update(
                tx,
                &document.note_id,
                NotePatch {
                    kind: Some(document.kind.clone()),
                    title: document.retitled.clone().map(Some),
                    ..NotePatch::default()
                },
            )?;
            done.documents += 1;
        }
        for note in &plan.notes {
            crate::actions::note::create(
                tx,
                NewNote {
                    kind: Some(note.kind.clone()),
                    title: Some(note.title.clone()),
                    body: note.body.clone(),
                    work_id: Some(note.work_id.clone()),
                    ..NewNote::default()
                },
            )?;
            done.notes += 1;
        }
        let mut by_mark: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for mark in &plan.marks {
            by_mark
                .entry(mark.mark.as_str())
                .or_default()
                .push(mark.work_id.clone());
        }
        for (mark, works) in by_mark {
            done.marks += crate::actions::work::mark(tx, &works, mark, true)?.changed;
        }
        if remove {
            let mut gone: BTreeSet<&str> = BTreeSet::new();
            gone.extend(plan.cards.iter().map(|card| card.note_id.as_str()));
            gone.extend(plan.kits.iter().map(|kit| kit.note_id.as_str()));
            gone.extend(plan.notes.iter().map(|note| note.from_note_id.as_str()));
            for id in gone {
                crate::actions::trash::discard(tx, crate::trash::Entity::Note, id)?;
                done.removed += 1;
            }
        }
        Ok(done)
    })
}

/// The root card that holds the dressing, and its section of kits.
fn dressing(
    conn: &Connection,
    profile_id: &str,
    config: &ProfileConfig,
) -> Result<(String, String)> {
    let kind = config
        .note_kinds
        .iter()
        .find(|kind| {
            kind.root
                && kind
                    .sections
                    .iter()
                    .any(|s| s.shape == SectionShape::Branding)
        })
        .ok_or_else(|| Error::refused("convert.noDressing"))?;
    let section = kind
        .sections
        .iter()
        .find(|s| s.shape == SectionShape::Branding)
        .map(|s| s.key.clone())
        .ok_or_else(|| Error::refused("convert.noDressing"))?;
    let filter = NoteFilter {
        kind: Some(kind.key.clone()),
        ..NoteFilter::default()
    };
    let root = crate::note::list(conn, profile_id, &filter)?
        .into_iter()
        .next()
        .ok_or_else(|| Error::refused("convert.noRootCard"))?;
    Ok((root.id, section))
}

/// Read the rules, plan against a space and write the plan and its report;
/// with `apply_it`, keep a copy of the space and apply. The folder the
/// report is written to comes back.
pub fn run(
    app_dir: &Path,
    space: Option<&str>,
    rules: &Path,
    apply_it: bool,
    remove: bool,
) -> Result<PathBuf> {
    use crate::space::registry;
    let rules: Rules = serde_json::from_str(&std::fs::read_to_string(rules)?)
        .map_err(|err| Error::refused("convert.badRules").param("error", err.to_string()))?;
    let app = crate::db::open_app(&app_dir.join(crate::db::APP_DATABASE_FILE))?;
    let found = super::target_space(&app, app_dir, space, apply_it)?;
    let folder = registry::folder(&app, app_dir, &found.id)?;
    let (conn, held_by) = crate::state::open_space(&folder, crate::state::Role::Companion)?;
    if apply_it && held_by.is_some() {
        return Err(Error::refused("space.unreachable").param("path", found.path.clone()));
    }
    let profile_id = crate::actions::active_profile_id(&conn)?;
    let moment = crate::time::now().replace([':', '.'], "-");
    let dir = app_dir.join("convert-notes").join(&moment);
    let planned = plan(&conn, &profile_id, &rules)?;
    let applied = if apply_it {
        crate::exchange::backup::write_database(&conn, &dir.join("kilna.before-convert.db"))?;
        Some(apply(&conn, &profile_id, &planned, remove)?)
    } else {
        None
    };
    report(&dir, &planned, applied.as_ref())?;
    Ok(dir)
}

/// The plan as `convert_plan.json` and a page, `convert_plan.md`.
fn report(dir: &Path, plan: &Plan, applied: Option<&Applied>) -> Result<()> {
    use std::fmt::Write as _;
    std::fs::create_dir_all(dir)?;
    let whole = json!({ "plan": plan, "applied": applied });
    std::fs::write(
        dir.join("convert_plan.json"),
        serde_json::to_string_pretty(&whole)?,
    )?;
    let mut page = String::new();
    let _ = writeln!(page, "# Converting notes\n");
    match applied {
        None => {
            let _ = writeln!(
                page,
                "A plan: nothing is written until it is run again with `--apply`.\n"
            );
        }
        Some(done) => {
            let _ = writeln!(
                page,
                "Applied: {} cards with {} facts and {} variants, {} kits, {} documents, {} notes, {} marks; {} notes to the trash. A copy of the space from before is `kilna.before-convert.db` beside this page.\n",
                done.cards,
                done.facts,
                done.variants,
                done.kits,
                done.documents,
                done.notes,
                done.marks,
                done.removed
            );
        }
    }
    let _ = writeln!(page, "- Cards: {}", plan.cards.len());
    let _ = writeln!(page, "- Kits of the dressing: {}", plan.kits.len());
    let _ = writeln!(page, "- Notes made documents: {}", plan.documents.len());
    let _ = writeln!(page, "- Notes on works: {}", plan.notes.len());
    let _ = writeln!(page, "- Works marked: {}", plan.marks.len());
    let _ = writeln!(page, "- Passed over: {}", plan.passed_over.len());
    let _ = writeln!(
        page,
        "- Sections naming no work: {}\n",
        plan.unmatched.len()
    );
    if !plan.cards.is_empty() {
        let _ = writeln!(page, "## Cards\n");
        for card in &plan.cards {
            let variant = card
                .variant_of
                .as_deref()
                .map(|base| format!(", a variant of {base}"))
                .unwrap_or_default();
            let _ = writeln!(
                page,
                "- {} ({}): {} facts{variant}",
                card.title,
                card.kind,
                card.facts.len()
            );
        }
        let _ = writeln!(page);
    }
    if !plan.kits.is_empty() {
        let _ = writeln!(page, "## Kits\n");
        for kit in &plan.kits {
            let archived = if kit.archived { ", archived" } else { "" };
            let prompt = if kit.prompt.is_some() {
                ""
            } else {
                ", no prompt found"
            };
            let _ = writeln!(page, "- {} ({}{archived}{prompt})", kit.name, kit.kind);
        }
        let _ = writeln!(page);
    }
    if !plan.unmatched.is_empty() {
        let _ = writeln!(page, "## Sections naming no work\n");
        for one in &plan.unmatched {
            let _ = writeln!(page, "- {one}");
        }
    }
    std::fs::write(dir.join("convert_plan.md"), page)?;
    Ok(())
}

#[cfg(test)]
mod tests;
