//! Where a short could be cut from (v0.103, ADR 0071).
//!
//! The Cuts tab keeps the stretches a person takes; this says where to look
//! before taking one. For each video a short is made from it gathers three
//! things. What a plugin heard in the video's finished file: where its parts
//! begin, for the track to mark and a dragged edge to land on, and the parts
//! worth a short. What the words say: the chorus and the sections whose lines
//! come back, placed by the seconds of the video's own scenes - kilna knows a
//! scene's section and its seconds without opening anything. And what the
//! short's siblings already took out of the same video, so a part is not
//! taken twice without knowing it.
//!
//! Nothing here is a stretch. A part becomes one when the person takes it,
//! as the ordinary cut it then is.

use std::collections::HashMap;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::donor_map::{self, Hearing};
use crate::error::Result;
use crate::profile::config::ProfileConfig;

/// How long a part offered from the words may run: past it, it is cut at the
/// last scene that still fits.
const LONGEST: f64 = 45.0;
/// How short a part from the words may be and still be offered.
const SHORTEST: f64 = 10.0;
/// How many parts the words offer for one video.
const MOST_BY_WORDS: usize = 3;

/// Where a part offered comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum PartSource {
    /// Heard in the video's file by a plugin.
    Sound,
    /// Read from the song's words and the video's scenes.
    Words,
}

/// A part of a video offered for a short.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct OfferedPart {
    pub starts_at: f64,
    pub ends_at: f64,
    /// Why: `chorus`, `loudest`, `repeats`, `lift`, `part` from the sound;
    /// `wordsChorus`, `wordsHook` from the words; a newer plugin's own key
    /// otherwise, shown as it is.
    pub reason: String,
    /// What the reason is about, in the work's own words: the section as the
    /// scenes name it and the line it opens with.
    pub detail: Option<String>,
    pub source: PartSource,
}

/// A stretch another short already took out of the same video.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct TakenStretch {
    pub work_id: String,
    pub title: String,
    pub starts_at: f64,
    pub ends_at: f64,
}

/// Whether the video was listened to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum HearingState {
    /// No plugin listened to it yet.
    Unheard,
    /// Listened to in the file it has now.
    Heard,
    /// Listened to in another file, or before this one was rendered again:
    /// nothing heard then is offered.
    Stale,
    /// It has no finished file to listen to.
    NoFile,
}

/// Everything offered for one video a short is made from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct DonorParts {
    pub source_id: String,
    pub source_title: String,
    /// The video's length in seconds, when its overview says it.
    pub duration: Option<f64>,
    pub hearing: HearingState,
    /// Seconds where its parts begin, as heard; empty unless heard.
    pub boundaries: Vec<f64>,
    /// The parts offered: from the sound first, best first, then the words.
    pub parts: Vec<OfferedPart>,
    /// What the short's siblings took out of it, in the order they run.
    pub taken: Vec<TakenStretch>,
}

/// What is offered for each video `short_id` is made from.
pub fn for_short(
    conn: &Connection,
    config: &ProfileConfig,
    short_id: &str,
) -> Result<Vec<DonorParts>> {
    let mut sources: Vec<String> = crate::link::sources(conn, short_id)?
        .into_iter()
        .filter(|link| link.role != crate::link::LAB)
        .map(|link| link.source_id)
        .collect();
    for source in crate::cut::sources_of(conn, short_id)? {
        if !sources.contains(&source) {
            sources.push(source);
        }
    }

    let mut offered = Vec::with_capacity(sources.len());
    for source_id in sources {
        let Some(donor) = crate::work::get(conn, &source_id)? else {
            continue;
        };
        let file = crate::folder::finished(conn, config, &donor)?;
        let (hearing, boundaries, mut parts) = match file.path.as_deref() {
            None => (HearingState::NoFile, Vec::new(), Vec::new()),
            Some(path) => match donor_map::hearing(conn, &donor.id, Some(path))? {
                Hearing::Unheard => (HearingState::Unheard, Vec::new(), Vec::new()),
                Hearing::Stale => (HearingState::Stale, Vec::new(), Vec::new()),
                Hearing::Heard(heard) => (
                    HearingState::Heard,
                    heard.boundaries,
                    heard
                        .parts
                        .into_iter()
                        .map(|part| OfferedPart {
                            starts_at: part.starts_at,
                            ends_at: part.ends_at,
                            reason: part.reason,
                            detail: None,
                            source: PartSource::Sound,
                        })
                        .collect(),
                ),
            },
        };
        parts.extend(by_words(conn, config, &donor)?);

        let mut taken = Vec::new();
        for cut in crate::cut::from_source(conn, &donor.id)? {
            if cut.work_id == short_id {
                continue;
            }
            let title = crate::work::get(conn, &cut.work_id)?
                .map(|work| work.title)
                .unwrap_or_default();
            taken.push(TakenStretch {
                work_id: cut.work_id,
                title,
                starts_at: cut.starts_at,
                ends_at: cut.ends_at,
            });
        }
        taken.sort_by(|a, b| a.starts_at.total_cmp(&b.starts_at));

        offered.push(DonorParts {
            duration: crate::scene::duration_of(&donor),
            source_id: donor.id,
            source_title: donor.title,
            hearing,
            boundaries,
            parts,
            taken,
        });
    }
    Ok(offered)
}

/// The parts the words offer in a video: runs of its timed scenes that play
/// one section of the song, scored by whether the section is a chorus, how
/// many of its lines come back elsewhere in the song, and how early it comes.
fn by_words(
    conn: &Connection,
    config: &ProfileConfig,
    donor: &crate::work::Work,
) -> Result<Vec<OfferedPart>> {
    let scenes = crate::scene::for_work(conn, &donor.id)?;
    // Runs of consecutive scenes on one section, every scene timed.
    let mut runs: Vec<(String, f64, Vec<f64>)> = Vec::new();
    for scene in &scenes {
        let (Some(section), Some(starts), Some(ends)) =
            (scene.section.as_deref(), scene.starts_at, scene.ends_at)
        else {
            continue;
        };
        let section = section.trim();
        if section.is_empty() || ends <= starts {
            continue;
        }
        match runs.last_mut() {
            Some((name, _, edges))
                if name.eq_ignore_ascii_case(section)
                    && edges.last().is_some_and(|end| (end - starts).abs() < 0.5) =>
            {
                edges.push(ends);
            }
            _ => runs.push((section.to_owned(), starts, vec![ends])),
        }
    }
    if runs.is_empty() {
        return Ok(Vec::new());
    }

    let text = song_text(conn, config, donor)?.unwrap_or_default();
    let sections = sections_of(&text);
    let counts = line_counts(&text);

    let mut seen_bases: HashMap<String, usize> = HashMap::new();
    let mut scored: Vec<(f64, OfferedPart)> = Vec::new();
    for (name, starts, edges) in runs {
        let base = base_of(&name);
        let nth = {
            let seen = seen_bases.entry(base.clone()).or_insert(0);
            *seen += 1;
            *seen
        };
        // The part runs to the last scene end that keeps it within LONGEST.
        let ends = edges
            .iter()
            .copied()
            .rfind(|end| end - starts <= LONGEST)
            .unwrap_or_else(|| (starts + LONGEST).min(edges[edges.len() - 1]));
        if ends - starts < SHORTEST {
            continue;
        }
        let lines = section_lines(&sections, &name, &base, nth);
        let coming_back = if lines.is_empty() {
            0.0
        } else {
            lines
                .iter()
                .filter(|line| counts.get(&normal(line)).copied().unwrap_or(0) > 1)
                .count() as f64
                / lines.len() as f64
        };
        let chorus = is_chorus(&base);
        if !chorus && coming_back < 0.5 {
            continue;
        }
        let score =
            if chorus { 0.5 } else { 0.0 } + 0.4 * coming_back + if nth == 1 { 0.1 } else { 0.0 };
        let opening = lines.first().map(|line| format!("{name}: {line}"));
        scored.push((
            score,
            OfferedPart {
                starts_at: starts,
                ends_at: ends,
                reason: if chorus { "wordsChorus" } else { "wordsHook" }.into(),
                detail: opening.or(Some(name)),
                source: PartSource::Words,
            },
        ));
    }
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    Ok(scored
        .into_iter()
        .take(MOST_BY_WORDS)
        .map(|(_, part)| part)
        .collect())
}

/// The words of the song a video is made from, in its first sung role.
fn song_text(
    conn: &Connection,
    config: &ProfileConfig,
    donor: &crate::work::Work,
) -> Result<Option<String>> {
    let Some(song) = crate::publication::origin(conn, config, &donor.id)? else {
        return Ok(None);
    };
    for role in config
        .all_version_roles()
        .into_iter()
        .filter(|role| role.sung)
    {
        if let Some(version) = crate::work::version::for_role(conn, &song.id, &role.key)? {
            return Ok(Some(version.body));
        }
    }
    Ok(None)
}

/// A song's sections as its text marks them - `[Chorus 1]` over its lines -
/// each with its name as written.
fn sections_of(text: &str) -> Vec<(String, Vec<String>)> {
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            sections.push((name.trim().to_owned(), Vec::new()));
        } else if let Some((_, lines)) = sections.last_mut() {
            lines.push(line.to_owned());
        }
    }
    sections
}

/// The lines of the section a scene names: by its whole name when the text
/// numbers its sections as the scenes do, else the `nth` section of that kind.
fn section_lines(
    sections: &[(String, Vec<String>)],
    name: &str,
    base: &str,
    nth: usize,
) -> Vec<String> {
    if let Some((_, lines)) = sections
        .iter()
        .find(|(section, _)| section.eq_ignore_ascii_case(name))
    {
        return lines.clone();
    }
    sections
        .iter()
        .filter(|(section, _)| base_of(section) == base)
        .nth(nth - 1)
        .map(|(_, lines)| lines.clone())
        .unwrap_or_default()
}

/// How many times each line is sung in the text, its sections' names left out.
fn line_counts(text: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('[') {
            continue;
        }
        *counts.entry(normal(line)).or_insert(0) += 1;
    }
    counts
}

/// A line as it is compared: letters and digits, lower case, one space.
fn normal(line: &str) -> String {
    line.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_lowercase().next().unwrap_or(c)
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A section's kind, its number left off: `chorus` of `Chorus 2`.
fn base_of(name: &str) -> String {
    name.trim()
        .trim_end_matches(|c: char| c.is_ascii_digit() || c.is_whitespace())
        .to_lowercase()
}

/// Whether a section's kind is the one a song keeps coming back to - not
/// the part that leads into it.
fn is_chorus(base: &str) -> bool {
    let before = ["pre-chorus", "prechorus", "pre chorus", "предприпев"];
    !before.iter().any(|word| base.contains(word))
        && ["chorus", "refrain", "hook", "припев", "рефрен"]
            .iter()
            .any(|word| base.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions;
    use crate::fixtures;

    /// A song with a clip and a short cut from it, the clip's scenes timed.
    fn a_family(conn: &Connection, profile_id: &str) -> (String, String, String) {
        let song = fixtures::work(conn, profile_id, "song", "Tide");
        fixtures::version(
            conn,
            &song.id,
            "lyrics",
            "[Verse 1]\nThe harbour lights are low\nAnd the water keeps the time\n\
             [Chorus 1]\nHold the tide\nHold the tide for me\n\
             [Verse 2]\nThe boats are sleeping now\nThe gulls have flown\n\
             [Chorus 2]\nHold the tide\nHold the tide for me\n",
        );
        let clip = actions::work::derive(conn, &song.id, "video", None, None, None).unwrap();
        let short = actions::work::derive(conn, &clip.work.id, "short", None, None, None).unwrap();
        for (position, section, starts, ends) in [
            (1, "Verse 1", 0.0, 20.0),
            (2, "Chorus 1", 20.0, 32.0),
            (3, "Chorus 1", 32.0, 44.0),
            (4, "Verse 2", 44.0, 64.0),
            (5, "chorus 2", 64.0, 90.0),
        ] {
            actions::scene::create(
                conn,
                crate::scene::NewScene {
                    work_id: clip.work.id.clone(),
                    position: Some(position),
                    section: Some(section.into()),
                    starts_at: Some(starts),
                    ends_at: Some(ends),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        (song.id, clip.work.id, short.work.id)
    }

    /// The words offer the choruses the scenes play, placed by their
    /// seconds, the first first; the verses, whose lines never come back,
    /// are not offered.
    #[test]
    fn the_words_offer_the_choruses_where_the_scenes_play_them() {
        let (conn, profile_id) = fixtures::workspace();
        let (_song, clip, short) = a_family(&conn, &profile_id);
        let config = crate::profile::config_for(&conn, &profile_id).unwrap();

        let offered = for_short(&conn, &config, &short).unwrap();

        assert_eq!(offered.len(), 1);
        let donor = &offered[0];
        assert_eq!(donor.source_id, clip);
        assert_eq!(donor.hearing, HearingState::NoFile, "no media folder here");
        let spans: Vec<(f64, f64)> = donor
            .parts
            .iter()
            .map(|part| (part.starts_at, part.ends_at))
            .collect();
        assert_eq!(
            spans,
            vec![(20.0, 44.0), (64.0, 90.0)],
            "both choruses, first first"
        );
        assert!(donor.parts.iter().all(|part| part.reason == "wordsChorus"));
        assert_eq!(
            donor.parts[0].detail.as_deref(),
            Some("Chorus 1: Hold the tide"),
            "the section as the scenes name it and the line it opens with"
        );
    }

    /// What a sibling took out of the same video is said, and the short's
    /// own stretches are not.
    #[test]
    fn what_a_sibling_took_is_said() {
        let (conn, profile_id) = fixtures::workspace();
        let (song, clip, short) = a_family(&conn, &profile_id);
        let sibling = actions::work::derive(&conn, &clip, "short", None, None, None).unwrap();
        for (work, starts) in [(&sibling.work.id, 20.0), (&short, 64.0)] {
            actions::cut::create(
                &conn,
                crate::cut::NewCut {
                    work_id: work.clone(),
                    source_id: clip.clone(),
                    starts_at: starts,
                    ends_at: starts + 24.0,
                    position: None,
                    label: None,
                },
            )
            .unwrap();
        }
        let config = crate::profile::config_for(&conn, &profile_id).unwrap();

        let offered = for_short(&conn, &config, &short).unwrap();

        let taken = &offered[0].taken;
        assert_eq!(taken.len(), 1, "{taken:?}");
        assert_eq!(taken[0].work_id, sibling.work.id);
        assert_eq!(taken[0].title, sibling.work.title);
        assert_eq!((taken[0].starts_at, taken[0].ends_at), (20.0, 44.0));
        let _ = song;
    }

    /// What was heard in the video's file is offered while it is still the
    /// video's finished file, ahead of what the words offer.
    #[test]
    fn what_was_heard_is_offered_first() {
        let (conn, profile_id) = fixtures::workspace();
        let (_song, clip, short) = a_family(&conn, &profile_id);
        let root = tempfile::tempdir().unwrap();
        crate::folder::set_root(
            &conn,
            &profile_id,
            Some(&root.path().to_string_lossy()),
            "2026-10-27T10:00:00Z",
        )
        .unwrap();
        let config = crate::profile::config_for(&conn, &profile_id).unwrap();
        let donor = crate::work::get(&conn, &clip).unwrap().unwrap();
        let place = crate::folder::locate(&conn, &config, &donor).unwrap();
        let folder = std::path::PathBuf::from(place.path.expect("the clip has a folder"));
        std::fs::create_dir_all(&folder).unwrap();
        let file = folder.join("tide.mp4");
        std::fs::write(&file, b"not really a video").unwrap();
        let path = crate::folder::finished(&conn, &config, &donor)
            .unwrap()
            .path
            .expect("the rule finds the render");
        donor_map::keep_heard(
            &conn,
            &profile_id,
            &clip,
            &path,
            &donor_map::Stamp::of(std::path::Path::new(&path)).unwrap(),
            &[0.0, 20.5, 44.0],
            &[donor_map::HeardPart {
                starts_at: 20.5,
                ends_at: 44.0,
                reason: "chorus".into(),
                score: 0.9,
            }],
            "cut 0.2.0",
            "2026-10-27T10:00:00Z",
        )
        .unwrap();

        let offered = for_short(&conn, &config, &short).unwrap();

        let donor = &offered[0];
        assert_eq!(donor.hearing, HearingState::Heard);
        assert_eq!(donor.boundaries, vec![0.0, 20.5, 44.0]);
        assert_eq!(donor.parts[0].source, PartSource::Sound);
        assert_eq!(donor.parts[0].reason, "chorus");
        assert_eq!(donor.parts[1].source, PartSource::Words);
    }

    #[test]
    fn a_section_kind_is_read_without_its_number() {
        assert_eq!(base_of("Chorus 2"), "chorus");
        assert_eq!(base_of("verse"), "verse");
        assert!(
            !is_chorus("pre-chorus"),
            "the part before the chorus is not it"
        );
        assert!(is_chorus("припев"));
        assert_eq!(normal("Hold the tide, for me!"), "hold the tide for me");
    }
}
