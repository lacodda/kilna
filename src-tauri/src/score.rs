use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::profile;
use crate::time::now;

/// A score is a snapshot pinned to a version, never an overwrite of the work.
/// That is what makes the effect of a revision visible — the reason scoring
/// exists at all. See ADR 0002.
#[derive(Debug, Clone, Serialize)]
pub struct Score {
    pub id: String,
    pub work_id: String,
    pub version_id: Option<String>,
    /// Axis keys are profile strings, not foreign keys, so reconfiguring a
    /// profile never invalidates a past snapshot.
    pub axes: Map<String, Value>,
    pub total: f64,
    pub tier: Option<String>,
    pub note: Option<String>,
    pub scored_at: String,
    /// Revision of the scored version, when it still exists.
    pub revision: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewScore {
    pub axes: Map<String, Value>,
    /// Defaults to the work's current version.
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// A work with its most recent score, for the catalogue.
#[derive(Debug, Clone, Serialize)]
pub struct ScoredWork {
    pub work_id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub total: Option<f64>,
    pub tier: Option<String>,
    pub scored_at: Option<String>,
    /// True when the work has changed since it was last scored — the score
    /// describes an older draft.
    pub stale: bool,
    /// How many releases of this work have gone out.
    pub released: i64,
    /// How many hold a slot in the calendar.
    pub scheduled: i64,
}

/// The score that speaks for a work, as a subquery returning one `work_score.id`.
///
/// **Finalised is the work.** Once a version is named as the current one, that
/// version *is* the work, and its score is the work's score — not the newest
/// snapshot on the record. Going back to judge an old draft otherwise announces
/// that the work got worse. With no current version, or none that was ever
/// judged, the strongest score stands: the best it has been shown to be, rather
/// than whatever happened last.
///
/// Written once and shared because three places used to answer this question
/// differently — the catalogue by this rule, the calendar and the displacement
/// contest by "latest". One work read 68.1 on one screen and 77.8 on the next,
/// and the rule that decides which release keeps a date used a third number
/// nothing displayed. `{work}` is the column holding the work id.
pub const SPEAKING_SCORE: &str = "SELECT id FROM work_score WHERE work_id = {work}      AND version_id IS NOT NULL AND version_id = (SELECT current_version_id FROM work WHERE id = {work})      ORDER BY scored_at DESC LIMIT 1";

/// The fallback half of [`SPEAKING_SCORE`]: strongest, then most recent.
pub const STRONGEST_SCORE: &str = "SELECT id FROM work_score WHERE work_id = {work}      ORDER BY total DESC, scored_at DESC LIMIT 1";

/// Both halves as one `coalesce`, ready to join against.
pub fn speaking_score_for(work_column: &str) -> String {
    format!(
        "coalesce(({}), ({}))",
        SPEAKING_SCORE.replace("{work}", work_column),
        STRONGEST_SCORE.replace("{work}", work_column),
    )
}

/// Record a score for a work.
///
/// The total and tier are computed from the active profile rather than accepted
/// from the caller: two clients must not be able to disagree about what a set
/// of axis values is worth.
pub fn create(conn: &Connection, work_id: &str, new: NewScore) -> Result<Score> {
    let (profile_id, current_version): (String, Option<String>) = conn
        .query_row(
            "SELECT profile_id, current_version_id FROM work WHERE id = ?1",
            params![work_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| Error::not_found("work", work_id))?;

    let config = profile::config_for(conn, &profile_id)?;
    let total = config.total(&new.axes);
    let tier = config.tier_for(total).map(|tier| tier.key.clone());

    let version_id = new.version_id.or(current_version);
    if let Some(version_id) = &version_id {
        let belongs: bool = conn
            .query_row(
                "SELECT 1 FROM work_version WHERE id = ?1 AND work_id = ?2",
                params![version_id, work_id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if !belongs {
            return Err(Error::Other(format!(
                "version `{version_id}` does not belong to work `{work_id}`"
            )));
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let scored_at = now();

    conn.execute(
        "INSERT INTO work_score (id, work_id, version_id, axes, total, tier, note, scored_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            work_id,
            version_id,
            Value::Object(new.axes).to_string(),
            total,
            tier,
            new.note,
            scored_at,
        ],
    )?;

    get(conn, &id)?.ok_or_else(|| Error::Other("the score vanished after insert".into()))
}

const SELECT_SCORE: &str = "SELECT s.id, s.work_id, s.version_id, s.axes, s.total, s.tier, \
     s.note, s.scored_at, v.revision \
     FROM work_score s LEFT JOIN work_version v ON v.id = s.version_id";

pub fn get(conn: &Connection, id: &str) -> Result<Option<Score>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_SCORE} WHERE s.id = ?1"),
            params![id],
            read_row,
        )
        .optional()?;

    raw.map(RawScore::into_score).transpose()
}

/// Every score of a work, newest first.
pub fn history(conn: &Connection, work_id: &str) -> Result<Vec<Score>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT_SCORE} WHERE s.work_id = ?1 ORDER BY s.scored_at DESC"
    ))?;
    let raw = statement
        .query_map(params![work_id], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    raw.into_iter().map(RawScore::into_score).collect()
}

/// The most recent score of a work.
pub fn latest(conn: &Connection, work_id: &str) -> Result<Option<Score>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_SCORE} WHERE s.work_id = ?1 ORDER BY s.scored_at DESC LIMIT 1"),
            params![work_id],
            read_row,
        )
        .optional()?;

    raw.map(RawScore::into_score).transpose()
}

/// Delete a score snapshot. History is otherwise append-only.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    if conn.execute("DELETE FROM work_score WHERE id = ?1", params![id])? == 0 {
        return Err(Error::not_found("score", id));
    }
    Ok(())
}

/// The catalogue: every work with the score that speaks for it, best first.
///
/// Unscored works sort last — they are not bad, they are unjudged.
///
/// **Which score speaks for a work.** Once a version is named as the current
/// one, that version *is* the work, and the catalogue reads the score of that
/// version — not the newest score on the record. The difference shows the
/// moment a person judges an old draft to see how far it has come: the newest
/// snapshot then describes the version they went back to, and reading it would
/// have the catalogue announce that the work got worse. A work with no current
/// version, or one whose current version was never judged, falls back to its
/// strongest score: the best it has been shown to be.
pub fn catalogue(conn: &Connection, profile_id: &str) -> Result<Vec<ScoredWork>> {
    let mut statement = conn.prepare(&format!(
        "SELECT w.id, w.title, w.kind, w.status, s.total, s.tier, s.scored_at,
                s.scored_at IS NOT NULL AND w.updated_at > s.scored_at AS stale,
                (SELECT count(*) FROM release r
                  WHERE r.work_id = w.id AND r.status = 'released') AS released,
                (SELECT count(*) FROM release r
                  WHERE r.work_id = w.id AND r.status = 'planned'
                    AND r.scheduled_at IS NOT NULL) AS scheduled
         FROM work w
         LEFT JOIN work_score s ON s.id = {speaking}
         WHERE w.profile_id = ?1
         ORDER BY s.total IS NULL, s.total DESC, w.title",
        speaking = speaking_score_for("w.id"),
    ))?;

    let rows = statement.query_map(params![profile_id], |row| {
        Ok(ScoredWork {
            work_id: row.get(0)?,
            title: row.get(1)?,
            kind: row.get(2)?,
            status: row.get(3)?,
            total: row.get(4)?,
            tier: row.get(5)?,
            scored_at: row.get(6)?,
            stale: row.get::<_, i64>(7)? == 1,
            released: row.get(8)?,
            scheduled: row.get(9)?,
        })
    })?;

    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

struct RawScore {
    id: String,
    work_id: String,
    version_id: Option<String>,
    axes: String,
    total: f64,
    tier: Option<String>,
    note: Option<String>,
    scored_at: String,
    revision: Option<i64>,
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawScore> {
    Ok(RawScore {
        id: row.get(0)?,
        work_id: row.get(1)?,
        version_id: row.get(2)?,
        axes: row.get(3)?,
        total: row.get(4)?,
        tier: row.get(5)?,
        note: row.get(6)?,
        scored_at: row.get(7)?,
        revision: row.get(8)?,
    })
}

impl RawScore {
    fn into_score(self) -> Result<Score> {
        Ok(Score {
            axes: serde_json::from_str(&self.axes)?,
            id: self.id,
            work_id: self.work_id,
            version_id: self.version_id,
            total: self.total,
            tier: self.tier,
            note: self.note,
            scored_at: self.scored_at,
            revision: self.revision,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::work::version::{self, NewVersion};
    use crate::work::{self, NewWork};
    use serde_json::json;

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    fn a_work(conn: &Connection, profile_id: &str, title: &str) -> String {
        work::create(
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
        .unwrap()
        .id
    }

    fn axes(values: serde_json::Value) -> Map<String, Value> {
        values.as_object().cloned().unwrap()
    }

    /// Score one named version, with every axis at the same mark so the total
    /// is that mark on a 0–100 scale whatever the profile weights.
    fn score_version(conn: &Connection, work_id: &str, version_id: &str, mark: f64) {
        create(
            conn,
            work_id,
            NewScore {
                axes: axes(json!({ "hook": mark, "text": mark, "voice": mark, "reach": mark })),
                version_id: Some(version_id.to_owned()),
                note: None,
            },
        )
        .unwrap();
    }

    fn catalogue_row(conn: &Connection, profile_id: &str, work_id: &str) -> ScoredWork {
        catalogue(conn, profile_id)
            .unwrap()
            .into_iter()
            .find(|row| row.work_id == work_id)
            .expect("the work is missing from the catalogue")
    }

    #[test]
    fn the_total_and_tier_are_computed_from_the_profile() {
        let (conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");

        // Every axis at full marks must reach the top tier.
        let score = create(
            &conn,
            &work_id,
            NewScore {
                axes: axes(json!({
                    "hook": 10, "lyrics": 10, "emotion": 10,
                    "production": 10, "originality": 10, "visual": 10
                })),
                version_id: None,
                note: None,
            },
        )
        .unwrap();

        assert!((score.total - 100.0).abs() < 1e-9, "got {}", score.total);
        assert_eq!(score.tier.as_deref(), Some("clip"));
    }

    #[test]
    fn a_weak_score_lands_in_the_bottom_tier() {
        let (conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id, "Weak");

        let score = create(
            &conn,
            &work_id,
            NewScore {
                axes: axes(json!({ "hook": 2, "lyrics": 3 })),
                version_id: None,
                note: None,
            },
        )
        .unwrap();

        assert_eq!(score.tier.as_deref(), Some("hold"));
    }

    /// The rule that decides which snapshot a work is judged by: the one on the
    /// version that currently *is* the work, not the one taken most recently.
    /// The counts behind the catalogue's "nothing planned yet" filter. A
    /// release with no date is an intention, not a slot — the same rule the
    /// status automation follows, and it has to mean the same thing in both
    /// places or the two screens disagree about the same work.
    #[test]
    fn the_catalogue_counts_slots_and_releases() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");

        let row = catalogue_row(&conn, &profile_id, &work_id);
        assert_eq!((row.released, row.scheduled), (0, 0));

        let undated = crate::release::create(
            &conn,
            crate::release::NewRelease {
                work_id: work_id.clone(),
                kind: "clip".into(),
                title: None,
                scheduled_at: None,
                meta: None,
            },
        )
        .unwrap();
        let row = catalogue_row(&conn, &profile_id, &work_id);
        assert_eq!(
            (row.released, row.scheduled),
            (0, 0),
            "a release with no date holds no slot"
        );

        crate::release::schedule(&mut conn, &undated.id, "2026-09-01").unwrap();
        let row = catalogue_row(&conn, &profile_id, &work_id);
        assert_eq!((row.released, row.scheduled), (0, 1));

        crate::release::mark_released(&conn, &undated.id, None).unwrap();
        let row = catalogue_row(&conn, &profile_id, &work_id);
        assert_eq!(
            (row.released, row.scheduled),
            (1, 0),
            "one that went out no longer holds its slot"
        );
    }

    #[test]
    fn the_catalogue_reads_the_score_of_the_current_version() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");

        let first = version::create(
            &mut conn,
            &work_id,
            NewVersion {
                role: "lyrics".into(),
                body: "a first pass".into(),
                label: None,
                meta: None,
                make_current: true,
            },
        )
        .unwrap();
        score_version(&conn, &work_id, &first.id, 9.0);

        let second = version::create(
            &mut conn,
            &work_id,
            NewVersion {
                role: "lyrics".into(),
                body: "the one that stands".into(),
                label: None,
                meta: None,
                make_current: true,
            },
        )
        .unwrap();
        score_version(&conn, &work_id, &second.id, 7.0);

        // Judged last, and the weaker of the two — but it is the version the
        // work currently is, so it is the number the catalogue shows.
        let row = catalogue_row(&conn, &profile_id, &work_id);
        assert_eq!(row.total.unwrap().round(), 70.0);

        // Going back to judge the older draft again must not restate the work
        // as a 9: that snapshot describes a version nobody is looking at.
        score_version(&conn, &work_id, &first.id, 9.0);
        let row = catalogue_row(&conn, &profile_id, &work_id);
        assert_eq!(row.total.unwrap().round(), 70.0);
    }

    /// With nothing named as the work, the best it has ever been shown to be is
    /// the fairest thing to show — a half-finished experiment scored at 3 should
    /// not become the work's number just by being last.
    #[test]
    fn a_work_with_no_current_version_is_read_at_its_strongest() {
        let (conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");

        create(
            &conn,
            &work_id,
            NewScore {
                axes: axes(json!({ "hook": 9, "text": 9, "voice": 9, "reach": 9 })),
                version_id: None,
                note: None,
            },
        )
        .unwrap();
        create(
            &conn,
            &work_id,
            NewScore {
                axes: axes(json!({ "hook": 3, "text": 3, "voice": 3, "reach": 3 })),
                version_id: None,
                note: None,
            },
        )
        .unwrap();

        let row = catalogue_row(&conn, &profile_id, &work_id);
        assert_eq!(row.total.unwrap().round(), 90.0);
    }

    #[test]
    fn a_score_pins_itself_to_the_works_current_version() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        let draft = version::create(
            &mut conn,
            &work_id,
            NewVersion {
                role: "lyrics".into(),
                body: "a draft".into(),
                label: None,
                meta: None,
                make_current: true,
            },
        )
        .unwrap();

        let score = create(
            &conn,
            &work_id,
            NewScore {
                axes: axes(json!({ "hook": 8 })),
                version_id: None,
                note: None,
            },
        )
        .unwrap();

        assert_eq!(score.version_id.as_deref(), Some(draft.id.as_str()));
        assert_eq!(score.revision, Some(1));
    }

    #[test]
    fn scoring_does_not_overwrite_the_previous_snapshot() {
        let (mut conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        version::create(
            &mut conn,
            &work_id,
            NewVersion {
                role: "lyrics".into(),
                body: "first".into(),
                label: None,
                meta: None,
                make_current: true,
            },
        )
        .unwrap();
        create(
            &conn,
            &work_id,
            NewScore {
                axes: axes(json!({ "hook": 5 })),
                version_id: None,
                note: None,
            },
        )
        .unwrap();

        // Revise, then score again.
        version::create(
            &mut conn,
            &work_id,
            NewVersion {
                role: "lyrics".into(),
                body: "second, better".into(),
                label: None,
                meta: None,
                make_current: true,
            },
        )
        .unwrap();
        create(
            &conn,
            &work_id,
            NewScore {
                axes: axes(json!({ "hook": 9 })),
                version_id: None,
                note: None,
            },
        )
        .unwrap();

        let history = history(&conn, &work_id).unwrap();

        assert_eq!(history.len(), 2, "both snapshots survive");
        assert_eq!(history[0].revision, Some(2), "newest first");
        assert_eq!(history[1].revision, Some(1));
        assert!(
            history[0].total > history[1].total,
            "the revision must be visible as a change in score"
        );
    }

    #[test]
    fn a_score_cannot_point_at_another_works_version() {
        let (mut conn, profile_id) = workspace();
        let mine = a_work(&conn, &profile_id, "Mine");
        let theirs = a_work(&conn, &profile_id, "Theirs");
        let stranger = version::create(
            &mut conn,
            &theirs,
            NewVersion {
                role: "lyrics".into(),
                body: "theirs".into(),
                label: None,
                meta: None,
                make_current: true,
            },
        )
        .unwrap();

        let result = create(
            &conn,
            &mine,
            NewScore {
                axes: axes(json!({ "hook": 5 })),
                version_id: Some(stranger.id),
                note: None,
            },
        );

        assert!(result.is_err());
    }

    #[test]
    fn a_snapshot_survives_the_profile_being_reconfigured() {
        let (conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        let score = create(
            &conn,
            &work_id,
            NewScore {
                axes: axes(json!({ "hook": 7, "lyrics": 6 })),
                version_id: None,
                note: None,
            },
        )
        .unwrap();

        // Drop an axis from the profile entirely.
        let mut config = profile::active(&conn).unwrap().unwrap().config;
        config.axes.retain(|axis| axis.key != "lyrics");
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![profile_id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        let reloaded = get(&conn, &score.id).unwrap().unwrap();

        assert_eq!(
            reloaded.axes.get("lyrics").unwrap(),
            &json!(6),
            "the old value is still readable"
        );
        assert!(
            (reloaded.total - score.total).abs() < 1e-9,
            "the total is not recomputed"
        );
    }

    #[test]
    fn the_catalogue_sorts_by_total_and_puts_unscored_works_last() {
        let (conn, profile_id) = workspace();
        let weak = a_work(&conn, &profile_id, "Weak");
        let strong = a_work(&conn, &profile_id, "Strong");
        a_work(&conn, &profile_id, "Unjudged");
        create(
            &conn,
            &weak,
            NewScore {
                axes: axes(json!({ "hook": 3 })),
                version_id: None,
                note: None,
            },
        )
        .unwrap();
        create(
            &conn,
            &strong,
            NewScore {
                axes: axes(json!({ "hook": 9 })),
                version_id: None,
                note: None,
            },
        )
        .unwrap();

        let catalogue = catalogue(&conn, &profile_id).unwrap();

        assert_eq!(catalogue[0].title, "Strong");
        assert_eq!(catalogue[1].title, "Weak");
        assert_eq!(catalogue[2].title, "Unjudged");
        assert!(catalogue[2].total.is_none());
    }

    #[test]
    fn a_work_edited_after_scoring_is_marked_stale() {
        let (conn, profile_id) = workspace();
        let work_id = a_work(&conn, &profile_id, "Subject");
        create(
            &conn,
            &work_id,
            NewScore {
                axes: axes(json!({ "hook": 8 })),
                version_id: None,
                note: None,
            },
        )
        .unwrap();

        let fresh = catalogue(&conn, &profile_id).unwrap();
        assert!(!fresh[0].stale);

        // Touch the work after the score was taken.
        conn.execute(
            "UPDATE work SET updated_at = '2099-01-01T00:00:00Z' WHERE id = ?1",
            params![work_id],
        )
        .unwrap();

        let stale = catalogue(&conn, &profile_id).unwrap();
        assert!(stale[0].stale, "the score describes an older draft");
    }

    #[test]
    fn scoring_an_unknown_work_fails() {
        let (conn, _) = workspace();

        let result = create(
            &conn,
            "nope",
            NewScore {
                axes: axes(json!({ "hook": 5 })),
                version_id: None,
                note: None,
            },
        );

        assert!(result.is_err());
    }
}
