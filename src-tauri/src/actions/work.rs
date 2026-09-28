//! Gestures on a work: making one, editing it, holding its status or tier by
//! hand, and the batches the catalogue sends.

use rusqlite::Connection;

use super::{BulkOutcome, Skipped, count, gesture};
use crate::db::unit::atomically;
use crate::error::{Error, Reason, Result};
use crate::journal::Record;
use crate::link::NewLink;
use crate::work::{self, NewWork, Work, WorkPatch};

/// Make a work.
///
/// The id and the moment are minted by the gesture, so the same values reach
/// the row and the log: a replay rebuilds the work under the id that every
/// version, score and release already names (ADR 0014).
pub fn create(conn: &Connection, new: NewWork) -> Result<Work> {
    gesture(conn, "work.create", |act| {
        act.json("work", &new)?;
        let minted = act.mint();
        let created = work::create_minted(act, act.profile_id(), new, minted)?;
        act.journal(
            Record::new("work.created")
                .param("title", created.title.clone())
                .about("work", created.id.clone()),
        );
        Ok(created)
    })
}

/// Edit a work.
///
/// Renaming and moving to another status are journalled separately, and with
/// both values: "renamed to X" without the old name is a line nobody can act
/// on, and a status change is the one edit the calendar and the catalogue
/// both react to.
pub fn update(conn: &Connection, id: &str, patch: WorkPatch) -> Result<Work> {
    gesture(conn, "work.update", |act| {
        let before = work::get(act, id)?;
        act.param("id", id);
        act.json("patch", &patch)?;
        act.before(before.as_ref(), &patch)?;
        act.stamped();

        let updated = work::update_at(act, id, patch, act.at())?;

        if let Some(before) = before {
            if before.title != updated.title {
                act.journal(
                    Record::new("work.renamed")
                        .param("from", before.title)
                        .param("to", updated.title.clone())
                        .about("work", updated.id.clone()),
                );
            }
            if before.status != updated.status {
                act.journal(
                    Record::new("work.status")
                        .param("title", updated.title.clone())
                        .param("from", before.status)
                        .param("to", updated.status.clone())
                        .about("work", updated.id.clone()),
                );
            }
        }
        Ok(updated)
    })
}

/// Make a version the work's current one.
///
/// A `work.update` of the one field, like any other edit of a work. It was
/// written past the log until v0.76.1: Ctrl+Z after "make current" took back
/// the edit before it, and a replay rebuilt the old current version.
pub fn set_current_version(conn: &Connection, work_id: &str, version_id: &str) -> Result<Work> {
    // Refused before anything is written or logged: another work's text is
    // not this work's version.
    crate::work::version::check_belongs(conn, work_id, version_id)?;
    update(
        conn,
        work_id,
        WorkPatch {
            current_version_id: Some(Some(version_id.to_owned())),
            ..WorkPatch::default()
        },
    )
}

/// Bring every work's status in line with the facts: what
/// [`work::status::drift`] reported.
pub fn resync_statuses(conn: &Connection) -> Result<Vec<work::status::Change>> {
    gesture(conn, "status.resync", |act| {
        act.stamped();
        let config = crate::profile::config_for(act, act.profile_id())?;
        let changes = work::status::resync_at(act, &config, act.profile_id(), act.at())?;
        if !changes.is_empty() {
            act.journal(Record::new("status.resynced").param("count", count(changes.len())));
        }
        Ok(changes)
    })
}

/// Hand one work's status back to the automation.
pub fn unpin_status(conn: &Connection, id: &str) -> Result<Work> {
    gesture(conn, "work.unpinStatus", |act| {
        act.param("id", id);
        act.stamped();
        let config = crate::profile::config_for(act, act.profile_id())?;
        if let Some(change) = work::status::unpin_at(act, &config, id, act.at())? {
            act.journal(
                Record::new("work.restated")
                    .param("title", change.title)
                    .param("from", change.from)
                    .param("to", change.to)
                    .about("work", id.to_owned()),
            );
        }
        work::get(act, id)?.ok_or_else(|| Error::not_found("work", id))
    })
}

/// Hold a work at a tier by hand, with the reason on record.
pub fn pin_tier(conn: &Connection, id: &str, tier: &str, reason: &str) -> Result<Work> {
    gesture(conn, "work.pinTier", |act| {
        let before = work::get(act, id)?;
        // The prior pin travels with the intent so an undo can put it back
        // exactly as it stood — `null` means there was no pin to restore.
        let (before_tier, before_reason) =
            before
                .as_ref()
                .map_or((serde_json::Value::Null, serde_json::Value::Null), |work| {
                    (
                        serde_json::to_value(&work.tier_pinned).unwrap_or(serde_json::Value::Null),
                        serde_json::to_value(&work.tier_pin_reason)
                            .unwrap_or(serde_json::Value::Null),
                    )
                });
        act.param("id", id);
        act.param("tier", tier);
        act.param("reason", reason);
        act.param("beforeTier", before_tier);
        act.param("beforeReason", before_reason);
        act.stamped();

        let pinned = work::pin_tier_at(act, id, tier, reason, act.at())?;
        act.journal(
            Record::new("tier.pinned")
                .param("title", pinned.title.clone())
                .param("tier", tier)
                .param("reason", reason.trim().to_owned())
                .about("work", id.to_owned()),
        );
        Ok(pinned)
    })
}

/// Let the score speak for the work's tier again.
pub fn unpin_tier(conn: &Connection, id: &str) -> Result<Work> {
    gesture(conn, "work.unpinTier", |act| {
        act.param("id", id);
        act.stamped();
        let freed = work::unpin_tier_at(act, id, act.at())?;
        act.journal(
            Record::new("tier.unpinned")
                .param("title", freed.title.clone())
                .about("work", id.to_owned()),
        );
        Ok(freed)
    })
}

/// Move several works to another status at once.
///
/// Chosen by hand, so each one pins exactly as the single-work path does —
/// otherwise the automation would derive the old status straight back and the
/// batch would appear to do nothing. One journal line for the batch, not one
/// per work: a person who moved twenty drafts did one thing, and twenty lines
/// would bury the day's real events.
pub fn set_status(conn: &Connection, work_ids: &[String], status: &str) -> Result<BulkOutcome> {
    gesture(conn, "work.setStatusBatch", |act| {
        // Refused before anything changes rather than once per work: a status
        // the profile does not have is a mistake about the whole batch. Any
        // kind's word will do here: the batch may hold songs and videos, and
        // a work whose kind lacks the word is skipped below, not refused.
        let config = crate::profile::config_for(act, act.profile_id())?;
        if !config
            .all_statuses()
            .iter()
            .any(|known| known.key == status)
        {
            return Err(Error::refused("work.unknownStatus").param("status", status));
        }

        let mut moved: Vec<String> = Vec::new();
        let mut skipped: Vec<Skipped> = Vec::new();
        for work_id in work_ids {
            let before = work::get(act, work_id)?;
            let title = before.as_ref().map(|work| work.title.clone());
            if before.as_ref().is_some_and(|work| work.status == status) {
                skipped.push(Skipped {
                    id: work_id.clone(),
                    title,
                    reason: Reason::of("skip.alreadyThere"),
                });
                continue;
            }
            let patch = WorkPatch {
                status: Some(status.to_owned()),
                ..WorkPatch::default()
            };
            // One work failing must not take the batch with it — the others
            // are unrelated, and a half-applied batch is more useful than
            // none. Its own unit, so what it wrote before failing goes too.
            match atomically(act, |tx| work::update_at(tx, work_id, patch, act.at())) {
                Ok(_) => moved.push(work_id.clone()),
                Err(cause) => skipped.push(Skipped {
                    id: work_id.clone(),
                    title,
                    reason: cause.reason(),
                }),
            }
        }

        // Only the works actually moved go in the log: a work already at this
        // status was skipped, and recording it would replay `updated_at` onto
        // a row a repeat run never touched.
        if moved.is_empty() {
            act.unchanged();
        } else {
            act.json("workIds", &moved)?;
            act.param("status", status);
            act.stamped();
            act.journal(
                Record::new("work.statusBatch")
                    .param("to", status)
                    .param("count", count(moved.len())),
            );
        }

        Ok(BulkOutcome {
            changed: moved.len(),
            skipped,
        })
    })
}

/// A second attempt at a video: the same donor, the same board, its own life.
///
/// Not "a version of the whole video" — a scene belongs to a work, and the
/// second attempt is a second work (decision of 2026-09-11). The first one is
/// left exactly as it was, which is the point: the two are compared.
pub fn clone(conn: &Connection, work_id: &str, title: &str) -> Result<crate::clone::Cloned> {
    gesture(conn, "work.clone", |act| {
        act.param("workId", work_id);
        act.param("title", title);
        // The new work's id travels in the operation: a replay lands the
        // clone under the id everything else already names, and an undo
        // knows which work to discard.
        let minted = act.mint();
        let made = crate::clone::clone_work_minted(act, work_id, title, minted)?;
        act.journal(
            Record::new("work.cloned")
                .param("title", made.work.title.clone())
                .param("scenes", made.scenes.to_string())
                .about("work", made.work.id.clone()),
        );
        Ok(made)
    })
}

/// Make a work from another: a video from a song.
///
/// The new work takes the source's title and the overview fields the profile
/// has — the inputs flow once, at creation, and never again (decision of
/// 2026-09-10): what the source does afterwards is a fact the link reports,
/// not a change pushed into the work. Its text is not copied: a video's roles
/// are its own. Two gestures, as a hand would make them - the work, then the
/// link - in one unit, so neither lands without the other.
pub fn derive(conn: &Connection, source_id: &str, kind: &str, title: Option<&str>) -> Result<Work> {
    atomically(conn, |conn| {
        let source =
            work::get(conn, source_id)?.ok_or_else(|| Error::not_found("work", source_id))?;
        let config = crate::profile::config_for(conn, &super::active_profile_id(conn)?)?;
        config.require_kind(kind)?;
        // Only fields the profile has: a stray key in the source's meta is not
        // carried into a new work.
        let meta: serde_json::Map<String, serde_json::Value> = source
            .meta
            .iter()
            .filter(|(key, _)| {
                config
                    .work_meta_fields
                    .iter()
                    .any(|field| field.key == **key)
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let created = create(
            conn,
            NewWork {
                kind: kind.to_owned(),
                title: title
                    .map(|title| title.trim().to_owned())
                    .filter(|title| !title.is_empty())
                    .unwrap_or_else(|| source.title.clone()),
                meta: Some(meta),
                ..NewWork::default()
            },
        )?;
        super::link::create(
            conn,
            NewLink {
                work_id: created.id.clone(),
                source_id: source_id.to_owned(),
                role: None,
                source_version_id: None,
            },
        )?;
        Ok(created)
    })
}

/// Move several works to the trash at once.
///
/// Each still gets its own trash entry, because an entry snapshots one entity —
/// so each is separately restorable, and the window offers a single undo across
/// the batch. One operation for the whole batch: an undo has one gesture to
/// reverse, not one per work.
///
/// All of them share one moment. Minting each entry with its own `now()` would
/// put several timestamps into a single operation, and the log has one field to
/// keep them in — so a rebuild would have to invent the rest. One gesture
/// happened at one time; the entries say so.
pub fn discard(conn: &Connection, ids: &[String]) -> Result<Discarded> {
    gesture(conn, "work.discardBatch", |act| {
        // Titles are read before anything is discarded — afterwards there is
        // nothing left to ask.
        let titles: Vec<String> = ids.iter().map(|id| act.title_of(id)).collect();
        let minted: Vec<crate::minted::Minted> = ids.iter().map(|_| act.fresh()).collect();
        let entry_ids: Vec<String> = minted.iter().map(|m| m.id().to_owned()).collect();
        act.json("workIds", &ids)?;
        act.json("entryIds", &entry_ids)?;
        act.stamped();

        let discarded = crate::trash::discard_batch(act, crate::trash::Entity::Work, ids, &minted)?;
        let skipped: Vec<Skipped> = discarded
            .failed
            .iter()
            .map(|(id, cause)| Skipped {
                id: id.clone(),
                title: ids
                    .iter()
                    .position(|candidate| candidate == id)
                    .map(|index| titles[index].clone())
                    .filter(|title| !title.is_empty()),
                reason: cause.reason(),
            })
            .collect();

        match discarded.done.as_slice() {
            [] => {}
            [(id, _)] => {
                let title = ids
                    .iter()
                    .position(|candidate| candidate == id)
                    .map(|index| titles[index].clone())
                    .filter(|title| !title.is_empty())
                    .unwrap_or_else(|| id.clone());
                act.journal(
                    Record::new("work.deleted")
                        .param("title", title)
                        .about("work", id.clone()),
                );
            }
            many => act.journal(Record::new("work.deletedBatch").param("count", count(many.len()))),
        }

        Ok(Discarded {
            entries: discarded.done.into_iter().map(|(_, entry)| entry).collect(),
            skipped,
        })
    })
}

/// What a batch deletion did: the trash entries an undo names, and each work
/// that stayed, with why.
#[derive(Debug, Clone, PartialEq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct Discarded {
    pub entries: Vec<String>,
    pub skipped: Vec<Skipped>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::operation;

    #[test]
    fn a_batch_names_what_it_passed_over_and_why() {
        let (conn, profile_id) = fixtures::workspace();
        let drafted = fixtures::song(&conn, &profile_id, "Drafted");
        let already = fixtures::song(&conn, &profile_id, "Already there");
        work::update_at(
            &conn,
            &already.id,
            WorkPatch {
                status: Some("scored".into()),
                ..WorkPatch::default()
            },
            &crate::time::now(),
        )
        .unwrap();

        let outcome = set_status(
            &conn,
            &[drafted.id.clone(), already.id.clone(), "gone".into()],
            "scored",
        )
        .unwrap();

        assert_eq!(outcome.changed, 1);
        let reasons: Vec<(&str, &str)> = outcome
            .skipped
            .iter()
            .map(|skip| (skip.id.as_str(), skip.reason.key.as_str()))
            .collect();
        assert_eq!(
            reasons,
            [
                (already.id.as_str(), "skip.alreadyThere"),
                ("gone", "error.notFound"),
            ]
        );
        let logged = operation::latest(&conn, 1).unwrap().remove(0);
        assert_eq!(logged.kind, "work.setStatusBatch");
        assert_eq!(
            logged.params["workIds"],
            serde_json::json!([drafted.id]),
            "only the work that moved is in the log"
        );
    }

    #[test]
    fn a_batch_that_moved_nothing_records_nothing() {
        let (conn, profile_id) = fixtures::workspace();
        let before = operation::count(&conn).unwrap();
        let outcome = set_status(&conn, &["gone".into()], "draft").unwrap();
        assert_eq!(outcome.changed, 0);
        assert_eq!(operation::count(&conn).unwrap(), before);
        let _ = profile_id;
    }

    #[test]
    fn a_derived_work_and_its_link_land_together() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");

        let video = derive(&conn, &song.id, "video", None).unwrap();

        assert_eq!(video.title, "Harbour lights");
        let links = crate::link::for_work(&conn, &video.id).unwrap();
        assert_eq!(links.sources.len(), 1);
        let kinds: Vec<String> = operation::latest(&conn, 2)
            .unwrap()
            .into_iter()
            .map(|op| op.kind)
            .collect();
        assert_eq!(kinds, ["link.create", "work.create"]);
    }

    #[test]
    fn a_derived_work_of_an_unknown_kind_is_refused_before_anything_is_written() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let before = operation::count(&conn).unwrap();

        let refused = derive(&conn, &song.id, "opera", None).unwrap_err();

        assert_eq!(refused.refusal().map(|r| r.code), Some("work.unknownKind"));
        assert_eq!(operation::count(&conn).unwrap(), before);
    }
}
