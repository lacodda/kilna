//! A channel's archive from before its plan (v0.164): the list a previous
//! system kept of every track that went out - `<Channel> - All Tracks.md`,
//! one numbered line a track, `N. [x] Title (Formula) (DD.MM.YYYY)` - and
//! the folder its files are numbered in, `Images/`, `WAV/`, `mp3/`, `mp4/`,
//! each file `NNN - Title (Formula).<ext>`. Read by shape, never for meaning.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// One track of the archive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArchiveTrack {
    /// Its number in the list - and in its files' names.
    pub number: u32,
    pub title: String,
    /// What the brackets after the title say: `Indian Chillwave`,
    /// `Ossetian x Industrial Techno`.
    pub formula: Option<String>,
    /// The day it went out, `YYYY-MM-DD`.
    pub date: String,
    /// Its files, each with whether it is the track's picture.
    pub files: Vec<(String, bool)>,
}

/// The subfolders an archive keeps its files in, and whether each holds
/// the tracks' pictures.
const FOLDERS: [(&str, bool); 4] = [
    ("Images", true),
    ("WAV", false),
    ("mp3", false),
    ("mp4", false),
];

/// The folder the list names for its files: the line `Files: \`<dir>\``.
pub fn folder_named(text: &str) -> Option<PathBuf> {
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("Files:")?;
        let start = rest.find('`')? + 1;
        let end = rest[start..].find('`')? + start;
        Some(PathBuf::from(rest[start..end].trim()))
    })
}

/// The tracks of a list, in its order; a line of any other shape is passed
/// over.
pub fn tracks(text: &str) -> Vec<ArchiveTrack> {
    text.lines().filter_map(track).collect()
}

/// One line: `N. [x] Title (Formula) (DD.MM.YYYY)`, with anything after the
/// date's bracket - a note of what is missing - left out.
fn track(line: &str) -> Option<ArchiveTrack> {
    let line = line.trim();
    let (number, rest) = line.split_once(". ")?;
    let number: u32 = number.trim().parse().ok()?;
    let rest = rest.trim_start();
    let rest = rest
        .strip_prefix("[x]")
        .or_else(|| rest.strip_prefix("[X]"))
        .or_else(|| rest.strip_prefix("[ ]"))?
        .trim();
    // The date is the last bracket that holds one.
    let (head, date) = last_bracket(rest, |inner| day_of(inner).is_some())?;
    let date = day_of(&date)?;
    let head = head.trim();
    let (title, formula) = match last_bracket(head, |_| true) {
        Some((title, formula)) if !title.trim().is_empty() && head.ends_with(')') => {
            (title.trim().to_owned(), Some(formula.trim().to_owned()))
        }
        _ => (head.to_owned(), None),
    };
    (!title.is_empty()).then_some(ArchiveTrack {
        number,
        title,
        formula: formula.filter(|formula| !formula.is_empty()),
        date,
        files: Vec::new(),
    })
}

/// The last `(...)` of `text` whose words pass `fits`, and what stands
/// before it; a bracket may hold brackets of its own.
fn last_bracket(text: &str, fits: impl Fn(&str) -> bool) -> Option<(String, String)> {
    let mut depth = 0usize;
    let mut close = None;
    for (at, c) in text.char_indices().rev() {
        match c {
            ')' => {
                if depth == 0 {
                    close = Some(at);
                }
                depth += 1;
            }
            '(' if depth > 0 => {
                depth -= 1;
                if depth == 0
                    && let Some(end) = close
                {
                    let inner = &text[at + 1..end];
                    if fits(inner) {
                        return Some((text[..at].to_owned(), inner.to_owned()));
                    }
                    close = None;
                }
            }
            _ => {}
        }
    }
    None
}

/// `DD.MM.YYYY` as `YYYY-MM-DD`.
fn day_of(text: &str) -> Option<String> {
    let mut parts = text.trim().split('.');
    let (day, month, year) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some()
        || day.len() != 2
        || month.len() != 2
        || year.len() != 4
        || ![day, month, year]
            .iter()
            .all(|part| part.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let date = format!("{year}-{month}-{day}");
    crate::time::is_date(&date).then_some(date)
}

/// Each track's files, found in `folder` by the number they begin with.
pub fn find_files(tracks: &mut [ArchiveTrack], folder: &Path) {
    for (sub, picture) in FOLDERS {
        let Ok(entries) = std::fs::read_dir(folder.join(sub)) else {
            continue;
        };
        let mut found: Vec<(u32, PathBuf)> = entries
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter_map(|path| {
                let name = path.file_name()?.to_string_lossy().into_owned();
                let (number, _) = name.split_once(" - ")?;
                Some((number.trim().parse().ok()?, path))
            })
            .collect();
        found.sort();
        for (number, path) in found {
            if let Some(track) = tracks.iter_mut().find(|track| track.number == number) {
                track
                    .files
                    .push((path.to_string_lossy().into_owned(), picture));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "# Demo - All Tracks

Total: 3 tracks | 19.09.2025 - 21.09.2025

Files: `C:\\YouTube\\Demo\\_All\\` -> `Images\\`, `mp3\\`, numbered to match this list.

1. [x] Sitar Skies (Indian Chillwave) (19.09.2025)
2. [ ] Ignition (Big Room EDM) (22.09.2025) - no mp4, no image
3. [x] Waltz (for Two) (Vienna x Lo-Fi) (21.09.2025)
4. [x] No Formula (23.09.2025)
not a track
";

    #[test]
    fn the_list_is_read_by_its_shape() {
        assert_eq!(
            folder_named(LIST),
            Some(PathBuf::from("C:\\YouTube\\Demo\\_All\\"))
        );
        let read = tracks(LIST);
        assert_eq!(read.len(), 4);
        assert_eq!(
            (
                read[0].number,
                read[0].title.as_str(),
                read[0].formula.as_deref(),
                read[0].date.as_str()
            ),
            (1, "Sitar Skies", Some("Indian Chillwave"), "2025-09-19")
        );
        assert_eq!(
            read[1].date, "2025-09-22",
            "a note after the date is left out"
        );
        assert_eq!(read[2].title, "Waltz (for Two)");
        assert_eq!(read[2].formula.as_deref(), Some("Vienna x Lo-Fi"));
        assert_eq!(
            (read[3].title.as_str(), read[3].formula.as_deref()),
            ("No Formula", None)
        );
    }

    #[test]
    fn a_tracks_files_are_found_by_their_number() {
        let dir = tempfile::tempdir().unwrap();
        for (sub, name) in [
            ("Images", "001 - Sitar Skies (Indian Chillwave).png"),
            ("mp3", "001 - Sitar Skies (Indian Chillwave).mp3"),
            ("mp3", "003 - Waltz (for Two) (Vienna x Lo-Fi).mp3"),
            ("mp3", "notes.txt"),
        ] {
            std::fs::create_dir_all(dir.path().join(sub)).unwrap();
            std::fs::write(dir.path().join(sub).join(name), b"x").unwrap();
        }
        let mut read = tracks(LIST);
        find_files(&mut read, dir.path());
        assert_eq!(read[0].files.len(), 2);
        assert!(
            read[0]
                .files
                .iter()
                .any(|(path, picture)| *picture && path.ends_with(".png"))
        );
        assert!(read[1].files.is_empty());
        assert_eq!(read[2].files.len(), 1);
    }
}
