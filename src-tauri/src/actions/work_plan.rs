//! Gestures on a work's planned slot (v0.138, FR-11): planning it on a day,
//! moving it, writing its note, taking it off, and making it real.

use rusqlite::Connection;

use super::{atomically, gesture};
use crate::error::{Error, Result};
use crate::journal::Record;
use crate::work_plan::{self, NewWorkPlan, WorkPlan, WorkPlanPatch};

/// Plan a work on a day. A work has one slot: planning it again moves the
/// slot it has - the same row, so its history stays one thread.
pub fn plan(conn: &Connection, new: NewWorkPlan) -> Result<WorkPlan> {
    let work = crate::work::get(conn, &new.work_id)?
        .ok_or_else(|| Error::not_found("work", &new.work_id))?;
    if let Some(slot) = work_plan::of_work(conn, &new.work_id)? {
        return update(
            conn,
            &slot.id,
            WorkPlanPatch {
                scheduled_at: Some(new.scheduled_at),
                note: new.note.map(Some),
            },
        );
    }
    gesture(conn, "workPlan.create", |act| {
        act.json("workPlan", &new)?;
        let minted = act.mint();
        let created = work_plan::create_minted(act, new, minted)?;
        act.param("title", work.title.clone());
        act.journal(
            Record::new("workPlan.created")
                .param("title", work.title.clone())
                .param("day", created.scheduled_at.clone())
                .about("work", work.id.clone()),
        );
        Ok(created)
    })
}

/// Move a slot to another day, or write its note.
pub fn update(conn: &Connection, id: &str, patch: WorkPlanPatch) -> Result<WorkPlan> {
    gesture(conn, "workPlan.update", |act| {
        let before = work_plan::get(act, id)?.ok_or_else(|| Error::not_found("work plan", id))?;
        let title = act.title_of(&before.work_id);
        act.param("id", id);
        act.param("title", title.clone());
        act.json("patch", &patch)?;
        act.before(Some(&before), &patch)?;
        act.stamped();
        let after = work_plan::update_at(act, id, patch, act.at())?;
        if after == before {
            act.unchanged();
        } else {
            act.journal(
                Record::new("workPlan.moved")
                    .param("title", title)
                    .param("day", after.scheduled_at.clone())
                    .about("work", after.work_id.clone()),
            );
        }
        Ok(after)
    })
}

/// Take a slot off the calendar. The log keeps a copy for the undo.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    gesture(conn, "workPlan.delete", |act| {
        let before = work_plan::get(act, id)?.ok_or_else(|| Error::not_found("work plan", id))?;
        let title = act.title_of(&before.work_id);
        act.param("id", id);
        act.param("title", title.clone());
        act.json("before", &before)?;
        work_plan::delete(act, id)?;
        act.journal(
            Record::new("workPlan.deleted")
                .param("title", title)
                .about("work", before.work_id.clone()),
        );
        Ok(())
    })
}

/// What making a slot real gave: the work that goes out on its day, and the
/// release booked for it.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
pub struct Fulfilled {
    pub work_id: String,
    pub release_id: String,
}

/// Make a slot real (FR-11 11.3.3): the release of the work itself on the
/// slot's day, when its kind goes out somewhere; or, for a work that does
/// not - a song - a work made from it of `kind` (an audio, a video, a short)
/// with its release on the slot's day. The slot goes in the same gesture:
/// what it promised is on the calendar now, and an undo takes all of it back.
pub fn fulfill(conn: &Connection, id: &str, kind: Option<&str>) -> Result<Fulfilled> {
    atomically(conn, |conn| {
        let slot = work_plan::get(conn, id)?.ok_or_else(|| Error::not_found("work plan", id))?;
        let work = crate::work::get(conn, &slot.work_id)?
            .ok_or_else(|| Error::not_found("work", &slot.work_id))?;
        let made_here = kind.is_none_or(|kind| kind == work.kind);
        let (work_id, release_id) = if made_here {
            let config = crate::profile::config_for(conn, &work.profile_id)?;
            let door = config
                .vocabulary(&work.kind)
                .release_kinds
                .first()
                .map(|door| door.key.clone())
                .ok_or_else(|| {
                    Error::refused("release.noDoors").param("kind", config.kind_named(&work.kind))
                })?;
            let release = super::release::create(
                conn,
                crate::release::NewRelease {
                    work_id: work.id.clone(),
                    kind: door,
                    scheduled_at: None,
                    meta: None,
                    scheduled_time: None,
                    time_zone: None,
                },
            )?;
            (work.id.clone(), release.id)
        } else {
            let made =
                super::work::derive(conn, &work.id, kind.unwrap_or_default(), None, None, None)?;
            let release_id = made.release_id.ok_or_else(|| {
                Error::refused("release.noDoors").param("kind", kind.unwrap_or_default())
            })?;
            (made.work.id, release_id)
        };
        // The day as an edit of the release, at the hour the rhythm says,
        // so the whole gesture is one an undo takes back.
        let config = crate::profile::config_for(conn, &work.profile_id)?;
        super::release::update(
            conn,
            &release_id,
            crate::release::ReleasePatch {
                scheduled_at: Some(Some(slot.scheduled_at.clone())),
                scheduled_time: config
                    .rhythm
                    .as_ref()
                    .and_then(|rhythm| rhythm.default_time.clone())
                    .map(Some),
                ..crate::release::ReleasePatch::default()
            },
        )?;
        delete(conn, &slot.id)?;
        Ok(Fulfilled {
            work_id,
            release_id,
        })
    })
}
