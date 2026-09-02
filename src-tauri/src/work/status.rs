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
    let meaning = fact_for(conn, work_id)?;
    Ok(status_named(config, meaning))
}

/// The strongest fact that is true of a work right now.
fn fact_for(conn: &Connection, work_id: &str) -> Result<Derive> {
    let released: bool = conn
        .query_row(
            "SELECT 1 FROM release WHERE work_id = ?1 AND status = ?2 LIMIT 1",
            params![work_id, release::RELEASED],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if released {
        return Ok(Derive::Released);
    }

    // A planned release without a date is an intention, not a slot: it holds
    // nothing in the calendar, so it does not make a work "scheduled".
    let scheduled: bool = conn
        .query_row(
            "SELECT 1 FROM release
              WHERE work_id = ?1 AND status = ?2 AND scheduled_at IS NOT NULL LIMIT 1",
            params![work_id, release::PLANNED],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
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
fn status_named(config: &ProfileConfig, meaning: Derive) -> Option<String> {
    if meaning == Derive::Manual {
        return None;
    }
    config
        .statuses
        .iter()
        .find(|status| status.derive == meaning)
        .map(|status| status.key.clone())
}

/// Recompute one work's status, unless a person pinned it.
///
/// Returns the status the work now holds. Called after anything that changes a
/// fact — a score, a schedule, a release going out — so the field never has to
/// be written by the code that changed the fact.
pub fn refresh(conn: &Connection, config: &ProfileConfig, work_id: &str) -> Result<Option<Change>> {
    let row: Option<(String, String, Option<String>)> = conn
        .query_row(
            "SELECT title, status, status_pinned_at FROM work WHERE id = ?1",
            params![work_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;

    let Some((title, current, pinned_at)) = row else {
        return Ok(None);
    };
    if pinned_at.is_some() {
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
        params![work_id, derived, now()],
    )?;

    Ok(Some(Change {
        work_id: work_id.to_owned(),
        title,
        from: current,
        to: derived,
    }))
}

/// A status the automation would change, or did.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
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
        "SELECT id, title, status FROM work
          WHERE profile_id = ?1 AND status_pinned_at IS NULL
          ORDER BY title",
    )?;
    let works: Vec<(String, String, String)> = statement
        .query_map(params![profile_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut changes = Vec::new();
    for (work_id, title, current) in works {
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
    let changes = drift(conn, config, profile_id)?;
    let timestamp = now();
    for change in &changes {
        conn.execute(
            "UPDATE work SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![change.work_id, change.to, timestamp],
        )?;
    }
    Ok(changes)
}

/// Hand the status back to the automation and recompute it at once.
///
/// Unpinning without recomputing would leave the hand-set word in place with
/// nothing claiming it — the state the whole arrangement exists to avoid.
pub fn unpin(conn: &Connection, config: &ProfileConfig, work_id: &str) -> Result<Option<Change>> {
    conn.execute(
        "UPDATE work SET status_pinned_at = NULL WHERE id = ?1",
        params![work_id],
    )?;
    refresh(conn, config, work_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::release::{self as releases, NewRelease};
    use crate::score::{self, NewScore};
    use crate::work::{self, NewWork, WorkPatch};
    use serde_json::json;

    fn workspace() -> (Connection, String, ProfileConfig) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        let config = profile::config_for(&conn, &profile_id).unwrap();
        (conn, profile_id, config)
    }

    fn a_work(conn: &Connection, profile_id: &str, title: &str) -> String {
        work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: title.into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id
    }

    fn a_release(conn: &Connection, work_id: &str) -> String {
        releases::create(
            conn,
            NewRelease {
                work_id: work_id.to_owned(),
                kind: "clip".into(),
                title: None,
                scheduled_at: None,
                meta: None,
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
        let (mut conn, profile_id, config) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        let release_id = a_release(&conn, &work_id);
        releases::schedule(&mut conn, &release_id, "2026-09-01").unwrap();

        refresh(&conn, &config, &work_id).unwrap();
        assert_eq!(status_of(&conn, &work_id), "scheduled");
    }

    /// Finality wins over recency: a work with one release out and another
    /// booked is released, not scheduled.
    #[test]
    fn going_out_outranks_being_booked() {
        let (mut conn, profile_id, config) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        let gone = a_release(&conn, &work_id);
        releases::mark_released(&conn, &gone, None, None).unwrap();
        let booked = a_release(&conn, &work_id);
        releases::schedule(&mut conn, &booked, "2026-09-01").unwrap();

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
        config.statuses.retain(|status| status.key != "scored");

        assert_eq!(derive_for(&conn, &config, &work_id).unwrap(), None);
        assert_eq!(refresh(&conn, &config, &work_id).unwrap(), None);
        assert_eq!(status_of(&conn, &work_id), "draft");
    }
}
