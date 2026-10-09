//! A text read against the phrases of the dictionary (v0.94).
//!
//! A style prompt is a line of phrases in the generator's own English -
//! `female vocal, chopped amen break, everything clipping` - and the person
//! writing it has to know what each one does before they reach for it. The
//! dictionary knows: a brick of a type of phrase is one phrase, word for word,
//! with what it means. So a text of a role a composition writes is read
//! against the bricks of that composition's types, by the same `check_text`
//! that reads a lyric against the register (ADR 0044):
//!
//! - a phrase the dictionary has is found, wherever it stands in the text;
//! - a tag the dictionary does not have - a short piece between commas - is
//!   said to be unknown, so it can be explained and kept;
//! - the work's own fields as the composition writes them (`92 bpm`) are
//!   neither: they are the work speaking, not a phrase to look up.
//!
//! A long piece between commas is prose, not a tag: the phrases inside it are
//! found, and the rest of it is left alone - "unknown" said of a sentence
//! would be noise.

pub mod compose;
pub mod proposal;

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Connection;
use serde::Serialize;

use crate::error::Result;
use crate::profile::config::{Composition, Label, MetaFieldType, ProfileConfig};
use crate::style_brick::{self, StyleBrick};

/// A tag of more words than this is a sentence, not a phrase to look up.
const TAG_WORDS: usize = 6;

/// A phrase of the dictionary a text says.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct PhraseHit {
    pub brick_id: String,
    pub type_key: String,
    /// The phrase as the dictionary writes it.
    pub phrase: String,
    /// What it means, per language as the brick carries it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub explanation: Option<Label>,
    /// When to reach for it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub when: Option<String>,
    /// One of the channel's house bricks.
    pub house: bool,
    /// How many times this text says it.
    pub count: usize,
}

/// A tag a text says that the dictionary does not know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct UnknownPhrase {
    /// As the text writes it.
    pub phrase: String,
    pub count: usize,
}

/// What reading a text against the dictionary found: the phrases it says,
/// the tags it says that the dictionary lacks, and where each stands, as
/// `(start, end, index)` in UTF-16 units - the window's offsets.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reading {
    pub phrases: Vec<PhraseHit>,
    pub unknown: Vec<UnknownPhrase>,
    pub phrase_places: Vec<(usize, usize, usize)>,
    pub unknown_places: Vec<(usize, usize, usize)>,
}

/// The bricks a composition reads a text against, found by their phrase.
pub struct Dictionary {
    bricks: Vec<StyleBrick>,
    house: BTreeSet<String>,
    /// Normalised phrase → index into `bricks`.
    by_key: BTreeMap<String, usize>,
    /// The phrases as word sequences, longest first: the order a phrase is
    /// looked for inside a sentence, so `intimate close-mic female vocal` is
    /// found before the `female vocal` inside it.
    by_length: Vec<(Vec<String>, usize)>,
}

impl Dictionary {
    /// The bricks of the composition's types that say something: a draft
    /// with no phrase yet cannot be found in a text, and a dropped brick is
    /// still a phrase the text says - it is found, so it is not called
    /// unknown.
    pub fn of(conn: &Connection, profile_id: &str, composition: &Composition) -> Result<Self> {
        let types: BTreeSet<&str> = composition.types().collect();
        let bricks: Vec<StyleBrick> =
            style_brick::list(conn, profile_id, &style_brick::StyleBrickFilter::default())?
                .into_iter()
                .filter(|brick| types.contains(brick.type_key.as_str()))
                .filter(|brick| {
                    brick
                        .description
                        .as_deref()
                        .is_some_and(|text| !key_of(text).is_empty())
                })
                .collect();
        Ok(Self::from_bricks(bricks, house(conn, profile_id)?))
    }

    pub fn from_bricks(bricks: Vec<StyleBrick>, house: BTreeSet<String>) -> Self {
        let mut by_key = BTreeMap::new();
        // A ready brick wins a phrase two bricks share: it is the one meant.
        let mut order: Vec<usize> = (0..bricks.len()).collect();
        order.sort_by_key(|&index| bricks[index].status != style_brick::READY);
        for index in order {
            let key = key_of(bricks[index].description.as_deref().unwrap_or_default());
            by_key.entry(key).or_insert(index);
        }
        let mut by_length: Vec<(Vec<String>, usize)> = by_key
            .iter()
            .map(|(key, &index)| (key.split(' ').map(str::to_owned).collect(), index))
            .collect();
        by_length.sort_by_key(|(words, _)| std::cmp::Reverse(words.len()));
        Self {
            bricks,
            house,
            by_key,
            by_length,
        }
    }

    /// The brick a phrase is, if the dictionary has it.
    pub fn find(&self, phrase: &str) -> Option<&StyleBrick> {
        self.by_key.get(&key_of(phrase)).map(|&i| &self.bricks[i])
    }

    pub fn bricks(&self) -> &[StyleBrick] {
        &self.bricks
    }
}

/// The values a work's fields are written as in its text: what is the work
/// speaking rather than a phrase. A number field is recognised by its shape -
/// an older version may say an older tempo - and any other by the value the
/// work holds now.
#[derive(Debug, Clone, Default)]
pub struct Fields {
    /// `(prefix, suffix)` of a number field's template.
    numbers: Vec<(String, String)>,
    /// A text field's value as the template writes it, normalised.
    values: BTreeSet<String>,
}

impl Fields {
    pub fn of(config: &ProfileConfig, composition: &Composition, meta: &serde_json::Value) -> Self {
        let mut fields = Fields::default();
        for part in &composition.fields {
            let number = config
                .work_meta_fields
                .iter()
                .find(|field| field.key == part.field)
                .is_some_and(|field| field.field_type == MetaFieldType::Number);
            if number {
                let (prefix, suffix) = part.template.split_once("{value}").unwrap_or(("", ""));
                fields.numbers.push((key_of(prefix), key_of(suffix)));
            } else if let Some(value) = compose::field_value(meta, &part.field) {
                fields.values.insert(key_of(&part.write(&value)));
            }
        }
        fields
    }

    fn says(&self, key: &str) -> bool {
        if self.values.contains(key) {
            return true;
        }
        self.numbers.iter().any(|(prefix, suffix)| {
            key.strip_prefix(prefix.as_str())
                .and_then(|rest| rest.strip_suffix(suffix.as_str()))
                .map(str::trim)
                .is_some_and(|middle| {
                    !middle.is_empty() && middle.replace(',', ".").parse::<f64>().is_ok()
                })
        })
    }
}

/// Read `text` against the dictionary.
pub fn read(text: &str, dictionary: &Dictionary, fields: &Fields) -> Reading {
    let mut reading = Reading::default();
    let mut hit_slot: BTreeMap<usize, usize> = BTreeMap::new();
    let mut unknown_slot: BTreeMap<String, usize> = BTreeMap::new();

    for piece in pieces(text) {
        let key = key_of(piece.text);
        if key.is_empty() || fields.says(&key) {
            continue;
        }
        // The whole tag, when the dictionary has it.
        if let Some(&index) = dictionary.by_key.get(&key) {
            let slot = hit(&mut reading, &mut hit_slot, dictionary, index);
            reading
                .phrase_places
                .push((piece.start16, piece.end16, slot));
            continue;
        }
        let words = word_spans(piece.text);
        if words.len() <= TAG_WORDS && looks_like_a_tag(piece.text) {
            let slot = match unknown_slot.get(&key) {
                Some(&slot) => {
                    reading.unknown[slot].count += 1;
                    slot
                }
                None => {
                    unknown_slot.insert(key, reading.unknown.len());
                    reading.unknown.push(UnknownPhrase {
                        phrase: piece.text.trim().to_owned(),
                        count: 1,
                    });
                    reading.unknown.len() - 1
                }
            };
            reading
                .unknown_places
                .push((piece.start16, piece.end16, slot));
            continue;
        }
        // A sentence: the phrases inside it, longest first, never two on
        // one word.
        let normal: Vec<String> = words.iter().map(|w| key_of(w.text)).collect();
        let mut taken = vec![false; words.len()];
        let mut found: Vec<(usize, usize, usize)> = Vec::new();
        for (phrase, index) in &dictionary.by_length {
            let n = phrase.len();
            if n == 0 || n > words.len() {
                continue;
            }
            let mut at = 0;
            while at + n <= words.len() {
                let fits = (at..at + n).all(|i| !taken[i] && normal[i] == phrase[i - at]);
                if fits {
                    for flag in &mut taken[at..at + n] {
                        *flag = true;
                    }
                    found.push((at, at + n, *index));
                    at += n;
                } else {
                    at += 1;
                }
            }
        }
        found.sort_unstable();
        for (first, last, index) in found {
            let slot = hit(&mut reading, &mut hit_slot, dictionary, index);
            let start = piece.start16 + words[first].start16;
            let end = piece.start16 + words[last - 1].end16;
            reading.phrase_places.push((start, end, slot));
        }
    }
    reading
}

fn hit(
    reading: &mut Reading,
    slots: &mut BTreeMap<usize, usize>,
    dictionary: &Dictionary,
    index: usize,
) -> usize {
    if let Some(&slot) = slots.get(&index) {
        reading.phrases[slot].count += 1;
        return slot;
    }
    let brick = &dictionary.bricks[index];
    slots.insert(index, reading.phrases.len());
    reading.phrases.push(PhraseHit {
        brick_id: brick.id.clone(),
        type_key: brick.type_key.clone(),
        phrase: brick.description.clone().unwrap_or_default(),
        explanation: brick.explanation.clone(),
        when: brick.when_to_use.clone(),
        house: dictionary.house.contains(&brick.id),
        count: 1,
    });
    reading.phrases.len() - 1
}

/// A phrase as it is compared: lower case, a hyphen read as a space, the
/// spaces collapsed, the punctuation at its ends dropped. `Lo-fi  grunge.`
/// and `lo fi grunge` are one phrase.
pub fn key_of(text: &str) -> String {
    let lowered = text
        .to_lowercase()
        .replace(['-', '\u{2010}', '\u{2011}'], " ");
    let words: Vec<&str> = lowered
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '#'))
        .filter(|word| !word.is_empty())
        .collect();
    words.join(" ")
}

/// Whether a short piece is a tag worth calling unknown: words in Latin
/// letters, not a heading, a section tag or a fence.
fn looks_like_a_tag(text: &str) -> bool {
    let text = text.trim();
    if text.starts_with(['#', '[', '`', '>', '*', '(']) {
        return false;
    }
    if text.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)) {
        return false;
    }
    text.chars().any(|c| c.is_ascii_alphabetic())
}

/// A piece of the text between separators, with where it stands.
struct Piece<'a> {
    text: &'a str,
    start16: usize,
    end16: usize,
}

/// The text cut at commas, semicolons, line breaks and sentence ends, each
/// piece trimmed - the tags of a tag line, the clauses of prose.
fn pieces(text: &str) -> Vec<Piece<'_>> {
    let mut out = Vec::new();
    let mut start_byte = 0;
    let mut start16 = 0;
    let mut unit = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for (i, &(byte, c)) in chars.iter().enumerate() {
        let ends_sentence = c == '.'
            && chars
                .get(i + 1)
                .is_none_or(|&(_, next)| next.is_whitespace());
        if matches!(c, ',' | ';' | '\n' | '\r' | '|') || ends_sentence {
            push_piece(&mut out, text, start_byte, byte, start16);
            start_byte = byte + c.len_utf8();
            start16 = unit + c.len_utf16();
        }
        unit += c.len_utf16();
    }
    push_piece(&mut out, text, start_byte, text.len(), start16);
    out
}

fn push_piece<'a>(out: &mut Vec<Piece<'a>>, text: &'a str, from: usize, to: usize, start16: usize) {
    let raw = &text[from..to];
    let lead = raw.len() - raw.trim_start().len();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return;
    }
    let lead16: usize = raw[..lead].encode_utf16().count();
    let start = start16 + lead16;
    out.push(Piece {
        text: trimmed,
        start16: start,
        end16: start + trimmed.encode_utf16().count(),
    });
}

struct Word<'a> {
    text: &'a str,
    start16: usize,
    end16: usize,
}

/// The words of a piece, split at spaces and hyphens - as `key_of` reads
/// them - with their places inside it.
fn word_spans(text: &str) -> Vec<Word<'_>> {
    let mut out = Vec::new();
    let mut current: Option<(usize, usize)> = None;
    let mut unit = 0;
    for (byte, c) in text.char_indices() {
        let breaks = c.is_whitespace() || matches!(c, '-' | '\u{2010}' | '\u{2011}');
        if breaks {
            if let Some((from, start16)) = current.take() {
                push_word(&mut out, text, from, byte, start16);
            }
        } else if current.is_none() {
            current = Some((byte, unit));
        }
        unit += c.len_utf16();
    }
    if let Some((from, start16)) = current {
        push_word(&mut out, text, from, text.len(), start16);
    }
    out
}

fn push_word<'a>(out: &mut Vec<Word<'a>>, text: &'a str, from: usize, to: usize, start16: usize) {
    let raw = &text[from..to];
    // The punctuation hugging a word is not part of it: `(lo-fi` is `lo`.
    let lead = raw.len()
        - raw
            .trim_start_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '#')
            .len();
    let word = raw[lead..].trim_end_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '#');
    if word.is_empty() {
        return;
    }
    let start = start16 + raw[..lead].encode_utf16().count();
    out.push(Word {
        text: word,
        start16: start,
        end16: start + word.encode_utf16().count(),
    });
}

/// The channel's house bricks, by id: every brick a styles-shaped section of
/// a root card names, whatever the section reads it as - the house styles of
/// a picture and the house sound alike.
pub fn house(conn: &Connection, profile_id: &str) -> Result<BTreeSet<String>> {
    use crate::profile::config::SectionShape;
    let config = crate::profile::config_for(conn, profile_id)?;
    let mut out = BTreeSet::new();
    let roots =
        crate::canon::view::cards(conn, profile_id, &crate::canon::view::CardFilter::default())?
            .into_iter()
            .filter(|card| config.card_kind(&card.kind).is_some_and(|kind| kind.root));
    for root in roots {
        let Some(kind) = config.card_kind(&root.kind) else {
            continue;
        };
        let view = crate::canon::view::card(conn, &root.id)?;
        for read in &view.facts {
            let styled = kind
                .section(&read.fact.section)
                .is_some_and(|section| section.shape == SectionShape::Styles);
            if !styled || read.fact.status == crate::canon::FactStatus::Retired {
                continue;
            }
            if let Some(id) = read.fact.data.get("styleId").and_then(|v| v.as_str()) {
                out.insert(id.to_owned());
            }
        }
    }
    Ok(out)
}

/// A tag the owner's texts say that the dictionary does not know, with how
/// often and where: what "Your own dictionary" lists to be explained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct FoundPhrase {
    /// As the texts write it most often.
    pub phrase: String,
    /// How many texts say it: versions, and the trials of experiments read
    /// by the same composition (v0.95).
    pub versions: usize,
    /// How many works say it.
    pub works: usize,
    /// A few of the works, by title - shown, never kept.
    pub seen_in: Vec<String>,
}

/// How many titles a found phrase names.
const SEEN_IN: usize = 3;

/// Every tag the texts of a composition's role say that the dictionary does
/// not know, most widely written first: every version, not the current one -
/// a phrase tried once in a draft and dropped is still the owner's word - and
/// every trial of an experiment whose trials the composition reads (v0.95):
/// a sweep of the field is where most new phrases are first written.
pub fn unknown_in_texts(
    conn: &Connection,
    profile_id: &str,
    composition_key: &str,
) -> Result<Vec<FoundPhrase>> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let composition = config
        .composition(composition_key)
        .ok_or_else(|| crate::error::Error::not_found("composition", composition_key))?
        .clone();
    let dictionary = Dictionary::of(conn, profile_id, &composition)?;

    let marks = composition
        .kinds
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT v.body, w.id, w.title, w.meta FROM work_version v
         JOIN work w ON w.id = v.work_id
         WHERE w.profile_id = ?1 AND v.role = ?2 AND w.kind IN ({marks})
         ORDER BY w.title COLLATE NOCASE, v.created_at, v.rowid",
    );
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(profile_id.to_owned()),
        Box::new(composition.role.clone()),
    ];
    for kind in &composition.kinds {
        values.push(Box::new(kind.clone()));
    }
    let mut statement = conn.prepare(&sql)?;
    let rows = statement
        .query_map(
            rusqlite::params_from_iter(values.iter().map(AsRef::as_ref)),
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut rows = rows;
    let labs: Vec<String> = config
        .work_kinds
        .iter()
        .filter(|kind| {
            config
                .trial_composition(&kind.key)
                .is_some_and(|read| read.key == composition.key)
        })
        .map(|kind| kind.key.clone())
        .collect();
    for lab in &labs {
        let mut trials = conn.prepare(
            "SELECT t.body, w.id, w.title, w.meta FROM trial t
             JOIN work w ON w.id = t.work_id
             WHERE t.profile_id = ?1 AND w.kind = ?2
             ORDER BY w.title COLLATE NOCASE, t.created_at, t.rowid",
        )?;
        let found = trials
            .query_map(rusqlite::params![profile_id, lab], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.extend(found);
    }

    struct Tally {
        spellings: BTreeMap<String, usize>,
        versions: usize,
        works: Vec<String>,
        titles: Vec<String>,
    }
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for (body, work_id, title, meta) in rows {
        let meta: serde_json::Value = meta
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        let fields = Fields::of(&config, &composition, &meta);
        for unknown in read(&body, &dictionary, &fields).unknown {
            let tally = tallies
                .entry(key_of(&unknown.phrase))
                .or_insert_with(|| Tally {
                    spellings: BTreeMap::new(),
                    versions: 0,
                    works: Vec::new(),
                    titles: Vec::new(),
                });
            *tally.spellings.entry(unknown.phrase.clone()).or_default() += 1;
            tally.versions += 1;
            if !tally.works.contains(&work_id) {
                tally.works.push(work_id.clone());
                if tally.titles.len() < SEEN_IN {
                    tally.titles.push(title.clone());
                }
            }
        }
    }
    let mut out: Vec<FoundPhrase> = tallies
        .into_values()
        .map(|tally| FoundPhrase {
            phrase: tally
                .spellings
                .iter()
                .max_by_key(|(_, count)| **count)
                .map(|(spelling, _)| spelling.clone())
                .unwrap_or_default(),
            versions: tally.versions,
            works: tally.works.len(),
            seen_in: tally.titles,
        })
        .collect();
    out.sort_by(|a, b| {
        b.works
            .cmp(&a.works)
            .then(b.versions.cmp(&a.versions))
            .then_with(|| a.phrase.to_lowercase().cmp(&b.phrase.to_lowercase()))
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style_brick::Origin;

    fn brick(id: &str, phrase: &str) -> StyleBrick {
        StyleBrick {
            id: id.into(),
            profile_id: "p".into(),
            type_key: "groove".into(),
            name: phrase.into(),
            description: Some(phrase.into()),
            hint: None,
            status: style_brick::READY.into(),
            created_at: String::new(),
            updated_at: String::new(),
            reference_count: 0,
            label: None,
            family: None,
            when_to_use: None,
            explanation: None,
            colours: Vec::new(),
            sample: None,
            set_key: None,
            origin: Origin::Own,
            set_digest: None,
            trial_id: None,
        }
    }

    /// A song in a workspace seeded with the shipped set, the sound in it.
    fn seeded_song() -> (rusqlite::Connection, String, crate::work::Work) {
        let (conn, profile_id) = crate::fixtures::workspace();
        crate::style_set::seed(&conn).unwrap();
        let song = crate::fixtures::song(&conn, &profile_id, "Paper lanterns");
        (conn, profile_id, song)
    }

    #[test]
    fn a_style_version_is_read_against_the_shipped_sound() {
        let (conn, profile_id, song) = seeded_song();
        let check = crate::register::check::version_text(
            &conn,
            &profile_id,
            "female vocal, chopped amen break, female vocal, the noise of a city, 92 bpm",
            false,
            &song,
            "style",
        )
        .unwrap();
        let phrases: Vec<(&str, &str, usize)> = check
            .phrases
            .iter()
            .map(|hit| (hit.phrase.as_str(), hit.type_key.as_str(), hit.count))
            .collect();
        assert_eq!(
            phrases,
            [
                ("female vocal", "vocal", 2),
                ("chopped amen break", "groove", 1)
            ],
            "the set's phrases are found under their types"
        );
        assert!(
            check.phrases[0]
                .explanation
                .as_ref()
                .is_some_and(|label| !label.in_locale("ru").is_empty()),
            "a found phrase carries what it means"
        );
        assert_eq!(check.unknown.len(), 1);
        assert_eq!(check.unknown[0].phrase, "the noise of a city");
        assert!(
            check.repeats.is_empty() && check.terms.is_empty(),
            "a line of phrases is not a lyric"
        );
        assert_eq!(
            check.marks.len(),
            4,
            "every phrase and the unknown tag are marked"
        );
    }

    #[test]
    fn a_lyric_is_not_read_against_the_dictionary() {
        let (conn, profile_id, song) = seeded_song();
        let check = crate::register::check::version_text(
            &conn,
            &profile_id,
            "female vocal, chopped amen break",
            true,
            &song,
            "lyrics",
        )
        .unwrap();
        assert!(check.phrases.is_empty() && check.unknown.is_empty());
    }

    #[test]
    fn a_brick_the_channels_card_names_is_a_house_brick() {
        let (conn, profile_id, _) = seeded_song();
        let config = crate::profile::config_for(&conn, &profile_id).unwrap();
        let composition = config.composition("sound").unwrap().clone();
        let dictionary = Dictionary::of(&conn, &profile_id, &composition).unwrap();
        let vocal = dictionary.find("female vocal").unwrap().id.clone();

        let channel = crate::fixtures::card(&conn, &profile_id, "channel", "The channel");
        let mut data = serde_json::Map::new();
        data.insert("styleId".into(), vocal.clone().into());
        crate::canon::fact::create_minted(
            &conn,
            crate::canon::NewFact {
                note_id: channel.id,
                section: "sound".into(),
                body: "female vocal".into(),
                data,
                ..Default::default()
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap();

        assert!(house(&conn, &profile_id).unwrap().contains(&vocal));
        let dictionary = Dictionary::of(&conn, &profile_id, &composition).unwrap();
        let reading = read("female vocal", &dictionary, &Fields::default());
        assert!(
            reading.phrases[0].house,
            "the hit says it is the channel's own"
        );
    }

    #[test]
    fn the_owners_own_words_are_found_across_every_version() {
        let (conn, profile_id, song) = seeded_song();
        let other = crate::fixtures::song(&conn, &profile_id, "Harbour lights");
        for (work, body) in [
            (&song.id, "female vocal, glass harmonica drones"),
            (&song.id, "Glass harmonica drones, breakbeat"),
            (&other.id, "glass harmonica drones, tape-warped choir"),
        ] {
            crate::work::version::create(
                &conn,
                work,
                crate::work::version::NewVersion {
                    role: "style".into(),
                    body: body.into(),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        let found = unknown_in_texts(&conn, &profile_id, "sound").unwrap();
        let first = &found[0];
        assert_eq!(
            first.phrase, "glass harmonica drones",
            "the commonest spelling names it"
        );
        assert_eq!((first.works, first.versions), (2, 3));
        assert_eq!(first.seen_in.len(), 2);
        assert!(
            found.iter().all(|one| key_of(&one.phrase) != "breakbeat"),
            "a phrase the dictionary knows is not the owner's to explain"
        );
        assert!(found.iter().any(|one| one.phrase == "tape-warped choir"));

        // A trial of an experiment is the owner's word too (v0.95).
        let lab = crate::fixtures::work(&conn, &profile_id, "experiment", "Breaks");
        crate::lab::trial::create_minted(
            &conn,
            &profile_id,
            crate::lab::NewTrial {
                work_id: lab.id.clone(),
                body: Some("glass harmonica drones, rusted spring reverb".into()),
                ..crate::lab::NewTrial::default()
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap();
        let found = unknown_in_texts(&conn, &profile_id, "sound").unwrap();
        assert_eq!((found[0].works, found[0].versions), (3, 4));
        assert!(found.iter().any(|one| one.phrase == "rusted spring reverb"));
    }

    fn dictionary(phrases: &[&str]) -> Dictionary {
        let bricks = phrases
            .iter()
            .enumerate()
            .map(|(i, phrase)| brick(&format!("b{i}"), phrase))
            .collect();
        Dictionary::from_bricks(bricks, BTreeSet::new())
    }

    fn slice16(text: &str, start: usize, end: usize) -> String {
        let units: Vec<u16> = text.encode_utf16().collect();
        String::from_utf16(&units[start..end]).unwrap()
    }

    #[test]
    fn a_tag_line_is_read_tag_by_tag() {
        let dictionary = dictionary(&["female vocal", "chopped amen break", "lo-fi grunge"]);
        let text = "Female vocal, lo fi grunge,  noise guitar bursts, chopped amen break";
        let reading = read(text, &dictionary, &Fields::default());

        let phrases: Vec<&str> = reading.phrases.iter().map(|p| p.phrase.as_str()).collect();
        assert_eq!(
            phrases,
            ["female vocal", "lo-fi grunge", "chopped amen break"],
            "case, hyphens and spaces do not make another phrase"
        );
        assert_eq!(
            reading.unknown,
            vec![UnknownPhrase {
                phrase: "noise guitar bursts".into(),
                count: 1
            }]
        );
        let marked: Vec<String> = reading
            .phrase_places
            .iter()
            .map(|&(s, e, _)| slice16(text, s, e))
            .collect();
        assert_eq!(
            marked,
            ["Female vocal", "lo fi grunge", "chopped amen break"],
            "a mark covers the tag as written, without its spaces"
        );
        let (s, e, _) = reading.unknown_places[0];
        assert_eq!(slice16(text, s, e), "noise guitar bursts");
    }

    #[test]
    fn a_sentence_is_searched_for_phrases_and_never_called_unknown() {
        let dictionary = dictionary(&["female vocal", "intimate close-mic female vocal"]);
        let text = "An intimate close-mic female vocal over a slow piano that keeps returning to the same three notes.";
        let reading = read(text, &dictionary, &Fields::default());
        assert!(reading.unknown.is_empty(), "prose is not a tag");
        assert_eq!(
            reading.phrases.len(),
            1,
            "the longest phrase wins its words"
        );
        assert_eq!(reading.phrases[0].phrase, "intimate close-mic female vocal");
        let (s, e, _) = reading.phrase_places[0];
        assert_eq!(slice16(text, s, e), "intimate close-mic female vocal");
    }

    #[test]
    fn a_phrase_said_twice_is_one_hit_counted_twice() {
        let dictionary = dictionary(&["breakbeat"]);
        let reading = read(
            "breakbeat, warm pads, breakbeat, warm pads",
            &dictionary,
            &Fields::default(),
        );
        assert_eq!(reading.phrases[0].count, 2);
        assert_eq!(reading.phrase_places.len(), 2);
        assert_eq!(reading.unknown[0].count, 2);
        assert_eq!(reading.unknown_places.len(), 2);
    }

    #[test]
    fn headings_section_tags_and_cyrillic_are_not_unknown_tags() {
        let dictionary = dictionary(&[]);
        let reading = read(
            "### Alternative 2: post-punk\n[Verse 1]\nрусский текст\n```",
            &dictionary,
            &Fields::default(),
        );
        assert!(reading.unknown.is_empty(), "{:?}", reading.unknown);
    }

    #[test]
    fn the_works_own_fields_are_neither_known_nor_unknown() {
        let (conn, profile_id) = crate::fixtures::workspace();
        let config = crate::profile::config_for(&conn, &profile_id).unwrap();
        let composition = config.composition("sound").unwrap().clone();
        let meta = serde_json::json!({ "bpm": 92, "key": "E minor" });
        let fields = Fields::of(&config, &composition, &meta);
        let reading = read(
            "100 bpm, E minor, 72 BPM, D major",
            &dictionary(&[]),
            &fields,
        );
        assert_eq!(
            reading
                .unknown
                .iter()
                .map(|u| u.phrase.as_str())
                .collect::<Vec<_>>(),
            ["D major"],
            "any tempo is the tempo field speaking; a key only the work's own"
        );
    }

    #[test]
    fn places_are_utf16_units() {
        let dictionary = dictionary(&["dream pop"]);
        let text = "🎸 noise, dream pop";
        let reading = read(text, &dictionary, &Fields::default());
        let (s, e, _) = reading.phrase_places[0];
        assert_eq!(slice16(text, s, e), "dream pop");
    }
}
