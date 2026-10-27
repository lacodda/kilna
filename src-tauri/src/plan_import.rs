//! A release plan read from a CSV file (v0.104).
//!
//! People keep plans in spreadsheets - a month of dates, a work on each, the
//! platform it goes out on - and copying one into the calendar by hand is a
//! drag per row. This reads such a file into the same thing the auto-layout
//! proposes: placements, each a release waiting in the queue on a day. The
//! calendar draws them as the layout's ghosts and books them with the
//! layout's one gesture (`layout::apply`), so an import is looked at before
//! it lands, lands whole or not at all, and is taken back with one undo.
//!
//! A row that cannot be placed is said, with its line and why, rather than
//! guessed at: a work the file names that the workspace has twice, a work
//! with nothing waiting to go out, a day already taken or closed. The plan
//! moves nothing that is already on the calendar and makes no releases; a
//! work with nothing waiting is made ready for a door first, on its card.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::layout::Placement;
use crate::profile::config::Label;
use crate::release::{self, Verdict};

/// Why a row of the file was left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum ImportReason {
    /// The first cell is not a day: `2026-11-03` or `03.11.2026`.
    BadDate,
    /// No work of this workspace has that title or id.
    NoWork,
    /// More than one work has that title; the file must name it by id.
    AmbiguousWork,
    /// The work has no release waiting in the queue - for that door, when
    /// the row names one.
    NothingWaiting,
    /// The work waits to go out on more than one door, and the row names none.
    AmbiguousDoor,
    /// Another release already holds the day, on the calendar or earlier in
    /// the file. The layout's rule: a person adds a second by hand.
    TakenDay,
    /// The day is closed (time off).
    ClosedDay,
    /// The release is placed by an earlier row already.
    Repeated,
}

/// A row left out, and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct ImportProblem {
    /// The line of the file, from 1.
    pub line: u32,
    /// The row as written, cells joined back with a comma.
    pub text: String,
    pub reason: ImportReason,
}

/// What the file says, as the calendar can book it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct ImportedPlan {
    /// In date order, as the auto-layout's are.
    pub placements: Vec<Placement>,
    pub problems: Vec<ImportProblem>,
}

/// Read `csv` against the workspace `profile_id` holds.
///
/// Cells are split on commas, or on semicolons when the first line has some
/// and no commas - a spreadsheet in a language that writes decimals with a
/// comma saves that way - and may be quoted. The columns are a day, a work
/// and, optionally, a door; a first row whose first cell is not a day is a
/// header, and names the columns when it says `date`/`дата`,
/// `work`/`title`/`работа`/`название` and `door`/`platform`/`площадка`.
pub fn read(conn: &Connection, profile_id: &str, csv: &str) -> Result<ImportedPlan> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let rows = rows_of(csv);
    let mut columns = Columns::default();
    let mut body = rows.as_slice();
    if let Some((_, first)) = rows.first()
        && first.first().and_then(|cell| day_of(cell)).is_none()
    {
        columns = Columns::from_header(first);
        body = &rows[1..];
    }

    // Every work by id and by title, and what waits for each in the queue.
    let works = crate::work::list(conn, profile_id, &crate::work::WorkFilter::default())?;
    let mut by_title: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let ids: BTreeSet<String> = works.iter().map(|work| work.id.clone()).collect();
    for work in &works {
        by_title
            .entry(fold(&work.title))
            .or_default()
            .push(work.id.clone());
    }
    let mut waiting: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for entry in release::queue(conn, profile_id)? {
        waiting
            .entry(entry.release.work_id.clone())
            .or_default()
            .push((entry.release.id.clone(), entry.release.kind.clone()));
    }
    let doors = config.all_release_kinds();

    let mut placements: Vec<Placement> = Vec::new();
    let mut days_used: BTreeSet<String> = BTreeSet::new();
    let mut placed: BTreeSet<String> = BTreeSet::new();
    let mut problems = Vec::new();
    for (line, cells) in body {
        let problem = |reason| ImportProblem {
            line: *line,
            text: cells.join(","),
            reason,
        };
        let cell = |index: usize| cells.get(index).map(|cell| cell.trim()).unwrap_or("");
        let Some(day) = day_of(cell(columns.date)) else {
            problems.push(problem(ImportReason::BadDate));
            continue;
        };
        let named = cell(columns.work);
        let work_id = if ids.contains(named) {
            named.to_owned()
        } else {
            match by_title.get(&fold(named)).map(Vec::as_slice) {
                Some([one]) => one.clone(),
                Some([_, _, ..]) => {
                    problems.push(problem(ImportReason::AmbiguousWork));
                    continue;
                }
                _ => {
                    problems.push(problem(ImportReason::NoWork));
                    continue;
                }
            }
        };
        let door = columns.door.map(cell).unwrap_or("");
        let candidates: Vec<&(String, String)> = waiting
            .get(&work_id)
            .into_iter()
            .flatten()
            .filter(|(id, _)| !placed.contains(id))
            .filter(|(_, kind)| {
                door.is_empty()
                    || kind.eq_ignore_ascii_case(door)
                    || doors
                        .iter()
                        .filter(|known| &known.key == kind)
                        .any(|known| {
                            names(&known.label)
                                .iter()
                                .any(|name| fold(name) == fold(door))
                        })
            })
            .collect();
        let release_id = match candidates.as_slice() {
            [(id, _)] => id.clone(),
            [] => {
                let all_placed = waiting.get(&work_id).is_some_and(|all| {
                    !all.is_empty() && all.iter().all(|(id, _)| placed.contains(id))
                });
                problems.push(problem(if all_placed {
                    ImportReason::Repeated
                } else {
                    ImportReason::NothingWaiting
                }));
                continue;
            }
            _ => {
                problems.push(problem(ImportReason::AmbiguousDoor));
                continue;
            }
        };
        let slot = release::preview(conn, &release_id, &day)?;
        if slot.closed.is_some() {
            problems.push(problem(ImportReason::ClosedDay));
            continue;
        }
        if slot.verdict != Verdict::Empty || days_used.contains(&day) {
            problems.push(problem(ImportReason::TakenDay));
            continue;
        }
        days_used.insert(day.clone());
        placed.insert(release_id.clone());
        placements.push(Placement {
            release_id,
            date: day,
        });
    }
    placements.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(ImportedPlan {
        placements,
        problems,
    })
}

/// A file's bytes as text: UTF-8, or else Windows-1251 - what a spreadsheet
/// in a Russian locale saves a CSV as.
pub fn text_of(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => bytes.iter().map(|&byte| cp1251(byte)).collect(),
    }
}

/// One byte of Windows-1251: the letters, and the punctuation a table cell
/// is likely to hold; anything else is the replacement character.
fn cp1251(byte: u8) -> char {
    match byte {
        0x00..=0x7F => char::from(byte),
        0xC0..=0xFF => char::from_u32(0x0410 + u32::from(byte - 0xC0)).unwrap_or('\u{fffd}'),
        0xA8 => 'Ё',
        0xB8 => 'ё',
        0xA0 => '\u{a0}',
        0x85 => '…',
        0x93 => '“',
        0x94 => '”',
        0x96 => '–',
        0x97 => '—',
        0xAB => '«',
        0xBB => '»',
        0xB9 => '№',
        _ => '\u{fffd}',
    }
}

/// Which cell holds what.
#[derive(Debug, Clone, Copy)]
struct Columns {
    date: usize,
    work: usize,
    door: Option<usize>,
}

impl Default for Columns {
    fn default() -> Self {
        Columns {
            date: 0,
            work: 1,
            door: Some(2),
        }
    }
}

impl Columns {
    /// The columns a header names; what it does not name keeps its place.
    fn from_header(cells: &[String]) -> Columns {
        let find = |words: &[&str]| {
            cells
                .iter()
                .position(|cell| words.contains(&fold(cell).as_str()))
        };
        let defaults = Columns::default();
        Columns {
            date: find(&["date", "day", "дата", "день"]).unwrap_or(defaults.date),
            work: find(&["work", "title", "работа", "название", "произведение"])
                .unwrap_or(defaults.work),
            door: find(&["door", "platform", "kind", "дверь", "площадка"]).or(defaults.door),
        }
    }
}

/// The file's non-empty rows, each with its line, split into cells.
fn rows_of(csv: &str) -> Vec<(u32, Vec<String>)> {
    let text = csv.trim_start_matches('\u{feff}');
    let first = text.lines().next().unwrap_or("");
    let separator = if first.contains(';') && !first.contains(',') {
        ';'
    } else if first.contains('\t') && !first.contains(',') {
        '\t'
    } else {
        ','
    };
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| (index as u32 + 1, cells_of(line, separator)))
        .collect()
}

/// One line's cells; a cell in double quotes may hold the separator, and a
/// doubled quote inside it is one quote.
fn cells_of(line: &str, separator: char) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            c if c == separator && !quoted => cells.push(std::mem::take(&mut cell)),
            c => cell.push(c),
        }
    }
    cells.push(cell);
    cells
}

/// A day as the file may write it - `2026-11-03` or `03.11.2026` - as ISO.
fn day_of(cell: &str) -> Option<String> {
    let cell = cell.trim();
    let (year, month, day) = if let Some((year, rest)) = cell.split_once('-') {
        let (month, day) = rest.split_once('-')?;
        (year, month, day)
    } else {
        let mut parts = cell.split('.');
        let (day, month, year) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() {
            return None;
        }
        (year, month, day)
    };
    let (year, month, day): (i32, u8, u8) =
        (year.parse().ok()?, month.parse().ok()?, day.parse().ok()?);
    let month = time::Month::try_from(month).ok()?;
    let date = time::Date::from_calendar_date(year, month, day).ok()?;
    Some(format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    ))
}

/// A name as rows are matched by: trimmed, lower case.
fn fold(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Every word a label is said by, in every language it carries.
fn names(label: &Label) -> Vec<String> {
    match label {
        Label::One(word) => vec![word.clone()],
        Label::PerLocale(words) => words.values().cloned().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    /// A workspace with two songs whose audio releases wait in the queue.
    fn a_queue() -> (Connection, String, String, String) {
        let (conn, profile_id) = fixtures::workspace();
        let mut waiting = Vec::new();
        for title in ["Tide", "Harbour"] {
            let song = fixtures::work(&conn, &profile_id, "song", title);
            let audio =
                crate::actions::work::derive(&conn, &song.id, "audio", None, None, None).unwrap();
            let release = release::for_work(&conn, &profile_id, &audio.work.id)
                .unwrap()
                .into_iter()
                .next()
                .expect("a publication is made with its release")
                .release;
            waiting.push((audio.work.title, release.id));
        }
        let [(_, tide), (_, harbour)] = <[_; 2]>::try_from(waiting).unwrap();
        (conn, profile_id, tide, harbour)
    }

    /// Rows become placements in date order, whatever the order of the file,
    /// with a header or without, in either date format and either separator.
    #[test]
    fn a_plan_is_read_into_placements() {
        let (conn, profile_id, tide, harbour) = a_queue();

        let plan = read(
            &conn,
            &profile_id,
            "Дата;Работа;Площадка\n10.11.2026;Harbour (audio);\n\"2026-11-03\";tide (audio);\n",
        )
        .unwrap();

        assert_eq!(plan.problems, vec![]);
        assert_eq!(
            plan.placements,
            vec![
                Placement {
                    release_id: tide,
                    date: "2026-11-03".into()
                },
                Placement {
                    release_id: harbour,
                    date: "2026-11-10".into()
                },
            ]
        );
    }

    /// What cannot be placed is said with its line and why, and the rest is
    /// placed.
    #[test]
    fn a_row_that_cannot_be_placed_says_why() {
        let (conn, profile_id, tide, _) = a_queue();

        let plan = read(
            &conn,
            &profile_id,
            "2026-11-03,Tide (audio)\n\
             2026-11-31,Harbour (audio)\n\
             2026-11-05,Nobody's song\n\
             2026-11-06,Tide (audio)\n\
             2026-11-03,Harbour (audio)\n",
        )
        .unwrap();

        assert_eq!(
            plan.placements,
            vec![Placement {
                release_id: tide,
                date: "2026-11-03".into()
            }]
        );
        let said: Vec<(u32, ImportReason)> = plan
            .problems
            .iter()
            .map(|problem| (problem.line, problem.reason))
            .collect();
        assert_eq!(
            said,
            vec![
                (2, ImportReason::BadDate),
                (3, ImportReason::NoWork),
                (4, ImportReason::Repeated),
                (5, ImportReason::TakenDay),
            ]
        );
    }

    #[test]
    fn a_file_saved_in_windows_1251_reads() {
        assert_eq!(text_of(&[0xC4, 0xE0, 0xF2, 0xE0, b';', 0xA8]), "Дата;Ё");
        assert_eq!(text_of("Дата".as_bytes()), "Дата");
    }

    #[test]
    fn cells_are_read_as_a_spreadsheet_writes_them() {
        assert_eq!(
            cells_of("2026-11-03,\"Tide, the \"\"long\"\" one\",youtube", ','),
            vec!["2026-11-03", "Tide, the \"long\" one", "youtube"]
        );
        assert_eq!(day_of("03.11.2026").as_deref(), Some("2026-11-03"));
        assert_eq!(day_of("2026-02-30"), None);
        assert_eq!(day_of("Nov 3"), None);
    }
}
