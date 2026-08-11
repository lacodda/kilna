use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::time::now;

/// What ships, where and when. The predecessor spread this across three tables;
/// here it is one row per unit of release.
#[derive(Debug, Clone, Serialize)]
pub struct Release {
    pub id: String,
    pub work_id: String,
    pub kind: String,
    pub status: String,
    pub title: Option<String>,
    /// Calendar slot. `None` means queued but unscheduled.
    pub scheduled_at: Option<String>,
    pub released_at: Option<String>,
    pub url: Option<String>,
    pub meta: Map<String, Value>,
    pub created_at: String,
    pub updated_at: String,
}

/// A release with the context the calendar needs to draw it.
#[derive(Debug, Clone, Serialize)]
pub struct ScheduledRelease {
    #[serde(flatten)]
    pub release: Release,
    pub work_title: String,
    /// Latest total for the work, so a slot can be judged against its neighbours.
    pub total: Option<f64>,
    pub tier: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewRelease {
    pub work_id: String,
    pub kind: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub scheduled_at: Option<String>,
    #[serde(default)]
    pub meta: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReleasePatch {
    pub kind: Option<String>,
    pub status: Option<String>,
    pub title: Option<Option<String>>,
    pub scheduled_at: Option<Option<String>>,
    pub url: Option<Option<String>>,
    pub meta: Option<Map<String, Value>>,
}

/// What happened when a slot was claimed.
#[derive(Debug, Clone, Serialize)]
pub struct Scheduling {
    pub release: Release,
    /// The release that lost the slot, if the new one displaced something.
    pub displaced: Option<Release>,
}

/// Status values a release moves through. These are fixed rather than profile
/// vocabulary: they describe the mechanism, not the craft.
pub const PLANNED: &str = "planned";
pub const RELEASED: &str = "released";

const SELECT_RELEASE: &str = "SELECT id, work_id, kind, status, title, scheduled_at, released_at, \
     url, meta, created_at, updated_at FROM release";

pub fn create(conn: &Connection, new: NewRelease) -> Result<Release> {
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM work WHERE id = ?1",
            params![new.work_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !exists {
        return Err(Error::Other(format!("no work with id `{}`", new.work_id)));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now();

    conn.execute(
        "INSERT INTO release (id, work_id, kind, status, title, scheduled_at, meta, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![
            id,
            new.work_id,
            new.kind,
            PLANNED,
            new.title,
            new.scheduled_at,
            Value::Object(new.meta.unwrap_or_default()).to_string(),
            timestamp,
        ],
    )?;

    get(conn, &id)?.ok_or_else(|| Error::Other("the release vanished after insert".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Release>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_RELEASE} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?;

    raw.map(RawRelease::into_release).transpose()
}

/// Put a release in a slot.
///
/// A slot holds one release. When it is taken, the stronger work keeps it and
/// the weaker one is returned to the queue rather than deleted — losing a slot
/// must never lose the plan for the work. Strength is the latest total; an
/// unscored release never displaces a scored one.
pub fn schedule(conn: &mut Connection, id: &str, slot: &str) -> Result<Scheduling> {
    let tx = conn.transaction()?;

    let Some(release) = tx
        .query_row(
            &format!("{SELECT_RELEASE} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?
        .map(RawRelease::into_release)
        .transpose()?
    else {
        return Err(unknown_release(id));
    };

    // Whoever already holds the slot, ignoring this release itself.
    let occupant = tx
        .query_row(
            &format!("{SELECT_RELEASE} WHERE scheduled_at = ?1 AND id <> ?2 AND status = '{PLANNED}' LIMIT 1"),
            params![slot, id],
            read_row,
        )
        .optional()?
        .map(RawRelease::into_release)
        .transpose()?;

    let timestamp = now();
    let mut displaced = None;

    if let Some(occupant) = occupant {
        let challenger = strength(&tx, &release.work_id)?;
        let holder = strength(&tx, &occupant.work_id)?;

        // Ties go to the release already in the slot: a plan should not move
        // without a reason to move it.
        if challenger.unwrap_or(f64::NEG_INFINITY) <= holder.unwrap_or(f64::NEG_INFINITY) {
            return Err(Error::Other(format!(
                "the slot is held by `{}`, which scores at least as well",
                occupant.title.as_deref().unwrap_or(&occupant.work_id)
            )));
        }

        tx.execute(
            "UPDATE release SET scheduled_at = NULL, updated_at = ?2 WHERE id = ?1",
            params![occupant.id, timestamp],
        )?;
        displaced = Some(Release {
            scheduled_at: None,
            ..occupant
        });
    }

    tx.execute(
        "UPDATE release SET scheduled_at = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, slot, timestamp],
    )?;

    tx.commit()?;

    Ok(Scheduling {
        release: get(conn, id)?.ok_or_else(|| unknown_release(id))?,
        displaced,
    })
}

/// Latest total for a work, or `None` when it has never been scored.
fn strength(conn: &Connection, work_id: &str) -> Result<Option<f64>> {
    Ok(conn
        .query_row(
            "SELECT total FROM work_score WHERE work_id = ?1 ORDER BY scored_at DESC LIMIT 1",
            params![work_id],
            |row| row.get(0),
        )
        .optional()?)
}

/// Take a release out of the calendar without deleting it.
pub fn unschedule(conn: &Connection, id: &str) -> Result<Release> {
    if conn.execute(
        "UPDATE release SET scheduled_at = NULL, updated_at = ?2 WHERE id = ?1",
        params![id, now()],
    )? == 0
    {
        return Err(unknown_release(id));
    }

    get(conn, id)?.ok_or_else(|| unknown_release(id))
}

/// Mark a release as out, optionally with the link it went out on.
///
/// Shipping is a state, not an integration: nothing is published from here.
pub fn mark_released(conn: &Connection, id: &str, url: Option<String>) -> Result<Release> {
    let timestamp = now();

    if conn.execute(
        "UPDATE release SET status = ?2, released_at = ?3, url = coalesce(?4, url), updated_at = ?3
         WHERE id = ?1",
        params![id, RELEASED, timestamp, url],
    )? == 0
    {
        return Err(unknown_release(id));
    }

    get(conn, id)?.ok_or_else(|| unknown_release(id))
}

pub fn update(conn: &Connection, id: &str, patch: ReleasePatch) -> Result<Release> {
    let mut assignments: Vec<String> = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    fn set(
        assignments: &mut Vec<String>,
        values: &mut Vec<Box<dyn rusqlite::ToSql>>,
        column: &str,
        value: Box<dyn rusqlite::ToSql>,
    ) {
        values.push(value);
        assignments.push(format!("{column} = ?{}", values.len()));
    }

    if let Some(kind) = patch.kind {
        set(&mut assignments, &mut values, "kind", Box::new(kind));
    }
    if let Some(status) = patch.status {
        set(&mut assignments, &mut values, "status", Box::new(status));
    }
    if let Some(title) = patch.title {
        set(&mut assignments, &mut values, "title", Box::new(title));
    }
    if let Some(scheduled_at) = patch.scheduled_at {
        set(
            &mut assignments,
            &mut values,
            "scheduled_at",
            Box::new(scheduled_at),
        );
    }
    if let Some(url) = patch.url {
        set(&mut assignments, &mut values, "url", Box::new(url));
    }
    if let Some(meta) = patch.meta {
        set(
            &mut assignments,
            &mut values,
            "meta",
            Box::new(Value::Object(meta).to_string()),
        );
    }

    if assignments.is_empty() {
        return get(conn, id)?.ok_or_else(|| unknown_release(id));
    }

    set(&mut assignments, &mut values, "updated_at", Box::new(now()));
    values.push(Box::new(id.to_owned()));

    let sql = format!(
        "UPDATE release SET {} WHERE id = ?{}",
        assignments.join(", "),
        values.len()
    );
    let params = rusqlite::params_from_iter(values.iter().map(AsRef::as_ref));

    if conn.execute(&sql, params)? == 0 {
        return Err(unknown_release(id));
    }

    get(conn, id)?.ok_or_else(|| unknown_release(id))
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    if conn.execute("DELETE FROM release WHERE id = ?1", params![id])? == 0 {
        return Err(unknown_release(id));
    }
    Ok(())
}

const SELECT_SCHEDULED: &str = "SELECT r.id, r.work_id, r.kind, r.status, r.title, r.scheduled_at, \
     r.released_at, r.url, r.meta, r.created_at, r.updated_at, w.title, s.total, s.tier \
     FROM release r \
     JOIN work w ON w.id = r.work_id \
     LEFT JOIN work_score s ON s.id = ( \
         SELECT id FROM work_score WHERE work_id = r.work_id ORDER BY scored_at DESC LIMIT 1 \
     ) \
     WHERE w.profile_id = ?1";

/// Everything with a slot, in calendar order.
pub fn calendar(conn: &Connection, profile_id: &str) -> Result<Vec<ScheduledRelease>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT_SCHEDULED} AND r.scheduled_at IS NOT NULL ORDER BY r.scheduled_at, w.title"
    ))?;
    read_scheduled(&mut statement, profile_id)
}

/// Everything planned but unscheduled, strongest first — the queue that feeds
/// the calendar.
pub fn queue(conn: &Connection, profile_id: &str) -> Result<Vec<ScheduledRelease>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT_SCHEDULED} AND r.scheduled_at IS NULL AND r.status = '{PLANNED}' \
         ORDER BY s.total IS NULL, s.total DESC, w.title"
    ))?;
    read_scheduled(&mut statement, profile_id)
}

/// Releases of one work, whatever their state.
pub fn for_work(conn: &Connection, work_id: &str) -> Result<Vec<Release>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT_RELEASE} WHERE work_id = ?1 ORDER BY coalesce(scheduled_at, created_at)"
    ))?;
    let raw = statement
        .query_map(params![work_id], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    raw.into_iter().map(RawRelease::into_release).collect()
}

fn read_scheduled(
    statement: &mut rusqlite::Statement<'_>,
    profile_id: &str,
) -> Result<Vec<ScheduledRelease>> {
    let rows = statement
        .query_map(params![profile_id], |row| {
            Ok((
                RawRelease {
                    id: row.get(0)?,
                    work_id: row.get(1)?,
                    kind: row.get(2)?,
                    status: row.get(3)?,
                    title: row.get(4)?,
                    scheduled_at: row.get(5)?,
                    released_at: row.get(6)?,
                    url: row.get(7)?,
                    meta: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                },
                row.get::<_, String>(11)?,
                row.get::<_, Option<f64>>(12)?,
                row.get::<_, Option<String>>(13)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    rows.into_iter()
        .map(|(raw, work_title, total, tier)| {
            Ok(ScheduledRelease {
                release: raw.into_release()?,
                work_title,
                total,
                tier,
            })
        })
        .collect()
}

fn unknown_release(id: &str) -> Error {
    Error::Other(format!("no release with id `{id}`"))
}

struct RawRelease {
    id: String,
    work_id: String,
    kind: String,
    status: String,
    title: Option<String>,
    scheduled_at: Option<String>,
    released_at: Option<String>,
    url: Option<String>,
    meta: String,
    created_at: String,
    updated_at: String,
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawRelease> {
    Ok(RawRelease {
        id: row.get(0)?,
        work_id: row.get(1)?,
        kind: row.get(2)?,
        status: row.get(3)?,
        title: row.get(4)?,
        scheduled_at: row.get(5)?,
        released_at: row.get(6)?,
        url: row.get(7)?,
        meta: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

impl RawRelease {
    fn into_release(self) -> Result<Release> {
        Ok(Release {
            meta: serde_json::from_str(&self.meta)?,
            id: self.id,
            work_id: self.work_id,
            kind: self.kind,
            status: self.status,
            title: self.title,
            scheduled_at: self.scheduled_at,
            released_at: self.released_at,
            url: self.url,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::score::{self, NewScore};
    use crate::work::{self, NewWork};
    use serde_json::json;

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    /// A work with an optional score, and one planned clip release.
    fn planned(conn: &Connection, profile_id: &str, title: &str, hook: Option<f64>) -> Release {
        let work = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: title.into(),
                status: None,
                collection_id: None,
                meta: None,
            },
        )
        .unwrap();

        if let Some(hook) = hook {
            score::create(
                conn,
                &work.id,
                NewScore {
                    axes: json!({ "hook": hook }).as_object().cloned().unwrap(),
                    version_id: None,
                    note: None,
                },
            )
            .unwrap();
        }

        create(
            conn,
            NewRelease {
                work_id: work.id,
                kind: "clip".into(),
                title: Some(title.into()),
                scheduled_at: None,
                meta: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn a_new_release_starts_planned_and_unscheduled() {
        let (conn, profile_id) = workspace();

        let release = planned(&conn, &profile_id, "Subject", None);

        assert_eq!(release.status, PLANNED);
        assert!(release.scheduled_at.is_none());
        assert!(release.released_at.is_none());
    }

    #[test]
    fn scheduling_an_empty_slot_displaces_nothing() {
        let (mut conn, profile_id) = workspace();
        let release = planned(&conn, &profile_id, "Subject", Some(8.0));

        let result = schedule(&mut conn, &release.id, "2026-09-01").unwrap();

        assert_eq!(result.release.scheduled_at.as_deref(), Some("2026-09-01"));
        assert!(result.displaced.is_none());
    }

    #[test]
    fn a_stronger_work_takes_the_slot_and_the_weaker_returns_to_the_queue() {
        let (mut conn, profile_id) = workspace();
        let weak = planned(&conn, &profile_id, "Weak", Some(4.0));
        let strong = planned(&conn, &profile_id, "Strong", Some(9.0));
        schedule(&mut conn, &weak.id, "2026-09-01").unwrap();

        let result = schedule(&mut conn, &strong.id, "2026-09-01").unwrap();

        assert_eq!(result.displaced.as_ref().unwrap().id, weak.id);
        assert!(
            result.displaced.as_ref().unwrap().scheduled_at.is_none(),
            "the displaced release keeps existing, it only loses the date"
        );
        let queued = queue(&conn, &profile_id).unwrap();
        assert!(
            queued.iter().any(|entry| entry.release.id == weak.id),
            "it must come back in the queue, not disappear"
        );
    }

    #[test]
    fn a_weaker_work_cannot_take_an_occupied_slot() {
        let (mut conn, profile_id) = workspace();
        let strong = planned(&conn, &profile_id, "Strong", Some(9.0));
        let weak = planned(&conn, &profile_id, "Weak", Some(4.0));
        schedule(&mut conn, &strong.id, "2026-09-01").unwrap();

        let result = schedule(&mut conn, &weak.id, "2026-09-01");

        assert!(result.is_err());
        let calendar = calendar(&conn, &profile_id).unwrap();
        assert_eq!(calendar.len(), 1);
        assert_eq!(calendar[0].release.id, strong.id);
    }

    #[test]
    fn an_equal_score_leaves_the_slot_with_whoever_holds_it() {
        let (mut conn, profile_id) = workspace();
        let holder = planned(&conn, &profile_id, "Holder", Some(7.0));
        let challenger = planned(&conn, &profile_id, "Challenger", Some(7.0));
        schedule(&mut conn, &holder.id, "2026-09-01").unwrap();

        // A plan should not move without a reason to move it.
        assert!(schedule(&mut conn, &challenger.id, "2026-09-01").is_err());
    }

    #[test]
    fn an_unscored_release_cannot_displace_a_scored_one() {
        let (mut conn, profile_id) = workspace();
        let scored = planned(&conn, &profile_id, "Scored", Some(3.0));
        let unscored = planned(&conn, &profile_id, "Unscored", None);
        schedule(&mut conn, &scored.id, "2026-09-01").unwrap();

        assert!(schedule(&mut conn, &unscored.id, "2026-09-01").is_err());
    }

    #[test]
    fn a_scored_release_displaces_an_unscored_one() {
        let (mut conn, profile_id) = workspace();
        let unscored = planned(&conn, &profile_id, "Unscored", None);
        let scored = planned(&conn, &profile_id, "Scored", Some(1.0));
        schedule(&mut conn, &unscored.id, "2026-09-01").unwrap();

        let result = schedule(&mut conn, &scored.id, "2026-09-01").unwrap();

        assert_eq!(result.displaced.unwrap().id, unscored.id);
    }

    #[test]
    fn rescheduling_the_same_release_does_not_displace_itself() {
        let (mut conn, profile_id) = workspace();
        let release = planned(&conn, &profile_id, "Subject", Some(6.0));
        schedule(&mut conn, &release.id, "2026-09-01").unwrap();

        let result = schedule(&mut conn, &release.id, "2026-09-01").unwrap();

        assert!(result.displaced.is_none());
        assert_eq!(result.release.scheduled_at.as_deref(), Some("2026-09-01"));
    }

    #[test]
    fn a_released_slot_does_not_block_a_new_one() {
        let (mut conn, profile_id) = workspace();
        let out = planned(&conn, &profile_id, "Already out", Some(9.0));
        schedule(&mut conn, &out.id, "2026-09-01").unwrap();
        mark_released(&conn, &out.id, Some("https://example.invalid/1".into())).unwrap();
        let next = planned(&conn, &profile_id, "Next", Some(2.0));

        // History occupies the date, but it is no longer a plan competing for it.
        let result = schedule(&mut conn, &next.id, "2026-09-01").unwrap();

        assert!(result.displaced.is_none());
    }

    #[test]
    fn marking_released_records_the_link_and_the_date() {
        let (conn, profile_id) = workspace();
        let release = planned(&conn, &profile_id, "Subject", None);

        let out =
            mark_released(&conn, &release.id, Some("https://example.invalid/x".into())).unwrap();

        assert_eq!(out.status, RELEASED);
        assert!(out.released_at.is_some());
        assert_eq!(out.url.as_deref(), Some("https://example.invalid/x"));
    }

    #[test]
    fn marking_released_without_a_link_keeps_the_previous_one() {
        let (conn, profile_id) = workspace();
        let release = planned(&conn, &profile_id, "Subject", None);
        mark_released(&conn, &release.id, Some("https://example.invalid/x".into())).unwrap();

        let again = mark_released(&conn, &release.id, None).unwrap();

        assert_eq!(again.url.as_deref(), Some("https://example.invalid/x"));
    }

    #[test]
    fn the_queue_is_strongest_first_with_unscored_last() {
        let (conn, profile_id) = workspace();
        planned(&conn, &profile_id, "Middle", Some(5.0));
        planned(&conn, &profile_id, "Unscored", None);
        planned(&conn, &profile_id, "Best", Some(9.0));

        let queue = queue(&conn, &profile_id).unwrap();

        assert_eq!(queue[0].work_title, "Best");
        assert_eq!(queue[1].work_title, "Middle");
        assert_eq!(queue[2].work_title, "Unscored");
    }

    #[test]
    fn unscheduling_keeps_the_release_but_frees_the_slot() {
        let (mut conn, profile_id) = workspace();
        let release = planned(&conn, &profile_id, "Subject", Some(7.0));
        schedule(&mut conn, &release.id, "2026-09-01").unwrap();

        let freed = unschedule(&conn, &release.id).unwrap();

        assert!(freed.scheduled_at.is_none());
        assert_eq!(freed.status, PLANNED);
        assert!(calendar(&conn, &profile_id).unwrap().is_empty());
    }

    #[test]
    fn deleting_a_work_takes_its_releases_with_it() {
        let (conn, profile_id) = workspace();
        let release = planned(&conn, &profile_id, "Doomed", None);

        work::delete(&conn, &release.work_id).unwrap();

        assert!(get(&conn, &release.id).unwrap().is_none());
    }

    #[test]
    fn a_release_needs_an_existing_work() {
        let (conn, _) = workspace();

        let result = create(
            &conn,
            NewRelease {
                work_id: "nope".into(),
                kind: "clip".into(),
                title: None,
                scheduled_at: None,
                meta: None,
            },
        );

        assert!(result.is_err());
    }
}
