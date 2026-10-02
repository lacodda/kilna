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
pub fn create(conn: &Connection, mut new: NewWork) -> Result<Work> {
    gesture(conn, "work.create", |act| {
        // The fields a work of this kind starts at - an audio release plays
        // the original unless someone says otherwise - filled in before the
        // work is logged, so a replay makes the work the log names rather
        // than asking a profile that may have changed since.
        let defaults = crate::profile::config_for(act, act.profile_id())?.defaults_of(&new.kind);
        if !defaults.is_empty() {
            let meta = new.meta.get_or_insert_with(serde_json::Map::new);
            for (key, value) in defaults {
                meta.entry(key).or_insert(value);
            }
        }
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

/// What making a work from another made: the work, and the release it was
/// given to go out through when its kind has a door - planned for no day yet.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
pub struct Made {
    pub work: Work,
    pub release_id: Option<String>,
    /// Where that release goes out: the door asked for, or the kind's first.
    pub door: Option<String>,
}

/// Make a work from another: a clip, an audio release or a short from a song.
///
/// The new work takes the source's overview fields that its own kind has -
/// the inputs flow once, at creation, and never again (decision of
/// 2026-09-10): what the source does afterwards is a fact the link reports,
/// not a change pushed into the work. A field that is a fact of the work's
/// own media - a length - is not taken (v0.90): a short is not as long as
/// the clip it is cut from. Its text is not copied: a video's roles are its
/// own. Its title is the kind's `made_title` - "Harbour lights (video)",
/// "Harbour lights (short 5)", numbered among the works of that kind made
/// from the same song (`publication::made_title`) - unless one is given. A
/// kind that goes out somewhere is planned its one release (ADR 0051)
/// through the door asked for, or else its first, with no day: "make an
/// audio" means "I am going to put this out", and the release is where what
/// it goes out under is written (v0.86).
///
/// Gestures as a hand would make them - the work, the link, the release - in
/// one unit, so none lands without the others.
pub fn derive(
    conn: &Connection,
    source_id: &str,
    kind: &str,
    title: Option<&str>,
    locale: Option<&str>,
    door: Option<&str>,
) -> Result<Made> {
    atomically(conn, |conn| {
        let source =
            work::get(conn, source_id)?.ok_or_else(|| Error::not_found("work", source_id))?;
        let config = crate::profile::config_for(conn, &super::active_profile_id(conn)?)?;
        let vocabulary = config.require_kind(kind)?;
        // Only the fields the new work's kind has: a stray key in the
        // source's meta, or a song's field no clip has, is not carried over.
        let meta: serde_json::Map<String, serde_json::Value> = source
            .meta
            .iter()
            .filter(|(key, _)| {
                config
                    .fields_of(kind)
                    .iter()
                    .any(|field| field.key == **key && !field.own)
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let title = match title.map(str::trim).filter(|title| !title.is_empty()) {
            Some(title) => title.to_owned(),
            None => crate::publication::made_title(
                conn,
                &config,
                kind,
                &source,
                locale.unwrap_or(crate::profile::config::SOURCE_LOCALE),
            )?,
        };
        let door = match door {
            Some(asked) => {
                config.require_release_kind(kind, asked)?;
                Some(asked.to_owned())
            }
            None => vocabulary
                .release_kinds
                .first()
                .map(|door| door.key.clone()),
        };
        let created = create(
            conn,
            NewWork {
                kind: kind.to_owned(),
                title,
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
        let release_id = match door.clone() {
            Some(door) => Some(
                super::release::create(
                    conn,
                    crate::release::NewRelease {
                        work_id: created.id.clone(),
                        kind: door,
                        scheduled_at: None,
                        meta: None,
                        scheduled_time: None,
                        time_zone: None,
                    },
                )?
                .id,
            ),
            None => None,
        };
        let work = work::get(conn, &created.id)?.unwrap_or(created);
        Ok(Made {
            work,
            release_id,
            door,
        })
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

        // What each work was made from, read while the links still stand, so
        // the songs whose status was theirs are asked again (v0.86).
        let mut sources: Vec<String> = Vec::new();
        for id in ids {
            for source in crate::link::ancestors(act, id)? {
                if !sources.contains(&source) && !ids.contains(&source) {
                    sources.push(source);
                }
            }
        }
        let discarded = crate::trash::discard_batch(act, crate::trash::Entity::Work, ids, &minted)?;
        for source in &sources {
            act.restate(source);
        }
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

        let made = derive(&conn, &song.id, "video", None, Some("en"), None).unwrap();

        assert_eq!(made.work.title, "Harbour lights (video)");
        let links = crate::link::for_work(&conn, &made.work.id).unwrap();
        assert_eq!(links.sources.len(), 1);
        let release = crate::release::get(&conn, made.release_id.as_deref().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(
            release.kind, "youtube",
            "a clip is planned through its first door"
        );
        assert_eq!(release.scheduled_at, None, "with no day yet");
        let kinds: Vec<String> = operation::latest(&conn, 3)
            .unwrap()
            .into_iter()
            .map(|op| op.kind)
            .collect();
        assert_eq!(kinds, ["release.create", "link.create", "work.create"]);
    }

    /// Made from a song, a work is named the way its kind names what is made -
    /// in the window's language, numbered among its kind made from the same
    /// song - and starts with the fields its kind has: the song's, and the
    /// audio's variant at the original.
    #[test]
    fn a_made_work_is_named_numbered_and_given_its_kinds_fields() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        work::update(
            &conn,
            &song.id,
            WorkPatch {
                meta: serde_json::json!({ "bpm": 96, "duration": 214, "stray": "no field" })
                    .as_object()
                    .cloned(),
                ..WorkPatch::default()
            },
        )
        .unwrap();

        let first = derive(&conn, &song.id, "short", None, Some("ru"), None).unwrap();
        let second = derive(&conn, &song.id, "short", None, Some("ru"), None).unwrap();
        let clip = derive(&conn, &song.id, "video", None, Some("ru"), None).unwrap();
        let again = derive(&conn, &song.id, "video", None, Some("en"), None).unwrap();
        let audio = derive(&conn, &song.id, "audio", None, None, None).unwrap();
        let named = derive(
            &conn,
            &song.id,
            "short",
            Some("  The hook  "),
            Some("ru"),
            None,
        )
        .unwrap();

        assert_eq!(first.work.title, "Harbour lights (short)");
        assert_eq!(second.work.title, "Harbour lights (short 2)");
        assert_eq!(
            clip.work.title, "Harbour lights (video)",
            "one name in every language"
        );
        assert_eq!(again.work.title, "Harbour lights (video 2)");
        assert_eq!(audio.work.title, "Harbour lights (audio)");
        assert_eq!(named.work.title, "The hook", "a title given wins");
        assert_eq!(
            audio.work.meta,
            serde_json::json!({ "bpm": 96, "variant": "original" })
                .as_object()
                .cloned()
                .unwrap()
        );
        assert_eq!(clip.work.meta.get("variant"), None, "a clip has no variant");
        assert_eq!(
            first.work.meta.get("duration"),
            None,
            "a short is not as long as the song it is cut from"
        );
        let door = crate::release::get(&conn, audio.release_id.as_deref().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(
            (door.kind.as_str(), door.scheduled_at.as_deref()),
            ("youtube", None)
        );
    }

    /// The place is chosen when the publication is made: an audio for the
    /// streaming services is planned there, and a door the kind does not
    /// have is refused before anything is written.
    #[test]
    fn a_made_work_goes_out_through_the_door_asked_for() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");

        let audio = derive(&conn, &song.id, "audio", None, None, Some("streaming")).unwrap();
        let door = crate::release::get(&conn, audio.release_id.as_deref().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(door.kind, "streaming");

        let before = operation::count(&conn).unwrap();
        let refused = derive(&conn, &song.id, "audio", None, None, Some("premiere")).unwrap_err();
        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("release.unknownKind")
        );
        assert_eq!(operation::count(&conn).unwrap(), before);
    }

    /// A work that goes out nowhere is made with no release to plan.
    #[test]
    fn a_made_work_of_a_kind_without_doors_plans_nothing() {
        let (conn, profile_id) = fixtures::workspace();
        let clip = fixtures::video(&conn, &profile_id, "Harbour lights — clip");

        let song = derive(&conn, &clip.id, "song", None, Some("en"), None).unwrap();

        assert_eq!(song.release_id, None);
        assert_eq!(song.work.title, "Harbour lights — clip");
    }

    #[test]
    fn a_derived_work_of_an_unknown_kind_is_refused_before_anything_is_written() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let before = operation::count(&conn).unwrap();

        let refused = derive(&conn, &song.id, "opera", None, None, None).unwrap_err();

        assert_eq!(refused.refusal().map(|r| r.code), Some("work.unknownKind"));
        assert_eq!(operation::count(&conn).unwrap(), before);
    }
}
