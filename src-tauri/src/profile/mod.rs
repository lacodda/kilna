pub mod config;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::db::migrations;
use crate::error::{Error, Result};
use crate::time::now;
use config::{Derive, ProfileConfig};

/// A profile as it ships with the application, before it reaches the database.
#[derive(Debug, Clone, Deserialize)]
pub struct BuiltinProfile {
    pub key: String,
    pub name: String,
    pub description: String,
    pub config: ProfileConfig,
}

/// A profile as the frontend sees it.
#[derive(Debug, Clone, Serialize)]
pub struct Profile {
    pub id: String,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub config: ProfileConfig,
    pub is_active: bool,
    pub is_builtin: bool,
}

/// Profiles compiled into the binary. Music ships first; the others arrive with
/// the profile editor.
pub fn builtin() -> Result<Vec<BuiltinProfile>> {
    // Four crafts against one schema. If a craft needed a schema change to fit,
    // the premise of ADR 0001 would be wrong — so far none has.
    const SOURCES: &[&str] = &[
        include_str!("../../profiles/music.json"),
        include_str!("../../profiles/novel.json"),
        include_str!("../../profiles/podcast.json"),
        include_str!("../../profiles/blog.json"),
    ];
    SOURCES
        .iter()
        .map(|source| serde_json::from_str(source).map_err(Error::from))
        .collect()
}

/// Install any built-in profile the workspace does not have yet, and make sure
/// exactly one profile is active.
///
/// Existing built-in rows are left alone: a user who edited the Music profile
/// keeps their edits across upgrades.
pub fn seed(conn: &Connection) -> Result<()> {
    let timestamp = now();

    for profile in builtin()? {
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM profile WHERE key = ?1",
                params![profile.key],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if exists {
            carry_forward(conn, &profile)?;
            continue;
        }

        conn.execute(
            "INSERT INTO profile (id, key, name, description, config, is_active, is_builtin, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 1, ?6, ?6)",
            params![
                uuid::Uuid::new_v4().to_string(),
                profile.key,
                profile.name,
                profile.description,
                serde_json::to_string(&profile.config)?,
                timestamp,
            ],
        )?;
    }

    ensure_one_active(conn)?;
    Ok(())
}

/// Carry newly shipped parts of a built-in profile into the copy a workspace
/// already holds.
///
/// A profile is copied into the database on first run and then belongs to the
/// user, so shipping a new field changes nothing for anybody who already ran
/// the app — the older copy simply lacks it. What is added here is what has no
/// user-authored value to overwrite:
///
/// * **Prompt templates** whose key is missing. A user who reworded an action
///   keeps their wording; one they deliberately deleted comes back at the next
///   upgrade — accepted, because an action nobody ever sees is worse.
/// * **The `derive` role of a status**, where the stored copy still says
///   `Manual` and the shipped one names a meaning. Without this the status
///   automation is silently inert in every workspace that predates it, which is
///   exactly how it was found: on a real database, deriving nothing at all.
///
/// A role the user deliberately set to `Manual` is indistinguishable from one
/// that was never written, and is restored along with the rest. That is the
/// price of not asking; the setting is one click away in the profile editor.
fn carry_forward(conn: &Connection, shipped: &BuiltinProfile) -> Result<()> {
    let Some((id, raw)): Option<(String, String)> = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = ?1",
            params![shipped.key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
    else {
        return Ok(());
    };

    let mut config: ProfileConfig = serde_json::from_str(&raw)?;
    let mut changed = false;

    let missing: Vec<_> = shipped
        .config
        .prompts
        .iter()
        .filter(|shipped| {
            !config
                .prompts
                .iter()
                .any(|existing| existing.key == shipped.key)
        })
        .cloned()
        .collect();

    if !missing.is_empty() {
        config.prompts.extend(missing);
        changed = true;
    }

    for status in &mut config.statuses {
        if status.derive != Derive::Manual {
            continue;
        }
        let Some(shipped) = shipped
            .config
            .statuses
            .iter()
            .find(|shipped| shipped.key == status.key)
        else {
            continue;
        };
        if shipped.derive != Derive::Manual {
            status.derive = shipped.derive;
            changed = true;
        }
    }

    if !changed {
        return Ok(());
    }

    conn.execute(
        "UPDATE profile SET config = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, serde_json::to_string(&config)?, now()],
    )?;

    Ok(())
}

/// Make sure a profile is active — a workspace is never without one.
///
/// The default is the first profile in `builtin()`, not the first by key or by
/// timestamp: the seeds all land in the same second, so ordering by either
/// would hand a new user whichever craft happened to sort first.
fn ensure_one_active(conn: &Connection) -> Result<()> {
    let active: i64 = conn.query_row(
        "SELECT count(*) FROM profile WHERE is_active = 1",
        [],
        |row| row.get(0),
    )?;

    if active > 0 {
        return Ok(());
    }

    let preferred = builtin()?.first().map(|profile| profile.key.clone());

    if let Some(key) = preferred {
        let changed = conn.execute(
            "UPDATE profile SET is_active = 1 WHERE key = ?1",
            params![key],
        )?;
        if changed > 0 {
            return Ok(());
        }
    }

    // A workspace with only user-made profiles falls back to the oldest.
    conn.execute(
        "UPDATE profile SET is_active = 1
         WHERE id = (SELECT id FROM profile ORDER BY created_at, key LIMIT 1)",
        [],
    )?;
    Ok(())
}

const SELECT_PROFILE: &str =
    "SELECT id, key, name, description, config, is_active, is_builtin FROM profile";

/// The profile the workspace is currently working in.
pub fn active(conn: &Connection) -> Result<Option<Profile>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_PROFILE} WHERE is_active = 1"),
            [],
            read_row,
        )
        .optional()?;

    raw.map(RawProfile::into_profile).transpose()
}

/// One profile's configuration, without the rest of the row.
///
/// Used by everything that has a profile id in hand and needs the vocabulary
/// behind it — scoring, status derivation — so the query lives here rather
/// than being written out again in each of them.
pub fn config_for(conn: &Connection, profile_id: &str) -> Result<config::ProfileConfig> {
    let raw: String = conn.query_row(
        "SELECT config FROM profile WHERE id = ?1",
        params![profile_id],
        |row| row.get(0),
    )?;
    Ok(serde_json::from_str(&raw)?)
}

/// Every profile in the workspace.
pub fn list(conn: &Connection) -> Result<Vec<Profile>> {
    let mut statement = conn.prepare(&format!("{SELECT_PROFILE} ORDER BY created_at, key"))?;
    let raw = statement
        .query_map([], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    raw.into_iter().map(RawProfile::into_profile).collect()
}

/// Make `id` the active profile, deactivating the previous one.
pub fn activate(conn: &mut Connection, id: &str) -> Result<()> {
    let tx = conn.transaction()?;
    // The partial unique index forbids two active rows, so clear first.
    tx.execute("UPDATE profile SET is_active = 0 WHERE is_active = 1", [])?;
    let changed = tx.execute(
        "UPDATE profile SET is_active = 1, updated_at = ?2 WHERE id = ?1",
        params![id, now()],
    )?;

    if changed == 0 {
        return Err(Error::not_found("profile", id));
    }

    tx.commit()?;
    Ok(())
}

/// Replace a profile's configuration.
///
/// Nothing else is touched: axis keys are strings, so past score snapshots stay
/// readable, and works keep whatever status and kind they already had even if
/// the vocabulary that named them was edited away.
pub fn update_config(conn: &Connection, id: &str, config: &ProfileConfig) -> Result<Profile> {
    let changed = conn.execute(
        "UPDATE profile SET config = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, serde_json::to_string(config)?, now()],
    )?;

    if changed == 0 {
        return Err(Error::not_found("profile", id));
    }

    let raw = conn
        .query_row(
            &format!("{SELECT_PROFILE} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?
        .ok_or_else(|| Error::not_found("profile", id))?;

    raw.into_profile()
}

/// A profile row before its config is parsed. Reading and parsing are separate
/// steps because rusqlite's row closure cannot fail with our error type.
struct RawProfile {
    id: String,
    key: String,
    name: String,
    description: Option<String>,
    config: String,
    is_active: bool,
    is_builtin: bool,
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawProfile> {
    Ok(RawProfile {
        id: row.get(0)?,
        key: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        config: row.get(4)?,
        is_active: row.get::<_, i64>(5)? == 1,
        is_builtin: row.get::<_, i64>(6)? == 1,
    })
}

impl RawProfile {
    fn into_profile(self) -> Result<Profile> {
        Ok(Profile {
            config: serde_json::from_str(&self.config)?,
            id: self.id,
            key: self.key,
            name: self.name,
            description: self.description,
            is_active: self.is_active,
            is_builtin: self.is_builtin,
        })
    }
}

/// A snapshot of the workspace for the status screen.
#[derive(Debug, Serialize)]
pub struct Workspace {
    pub schema_version: i64,
    pub profile: Option<Profile>,
    pub works: i64,
    pub releases: i64,
}

pub fn workspace(conn: &Connection) -> Result<Workspace> {
    Ok(Workspace {
        schema_version: migrations::current_version(conn)?,
        profile: active(conn)?,
        works: conn.query_row("SELECT count(*) FROM work", [], |row| row.get(0))?,
        releases: conn.query_row("SELECT count(*) FROM release", [], |row| row.get(0))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile::config::Derive;

    #[test]
    fn every_builtin_profile_parses() {
        let profiles = builtin().unwrap();
        assert!(!profiles.is_empty());
        for profile in profiles {
            assert!(
                !profile.config.axes.is_empty(),
                "{} has no axes",
                profile.key
            );
            assert!(
                !profile.config.tiers.is_empty(),
                "{} has no tiers",
                profile.key
            );
        }
    }

    #[test]
    fn builtin_keys_are_unique() {
        let profiles = builtin().unwrap();
        let mut keys: Vec<_> = profiles.iter().map(|p| p.key.as_str()).collect();
        keys.sort_unstable();
        let count = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), count, "two built-in profiles share a key");
    }

    #[test]
    fn seed_installs_the_builtins_and_activates_one() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let profiles = list(&conn).unwrap();
        assert_eq!(profiles.len(), builtin().unwrap().len());
        let active = active(&conn).unwrap().expect("a profile must be active");
        assert!(active.is_builtin);
    }

    #[test]
    fn a_new_workspace_starts_on_the_first_declared_profile() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        // Not the first by key — the seeds share a timestamp, so ordering by
        // key or date would pick a craft at random.
        assert_eq!(
            active(&conn).unwrap().unwrap().key,
            builtin().unwrap()[0].key
        );
    }

    #[test]
    fn seed_does_not_duplicate_on_a_second_run() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        seed(&conn).unwrap();

        assert_eq!(list(&conn).unwrap().len(), builtin().unwrap().len());
    }

    /// A workspace created before a field existed keeps the profile it copied
    /// on first run, and shipping the field changes nothing there on its own.
    /// This is not hypothetical: the status automation was found inert on a
    /// real database for exactly this reason, deriving `Manual` for every
    /// status while the shipped profile named meanings for four of them.
    #[test]
    fn an_older_workspace_gains_the_meanings_its_statuses_were_missing() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        // Wind the stored copy back to what a pre-`derive` workspace holds: the
        // same words, with no meaning attached to any of them.
        let (id, raw): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
        for status in &mut config.statuses {
            status.derive = Derive::Manual;
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let config = config_for(&conn, &id).unwrap();
        let derive_of = |key: &str| {
            config
                .statuses
                .iter()
                .find(|status| status.key == key)
                .unwrap()
                .derive
        };
        assert_eq!(derive_of("draft"), Derive::Draft);
        assert_eq!(derive_of("scored"), Derive::Scored);
        assert_eq!(derive_of("scheduled"), Derive::Scheduled);
        assert_eq!(derive_of("released"), Derive::Released);
        // Nothing is invented for a status the shipped profile leaves manual.
        assert_eq!(derive_of("shelved"), Derive::Manual);
    }

    /// The labels are the user's, and an upgrade must not take them back.
    #[test]
    fn carrying_meanings_forward_leaves_renamed_statuses_renamed() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let (id, raw): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
        for status in &mut config.statuses {
            status.derive = Derive::Manual;
            if status.key == "released" {
                status.label = "Out in the world".into();
            }
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let config = config_for(&conn, &id).unwrap();
        let released = config
            .statuses
            .iter()
            .find(|status| status.key == "released")
            .unwrap();
        assert_eq!(released.label, "Out in the world");
        assert_eq!(released.derive, Derive::Released);
    }

    /// Every profile must be self-consistent, because a broken one is only
    /// discovered when a user switches to it.
    #[test]
    fn every_builtin_profile_is_internally_consistent() {
        for profile in builtin().unwrap() {
            let config = &profile.config;
            let key = &profile.key;

            assert!(!config.statuses.is_empty(), "{key} has no statuses");

            // A profile whose statuses carry no meaning derives nothing, and
            // the automation would sit there doing nothing with no error to
            // show for it. Every derivable meaning needs a word, and no two
            // words may claim the same one — the automation would pick between
            // them by list order, which is not a decision anybody made.
            for meaning in [
                Derive::Draft,
                Derive::Scored,
                Derive::Scheduled,
                Derive::Released,
            ] {
                let named: Vec<&str> = config
                    .statuses
                    .iter()
                    .filter(|status| status.derive == meaning)
                    .map(|status| status.key.as_str())
                    .collect();
                assert_eq!(
                    named.len(),
                    1,
                    "{key} names {} status(es) for {meaning:?}: {named:?}",
                    named.len()
                );
            }
            assert!(!config.work_kinds.is_empty(), "{key} has no work kinds");
            assert!(
                !config.version_roles.is_empty(),
                "{key} has no version roles"
            );

            // A tier reachable by nothing is a tier that never appears.
            assert!(
                config.tiers.iter().any(|tier| tier.min == 0.0),
                "{key} has no tier a zero score falls into"
            );
            assert!(
                config.tiers.iter().all(|tier| tier.min <= 100.0),
                "{key} has a tier no score can reach"
            );

            // Full marks on every axis must land in the top tier, or the
            // scoring scale and the tiers disagree with each other.
            let full: serde_json::Map<String, serde_json::Value> = config
                .axes
                .iter()
                .map(|axis| (axis.key.clone(), serde_json::json!(axis.scale)))
                .collect();
            let total = config.total(&full);
            assert!(
                (total - 100.0).abs() < 1e-9,
                "{key}: full marks scored {total}, not 100"
            );

            // Prompts may only ask for roles the profile actually has.
            for prompt in &config.prompts {
                for fragment in prompt.template.split("{role:").skip(1) {
                    let role = fragment.split('}').next().unwrap_or_default();
                    assert!(
                        config.version_roles.iter().any(|r| r.key == role),
                        "{key}: prompt `{}` asks for unknown role `{role}`",
                        prompt.key
                    );
                }
            }
        }
    }

    #[test]
    fn seed_carries_newly_shipped_prompts_into_an_existing_profile() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        // A workspace created before the prompts shipped.
        let mut config = active(&conn).unwrap().unwrap().config;
        config.prompts.clear();
        conn.execute(
            "UPDATE profile SET config = ?1 WHERE key = 'music'",
            params![serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let shipped = builtin().unwrap()[0].config.prompts.len();
        assert_eq!(
            active(&conn).unwrap().unwrap().config.prompts.len(),
            shipped
        );
    }

    #[test]
    fn seed_does_not_overwrite_a_reworded_prompt() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        let mut config = active(&conn).unwrap().unwrap().config;
        let key = config.prompts[0].key.clone();
        config.prompts[0].label = "My own wording".into();
        conn.execute(
            "UPDATE profile SET config = ?1 WHERE key = 'music'",
            params![serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let reloaded = active(&conn).unwrap().unwrap().config;
        let kept = reloaded.prompts.iter().find(|p| p.key == key).unwrap();
        assert_eq!(kept.label, "My own wording");
        assert_eq!(
            reloaded.prompts.len(),
            builtin().unwrap()[0].config.prompts.len(),
            "no duplicate was added alongside it"
        );
    }

    #[test]
    fn seed_keeps_user_edits_to_a_builtin_profile() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        conn.execute(
            "UPDATE profile SET name = 'My music' WHERE key = 'music'",
            [],
        )
        .unwrap();

        seed(&conn).unwrap();

        let profile = list(&conn)
            .unwrap()
            .into_iter()
            .find(|p| p.key == "music")
            .unwrap();
        assert_eq!(profile.name, "My music");
    }

    #[test]
    fn activate_moves_the_active_flag() {
        let mut conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        conn.execute(
            "INSERT INTO profile (id, key, name, config, is_active, is_builtin, created_at, updated_at)
             VALUES ('p-mine', 'mine', 'Mine', ?1, 0, 0, '2026-02-01T00:00:00Z', '2026-02-01T00:00:00Z')",
            params![serde_json::to_string(&builtin().unwrap()[0].config).unwrap()],
        )
        .unwrap();

        activate(&mut conn, "p-mine").unwrap();

        assert_eq!(active(&conn).unwrap().unwrap().id, "p-mine");
        let active_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM profile WHERE is_active = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(active_count, 1);
    }

    #[test]
    fn activating_an_unknown_profile_fails_without_deactivating_the_current_one() {
        let mut conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        assert!(activate(&mut conn, "nope").is_err());
        assert!(active(&conn).unwrap().is_some());
    }

    #[test]
    fn workspace_reports_the_schema_and_counts() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let workspace = workspace(&conn).unwrap();

        assert_eq!(workspace.schema_version, migrations::latest_version());
        assert_eq!(workspace.works, 0);
        assert!(workspace.profile.is_some());
    }
}
