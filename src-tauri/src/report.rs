//! What a week came to, and what happened since the person last looked
//! (v0.105).
//!
//! The week is read from the operations log, not from the history: the
//! history is written for a person to read and forgets what was read a week
//! ago on purpose, while the log keeps every gesture for as long as the
//! workspace lives. A week's report is a count of the gestures that made
//! something - a work, a version, a score, a release out, a note, a trial -
//! with the works they made it for, by their titles now. The words of the
//! report are the window's: this returns the facts, and the window writes
//! them out in its language.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};

/// A work the week touched, and how many times.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Touched {
    pub work_id: String,
    /// Its title now, or as it was made when it is gone.
    pub title: String,
    pub count: i64,
}

/// What a week came to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct WeekReport {
    /// The first day, and the day after the last: `[from, until)`.
    pub from: String,
    pub until: String,
    /// Works made.
    pub made: Vec<Touched>,
    /// Versions written, by work.
    pub versions: Vec<Touched>,
    /// Scores given, by work.
    pub scored: Vec<Touched>,
    /// Releases marked out, by work.
    pub released: Vec<Touched>,
    pub notes: i64,
    pub trials: i64,
}

/// The seven days from `from`, a date.
pub fn week(conn: &Connection, profile_id: &str, from: &str) -> Result<WeekReport> {
    let format = time::macros::format_description!("[year]-[month]-[day]");
    let start = time::Date::parse(from, &format)
        .map_err(|_| Error::refused("report.badDate").param("value", from))?;
    let end = start + time::Duration::days(7);
    let until = format!(
        "{:04}-{:02}-{:02}",
        end.year(),
        u8::from(end.month()),
        end.day()
    );

    let mut statement = conn.prepare(
        "SELECT kind, params FROM operation \
         WHERE profile_id = ?1 AND recorded_at >= ?2 AND recorded_at < ?3 \
           AND kind IN ('work.create', 'version.create', 'score.create', \
                        'release.markReleased', 'note.create', 'trial.create') \
         ORDER BY seq",
    )?;
    let rows = statement
        .query_map(params![profile_id, from, until], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut made = Tally::default();
    let mut versions = Tally::default();
    let mut scored = Tally::default();
    let mut released = Tally::default();
    let (mut notes, mut trials) = (0, 0);
    for (kind, raw) in rows {
        let params: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
        let text = |key: &str| params.get(key).and_then(Value::as_str).map(str::to_owned);
        match kind.as_str() {
            "work.create" => {
                if let Some(id) = text("id") {
                    let made_as = params
                        .pointer("/work/title")
                        .and_then(Value::as_str)
                        .map(str::to_owned);
                    made.add(conn, &id, made_as)?;
                }
            }
            "version.create" => {
                if let Some(id) = text("workId") {
                    versions.add(conn, &id, None)?;
                }
            }
            "score.create" => {
                if let Some(id) = text("workId") {
                    scored.add(conn, &id, None)?;
                }
            }
            "release.markReleased" => {
                let work = match text("id") {
                    Some(release) => conn
                        .query_row(
                            "SELECT work_id FROM release WHERE id = ?1",
                            params![release],
                            |row| row.get::<_, String>(0),
                        )
                        .optional()?,
                    None => None,
                };
                if let Some(id) = work {
                    released.add(conn, &id, None)?;
                }
            }
            "note.create" => notes += 1,
            "trial.create" => trials += 1,
            _ => {}
        }
    }

    Ok(WeekReport {
        from: from.to_owned(),
        until,
        made: made.done(),
        versions: versions.done(),
        scored: scored.done(),
        released: released.done(),
        notes,
        trials,
    })
}

/// Works counted in the order they were first touched.
#[derive(Default)]
struct Tally {
    order: Vec<String>,
    by_work: BTreeMap<String, Touched>,
}

impl Tally {
    fn add(&mut self, conn: &Connection, work_id: &str, made_as: Option<String>) -> Result<()> {
        if let Some(touched) = self.by_work.get_mut(work_id) {
            touched.count += 1;
            return Ok(());
        }
        let title = crate::work::get(conn, work_id)?
            .map(|work| work.title)
            .or(made_as)
            .unwrap_or_default();
        self.order.push(work_id.to_owned());
        self.by_work.insert(
            work_id.to_owned(),
            Touched {
                work_id: work_id.to_owned(),
                title,
                count: 1,
            },
        );
        Ok(())
    }

    fn done(mut self) -> Vec<Touched> {
        self.order
            .iter()
            .filter_map(|id| self.by_work.remove(id))
            .collect()
    }
}

/// What happened since the person last caught up with the history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct SinceVisit {
    /// When the history was last marked seen; `None` when it never was.
    pub since: Option<String>,
    /// Entries of the history nobody has read.
    pub unread: i64,
    /// Assistant runs that ended since then, by how they ended.
    pub runs_done: i64,
    pub runs_failed: i64,
}

/// What happened since the history was last marked seen: the unread entries,
/// and the assistant's runs that ended since - which the history does not
/// list one by one, and which a person away for a day wants counted.
pub fn since_visit(conn: &Connection, profile_id: &str) -> Result<SinceVisit> {
    let since: Option<String> = conn.query_row(
        "SELECT max(read_at) FROM journal WHERE profile_id = ?1",
        params![profile_id],
        |row| row.get(0),
    )?;
    let unread: i64 = conn.query_row(
        "SELECT count(*) FROM journal WHERE profile_id = ?1 AND read_at IS NULL",
        params![profile_id],
        |row| row.get(0),
    )?;
    let count = |state: &str| -> Result<i64> {
        Ok(conn.query_row(
            "SELECT count(*) FROM chat_run r JOIN chat c ON c.id = r.chat_id \
             WHERE c.profile_id = ?1 AND r.state = ?2 AND r.ended_at IS NOT NULL \
               AND (?3 IS NULL OR r.ended_at > ?3)",
            params![profile_id, state, since],
            |row| row.get(0),
        )?)
    };
    Ok(SinceVisit {
        unread,
        runs_done: count("done")?,
        runs_failed: count("failed")?,
        since,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{actions, fixtures};

    /// A week counts what was made in it, by work, and nothing from before.
    #[test]
    fn a_week_counts_what_was_made_in_it() {
        let (conn, profile_id) = fixtures::workspace();
        let song = actions::work::create(
            &conn,
            crate::work::NewWork {
                kind: "song".into(),
                title: "Tide".into(),
                ..Default::default()
            },
        )
        .unwrap();
        for body in ["first", "second"] {
            actions::version::create(
                &conn,
                &song.id,
                crate::work::version::NewVersion {
                    role: "lyrics".into(),
                    body: body.into(),
                    label: None,
                    meta: None,
                    make_current: true,
                    parent_version_id: None,
                    trial_id: None,
                },
            )
            .unwrap();
        }
        let today = crate::time::now()[..10].to_owned();

        let report = week(&conn, &profile_id, &today).unwrap();

        assert_eq!(report.made.len(), 1);
        assert_eq!(report.made[0].title, "Tide");
        assert_eq!(report.versions[0].count, 2, "two versions of one work");
        assert!(report.scored.is_empty());

        let earlier = week(&conn, &profile_id, "2020-01-06").unwrap();
        assert!(earlier.made.is_empty() && earlier.versions.is_empty());
        assert_eq!(earlier.until, "2020-01-13");
    }

    #[test]
    fn a_week_from_no_day_is_refused() {
        let (conn, profile_id) = fixtures::workspace();
        let refused = week(&conn, &profile_id, "last monday").unwrap_err();
        assert_eq!(refused.refusal().map(|r| r.code), Some("report.badDate"));
    }

    /// Since the history was last seen: the entries still unread, counted
    /// with nothing having run.
    #[test]
    fn since_the_last_visit_counts_what_is_unread() {
        let (conn, profile_id) = fixtures::workspace();
        crate::journal::record(
            &conn,
            &profile_id,
            crate::journal::Record::new("assistant.batchStarted").param("count", 2),
        );

        let since = since_visit(&conn, &profile_id).unwrap();

        assert_eq!(since.unread, 1);
        assert_eq!((since.runs_done, since.runs_failed), (0, 0));
    }
}
