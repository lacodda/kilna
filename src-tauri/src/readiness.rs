//! Whether a release could go out today, judged from facts.
//!
//! A release is ready when the work has a version for every role its kind
//! requires and a score speaking for it. Nothing here is written anywhere: like
//! the derived status, readiness is computed from what is true right now, so it
//! can never drift from the versions and scores it describes.

use std::collections::{BTreeSet, HashMap};

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::error::Result;
use crate::profile::config::ProfileConfig;

/// One release's distance from shippable.
#[derive(Debug, Clone, Serialize)]
pub struct Readiness {
    /// One mark per version role of the profile, in the profile's order.
    pub roles: Vec<RoleMark>,
    /// Whether a score speaks for the work.
    pub scored: bool,
    /// Everything the kind requires is there, and the work is scored.
    pub ready: bool,
}

/// Three states, not two: a short that needs no outline is different from a
/// short that lacks one.
#[derive(Debug, Clone, Serialize)]
pub struct RoleMark {
    pub role: String,
    /// `Some(true)` the role is there, `Some(false)` it is missing, `None` the
    /// kind does not require it.
    pub present: Option<bool>,
}

/// Judge one release from the facts already in hand.
///
/// `present` holds the roles the work has at least one version for. A kind the
/// profile does not know requires nothing, like a kind with an empty list.
pub fn assess(
    config: &ProfileConfig,
    kind: &str,
    present: &BTreeSet<String>,
    scored: bool,
) -> Readiness {
    let required = config
        .release_kinds
        .iter()
        .find(|entry| entry.key == kind)
        .map(|entry| entry.requires.as_slice())
        .unwrap_or_default();

    let roles = config
        .version_roles
        .iter()
        .map(|role| RoleMark {
            role: role.key.clone(),
            present: required
                .contains(&role.key)
                .then(|| present.contains(&role.key)),
        })
        .collect();

    // Judged against the requirements themselves rather than the marks: a
    // requirement naming a role the profile no longer defines has no mark, but
    // it still cannot be satisfied and the release is still not ready.
    let ready = scored && required.iter().all(|role| present.contains(role));

    Readiness {
        roles,
        scored,
        ready,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ProfileConfig {
        serde_json::from_value(serde_json::json!({
            "work_kinds": [{ "key": "song", "label": "Song" }],
            "release_kinds": [
                { "key": "clip", "label": "Clip", "requires": ["lyrics", "style"] },
                { "key": "teaser", "label": "Teaser" }
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

    fn mark_of(readiness: &Readiness, role: &str) -> Option<bool> {
        readiness
            .roles
            .iter()
            .find(|mark| mark.role == role)
            .unwrap()
            .present
    }

    #[test]
    fn a_missing_required_role_is_marked_and_blocks_readiness() {
        let judged = assess(&config(), "clip", &roles(&["lyrics"]), true);

        assert_eq!(mark_of(&judged, "lyrics"), Some(true));
        assert_eq!(mark_of(&judged, "style"), Some(false));
        assert!(!judged.ready);
    }

    /// The third state: a role the kind does not ask for is neither there nor
    /// missing, and must not block anything.
    #[test]
    fn an_unrequired_role_is_not_applicable_rather_than_missing() {
        let judged = assess(&config(), "clip", &roles(&["lyrics", "style"]), true);

        assert_eq!(mark_of(&judged, "notes"), None);
        assert!(judged.ready);
    }

    #[test]
    fn a_kind_stating_no_requirements_is_ready_once_scored() {
        let judged = assess(&config(), "teaser", &roles(&[]), true);

        assert!(judged.roles.iter().all(|mark| mark.present.is_none()));
        assert!(judged.ready);
        assert!(!assess(&config(), "teaser", &roles(&[]), false).ready);
    }

    #[test]
    fn an_unscored_release_is_never_ready() {
        let judged = assess(&config(), "clip", &roles(&["lyrics", "style"]), false);

        assert!(!judged.scored);
        assert!(!judged.ready);
    }

    /// A kind the profile does not know — a release created under an older
    /// vocabulary — requires nothing rather than erroring.
    #[test]
    fn an_unknown_kind_requires_nothing() {
        let judged = assess(&config(), "gone", &roles(&[]), true);

        assert!(judged.roles.iter().all(|mark| mark.present.is_none()));
        assert!(judged.ready);
    }

    /// A requirement can outlive the role it names: the profile editor removes
    /// the role, the kind still requires it. There is no mark to draw, but the
    /// release must not quietly count as ready.
    #[test]
    fn a_requirement_for_a_role_the_profile_lost_still_blocks() {
        let mut config = config();
        config.version_roles.retain(|role| role.key != "style");

        let judged = assess(&config, "clip", &roles(&["lyrics"]), true);

        assert!(judged.roles.iter().all(|mark| mark.role != "style"));
        assert!(!judged.ready);
    }

    #[test]
    fn roles_present_maps_each_work_to_its_roles() {
        let conn = crate::db::open_in_memory().unwrap();
        crate::profile::seed(&conn).unwrap();
        let profile_id = crate::profile::active(&conn).unwrap().unwrap().id;

        let mut conn = conn;
        let with_versions = sample_work(&conn, &profile_id, "With versions");
        for role in ["lyrics", "style"] {
            crate::work::version::create(
                &mut conn,
                &with_versions,
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
                status: None,
                collection_id: None,
                meta: None,
            },
        )
        .unwrap()
        .id
    }
}
