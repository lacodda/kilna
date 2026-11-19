//! A batch of files told apart by the works they belong to (v0.157, FR-21
//! 21.2 p.2): takes and covers dropped in a heap, each matched to a work
//! without guessing - by the work's id in its name, by its title, by the
//! first words of its cover's prompt that an image generator names its
//! file after, by the title written into a track's tags.
//!
//! Nothing here moves a file: what is matched is shown in a table the
//! person reads and corrects, and only what they confirm is taken in, by
//! the action every file goes in through (`actions::media::take`). A
//! picture named by a moment and nothing else - `202604112318.png` - is
//! left unmatched for the person or the assistant to place.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;

use crate::actions::media::MediaRole;
use crate::error::Result;
use crate::work::Work;

/// How a file was matched to its work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum MatchedBy {
    /// The work's id, whole or its first eight characters, is in the name.
    Id,
    /// The title in a track's tags is the work's.
    Tag,
    /// The work's title is the name, or in it.
    Title,
    /// The name begins as the work's cover prompt does.
    CoverPrompt,
}

/// One file of a batch, as it was matched.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct MediaMatch {
    pub path: String,
    pub name: String,
    /// `audio`, `image`, `video` or `other`.
    pub kind: String,
    /// The work it was matched to, when one was.
    pub work_id: Option<String>,
    pub work_title: Option<String>,
    pub by: Option<MatchedBy>,
    /// How sure the match is, from 0 to 1.
    pub confidence: Option<f64>,
    /// The works it could equally be, when it matched more than one.
    pub ambiguous: Vec<String>,
    /// What it would be taken as.
    pub role: MediaRole,
    /// A second download of the same file - `take (1).mp3` - likely to be
    /// let go.
    pub duplicate: bool,
}

const AUDIO: [&str; 7] = ["mp3", "wav", "flac", "m4a", "ogg", "aac", "opus"];
const PICTURES: [&str; 5] = ["png", "jpg", "jpeg", "webp", "avif"];
const VIDEOS: [&str; 5] = ["mp4", "mov", "webm", "mkv", "avi"];

/// Words of a text as they are compared: lower case, letters and digits,
/// `ё` read as `е`, every run of anything else one space.
pub fn normalized(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut gap = false;
    for c in text.chars().flat_map(char::to_lowercase) {
        let c = if c == 'ё' { 'е' } else { c };
        if c.is_alphanumeric() {
            if gap && !out.is_empty() {
                out.push(' ');
            }
            gap = false;
            out.push(c);
        } else {
            gap = true;
        }
    }
    out
}

/// The stem of a file's name, without what a browser adds to a second
/// download of it, and whether it added that.
fn stem_of(name: &str) -> (String, bool) {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(name)
        .trim()
        .to_owned();
    // `take (1)`, `take(2)`: a number in brackets at the very end.
    if let Some(open) = stem.rfind('(')
        && stem.ends_with(')')
        && stem[open + 1..stem.len() - 1]
            .chars()
            .all(|c| c.is_ascii_digit())
        && open + 2 < stem.len()
    {
        return (stem[..open].trim_end().to_owned(), true);
    }
    (stem, false)
}

fn kind_of(name: &str) -> &'static str {
    let ext = Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if AUDIO.contains(&ext.as_str()) {
        "audio"
    } else if PICTURES.contains(&ext.as_str()) {
        "image"
    } else if VIDEOS.contains(&ext.as_str()) {
        "video"
    } else {
        "other"
    }
}

/// The title written into a track's tags, when it has one.
fn tag_title(path: &Path) -> Option<String> {
    use lofty::file::TaggedFileExt;
    use lofty::tag::Accessor;
    let tagged = lofty::read_from_path(path).ok()?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;
    tag.title().map(|title| title.into_owned())
}

/// A work as the matcher reads it.
struct Known<'a> {
    work: &'a Work,
    title: String,
    prompt: String,
}

/// The first words of a prompt an image generator names its file after.
const PROMPT_WORDS: usize = 4;

fn prompt_start(text: &str) -> String {
    normalized(text)
        .split(' ')
        .take(PROMPT_WORDS)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Match one file against the works.
fn one(path: &Path, works: &[Known<'_>]) -> (Option<(usize, MatchedBy, f64)>, Vec<usize>) {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let lower = name.to_lowercase();
    let (stem, _) = stem_of(name);
    let words = normalized(&stem);

    // The id: as sure as a match gets.
    let by_id: Vec<usize> = works
        .iter()
        .enumerate()
        .filter(|(_, known)| {
            let id = known.work.id.to_lowercase();
            lower.contains(&id)
                || id
                    .get(..8)
                    .is_some_and(|short| contains_word(&lower, short))
        })
        .map(|(index, _)| index)
        .collect();
    if let [only] = by_id[..] {
        return (Some((only, MatchedBy::Id, 1.0)), Vec::new());
    }

    if kind_of(name) == "audio"
        && let Some(title) = tag_title(path).map(|title| normalized(&title))
        && !title.is_empty()
    {
        let tagged: Vec<usize> = works
            .iter()
            .enumerate()
            .filter(|(_, known)| known.title == title)
            .map(|(index, _)| index)
            .collect();
        match tagged[..] {
            [only] => return (Some((only, MatchedBy::Tag, 0.95)), Vec::new()),
            [] => {}
            _ => return (None, tagged),
        }
    }

    // The title: the whole name, else the longest title inside it.
    let exact: Vec<usize> = works
        .iter()
        .enumerate()
        .filter(|(_, known)| !known.title.is_empty() && known.title == words)
        .map(|(index, _)| index)
        .collect();
    match exact[..] {
        [only] => return (Some((only, MatchedBy::Title, 0.9)), Vec::new()),
        [] => {}
        _ => return (None, exact),
    }
    let longest = works
        .iter()
        .filter(|known| known.title.chars().count() >= 3 && contains_word(&words, &known.title))
        .map(|known| known.title.chars().count())
        .max();
    if let Some(longest) = longest {
        let inside: Vec<usize> = works
            .iter()
            .enumerate()
            .filter(|(_, known)| {
                known.title.chars().count() == longest && contains_word(&words, &known.title)
            })
            .map(|(index, _)| index)
            .collect();
        match inside[..] {
            [only] => return (Some((only, MatchedBy::Title, 0.8)), Vec::new()),
            _ => return (None, inside),
        }
    }

    // The cover prompt: a picture named after the words it was drawn from.
    if kind_of(name) == "image" {
        let prompted: Vec<usize> = works
            .iter()
            .enumerate()
            .filter(|(_, known)| {
                known.prompt.split(' ').count() >= 2
                    && (words.starts_with(&known.prompt) || known.prompt.starts_with(&words))
                    && words.split(' ').count() >= 2
            })
            .map(|(index, _)| index)
            .collect();
        match prompted[..] {
            [only] => return (Some((only, MatchedBy::CoverPrompt, 0.7)), Vec::new()),
            [] => {}
            _ => return (None, prompted),
        }
    }
    (None, Vec::new())
}

/// Whether `needle` stands in `hay` as whole words.
fn contains_word(hay: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    hay.match_indices(needle).any(|(at, _)| {
        let before = hay[..at].chars().next_back();
        let after = hay[at + needle.len()..].chars().next();
        before.is_none_or(|c| !c.is_alphanumeric()) && after.is_none_or(|c| !c.is_alphanumeric())
    })
}

/// Match a batch of files against works. The first file of a kind matched
/// to a work is taken as what it finishes - its track, its cover, its video
/// - and the next ones of that kind as ones to choose from.
pub fn match_files(paths: &[String], works: &[Work]) -> Vec<MediaMatch> {
    let known: Vec<Known<'_>> = works
        .iter()
        .map(|work| Known {
            work,
            title: normalized(&work.title),
            prompt: prompt_start(
                work.cover
                    .sent
                    .as_ref()
                    .map_or(work.cover.scene.as_str(), |sent| sent.picture.as_str()),
            ),
        })
        .collect();
    let mut taken: HashMap<(String, &'static str), usize> = HashMap::new();
    let mut out = Vec::new();
    for path in paths {
        let file = Path::new(path);
        let name = file
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        let kind = kind_of(&name);
        let (_, duplicate) = stem_of(&name);
        let (found, ambiguous) = one(file, &known);
        let work = found.map(|(index, _, _)| known[index].work);
        // A second download takes no place: `take (1).mp3` sorted before
        // `take.mp3` must not make the original a take to choose from.
        let first = !duplicate
            && work.is_none_or(|work| {
                let count = taken.entry((work.id.clone(), kind)).or_insert(0);
                *count += 1;
                *count == 1
            });
        let role = match kind {
            "audio" if first => MediaRole::FinalAudio,
            "audio" => MediaRole::CandidateAudio,
            "image" if first => MediaRole::FinalCover,
            "image" => MediaRole::CandidateCover,
            "video" => MediaRole::Video,
            _ => MediaRole::Asset,
        };
        out.push(MediaMatch {
            path: path.clone(),
            name,
            kind: kind.to_owned(),
            work_id: work.map(|work| work.id.clone()),
            work_title: work.map(|work| work.title.clone()),
            by: found.map(|(_, by, _)| by),
            confidence: found.map(|(_, _, sure)| sure),
            ambiguous: ambiguous
                .into_iter()
                .map(|index| known[index].work.id.clone())
                .collect(),
            role,
            duplicate,
        });
    }
    out
}

/// The works a batch is matched against: a collection's, or every work of
/// the space.
pub fn candidates(
    conn: &Connection,
    profile_id: &str,
    collection_id: Option<&str>,
) -> Result<Vec<Work>> {
    crate::work::list(
        conn,
        profile_id,
        &crate::work::WorkFilter {
            collection_id: collection_id.map(str::to_owned),
            ..crate::work::WorkFilter::default()
        },
    )
}

#[cfg(test)]
mod tests;
