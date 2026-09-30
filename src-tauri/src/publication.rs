//! A work that goes out, and the work it goes out for.
//!
//! A song is a thing: its text, its style, its score. It is never in front of
//! anyone as the song. What an audience meets is a clip cut to it, the track
//! on a video platform under one picture, a short - its *publications*, each a
//! work of its own with its own releases, files, cover and comments (the
//! owner's decision of 2026-09-27). So a kind of work may have no door of its
//! own at all, and from v0.86 the song has none: its facts - booked, gone out -
//! are the facts of what was made from it (`work::status::speaking_for`).
//!
//! This module answers the two questions that follow. What are a work's
//! publications, and which of them says where it stands ([`of`]). And, once:
//! how does a workspace that planned its songs' audio releases on the songs
//! themselves come over that line ([`upgrade`]) - the second half of ADR 0030,
//! which took the clip and short doors off the song in v0.74 and left it the
//! audio one.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::error::Result;
use crate::journal::{self, Record};
use crate::link::{self, NewLink};
use crate::minted::Minted;
use crate::profile::config::{Label, ProfileConfig};
use crate::release;
use crate::time::now;
use crate::work::{self, NewWork, status};

/// One work made from another, as the work it was made from lists it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Publication {
    pub work_id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub stage: Option<i64>,
    /// How far down it stands: 1 for a clip made from the song, 2 for a short
    /// cut from that clip.
    pub depth: i64,
    /// What it was made from, when that is not the work asking: the clip a
    /// short was cut from.
    pub via: Option<String>,
    /// Its releases, and how many of them went out.
    pub releases: i64,
    pub released: i64,
    /// When the latest of them went out.
    pub last_released_at: Option<String>,
    /// The earliest day one still waiting is booked for - past, when it is
    /// late.
    pub next_scheduled_at: Option<String>,
    /// The comments under it that are not archived, and how many of those
    /// still wait for an answer.
    pub comments: i64,
    pub comments_waiting: i64,
    pub created_at: String,
}

/// What made a work's status what it is, when it is a publication's fact: the
/// audio that went out on the 22nd, the clip booked for the 3rd.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct Basis {
    pub work_id: String,
    pub title: String,
    pub kind: String,
    /// Went out, rather than booked.
    pub released: bool,
    /// The day it went out, or is booked for.
    pub day: String,
}

/// A work's publications, and the one its status stands on.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Publications {
    pub items: Vec<Publication>,
    /// Absent when nothing made from the work has gone out or holds a day.
    pub basis: Option<Basis>,
}

/// Everything made from a work, directly or through what was made from it,
/// in the order the profile lists the kinds and then oldest first; and the
/// release that decides where the work stands - the latest that went out, or
/// else the earliest booked.
pub fn of(conn: &Connection, config: &ProfileConfig, work_id: &str) -> Result<Publications> {
    let mut items = Vec::new();
    for (id, depth) in link::descendants(conn, work_id)? {
        let Some(made) = work::get(conn, &id)? else {
            continue;
        };
        let via = if depth > 1 {
            link::sources(conn, &id)?
                .into_iter()
                .find(|source| source.source_id != work_id)
                .map(|source| source.source_title)
        } else {
            None
        };
        let (releases, released, last_released_at, next_scheduled_at): (
            i64,
            i64,
            Option<String>,
            Option<String>,
        ) = conn.query_row(
            "SELECT count(*),
                    coalesce(sum(status = ?2), 0),
                    max(CASE WHEN status = ?2 THEN released_at END),
                    min(CASE WHEN status <> ?2 THEN scheduled_at END)
               FROM release WHERE work_id = ?1",
            params![id, release::RELEASED],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        let (comments, comments_waiting) = crate::comment::count_for_work(conn, &id)?;
        items.push(Publication {
            work_id: made.id,
            title: made.title,
            kind: made.kind,
            status: made.status,
            stage: made.stage,
            depth,
            via,
            releases,
            released,
            last_released_at,
            next_scheduled_at,
            comments,
            comments_waiting,
            created_at: made.created_at,
        });
    }
    let order = |kind: &str| {
        config
            .work_kinds
            .iter()
            .position(|known| known.key == kind)
            .unwrap_or(usize::MAX)
    };
    items.sort_by(|a, b| {
        order(&a.kind)
            .cmp(&order(&b.kind))
            .then_with(|| a.created_at.cmp(&b.created_at))
            .then_with(|| a.title.cmp(&b.title))
    });

    let gone = items
        .iter()
        .filter_map(|item| Some((item, item.last_released_at.clone()?)))
        .max_by(|a, b| a.1.cmp(&b.1));
    let booked = items
        .iter()
        .filter_map(|item| Some((item, item.next_scheduled_at.clone()?)))
        .min_by(|a, b| a.1.cmp(&b.1));
    let basis = match (gone, booked) {
        (Some((item, day)), _) => Some((item, true, day)),
        (None, Some((item, day))) => Some((item, false, day)),
        (None, None) => None,
    }
    .map(|(item, released, day)| Basis {
        work_id: item.work_id.clone(),
        title: item.title.clone(),
        kind: item.kind.clone(),
        released,
        day: day.get(..10).unwrap_or(&day).to_owned(),
    });

    Ok(Publications { items, basis })
}

/// The key of the shipped Studio profile - the only one this move reads. A
/// profile the owner invented may give a song any door it likes.
const STUDIO: &str = "music";

/// The kinds of work whose audio door comes off.
const SOURCE_KINDS: [&str; 2] = ["song", "instrumental"];

/// The door that moves, the kind of work it moves onto, and the door it goes
/// out through there.
const DOOR: &str = "audio";
const TARGET_KIND: &str = "audio";
const TARGET_DOOR: &str = "youtube";

/// The kind that was folded into the audio kind's variants (v0.86).
const INSTRUMENTAL: &str = "instrumental";

/// What the song's audio door was called in the profiles kilna shipped until
/// v0.86, in each language they carried. The shipped document no longer has
/// the door, so its old words are kept here for the one question they still
/// answer: is a work named "<song> — audio release" that song's release?
const SHIPPED_DOOR_NAMES: [&str; 2] = ["Audio release", "Аудио-релиз"];

/// What the move did, for the history and for a test to read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Releases that changed hands.
    pub moved: usize,
    /// Audio works made to take them.
    pub created: usize,
    /// Works of another kind that turned out to be a song's audio release
    /// already - named after the song and the door - and became audio works.
    pub converted: usize,
    /// Publications made from a song before a link could say so, found by
    /// the name kilna gives what it makes - "Harbour lights — clip" - and
    /// linked to it now.
    pub linked: usize,
    /// Publications left holding more than one release through the same
    /// door: two plans of one upload, or two uploads. Not guessed at: listed
    /// in the history for the owner to settle.
    pub disputed: Vec<String>,
}

/// Move every audio release planned on a song onto the song's audio
/// publication, and take the door off the song. Runs when a workspace opens,
/// after the profiles are seeded - the carry-forward is what brings the audio
/// kind into a workspace that predates it - and only while the stored song
/// still has the door: once it is off, there is nothing to move, and a video
/// named like an audio release a year from now is not quietly turned into one.
///
/// One transaction per profile: a workspace is carried over or left as it
/// was, never half of each. Not written to the operation log, for the reason
/// the door move of v0.74 was not (ADR 0030): it is what the schema of this
/// version means, not something a person did.
pub fn upgrade(conn: &mut Connection) -> Result<Report> {
    let profiles: Vec<(String, String)> = conn
        .prepare("SELECT id, config FROM profile WHERE key = ?1")?
        .query_map(params![STUDIO], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let shipped = crate::profile::builtin()?
        .into_iter()
        .find(|profile| profile.key == STUDIO)
        .map(|profile| profile.config);

    let mut total = Report::default();
    for (profile_id, raw) in profiles {
        let mut config: ProfileConfig = serde_json::from_str(&raw)?;
        if !still_has_the_door(&config) || !target_ready(&config) {
            continue;
        }
        let tx = conn.transaction()?;
        let (report, touched) = move_releases(&tx, &profile_id, &config, shipped.as_ref())?;

        take_the_door_off(&mut config);
        drop_instrumental(&tx, &profile_id, &mut config)?;
        tx.execute(
            "UPDATE profile SET config = ?2, updated_at = ?3 WHERE id = ?1",
            params![profile_id, serde_json::to_string(&config)?, now()],
        )?;

        // Every status the move touched, derived again against the document
        // as it now stands: a song without its door speaks through what was
        // made from it, which is the point of the move. Derived before the
        // door came off, it would still read its own - now empty - releases.
        for work_id in &touched {
            status::refresh(&tx, &config, work_id)?;
        }
        // And every other work that no longer goes out itself: the rule its
        // status is read by changed with this move, not only for the songs
        // whose releases moved. A song whose short went out last month is
        // out, and says so from now on. A status a person pinned stays.
        let doorless: Vec<String> = config
            .work_kinds
            .iter()
            .filter(|kind| !kind.has_doors())
            .map(|kind| kind.key.clone())
            .collect();
        let mut restated = 0usize;
        for kind in &doorless {
            let ids: Vec<String> = tx
                .prepare("SELECT id FROM work WHERE profile_id = ?1 AND kind = ?2")?
                .query_map(params![profile_id, kind], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            for id in ids {
                if status::refresh(&tx, &config, &id)?.is_some() {
                    restated += 1;
                }
            }
        }
        if restated > 0 {
            journal::record(
                &tx,
                &profile_id,
                Record::new("status.resynced").param("count", restated as i64),
            );
        }

        if report.moved > 0 || report.converted > 0 || report.linked > 0 {
            journal::record(
                &tx,
                &profile_id,
                Record::new("upgrade.publicationsMoved")
                    .param("count", report.moved as i64)
                    .param("created", report.created as i64)
                    .param("converted", report.converted as i64)
                    .param("linked", report.linked as i64),
            );
        }
        for title in &report.disputed {
            journal::record(
                &tx,
                &profile_id,
                Record::new("upgrade.publicationDisputed")
                    .param("title", title.clone())
                    .warn(),
            );
        }
        tx.commit()?;

        total.moved += report.moved;
        total.created += report.created;
        total.converted += report.converted;
        total.linked += report.linked;
        total.disputed.extend(report.disputed);
    }
    Ok(total)
}

/// Whether the stored song or instrumental still lists the audio door.
fn still_has_the_door(config: &ProfileConfig) -> bool {
    config.work_kinds.iter().any(|kind| {
        SOURCE_KINDS.contains(&kind.key.as_str())
            && kind.release_kinds.iter().any(|door| door.key == DOOR)
    })
}

/// Whether the audio kind can take the releases: it is there, can hold a
/// work, and goes out through the door they move to. An owner who deleted it
/// keeps the door on the song and every release where it was.
fn target_ready(config: &ProfileConfig) -> bool {
    config.kind(TARGET_KIND).is_some_and(|kind| {
        kind.starting_status().is_some()
            && kind
                .release_kinds
                .iter()
                .any(|door| door.key == TARGET_DOOR)
    })
}

fn move_releases(
    conn: &Connection,
    profile_id: &str,
    config: &ProfileConfig,
    shipped: Option<&ProfileConfig>,
) -> Result<(Report, Vec<String>)> {
    let mut report = Report::default();
    let mut touched: Vec<String> = Vec::new();
    let songs: Vec<(String, String)> = conn
        .prepare(
            "SELECT id, title FROM work
              WHERE profile_id = ?1 AND kind IN (?2, ?3)
              ORDER BY created_at, rowid",
        )?
        .query_map(
            params![profile_id, SOURCE_KINDS[0], SOURCE_KINDS[1]],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?
        .collect::<rusqlite::Result<_>>()?;
    let names = door_names(config, shipped);
    let made_names = made_names(config, shipped);

    for (song_id, song_title) in songs {
        // Its publications made before a link could say so - the clip moved
        // in from the predecessor on 12.09 is one - named the way kilna
        // names what it makes from a song.
        for made in named_as_made(conn, profile_id, config, &song_title, &made_names)? {
            link::create_minted(
                conn,
                profile_id,
                NewLink {
                    work_id: made.clone(),
                    source_id: song_id.clone(),
                    role: None,
                    source_version_id: None,
                },
                Minted::fresh(),
            )?;
            report.linked += 1;
            touched.push(made);
            touched.push(song_id.clone());
        }

        let releases: Vec<(String, String)> = conn
            .prepare(
                "SELECT id, status FROM release WHERE work_id = ?1 AND kind = ?2
                  ORDER BY created_at, rowid",
            )?
            .query_map(params![song_id, DOOR], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;

        // The song's audio publication: one already made, one that was
        // made under another kind and named after the song and the door, or
        // a new one when there are releases to take.
        let publication = match existing_audio(conn, &song_id)? {
            Some(id) => Some(id),
            None => match named_after(conn, profile_id, &song_id, &song_title, &names)? {
                Some(id) => {
                    if convert(conn, profile_id, config, &id, &song_id)? {
                        report.converted += 1;
                        touched.push(id.clone());
                        touched.push(song_id.clone());
                        Some(id)
                    } else {
                        // A work that cannot become audio - one of its
                        // releases goes through a door audio does not have -
                        // is the owner's to sort out.
                        report.disputed.push(song_title.clone());
                        None
                    }
                }
                None => None,
            },
        };
        if releases.is_empty() {
            continue;
        }
        let publication = match publication {
            Some(id) => id,
            None => {
                report.created += 1;
                create_audio(conn, profile_id, config, &song_id, &song_title)?
            }
        };

        for (release_id, release_status) in &releases {
            conn.execute(
                "UPDATE release SET work_id = ?2, kind = ?3, updated_at = ?4 WHERE id = ?1",
                params![release_id, publication, TARGET_DOOR, now()],
            )?;
            // A work that shipped is finished by definition, as it was for the
            // clips and shorts in v0.74.
            if release_status == release::RELEASED {
                conn.execute(
                    "UPDATE work SET stage = 100 WHERE id = ?1",
                    params![publication],
                )?;
            }
            report.moved += 1;
        }

        let through_the_door: i64 = conn.query_row(
            "SELECT count(*) FROM release WHERE work_id = ?1 AND kind = ?2",
            params![publication, TARGET_DOOR],
            |row| row.get(0),
        )?;
        if through_the_door > 1 {
            report.disputed.push(song_title.clone());
        }

        touched.push(publication);
        touched.push(song_id);
    }
    touched.sort();
    touched.dedup();
    Ok((report, touched))
}

/// What kilna calls a work made from another, by kind, with the source's
/// title and the number taken out and folded: "clip", "клип", "short",
/// "шортс". Read from the stored document and the shipped one.
fn made_names(config: &ProfileConfig, shipped: Option<&ProfileConfig>) -> Vec<(String, String)> {
    let mut names: Vec<(String, String)> = Vec::new();
    for profile in std::iter::once(config).chain(shipped) {
        for kind in &profile.work_kinds {
            let Some(template) = &kind.made_title else {
                continue;
            };
            for word in template.words() {
                let rest = fold(&word.replace("{title}", "").replace("{n}", ""));
                if !rest.is_empty() && !names.contains(&(kind.key.clone(), rest.clone())) {
                    names.push((kind.key.clone(), rest));
                }
            }
        }
    }
    names
}

/// Works made from nothing whose title is a song's followed by the name
/// kilna gives its own kind - "Harbour lights — clip", "Harbour lights ·
/// short 3". Only a work of a kind that goes out, and only one linked to
/// nothing: a link already made is the owner's word and is not second-guessed.
fn named_as_made(
    conn: &Connection,
    profile_id: &str,
    config: &ProfileConfig,
    song_title: &str,
    made_names: &[(String, String)],
) -> Result<Vec<String>> {
    let song = fold(song_title);
    if song.is_empty() {
        return Ok(Vec::new());
    }
    let candidates: Vec<(String, String, String)> = conn
        .prepare(
            "SELECT w.id, w.kind, w.title FROM work w
              WHERE w.profile_id = ?1
                AND NOT EXISTS (SELECT 1 FROM work_link l WHERE l.work_id = w.id)
              ORDER BY w.created_at, w.rowid",
        )?
        .query_map(params![profile_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut found = Vec::new();
    for (id, kind, title) in candidates {
        if !config.vocabulary(&kind).has_doors() {
            continue;
        }
        let folded = fold(&title);
        let Some(rest) = folded.strip_prefix(&song) else {
            continue;
        };
        // Only a whole word after the song's title: "Harbour" is not the
        // song of "Harbourside — clip".
        if !rest.is_empty() && !rest.starts_with(' ') {
            continue;
        }
        let rest = rest
            .trim()
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .trim();
        if made_names
            .iter()
            .any(|(named_kind, name)| *named_kind == kind && name == rest)
        {
            found.push(id);
        }
    }
    Ok(found)
}

/// The audio work already made from a song, oldest first.
fn existing_audio(conn: &Connection, song_id: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT w.id FROM work_link l JOIN work w ON w.id = l.work_id
              WHERE l.source_id = ?1 AND w.kind = ?2
              ORDER BY w.created_at, w.rowid LIMIT 1",
            params![song_id, TARGET_KIND],
            |row| row.get(0),
        )
        .optional()?)
}

/// Every word the door and the audio kind are called by, stored or shipped,
/// in any language, folded for comparison: "audio release", "аудио-релиз",
/// "audio", "аудио".
fn door_names(config: &ProfileConfig, shipped: Option<&ProfileConfig>) -> Vec<String> {
    let mut labels: Vec<&Label> = Vec::new();
    for profile in std::iter::once(config).chain(shipped) {
        for kind in &profile.work_kinds {
            if SOURCE_KINDS.contains(&kind.key.as_str()) {
                labels.extend(
                    kind.release_kinds
                        .iter()
                        .filter(|door| door.key == DOOR)
                        .map(|door| &door.label),
                );
            }
            if kind.key == TARGET_KIND {
                labels.push(&kind.label);
            }
        }
    }
    let mut names: Vec<String> = labels
        .into_iter()
        .flat_map(Label::words)
        .chain(SHIPPED_DOOR_NAMES)
        .map(fold)
        .filter(|name| !name.is_empty())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// A title as the move compares it: lower case, dashes and quotes gone, one
/// space between words.
fn fold(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            '—' | '–' | '-' | '·' | ':' | '«' | '»' | '"' | '“' | '”' => ' ',
            other => other,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A work of another kind that is this song's audio release in all but kind:
/// made from the song or from nothing, and called the song's title followed by
/// the door's name - "Harbour lights — audio release". The owner made three
/// such videos before the audio kind existed.
fn named_after(
    conn: &Connection,
    profile_id: &str,
    song_id: &str,
    song_title: &str,
    names: &[String],
) -> Result<Option<String>> {
    let song = fold(song_title);
    if song.is_empty() {
        return Ok(None);
    }
    let candidates: Vec<(String, String, bool)> = conn
        .prepare(
            "SELECT w.id, w.title,
                    EXISTS (SELECT 1 FROM work_link l WHERE l.work_id = w.id AND l.source_id = ?2)
               FROM work w
              WHERE w.profile_id = ?1 AND w.kind NOT IN (?3, ?4, ?5)
                AND NOT EXISTS (SELECT 1 FROM work_link l
                                 WHERE l.work_id = w.id AND l.source_id <> ?2)
              ORDER BY w.created_at, w.rowid",
        )?
        .query_map(
            params![
                profile_id,
                song_id,
                TARGET_KIND,
                SOURCE_KINDS[0],
                SOURCE_KINDS[1]
            ],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?
        .collect::<rusqlite::Result<_>>()?;
    for (id, title, _linked) in candidates {
        let folded = fold(&title);
        let Some(rest) = folded.strip_prefix(&song) else {
            continue;
        };
        if !rest.starts_with(' ') {
            continue;
        }
        let rest = rest.trim();
        if names.iter().any(|name| name == rest) {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// Make a work of another kind the audio work it already was: its kind, a
/// donor link to the song when it has none, the fields audio starts with.
/// Refused - `false` - when one of its releases goes out through a door the
/// audio kind does not have, which would leave that release nowhere.
fn convert(
    conn: &Connection,
    profile_id: &str,
    config: &ProfileConfig,
    work_id: &str,
    song_id: &str,
) -> Result<bool> {
    let audio = config.vocabulary(TARGET_KIND);
    let doors: Vec<String> = conn
        .prepare("SELECT DISTINCT kind FROM release WHERE work_id = ?1")?
        .query_map(params![work_id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    if doors
        .iter()
        .any(|door| !audio.release_kinds.iter().any(|known| known.key == *door))
    {
        return Ok(false);
    }
    let Some(found) = work::get(conn, work_id)? else {
        return Ok(false);
    };
    // The status keeps its word when audio has it, and otherwise takes the
    // word audio has for the same meaning.
    let status = if audio.statuses.iter().any(|known| known.key == found.status) {
        found.status.clone()
    } else {
        audio
            .starting_status()
            .map_or(found.status.clone(), |known| known.key.clone())
    };
    let mut meta = found.meta.clone();
    for (key, value) in config.defaults_of(TARGET_KIND) {
        meta.entry(key).or_insert(value);
    }
    conn.execute(
        "UPDATE work SET kind = ?2, status = ?3, meta = ?4, updated_at = ?5 WHERE id = ?1",
        params![
            work_id,
            TARGET_KIND,
            status,
            Value::Object(meta).to_string(),
            now()
        ],
    )?;
    let linked: bool = conn
        .query_row(
            "SELECT 1 FROM work_link WHERE work_id = ?1 AND source_id = ?2",
            params![work_id, song_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !linked {
        link::create_minted(
            conn,
            profile_id,
            NewLink {
                work_id: work_id.to_owned(),
                source_id: song_id.to_owned(),
                role: None,
                source_version_id: None,
            },
            Minted::fresh(),
        )?;
    }
    Ok(true)
}

/// A new audio work for a song: the song's title and the fields audio has,
/// made from the song at the version it is on, the way *Make an audio* makes
/// one - except for the title, which stays the song's: the words a window
/// would add ("— audio") are in the language of a window this move has not
/// got, and the owner's titles are their own words.
fn create_audio(
    conn: &Connection,
    profile_id: &str,
    config: &ProfileConfig,
    song_id: &str,
    song_title: &str,
) -> Result<String> {
    let song = work::get(conn, song_id)?
        .map(|song| song.meta)
        .unwrap_or_default();
    let mut meta: Map<String, Value> = song
        .into_iter()
        .filter(|(key, _)| {
            config
                .fields_of(TARGET_KIND)
                .iter()
                .any(|field| field.key == *key)
        })
        .collect();
    for (key, value) in config.defaults_of(TARGET_KIND) {
        meta.entry(key).or_insert(value);
    }
    let created = work::create_minted(
        conn,
        profile_id,
        NewWork {
            kind: TARGET_KIND.to_owned(),
            title: song_title.to_owned(),
            meta: Some(meta),
            ..NewWork::default()
        },
        Minted::fresh(),
    )?;
    link::create_minted(
        conn,
        profile_id,
        NewLink {
            work_id: created.id.clone(),
            source_id: song_id.to_owned(),
            role: None,
            source_version_id: None,
        },
        Minted::fresh(),
    )?;
    Ok(created.id)
}

/// Take the audio door off the song and the instrumental in the stored
/// document. The tiers stay: a song's "clip" tier is a judgement of the song.
fn take_the_door_off(config: &mut ProfileConfig) {
    for kind in &mut config.work_kinds {
        if SOURCE_KINDS.contains(&kind.key.as_str()) {
            kind.release_kinds.retain(|door| door.key != DOOR);
        }
    }
}

/// The instrumental kind comes off the stored document when nothing is of it:
/// an instrumental is a variant of an audio release now, not a kind of work.
/// A workspace with instrumentals keeps the kind - they are someone's works.
fn drop_instrumental(
    conn: &Connection,
    profile_id: &str,
    config: &mut ProfileConfig,
) -> Result<()> {
    let held: i64 = conn.query_row(
        "SELECT count(*) FROM work WHERE profile_id = ?1 AND kind = ?2",
        params![profile_id, INSTRUMENTAL],
        |row| row.get(0),
    )?;
    if held == 0 {
        config.work_kinds.retain(|kind| kind.key != INSTRUMENTAL);
        for prompt in &mut config.prompts {
            prompt.kinds.retain(|kind| kind != INSTRUMENTAL);
        }
        if let Some(columns) = config.catalogue_columns_by_kind.as_mut() {
            columns.remove(INSTRUMENTAL);
        }
        for field in &mut config.work_meta_fields {
            field.kinds.retain(|kind| kind != INSTRUMENTAL);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::profile::{self, config::ReleaseKind};
    use crate::release::{self as releases, NewRelease, ReleasePatch};
    use serde_json::json;

    fn link(conn: &Connection, profile_id: &str, made: &str, source: &str) {
        link::create(
            conn,
            profile_id,
            NewLink {
                work_id: made.to_owned(),
                source_id: source.to_owned(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();
    }

    fn status_of(conn: &Connection, work_id: &str) -> String {
        work::get(conn, work_id).unwrap().unwrap().status
    }

    fn refresh(conn: &Connection, profile_id: &str, work_id: &str) {
        crate::actions::restate(conn, profile_id, work_id);
    }

    /// A song has no door of its own: it is booked when something made from
    /// it holds a day, and out when something made from it went out - a
    /// short cut from its clip included. A clip, which has doors, speaks only
    /// for itself.
    #[test]
    fn a_song_stands_where_what_was_made_from_it_stands() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let clip = fixtures::video(&conn, &profile_id, "Harbour lights — clip");
        let short = fixtures::work(&conn, &profile_id, "short", "Harbour lights · short 1");
        link(&conn, &profile_id, &clip.id, &song.id);
        link(&conn, &profile_id, &short.id, &clip.id);
        assert_eq!(status_of(&conn, &song.id), "draft");

        fixtures::release(&conn, &short.id, "short", Some("2026-10-03"));
        refresh(&conn, &profile_id, &short.id);
        assert_eq!(
            status_of(&conn, &song.id),
            "scheduled",
            "the short cut from the clip holds a day, so the song is booked"
        );
        assert_eq!(
            status_of(&conn, &clip.id),
            "draft",
            "a clip goes out through its own doors, not its short's"
        );

        let out = fixtures::release(&conn, &clip.id, "youtube", Some("2026-09-01"));
        releases::mark_released(&conn, &out.id, None, Some("2026-09-01".into())).unwrap();
        refresh(&conn, &profile_id, &clip.id);
        assert_eq!(status_of(&conn, &song.id), "released");

        let config = profile::config_for(&conn, &profile_id).unwrap();
        let found = of(&conn, &config, &song.id).unwrap();
        assert_eq!(
            found
                .items
                .iter()
                .map(|item| (item.title.as_str(), item.depth, item.via.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("Harbour lights — clip", 1, None),
                ("Harbour lights · short 1", 2, Some("Harbour lights — clip")),
            ],
            "in the profile's order of kinds, the short named by what it was cut from"
        );
        assert_eq!(
            found.basis,
            Some(Basis {
                work_id: clip.id.clone(),
                title: clip.title.clone(),
                kind: "video".into(),
                released: true,
                day: "2026-09-01".into(),
            }),
            "what went out outranks what is booked"
        );

        // Taking the release back takes the song back to booked, through
        // the same restatement every gesture makes.
        crate::actions::release::unmark_released(&conn, &out.id).unwrap();
        assert_eq!(status_of(&conn, &song.id), "scheduled");
    }

    /// Unlinking the clip takes its facts off the song, and trashing the
    /// short takes its day away: both are gestures, and both restate.
    #[test]
    fn a_song_follows_its_publications_as_they_come_and_go() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let clip = fixtures::video(&conn, &profile_id, "Harbour lights — clip");
        let out = fixtures::release(&conn, &clip.id, "youtube", Some("2026-09-01"));
        releases::mark_released(&conn, &out.id, None, None).unwrap();

        let made = crate::actions::link::create(
            &conn,
            NewLink {
                work_id: clip.id.clone(),
                source_id: song.id.clone(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();
        assert_eq!(status_of(&conn, &song.id), "released", "linking restates");

        crate::actions::link::delete(&conn, &made.id).unwrap();
        assert_eq!(status_of(&conn, &song.id), "draft", "unlinking restates");

        let short = fixtures::work(&conn, &profile_id, "short", "Harbour lights · short 1");
        link(&conn, &profile_id, &short.id, &song.id);
        fixtures::release(&conn, &short.id, "short", Some("2026-10-03"));
        refresh(&conn, &profile_id, &short.id);
        assert_eq!(status_of(&conn, &song.id), "scheduled");

        crate::actions::trash::discard(&conn, crate::trash::Entity::Work, &short.id).unwrap();
        assert_eq!(
            status_of(&conn, &song.id),
            "draft",
            "the short went to the trash with its day"
        );
    }

    /// The door a song had before v0.86, as the owner's workspace stores it,
    /// and the instrumental kind beside it.
    fn older_workspace(conn: &Connection) -> String {
        profile::seed(conn).unwrap();
        let profile_id = profile::id_for_key(conn, STUDIO).unwrap().unwrap();
        let mut config = profile::config_for(conn, &profile_id).unwrap();
        for kind in &mut config.work_kinds {
            if kind.key == "song" {
                kind.release_kinds = vec![
                    ReleaseKind::new(DOOR, "Audio release", &["lyrics", "style"]).with_icon("disc"),
                ];
            }
        }
        let mut instrumental = config.kind("song").unwrap().clone();
        instrumental.key = INSTRUMENTAL.into();
        instrumental.label = "Instrumental".into();
        config.work_kinds.insert(1, instrumental);
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![profile_id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();
        profile_id
    }

    fn audio_release(conn: &Connection, song: &str) -> String {
        releases::create(
            conn,
            NewRelease {
                work_id: song.to_owned(),
                kind: DOOR.into(),
                title: Some("Harbour lights".into()),
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap()
        .id
    }

    fn doors_of(conn: &Connection, profile_id: &str, kind: &str) -> Vec<String> {
        profile::config_for(conn, profile_id)
            .unwrap()
            .vocabulary(kind)
            .release_kinds
            .iter()
            .map(|door| door.key.clone())
            .collect()
    }

    fn works_of(conn: &Connection, profile_id: &str, kind: &str) -> Vec<work::Work> {
        work::list(
            conn,
            profile_id,
            &work::WorkFilter {
                kind: Some(kind.into()),
                ..Default::default()
            },
        )
        .unwrap()
    }

    fn config_row(conn: &Connection, profile_id: &str) -> (String, String) {
        conn.query_row(
            "SELECT config, updated_at FROM profile WHERE id = ?1",
            params![profile_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
    }

    fn count_works(conn: &Connection) -> i64 {
        conn.query_row("SELECT count(*) FROM work", [], |row| row.get(0))
            .unwrap()
    }

    /// The owner's case: an audio release planned on the song, one that went
    /// out with its day, link and text. It moves onto an audio work made
    /// from the song, and keeps every word of it.
    #[test]
    fn a_songs_audio_release_moves_onto_an_audio_work_of_its_own() {
        let mut conn = crate::db::open_in_memory().unwrap();
        let profile_id = older_workspace(&conn);
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        conn.execute(
            "UPDATE work SET meta = ?2 WHERE id = ?1",
            params![song.id, json!({ "bpm": 96, "stray": 1 }).to_string()],
        )
        .unwrap();
        let lyrics = fixtures::version(&conn, &song.id, "lyrics", "one line");
        let release = audio_release(&conn, &song.id);
        releases::schedule(&conn, &release, "2026-07-22").unwrap();
        releases::update(
            &conn,
            &release,
            ReleasePatch {
                meta: json!({ "title": "Harbour lights", "source_kind": "picture" })
                    .as_object()
                    .cloned(),
                ..Default::default()
            },
        )
        .unwrap();
        releases::mark_released(
            &conn,
            &release,
            Some("https://example.com/v/1".into()),
            None,
        )
        .unwrap();
        let before = releases::get(&conn, &release).unwrap().unwrap();

        let report = upgrade(&mut conn).unwrap();

        assert_eq!((report.moved, report.created, report.converted), (1, 1, 0));
        assert_eq!(report.linked, 0);
        assert!(report.disputed.is_empty());
        let made = works_of(&conn, &profile_id, TARGET_KIND);
        assert_eq!(made.len(), 1);
        let audio = &made[0];
        assert_eq!(
            audio.title, "Harbour lights",
            "the owner's words, not the window's"
        );
        assert_eq!(
            audio.meta,
            json!({ "bpm": 96, "variant": "original" })
                .as_object()
                .cloned()
                .unwrap(),
            "the song's fields the audio kind has, and the variant it starts at"
        );
        assert_eq!(audio.status, "released");
        assert_eq!(audio.stage, Some(100));
        let sources = link::sources(&conn, &audio.id).unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source_id, song.id);
        assert_eq!(
            sources[0].source_version_id.as_deref(),
            Some(lyrics.id.as_str())
        );

        let after = releases::get(&conn, &release).unwrap().unwrap();
        assert_eq!(
            (after.work_id.as_str(), after.kind.as_str()),
            (audio.id.as_str(), TARGET_DOOR)
        );
        assert_eq!(before.scheduled_at, after.scheduled_at);
        assert_eq!(before.released_at, after.released_at);
        assert_eq!(before.url, after.url);
        assert_eq!(before.meta, after.meta, "the text it went out under stays");
        assert_eq!(
            status_of(&conn, &song.id),
            "released",
            "the song went out as its audio"
        );

        assert!(
            doors_of(&conn, &profile_id, "song").is_empty(),
            "the door is off the song"
        );
        let config = profile::config_for(&conn, &profile_id).unwrap();
        assert!(
            config.kind(INSTRUMENTAL).is_none(),
            "nothing was an instrumental, so the kind is gone"
        );
        let lines = crate::journal::list(&conn, &profile_id).unwrap();
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.action == "upgrade.publicationsMoved")
                .count(),
            1
        );
    }

    /// A video named after the song and the door is the song's audio release
    /// made before the kind existed: it becomes one, keeps its versions and
    /// its own release, and takes the song's. Two releases through one door
    /// are the owner's to settle, and the history says so.
    #[test]
    fn a_work_named_after_the_song_and_the_door_becomes_its_audio() {
        let mut conn = crate::db::open_in_memory().unwrap();
        let profile_id = older_workspace(&conn);
        let song = fixtures::song(&conn, &profile_id, "Harbour Lights");
        let named = fixtures::video(&conn, &profile_id, "Harbour lights — аудио-релиз");
        fixtures::version(&conn, &named.id, "plot", "one frame, one loop");
        let own = fixtures::release(&conn, &named.id, "youtube", Some("2026-09-19"));
        let planned = audio_release(&conn, &song.id);
        fixtures::song(&conn, &profile_id, "Quiet harbour");
        let unrelated = fixtures::video(&conn, &profile_id, "Quiet harbour — clip");

        let report = upgrade(&mut conn).unwrap();

        assert_eq!((report.moved, report.created, report.converted), (1, 0, 1));
        assert_eq!(report.disputed, vec!["Harbour Lights".to_owned()]);
        let converted = work::get(&conn, &named.id).unwrap().unwrap();
        assert_eq!(converted.kind, TARGET_KIND);
        assert_eq!(converted.meta.get("variant"), Some(&json!("original")));
        assert_eq!(
            link::sources(&conn, &named.id).unwrap()[0].source_id,
            song.id
        );
        assert_eq!(
            crate::work::version::list(&conn, &named.id).unwrap().len(),
            1,
            "its concept stays: the audio kind has the role"
        );
        for id in [&own.id, &planned] {
            let found = releases::get(&conn, id).unwrap().unwrap();
            assert_eq!(
                (found.work_id.as_str(), found.kind.as_str()),
                (named.id.as_str(), TARGET_DOOR)
            );
        }
        assert_eq!(
            work::get(&conn, &unrelated.id).unwrap().unwrap().kind,
            "video",
            "a clip of another song, named as a clip, is left alone"
        );
        assert_eq!(works_of(&conn, &profile_id, TARGET_KIND).len(), 1);
        let warnings: Vec<_> = crate::journal::list(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .filter(|line| line.action == "upgrade.publicationDisputed")
            .collect();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].params["title"], "Harbour Lights");
    }

    /// A song with no release and an audio release made as a video still
    /// gets it named as its own: the move is about what the song's audio
    /// release is, not only about where the releases go.
    #[test]
    fn a_named_audio_is_claimed_even_where_the_song_planned_nothing() {
        let mut conn = crate::db::open_in_memory().unwrap();
        let profile_id = older_workspace(&conn);
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let named = fixtures::video(&conn, &profile_id, "Harbour lights - Audio release");

        let report = upgrade(&mut conn).unwrap();

        assert_eq!((report.moved, report.converted), (0, 1));
        assert_eq!(
            work::get(&conn, &named.id).unwrap().unwrap().kind,
            TARGET_KIND
        );
        assert_eq!(
            link::sources(&conn, &named.id).unwrap()[0].source_id,
            song.id
        );
    }

    /// A work that cannot become audio - it goes out through a door audio
    /// does not have - stays as it is and is named for the owner.
    #[test]
    fn a_named_work_with_a_door_audio_lacks_is_listed_not_converted() {
        let mut conn = crate::db::open_in_memory().unwrap();
        let profile_id = older_workspace(&conn);
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let named = fixtures::video(&conn, &profile_id, "Harbour lights — audio");
        fixtures::release(&conn, &named.id, "premiere", None);
        audio_release(&conn, &song.id);

        let report = upgrade(&mut conn).unwrap();

        assert_eq!(report.converted, 0);
        assert_eq!(report.created, 1, "the song's release still finds a home");
        assert_eq!(report.disputed, vec!["Harbour lights".to_owned()]);
        assert_eq!(work::get(&conn, &named.id).unwrap().unwrap().kind, "video");
    }

    /// A clip made before a link could say where from, named the way kilna
    /// names a clip of a song, is linked to the song; one named otherwise,
    /// or already linked to something, is left as it is.
    #[test]
    fn a_publication_named_as_kilna_names_it_is_linked_to_its_song() {
        let mut conn = crate::db::open_in_memory().unwrap();
        let profile_id = older_workspace(&conn);
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let clip = fixtures::video(&conn, &profile_id, "Harbour lights — клип");
        let short = fixtures::work(&conn, &profile_id, "short", "Harbour lights · short 4");
        let other = fixtures::video(&conn, &profile_id, "Harbour lights at night");
        let near = fixtures::video(&conn, &profile_id, "Harbour lightship — clip");

        let report = upgrade(&mut conn).unwrap();

        assert_eq!(report.linked, 2);
        for made in [&clip.id, &short.id] {
            assert_eq!(link::sources(&conn, made).unwrap()[0].source_id, song.id);
        }
        for left in [&other.id, &near.id] {
            assert!(link::sources(&conn, left).unwrap().is_empty());
        }
    }

    #[test]
    fn running_the_move_again_changes_nothing() {
        let mut conn = crate::db::open_in_memory().unwrap();
        let profile_id = older_workspace(&conn);
        let song = fixtures::song(&conn, &profile_id, "Twice");
        audio_release(&conn, &song.id);
        upgrade(&mut conn).unwrap();
        let row = config_row(&conn, &profile_id);
        let works = count_works(&conn);

        assert_eq!(upgrade(&mut conn).unwrap(), Report::default());

        assert_eq!(
            config_row(&conn, &profile_id),
            row,
            "the document is not rewritten for nothing"
        );
        assert_eq!(count_works(&conn), works);
    }

    /// A workspace with an instrumental keeps the kind: they are someone's
    /// works. And a fresh workspace has nothing to move at all.
    #[test]
    fn instrumentals_keep_their_kind_and_a_fresh_workspace_is_left_alone() {
        let mut conn = crate::db::open_in_memory().unwrap();
        let profile_id = older_workspace(&conn);
        fixtures::work(&conn, &profile_id, INSTRUMENTAL, "Night theme");
        upgrade(&mut conn).unwrap();
        let config = profile::config_for(&conn, &profile_id).unwrap();
        assert!(config.kind(INSTRUMENTAL).is_some());
        assert!(doors_of(&conn, &profile_id, INSTRUMENTAL).is_empty());

        let (mut fresh, fresh_id) = fixtures::workspace();
        let before = config_row(&fresh, &fresh_id);
        assert_eq!(upgrade(&mut fresh).unwrap(), Report::default());
        assert_eq!(config_row(&fresh, &fresh_id), before);
    }
}
