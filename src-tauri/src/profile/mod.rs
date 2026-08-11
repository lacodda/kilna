pub mod config;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::db::migrations;
use crate::error::{Error, Result};
use crate::time::now;
use config::ProfileConfig;

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
    const SOURCES: &[&str] = &[include_str!("../../profiles/music.json")];
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
            add_new_prompts(conn, &profile)?;
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

/// Carry newly shipped prompt templates into a built-in profile the workspace
/// already has.
///
/// Only prompts whose key is missing are added: a user who reworded an action
/// keeps their wording, and one they deliberately deleted stays deleted only
/// until the next upgrade — accepted, because a new action nobody ever sees is
/// the worse failure. Nothing else in the profile is touched.
fn add_new_prompts(conn: &Connection, shipped: &BuiltinProfile) -> Result<()> {
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

    if missing.is_empty() {
        return Ok(());
    }

    config.prompts.extend(missing);
    conn.execute(
        "UPDATE profile SET config = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, serde_json::to_string(&config)?, now()],
    )?;

    Ok(())
}

/// Activate the oldest profile when none is active — a workspace is never
/// without one.
fn ensure_one_active(conn: &Connection) -> Result<()> {
    let active: i64 = conn.query_row(
        "SELECT count(*) FROM profile WHERE is_active = 1",
        [],
        |row| row.get(0),
    )?;

    if active > 0 {
        return Ok(());
    }

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
        return Err(Error::Other(format!("no profile with id `{id}`")));
    }

    tx.commit()?;
    Ok(())
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
    fn seed_does_not_duplicate_on_a_second_run() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        seed(&conn).unwrap();

        assert_eq!(list(&conn).unwrap().len(), builtin().unwrap().len());
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
             VALUES ('p-novel', 'novel', 'Novel', ?1, 0, 0, '2026-02-01T00:00:00Z', '2026-02-01T00:00:00Z')",
            params![serde_json::to_string(&builtin().unwrap()[0].config).unwrap()],
        )
        .unwrap();

        activate(&mut conn, "p-novel").unwrap();

        assert_eq!(active(&conn).unwrap().unwrap().id, "p-novel");
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
