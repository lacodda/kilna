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
    /// Set when a person settled this date. A pinned slot is not contested —
    /// see [`schedule`].
    pub slot_pinned_at: Option<String>,
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
    /// How far this release is from shippable — see [`crate::readiness`].
    pub readiness: crate::readiness::Readiness,
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

/// How claiming a slot would end. One vocabulary for the dry run and the real
/// one: v0.24 shipped a contest the screens described with a different rule,
/// and a preview that can disagree with the drop is worse than none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// Nothing planned holds the day; claiming it contests nothing.
    Empty,
    /// The challenger is stronger: the holder would return to the queue.
    Displaces,
    /// The holder scores at least as well; the claim would be refused.
    Held,
    /// The date was settled by a person; the claim would be refused unjudged.
    Pinned,
}

/// The dry run of a claim, shaped for the calendar to show before a drop.
#[derive(Debug, Clone, Serialize)]
pub struct SlotPreview {
    pub verdict: Verdict,
    /// Who holds the day, when anything does.
    pub holder_title: Option<String>,
}

/// A judged claim: who holds the slot and how the contest would end.
struct Contest {
    occupant: Option<Release>,
    verdict: Verdict,
}

/// Status values a release moves through. These are fixed rather than profile
/// vocabulary: they describe the mechanism, not the craft.
pub const PLANNED: &str = "planned";
pub const RELEASED: &str = "released";

const SELECT_RELEASE: &str = "SELECT id, work_id, kind, status, title, scheduled_at, released_at, \
     url, slot_pinned_at, meta, created_at, updated_at FROM release";

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
        return Err(Error::not_found("work", new.work_id.clone()));
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

    let judged = judge(&tx, id, slot)?;
    let timestamp = now();
    let mut displaced = None;

    match judged.verdict {
        // A pinned slot is not a contest. Strength is never consulted: the date
        // was decided by a person, and the whole point of deciding is that a
        // better score does not reopen it.
        Verdict::Pinned => {
            let occupant = judged.occupant.expect("a pinned verdict names a holder");
            return Err(Error::SlotPinned(format!(
                "`{}` holds that date and it is pinned",
                occupant.title.as_deref().unwrap_or(&occupant.work_id)
            )));
        }
        // Ties go to the release already in the slot: a plan should not move
        // without a reason to move it.
        Verdict::Held => {
            let occupant = judged.occupant.expect("a held verdict names a holder");
            return Err(Error::SlotHeld(format!(
                "the slot is held by `{}`, which scores at least as well",
                occupant.title.as_deref().unwrap_or(&occupant.work_id)
            )));
        }
        Verdict::Displaces => {
            let occupant = judged
                .occupant
                .expect("a displacing verdict names a holder");
            // No pin to clear here: a pinned slot refuses the challenge above,
            // so anything that reaches this line was never pinned. The other
            // two ways a date is lost do clear it — see `unschedule` and
            // `update`.
            tx.execute(
                "UPDATE release SET scheduled_at = NULL, updated_at = ?2 WHERE id = ?1",
                params![occupant.id, timestamp],
            )?;
            displaced = Some(Release {
                scheduled_at: None,
                ..occupant
            });
        }
        Verdict::Empty => {}
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

/// The dry run: how [`schedule`] would end, without moving anything.
///
/// Shown under the cursor while a release hovers over a day, so displacement
/// stops being a surprise announced after the drop.
pub fn preview(conn: &Connection, id: &str, slot: &str) -> Result<SlotPreview> {
    let judged = judge(conn, id, slot)?;

    let holder_title = judged.occupant.as_ref().map(|occupant| {
        occupant.title.clone().unwrap_or_else(|| {
            crate::journal::work_title(conn, &occupant.work_id).unwrap_or_default()
        })
    });

    Ok(SlotPreview {
        verdict: judged.verdict,
        holder_title,
    })
}

/// Decide how claiming `slot` for `id` would end. The one place the rule
/// lives: [`schedule`] acts on the verdict, [`preview`] shows it.
fn judge(conn: &Connection, id: &str, slot: &str) -> Result<Contest> {
    let Some(release) = conn
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

    // Whoever already holds the slot, ignoring this release itself. The
    // challenger's own row is read first purely so an unknown id errs rather
    // than previewing an empty day.
    let challenger_work = release.work_id;
    let occupant = conn
        .query_row(
            &format!(
                "{SELECT_RELEASE} WHERE scheduled_at = ?1 AND id <> ?2 AND status = '{PLANNED}' LIMIT 1"
            ),
            params![slot, id],
            read_row,
        )
        .optional()?
        .map(RawRelease::into_release)
        .transpose()?;

    let Some(occupant) = occupant else {
        return Ok(Contest {
            occupant: None,
            verdict: Verdict::Empty,
        });
    };

    let verdict = if occupant.slot_pinned_at.is_some() {
        Verdict::Pinned
    } else {
        let challenger = strength(conn, &challenger_work)?;
        let holder = strength(conn, &occupant.work_id)?;
        if challenger.unwrap_or(f64::NEG_INFINITY) <= holder.unwrap_or(f64::NEG_INFINITY) {
            Verdict::Held
        } else {
            Verdict::Displaces
        }
    };

    Ok(Contest {
        occupant: Some(occupant),
        verdict,
    })
}

/// Latest total for a work, or `None` when it has never been scored.
fn strength(conn: &Connection, work_id: &str) -> Result<Option<f64>> {
    // The same score the catalogue shows. A contest decided by a number nobody
    // can see is a contest nobody can predict — and until v0.24 this read the
    // latest snapshot while the catalogue read the current version's.
    let sql = format!(
        "SELECT total FROM work_score WHERE id = {}",
        crate::score::speaking_score_for("?1")
    );
    Ok(conn
        .query_row(&sql, params![work_id], |row| row.get(0))
        .optional()?)
}

/// Settle a date, or hand it back to the contest.
///
/// Pinning a release with no date would pin nothing, so it is refused rather
/// than silently accepted.
pub fn set_slot_pin(conn: &Connection, id: &str, pinned: bool) -> Result<Release> {
    let scheduled: Option<Option<String>> = conn
        .query_row(
            "SELECT scheduled_at FROM release WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?;

    let Some(scheduled) = scheduled else {
        return Err(unknown_release(id));
    };
    if pinned && scheduled.is_none() {
        return Err(Error::Other(
            "a release with no date has no slot to pin".into(),
        ));
    }

    let timestamp = now();
    conn.execute(
        "UPDATE release SET slot_pinned_at = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, pinned.then(|| timestamp.clone()), timestamp],
    )?;

    get(conn, id)?.ok_or_else(|| unknown_release(id))
}

/// Take a release out of the calendar without deleting it.
pub fn unschedule(conn: &Connection, id: &str) -> Result<Release> {
    // The pin goes with the date. A pin describes a date that was decided, and
    // there is no longer a date — leaving it behind creates a state nothing
    // else in the app can produce and `set_slot_pin` explicitly refuses.
    if conn.execute(
        "UPDATE release SET scheduled_at = NULL, slot_pinned_at = NULL, updated_at = ?2 WHERE id = ?1",
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
        // Clearing the date clears the pin with it, for the reason given in
        // `unschedule`: a pin without a date is a state nothing can act on.
        let clearing = scheduled_at.is_none();
        set(
            &mut assignments,
            &mut values,
            "scheduled_at",
            Box::new(scheduled_at),
        );
        if clearing {
            set(
                &mut assignments,
                &mut values,
                "slot_pinned_at",
                Box::new(None::<String>),
            );
        }
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

const SELECT_SCHEDULED_TEMPLATE: &str = "SELECT r.id, r.work_id, r.kind, r.status, r.title, r.scheduled_at, \
     r.released_at, r.url, r.slot_pinned_at, r.meta, r.created_at, r.updated_at, \
     w.title, s.total, s.tier \
     FROM release r \
     JOIN work w ON w.id = r.work_id \
     LEFT JOIN work_score s ON s.id = {speaking} \
     WHERE w.profile_id = ?1";

/// The listing query with the shared scoring rule filled in.
fn select_scheduled() -> String {
    SELECT_SCHEDULED_TEMPLATE.replace("{speaking}", &crate::score::speaking_score_for("r.work_id"))
}

/// Everything with a slot, in calendar order.
pub fn calendar(conn: &Connection, profile_id: &str) -> Result<Vec<ScheduledRelease>> {
    let mut statement = conn.prepare(&format!(
        "{} AND r.scheduled_at IS NOT NULL ORDER BY r.scheduled_at, w.title",
        select_scheduled()
    ))?;
    read_scheduled(conn, &mut statement, profile_id)
}

/// Everything planned but unscheduled, strongest first — the queue that feeds
/// the calendar.
pub fn queue(conn: &Connection, profile_id: &str) -> Result<Vec<ScheduledRelease>> {
    let mut statement = conn.prepare(&format!(
        "{} AND r.scheduled_at IS NULL AND r.status = '{PLANNED}' \
         ORDER BY s.total IS NULL, s.total DESC, w.title",
        select_scheduled()
    ))?;
    read_scheduled(conn, &mut statement, profile_id)
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
    conn: &Connection,
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
                    slot_pinned_at: row.get(8)?,
                    meta: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                },
                row.get::<_, String>(12)?,
                row.get::<_, Option<f64>>(13)?,
                row.get::<_, Option<String>>(14)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    // Judged here rather than by the screens: the warning the journal writes
    // about an unready release must mean exactly what the chip shows.
    let config = crate::profile::config_for(conn, profile_id)?;
    let roles = crate::readiness::roles_present(conn, profile_id)?;
    let none = std::collections::BTreeSet::new();

    rows.into_iter()
        .map(|(raw, work_title, total, tier)| {
            let readiness = crate::readiness::assess(
                &config,
                &raw.kind,
                roles.get(&raw.work_id).unwrap_or(&none),
                total.is_some(),
            );
            Ok(ScheduledRelease {
                release: raw.into_release()?,
                work_title,
                total,
                tier,
                readiness,
            })
        })
        .collect()
}

fn unknown_release(id: &str) -> Error {
    Error::not_found("release", id)
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
    slot_pinned_at: Option<String>,
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
        slot_pinned_at: row.get(8)?,
        meta: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
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
            slot_pinned_at: self.slot_pinned_at,
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
                ..NewWork::default()
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

    /// The contest and the screens must agree about how strong a work is.
    ///
    /// Until v0.24 this read the latest snapshot while the catalogue read the
    /// current version's, so one work showed 68.1 on one screen and 77.8 on the
    /// next — and the rule deciding who keeps a date used a third number that
    /// nothing displayed.
    #[test]
    fn strength_is_the_score_that_speaks_for_the_work() {
        let (mut conn, profile_id) = workspace();
        let held = planned(&conn, &profile_id, "Subject", Some(7.0));

        // A second, weaker score taken later: with no current version named,
        // the strongest still speaks for the work.
        score::create(
            &conn,
            &held.work_id,
            NewScore {
                axes: json!({ "hook": 3.0 }).as_object().cloned().unwrap(),
                version_id: None,
                note: None,
            },
        )
        .unwrap();

        let total = strength(&conn, &held.work_id).unwrap().unwrap();
        let strongest = score::history(&conn, &held.work_id)
            .unwrap()
            .into_iter()
            .map(|score| score.total)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (total - strongest).abs() < 1e-9,
            "contest read {total}, catalogue would read {strongest}"
        );

        // And it agrees with what the calendar shows for the same work.
        schedule(&mut conn, &held.id, "2026-09-01").unwrap();
        let shown = calendar(&conn, &profile_id).unwrap();
        let entry = shown.iter().find(|row| row.release.id == held.id).unwrap();
        assert_eq!(entry.total, Some(total));
    }

    /// A pinned date is a decision, and a decision does not reopen because a
    /// better score turned up.
    #[test]
    fn a_pinned_slot_refuses_a_stronger_challenger() {
        let (mut conn, profile_id) = workspace();

        let weak = planned(&conn, &profile_id, "Weak", Some(4.0));
        schedule(&mut conn, &weak.id, "2026-09-01").unwrap();
        set_slot_pin(&conn, &weak.id, true).unwrap();

        let strong = planned(&conn, &profile_id, "Strong", Some(9.0));
        let refused = schedule(&mut conn, &strong.id, "2026-09-01").unwrap_err();
        assert_eq!(refused.kind(), "slotPinned", "got {refused}");

        // The holder keeps both its date and its pin.
        let holder = get(&conn, &weak.id).unwrap().unwrap();
        assert_eq!(holder.scheduled_at.as_deref(), Some("2026-09-01"));
        assert!(holder.slot_pinned_at.is_some());

        // Unpinned, the same challenger wins the ordinary way.
        set_slot_pin(&conn, &weak.id, false).unwrap();
        let outcome = schedule(&mut conn, &strong.id, "2026-09-01").unwrap();
        assert!(outcome.displaced.is_some(), "the contest did not resume");
    }

    /// A pin describes a date, so losing the date loses the pin — otherwise the
    /// release sits in a state `set_slot_pin` itself refuses to create. Found on
    /// a live database rather than reasoned about.
    ///
    /// Displacement is not listed here: a pinned slot refuses the challenge, so
    /// nothing displaced was ever pinned.
    #[test]
    fn losing_the_date_loses_the_pin() {
        let (mut conn, profile_id) = workspace();

        // By unscheduling.
        let first = planned(&conn, &profile_id, "First", Some(5.0));
        schedule(&mut conn, &first.id, "2026-09-01").unwrap();
        set_slot_pin(&conn, &first.id, true).unwrap();
        let back = unschedule(&conn, &first.id).unwrap();
        assert_eq!(back.scheduled_at, None);
        assert_eq!(back.slot_pinned_at, None, "unscheduling kept the pin");

        // By clearing the date through a patch.
        schedule(&mut conn, &first.id, "2026-09-02").unwrap();
        set_slot_pin(&conn, &first.id, true).unwrap();
        let cleared = update(
            &conn,
            &first.id,
            ReleasePatch {
                scheduled_at: Some(None),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            cleared.slot_pinned_at, None,
            "clearing the date kept the pin"
        );
    }

    /// `update` writes a date without asking who holds it — by design: editing
    /// a booking is not bidding for one. This pins that behaviour down, because
    /// v0.24 wired dragging to `update` and shipped a calendar where two
    /// releases could share a day. Anything that lets a person *take* a date
    /// must go through `schedule`.
    #[test]
    fn update_moves_a_date_without_contesting_it() {
        let (mut conn, profile_id) = workspace();

        let holder = planned(&conn, &profile_id, "Holder", Some(9.0));
        schedule(&mut conn, &holder.id, "2026-09-05").unwrap();

        let other = planned(&conn, &profile_id, "Other", Some(1.0));
        update(
            &conn,
            &other.id,
            ReleasePatch {
                scheduled_at: Some(Some("2026-09-05".into())),
                ..Default::default()
            },
        )
        .unwrap();

        let sharing = calendar(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .filter(|row| row.release.scheduled_at.as_deref() == Some("2026-09-05"))
            .count();
        assert_eq!(
            sharing, 2,
            "update is expected not to contest — see the doc comment"
        );

        // And the contest, for the same pair, refuses the weaker one.
        let refused = schedule(&mut conn, &other.id, "2026-09-05");
        assert!(refused.is_err(), "schedule must still contest");
    }

    /// There is no slot to pin on something that holds no date.
    #[test]
    fn pinning_needs_a_date() {
        let (conn, profile_id) = workspace();
        let queued = planned(&conn, &profile_id, "Subject", Some(5.0));

        assert!(set_slot_pin(&conn, &queued.id, true).is_err());
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

    /// The dry run and the real claim are one rule: whatever `preview` says a
    /// drop will do is exactly what `schedule` then does. Each verdict is
    /// checked against the real outcome, because a preview that drifts from
    /// the contest misleads at the worst moment — with a release in the air.
    #[test]
    fn the_preview_says_what_scheduling_then_does() {
        let (mut conn, profile_id) = workspace();

        let strong = planned(&conn, &profile_id, "Strong", Some(9.0));
        let weak = planned(&conn, &profile_id, "Weak", Some(4.0));
        let pinned = planned(&conn, &profile_id, "Pinned", Some(5.0));

        // An empty day: claimed without displacing.
        let dry = preview(&conn, &strong.id, "2026-09-01").unwrap();
        assert_eq!(dry.verdict, Verdict::Empty);
        assert!(dry.holder_title.is_none());
        let real = schedule(&mut conn, &strong.id, "2026-09-01").unwrap();
        assert!(real.displaced.is_none());

        // A weaker challenger: refused, and the preview names the holder.
        let dry = preview(&conn, &weak.id, "2026-09-01").unwrap();
        assert_eq!(dry.verdict, Verdict::Held);
        assert_eq!(dry.holder_title.as_deref(), Some("Strong"));
        assert!(schedule(&mut conn, &weak.id, "2026-09-01").is_err());

        // A stronger challenger against the weak holder: displaces.
        schedule(&mut conn, &weak.id, "2026-09-02").unwrap();
        let dry = preview(&conn, &strong.id, "2026-09-02").unwrap();
        assert_eq!(dry.verdict, Verdict::Displaces);
        assert_eq!(dry.holder_title.as_deref(), Some("Weak"));
        let real = schedule(&mut conn, &strong.id, "2026-09-02").unwrap();
        assert_eq!(real.displaced.unwrap().id, weak.id);

        // A pinned holder: refused before strength is even consulted.
        schedule(&mut conn, &pinned.id, "2026-09-03").unwrap();
        set_slot_pin(&conn, &pinned.id, true).unwrap();
        let dry = preview(&conn, &strong.id, "2026-09-03").unwrap();
        assert_eq!(dry.verdict, Verdict::Pinned);
        let refused = schedule(&mut conn, &strong.id, "2026-09-03").unwrap_err();
        assert_eq!(refused.kind(), "slotPinned");

        // Hovering over its own day reads as empty: dropping there is a no-op,
        // not a contest against itself.
        let own = preview(&conn, &strong.id, "2026-09-02").unwrap();
        assert_eq!(own.verdict, Verdict::Empty);

        // And an unknown release errs rather than previewing an empty day.
        assert!(preview(&conn, "nope", "2026-09-01").is_err());
    }

    /// The calendar's chips carry readiness judged from the same facts every
    /// other screen shows: the roles with a version, and the speaking score.
    #[test]
    fn the_calendar_reports_how_ready_each_release_is() {
        let (mut conn, profile_id) = workspace();

        // Scored but missing both roles a clip requires.
        let bare = planned(&conn, &profile_id, "Bare", Some(6.0));
        schedule(&mut conn, &bare.id, "2026-09-01").unwrap();

        // Scored, with a version for every required role.
        let full = planned(&conn, &profile_id, "Full", Some(7.0));
        for role in ["lyrics", "style"] {
            crate::work::version::create(
                &mut conn,
                &full.work_id,
                crate::work::version::NewVersion {
                    role: role.into(),
                    body: "body".into(),
                    label: None,
                    meta: None,
                    make_current: true,
                },
            )
            .unwrap();
        }
        schedule(&mut conn, &full.id, "2026-09-02").unwrap();

        let shown = calendar(&conn, &profile_id).unwrap();
        let of = |id: &str| shown.iter().find(|row| row.release.id == id).unwrap();

        assert!(!of(&bare.id).readiness.ready);
        assert!(
            of(&bare.id)
                .readiness
                .roles
                .iter()
                .any(|mark| mark.present == Some(false)),
            "a missing required role must be marked missing"
        );
        assert!(of(&full.id).readiness.ready);

        // The queue is judged the same way.
        let queued = planned(&conn, &profile_id, "Queued unscored", None);
        let queue = queue(&conn, &profile_id).unwrap();
        let entry = queue
            .iter()
            .find(|row| row.release.id == queued.id)
            .unwrap();
        assert!(!entry.readiness.scored);
        assert!(!entry.readiness.ready);
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
