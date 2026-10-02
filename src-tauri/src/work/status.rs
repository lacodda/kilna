//! Where a work's status comes from.
//!
//! The predecessor let four places write the field from the edges — scoring set
//! one value, scheduling another, the card a third — and it drifted until it
//! meant nothing. The deal here is narrower: **the automation derives the
//! status from facts, and setting it by hand pins it.** A pinned work is
//! stepped over entirely, so the one thing a person said out loud is the one
//! thing that is never overwritten.
//!
//! Facts, in descending finality: a release has gone out → a release holds a
//! slot → a score exists → nothing yet. Which word each of those maps to is the
//! profile's business, not this module's ([`Derive`]).

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::error::Result;
use crate::profile::config::{Derive, ProfileConfig};
use crate::release;
use crate::time::now;

/// What the facts say a work's status should be.
///
/// `None` when the profile names no status for that meaning: a profile whose
/// statuses are all `manual` derives nothing at all, which is a legitimate way
/// to run the app by hand.
pub fn derive_for(
    conn: &Connection,
    config: &ProfileConfig,
    work_id: &str,
) -> Result<Option<String>> {
    // The word for the fact is the work's kind's word: a video and a song may
    // call "released" differently, or one of them may have no word for it.
    let kind: String = conn.query_row(
        "SELECT kind FROM work WHERE id = ?1",
        params![work_id],
        |row| row.get(0),
    )?;
    let vocabulary = config.vocabulary(&kind);
    let meaning = fact_for(conn, work_id, vocabulary.has_doors())?;
    Ok(status_named(vocabulary, meaning))
}

/// The works whose releases speak for this one: itself and, for a work whose
/// kind has no door of its own, everything made from it (v0.86). A song goes
/// out as its clip, its audio and its shorts - so it has gone out when any of
/// them has, and is booked when any of them holds a slot. A kind with doors
/// speaks only for itself: a clip is not released because a short cut from it
/// went out. The work's own releases always count, so a song that still holds
/// one - written before its doors came off - is not silently unreleased.
pub fn speaking_for(conn: &Connection, work_id: &str, has_doors: bool) -> Result<Vec<String>> {
    let mut works = vec![work_id.to_owned()];
    if !has_doors {
        works.extend(
            crate::link::descendants(conn, work_id)?
                .into_iter()
                .map(|(id, _)| id),
        );
    }
    Ok(works)
}

/// The strongest fact that is true of a work right now.
fn fact_for(conn: &Connection, work_id: &str, has_doors: bool) -> Result<Derive> {
    let works = speaking_for(conn, work_id, has_doors)?;
    let any = |sql: &str| -> Result<bool> {
        for id in &works {
            let found = conn
                .query_row(sql, params![id], |_| Ok(true))
                .optional()?
                .unwrap_or(false);
            if found {
                return Ok(true);
            }
        }
        Ok(false)
    };

    let released = any(&format!(
        "SELECT 1 FROM release WHERE work_id = ?1 AND status = '{}' LIMIT 1",
        release::RELEASED
    ))?;
    if released {
        return Ok(Derive::Released);
    }

    // A planned release without a date is an intention, not a slot: it holds
    // nothing in the calendar, so it does not make a work "scheduled".
    let scheduled = any(&format!(
        "SELECT 1 FROM release
          WHERE work_id = ?1 AND status = '{}' AND scheduled_at IS NOT NULL LIMIT 1",
        release::PLANNED
    ))?;
    if scheduled {
        return Ok(Derive::Scheduled);
    }

    let scored: bool = conn
        .query_row(
            "SELECT 1 FROM work_score WHERE work_id = ?1 LIMIT 1",
            params![work_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if scored {
        return Ok(Derive::Scored);
    }

    Ok(Derive::Draft)
}

/// The profile's word for a meaning.
///
/// `Manual` is never looked up: it is the absence of a derivation, and a
/// profile that maps a word to it is saying "only a person puts this here".
fn status_named(kind: &crate::profile::config::WorkKind, meaning: Derive) -> Option<String> {
    kind.status_meaning(meaning)
        .map(|status| status.key.clone())
}

/// Recompute one work's status, unless a person pinned it.
///
/// Returns the status the work now holds. Called after anything that changes a
/// fact — a score, a schedule, a release going out — so the field never has to
/// be written by the code that changed the fact.
pub fn refresh(conn: &Connection, config: &ProfileConfig, work_id: &str) -> Result<Option<Change>> {
    refresh_at(conn, config, work_id, &now())
}

/// Recompute one work's status with the change's timestamp already decided.
///
/// The seam a replay comes back through: live, `refresh` stamps `now()`;
/// replaying, the log supplies the moment the first run recorded. See ADR 0014.
pub fn refresh_at(
    conn: &Connection,
    config: &ProfileConfig,
    work_id: &str,
    at: &str,
) -> Result<Option<Change>> {
    derive_at(conn, config, work_id, at, false)
}

/// Recompute one work's status. `handed_back` is a person giving the status
/// back to the automation: then a word only a person can say - shelved - is
/// no longer theirs to keep, and the facts speak.
fn derive_at(
    conn: &Connection,
    config: &ProfileConfig,
    work_id: &str,
    at: &str,
    handed_back: bool,
) -> Result<Option<Change>> {
    let row: Option<(String, String, Option<String>, String)> = conn
        .query_row(
            "SELECT title, status, status_pinned_at, kind FROM work WHERE id = ?1",
            params![work_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;

    let Some((title, current, pinned_at, kind)) = row else {
        return Ok(None);
    };
    if pinned_at.is_some() || (!handed_back && said_by_hand(config, &kind, &current)) {
        return Ok(None);
    }

    let Some(derived) = derive_for(conn, config, work_id)? else {
        return Ok(None);
    };
    if derived == current {
        return Ok(None);
    }

    conn.execute(
        "UPDATE work SET status = ?2, updated_at = ?3 WHERE id = ?1",
        params![work_id, derived, at],
    )?;

    Ok(Some(Change {
        work_id: work_id.to_owned(),
        title,
        from: current,
        to: derived,
    }))
}

/// Whether a status is one only a person can say - shelved, on hold - and so
/// a person's word whether or not it carries a pin. Setting one by hand pins
/// it; one that arrived another way (an import of the predecessor's
/// catalogue) is the same decision, and the automation has no fact that could
/// replace it with anything truer.
fn said_by_hand(config: &ProfileConfig, kind: &str, status: &str) -> bool {
    config
        .vocabulary(kind)
        .statuses
        .iter()
        .any(|known| known.key == status && known.derive == Derive::Manual)
}

/// A status the automation would change, or did.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, ts_rs::TS)]
pub struct Change {
    pub work_id: String,
    /// Carried along because the dry run is read as a list of works, not of
    /// ids: "Winter road: draft → scored" is a decision someone can make.
    pub title: String,
    pub from: String,
    pub to: String,
}

/// What a full recompute would change, without changing it.
///
/// The dry run exists because the alternative is finding out: a profile whose
/// `derive` roles are wrong would quietly restate every work in the workspace,
/// and there is no undo for "all of them at once".
pub fn drift(conn: &Connection, config: &ProfileConfig, profile_id: &str) -> Result<Vec<Change>> {
    let mut statement = conn.prepare(
        "SELECT id, title, status, kind FROM work
          WHERE profile_id = ?1 AND status_pinned_at IS NULL
          ORDER BY title",
    )?;
    let works: Vec<(String, String, String, String)> = statement
        .query_map(params![profile_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut changes = Vec::new();
    for (work_id, title, current, kind) in works {
        if said_by_hand(config, &kind, &current) {
            continue;
        }
        let Some(derived) = derive_for(conn, config, &work_id)? else {
            continue;
        };
        if derived != current {
            changes.push(Change {
                work_id,
                title,
                from: current,
                to: derived,
            });
        }
    }
    Ok(changes)
}

/// Apply what [`drift`] found. Pinned works are excluded by `drift` itself, so
/// this cannot reach one.
pub fn resync(conn: &Connection, config: &ProfileConfig, profile_id: &str) -> Result<Vec<Change>> {
    resync_at(conn, config, profile_id, &now())
}

/// Apply what [`drift`] found with the change's timestamp already decided.
///
/// The seam a replay comes back through: live, `resync` stamps `now()`;
/// replaying, the log supplies the moment the first run recorded. See ADR 0014.
pub fn resync_at(
    conn: &Connection,
    config: &ProfileConfig,
    profile_id: &str,
    at: &str,
) -> Result<Vec<Change>> {
    let changes = drift(conn, config, profile_id)?;
    for change in &changes {
        conn.execute(
            "UPDATE work SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![change.work_id, change.to, at],
        )?;
    }
    Ok(changes)
}

/// Hand the status back to the automation and recompute it at once.
///
/// Unpinning without recomputing would leave the hand-set word in place with
/// nothing claiming it — the state the whole arrangement exists to avoid.
pub fn unpin(conn: &Connection, config: &ProfileConfig, work_id: &str) -> Result<Option<Change>> {
    unpin_at(conn, config, work_id, &now())
}

/// Unpin a status with the change's timestamp already decided.
///
/// The seam a replay comes back through: live, `unpin` stamps `now()`;
/// replaying, the log supplies the moment the first run recorded. See ADR 0014.
pub fn unpin_at(
    conn: &Connection,
    config: &ProfileConfig,
    work_id: &str,
    at: &str,
) -> Result<Option<Change>> {
    conn.execute(
        "UPDATE work SET status_pinned_at = NULL WHERE id = ?1",
        params![work_id],
    )?;
    derive_at(conn, config, work_id, at, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::profile;
    use crate::release::{self as releases, NewRelease};
    use crate::score::{self, NewScore};
    use crate::work::{self, WorkPatch};
    use serde_json::json;

    /// [`fixtures::workspace`], and the profile's config alongside it - most
    /// tests here derive a status from it.
    fn workspace() -> (Connection, String, ProfileConfig) {
        let (conn, profile_id) = fixtures::workspace();
        let config = profile::config_for(&conn, &profile_id).unwrap();
        (conn, profile_id, config)
    }

    fn a_work(conn: &Connection, profile_id: &str, title: &str) -> String {
        fixtures::song(conn, profile_id, title).id
    }

    fn a_release(conn: &Connection, work_id: &str) -> String {
        releases::create(
            conn,
            NewRelease {
                work_id: work_id.to_owned(),
                kind: "clip".into(),
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap()
        .id
    }

    fn a_score(conn: &Connection, work_id: &str) {
        score::create(
            conn,
            work_id,
            NewScore {
                axes: json!({ "hook": 8.0 }).as_object().cloned().unwrap(),
                version_id: None,
                note: None,
                rater: None,
            },
        )
        .unwrap();
    }

    fn status_of(conn: &Connection, work_id: &str) -> String {
        work::get(conn, work_id).unwrap().unwrap().status
    }

    #[test]
    fn a_new_work_is_a_draft() {
        let (conn, profile_id, config) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");

        assert_eq!(
            derive_for(&conn, &config, &work_id).unwrap().unwrap(),
            "draft"
        );
    }

    #[test]
    fn a_score_makes_a_work_scored() {
        let (conn, profile_id, config) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        a_score(&conn, &work_id);

        refresh(&conn, &config, &work_id).unwrap();
        assert_eq!(status_of(&conn, &work_id), "scored");
    }

    /// A release with no date is an intention. It holds nothing in the calendar,
    /// so it must not read as scheduled — otherwise creating a release ahead of
    /// deciding when it goes out would silently claim it was booked.
    #[test]
    fn a_release_without_a_date_does_not_schedule_a_work() {
        let (conn, profile_id, config) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        a_score(&conn, &work_id);
        a_release(&conn, &work_id);

        refresh(&conn, &config, &work_id).unwrap();
        assert_eq!(status_of(&conn, &work_id), "scored");
    }

    #[test]
    fn a_slot_makes_a_work_scheduled() {
        let (conn, profile_id, config) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        let release_id = a_release(&conn, &work_id);
        releases::schedule(&conn, &release_id, "2026-09-01").unwrap();

        refresh(&conn, &config, &work_id).unwrap();
        assert_eq!(status_of(&conn, &work_id), "scheduled");
    }

    /// Finality wins over recency: a work with one release out and another
    /// booked is released, not scheduled.
    #[test]
    fn going_out_outranks_being_booked() {
        let (conn, profile_id, config) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        // A publication goes out once (ADR 0051): what went out and what is
        // booked are two works made from the song.
        let made = |title: &str| {
            let made = fixtures::work(&conn, &profile_id, "audio", title);
            crate::link::create(
                &conn,
                &profile_id,
                crate::link::NewLink {
                    work_id: made.id.clone(),
                    source_id: work_id.clone(),
                    role: None,
                    source_version_id: None,
                },
            )
            .unwrap();
            made.id
        };
        let gone = a_release(&conn, &made("Subject (audio)"));
        releases::mark_released(&conn, &gone, None, None).unwrap();
        let booked = a_release(&conn, &made("Subject (audio 2)"));
        releases::schedule(&conn, &booked, "2026-09-01").unwrap();

        refresh(&conn, &config, &work_id).unwrap();
        assert_eq!(status_of(&conn, &work_id), "released");
    }

    /// The deal in one test: a hand-set status is not overwritten, however
    /// loudly the facts disagree with it.
    #[test]
    fn the_automation_steps_over_a_pinned_work() {
        let (conn, profile_id, config) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        work::update(
            &conn,
            &work_id,
            WorkPatch {
                status: Some("shelved".into()),
                ..Default::default()
            },
        )
        .unwrap();
        a_score(&conn, &work_id);

        assert_eq!(refresh(&conn, &config, &work_id).unwrap(), None);
        assert_eq!(status_of(&conn, &work_id), "shelved");
    }

    /// A status only a person can say - shelved - stands even without a pin,
    /// the way one arrives from an import: the automation has no fact that
    /// could say something truer.
    #[test]
    fn a_status_only_a_person_can_say_stands_without_a_pin() {
        let (conn, profile_id, config) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        conn.execute(
            "UPDATE work SET status = 'shelved' WHERE id = ?1",
            params![work_id],
        )
        .unwrap();
        a_score(&conn, &work_id);

        assert_eq!(refresh(&conn, &config, &work_id).unwrap(), None);
        assert_eq!(status_of(&conn, &work_id), "shelved");
        assert!(
            drift(&conn, &config, &profile_id).unwrap().is_empty(),
            "nor does a full recompute offer to change it"
        );
    }

    #[test]
    fn unpinning_hands_the_status_back_and_recomputes_it_at_once() {
        let (conn, profile_id, config) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        a_score(&conn, &work_id);
        work::update(
            &conn,
            &work_id,
            WorkPatch {
                status: Some("shelved".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let change = unpin(&conn, &config, &work_id).unwrap().unwrap();
        assert_eq!(
            (change.from.as_str(), change.to.as_str()),
            ("shelved", "scored")
        );
        assert_eq!(status_of(&conn, &work_id), "scored");
        assert!(
            work::get(&conn, &work_id)
                .unwrap()
                .unwrap()
                .status_pinned_at
                .is_none(),
            "unpinning left the pin in place"
        );
    }

    /// A dry run has to be exactly that: it reports and changes nothing.
    #[test]
    fn drift_reports_without_touching_anything() {
        let (conn, profile_id, config) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        a_score(&conn, &work_id);

        let found = drift(&conn, &config, &profile_id).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].to, "scored");
        assert_eq!(
            status_of(&conn, &work_id),
            "draft",
            "the dry run wrote to the database"
        );

        let applied = resync(&conn, &config, &profile_id).unwrap();
        assert_eq!(applied, found);
        assert_eq!(status_of(&conn, &work_id), "scored");
        assert!(
            drift(&conn, &config, &profile_id).unwrap().is_empty(),
            "a second pass still found work to do"
        );
    }

    #[test]
    fn drift_leaves_pinned_works_out_of_its_report() {
        let (conn, profile_id, config) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        a_score(&conn, &work_id);
        work::update(
            &conn,
            &work_id,
            WorkPatch {
                status: Some("shelved".into()),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(drift(&conn, &config, &profile_id).unwrap().is_empty());
    }

    /// A profile that names no word for a meaning derives nothing rather than
    /// inventing one: writing a status the vocabulary does not contain would put
    /// a value in the column that no screen can render.
    #[test]
    fn a_meaning_the_profile_does_not_name_derives_nothing() {
        let (conn, profile_id, mut config) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        a_score(&conn, &work_id);
        config.work_kinds[0]
            .statuses
            .retain(|status| status.key != "scored");

        assert_eq!(derive_for(&conn, &config, &work_id).unwrap(), None);
        assert_eq!(refresh(&conn, &config, &work_id).unwrap(), None);
        assert_eq!(status_of(&conn, &work_id), "draft");
    }
}
