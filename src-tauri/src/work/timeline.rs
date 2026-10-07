//! A work's history on one axis: when it was begun, every version, every
//! score, every publication made from it and when each went out, and the
//! lines of the journal the rows themselves do not hold (ADR 0056).
//!
//! Read off the rows, not off the journal. The journal is the feed of what
//! happened and forgets on purpose - a line read a week ago is swept - and a
//! work brought in from elsewhere arrives with every version and score and no
//! line about any of them. A version's `created_at`, a score's `scored_at`, a
//! release's day are kept as long as the thing is, so the axis is drawn from
//! those, and the journal adds only what no row remembers: a rename, a status
//! that moved, a scene put on the board, a proposal that arrived.

use std::collections::HashMap;

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::error::Result;
use crate::journal::{self, Entry};
use crate::link;
use crate::release;
use crate::score;
use crate::work::{self, version};

/// The journal's lines for things the rows already say, at the same moment:
/// the work was made, a version written, a score given, a release went out.
/// Shown once, from the row, which outlives the line.
const SAID_BY_THE_ROWS: [&str; 4] = [
    "work.created",
    "version.created",
    "score.added",
    "release.released",
];

/// One thing on the axis.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Moment {
    /// The work was made.
    Begun { at: String },
    /// A version was written. `parent` is the revision it was written from,
    /// when that was recorded and is still there.
    Version {
        at: String,
        id: String,
        role: String,
        revision: i64,
        label: Option<String>,
        parent: Option<i64>,
    },
    /// A score was given, to the version it read when it read one.
    Score {
        at: String,
        id: String,
        total: f64,
        tier: Option<String>,
        rater: Option<String>,
        version_id: Option<String>,
        role: Option<String>,
        revision: Option<i64>,
    },
    /// A publication was made from the work - a clip, an audio, a short cut
    /// from the clip (`depth` 2).
    Made {
        at: String,
        work_id: String,
        title: String,
        kind: String,
        depth: i64,
    },
    /// A release on its day: gone out, or booked. The work's own, or one of
    /// what was made from it - a song goes out only as its publications.
    Release {
        at: String,
        id: String,
        work_id: String,
        title: String,
        /// The door: where it goes out.
        kind: String,
        released: bool,
        time: Option<String>,
        url: Option<String>,
    },
    /// A line of the journal no row holds.
    Entry { at: String, entry: Entry },
}

impl Moment {
    /// When it happened, or is booked to: a timestamp, or a bare day for a
    /// release.
    pub fn at(&self) -> &str {
        match self {
            Self::Begun { at }
            | Self::Version { at, .. }
            | Self::Score { at, .. }
            | Self::Made { at, .. }
            | Self::Release { at, .. }
            | Self::Entry { at, .. } => at,
        }
    }
}

/// Everything on a work's axis, latest first: a release booked for next week
/// stands above what happened today.
///
/// A work that does not exist has no history, said as an empty axis rather
/// than an error: the card asks while it may be in the trash.
pub fn of(conn: &Connection, work_id: &str) -> Result<Vec<Moment>> {
    let Some(found) = work::get(conn, work_id)? else {
        return Ok(Vec::new());
    };
    let mut moments = vec![Moment::Begun {
        at: found.created_at.clone(),
    }];

    let versions = version::list(conn, work_id)?;
    let revisions: HashMap<&str, (&str, i64)> = versions
        .iter()
        .map(|v| (v.id.as_str(), (v.role.as_str(), v.revision)))
        .collect();
    for v in &versions {
        moments.push(Moment::Version {
            at: v.created_at.clone(),
            id: v.id.clone(),
            role: v.role.clone(),
            revision: v.revision,
            label: v.label.clone(),
            parent: v
                .parent_version_id
                .as_deref()
                .and_then(|parent| revisions.get(parent))
                .map(|(_, revision)| *revision),
        });
    }

    for s in score::history(conn, work_id)? {
        let read = s
            .version_id
            .as_deref()
            .and_then(|id| revisions.get(id))
            .copied();
        moments.push(Moment::Score {
            at: s.scored_at,
            id: s.id,
            total: s.total,
            tier: s.tier,
            rater: s.rater,
            version_id: s.version_id,
            role: read.map(|(role, _)| role.to_owned()),
            revision: read.map(|(_, revision)| revision),
        });
    }

    if let Some(own) = release::of_work(conn, work_id)? {
        moments.extend(release_moment(own, found.title.clone()));
    }
    for (id, depth) in link::descendants(conn, work_id)? {
        let Some(made) = work::get(conn, &id)? else {
            continue;
        };
        moments.push(Moment::Made {
            at: made.created_at.clone(),
            work_id: made.id.clone(),
            title: made.title.clone(),
            kind: made.kind.clone(),
            depth,
        });
        if let Some(going) = release::of_work(conn, &id)? {
            moments.extend(release_moment(going, made.title));
        }
    }

    for entry in journal::for_entity(conn, "work", work_id)? {
        if SAID_BY_THE_ROWS.contains(&entry.action.as_str()) {
            continue;
        }
        moments.push(Moment::Entry {
            at: entry.created_at.clone(),
            entry,
        });
    }

    // Latest first. A bare day and a timestamp of the same day compare as
    // strings, which puts the day's moments above its release - the window
    // sorts again by its own clock, which is the only side that knows the
    // day a timestamp falls on where the person is.
    moments.sort_by(|a, b| b.at().cmp(a.at()));
    Ok(moments)
}

/// How many moments the History tab lists, without the one every work has -
/// its beginning: a number on every tab of every card says nothing.
pub fn count(conn: &Connection, work_id: &str) -> Result<i64> {
    if work::get(conn, work_id)?.is_none() {
        return Ok(0);
    }
    let skipped = SAID_BY_THE_ROWS
        .iter()
        .map(|action| format!("'{action}'"))
        .collect::<Vec<_>>()
        .join(", ");
    // The page of the journal the tab reads, less the lines of it the rows
    // already say: the tab skips those, so the count does too.
    let entries: i64 = conn.query_row(
        &format!(
            "SELECT count(*) FROM (SELECT action FROM journal
              WHERE entity = 'work' AND entity_id = ?1
              ORDER BY created_at DESC, rowid DESC LIMIT ?2)
             WHERE action NOT IN ({skipped})"
        ),
        params![work_id, journal::PAGE],
        |row| row.get(0),
    )?;
    let rows: i64 = conn.query_row(
        "WITH RECURSIVE down(id, depth) AS (
             SELECT work_id, 1 FROM work_link WHERE source_id = ?1
             UNION
             SELECT l.work_id, down.depth + 1
               FROM work_link l JOIN down ON l.source_id = down.id
              WHERE down.depth < 16
         ),
         made AS (SELECT DISTINCT down.id FROM down JOIN work w ON w.id = down.id
                   WHERE down.id <> ?1)
         SELECT (SELECT count(*) FROM work_version WHERE work_id = ?1)
              + (SELECT count(*) FROM work_score WHERE work_id = ?1)
              + (SELECT count(*) FROM made)
              + (SELECT count(*) FROM release r
                  WHERE (r.work_id = ?1 OR r.work_id IN (SELECT id FROM made))
                    AND ((r.status = ?2 AND r.released_at IS NOT NULL)
                         OR (r.status <> ?2 AND r.scheduled_at IS NOT NULL)))",
        params![work_id, release::RELEASED],
        |row| row.get(0),
    )?;
    Ok(rows + entries)
}

/// A release on the axis, on the day it went out or is booked for. A release
/// with neither - queued, no day yet - has no place on an axis of days.
fn release_moment(release: release::Release, title: String) -> Option<Moment> {
    let released = release.status == release::RELEASED;
    let at = if released {
        release.released_at.clone()
    } else {
        release.scheduled_at.clone()
    }?;
    Some(Moment::Release {
        at,
        id: release.id,
        work_id: release.work_id,
        title,
        kind: release.kind,
        released,
        time: release.scheduled_time,
        url: release.url,
    })
}

#[cfg(test)]
mod tests {
    use rusqlite::params;
    use serde_json::json;

    use super::*;
    use crate::fixtures;
    use crate::journal::Record;
    use crate::link::{self, NewLink};
    use crate::work::version::NewVersion;

    fn at(conn: &Connection, table: &str, column: &str, id: &str, when: &str) {
        conn.execute(
            &format!("UPDATE {table} SET {column} = ?2 WHERE id = ?1"),
            params![id, when],
        )
        .unwrap();
    }

    fn kinds(moments: &[Moment]) -> Vec<&'static str> {
        moments
            .iter()
            .map(|moment| match moment {
                Moment::Begun { .. } => "begun",
                Moment::Version { .. } => "version",
                Moment::Score { .. } => "score",
                Moment::Made { .. } => "made",
                Moment::Release { .. } => "release",
                Moment::Entry { .. } => "entry",
            })
            .collect()
    }

    /// The axis is read off the rows: a work brought in with its versions and
    /// a score, and not one line in the journal about either, still has its
    /// whole history - in order, latest first, and the version it was
    /// written from named by revision.
    #[test]
    fn the_axis_is_read_off_the_rows_latest_first() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        at(
            &conn,
            "work",
            "created_at",
            &song.id,
            "2026-01-01T10:00:00.000Z",
        );
        let first = fixtures::version(&conn, &song.id, "lyrics", "one");
        at(
            &conn,
            "work_version",
            "created_at",
            &first.id,
            "2026-01-02T10:00:00.000Z",
        );
        let second = version::create(
            &conn,
            &song.id,
            NewVersion {
                role: "lyrics".into(),
                body: "one, two".into(),
                parent_version_id: Some(first.id.clone()),
                ..NewVersion::default()
            },
        )
        .unwrap();
        at(
            &conn,
            "work_version",
            "created_at",
            &second.id,
            "2026-01-04T10:00:00.000Z",
        );
        let judged = fixtures::score(&conn, &song.id, json!({ "hook": 7 }));
        at(
            &conn,
            "work_score",
            "scored_at",
            &judged.id,
            "2026-01-05T10:00:00.000Z",
        );

        let moments = of(&conn, &song.id).unwrap();

        assert_eq!(kinds(&moments), ["score", "version", "version", "begun"]);
        match &moments[1] {
            Moment::Version {
                revision, parent, ..
            } => {
                assert_eq!(*revision, 2);
                assert_eq!(*parent, Some(1), "the parent is named by its revision");
            }
            other => panic!("expected the second version, got {other:?}"),
        }
        match &moments[0] {
            Moment::Score { role, revision, .. } => {
                assert_eq!(role.as_deref(), Some("lyrics"));
                assert_eq!(*revision, Some(2), "a score names the version it read");
            }
            other => panic!("expected the score, got {other:?}"),
        }
    }

    /// A song goes out only as what was made from it: the clip made from it
    /// is a moment on its axis, and so is the day the clip goes out - booked
    /// in the future stands above today. A release with no day has no place
    /// on an axis of days.
    #[test]
    fn what_was_made_and_when_it_goes_out_is_on_the_song() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        at(
            &conn,
            "work",
            "created_at",
            &song.id,
            "2026-01-01T10:00:00.000Z",
        );
        let clip = fixtures::video(&conn, &profile_id, "Tide (video)");
        at(
            &conn,
            "work",
            "created_at",
            &clip.id,
            "2026-01-03T10:00:00.000Z",
        );
        let audio = fixtures::work(&conn, &profile_id, "audio", "Tide (audio)");
        at(
            &conn,
            "work",
            "created_at",
            &audio.id,
            "2026-01-02T10:00:00.000Z",
        );
        for made in [&clip.id, &audio.id] {
            link::create(
                &conn,
                &profile_id,
                NewLink {
                    work_id: made.clone(),
                    source_id: song.id.clone(),
                    role: None,
                    source_version_id: None,
                },
            )
            .unwrap();
        }
        fixtures::release(&conn, &clip.id, "youtube", Some("2099-03-01"));
        // Queued, no day: nothing to draw.
        fixtures::release(&conn, &audio.id, "youtube", None);

        let moments = of(&conn, &song.id).unwrap();

        assert_eq!(kinds(&moments), ["release", "made", "made", "begun"]);
        match &moments[0] {
            Moment::Release {
                title,
                released,
                at,
                ..
            } => {
                assert_eq!(title, "Tide (video)");
                assert!(!released, "booked, not gone out");
                assert_eq!(at, "2099-03-01");
            }
            other => panic!("expected the clip's release, got {other:?}"),
        }
    }

    /// The journal adds what no row holds - a rename - and nothing the rows
    /// already say: a version written leaves a line too, but the version is
    /// on the axis already, and twice would be once too often.
    #[test]
    fn the_journal_adds_only_what_the_rows_do_not_hold() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        fixtures::version(&conn, &song.id, "lyrics", "one");
        for action in ["work.created", "version.created", "work.renamed"] {
            journal::record(
                &conn,
                &profile_id,
                Record::new(action).about("work", &song.id),
            );
        }

        let moments = of(&conn, &song.id).unwrap();

        let entries: Vec<&str> = moments
            .iter()
            .filter_map(|moment| match moment {
                Moment::Entry { entry, .. } => Some(entry.action.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(entries, ["work.renamed"]);
        assert_eq!(
            kinds(&moments).iter().filter(|k| **k == "version").count(),
            1
        );
    }

    /// The number beside the tab is the axis less its beginning, by whatever
    /// the axis holds: versions, scores, what was made, days of release, the
    /// journal's own lines.
    #[test]
    fn the_count_is_the_axis_without_its_beginning() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        assert_eq!(
            count(&conn, &song.id).unwrap(),
            0,
            "a new work has only begun"
        );

        fixtures::version(&conn, &song.id, "lyrics", "one");
        fixtures::score(&conn, &song.id, json!({ "hook": 7 }));
        let clip = fixtures::video(&conn, &profile_id, "Tide (video)");
        link::create(
            &conn,
            &profile_id,
            NewLink {
                work_id: clip.id.clone(),
                source_id: song.id.clone(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();
        let going = fixtures::release(&conn, &clip.id, "youtube", Some("2026-03-01"));
        release::mark_released(&conn, &going.id, None, Some("2026-03-01".into())).unwrap();
        for action in ["version.created", "work.renamed"] {
            journal::record(
                &conn,
                &profile_id,
                Record::new(action).about("work", &song.id),
            );
        }

        let moments = of(&conn, &song.id).unwrap();
        assert_eq!(
            count(&conn, &song.id).unwrap(),
            moments.len() as i64 - 1,
            "{:?}",
            kinds(&moments)
        );
        assert_eq!(count(&conn, &song.id).unwrap(), 5);
        assert_eq!(count(&conn, "no-such-work").unwrap(), 0);
        assert!(of(&conn, "no-such-work").unwrap().is_empty());
    }
}
