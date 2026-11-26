//! The words of a song with the second each is sung at (v0.168).
//!
//! The words come from the song's text; the times come from a word alignment
//! of its recording, laid beside the song as `*.alignment.json`. The
//! alignment hears the song its own way - syllables instead of words,
//! misheard words, lines broken elsewhere - so the two are matched letter by
//! letter, and every word of the song takes the time of the letters it
//! landed on. What a short shows under its picture is cut from these lines by
//! whoever builds it; nothing here opens a recording.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// The ending a file of timings is known by.
pub const ENDING: &str = ".alignment.json";

/// One thing the alignment heard: a word or a piece of one, and when.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Heard {
    pub word: String,
    pub start_s: f64,
    pub end_s: f64,
}

/// A file of timings.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Alignment {
    pub words: Vec<Heard>,
    /// The recording the times are of, as a file beside this one, when it is
    /// said.
    pub audio: Option<String>,
    /// How many seconds of that recording come before the first second of
    /// the video a short is cut from, when it is said rather than measured.
    pub offset: Option<f64>,
    /// Words shown otherwise than the text writes them: the text's word, and
    /// the word to show.
    pub replace: BTreeMap<String, String>,
}

/// One word of the song and when it is sung, in seconds of the recording.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Timed {
    pub word: String,
    pub at: f64,
    pub end: f64,
}

/// The letters of a text, as two hearings of it can be compared: lower case,
/// no marks, `ё` as `е`.
fn letters(text: &str) -> Vec<char> {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| if c == 'ё' { 'е' } else { c })
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// A word as it is shown: the capital vowel that marks the stress for the
/// singer goes back to lower case, a long dash becomes a hyphen.
fn plain(word: &str) -> String {
    let lettered: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
    let shouted = lettered.len() > 1 && lettered.iter().all(|c| !c.is_lowercase());
    let mut out = String::new();
    let mut first = true;
    for c in word.chars() {
        if c.is_alphabetic() && !shouted && word.chars().count() > 1 {
            if first {
                out.push(c);
            } else {
                out.extend(c.to_lowercase());
            }
            first = false;
        } else {
            out.push(match c {
                '—' | '–' => '-',
                other => other,
            });
        }
    }
    out
}

/// The sung lines of a text: no section tags, no backing lines in brackets.
pub fn song_lines(text: &str, replace: &BTreeMap<String, String>) -> Vec<Vec<String>> {
    let mut lines = Vec::new();
    for raw in text.lines() {
        let raw = raw.trim();
        if raw.is_empty() || raw.starts_with('[') || (raw.starts_with('(') && raw.ends_with(')')) {
            continue;
        }
        let mut words: Vec<String> = Vec::new();
        for token in raw.split_whitespace() {
            let token = plain(token);
            let token = replace.get(&token).cloned().unwrap_or(token);
            match words.last_mut() {
                // A dash or a lone mark belongs to the word before it.
                Some(last) if letters(&token).is_empty() => {
                    last.push(' ');
                    last.push_str(&token);
                }
                _ => words.push(token),
            }
        }
        if !words.is_empty() {
            lines.push(words);
        }
    }
    lines
}

/// The stretches two sequences share, longest first and then what lies
/// either side of each: `(at in a, at in b, length)`, in order.
fn matching_blocks(a: &[char], b: &[char]) -> Vec<(usize, usize, usize)> {
    let mut places: BTreeMap<char, Vec<usize>> = BTreeMap::new();
    for (j, c) in b.iter().enumerate() {
        places.entry(*c).or_default().push(j);
    }
    // How long the match ending at each place of `b` runs, for the letter of
    // `a` before this one and for this one.
    let mut before = vec![0usize; b.len() + 1];
    let mut now = vec![0usize; b.len() + 1];
    let mut touched_before: Vec<usize> = Vec::new();
    let mut touched_now: Vec<usize> = Vec::new();
    let mut longest = |alo: usize, ahi: usize, blo: usize, bhi: usize| {
        let (mut best_i, mut best_j, mut best) = (alo, blo, 0usize);
        for (i, word) in a.iter().enumerate().take(ahi).skip(alo) {
            if let Some(found) = places.get(word) {
                for &j in found {
                    if j < blo {
                        continue;
                    }
                    if j >= bhi {
                        break;
                    }
                    let run = before[j] + 1;
                    now[j + 1] = run;
                    touched_now.push(j + 1);
                    if run > best {
                        best_i = i + 1 - run;
                        best_j = j + 1 - run;
                        best = run;
                    }
                }
            }
            for at in touched_before.drain(..) {
                before[at] = 0;
            }
            std::mem::swap(&mut before, &mut now);
            std::mem::swap(&mut touched_before, &mut touched_now);
        }
        for at in touched_before.drain(..) {
            before[at] = 0;
        }
        (best_i, best_j, best)
    };
    let mut queue = vec![(0, a.len(), 0, b.len())];
    let mut blocks = Vec::new();
    while let Some((alo, ahi, blo, bhi)) = queue.pop() {
        let (i, j, size) = longest(alo, ahi, blo, bhi);
        if size == 0 {
            continue;
        }
        blocks.push((i, j, size));
        if alo < i && blo < j {
            queue.push((alo, i, blo, j));
        }
        if i + size < ahi && j + size < bhi {
            queue.push((i + size, ahi, j + size, bhi));
        }
    }
    blocks.sort_unstable();
    blocks
}

/// A word of the text and the span it was heard in, when it was heard.
type Spanned = (String, Option<(f64, f64)>);

/// Every word of `lines` with when it is sung, by what `heard` heard.
pub fn timed(lines: &[Vec<String>], heard: &[Heard]) -> Vec<Vec<Timed>> {
    // Each letter the alignment heard, with its share of its token's time.
    let mut heard_letters: Vec<char> = Vec::new();
    let mut when: Vec<(f64, f64)> = Vec::new();
    for token in heard {
        let chars = letters(&token.word);
        let span = token.end_s - token.start_s;
        let count = chars.len() as f64;
        for (i, c) in chars.into_iter().enumerate() {
            when.push((
                token.start_s + span * i as f64 / count,
                token.start_s + span * (i + 1) as f64 / count,
            ));
            heard_letters.push(c);
        }
    }
    // Each letter of the song, with the word it belongs to.
    let mut sung: Vec<char> = Vec::new();
    let mut owner: Vec<(usize, usize)> = Vec::new();
    for (li, words) in lines.iter().enumerate() {
        for (wi, word) in words.iter().enumerate() {
            for c in letters(word) {
                sung.push(c);
                owner.push((li, wi));
            }
        }
    }
    let mut spans: BTreeMap<(usize, usize), (f64, f64)> = BTreeMap::new();
    let mut found = vec![0usize; lines.len()];
    for (at_sung, at_heard, size) in matching_blocks(&sung, &heard_letters) {
        for k in 0..size {
            let key = owner[at_sung + k];
            let (start, end) = when[at_heard + k];
            let span = spans.entry(key).or_insert((start, end));
            span.0 = span.0.min(start);
            span.1 = span.1.max(end);
            found[key.0] += 1;
        }
    }
    let mut out: Vec<Vec<Spanned>> = lines
        .iter()
        .enumerate()
        .map(|(li, words)| {
            words
                .iter()
                .enumerate()
                .map(|(wi, word)| (word.clone(), spans.get(&(li, wi)).copied()))
                .collect()
        })
        .collect();
    let size_of = |row: &[(String, Option<(f64, f64)>)]| -> usize {
        row.iter().map(|(word, _)| letters(word).len()).sum()
    };
    // A line is heard when at least half its letters were found.
    let well: Vec<bool> = out
        .iter()
        .enumerate()
        .map(|(li, row)| found[li] * 2 >= size_of(row))
        .collect();

    // A line the alignment misheard has no word times worth keeping: its
    // words are spread, by their length, over what was heard between the
    // lines around it.
    for li in 0..out.len() {
        if well[li] {
            continue;
        }
        let before = out[..li]
            .iter()
            .enumerate()
            .rev()
            .filter(|(at, _)| well[*at])
            .find_map(|(_, row)| row.iter().rev().find_map(|(_, span)| span.map(|s| s.1)));
        let after = out[li + 1..]
            .iter()
            .enumerate()
            .filter(|(at, _)| well[li + 1 + at])
            .find_map(|(_, row)| row.iter().find_map(|(_, span)| span.map(|s| s.0)));
        let (Some(before), Some(after)) = (before, after) else {
            continue;
        };
        let inside: Vec<&Heard> = heard
            .iter()
            .filter(|token| {
                token.start_s >= before - 0.01
                    && token.end_s <= after + 0.01
                    && !letters(&token.word).is_empty()
            })
            .collect();
        let (Some(first), Some(last)) = (inside.first(), inside.last()) else {
            continue;
        };
        let (start, end) = (first.start_s, last.end_s);
        let size = size_of(&out[li]).max(1) as f64;
        let mut at = start;
        for (word, span) in &mut out[li] {
            let share = (end - start) * letters(word).len() as f64 / size;
            *span = Some((at, at + share));
            at += share;
        }
    }
    // The opening words the alignment placed long before the rest of their
    // line are pulled up to it; a gap near the end of a line is left alone -
    // that is a held pause.
    for row in &mut out {
        let half = row.len().saturating_sub(1) / 2;
        for i in (0..half).rev() {
            if let (Some(this), Some(next)) = (row[i].1, row[i + 1].1)
                && next.0 - this.0 > 3.0
            {
                row[i].1 = Some((next.0 - 0.45, next.0));
            }
        }
    }
    // A word the alignment never heard starts where its neighbour ends.
    let mut last_end = 0.0;
    out.into_iter()
        .map(|row| {
            row.into_iter()
                .map(|(word, span)| {
                    let (at, end) = span.unwrap_or((last_end, last_end + 0.25));
                    last_end = end;
                    Timed { word, at, end }
                })
                .collect()
        })
        .collect()
}

/// The newest file of timings in the first of `folders` that holds one.
pub fn find(folders: &[PathBuf]) -> Option<PathBuf> {
    folders.iter().find_map(|folder| {
        std::fs::read_dir(folder)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().to_lowercase().ends_with(ENDING))
            })
            .max_by_key(|path| {
                std::fs::metadata(path)
                    .and_then(|metadata| metadata.modified())
                    .ok()
            })
    })
}

/// Read a file of timings.
pub fn read(path: &Path) -> Result<Alignment> {
    let unreadable = |cause: String| {
        Error::refused("short.timingsUnreadable")
            .param("file", path.to_string_lossy())
            .param("cause", cause)
    };
    let text = std::fs::read_to_string(path).map_err(|cause| unreadable(cause.to_string()))?;
    serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|cause| unreadable(cause.to_string()))
}

/// The recording a file of timings is of: the one it names, or else the
/// newest recording beside it. None when there is none - the times are then
/// taken to be of the sound of the video itself.
pub fn recording(file: &Path, alignment: &Alignment) -> Option<PathBuf> {
    let folder = file.parent()?;
    if let Some(named) = alignment
        .audio
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        let path = folder.join(named);
        return path.is_file().then_some(path);
    }
    std::fs::read_dir(folder)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .map(|ending| ending.to_string_lossy().to_lowercase())
                    .is_some_and(|ending| {
                        matches!(ending.as_str(), "wav" | "flac" | "mp3" | "m4a" | "aiff")
                    })
        })
        .max_by_key(|path| {
            std::fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heard(tokens: &[(&str, f64, f64)]) -> Vec<Heard> {
        tokens
            .iter()
            .map(|(word, start_s, end_s)| Heard {
                word: (*word).to_owned(),
                start_s: *start_s,
                end_s: *end_s,
            })
            .collect()
    }

    /// Section tags and backing lines are not sung lines; a stress mark goes
    /// back to lower case, a lone dash joins the word before it, and a word
    /// the timings ask to show otherwise is shown so.
    #[test]
    fn the_sung_lines_of_a_text() {
        let text = "[Verse 1]\nThe tIde — comes in\n(ooh, ooh)\n\nNASA said: «AgAin»\n";
        let replace = BTreeMap::from([("Again»".to_owned(), "again»".to_owned())]);

        let lines = song_lines(text, &replace);

        assert_eq!(
            lines,
            vec![
                vec!["The", "tide -", "comes", "in"],
                vec!["NASA", "said:", "«Again»"],
            ]
        );
        assert_eq!(
            song_lines(
                "«AgAin»",
                &BTreeMap::from([("«Again»".to_owned(), "again".to_owned())])
            ),
            vec![vec!["again"]]
        );
    }

    #[test]
    fn the_longest_shared_stretches_are_found_in_order() {
        let a: Vec<char> = "the tide comes in".chars().collect();
        let b: Vec<char> = "a tide came in".chars().collect();
        let blocks = matching_blocks(&a, &b);
        let shared: Vec<String> = blocks
            .iter()
            .map(|(i, _, size)| a[*i..*i + *size].iter().collect())
            .collect();
        assert_eq!(shared, vec![" tide c", "me", " in"]);
        assert!(matching_blocks(&a, &[]).is_empty());
    }

    /// Words heard as syllables take the time of their letters; a word never
    /// heard starts where its neighbour ends.
    #[test]
    fn every_word_takes_the_time_of_its_letters() {
        let lines = vec![
            vec!["The".to_owned(), "harbour".to_owned(), "lights".to_owned()],
            vec!["at".to_owned(), "dawn".to_owned()],
        ];
        let tokens = heard(&[
            ("The", 1.0, 1.3),
            (" har", 1.3, 1.6),
            ("bour", 1.6, 2.0),
            (" lights", 2.0, 2.6),
            (" dawn", 4.0, 4.8),
        ]);

        let rows = timed(&lines, &tokens);

        let sung = |li: usize, wi: usize, at: f64, end: f64| {
            let word = &rows[li][wi];
            assert!(
                (word.at - at).abs() < 1e-9 && (word.end - end).abs() < 1e-9,
                "{word:?}, not {at}..{end}"
            );
        };
        sung(0, 0, 1.0, 1.3);
        // Two syllables, one word.
        sung(0, 1, 1.3, 2.0);
        sung(0, 2, 2.0, 2.6);
        // Never heard: after its neighbour.
        sung(1, 0, 2.6, 2.85);
        sung(1, 1, 4.0, 4.8);
    }

    /// A line the alignment misheard is spread over what was heard between
    /// the lines around it.
    #[test]
    fn a_misheard_line_is_spread_between_its_neighbours() {
        let lines = vec![
            vec!["first".to_owned(), "line".to_owned()],
            vec!["quite".to_owned(), "another".to_owned()],
            vec!["third".to_owned(), "line".to_owned()],
        ];
        let tokens = heard(&[
            ("first", 1.0, 1.5),
            (" line", 1.5, 2.0),
            (" zzzz", 3.0, 4.0),
            (" xxxxxxxx", 4.0, 6.0),
            (" third", 7.0, 7.5),
            (" line", 7.5, 8.0),
        ]);

        let rows = timed(&lines, &tokens);

        assert!((rows[1][0].at - 3.0).abs() < 1e-9 && (rows[1][1].end - 6.0).abs() < 1e-9);
        assert!(
            (rows[1][0].end - 3.0 - 3.0 * 5.0 / 12.0).abs() < 1e-9,
            "by its length"
        );
        assert!((rows[2][0].at - 7.0).abs() < 1e-9);
    }

    /// The newest file of timings in the first folder that has one, and the
    /// recording it names or the one beside it.
    #[test]
    fn the_timings_are_found_beside_the_song() {
        let dir = tempfile::tempdir().unwrap();
        let (empty, song) = (dir.path().join("empty"), dir.path().join("song"));
        std::fs::create_dir_all(&empty).unwrap();
        std::fs::create_dir_all(&song).unwrap();
        assert_eq!(find(&[empty.clone(), song.clone()]), None);

        let file = song.join("Tide.Alignment.JSON");
        std::fs::write(&file, "\u{feff}{\"words\": [{\"word\": \"tide\", \"start_s\": 1, \"end_s\": 2, \"success\": true}]}").unwrap();
        std::fs::write(song.join("notes.json"), "{}").unwrap();
        assert_eq!(find(&[empty, song.clone()]), Some(file.clone()));

        let alignment = read(&file).unwrap();
        assert_eq!(alignment.words.len(), 1);
        assert_eq!(recording(&file, &alignment), None);
        std::fs::write(song.join("take.wav"), b"x").unwrap();
        assert_eq!(recording(&file, &alignment), Some(song.join("take.wav")));
        let named = Alignment {
            audio: Some("missing.wav".into()),
            ..alignment
        };
        assert_eq!(recording(&file, &named), None, "the one it names, or none");

        std::fs::write(&file, "not json").unwrap();
        assert!(read(&file).is_err());
    }
}
