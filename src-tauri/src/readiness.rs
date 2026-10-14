//! Whether a release could go out today, judged from facts.
//!
//! A release is ready when everything its kind asks for is there: a version
//! of every role it requires, a score speaking for the work when the work is
//! judged, a cover when the place wants one, and every field it cannot go out
//! without written (v0.97). The list is the profile's - the craft says what a
//! place needs - and ADR 0063 says why each requirement sits where it does.
//! Nothing here is written anywhere: like the derived status, readiness is
//! computed from what is true right now, so it can never drift from the
//! versions, scores, covers and words it describes.

use std::collections::{BTreeSet, HashMap};

use rusqlite::{Connection, params};
use serde::Serialize;
use serde_json::{Map, Value};
use time::{Date, Duration, macros::format_description};

use crate::error::{Error, Result};
use crate::profile::config::ProfileConfig;
use crate::release::{PLANNED, ScheduledRelease};

/// One release's distance from shippable: everything its kind asks for, in
/// order, each with whether it is there.
///
/// One list rather than a field per sort of requirement (v0.97): every screen
/// that says what is missing - the chip, the queue, the catalogue, the
/// dashboard, the release's own row - reads the same list in the same order,
/// so the first gap one shows is the first gap another shows.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Readiness {
    /// What the release's kind asks for: the roles it names, a score when the
    /// kind is judged, a cover when the place wants one, the fields it cannot
    /// go out without - in that order. What the kind does not ask for is not
    /// listed: a short that needs no outline is not a short missing one.
    pub checks: Vec<Check>,
    /// Every check is done.
    pub ready: bool,
}

/// One thing a release cannot go out without, and whether it is there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct Check {
    pub need: Need,
    pub done: bool,
}

/// What a check asks for. Each names the place it is answered: a role on
/// the versions, the score on the score, the cover on its tab, a field on
/// the release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Need {
    /// A version of this role. The key may name a role the profile has since
    /// lost: such a requirement can never be met, and is said by its key
    /// rather than left to block in silence.
    Role { key: String },
    /// A score speaking for the work. Asked only of a kind that names axes:
    /// an audio release goes out for a song already judged (v0.86).
    Score,
    /// A cover chosen for the work.
    Cover,
    /// Something written under this field of the release.
    Field { key: String },
}

/// What is true of one release and its work, gathered by the caller - the
/// calendar asks about every chip at once and gathers each fact in one query.
pub struct Facts<'a> {
    /// The roles the work has at least one version of.
    pub roles: &'a BTreeSet<String>,
    /// A score speaks for the work.
    pub scored: bool,
    /// A cover is chosen for the work.
    pub cover: bool,
    /// What is written about the release, by field key.
    pub meta: &'a Map<String, Value>,
}

/// Judge one release from the facts already in hand.
///
/// A kind of release the work's kind does not know - one created under an
/// older vocabulary - asks for nothing of its own, and only for the score its
/// work is judged by.
pub fn assess(config: &ProfileConfig, work_kind: &str, kind: &str, facts: &Facts<'_>) -> Readiness {
    // The release kind and what it requires are the work's kind's: a video's
    // clip and a song's clip may require different bodies.
    let vocabulary = config.vocabulary(work_kind);
    let door = vocabulary
        .release_kinds
        .iter()
        .find(|entry| entry.key == kind);

    let mut checks = Vec::new();
    let mut named = BTreeSet::new();
    for role in door
        .map(|door| door.requires.as_slice())
        .unwrap_or_default()
    {
        if named.insert(role.as_str()) {
            checks.push(Check {
                need: Need::Role { key: role.clone() },
                done: facts.roles.contains(role),
            });
        }
    }
    // A kind nobody judges cannot wait for a score: it would never be ready,
    // and every release of it would warn for ever.
    if !vocabulary.axes.is_empty() {
        checks.push(Check {
            need: Need::Score,
            done: facts.scored,
        });
    }
    if let Some(door) = door {
        if door.needs_cover() {
            checks.push(Check {
                need: Need::Cover,
                done: facts.cover,
            });
        }
        for field in door.fields.iter().filter(|field| field.is_required()) {
            checks.push(Check {
                need: Need::Field {
                    key: field.key.clone(),
                },
                done: field.is_filled(facts.meta.get(&field.key)),
            });
        }
    }

    let ready = checks.iter().all(|check| check.done);
    Readiness { checks, ready }
}

impl Readiness {
    /// What is still missing, in the order it is asked for.
    pub fn missing(&self) -> impl Iterator<Item = &Need> {
        self.checks
            .iter()
            .filter(|check| !check.done)
            .map(|check| &check.need)
    }
}

/// Which roles each work of the profile has a version for, in one query —
/// the calendar asks about every chip at once, and a query per chip is how a
/// month view gets slow.
pub fn roles_present(
    conn: &Connection,
    profile_id: &str,
) -> Result<HashMap<String, BTreeSet<String>>> {
    let mut statement = conn.prepare(
        "SELECT DISTINCT v.work_id, v.role
           FROM work_version v
           JOIN work w ON w.id = v.work_id
          WHERE w.profile_id = ?1",
    )?;
    let rows = statement
        .query_map(params![profile_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut map: HashMap<String, BTreeSet<String>> = HashMap::new();
    for (work_id, role) in rows {
        map.entry(work_id).or_default().insert(role);
    }
    Ok(map)
}

/// How far ahead an unready release is worth a warning, in days.
///
/// The frontend's amber band uses the same number: the journal starts talking
/// at the same distance the calendar starts colouring.
pub const WARNING_HORIZON_DAYS: i64 = 7;

/// The planned releases inside the warning window that are not ready.
///
/// `today` is the user's local date, supplied by the frontend: the backend
/// only knows UTC, and at a negative offset UTC is already tomorrow — the
/// v0.23 lesson. Dates compare as strings; ISO dates sort chronologically.
pub fn unready_upcoming<'a>(
    releases: &'a [ScheduledRelease],
    today: &str,
    horizon_days: i64,
) -> Result<Vec<&'a ScheduledRelease>> {
    let iso = format_description!("[year]-[month]-[day]");
    let start = Date::parse(today, iso)
        .map_err(|_| Error::refused("readiness.badToday").param("value", today))?;
    let until = (start + Duration::days(horizon_days))
        .format(iso)
        .map_err(|cause| {
            Error::Internal(format!("could not format the horizon's end date: {cause}"))
        })?;

    Ok(releases
        .iter()
        .filter(|entry| entry.release.status == PLANNED && !entry.readiness.ready)
        .filter(|entry| {
            entry
                .release
                .scheduled_at
                .as_deref()
                .is_some_and(|date| date >= today && date <= until.as_str())
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ProfileConfig {
        serde_json::from_value(serde_json::json!({
            "work_kinds": [{ "key": "song", "label": "Song" }],
            "release_kinds": [
                { "key": "clip", "label": "Clip", "requires": ["lyrics", "style"] },
                { "key": "teaser", "label": "Teaser" },
                {
                    "key": "video",
                    "label": "Video",
                    "requires": ["lyrics"],
                    "requires_cover": true,
                    "fields": [
                        { "key": "title", "label": "Title", "required": true, "suffix": " (audio)" },
                        { "key": "tags", "label": "Tags", "type": "tags", "required": true },
                        { "key": "pinned", "label": "Pinned", "type": "text" }
                    ]
                }
            ],
            "collection_kinds": [],
            "version_roles": [
                { "key": "lyrics", "label": "Lyrics" },
                { "key": "style", "label": "Style" },
                { "key": "notes", "label": "Notes" }
            ],
            "statuses": [{ "key": "draft", "label": "Draft" }],
            "axes": [{ "key": "hook", "label": "Hook", "weight": 1.0, "scale": 10.0 }],
            "tiers": [{ "key": "hold", "label": "Hold", "min": 0.0 }],
            "work_meta_fields": []
        }))
        .unwrap()
    }

    fn roles(present: &[&str]) -> BTreeSet<String> {
        present.iter().map(|role| (*role).to_owned()).collect()
    }

    /// The facts of a work with these roles, judged or not, with no cover
    /// and nothing written about its release.
    fn judge(config: &ProfileConfig, kind: &str, present: &[&str], scored: bool) -> Readiness {
        let roles = roles(present);
        let meta = Map::new();
        assess(
            config,
            "song",
            kind,
            &Facts {
                roles: &roles,
                scored,
                cover: false,
                meta: &meta,
            },
        )
    }

    fn role(key: &str) -> Need {
        Need::Role { key: key.into() }
    }

    fn field(key: &str) -> Need {
        Need::Field { key: key.into() }
    }

    /// The needs of a judgement, each with whether it is met, in order.
    fn listed(readiness: &Readiness) -> Vec<(Need, bool)> {
        readiness
            .checks
            .iter()
            .map(|check| (check.need.clone(), check.done))
            .collect()
    }

    #[test]
    fn a_missing_required_role_is_listed_and_blocks_readiness() {
        let judged = judge(&config(), "clip", &["lyrics"], true);

        assert_eq!(
            listed(&judged),
            [
                (role("lyrics"), true),
                (role("style"), false),
                (Need::Score, true)
            ]
        );
        assert_eq!(judged.missing().collect::<Vec<_>>(), [&role("style")]);
        assert!(!judged.ready);
    }

    /// A role the kind does not ask for is neither there nor missing: it is
    /// not on the list at all, and must not block anything.
    #[test]
    fn an_unrequired_role_is_not_listed() {
        let judged = judge(&config(), "clip", &["lyrics", "style"], true);

        assert!(
            !listed(&judged)
                .iter()
                .any(|(need, _)| *need == role("notes"))
        );
        assert!(judged.ready);
    }

    #[test]
    fn a_kind_stating_no_requirements_is_ready_once_scored() {
        let judged = judge(&config(), "teaser", &[], true);

        assert_eq!(listed(&judged), [(Need::Score, true)]);
        assert!(judged.ready);
        assert!(!judge(&config(), "teaser", &[], false).ready);
    }

    #[test]
    fn an_unscored_release_is_never_ready() {
        let judged = judge(&config(), "clip", &["lyrics", "style"], false);

        assert_eq!(judged.missing().collect::<Vec<_>>(), [&Need::Score]);
        assert!(!judged.ready);
    }

    /// A kind nobody judges - it names no axes, like an audio release that
    /// goes out for a song already judged - is ready without a score: waiting
    /// for one would keep every release of it unready and warning for ever.
    #[test]
    fn a_kind_nobody_judges_does_not_wait_for_a_score() {
        let mut config = config();
        config.work_kinds[0].axes.clear();

        let judged = judge(&config, "clip", &["lyrics", "style"], false);

        assert!(
            !listed(&judged).iter().any(|(need, _)| *need == Need::Score),
            "no score is asked for"
        );
        assert!(judged.ready);
        assert!(
            !judge(&config, "clip", &["lyrics"], false).ready,
            "the roles it requires still count"
        );
    }

    /// A kind the profile does not know - a release created under an older
    /// vocabulary - asks for nothing of its own rather than erroring.
    #[test]
    fn an_unknown_kind_asks_only_for_the_score() {
        let judged = judge(&config(), "gone", &[], true);

        assert_eq!(listed(&judged), [(Need::Score, true)]);
        assert!(judged.ready);
    }

    /// A requirement can outlive the role it names: the profile editor removes
    /// the role, the kind still requires it. It is listed by its key, so a
    /// screen can say what blocks the release instead of blocking in silence.
    #[test]
    fn a_requirement_for_a_role_the_profile_lost_still_blocks() {
        let mut config = config();
        config.work_kinds[0]
            .version_roles
            .retain(|role| role.key != "style");

        let judged = judge(&config, "clip", &["lyrics"], true);

        assert_eq!(judged.missing().collect::<Vec<_>>(), [&role("style")]);
        assert!(!judged.ready);
    }

    /// The place's own needs come after the work's: the cover it asks for,
    /// then every field it cannot go out without, in the door's order. A
    /// field the door does not require is not listed.
    #[test]
    fn a_place_asks_for_its_cover_and_its_required_fields() {
        let config = config();
        let present = roles(&["lyrics"]);
        let meta = Map::new();
        let bare = assess(
            &config,
            "song",
            "video",
            &Facts {
                roles: &present,
                scored: true,
                cover: false,
                meta: &meta,
            },
        );

        assert_eq!(
            listed(&bare),
            [
                (role("lyrics"), true),
                (Need::Score, true),
                (Need::Cover, false),
                (field("title"), false),
                (field("tags"), false)
            ]
        );
        assert!(!bare.ready);

        let written = serde_json::json!({ "title": "Tide", "tags": "rain, sea" });
        let full = assess(
            &config,
            "song",
            "video",
            &Facts {
                roles: &present,
                scored: true,
                cover: true,
                meta: written.as_object().unwrap(),
            },
        );
        assert!(full.ready, "{:?}", listed(&full));
    }

    /// Filled means said something: blank is empty, a list of commas is
    /// empty, and so is the tail a field keeps on its own - " (audio)" is not
    /// a title anybody wrote.
    #[test]
    fn a_field_holding_only_blanks_or_its_tail_is_not_written() {
        let config = config();
        let present = roles(&["lyrics"]);
        let missing = |meta: serde_json::Value| -> Vec<Need> {
            assess(
                &config,
                "song",
                "video",
                &Facts {
                    roles: &present,
                    scored: true,
                    cover: true,
                    meta: meta.as_object().unwrap(),
                },
            )
            .missing()
            .cloned()
            .collect()
        };

        assert_eq!(
            missing(serde_json::json!({ "title": "   ", "tags": " , ,", "pinned": "" })),
            [field("title"), field("tags")]
        );
        assert_eq!(
            missing(serde_json::json!({ "title": " (audio)", "tags": "rain" })),
            [field("title")]
        );
        assert_eq!(
            missing(serde_json::json!({ "title": 7, "tags": "rain" })),
            [field("title")],
            "a value that is not text is nothing a platform can be told"
        );
        assert!(missing(serde_json::json!({ "title": "Tide (audio)", "tags": "rain" })).is_empty());
    }

    /// The window is [today, today + horizon], both ends included, and only
    /// planned, unready releases fall into it. Boundaries are checked on their
    /// own days: an off-by-one here warns a day late, which is the expensive
    /// direction.
    #[test]
    fn the_warning_window_takes_the_unready_and_only_the_upcoming() {
        let conn = crate::db::open_in_memory().unwrap();
        crate::profile::seed(&conn).unwrap();
        let profile_id = crate::profile::active(&conn).unwrap().unwrap().id;

        // None of these works has versions or a score, so every release is
        // unready unless said otherwise.
        let plan = |title: &str, date: &str| {
            let work_id = sample_work(&conn, &profile_id, title);
            crate::release::create(
                &conn,
                crate::release::NewRelease {
                    work_id,
                    kind: "clip".into(),
                    scheduled_at: Some(date.into()),
                    meta: None,
                    scheduled_time: None,
                    time_zone: None,
                },
            )
            .unwrap()
        };

        plan("Inside", "2026-09-04");
        plan("On the edge", "2026-09-08");
        plan("Today itself", "2026-09-01");
        plan("Beyond", "2026-09-21");
        plan("Yesterday", "2026-08-31");
        let released = plan("Released", "2026-09-05");
        crate::release::mark_released(&conn, &released.id, None, None).unwrap();

        // One release in the window with nothing missing: scored, and a
        // version for each role its kind requires.
        let ready = plan("Ready", "2026-09-06");
        crate::score::create(
            &conn,
            &ready.work_id,
            crate::score::NewScore {
                axes: serde_json::json!({ "hook": 8.0 })
                    .as_object()
                    .cloned()
                    .unwrap(),
                version_id: None,
                note: None,
                rater: None,
            },
        )
        .unwrap();
        let conn = conn;
        for role in ["lyrics", "style"] {
            crate::work::version::create(
                &conn,
                &ready.work_id,
                crate::work::version::NewVersion {
                    role: role.into(),
                    body: "body".into(),
                    label: None,
                    meta: None,
                    make_current: true,
                    parent_version_id: None,
                    trial_id: None,
                },
            )
            .unwrap();
        }

        let releases = crate::release::calendar(&conn, &profile_id).unwrap();
        let warned = unready_upcoming(&releases, "2026-09-01", 7).unwrap();
        let mut titles: Vec<&str> = warned
            .iter()
            .map(|entry| entry.work_title.as_str())
            .collect();
        titles.sort_unstable();

        assert_eq!(titles, ["Inside", "On the edge", "Today itself"]);

        // And a malformed today is an error, not an empty answer.
        assert!(unready_upcoming(&releases, "someday", 7).is_err());
    }

    #[test]
    fn roles_present_maps_each_work_to_its_roles() {
        let conn = crate::db::open_in_memory().unwrap();
        crate::profile::seed(&conn).unwrap();
        let profile_id = crate::profile::active(&conn).unwrap().unwrap().id;

        let conn = conn;
        let with_versions = sample_work(&conn, &profile_id, "With versions");
        for role in ["lyrics", "style"] {
            crate::work::version::create(
                &conn,
                &with_versions,
                crate::work::version::NewVersion {
                    role: role.into(),
                    body: "body".into(),
                    label: None,
                    meta: None,
                    make_current: true,
                    parent_version_id: None,
                    trial_id: None,
                },
            )
            .unwrap();
        }
        let without = sample_work(&conn, &profile_id, "Without");

        let map = roles_present(&conn, &profile_id).unwrap();

        assert_eq!(map.get(&with_versions), Some(&roles(&["lyrics", "style"])));
        assert!(!map.contains_key(&without));
    }

    fn sample_work(conn: &Connection, profile_id: &str, title: &str) -> String {
        crate::work::create(
            conn,
            profile_id,
            crate::work::NewWork {
                kind: "song".into(),
                title: title.into(),
                ..crate::work::NewWork::default()
            },
        )
        .unwrap()
        .id
    }
}
