//! The library: the works of culture a workspace's boards draw on (v0.100,
//! ADR 0066).
//!
//! A song is researched before it is written - the novels, films, paintings
//! and gospels it could draw on - and the same film turns up in the research
//! of two songs. It is one film: an entry of the library is a note of the
//! kind the profile marks `library`, kept once for the whole workspace, and a
//! card on a work's board links to it with what it is to that work. The
//! entry says what the work of culture is - its title, its author, its other
//! titles, its medium, when it was made - and nothing of any one song.
//!
//! An entry is found again by its name, so an answer that brings "Old Joy"
//! for the second song links to the entry the first song made rather than to
//! a second copy beside it.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::note::{Note, NoteFilter};
use crate::profile::config::{NoteKind, ProfileConfig};

/// An entry of the library as a card shows it: read off the note it is,
/// never copied onto the card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
#[ts(rename = "LibraryEntry")]
pub struct Entry {
    /// The note's id.
    pub id: String,
    pub title: String,
    /// Who made it: the novelist, the director, the painter. The note's
    /// attribution.
    pub author: Option<String>,
    pub year: Option<String>,
    /// By the key of a medium the library's kind declares.
    pub medium: Option<String>,
    /// Its other titles: the original, a translation.
    pub aliases: Vec<String>,
}

impl Entry {
    /// What the entry is called, the way a list or a writer reads it: the
    /// author, the title, when it was made - "Kelly Reichardt, “Old Joy”,
    /// 2006". A part with nothing in it is left out.
    pub fn headline(&self) -> String {
        let mut out = String::new();
        if let Some(author) = self.author.as_deref().filter(|a| !a.trim().is_empty()) {
            out.push_str(author.trim());
            out.push_str(", ");
        }
        out.push_str(&format!("“{}”", self.title.trim()));
        if let Some(year) = self.year.as_deref().filter(|y| !y.trim().is_empty()) {
            out.push_str(&format!(", {}", year.trim()));
        }
        out
    }

    /// The headline with the medium's word after it, when it has one.
    pub fn described(&self, library: Option<&NoteKind>) -> String {
        match self.medium_word(library) {
            Some(word) => format!("{} · {word}", self.headline()),
            None => self.headline(),
        }
    }

    /// The word the library has for the entry's medium.
    pub fn medium_word(&self, library: Option<&NoteKind>) -> Option<String> {
        let key = self.medium.as_deref()?;
        Some(
            library
                .and_then(|kind| kind.medium(key))
                .map_or_else(|| key.to_owned(), |medium| medium.label.to_string()),
        )
    }

    /// Whether a work of the entry's medium is ever quoted to a writer. An
    /// entry with no medium, or one the library no longer names, is.
    pub fn quoted(&self, library: Option<&NoteKind>) -> bool {
        self.medium
            .as_deref()
            .and_then(|key| library.and_then(|kind| kind.medium(key)))
            .is_none_or(|medium| medium.quoted)
    }

    /// Every name it goes by.
    fn names(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.title.as_str()).chain(self.aliases.iter().map(String::as_str))
    }

    /// Whether the entry already goes by this name, in any case and quotes.
    pub fn knows(&self, name: &str) -> bool {
        let name = key_of(name);
        !name.is_empty() && self.names().any(|known| key_of(known) == name)
    }

    /// The entry a note is, when it is one of the library.
    pub fn of(note: &Note) -> Entry {
        Entry {
            id: note.id.clone(),
            title: note.title.clone().unwrap_or_default(),
            author: note.attribution.clone(),
            year: note.year.clone(),
            medium: note.medium.clone(),
            aliases: note.aliases.clone(),
        }
    }
}

/// What an answer or an agent says a work of culture is, before it is found
/// in the library or written into it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
#[ts(rename = "LibraryAsked")]
pub struct Asked {
    pub title: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    /// By the key of a medium of the library; a word it does not name is left
    /// out when the answer is read.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<String>,
    /// Its title in the language it was made in, when the title above is a
    /// translation: "Old Joy" for «Старая радость».
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original: Option<String>,
}

impl Asked {
    /// Every name it is asked by.
    fn names(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.title.as_str()).chain(self.original.as_deref())
    }

    /// The entry it would be, as a proposal shows it before it lands.
    pub fn headline(&self) -> String {
        Entry {
            id: String::new(),
            title: self.title.clone(),
            author: self.author.clone(),
            year: self.year.clone(),
            medium: self.medium.clone(),
            aliases: Vec::new(),
        }
        .headline()
    }
}

/// A name as two names are compared: lowercased, `ё` as `е`, quotes and
/// the space around words dropped - «Старая радость» is “старая радость”.
fn key_of(name: &str) -> String {
    crate::words::plain(name)
        .chars()
        .filter(|c| !matches!(c, '«' | '»' | '“' | '”' | '„' | '"' | '\''))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether a text says something.
fn said(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

/// Whether the entry is the work asked for: a name of one is a name of the
/// other, and their authors do not contradict each other - the same, one of
/// them unsaid, or, written in two scripts, made the same year. Two films
/// called "Solaris" by two directors are two entries.
fn is(entry: &Entry, asked: &Asked) -> bool {
    let named = entry.names().any(|name| {
        let name = key_of(name);
        !name.is_empty() && asked.names().any(|other| key_of(other) == name)
    });
    if !named {
        return false;
    }
    match (said(entry.author.as_deref()), said(asked.author.as_deref())) {
        (Some(one), Some(other)) if key_of(one) != key_of(other) => {
            matches!(
                (said(entry.year.as_deref()), said(asked.year.as_deref())),
                (Some(one), Some(other)) if one == other
            )
        }
        _ => true,
    }
}

/// Every entry of the library of a profile, newest first; none when the
/// profile has no library.
pub fn entries(conn: &Connection, profile_id: &str) -> Result<Vec<Entry>> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let Some(library) = config.library() else {
        return Ok(Vec::new());
    };
    Ok(crate::note::list(
        conn,
        profile_id,
        &NoteFilter {
            kind: Some(library.key.clone()),
            ..NoteFilter::default()
        },
    )?
    .iter()
    .map(Entry::of)
    .collect())
}

/// The entry of the library that is the work asked for, when the library
/// has one.
pub fn find(conn: &Connection, profile_id: &str, asked: &Asked) -> Result<Option<Entry>> {
    if said(Some(&asked.title)).is_none() {
        return Ok(None);
    }
    Ok(entries(conn, profile_id)?
        .into_iter()
        .find(|entry| is(entry, asked)))
}

/// The entry of the library a note is, or the refusal that says the note is
/// not one: it is gone, of another profile, or of another kind.
pub fn require(conn: &Connection, profile_id: &str, note_id: &str) -> Result<Entry> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let note = crate::note::get(conn, note_id)?
        .filter(|note| note.profile_id == profile_id)
        .ok_or_else(|| Error::not_found("note", note_id))?;
    match config.library() {
        Some(library) if library.key == note.kind => Ok(Entry::of(&note)),
        _ => Err(Error::refused("library.notAnEntry")
            .param("title", note.title.clone().unwrap_or_default())),
    }
}

/// The entry a card links to, read whole: what a board shows.
pub fn get(conn: &Connection, note_id: &str) -> Result<Option<Entry>> {
    Ok(crate::note::get(conn, note_id)?.map(|note| Entry::of(&note)))
}

/// A work an entry has worked in: it stands on that work's board, not
/// turned down.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
#[ts(rename = "WorkedIn")]
pub struct WorkedIn {
    pub work_id: String,
    pub title: String,
}

/// Where the entries on one board of a work have worked besides it: the
/// other works whose boards hold a card linking to each, not turned down.
/// What a card shows as "already worked in…", and what the guard of repeats
/// reads (ADR 0066).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct Elsewhere {
    pub note_id: String,
    pub works: Vec<WorkedIn>,
}

/// [`Elsewhere`] for every entry on a board of a work, by title of the other
/// work. An entry that worked nowhere else is left out.
pub fn elsewhere(conn: &Connection, work_id: &str, board: &str) -> Result<Vec<Elsewhere>> {
    let mut statement = conn.prepare(
        "SELECT DISTINCT c.note_id, w.id, w.title FROM board_card c
           JOIN work w ON w.id = c.work_id
          WHERE c.note_id IN (SELECT note_id FROM board_card
                               WHERE work_id = ?1 AND board = ?2 AND note_id IS NOT NULL)
            AND c.work_id <> ?1
            AND (c.verdict IS NULL OR c.verdict <> 'rejected')
          ORDER BY c.note_id, w.title, w.id",
    )?;
    let rows = statement
        .query_map(params![work_id, board], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut out: Vec<Elsewhere> = Vec::new();
    for (note_id, id, title) in rows {
        let found = WorkedIn { work_id: id, title };
        match out.last_mut() {
            Some(last) if last.note_id == note_id => last.works.push(found),
            _ => out.push(Elsewhere {
                note_id,
                works: vec![found],
            }),
        }
    }
    Ok(out)
}

/// Every work an entry has worked in: what the library shows on the entry.
pub fn worked_in(conn: &Connection, note_id: &str) -> Result<Vec<WorkedIn>> {
    let mut statement = conn.prepare(
        "SELECT DISTINCT w.id, w.title FROM board_card c
           JOIN work w ON w.id = c.work_id
          WHERE c.note_id = ?1 AND (c.verdict IS NULL OR c.verdict <> 'rejected')
          ORDER BY w.title, w.id",
    )?;
    let rows = statement
        .query_map(params![note_id], |row| {
            Ok(WorkedIn {
                work_id: row.get(0)?,
                title: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Refuse to delete an entry a board still links to, naming the works: the
/// cards would be left saying what they are to a work of culture nobody can
/// name. Taking the cards off the boards - or the works away - comes first.
pub fn refuse_while_linked(conn: &Connection, note_id: &str) -> Result<()> {
    let linked: Option<String> = conn
        .query_row(
            "SELECT group_concat(title, ', ') FROM (
                 SELECT DISTINCT w.title FROM board_card c JOIN work w ON w.id = c.work_id
                  WHERE c.note_id = ?1 ORDER BY w.title)",
            params![note_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    match linked {
        Some(works) if !works.is_empty() => {
            Err(Error::refused("library.onBoards").param("works", works))
        }
        _ => Ok(()),
    }
}

/// What a writer is told the library already gave other works: every entry
/// that stands, not turned down, on a board of a work other than this one,
/// with the works it worked in - so a run brings one back only when nothing
/// new fits as well. The most recent first, at most `most`.
pub fn others_sheet(
    conn: &Connection,
    config: &ProfileConfig,
    work_id: &str,
    most: usize,
) -> Result<String> {
    let mut statement = conn.prepare(
        "SELECT c.note_id, w.title, max(c.updated_at) AS at FROM board_card c
           JOIN work w ON w.id = c.work_id
          WHERE c.note_id IS NOT NULL AND c.work_id <> ?1
            AND (c.verdict IS NULL OR c.verdict <> 'rejected')
          GROUP BY c.note_id, w.id
          ORDER BY at DESC",
    )?;
    let rows = statement
        .query_map(params![work_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut order: Vec<String> = Vec::new();
    let mut works: std::collections::HashMap<String, Vec<String>> = Default::default();
    for (note_id, title) in rows {
        if !works.contains_key(&note_id) {
            order.push(note_id.clone());
        }
        works.entry(note_id).or_default().push(title);
    }
    let library = config.library();
    let mut lines = Vec::new();
    for note_id in order.into_iter().take(most) {
        let Some(entry) = get(conn, &note_id)? else {
            continue;
        };
        let titles = works
            .get(&note_id)
            .map(|titles| {
                titles
                    .iter()
                    .map(|title| format!("“{title}”"))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        lines.push(format!("- {} - in {titles}", entry.described(library)));
    }
    Ok(lines.join("\n"))
}

/// The chips of a field: one to a line, or parted by semicolons - each
/// trimmed, the empty ones left out, each once whatever its case.
pub fn chips(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for chip in text.split(['\n', ';']) {
        let chip = chip.trim().trim_end_matches(['.', ',']).trim();
        if chip.is_empty() || out.iter().any(|kept| key_of(kept) == key_of(chip)) {
            continue;
        }
        out.push(chip.to_owned());
    }
    out
}

/// Whether a text is a link a person can open: where a quote was checked.
pub fn is_link(text: &str) -> bool {
    let text = text.trim();
    (text.starts_with("https://") || text.starts_with("http://"))
        && text.len() > "https://".len()
        && !text.contains(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str, author: Option<&str>, year: Option<&str>) -> Entry {
        Entry {
            id: "e".into(),
            title: title.into(),
            author: author.map(str::to_owned),
            year: year.map(str::to_owned),
            medium: None,
            aliases: vec!["Old Joy".into()],
        }
    }

    fn asked(title: &str, author: Option<&str>, year: Option<&str>) -> Asked {
        Asked {
            title: title.into(),
            author: author.map(str::to_owned),
            year: year.map(str::to_owned),
            ..Asked::default()
        }
    }

    /// An entry is the work asked for by any of its names, in any case and
    /// quotes, unless the authors say they are two works - and authors in
    /// two scripts are one when the years agree.
    #[test]
    fn an_entry_is_found_by_its_names_unless_the_authors_disagree() {
        let film = entry("Старая радость", Some("Келли Райхардт"), Some("2006"));
        assert!(is(&film, &asked("«старая радость»", None, None)));
        assert!(is(&film, &asked("OLD JOY", Some("Келли Райхардт"), None)));
        assert!(is(
            &film,
            &asked("Old Joy", Some("Kelly Reichardt"), Some("2006"))
        ));
        assert!(!is(&film, &asked("Old Joy", Some("Someone else"), None)));
        assert!(!is(
            &film,
            &asked("Frances Ha", Some("Келли Райхардт"), None)
        ));
        assert!(!is(&film, &asked("  ", None, None)));
    }

    /// The headline reads author, title, year, leaving out what is unsaid.
    #[test]
    fn an_entry_is_called_by_its_author_title_and_year() {
        assert_eq!(
            entry("Old Joy", Some("Kelly Reichardt"), Some("2006")).headline(),
            "Kelly Reichardt, “Old Joy”, 2006"
        );
        assert_eq!(entry("The Odyssey", None, None).headline(), "“The Odyssey”");
    }

    /// Chips part on lines and semicolons, once each whatever their case.
    #[test]
    fn chips_are_parted_and_kept_once() {
        assert_eq!(
            chips("херувим; Арзамас\n  серафим.\nАрзамас;;  "),
            ["херувим", "Арзамас", "серафим"]
        );
    }

    /// A proof is a link that can be opened, nothing else.
    #[test]
    fn a_proof_is_a_link() {
        assert!(is_link("https://www.gutenberg.org/ebooks/1342"));
        assert!(!is_link("checked"));
        assert!(!is_link("https://"));
        assert!(!is_link("https://example.org/a b"));
    }
}
