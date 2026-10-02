//! A work that goes out, and the work it goes out for.
//!
//! A song is a thing: its text, its style, its score. It is never in front of
//! anyone as the song. What an audience meets is a clip cut to it, the track
//! on a video platform under one picture, a short - its *publications*, each a
//! work of its own with its own release, files, cover and comments (the
//! owner's decision of 2026-09-27). So a kind of work may have no door of its
//! own at all, and from v0.86 the song has none: its facts - booked, gone out -
//! are the facts of what was made from it (`work::status::speaking_for`). And
//! a publication goes out once (ADR 0051): another place is another
//! publication.
//!
//! This module answers three questions. What are a work's publications,
//! where does each stand, and which of them says where the work stands
//! ([`of`]). What is a publication all made from - the song, for a short cut
//! from its clip ([`origin`]) - which names it ([`made_title`]) and titles
//! what it goes out under (`{origin}`). And, once: how does a workspace whose
//! publications kilna named "Song — clip" come over to the names of v0.90,
//! "Song (video)" ([`upgrade`]).

use std::collections::{BTreeSet, HashMap};

use rusqlite::{Connection, params};
use serde::Serialize;
use serde_json::Value;

use crate::error::Result;
use crate::journal::{self, Record};
use crate::link;
use crate::profile::config::{ProfileConfig, SOURCE_LOCALE};
use crate::release;
use crate::time::now;
use crate::work::{self, Work};

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
    /// Its release, when one is planned: where it goes out, the day and
    /// whether it went (v0.90).
    pub release: Option<Going>,
    /// The comments under it that are not archived, and how many of those
    /// still wait for an answer.
    pub comments: i64,
    pub comments_waiting: i64,
    pub created_at: String,
}

/// A publication's one release, as a line about the publication shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct Going {
    pub id: String,
    /// The door: where it goes out.
    pub kind: String,
    pub status: String,
    pub scheduled_at: Option<String>,
    pub scheduled_time: Option<String>,
    pub released_at: Option<String>,
    pub url: Option<String>,
}

impl Going {
    fn of(release: release::Release) -> Self {
        Self {
            id: release.id,
            kind: release.kind,
            status: release.status,
            scheduled_at: release.scheduled_at,
            scheduled_time: release.scheduled_time,
            released_at: release.released_at,
            url: release.url,
        }
    }

    fn went_out(&self) -> bool {
        self.status == release::RELEASED
    }
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
        let going = release::of_work(conn, &id)?.map(Going::of);
        let (comments, comments_waiting) = crate::comment::count_for_work(conn, &id)?;
        items.push(Publication {
            work_id: made.id,
            title: made.title,
            kind: made.kind,
            status: made.status,
            stage: made.stage,
            depth,
            via,
            release: going,
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
        .filter_map(|item| {
            let going = item.release.as_ref().filter(|going| going.went_out())?;
            Some((item, going.released_at.clone()?))
        })
        .max_by(|a, b| a.1.cmp(&b.1));
    let booked = items
        .iter()
        .filter_map(|item| {
            let going = item.release.as_ref().filter(|going| !going.went_out())?;
            Some((item, going.scheduled_at.clone()?))
        })
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

/// What `work_id` is all made from: the nearest work up the chain "made
/// from" that never goes out itself - the song, for its clip, its audio and a
/// short cut from the clip - or else the furthest one up. None for a work
/// made from nothing.
///
/// One answer for every place that asks: the title a cover letters, the
/// neighbours on an idea board, the `{origin}` a release's title reads, the
/// name kilna gives what it makes.
pub fn origin(conn: &Connection, config: &ProfileConfig, work_id: &str) -> Result<Option<Work>> {
    let mut furthest = None;
    for id in link::ancestors(conn, work_id)? {
        let Some(found) = work::get(conn, &id)? else {
            continue;
        };
        let doorless = !config.vocabulary(&found.kind).has_doors();
        furthest = Some(found);
        if doorless {
            break;
        }
    }
    Ok(furthest)
}

/// What `{origin}` reads for `work`: the title of what it is all made from,
/// or its own title when it is made from nothing - an action about a
/// standalone video still has a video to name.
pub fn origin_title(conn: &Connection, config: &ProfileConfig, work: &Work) -> Result<String> {
    Ok(origin(conn, config, &work.id)?.map_or_else(|| work.title.clone(), |found| found.title))
}

/// The title a work of `kind` takes when it is made from `source`: the
/// kind's `made_title` read against the title of what it is all made from,
/// numbered among the works of that kind already made from it - "Song
/// (short 3)" for the third short, whether it was cut from the clip or the
/// audio - and never a title one of them already has.
pub fn made_title(
    conn: &Connection,
    config: &ProfileConfig,
    kind: &str,
    source: &Work,
    locale: &str,
) -> Result<String> {
    // The source is the origin itself when it never goes out (a song), or
    // stands on one (a short cut from a clip).
    let root = if config.vocabulary(&source.kind).has_doors() {
        origin(conn, config, &source.id)?.unwrap_or_else(|| source.clone())
    } else {
        source.clone()
    };
    let mut siblings: BTreeSet<String> = BTreeSet::new();
    for (id, _) in link::descendants(conn, &root.id)? {
        if let Some(found) = work::get(conn, &id)?
            && found.kind == kind
        {
            siblings.insert(found.title);
        }
    }
    let vocabulary = config.vocabulary(kind);
    let mut number = siblings.len() + 1;
    loop {
        let title = vocabulary.title_made_from(&root.title, locale, number);
        if !siblings.contains(&title) {
            return Ok(title);
        }
        number += 1;
    }
}

/// The key of the shipped Studio profile - the only one the renaming reads.
const STUDIO: &str = "music";

/// What the Studio profile named the works it made until v0.90, in each
/// language it shipped them in. A stored kind still saying one of these is
/// one the owner never renamed, and its works are named the new way.
const NAMED_BEFORE_V090: [(&str, [&str; 2]); 3] = [
    ("video", ["{title} — clip", "{title} — клип"]),
    ("audio", ["{title} — audio", "{title} — аудио"]),
    ("short", ["{title} · short {n}", "{title} · шортс {n}"]),
];

/// What the renaming did, for the history and for a test to read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Publications whose title changed.
    pub renamed: usize,
    /// Release fields that gained the tail their door keeps.
    pub tails: usize,
    /// Titles that changed on a release that already went out: the place it
    /// went to still shows the old one, and only the owner can change it
    /// there.
    pub on_platforms: Vec<PlatformRename>,
}

/// A title to change by hand where a release went out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformRename {
    pub work_id: String,
    pub title: String,
    pub was: String,
    pub now: String,
    pub url: Option<String>,
}

/// Name every publication of the Studio profile the way v0.90 does - "Song
/// (video)", "Song (audio)", "Song (short 2)", one string for every language,
/// numbered from the second in the order they went out - and give the fields
/// of every release the tail their door keeps: an audio track's title on a
/// video platform ends with " (audio)". Releases that already went out are
/// listed in the history, title by title, for the owner to change where they
/// went; kilna does not reach into a platform (decision of 2026-10-01).
///
/// Runs when a workspace opens, after the profiles are seeded - the
/// carry-forward is what gives the stored fields their tails - and only while
/// a stored kind still names its works the way kilna did before v0.90: a
/// kind the owner renamed keeps the owner's names, and a workspace that came
/// over once does not come over again. One transaction per profile, and not
/// written to the operation log, for ADR 0030's reason: it is what this
/// version means by a name, not something a person did.
pub fn upgrade(conn: &mut Connection) -> Result<Report> {
    let profiles: Vec<(String, String)> = conn
        .prepare("SELECT id, config FROM profile WHERE key = ?1")?
        .query_map(params![STUDIO], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let Some(shipped) = crate::profile::builtin()?
        .into_iter()
        .find(|profile| profile.key == STUDIO)
        .map(|profile| profile.config)
    else {
        return Ok(Report::default());
    };

    let mut total = Report::default();
    for (profile_id, raw) in profiles {
        let mut config: ProfileConfig = serde_json::from_str(&raw)?;
        let kinds = rename_templates(&mut config, &shipped);
        if kinds.is_empty() {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute(
            "UPDATE profile SET config = ?2, updated_at = ?3 WHERE id = ?1",
            params![profile_id, serde_json::to_string(&config)?, now()],
        )?;
        let mut report = Report::default();
        for kind in &kinds {
            report.renamed += rename_publications(&tx, &profile_id, &config, kind)?;
        }
        keep_tails(&tx, &profile_id, &config, &mut report)?;

        if report.renamed > 0 || report.tails > 0 {
            journal::record(
                &tx,
                &profile_id,
                Record::new("upgrade.publicationsRenamed")
                    .param("count", report.renamed as i64)
                    .param("tails", report.tails as i64),
            );
        }
        for change in &report.on_platforms {
            let mut record = Record::new("upgrade.renameOnPlatform")
                .param("title", change.title.clone())
                .param("was", change.was.clone())
                .param("now", change.now.clone())
                .about("work", change.work_id.clone())
                .warn();
            if let Some(url) = &change.url {
                record = record.param("url", url.clone());
            }
            journal::record(&tx, &profile_id, record);
        }
        tx.commit()?;

        total.renamed += report.renamed;
        total.tails += report.tails;
        total.on_platforms.extend(report.on_platforms);
    }
    Ok(total)
}

/// Give every stored kind still naming its works the pre-v0.90 way the
/// shipped name, and say which kinds those were.
fn rename_templates(config: &mut ProfileConfig, shipped: &ProfileConfig) -> Vec<String> {
    let mut kinds = Vec::new();
    for kind in &mut config.work_kinds {
        let Some((_, before)) = NAMED_BEFORE_V090.iter().find(|(key, _)| *key == kind.key) else {
            continue;
        };
        let untouched = kind.made_title.as_ref().is_some_and(|label| {
            let words = label.words();
            !words.is_empty() && words.iter().all(|word| before.contains(word))
        });
        let Some(now) = shipped
            .kind(&kind.key)
            .and_then(|found| found.made_title.clone())
        else {
            continue;
        };
        if untouched && kind.made_title.as_ref() != Some(&now) {
            kind.made_title = Some(now);
            kinds.push(kind.key.clone());
        }
    }
    kinds
}

/// Title every publication of `kind` that is made from something by the
/// kind's name for it, numbered among those made from the same origin in the
/// order they went out - or are booked, or were made. How many changed.
fn rename_publications(
    conn: &Connection,
    profile_id: &str,
    config: &ProfileConfig,
    kind: &str,
) -> Result<usize> {
    let rows: Vec<(String, String)> = conn
        .prepare(
            "SELECT w.id, w.title FROM work w LEFT JOIN release r ON r.work_id = w.id
              WHERE w.profile_id = ?1 AND w.kind = ?2
              ORDER BY coalesce(r.released_at, r.scheduled_at, w.created_at), w.created_at, w.rowid",
        )?
        .query_map(params![profile_id, kind], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let vocabulary = config.vocabulary(kind);
    let mut numbers: HashMap<String, usize> = HashMap::new();
    let mut renamed = 0;
    let at = now();
    for (id, title) in rows {
        let Some(root) = origin(conn, config, &id)? else {
            continue;
        };
        let number = numbers.entry(root.id.clone()).or_default();
        *number += 1;
        let named = vocabulary.title_made_from(&root.title, SOURCE_LOCALE, *number);
        if named != title {
            conn.execute(
                "UPDATE work SET title = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, named, at],
            )?;
            renamed += 1;
        }
    }
    Ok(renamed)
}

/// Every release field whose door keeps a tail, given it - and, for a release
/// that already went out, a line for the owner saying what to change where it
/// went.
fn keep_tails(
    conn: &Connection,
    profile_id: &str,
    config: &ProfileConfig,
    report: &mut Report,
) -> Result<()> {
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        String,
    )> = conn
        .prepare(
            "SELECT r.id, w.id, w.title, w.kind, r.kind, r.meta, r.url, r.status FROM release r
               JOIN work w ON w.id = r.work_id
              WHERE w.profile_id = ?1",
        )?
        .query_map(params![profile_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let at = now();
    for (release_id, work_id, title, kind, door, raw, url, status) in rows {
        let Some(door) = config
            .vocabulary(&kind)
            .release_kinds
            .iter()
            .find(|candidate| candidate.key == door)
        else {
            continue;
        };
        let mut meta: serde_json::Map<String, Value> = serde_json::from_str(&raw)?;
        let mut changed = false;
        for field in door.fields.iter().filter(|field| field.suffix().is_some()) {
            let Some(was) = meta
                .get(&field.key)
                .and_then(Value::as_str)
                .map(str::to_owned)
            else {
                continue;
            };
            let kept = field.with_suffix(&was);
            if kept == was {
                continue;
            }
            meta.insert(field.key.clone(), Value::String(kept.clone()));
            changed = true;
            report.tails += 1;
            if status == release::RELEASED {
                report.on_platforms.push(PlatformRename {
                    work_id: work_id.clone(),
                    title: title.clone(),
                    was,
                    now: kept,
                    url: url.clone(),
                });
            }
        }
        if changed {
            conn.execute(
                "UPDATE release SET meta = ?2, updated_at = ?3 WHERE id = ?1",
                params![release_id, Value::Object(meta).to_string(), at],
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::fixtures;
    use crate::link::NewLink;
    use crate::profile::{self, config::Label};
    use crate::release::{self as releases, ReleasePatch};
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

    fn store(conn: &Connection, profile_id: &str, config: &ProfileConfig) {
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![profile_id, serde_json::to_string(config).unwrap()],
        )
        .unwrap();
    }

    fn title_of(conn: &Connection, work_id: &str) -> String {
        work::get(conn, work_id).unwrap().unwrap().title
    }

    /// A short cut from a clip is all made from the song, and so is the
    /// clip; the song is made from nothing and reads its own title.
    #[test]
    fn the_origin_is_the_song_up_the_chain() {
        let (conn, profile_id) = fixtures::workspace();
        let config = profile::config_for(&conn, &profile_id).unwrap();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let clip = fixtures::video(&conn, &profile_id, "Harbour lights (video)");
        let short = fixtures::work(&conn, &profile_id, "short", "Harbour lights (short)");
        link(&conn, &profile_id, &clip.id, &song.id);
        link(&conn, &profile_id, &short.id, &clip.id);

        let origin_of = |id: &str| origin(&conn, &config, id).unwrap().map(|found| found.id);
        assert_eq!(origin_of(&short.id), Some(song.id.clone()));
        assert_eq!(origin_of(&clip.id), Some(song.id.clone()));
        assert_eq!(origin_of(&song.id), None);
        assert_eq!(
            origin_title(&conn, &config, &short).unwrap(),
            "Harbour lights"
        );
        assert_eq!(
            origin_title(&conn, &config, &song).unwrap(),
            "Harbour lights"
        );
    }

    /// One name in every language, numbered from the second among the works
    /// of the kind made from the same song - whichever publication a short
    /// was cut from.
    #[test]
    fn a_publication_is_named_by_its_kind_and_numbered_from_the_second() {
        let (conn, profile_id) = fixtures::workspace();
        let config = profile::config_for(&conn, &profile_id).unwrap();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        let name = |kind: &str, source: &Work, locale: &str| {
            made_title(&conn, &config, kind, source, locale).unwrap()
        };
        assert_eq!(name("video", &song, "ru"), "Tide (video)");
        assert_eq!(name("audio", &song, "en"), "Tide (audio)");

        let clip = fixtures::video(&conn, &profile_id, "Tide (video)");
        link(&conn, &profile_id, &clip.id, &song.id);
        assert_eq!(
            name("short", &clip, "ru"),
            "Tide (short)",
            "the song's name, not the clip's"
        );
        let first = fixtures::work(&conn, &profile_id, "short", "Tide (short)");
        link(&conn, &profile_id, &first.id, &clip.id);
        assert_eq!(
            name("short", &song, "en"),
            "Tide (short 2)",
            "counted under the song, cut from the clip or not"
        );
        assert_eq!(name("video", &song, "en"), "Tide (video 2)");
    }

    /// A short thrown away leaves a gap; the next one does not take the
    /// name of one still there.
    #[test]
    fn a_made_title_is_never_one_already_taken() {
        let (conn, profile_id) = fixtures::workspace();
        let config = profile::config_for(&conn, &profile_id).unwrap();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        let third = fixtures::work(&conn, &profile_id, "short", "Tide (short 2)");
        link(&conn, &profile_id, &third.id, &song.id);

        assert_eq!(
            made_title(&conn, &config, "short", &song, "en").unwrap(),
            "Tide (short 3)"
        );
    }

    /// The owner's workspace as v0.89 left it: kilna's old names, a short
    /// cut from the clip and one from the audio, an audio that went out
    /// under a title with no tail.
    fn before_v090(conn: &Connection, profile_id: &str) {
        let mut config = profile::config_for(conn, profile_id).unwrap();
        for (key, [en, ru]) in NAMED_BEFORE_V090 {
            let kind = config
                .work_kinds
                .iter_mut()
                .find(|kind| kind.key == key)
                .unwrap();
            kind.made_title = Some(Label::PerLocale(BTreeMap::from([
                ("en".to_owned(), en.to_owned()),
                ("ru".to_owned(), ru.to_owned()),
            ])));
        }
        store(conn, profile_id, &config);
    }

    /// Fields as v0.89 wrote them: past the release, which would give them
    /// their tails today.
    fn fields(conn: &Connection, release_id: &str, values: Value) {
        conn.execute(
            "UPDATE release SET meta = ?2 WHERE id = ?1",
            params![release_id, values.to_string()],
        )
        .unwrap();
    }

    fn field(conn: &Connection, release_id: &str, key: &str) -> String {
        releases::get(conn, release_id).unwrap().unwrap().meta[key]
            .as_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn publications_are_renamed_once_and_titles_already_out_are_listed() {
        let (mut conn, profile_id) = fixtures::workspace();
        before_v090(&conn, &profile_id);
        let song = fixtures::song(&conn, &profile_id, "Tide");
        let clip = fixtures::video(&conn, &profile_id, "Tide — клип");
        let audio = fixtures::work(&conn, &profile_id, "audio", "Tide — аудио");
        let late = fixtures::work(&conn, &profile_id, "short", "Tide");
        let early = fixtures::work(&conn, &profile_id, "short", "Tide");
        let alone = fixtures::video(&conn, &profile_id, "A video of its own");
        link(&conn, &profile_id, &clip.id, &song.id);
        link(&conn, &profile_id, &audio.id, &song.id);
        link(&conn, &profile_id, &late.id, &clip.id);
        link(&conn, &profile_id, &early.id, &audio.id);
        fixtures::release(&conn, &late.id, "short", Some("2026-10-05"));
        fixtures::release(&conn, &early.id, "short", Some("2026-09-05"));
        let out = fixtures::release(&conn, &audio.id, "youtube", None);
        fields(
            &conn,
            &out.id,
            json!({ "title": "Tide - one line", "description": "words" }),
        );
        releases::update(
            &conn,
            &out.id,
            ReleasePatch {
                url: Some(Some("https://example.com/watch".into())),
                ..ReleasePatch::default()
            },
        )
        .unwrap();
        releases::mark_released(&conn, &out.id, None, Some("2026-09-01".into())).unwrap();
        let planned = fixtures::release(&conn, &clip.id, "youtube", None);
        fields(&conn, &planned.id, json!({ "title": "Tide" }));

        let report = upgrade(&mut conn).unwrap();

        assert_eq!(title_of(&conn, &clip.id), "Tide (video)");
        assert_eq!(title_of(&conn, &audio.id), "Tide (audio)");
        assert_eq!(
            (title_of(&conn, &early.id), title_of(&conn, &late.id)),
            ("Tide (short)".to_owned(), "Tide (short 2)".to_owned()),
            "numbered in the order they go out, across the clip and the audio"
        );
        assert_eq!(
            title_of(&conn, &alone.id),
            "A video of its own",
            "a work made from nothing keeps its name"
        );
        assert_eq!(field(&conn, &out.id, "title"), "Tide - one line (audio)");
        assert_eq!(field(&conn, &out.id, "description"), "words");
        assert_eq!(
            field(&conn, &planned.id, "title"),
            "Tide",
            "a clip keeps no tail"
        );
        assert_eq!(report.renamed, 4);
        assert_eq!(report.tails, 1);
        assert_eq!(
            report.on_platforms,
            vec![PlatformRename {
                work_id: audio.id.clone(),
                title: "Tide (audio)".into(),
                was: "Tide - one line".into(),
                now: "Tide - one line (audio)".into(),
                url: Some("https://example.com/watch".into()),
            }]
        );
        assert!(
            journal::for_entity(&conn, "work", &audio.id)
                .unwrap()
                .iter()
                .any(|entry| entry.action == "upgrade.renameOnPlatform"),
            "the owner finds what to change on the platform in the history"
        );

        // Once over, never again: a title the owner gives afterwards stays.
        work::update(
            &conn,
            &late.id,
            work::WorkPatch {
                title: Some("Tide, the bridge".into()),
                ..work::WorkPatch::default()
            },
        )
        .unwrap();
        assert_eq!(upgrade(&mut conn).unwrap(), Report::default());
        assert_eq!(title_of(&conn, &late.id), "Tide, the bridge");
    }

    /// A kind the owner named their own way keeps the owner's names; the
    /// others still come over.
    #[test]
    fn a_name_the_owner_wrote_is_left_alone() {
        let (mut conn, profile_id) = fixtures::workspace();
        before_v090(&conn, &profile_id);
        let mut config = profile::config_for(&conn, &profile_id).unwrap();
        config
            .work_kinds
            .iter_mut()
            .find(|kind| kind.key == "video")
            .unwrap()
            .made_title = Some("{title}, the film".into());
        store(&conn, &profile_id, &config);
        let song = fixtures::song(&conn, &profile_id, "Tide");
        let clip = fixtures::video(&conn, &profile_id, "Tide, the film");
        let audio = fixtures::work(&conn, &profile_id, "audio", "Tide — аудио");
        link(&conn, &profile_id, &clip.id, &song.id);
        link(&conn, &profile_id, &audio.id, &song.id);

        upgrade(&mut conn).unwrap();

        assert_eq!(title_of(&conn, &clip.id), "Tide, the film");
        assert_eq!(title_of(&conn, &audio.id), "Tide (audio)");
        let stored = profile::config_for(&conn, &profile_id).unwrap();
        assert_eq!(
            stored.kind("video").unwrap().made_title,
            Some("{title}, the film".into())
        );
    }
}
