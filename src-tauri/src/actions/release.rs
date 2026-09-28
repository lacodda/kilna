//! Gestures on releases: planning one, booking it a day, writing what it goes
//! out as, and marking that it went.

use std::collections::BTreeMap;

use rusqlite::Connection;
use serde_json::Value;

use super::{BulkOutcome, Skipped, count, gesture};
use crate::db::unit::atomically;
use crate::error::{Error, Reason, Result};
use crate::journal::Record;
use crate::layout;
use crate::profile::config::Label;
use crate::release::{self, NewRelease, Release, ReleasePatch, Scheduling};
use crate::release_meta;

/// Plan a release.
///
/// Refused when the kind of release is not one this kind of work ships: a
/// release nobody could have planned from its card is not one a package may
/// plan either.
pub fn create(conn: &Connection, new: NewRelease) -> Result<Release> {
    gesture(conn, "release.create", |act| {
        let work = crate::work::get(act, &new.work_id)?
            .ok_or_else(|| Error::not_found("work", &new.work_id))?;
        crate::profile::config_for(act, act.profile_id())?
            .require_release_kind(&work.kind, &new.kind)?;
        act.json("release", &new)?;
        let minted = act.mint();
        let created = release::create_minted(act, new, minted)?;
        act.journal(
            Record::new("release.created")
                .param("title", act.title_of(&created.work_id))
                .param("kind", created.kind.clone())
                .about("work", created.work_id.clone()),
        );
        Ok(created)
    })
}

/// Edit a release: its kind, its date, the link it went out on, its fields.
///
/// Moving a date here changes the same fact `schedule` changes, so it has to
/// leave the same trail. Without this the calendar could quietly take a work
/// out of its slot while the work still called itself scheduled.
///
/// It does not compete for a slot the way claiming one does: this is editing a
/// booking you already hold, and a date typed into a form is not a bid.
pub fn update(conn: &Connection, id: &str, patch: ReleasePatch) -> Result<Release> {
    gesture(conn, "release.update", |act| {
        let before = release::get(act, id)?;
        act.param("id", id);
        act.json("patch", &patch)?;
        act.before(before.as_ref(), &patch)?;
        act.stamped();

        let updated = release::update_at(act, id, patch, act.at())?;

        let moved = before
            .as_ref()
            .is_some_and(|before| before.scheduled_at != updated.scheduled_at);
        if moved {
            act.journal(
                Record::new("release.moved")
                    .param("title", act.title_of(&updated.work_id))
                    .param("slot", updated.scheduled_at.clone().unwrap_or_default())
                    .about("work", updated.work_id.clone()),
            );
            act.restate(&updated.work_id);
        }
        Ok(updated)
    })
}

/// Write a release's fields.
///
/// Goes through the same patch the rest of the tab writes through, which is
/// what gives it undo and the operations log without a line of its own: `meta`
/// is a column of the release, and editing it is editing the release.
///
/// Keys the profile does not declare are refused rather than stored. The map
/// is open on purpose -- an agent's package and an older profile may both
/// have left things in it, and those are kept -- but a *typed* key that no
/// field names could only come from a bug, and storing it would leave a value
/// no screen can ever show again.
pub fn set_fields(
    conn: &Connection,
    id: &str,
    values: &BTreeMap<String, String>,
) -> Result<Release> {
    let before = release::get(conn, id)?.ok_or_else(|| Error::not_found("release", id))?;
    let known = release_meta::fields(conn, id)?;
    for key in values.keys() {
        if !known.iter().any(|field| &field.key == key) {
            return Err(Error::refused("release.unknownField").param("field", key.as_str()));
        }
    }
    let mut meta = before.meta.clone();
    for (key, value) in values {
        meta.insert(key.clone(), Value::String(value.clone()));
    }
    update(
        conn,
        id,
        ReleasePatch {
            meta: Some(meta),
            ..ReleasePatch::default()
        },
    )
}

/// Fill a release's fields from the profile's templates.
///
/// Only templated fields are touched; what has no template stays as it was
/// typed. Fields the renderer refused are reported back rather than written
/// blank -- see [`release_meta::generate`]. Nothing rendered leaves the release
/// exactly as it was, with no operation logged.
pub fn generate_fields(conn: &Connection, id: &str) -> Result<release_meta::Generated> {
    let before = release::get(conn, id)?.ok_or_else(|| Error::not_found("release", id))?;
    let generated = release_meta::generate(conn, id)?;
    if !generated.values.is_empty() {
        update(
            conn,
            id,
            ReleasePatch {
                meta: Some(release_meta::merged(&before.meta, &generated.values)),
                ..ReleasePatch::default()
            },
        )?;
    }
    Ok(generated)
}

/// What a batch generation did, and to what.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedBatch {
    /// Releases that gained at least one field.
    pub filled: usize,
    /// Releases passed over, with the reason: a kind with no templated field
    /// asks nothing of them, and one that failed says why.
    pub skipped: Vec<Skipped>,
    /// Fields that could not be rendered, with the release they belong to.
    /// Carried out of the batch rather than counted, because "seven releases
    /// are waiting for lyrics" is only useful if you can see which seven.
    pub refused: Vec<BatchRefusal>,
}

/// A field a batch could not fill, named by the work it is on.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRefusal {
    pub release_id: String,
    pub work_title: String,
    pub label: Label,
    pub reason: String,
}

/// Fill the fields of several releases at once.
///
/// The point of the batch is a week of the calendar: a person who has planned
/// six videos writes their metadata in one go or not at all. Each release is
/// generated exactly as it would be alone - its own operation, its own undo
/// step - and one release failing does not take the rest with it. One journal
/// line for the whole batch, not one per release.
pub fn generate_fields_batch(conn: &Connection, ids: &[String]) -> Result<GeneratedBatch> {
    atomically(conn, |conn| {
        let profile_id = super::active_profile_id(conn)?;
        let mut filled = 0usize;
        let mut skipped: Vec<Skipped> = Vec::new();
        let mut refused: Vec<BatchRefusal> = Vec::new();

        for id in ids {
            let title = release::get(conn, id)?
                .map(|found| crate::journal::work_title(conn, &found.work_id).unwrap_or_default());
            match generate_fields(conn, id) {
                Ok(generated) => {
                    for refusal in &generated.refused {
                        refused.push(BatchRefusal {
                            release_id: id.clone(),
                            work_title: title.clone().unwrap_or_default(),
                            label: refusal.label.clone(),
                            reason: refusal.reason.clone(),
                        });
                    }
                    if generated.values.is_empty() {
                        skipped.push(Skipped {
                            id: id.clone(),
                            title,
                            reason: Reason::of("skip.nothingToFill"),
                        });
                    } else {
                        filled += 1;
                    }
                }
                Err(cause) => skipped.push(Skipped {
                    id: id.clone(),
                    title,
                    reason: cause.reason(),
                }),
            }
        }

        if filled > 0 {
            crate::journal::record(
                conn,
                &profile_id,
                Record::new("release.fieldsBatch").param("count", count(filled)),
            );
        }
        Ok(GeneratedBatch {
            filled,
            skipped,
            refused,
        })
    })
}

/// Claim a calendar slot. Displacing a weaker release is reported back rather
/// than done silently — the person should see what moved.
pub fn schedule(conn: &Connection, id: &str, slot: &str) -> Result<Scheduling> {
    gesture(conn, "release.schedule", |act| {
        act.param("id", id);
        act.param("slot", slot);
        act.stamped();
        let outcome = release::schedule_at(act, id, slot, act.at())?;
        act.journal(
            Record::new("release.scheduled")
                .param("title", act.title_of(&outcome.release.work_id))
                .param("slot", slot)
                .about("work", outcome.release.work_id.clone()),
        );
        act.restate(&outcome.release.work_id);
        Ok(outcome)
    })
}

/// Settle a date so the contest leaves it alone, or hand it back.
pub fn set_slot_pin(conn: &Connection, id: &str, pinned: bool) -> Result<Release> {
    gesture(conn, "release.setSlotPin", |act| {
        act.param("id", id);
        act.param("pinned", pinned);
        act.stamped();
        let updated = release::set_slot_pin_at(act, id, pinned, act.at())?;

        // Both keys are written out literally rather than chosen inside the
        // call: the gate that checks every recorded action has a sentence
        // reads the source for literal keys, and one computed in the argument
        // is invisible to it.
        let entry = if pinned {
            Record::new("release.pinned")
        } else {
            Record::new("release.unpinned")
        };
        act.journal(
            entry
                .param("title", act.title_of(&updated.work_id))
                .param("slot", updated.scheduled_at.clone().unwrap_or_default())
                .about("work", updated.work_id.clone()),
        );
        Ok(updated)
    })
}

/// Take a release off the calendar, back into the queue.
pub fn unschedule(conn: &Connection, id: &str) -> Result<Release> {
    gesture(conn, "release.unschedule", |act| {
        act.param("id", id);
        act.stamped();
        let release = release::unschedule_at(act, id, act.at())?;
        act.restate(&release.work_id);
        Ok(release)
    })
}

/// Take several works off the calendar at once.
///
/// Only planned releases holding a slot are touched; anything already out
/// stays where it is. Each work is restated afterwards, so a work that has
/// nothing booked any more goes back to saying so on its own.
pub fn unschedule_works(conn: &Connection, work_ids: &[String]) -> Result<BulkOutcome> {
    gesture(conn, "release.unscheduleBatch", |act| {
        let mut released: Vec<String> = Vec::new();
        let mut skipped: Vec<Skipped> = Vec::new();

        for work_id in work_ids {
            let title = crate::journal::work_title(act, work_id);
            let scheduled = release::scheduled_for(act, work_id)?;
            if scheduled.is_empty() {
                skipped.push(Skipped {
                    id: work_id.clone(),
                    title,
                    reason: Reason::of("skip.nothingBooked"),
                });
                continue;
            }
            for release_id in scheduled {
                match atomically(act, |tx| release::unschedule_at(tx, &release_id, act.at())) {
                    Ok(_) => released.push(release_id),
                    Err(cause) => skipped.push(Skipped {
                        id: work_id.clone(),
                        title: title.clone(),
                        reason: cause.reason(),
                    }),
                }
            }
        }

        for work_id in work_ids {
            act.restate(work_id);
        }

        // Only the releases actually taken off the calendar go in the log: a
        // work with nothing booked was skipped, and recording it would replay
        // an unschedule onto a release a repeat run never touched.
        if released.is_empty() {
            act.unchanged();
        } else {
            act.json("releaseIds", &released)?;
            act.stamped();
            act.journal(
                Record::new("release.unscheduledBatch").param("count", count(released.len())),
            );
        }

        Ok(BulkOutcome {
            changed: released.len(),
            skipped,
        })
    })
}

/// The end of the circle: something actually went out.
///
/// `on_day` is the day it shipped, when that is not today — a release marked
/// late, or one whose real date is known from elsewhere. It is the person's
/// own data, separate from the gesture's moment, which is when the mark was
/// recorded. See [`release::mark_released_at`].
pub fn mark_released(
    conn: &Connection,
    id: &str,
    url: Option<String>,
    on_day: Option<String>,
) -> Result<Release> {
    gesture(conn, "release.markReleased", |act| {
        act.param("id", id);
        act.param("url", url.clone());
        act.param("on_day", on_day.clone());
        act.stamped();
        let released = release::mark_released_at(act, id, url, on_day, act.at())?;
        act.journal(
            Record::new("release.released")
                .param("title", act.title_of(&released.work_id))
                .param("kind", released.kind.clone())
                .about("work", released.work_id.clone()),
        );
        act.restate(&released.work_id);
        Ok(released)
    })
}

/// Undoing the mark: it did not go out after all.
///
/// The work's status follows on its own — it is derived from the facts. The
/// link is deliberately kept; see [`release::unmark_released`].
pub fn unmark_released(conn: &Connection, id: &str) -> Result<Release> {
    gesture(conn, "release.unmarkReleased", |act| {
        act.param("id", id);
        act.stamped();
        let planned = release::unmark_released_at(act, id, act.at())?;
        act.journal(
            Record::new("release.unreleased")
                .param("title", act.title_of(&planned.work_id))
                .param("kind", planned.kind.clone())
                .about("work", planned.work_id.clone()),
        );
        act.restate(&planned.work_id);
        Ok(planned)
    })
}

/// Book exactly the plan the person approved — all of it or none of it.
///
/// One journal line for the whole batch rather than one per release: fifty
/// placements telling the story fifty times is how a history stops being
/// read. Statuses are brought in line the way the resync does it — silently,
/// with the batch line standing for all of them.
pub fn apply_layout(conn: &Connection, placements: &[layout::Placement]) -> Result<usize> {
    gesture(conn, "layout.apply", |act| {
        act.json("placements", &placements)?;
        act.stamped();
        let applied = layout::apply_at(act, placements, act.at())?;

        if applied > 0 {
            // The plan arrives in date order, but the journal line should not
            // depend on that holding.
            let from = placements.iter().map(|p| p.date.as_str()).min();
            let to = placements.iter().map(|p| p.date.as_str()).max();
            act.journal(
                Record::new("layout.applied")
                    .param("count", count(applied))
                    .param("from", from.unwrap_or_default())
                    .param("to", to.unwrap_or_default()),
            );
            let config = crate::profile::config_for(act, act.profile_id())?;
            if let Err(cause) = crate::work::status::resync(act, &config, act.profile_id()) {
                crate::log::error("layout", &format!("could not restate the works: {cause}"));
            }
        }
        Ok(applied)
    })
}

/// Warn about every release inside the coming week that could not go out
/// today — once per release and date.
///
/// The window calls this at startup and after calendar changes, passing the
/// person's local date: the backend only knows UTC, which at a negative offset
/// is already tomorrow. The entry uses once-semantics — a warning the person
/// already dismissed is not re-lit by the same sweep noticing the same gap,
/// but the release moving to a new date is a new situation and warns afresh.
///
/// Journal lines only: nothing in the workspace changes, so no operation.
pub fn warn_unready(conn: &Connection, today: &str) -> Result<usize> {
    let profile_id = super::active_profile_id(conn)?;
    let releases = release::calendar(conn, &profile_id)?;
    let unready = crate::readiness::unready_upcoming(
        &releases,
        today,
        crate::readiness::WARNING_HORIZON_DAYS,
    )?;

    for entry in &unready {
        let date = entry.release.scheduled_at.clone().unwrap_or_default();
        crate::journal::record(
            conn,
            &profile_id,
            Record::new("release.notReady")
                .param("title", entry.work_title.clone())
                .param("date", date.clone())
                .about("work", entry.release.work_id.clone())
                .once(format!("ready:{}:{date}", entry.release.id))
                .warn(),
        );
    }
    Ok(unready.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fixtures, journal, work};

    /// The batch speaks once: applying a layout writes one line with the count
    /// and the range, not one per release — and the statuses still catch up,
    /// silently, the way the resync does it.
    #[test]
    fn an_applied_layout_is_one_journal_line_and_the_statuses_follow() {
        let (conn, profile_id) = fixtures::workspace();

        let mut work_ids = Vec::new();
        for title in ["First", "Second"] {
            let work = fixtures::song(&conn, &profile_id, title);
            fixtures::score(&conn, &work.id, serde_json::json!({ "hook": 7.0 }));
            fixtures::release(&conn, &work.id, "clip", None);
            work_ids.push(work.id);
        }

        let plan = layout::plan(&conn, &profile_id, "2026-09-01").unwrap();
        apply_layout(&conn, &plan).unwrap();

        let entries = journal::list(&conn, &profile_id).unwrap();
        let batch: Vec<_> = entries
            .iter()
            .filter(|entry| entry.action == "layout.applied")
            .collect();
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].params["count"], 2);
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.action == "work.restated")
                .count(),
            0,
            "the batch line stands for the status changes"
        );

        for work_id in &work_ids {
            let restated = work::get(&conn, work_id).unwrap().unwrap();
            assert_eq!(restated.status, "scheduled");
        }
    }

    /// The whole warning path: an unready release inside the window writes one
    /// warning, and the same sweep running again — every startup does — leaves
    /// it exactly as the person left it.
    #[test]
    fn unready_warnings_are_written_once_and_stay_dismissed() {
        let (conn, profile_id) = fixtures::workspace();
        let work = fixtures::song(&conn, &profile_id, "Subject");
        fixtures::release(&conn, &work.id, "clip", Some("2026-09-03"));

        assert_eq!(warn_unready(&conn, "2026-09-01").unwrap(), 1);
        let entries = journal::list(&conn, &profile_id).unwrap();
        let warning = entries
            .iter()
            .find(|entry| entry.action == "release.notReady")
            .expect("the gap inside the window is worth a line");
        assert_eq!(warning.params["title"], "Subject");
        assert_eq!(warning.params["date"], "2026-09-03");
        assert_eq!(journal::unread_count(&conn, &profile_id).unwrap(), 1);

        journal::mark_read(&conn, &profile_id).unwrap();
        warn_unready(&conn, "2026-09-01").unwrap();

        let entries = journal::list(&conn, &profile_id).unwrap();
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.action == "release.notReady")
                .count(),
            1,
            "the same standing gap is one line, not one per startup"
        );
        assert_eq!(
            journal::unread_count(&conn, &profile_id).unwrap(),
            0,
            "a dismissed warning stays dismissed"
        );
    }

    /// A release a package plans leaves the same line in the history a hand
    /// planning it leaves. The package's copy of this gesture used to have
    /// none.
    #[test]
    fn a_planned_release_is_in_the_history() {
        let (conn, profile_id) = fixtures::workspace();
        let work = fixtures::video(&conn, &profile_id, "Harbour lights");

        let kind = crate::profile::config_for(&conn, &profile_id)
            .unwrap()
            .vocabulary("video")
            .release_kinds[0]
            .key
            .clone();
        create(
            &conn,
            NewRelease {
                work_id: work.id.clone(),
                kind,
                title: None,
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap();

        assert!(
            journal::for_entity(&conn, "work", &work.id)
                .unwrap()
                .iter()
                .any(|entry| entry.action == "release.created")
        );
    }

    #[test]
    fn a_release_of_a_kind_the_work_does_not_ship_is_refused() {
        let (conn, profile_id) = fixtures::workspace();
        let work = fixtures::song(&conn, &profile_id, "Harbour lights");

        let refused = create(
            &conn,
            NewRelease {
                work_id: work.id,
                kind: "podcast-episode".into(),
                title: None,
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap_err();

        assert_eq!(
            refused.refusal().map(|refusal| refusal.code),
            Some("release.unknownKind")
        );
    }
}
