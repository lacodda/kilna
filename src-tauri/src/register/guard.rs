//! The guard of repeats: a song held against the songs that went out before
//! it goes out itself (ADR 0054).
//!
//! The register says what is spent; the guard says when a song is about to
//! spend it again. A song that has not gone out - nothing made from it has -
//! is read against every song that has, and every song that holds a day in
//! the calendar, and what they share is a finding:
//!
//! - **orange** - a term of the register both say ("кофе", "шторы",
//!   "холодильник": spent, and one more use says nothing new - whatever its
//!   strictness, and whenever the other song went out), or a rare word the
//!   other song said long ago;
//! - **red** - a rare word the other song said within the window (90 days by
//!   default): a listener who heard "пульсар" last month hears it again.
//!
//! A word is rare by two measures and no model: a Russian word past a rank in
//! the language's frequency list (the language pack) and in no more than a
//! couple of the owner's own works - a word the owner says in twenty songs is
//! theirs, and spent only if the register says so; or a word the owner keeps
//! in the bank. A word in another script has no frequency to read and is
//! rare only when the bank says so. Measured on the owner's workspace on
//! 2026-10-01: the register's "rare" terms, read as rare words, lit 69 songs
//! red for words already in twenty songs each; English lines lit a dozen
//! more.
//!
//! Nothing here is stored (ADR 0010): the findings are read off the texts and
//! the calendar every time they are asked for, so a rewrite that drops the
//! word drops the finding, and a release moved out of the window turns red to
//! orange. What a person says about one - "I know, I am keeping it" - is
//! stored, on the pair of the song and the word (`repeat_kept`).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::{Connection, params};
use serde::Serialize;
use time::{Date, macros::format_description};

use super::Bank;
use super::matching::{Matcher, Stems};
use crate::error::Result;
use crate::words::{self, pack};

/// How loud a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum RepeatLevel {
    /// Spent, or said long ago: worth a look.
    Orange,
    /// A rare word said lately: the listener remembers it.
    Red,
}

/// RepeatWhy two songs sharing a word is a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum RepeatWhy {
    /// A term of the register: spent, whenever it was said.
    Register,
    /// A rare word: the closer the other song, the louder.
    Rare,
}

/// One word a song shares with the songs that went out or are booked: one
/// finding a word, however many terms of the register catch it and however
/// many songs say it - what "I know, I am keeping it" is said about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct RepeatFinding {
    /// The word as this song writes it.
    pub word: String,
    /// The loudest any of the other songs makes it.
    pub level: RepeatLevel,
    pub why: RepeatWhy,
    /// The other song that makes it loudest - of those, the latest.
    pub neighbour_id: String,
    pub neighbour_title: String,
    /// The day it went out, or is booked for, `YYYY-MM-DD`.
    pub day: String,
    /// Booked rather than out.
    pub booked: bool,
    /// How many more songs out or booked say it.
    pub also: u32,
    /// The person said they know and keep it: drawn, not counted.
    pub kept: bool,
}

impl RepeatFinding {
    /// Whether this finding says more than `other` about the same word: the
    /// louder, then the later.
    fn outranks(&self, other: &RepeatFinding) -> bool {
        (self.level, &self.day) > (other.level, &other.day)
    }
}

/// What the guard says about one song.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct Repeats {
    /// The song.
    pub work_id: String,
    pub title: String,
    /// The loudest finding not kept; none when every one is kept.
    pub level: Option<RepeatLevel>,
    /// Loudest first, then the most recent neighbour first.
    pub findings: Vec<RepeatFinding>,
}

/// The mark a work wears in a list, a chip, a header: its own song's level,
/// and the finding that says why - for a publication, the song it is made
/// from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct RepeatMark {
    pub work_id: String,
    /// The song the mark is about: the work itself, or what it is made from.
    pub song_id: String,
    pub level: RepeatLevel,
    /// The loudest finding, for a hint: the word, the other song, the day.
    pub top: RepeatFinding,
    /// How many findings stand, not kept.
    pub count: usize,
}

/// A song's text, and where it stands.
struct Song {
    id: String,
    title: String,
    body: String,
    /// The first day something made from it went out.
    out: Option<Date>,
    /// The first day something made from it is booked for, not out yet.
    booked: Option<Date>,
}

impl Song {
    /// The day it said its words to the audience, or will.
    fn day(&self) -> Option<(Date, bool)> {
        match (self.out, self.booked) {
            (Some(out), _) => Some((out, false)),
            (None, Some(booked)) => Some((booked, true)),
            (None, None) => None,
        }
    }
}

fn day_of(raw: &str) -> Option<Date> {
    let format = format_description!("[year]-[month]-[day]");
    Date::parse(raw.get(..10)?, &format).ok()
}

fn iso(day: Date) -> String {
    let format = format_description!("[year]-[month]-[day]");
    day.format(&format).unwrap_or_default()
}

/// Every work with a sung text, the text of its sung roles as they stand -
/// the latest version of each - and where it stands, read off its own
/// releases and those of everything made from it.
fn songs(conn: &Connection, profile_id: &str) -> Result<Vec<Song>> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let mut statement = conn.prepare(
        "SELECT w.id, w.title, w.kind, v.role, v.body FROM work w
           JOIN work_version v ON v.work_id = w.id
          WHERE w.profile_id = ?1
            AND v.id = (SELECT v2.id FROM work_version v2
                         WHERE v2.work_id = w.id AND v2.role = v.role
                         ORDER BY v2.created_at DESC, v2.rowid DESC LIMIT 1)
          ORDER BY w.created_at, w.rowid",
    )?;
    let rows = statement
        .query_map(params![profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut by_work: Vec<Song> = Vec::new();
    for (id, title, kind, role, body) in rows {
        let sung = config
            .vocabulary(&kind)
            .version_roles
            .iter()
            .any(|known| known.key == role && known.sung);
        if !sung {
            continue;
        }
        match by_work.iter_mut().find(|song| song.id == id) {
            Some(song) => {
                song.body.push('\n');
                song.body.push_str(&body);
            }
            None => by_work.push(Song {
                id,
                title,
                body,
                out: None,
                booked: None,
            }),
        }
    }

    // A release speaks for its own work and for everything up the chain it
    // was made from: a song is out when its audio is.
    let up = crate::link::ancestors_by_work(conn, profile_id)?;
    let mut statement = conn.prepare(
        "SELECT r.work_id, r.status, r.released_at, r.scheduled_at FROM release r
           JOIN work w ON w.id = r.work_id
          WHERE w.profile_id = ?1",
    )?;
    let releases = statement
        .query_map(params![profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (work_id, status, released_at, scheduled_at) in releases {
        let out = (status == crate::release::RELEASED)
            .then(|| {
                released_at
                    .as_deref()
                    .or(scheduled_at.as_deref())
                    .and_then(day_of)
            })
            .flatten();
        let booked = (status != crate::release::RELEASED)
            .then(|| scheduled_at.as_deref().and_then(day_of))
            .flatten();
        let speaks_for = std::iter::once(&work_id).chain(up.get(&work_id).into_iter().flatten());
        for id in speaks_for {
            if let Some(song) = by_work.iter_mut().find(|song| song.id == *id) {
                if let Some(out) = out {
                    song.out = Some(song.out.map_or(out, |known| known.min(out)));
                }
                if let Some(booked) = booked {
                    song.booked = Some(song.booked.map_or(booked, |known| known.min(booked)));
                }
            }
        }
    }
    Ok(by_work)
}

/// The words of a text by stem, each with the first way the text writes it:
/// what two texts share. Section names, function words and words of fewer
/// than three letters are not words a listener remembers.
fn stems_of(text: &str, stems: &mut Stems) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for word in words::words(text) {
        if word.bracketed || word.is_stop() || word.text.chars().count() < 3 {
            continue;
        }
        let key = stems.of(word.text);
        out.entry(key)
            .or_insert_with(|| words::sung::clean(word.text, &[]).to_lowercase());
    }
    out
}

/// What the guard says about every song of a profile that has not gone out,
/// on `today` (`YYYY-MM-DD`): the songs it shares something with, and how
/// loudly. Songs with nothing to say are left out.
pub fn all(conn: &Connection, profile_id: &str, today: &str) -> Result<Vec<Repeats>> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let guard = config.guard.unwrap_or_default();
    let Some(today) = day_of(today) else {
        return Err(crate::error::Error::refused("release.badDay").param("value", today));
    };
    let songs = songs(conn, profile_id)?;
    if songs.is_empty() {
        return Ok(Vec::new());
    }

    let mut stems = Stems::default();
    let words_of: Vec<BTreeMap<String, String>> = songs
        .iter()
        .map(|song| stems_of(&song.body, &mut stems))
        .collect();
    // In how many of the owner's works each stem stands.
    let mut corpus: HashMap<&str, usize> = HashMap::new();
    for words in &words_of {
        for stem in words.keys() {
            *corpus.entry(stem.as_str()).or_default() += 1;
        }
    }

    // The register, found in each text; and the bank's words by stem.
    let terms = super::list(conn, profile_id)?;
    let spent: Vec<super::Term> = terms
        .iter()
        .filter(|term| term.strictness.is_some())
        .cloned()
        .collect();
    let matcher = Matcher::new(&spent, &mut stems);
    let mut named: HashMap<String, BTreeSet<String>> = HashMap::new();
    {
        let mut statement =
            conn.prepare("SELECT term_id, work_id FROM term_work WHERE profile_id = ?1")?;
        let rows = statement.query_map(params![profile_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (term, work) = row?;
            named.entry(work).or_default().insert(term);
        }
    }
    let taken: Vec<BTreeMap<usize, String>> = songs
        .iter()
        .map(|song| {
            let units: Vec<u16> = song.body.encode_utf16().collect();
            let mut out: BTreeMap<usize, String> = BTreeMap::new();
            for hit in matcher.find(&song.body, &mut stems) {
                let said = String::from_utf16_lossy(&units[hit.start..hit.end]);
                out.entry(hit.term)
                    .or_insert_with(|| words::sung::clean(&said, &[]).to_lowercase());
            }
            for (index, term) in spent.iter().enumerate() {
                if named
                    .get(&song.id)
                    .is_some_and(|ids| ids.contains(&term.id))
                {
                    out.entry(index).or_insert_with(|| term.word.clone());
                }
            }
            out
        })
        .collect();
    let banked: BTreeSet<String> = terms
        .iter()
        .filter(|term| term.bank.is_some() && term.bank != Some(Bank::Dropped))
        .flat_map(|term| std::iter::once(&term.word).chain(term.forms.iter()))
        .map(|word| stems.of(word))
        .collect();
    // A word is rare when the bank keeps it, or when it is a Russian word
    // past the rank and in no more than a couple of the owner's works.
    let is_rare = |stem: &str, word: &str| -> bool {
        if banked.contains(stem) {
            return true;
        }
        if !pack::is_cyrillic(word) {
            return false;
        }
        let in_language = pack::rank(stem).is_none_or(|rank| rank > u64::from(guard.rare_rank));
        let in_works = corpus.get(stem).copied().unwrap_or(0);
        in_language && in_works <= guard.rare_in_works as usize
    };
    let near =
        |day: Date| (day - today).whole_days().unsigned_abs() <= u64::from(guard.window_days);

    let kept = kept_pairs(conn, profile_id, &mut stems)?;

    let mut out = Vec::new();
    for (index, song) in songs.iter().enumerate() {
        if song.out.is_some() {
            continue;
        }
        // One finding a stem, with the songs that say it.
        let mut by_stem: BTreeMap<String, (RepeatFinding, BTreeSet<String>)> = BTreeMap::new();
        let mut add = |stem: String, found: RepeatFinding| {
            let neighbour = found.neighbour_id.clone();
            match by_stem.get_mut(&stem) {
                Some((known, neighbours)) => {
                    if found.outranks(known) {
                        *known = found;
                    }
                    neighbours.insert(neighbour);
                }
                None => {
                    by_stem.insert(stem, (found, BTreeSet::from([neighbour])));
                }
            }
        };
        for (other, neighbour) in songs.iter().enumerate() {
            if other == index {
                continue;
            }
            let Some((day, booked)) = neighbour.day() else {
                continue;
            };
            let finding =
                |word: &str, why: RepeatWhy, level: RepeatLevel, stem: &str| RepeatFinding {
                    word: word.to_owned(),
                    level,
                    why,
                    neighbour_id: neighbour.id.clone(),
                    neighbour_title: neighbour.title.clone(),
                    day: iso(day),
                    booked,
                    also: 0,
                    kept: kept.contains(&(song.id.clone(), stem.to_owned())),
                };
            let mut said: BTreeSet<String> = BTreeSet::new();
            for (term, word) in &taken[index] {
                if !taken[other].contains_key(term) {
                    continue;
                }
                let stem = stems.of(word);
                said.insert(stem.clone());
                let found = finding(word, RepeatWhy::Register, RepeatLevel::Orange, &stem);
                add(stem, found);
            }
            for (stem, word) in &words_of[index] {
                if said.contains(stem)
                    || !words_of[other].contains_key(stem)
                    || !is_rare(stem, word)
                {
                    continue;
                }
                let level = if near(day) {
                    RepeatLevel::Red
                } else {
                    RepeatLevel::Orange
                };
                add(stem.clone(), finding(word, RepeatWhy::Rare, level, stem));
            }
        }
        if by_stem.is_empty() {
            continue;
        }
        let mut findings: Vec<RepeatFinding> = by_stem
            .into_values()
            .map(|(mut found, neighbours)| {
                found.also = u32::try_from(neighbours.len().saturating_sub(1)).unwrap_or(u32::MAX);
                found
            })
            .collect();
        findings.sort_by(|a, b| {
            a.kept
                .cmp(&b.kept)
                .then(b.level.cmp(&a.level))
                .then_with(|| b.day.cmp(&a.day))
                .then_with(|| a.word.cmp(&b.word))
        });
        let level = findings
            .iter()
            .filter(|finding| !finding.kept)
            .map(|finding| finding.level)
            .max();
        out.push(Repeats {
            work_id: song.id.clone(),
            title: song.title.clone(),
            level,
            findings,
        });
    }
    Ok(out)
}

/// The songs that went out, newest first, each with its day and its words -
/// for `{released}`, what an action reads a song against for a meaning the
/// stemmer cannot see (ADR 0054). The song asking is left out.
pub fn released_sheet(conn: &Connection, profile_id: &str, except: &str) -> Result<String> {
    let mut out: Vec<Song> = songs(conn, profile_id)?
        .into_iter()
        .filter(|song| song.out.is_some() && song.id != except)
        .collect();
    if out.is_empty() {
        return Ok("(no song has gone out yet)".to_owned());
    }
    out.sort_by(|a, b| b.out.cmp(&a.out).then_with(|| a.title.cmp(&b.title)));
    let mut sheet = String::from("The songs that went out, newest first:\n");
    for song in out {
        sheet.push_str(&format!(
            "\n### “{}” - out {}\n\n{}\n",
            song.title,
            song.out.map(iso).unwrap_or_default(),
            crate::words::sung::clean(song.body.trim(), &[])
        ));
    }
    Ok(sheet.trim_end().to_owned())
}

/// The pairs of a song and a word the person said they keep, by stem.
fn kept_pairs(
    conn: &Connection,
    profile_id: &str,
    stems: &mut Stems,
) -> Result<BTreeSet<(String, String)>> {
    let mut statement =
        conn.prepare("SELECT work_id, word FROM repeat_kept WHERE profile_id = ?1")?;
    let rows = statement
        .query_map(params![profile_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows
        .into_iter()
        .map(|(work, word)| (work, stems.of(&word)))
        .collect())
}

/// The mark every work wears: a song its own, a publication its song's - the
/// nearest work up the chain the guard has something to say about. Works with
/// nothing to wear are left out, and so is a song whose findings are all
/// kept.
pub fn marks(conn: &Connection, profile_id: &str, today: &str) -> Result<Vec<RepeatMark>> {
    let found = all(conn, profile_id, today)?;
    let by_song: HashMap<&str, &Repeats> = found
        .iter()
        .map(|repeats| (repeats.work_id.as_str(), repeats))
        .collect();
    let mark = |work_id: &str, repeats: &Repeats| -> Option<RepeatMark> {
        let level = repeats.level?;
        let standing: Vec<&RepeatFinding> = repeats
            .findings
            .iter()
            .filter(|finding| !finding.kept)
            .collect();
        Some(RepeatMark {
            work_id: work_id.to_owned(),
            song_id: repeats.work_id.clone(),
            level,
            top: (*standing.first()?).clone(),
            count: standing.len(),
        })
    };
    let mut out: Vec<RepeatMark> = found
        .iter()
        .filter_map(|repeats| mark(&repeats.work_id, repeats))
        .collect();
    for (work_id, ancestors) in crate::link::ancestors_by_work(conn, profile_id)? {
        if by_song.contains_key(work_id.as_str()) {
            continue;
        }
        if let Some(repeats) = ancestors.iter().find_map(|id| by_song.get(id.as_str()))
            && let Some(found) = mark(&work_id, repeats)
        {
            out.push(found);
        }
    }
    Ok(out)
}

/// What the guard says about the song a work is, or is made from.
pub fn of_work(conn: &Connection, work_id: &str, today: &str) -> Result<Option<Repeats>> {
    let Some(work) = crate::work::get(conn, work_id)? else {
        return Ok(None);
    };
    let found = all(conn, &work.profile_id, today)?;
    if let Some(own) = found.iter().find(|repeats| repeats.work_id == work_id) {
        return Ok(Some(own.clone()));
    }
    for id in crate::link::ancestors(conn, work_id)? {
        if let Some(theirs) = found.iter().find(|repeats| repeats.work_id == id) {
            return Ok(Some(theirs.clone()));
        }
    }
    Ok(None)
}

/// The findings that would be red were `work_id` to go out on `slot`: a rare
/// word of its song shared with a song out or booked within the window of
/// that day. What the calendar says before the release lands (v0.90).
pub fn at_slot(conn: &Connection, work_id: &str, slot: &str) -> Result<Vec<RepeatFinding>> {
    let Some(work) = crate::work::get(conn, work_id)? else {
        return Ok(Vec::new());
    };
    let Some(slot_day) = day_of(slot) else {
        return Ok(Vec::new());
    };
    let guard = crate::profile::config_for(conn, &work.profile_id)?
        .guard
        .unwrap_or_default();
    let Some(repeats) = of_work(conn, work_id, slot)? else {
        return Ok(Vec::new());
    };
    Ok(repeats
        .findings
        .into_iter()
        .filter(|finding| !finding.kept && finding.neighbour_id != work_id)
        .filter(|finding| {
            day_of(&finding.day).is_some_and(|day| {
                (day - slot_day).whole_days().unsigned_abs() <= u64::from(guard.window_days)
            })
        })
        .filter(|finding| finding.level == RepeatLevel::Red)
        .collect())
}

/// Whether the person said they keep `word` in `work_id`'s song.
pub fn is_kept(conn: &Connection, work_id: &str, word: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM repeat_kept WHERE work_id = ?1 AND word = ?2)",
        params![work_id, word.trim().to_lowercase()],
        |row| row.get(0),
    )?)
}

/// "I know, I am keeping it": the finding on this word stays drawn and stops
/// counting. With the id and moment decided (ADR 0014).
pub fn keep_minted(
    conn: &Connection,
    profile_id: &str,
    work_id: &str,
    word: &str,
    minted: crate::minted::Minted,
) -> Result<()> {
    let word = word.trim().to_lowercase();
    if word.is_empty() {
        return Err(crate::error::Error::refused("repeat.needsWord"));
    }
    conn.execute(
        "INSERT INTO repeat_kept (id, profile_id, work_id, word, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT (work_id, word) DO NOTHING",
        params![minted.id(), profile_id, work_id, word, minted.at()],
    )?;
    Ok(())
}

/// Take back "I am keeping it". Whether there was one.
pub fn unkeep(conn: &Connection, work_id: &str, word: &str) -> Result<bool> {
    Ok(conn.execute(
        "DELETE FROM repeat_kept WHERE work_id = ?1 AND word = ?2",
        params![work_id, word.trim().to_lowercase()],
    )? > 0)
}

/// Drop the decisions about works that are gone for good: not in the table
/// and not in the trash to come back from. Run at startup, as the dashboard's
/// dismissals are (ADR 0010).
pub fn sweep(conn: &Connection) -> Result<usize> {
    Ok(conn.execute(
        "DELETE FROM repeat_kept
          WHERE work_id NOT IN (SELECT id FROM work)
            AND work_id NOT IN (SELECT entity_id FROM deletion WHERE entity = 'work')",
        [],
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::register::{self, NewTerm, Strictness};

    const TODAY: &str = "2026-10-01";

    /// A song with its lyric, and - when `out` names a day - an audio made
    /// from it that went out that day.
    fn song(
        conn: &Connection,
        profile_id: &str,
        title: &str,
        text: &str,
        out: Option<&str>,
    ) -> String {
        let made = fixtures::song(conn, profile_id, title);
        fixtures::version(conn, &made.id, "lyrics", text);
        if let Some(day) = out {
            let audio =
                crate::actions::work::derive(conn, &made.id, "audio", None, None, None).unwrap();
            crate::release::mark_released(
                conn,
                audio.release_id.as_deref().unwrap(),
                None,
                Some(day.into()),
            )
            .unwrap();
        }
        made.id
    }

    fn findings(
        conn: &Connection,
        profile_id: &str,
        work_id: &str,
    ) -> Vec<(String, RepeatLevel, RepeatWhy, bool)> {
        all(conn, profile_id, TODAY)
            .unwrap()
            .into_iter()
            .find(|repeats| repeats.work_id == work_id)
            .map(|repeats| {
                repeats
                    .findings
                    .into_iter()
                    .map(|f| (f.word, f.level, f.why, f.kept))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn a_spent_term_is_orange_and_a_rare_word_is_red_while_it_is_recent() {
        let (conn, profile_id) = fixtures::workspace();
        register::create(
            &conn,
            &profile_id,
            NewTerm {
                word: "кофе".into(),
                ..NewTerm::default()
            },
        )
        .unwrap();
        song(
            &conn,
            &profile_id,
            "Out lately",
            "кофе на столе и пульсар над крышей",
            Some("2026-09-20"),
        );
        let draft = song(
            &conn,
            &profile_id,
            "Draft",
            "остывший кофе, далёкий пульсар",
            None,
        );

        assert_eq!(
            findings(&conn, &profile_id, &draft),
            vec![
                (
                    "пульсар".to_owned(),
                    RepeatLevel::Red,
                    RepeatWhy::Rare,
                    false
                ),
                (
                    "кофе".to_owned(),
                    RepeatLevel::Orange,
                    RepeatWhy::Register,
                    false
                ),
            ]
        );
    }

    #[test]
    fn a_word_is_one_finding_however_many_terms_catch_it_and_songs_say_it() {
        let (conn, profile_id) = fixtures::workspace();
        for term in [
            NewTerm {
                word: "кофе".into(),
                ..NewTerm::default()
            },
            NewTerm {
                word: "напиток".into(),
                forms: vec!["кофе".into()],
                ..NewTerm::default()
            },
            NewTerm::banked("пульсар"),
        ] {
            register::create(&conn, &profile_id, term).unwrap();
        }
        song(
            &conn,
            &profile_id,
            "Long ago",
            "кофе и пульсар",
            Some("2026-01-10"),
        );
        let lately = song(
            &conn,
            &profile_id,
            "Lately",
            "пульсар и кофе",
            Some("2026-09-20"),
        );
        let draft = song(&conn, &profile_id, "Draft", "кофе, пульсар", None);

        let found = all(&conn, &profile_id, TODAY)
            .unwrap()
            .into_iter()
            .find(|repeats| repeats.work_id == draft)
            .unwrap()
            .findings;
        let said: Vec<(&str, RepeatLevel, &str, u32)> = found
            .iter()
            .map(|f| (f.word.as_str(), f.level, f.neighbour_id.as_str(), f.also))
            .collect();
        assert_eq!(
            said,
            vec![
                ("пульсар", RepeatLevel::Red, lately.as_str(), 1),
                ("кофе", RepeatLevel::Orange, lately.as_str(), 1),
            ],
            "the loudest song names the word, the latest of equals; the other is counted"
        );
    }

    #[test]
    fn a_rare_word_said_long_ago_is_orange_and_a_word_the_owner_says_everywhere_is_not_rare() {
        let (conn, profile_id) = fixtures::workspace();
        song(
            &conn,
            &profile_id,
            "Long ago",
            "пульсар и туманность",
            Some("2026-01-10"),
        );
        song(
            &conn,
            &profile_id,
            "Two",
            "туманность снова",
            Some("2026-09-25"),
        );
        song(
            &conn,
            &profile_id,
            "Three",
            "туманность опять",
            Some("2026-09-26"),
        );
        let draft = song(&conn, &profile_id, "Draft", "пульсар, туманность", None);

        assert_eq!(
            findings(&conn, &profile_id, &draft),
            vec![(
                "пульсар".to_owned(),
                RepeatLevel::Orange,
                RepeatWhy::Rare,
                false
            )],
            "туманность stands in four of the owner's works: it is theirs, not rare"
        );
    }

    #[test]
    fn a_word_of_the_bank_is_rare_however_common_and_a_song_that_went_out_is_not_held() {
        let (conn, profile_id) = fixtures::workspace();
        register::create(&conn, &profile_id, NewTerm::banked("окно")).unwrap();
        let out = song(
            &conn,
            &profile_id,
            "Out",
            "открытое окно",
            Some("2026-09-01"),
        );
        let draft = song(&conn, &profile_id, "Draft", "моё окно", None);

        assert_eq!(
            findings(&conn, &profile_id, &draft),
            vec![("окно".to_owned(), RepeatLevel::Red, RepeatWhy::Rare, false)]
        );
        assert!(
            findings(&conn, &profile_id, &out).is_empty(),
            "it went out already"
        );
    }

    /// The register's own word is spent, whatever it calls it and however
    /// recently it went out: orange. A word in another script has no
    /// frequency to read, and is not rare unless the bank keeps it.
    #[test]
    fn a_term_of_the_register_is_orange_and_a_foreign_word_is_not_rare() {
        let (conn, profile_id) = fixtures::workspace();
        register::create(
            &conn,
            &profile_id,
            NewTerm {
                word: "пульсар".into(),
                strictness: Some(Strictness::Rare),
                ..NewTerm::default()
            },
        )
        .unwrap();
        song(
            &conn,
            &profile_id,
            "Out lately",
            "пульсар, lighthouse",
            Some("2026-09-20"),
        );
        let draft = song(&conn, &profile_id, "Draft", "пульсар и lighthouse", None);

        assert_eq!(
            findings(&conn, &profile_id, &draft),
            vec![(
                "пульсар".to_owned(),
                RepeatLevel::Orange,
                RepeatWhy::Register,
                false
            )]
        );
    }

    #[test]
    fn a_word_kept_stays_drawn_and_stops_counting() {
        let (conn, profile_id) = fixtures::workspace();
        song(&conn, &profile_id, "Out", "пульсар", Some("2026-09-20"));
        let draft = song(&conn, &profile_id, "Draft", "пульсар", None);

        crate::actions::register::keep_repeat(&conn, &draft, "Пульсар").unwrap();
        crate::actions::register::keep_repeat(&conn, &draft, "пульсар").unwrap();

        let found = all(&conn, &profile_id, TODAY).unwrap();
        let repeats = found.iter().find(|r| r.work_id == draft).unwrap();
        assert_eq!(repeats.level, None);
        assert!(repeats.findings[0].kept);
        assert!(marks(&conn, &profile_id, TODAY).unwrap().is_empty());

        crate::actions::register::unkeep_repeat(&conn, &draft, "пульсар").unwrap();
        assert_eq!(
            all(&conn, &profile_id, TODAY).unwrap()[0].level,
            Some(RepeatLevel::Red)
        );
    }

    #[test]
    fn a_publication_wears_its_songs_mark_and_a_day_near_the_neighbour_warns() {
        let (conn, profile_id) = fixtures::workspace();
        song(&conn, &profile_id, "Out", "пульсар", Some("2026-09-20"));
        let draft = song(&conn, &profile_id, "Draft", "пульсар", None);
        let clip = crate::actions::work::derive(&conn, &draft, "video", None, None, None).unwrap();

        let worn: Vec<(String, String, RepeatLevel)> = marks(&conn, &profile_id, TODAY)
            .unwrap()
            .into_iter()
            .map(|mark| (mark.work_id, mark.song_id, mark.level))
            .collect();
        assert!(worn.contains(&(draft.clone(), draft.clone(), RepeatLevel::Red)));
        assert!(worn.contains(&(clip.work.id.clone(), draft.clone(), RepeatLevel::Red)));

        assert_eq!(
            at_slot(&conn, &clip.work.id, "2026-10-05").unwrap().len(),
            1
        );
        assert!(
            at_slot(&conn, &clip.work.id, "2027-06-01")
                .unwrap()
                .is_empty(),
            "far from the neighbour, nothing to warn about"
        );
    }

    #[test]
    fn a_booked_song_is_a_neighbour_by_its_day() {
        let (conn, profile_id) = fixtures::workspace();
        let booked = song(&conn, &profile_id, "Booked", "пульсар", None);
        let audio =
            crate::actions::work::derive(&conn, &booked, "audio", None, None, None).unwrap();
        crate::release::schedule(&conn, audio.release_id.as_deref().unwrap(), "2026-10-10")
            .unwrap();
        let draft = song(&conn, &profile_id, "Draft", "пульсар", None);

        let found = all(&conn, &profile_id, TODAY).unwrap();
        let mine = found.iter().find(|r| r.work_id == draft).unwrap();
        assert!(mine.findings[0].booked);
        assert_eq!(mine.findings[0].day, "2026-10-10");
        assert_eq!(mine.level, Some(RepeatLevel::Red));
    }
}
