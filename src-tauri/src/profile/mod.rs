pub mod config;
pub mod shipped;
mod upgrade;

#[cfg(test)]
mod tests;

use std::sync::LazyLock;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::db::migrations;
use crate::error::{Error, Result};
use crate::time::now;
use config::ProfileConfig;
use upgrade::{carry_forward, rewrite_older_formats};

/// A profile as it ships with the application, before it reaches the database.
#[derive(Debug, Clone, Deserialize)]
pub struct BuiltinProfile {
    pub key: String,
    pub name: String,
    pub description: String,
    pub config: ProfileConfig,
    /// The starter set of the style dictionary (ADR 0048). Beside the
    /// document, not in it: the bricks it seeds are the workspace's copy, and
    /// a second copy inside every stored document would be two truths.
    #[serde(default)]
    pub style_set: Vec<crate::style_set::SetBrick>,
}

/// A profile as the frontend sees it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
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
///
/// Read once per process and handed out as copies: the documents are half a
/// megabyte of JSON, and every workspace a test opens seeds from them.
pub fn builtin() -> Result<Vec<BuiltinProfile>> {
    static SHIPPED: LazyLock<Option<Vec<BuiltinProfile>>> = LazyLock::new(|| read_builtin().ok());
    match &*SHIPPED {
        Some(shipped) => Ok(shipped.clone()),
        // The sources are fixed, so reading them again fails the same way,
        // and the caller gets the error itself rather than a retelling.
        None => read_builtin(),
    }
}

fn read_builtin() -> Result<Vec<BuiltinProfile>> {
    // Four crafts against one schema. If a craft needed a schema change to fit,
    // the premise of ADR 0001 would be wrong — so far none has. And the empty
    // one last: a craft kilna does not ship starts from it (v0.112).
    const SOURCES: &[&str] = &[
        include_str!("../../profiles/music.json"),
        include_str!("../../profiles/novel.json"),
        include_str!("../../profiles/podcast.json"),
        include_str!("../../profiles/blog.json"),
        include_str!("../../profiles/blank.json"),
    ];
    SOURCES
        .iter()
        .map(|source| serde_json::from_str(source).map_err(Error::from))
        .collect()
}

/// The kind a space is made of when nothing says otherwise: Studio, the
/// first profile kilna ships.
pub const DEFAULT_KIND: &str = "music";

/// The empty profile (v0.112, ADR 0079): one kind of work, a text, four
/// statuses and one way out. What a space of a craft kilna does not ship is
/// made of, the rest named in Settings; and what a profile document of an
/// unknown craft grows from.
pub const BLANK_KIND: &str = "blank";

/// Refuse a document with anything wrong in it, whole, with every problem
/// named: a document edited by hand is fixed by reading the list, not by
/// guessing which line the app minded. The problems travel as reasons, so
/// the window says each in its own language.
pub fn check(config: &ProfileConfig) -> Result<()> {
    let problems = config.problems();
    if problems.is_empty() {
        return Ok(());
    }
    Err(Error::refused("profile.invalid").param(
        "problems",
        serde_json::Value::Array(problems.into_iter().map(Into::into).collect()),
    ))
}

/// Give a new space the profile of a type (v0.112, ADR 0079): the type's
/// document under the type's name, stored as the profile of the kind it grew
/// from - and so carried forward as that kind's is, at once and at every
/// start after, which is how an action shipped since the type was saved
/// reaches the space. Only an empty database is seeded; one that holds a
/// profile keeps it.
pub fn seed_from(
    conn: &Connection,
    kind: &str,
    name: &str,
    description: Option<&str>,
    config: &ProfileConfig,
) -> Result<()> {
    let stored: i64 = conn.query_row("SELECT count(*) FROM profile", [], |row| row.get(0))?;
    if stored == 0 {
        check(config)?;
        let timestamp = now();
        conn.execute(
            "INSERT INTO profile (id, key, name, description, config, is_active, is_builtin, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, 1, ?6, ?6)",
            params![
                uuid::Uuid::new_v4().to_string(),
                kind,
                name,
                description,
                serde_json::to_string(config)?,
                timestamp,
            ],
        )?;
        // The type's words are its maker's: given as they stand, so the
        // standard is not offered against each of them (ADR 0087).
        if let Some(id) = id_for_key(conn, kind)? {
            shipped::settle(conn, &id)?;
        }
    }
    seed(conn, kind)
}

/// Give a space its profile, and make sure exactly one profile is active.
///
/// A space is of one kind (ADR 0077): an empty database gets the shipped
/// profile `kind` names, and nothing else - Blog, Novel and Podcast are kinds
/// of other spaces, not rows waiting in this one. A database that already
/// holds profiles gets none added; each of them that kilna ships is carried
/// forward, and a user who edited the Studio profile keeps their edits across
/// upgrades.
pub fn seed(conn: &Connection, kind: &str) -> Result<()> {
    let timestamp = now();
    let shipped = builtin()?;

    let stored: Vec<String> = conn
        .prepare("SELECT key FROM profile")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;

    if stored.is_empty() {
        let profile = shipped
            .iter()
            .find(|profile| profile.key == kind)
            .ok_or_else(|| {
                Error::refused("space.unknownKind")
                    .param("kind", kind)
                    .param(
                        "known",
                        shipped
                            .iter()
                            .map(|profile| profile.key.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    )
            })?;
        conn.execute(
            "INSERT INTO profile (id, key, name, description, config, is_active, is_builtin, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, 1, ?6, ?6)",
            params![
                uuid::Uuid::new_v4().to_string(),
                profile.key,
                profile.name,
                profile.description,
                serde_json::to_string(&profile.config)?,
                timestamp,
            ],
        )?;
        // Given everything at once: an entry edited before the next start
        // is then the owner's change to a given entry, not one first met.
        if let Some(id) = id_for_key(conn, &profile.key)? {
            shipped::settle(conn, &id)?;
        }
    } else {
        for profile in shipped
            .iter()
            .filter(|profile| stored.contains(&profile.key))
        {
            carry_forward(conn, profile)?;
        }
    }

    // A profile the owner made themselves is carried forward by nobody, so
    // its document is brought to the current format here: read, which
    // migrates, and written back so the next read finds it done.
    rewrite_older_formats(conn)?;

    ensure_one_active(conn, Some(kind))?;
    Ok(())
}

/// Make sure a profile is active — a workspace is never without one.
///
/// The default is `preferred`, the first profile in `builtin()`, not the first
/// by key or by timestamp: the seeds all land in the same second, so ordering
/// by either would hand a new user whichever craft happened to sort first.
fn ensure_one_active(conn: &Connection, preferred: Option<&str>) -> Result<()> {
    let active: i64 = conn.query_row(
        "SELECT count(*) FROM profile WHERE is_active = 1",
        [],
        |row| row.get(0),
    )?;

    if active > 0 {
        return Ok(());
    }

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

/// The profile a stable key names, whatever id this workspace gave it.
///
/// A profile's id is minted by [`seed`] on the machine that first opened the
/// workspace, so the same builtin profile has a different id in every copy. The
/// key does not move, which is why the operations log names profiles by key and
/// resolves them through here when a replay rebuilds elsewhere. See ADR 0014.
pub fn id_for_key(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM profile WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?)
}

/// The stable key of the profile an id names.
pub fn key_for_id(conn: &Connection, profile_id: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT key FROM profile WHERE id = ?1",
            params![profile_id],
            |row| row.get(0),
        )
        .optional()?)
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

/// Replace a profile's configuration.
///
/// Nothing else is touched: axis keys are strings, so past score snapshots stay
/// readable, and works keep whatever status and kind they already had even if
/// the vocabulary that named them was edited away.
pub fn update_config(conn: &Connection, id: &str, config: &ProfileConfig) -> Result<Profile> {
    update_config_at(conn, id, config, &now())
}

/// [`update_config`] with the moment decided by the caller - the operations
/// log records it, so a replay writes the same `updated_at` (ADR 0014).
pub fn update_config_at(
    conn: &Connection,
    id: &str,
    config: &ProfileConfig,
    at: &str,
) -> Result<Profile> {
    check(config)?;

    let changed = conn.execute(
        "UPDATE profile SET config = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, serde_json::to_string(config)?, at],
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
#[derive(Debug, Serialize, ts_rs::TS)]
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
