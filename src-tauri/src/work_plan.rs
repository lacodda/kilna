//! A planned slot of a work (v0.138, FR-11): a day a work is meant to go out
//! before anything of it can - a song with no door of its own, a sketch with
//! no prompts yet - kept on the calendar as a promise, not a release.
//!
//! One slot per work: planning it again moves the day. A slot is not a
//! release: it makes nothing released, counts toward no rhythm, is no
//! unready publication and asks nobody "did it go out?". What it does is
//! remind - from `plan_lead_days` before its day - and stand in the repeat
//! guard's booked days, so a song is checked against what is planned around
//! it before anything is made of it.
//!
//! It closes by one rule, the same that silences its reminder, so no slot
//! is left standing after its work went out: the work's own release is on
//! the calendar, or something made from it is booked within
//! `plan_match_window_days` of the slot's day. A slot that closed this way is
//! read as gone everywhere; its row stays until the work is planned again.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::minted::Minted;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct WorkPlan {
    pub id: String,
    pub work_id: String,
    /// The day it is meant to go out, `YYYY-MM-DD`.
    pub scheduled_at: String,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// What a planning names.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct NewWorkPlan {
    pub work_id: String,
    pub scheduled_at: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// What moving a slot or writing its note changes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct WorkPlanPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub scheduled_at: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(optional)]
    pub note: Option<Option<String>>,
}

const SELECT: &str =
    "SELECT id, work_id, scheduled_at, note, created_at, updated_at FROM work_plan";

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkPlan> {
    Ok(WorkPlan {
        id: row.get(0)?,
        work_id: row.get(1)?,
        scheduled_at: row.get(2)?,
        note: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn unknown(id: &str) -> Error {
    Error::not_found("work plan", id)
}

/// A day as the calendar writes one.
fn check_day(day: &str) -> Result<()> {
    let format = time::macros::format_description!("[year]-[month]-[day]");
    time::Date::parse(day, &format)
        .map(|_| ())
        .map_err(|_| Error::refused("workPlan.badDay").param("day", day))
}

fn tidy(note: Option<String>) -> Option<String> {
    note.map(|note| note.trim().to_owned())
        .filter(|note| !note.is_empty())
}

pub fn create_minted(conn: &Connection, new: NewWorkPlan, minted: Minted) -> Result<WorkPlan> {
    check_day(&new.scheduled_at)?;
    conn.execute(
        "INSERT INTO work_plan (id, work_id, scheduled_at, note, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![
            minted.id(),
            new.work_id,
            new.scheduled_at,
            tidy(new.note),
            minted.at()
        ],
    )?;
    get(conn, minted.id())?
        .ok_or_else(|| Error::Internal("the work plan vanished after insert".into()))
}

/// Move a slot or write its note. A patch that changes nothing writes
/// nothing.
pub fn update_at(conn: &Connection, id: &str, patch: WorkPlanPatch, at: &str) -> Result<WorkPlan> {
    let found = get(conn, id)?.ok_or_else(|| unknown(id))?;
    let scheduled_at = patch
        .scheduled_at
        .unwrap_or_else(|| found.scheduled_at.clone());
    check_day(&scheduled_at)?;
    let note = match patch.note {
        Some(note) => tidy(note),
        None => found.note.clone(),
    };
    if scheduled_at == found.scheduled_at && note == found.note {
        return Ok(found);
    }
    conn.execute(
        "UPDATE work_plan SET scheduled_at = ?2, note = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, scheduled_at, note, at],
    )?;
    get(conn, id)?.ok_or_else(|| unknown(id))
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    if conn.execute("DELETE FROM work_plan WHERE id = ?1", params![id])? == 0 {
        return Err(unknown(id));
    }
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<WorkPlan>> {
    Ok(conn
        .query_row(&format!("{SELECT} WHERE id = ?1"), params![id], read)
        .optional()?)
}

/// The slot of a work, whether or not it closed since.
pub fn of_work(conn: &Connection, work_id: &str) -> Result<Option<WorkPlan>> {
    Ok(conn
        .query_row(
            &format!("{SELECT} WHERE work_id = ?1"),
            params![work_id],
            read,
        )
        .optional()?)
}

/// The days of the work's own releases on the calendar, and of the releases
/// of everything made from it, `YYYY-MM-DD`.
fn booked_days(conn: &Connection, work_id: &str) -> Result<(bool, Vec<String>)> {
    let own: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM release
                         WHERE work_id = ?1 AND (scheduled_at IS NOT NULL OR status = 'released'))",
        params![work_id],
        |row| row.get(0),
    )?;
    let mut statement = conn.prepare(
        "SELECT substr(r.scheduled_at, 1, 10) FROM release r
           JOIN work_link l ON l.work_id = r.work_id
          WHERE l.source_id = ?1 AND r.scheduled_at IS NOT NULL
          ORDER BY r.scheduled_at, r.rowid",
    )?;
    let days = statement
        .query_map(params![work_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    Ok((own, days))
}

fn day_number(day: &str) -> Option<i64> {
    let format = time::macros::format_description!("[year]-[month]-[day]");
    time::Date::parse(day.get(..10)?, &format)
        .ok()
        .map(|date| i64::from(date.to_julian_day()))
}

/// Whether a slot has closed (FR-11 11.3.4): the work's own release is on
/// the calendar or went out, or something made from it is booked within
/// `window` days of the slot's day. The one rule the calendar, the reminder
/// and the bell all read.
pub fn closed(conn: &Connection, plan: &WorkPlan, window: u32) -> Result<bool> {
    let (own, days) = booked_days(conn, &plan.work_id)?;
    if own {
        return Ok(true);
    }
    let Some(slot) = day_number(&plan.scheduled_at) else {
        return Ok(false);
    };
    Ok(days
        .iter()
        .filter_map(|day| day_number(day))
        .any(|day| (day - slot).unsigned_abs() <= u64::from(window)))
}

/// The open slots of a profile's works, earliest first: what the calendar
/// draws and the reminders read. A work in the trash has no slot.
pub fn open(conn: &Connection, profile_id: &str, window: u32) -> Result<Vec<WorkPlan>> {
    let mut statement = conn.prepare(
        "SELECT p.id, p.work_id, p.scheduled_at, p.note, p.created_at, p.updated_at
           FROM work_plan p JOIN work w ON w.id = p.work_id
          WHERE w.profile_id = ?1
          ORDER BY p.scheduled_at, p.created_at, p.rowid",
    )?;
    let plans = statement
        .query_map(params![profile_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut out = Vec::with_capacity(plans.len());
    for plan in plans {
        if !closed(conn, &plan, window)? {
            out.push(plan);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    fn planned(conn: &Connection, work_id: &str, day: &str) -> WorkPlan {
        crate::actions::work_plan::plan(
            conn,
            NewWorkPlan {
                work_id: work_id.into(),
                scheduled_at: day.into(),
                note: None,
            },
        )
        .unwrap()
    }

    fn undo_last(conn: &Connection) {
        let offer = crate::undo::last(conn).unwrap().unwrap();
        crate::undo::undo(conn, &offer.operation_id).unwrap();
    }

    #[test]
    fn a_work_has_one_slot_and_planning_it_again_moves_it() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Lanterns");
        let first = planned(&conn, &song.id, "2026-10-02");
        let again = planned(&conn, &song.id, "2026-10-09");
        assert_eq!(again.id, first.id, "the same slot, moved");
        assert_eq!(again.scheduled_at, "2026-10-09");
        undo_last(&conn);
        assert_eq!(
            get(&conn, &first.id).unwrap().unwrap().scheduled_at,
            "2026-10-02"
        );
        // An undo is not undone by the next press: a new planning is.
        let other = fixtures::song(&conn, &profile_id, "Harbour");
        let made = planned(&conn, &other.id, "2026-11-01");
        undo_last(&conn);
        assert!(
            get(&conn, &made.id).unwrap().is_none(),
            "undo takes the planning back"
        );

        let slot = of_work(&conn, &song.id).unwrap().unwrap();
        crate::actions::work_plan::delete(&conn, &slot.id).unwrap();
        assert!(of_work(&conn, &song.id).unwrap().is_none());
        undo_last(&conn);
        assert_eq!(of_work(&conn, &song.id).unwrap().unwrap().id, slot.id);
        let refused = crate::actions::work_plan::plan(
            &conn,
            NewWorkPlan {
                work_id: song.id.clone(),
                scheduled_at: "next week".into(),
                note: None,
            },
        );
        assert_eq!(
            refused.unwrap_err().refusal().map(|r| r.code),
            Some("workPlan.badDay")
        );
    }

    /// One rule closes a slot and silences its reminder: the work's own
    /// release on the calendar, or something made from it booked within the
    /// window of the slot's day - and only then.
    #[test]
    fn a_slot_closes_when_what_it_promised_is_booked_near_its_day() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Lanterns");
        let slot = planned(&conn, &song.id, "2026-10-10");
        assert_eq!(open(&conn, &profile_id, 7).unwrap().len(), 1);

        let far = crate::actions::work::derive(&conn, &song.id, "short", None, None, None).unwrap();
        crate::actions::release::schedule(&conn, &far.release_id.unwrap(), "2026-12-20").unwrap();
        assert!(
            !closed(&conn, &slot, 7).unwrap(),
            "a short two months on closes nothing"
        );

        let near =
            crate::actions::work::derive(&conn, &song.id, "audio", None, None, None).unwrap();
        crate::actions::release::schedule(&conn, &near.release_id.unwrap(), "2026-10-14").unwrap();
        assert!(
            closed(&conn, &slot, 7).unwrap(),
            "an audio four days on closes it"
        );
        assert!(
            !closed(&conn, &slot, 3).unwrap(),
            "but not within a narrower window"
        );
        assert!(open(&conn, &profile_id, 7).unwrap().is_empty());
    }

    /// The bell warns once when a slot's lead days begin, and once when its
    /// day is past - and a closed slot says nothing.
    #[test]
    fn the_bell_warns_of_a_slot_once_on_its_way_and_once_late() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Lanterns");
        planned(&conn, &song.id, "2026-10-10");
        let warnings = |day: &str| {
            crate::actions::release::warn_unready(&conn, day).unwrap();
            crate::journal::list(&conn, &profile_id)
                .unwrap()
                .into_iter()
                .filter(|entry| {
                    entry.action.starts_with("workPlan.") && entry.action != "workPlan.created"
                })
                .map(|entry| entry.action)
                .collect::<Vec<_>>()
        };
        assert!(
            warnings("2026-09-30").is_empty(),
            "ten days ahead says nothing"
        );
        assert_eq!(warnings("2026-10-05"), ["workPlan.due"]);
        assert_eq!(warnings("2026-10-06"), ["workPlan.due"], "said once");
        assert_eq!(warnings("2026-10-12").len(), 2, "late is said once more");
    }

    /// Making a slot real is one gesture: the audio made from the song, its
    /// release on the slot's day, the slot gone - and one undo takes it all.
    #[test]
    fn a_slot_made_real_books_a_work_made_from_it_on_its_day() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Lanterns");
        let slot = planned(&conn, &song.id, "2026-10-10");
        let done = crate::actions::work_plan::fulfill(&conn, &slot.id, Some("audio")).unwrap();
        let release = crate::release::get(&conn, &done.release_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            release.scheduled_at.as_deref().map(|at| &at[..10]),
            Some("2026-10-10")
        );
        assert_eq!(
            crate::work::get(&conn, &done.work_id)
                .unwrap()
                .unwrap()
                .kind,
            "audio"
        );
        assert!(get(&conn, &slot.id).unwrap().is_none());

        undo_last(&conn);
        assert!(get(&conn, &slot.id).unwrap().is_some(), "the slot is back");
        assert!(
            crate::work::get(&conn, &done.work_id).unwrap().is_none(),
            "and the audio gone"
        );

        // A song goes out nowhere itself: made real as itself, it is refused.
        let refused = crate::actions::work_plan::fulfill(&conn, &slot.id, None);
        assert_eq!(
            refused.unwrap_err().refusal().map(|r| r.code),
            Some("release.noDoors")
        );
    }
}
