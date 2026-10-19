//! The phrases as their screen reads them (ADR 0064).
//!
//! A phrase is a note of a kind the profile keeps as one line. The screen
//! shows each with what the rest of the record knows about it: the shelves it
//! stands on, how many works already say it, and - where the record keeps a
//! way of singing it - how it is sung. None of that is stored on the phrase:
//! the shelves are rows of their own, the works are read off their current
//! texts every time it is asked, the way the register's counts are (ADR
//! 0044), and the singing is the record's word on that word (ADR 0052).

use std::collections::HashMap;

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::error::Result;
use crate::note::{self, Note, NoteFilter};
use crate::register::{self, Sung, Term, TermKind, matching};

/// One phrase with what the record knows about it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct LineRow {
    #[serde(flatten)]
    pub note: Note,
    /// The shelves it stands on, by id.
    pub shelf_ids: Vec<String>,
    /// How many works say it now: found in their current text by its words,
    /// compared by stem, the way the register finds a term.
    pub uses: usize,
    /// How its forms are sung, where the record keeps a term of the same
    /// words that says so.
    pub sung: Vec<Sung>,
    /// That term, when there is one.
    pub term_id: Option<String>,
}

/// The phrases a filter leaves, each with its shelves, its uses and its
/// singing, in the order the notes list reads them.
///
/// The filter is narrowed to the lines whatever it says: a page is not a row
/// of this list.
pub fn rows(conn: &Connection, profile_id: &str, filter: &NoteFilter) -> Result<Vec<LineRow>> {
    let narrowed = NoteFilter {
        line: Some(true),
        canon: None,
        ..filter.clone()
    };
    let lines = note::list(conn, profile_id, &narrowed)?;
    if lines.is_empty() {
        return Ok(Vec::new());
    }

    let shelves = shelves_by_note(conn, profile_id)?;
    // The record's word on each word, by its plain spelling: a phrase written
    // "Марсель" is the term "марсель" and is sung the way it says.
    let terms: HashMap<String, Term> = register::list(conn, profile_id)?
        .into_iter()
        .map(|term| (crate::words::plain(&term.word), term))
        .collect();

    // Every phrase as a term of one moment, so the corpus is walked once for
    // all of them. Its forms are the term's where the record keeps one: "окон"
    // is the same word only because somebody said so.
    let drafts: Vec<Term> = lines
        .iter()
        .map(|line| {
            let known = terms.get(&crate::words::plain(line.body.trim()));
            Term {
                id: line.id.clone(),
                profile_id: profile_id.to_owned(),
                word: line.body.trim().to_owned(),
                forms: known.map(|term| term.forms.clone()).unwrap_or_default(),
                // A line is wording by nature - an image, a scene is the
                // register's own reading of a meaning, not a phrase's.
                kind: TermKind::guess(&line.body),
                strictness: None,
                sung: Vec::new(),
                topic: None,
                note: None,
                created_at: String::new(),
                updated_at: String::new(),
            }
        })
        .collect();
    let found = matching::found_in_corpus(conn, profile_id, &drafts)?;

    Ok(lines
        .into_iter()
        .map(|line| {
            let known = terms.get(&crate::words::plain(line.body.trim()));
            LineRow {
                shelf_ids: shelves.get(&line.id).cloned().unwrap_or_default(),
                uses: found.get(&line.id).map_or(0, HashMap::len),
                sung: known.map(|term| term.sung.clone()).unwrap_or_default(),
                term_id: known.map(|term| term.id.clone()),
                note: line,
            }
        })
        .collect())
}

/// The shelves each phrase of a profile stands on, in the order it was put
/// on them.
fn shelves_by_note(conn: &Connection, profile_id: &str) -> Result<HashMap<String, Vec<String>>> {
    let mut statement = conn.prepare(
        "SELECT note_id, shelf_id FROM shelf_note WHERE profile_id = ?1 ORDER BY created_at, rowid",
    )?;
    let rows = statement.query_map(params![profile_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let (note, shelf) = row?;
        out.entry(note).or_default().push(shelf);
    }
    Ok(out)
}

/// The lines for `{lines}` and `{lines:<shelf>}`: the fresh phrases kept for
/// the work - tied to it - or the fresh phrases of one shelf by name, each
/// with whose words it is and how it is sung where the record says so (ADR
/// 0064). A shelf nobody has is said so rather than read as an empty one: a
/// template that names a shelf renamed since is a mistake worth seeing in the
/// prompt.
pub fn sheet(
    conn: &Connection,
    profile_id: &str,
    work_id: &str,
    shelf: Option<&str>,
) -> Result<String> {
    let rows = rows(
        conn,
        profile_id,
        &NoteFilter {
            state: Some(note::NoteState::Fresh),
            ..NoteFilter::default()
        },
    )?;
    let (heading, kept): (String, Vec<&LineRow>) = match shelf {
        Some(named) => {
            let Some(found) = crate::shelf::find(conn, profile_id, named)? else {
                return Ok(format!("(there is no shelf called “{named}”)"));
            };
            let on: Vec<&LineRow> = rows
                .iter()
                .filter(|row| row.shelf_ids.contains(&found.id))
                .collect();
            if on.is_empty() {
                return Ok(format!("(the shelf “{}” holds no fresh lines)", found.name));
            }
            (
                format!(
                    "Lines the author keeps for works to come - the shelf “{}”.",
                    found.name
                ),
                on,
            )
        }
        None => {
            let tied: Vec<&LineRow> = rows
                .iter()
                .filter(|row| row.note.work_id.as_deref() == Some(work_id))
                .collect();
            if tied.is_empty() {
                return Ok("(no lines are kept for this work)".to_owned());
            }
            ("Lines the author keeps for this work.".to_owned(), tied)
        }
    };
    let lines: Vec<String> = kept.iter().map(|row| said(row)).collect();
    Ok(format!(
        "{heading} Use them where they fit, sung as written here:\n{}",
        lines.join("\n")
    ))
}

/// One line as a prompt reads it: the words, whose they are, how they are
/// sung.
fn said(row: &LineRow) -> String {
    let mut line = format!("- {}", row.note.body.trim());
    if let Some(whose) = row.note.attribution.as_deref() {
        line.push_str(&format!(" - {whose}"));
    }
    let sung: Vec<String> = row
        .sung
        .iter()
        .map(|one| {
            if crate::words::plain(&one.written) == crate::words::plain(&row.note.body) {
                one.sung.clone()
            } else {
                format!("{} → {}", one.written, one.sung)
            }
        })
        .collect();
    if !sung.is_empty() {
        line.push_str(&format!(" (sung: {})", sung.join(", ")));
    }
    line
}

/// The phrase of a profile written in these words, whatever their case: a
/// line typed twice is one phrase, not two.
pub fn find(conn: &Connection, profile_id: &str, body: &str) -> Result<Option<Note>> {
    let wanted = crate::words::plain(body.trim());
    if wanted.is_empty() {
        return Ok(None);
    }
    Ok(note::list(
        conn,
        profile_id,
        &NoteFilter {
            line: Some(true),
            ..NoteFilter::default()
        },
    )?
    .into_iter()
    .find(|line| crate::words::plain(line.body.trim()) == wanted))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::minted::Minted;
    use crate::note::NewNote;
    use crate::register::NewTerm;
    use crate::work::{self, NewWork};

    fn phrase(conn: &Connection, profile_id: &str, body: &str) -> Note {
        note::create(
            conn,
            profile_id,
            NewNote {
                body: body.into(),
                kind: Some("phrase".into()),
                ..NewNote::default()
            },
        )
        .unwrap()
    }

    fn song(conn: &Connection, profile_id: &str, title: &str, lyrics: &str) {
        let made = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: title.into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        crate::work::version::create(
            conn,
            &made.id,
            crate::work::version::NewVersion {
                role: "lyrics".into(),
                body: lyrics.into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
                trial_id: None,
            },
        )
        .unwrap();
    }

    /// What a row says is read, not stored: the works that say a phrase now,
    /// the shelves it stands on, and the record's way of singing its word.
    #[test]
    fn a_row_says_where_the_phrase_stands_and_how_it_is_sung() {
        let (conn, profile_id) = fixtures::workspace();
        song(
            &conn,
            &profile_id,
            "Harbour",
            "the lighthouse keeps the hours\nа на краю окна",
        );
        song(&conn, &profile_id, "Tide", "no lighthouse here");
        let lighthouse = phrase(&conn, &profile_id, "lighthouse");
        let edge = phrase(&conn, &profile_id, "на краю");
        let marseille = phrase(&conn, &profile_id, "Марсель");
        register::create(
            &conn,
            &profile_id,
            NewTerm {
                sung: vec![Sung {
                    written: "Марсель".into(),
                    sung: "МарсЭль".into(),
                }],
                ..NewTerm::unspent("марсель")
            },
        )
        .unwrap();
        let space =
            crate::shelf::create_minted(&conn, &profile_id, "Space", Minted::fresh()).unwrap();
        crate::shelf::add_minted(&conn, &space.id, &edge.id, Minted::fresh()).unwrap();

        let rows = rows(&conn, &profile_id, &NoteFilter::default()).unwrap();
        let row = |id: &str| rows.iter().find(|row| row.note.id == id).unwrap();

        assert_eq!(row(&lighthouse.id).uses, 2, "two songs say it");
        assert_eq!(
            row(&edge.id).uses,
            1,
            "a phrase is found by its stems in a row"
        );
        assert_eq!(row(&edge.id).shelf_ids, vec![space.id.clone()]);
        assert_eq!(row(&marseille.id).uses, 0);
        assert_eq!(
            row(&marseille.id).sung,
            vec![Sung {
                written: "Марсель".into(),
                sung: "МарсЭль".into(),
            }],
            "the record's word on the word, whatever its case"
        );
        assert!(row(&lighthouse.id).term_id.is_none());
    }

    #[test]
    fn a_page_is_not_a_row_whatever_the_filter_says() {
        let (conn, profile_id) = fixtures::workspace();
        phrase(&conn, &profile_id, "a line");
        note::create(
            &conn,
            &profile_id,
            NewNote {
                body: "a page".into(),
                ..NewNote::default()
            },
        )
        .unwrap();

        let rows = rows(
            &conn,
            &profile_id,
            &NoteFilter {
                line: Some(false),
                ..NoteFilter::default()
            },
        )
        .unwrap();
        assert_eq!(
            rows.iter()
                .map(|r| r.note.body.as_str())
                .collect::<Vec<_>>(),
            ["a line"]
        );
    }

    /// What a prompt is handed: the work's own fresh lines, or a shelf's -
    /// never a line given up on, and never a shelf that is not there passed
    /// off as an empty one.
    #[test]
    fn the_sheet_reads_the_lines_of_the_work_or_of_a_shelf() {
        let (conn, profile_id) = fixtures::workspace();
        let made = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Bottle".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        let tied = |body: &str, attribution: Option<&str>, state| {
            note::create(
                &conn,
                &profile_id,
                NewNote {
                    body: body.into(),
                    kind: Some("phrase".into()),
                    work_id: Some(made.id.clone()),
                    attribution: attribution.map(str::to_owned),
                    state: Some(state),
                    ..NewNote::default()
                },
            )
            .unwrap()
        };
        tied(
            "sorrow is worn-out joy",
            Some("Old Joy"),
            note::NoteState::Fresh,
        );
        tied("given up on", None, note::NoteState::Dropped);
        let nebula = phrase(&conn, &profile_id, "nebula");
        let space =
            crate::shelf::create_minted(&conn, &profile_id, "Space", Minted::fresh()).unwrap();
        crate::shelf::add_minted(&conn, &space.id, &nebula.id, Minted::fresh()).unwrap();

        let own = sheet(&conn, &profile_id, &made.id, None).unwrap();
        assert!(own.contains("- sorrow is worn-out joy - Old Joy"), "{own}");
        assert!(!own.contains("given up on"), "{own}");
        assert!(
            !own.contains("nebula"),
            "a shelf's line is not the work's: {own}"
        );

        let shelved = sheet(&conn, &profile_id, &made.id, Some("space")).unwrap();
        assert!(shelved.contains("the shelf “Space”") && shelved.contains("- nebula"));
        assert_eq!(
            sheet(&conn, &profile_id, &made.id, Some("kitchen")).unwrap(),
            "(there is no shelf called “kitchen”)"
        );
        let other = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Other".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        assert_eq!(
            sheet(&conn, &profile_id, &other.id, None).unwrap(),
            "(no lines are kept for this work)"
        );
    }

    #[test]
    fn a_phrase_is_found_by_its_words_whatever_their_case() {
        let (conn, profile_id) = fixtures::workspace();
        let made = phrase(&conn, &profile_id, "Арзамас");

        assert_eq!(
            find(&conn, &profile_id, " арзамас ").unwrap().map(|n| n.id),
            Some(made.id)
        );
        assert!(find(&conn, &profile_id, "Саров").unwrap().is_none());
        assert!(find(&conn, &profile_id, "  ").unwrap().is_none());
    }
}
