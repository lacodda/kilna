//! Carrying a workspace from before spaces into them, once (v0.109).
//!
//! Until v0.109 the application's data directory held one database with
//! every profile in it - `kilna.db` with its `media/` and `assistant/` beside
//! it. The first start of this build moves that into spaces:
//!
//! - the database is copied aside first, `kilna.before-spaces.<moment>.db`;
//! - the assistant's settings move into `app.db`, before the schema step that
//!   drops them from the workspace runs;
//! - the profile with works - the open one when it has any - becomes the first
//!   space, and its database is the old one, moved, not copied;
//! - every other profile with something in it becomes a space of its own: a
//!   copy of the database with only that profile left in it, and a copy of
//!   the files;
//! - the shipped profiles nobody used are taken out;
//! - each space is asked its name when the window first opens it.
//!
//! Each step can be run again: a start interrupted half-way finishes the move
//! the next time, and a moved workspace is not moved twice.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use super::{Folder, Manifest, SPACES_DIR, registry};
use crate::db::{self, DATABASE_FILE};
use crate::error::Result;
use crate::state::MEDIA_DIR;

/// What the directory of the run's assistant was called beside the database.
const ASSISTANT_DIR: &str = "assistant";

/// The ids of the spaces the move made, the first one first; empty when there
/// was nothing to move.
pub fn adopt(app_dir: &Path, app: &Connection) -> Result<Vec<String>> {
    let legacy = app_dir.join(DATABASE_FILE);
    if !legacy.is_file() {
        return Ok(Vec::new());
    }
    crate::log::info("space", "moving the workspace into spaces");

    copy_settings(&legacy, app)?;
    keep_a_copy(app_dir, &legacy)?;

    let conn = db::open(&legacy)?;
    let mut keepers = profiles_with_content(&conn)?;
    if keepers.is_empty() {
        // Nothing in it at all: the profile that was open is the first space,
        // so a person who never wrote a line still starts where they were.
        if let Some(open) = crate::profile::active(&conn)? {
            keepers.push(Keeper {
                id: open.id,
                key: open.key,
                name: open.name,
            });
        }
    }
    let Some(first) = keepers.first().cloned() else {
        return Err(crate::error::Error::Internal(
            "the workspace before spaces holds no profile to move".into(),
        ));
    };

    let spaces = app_dir.join(SPACES_DIR);
    let mut made = Vec::new();
    for other in keepers.iter().skip(1) {
        made.push(split_off(app_dir, app, &conn, &spaces, other)?);
    }

    // The first keeps the database itself: only the profiles that went to
    // spaces of their own, and the shipped ones nobody used, leave it.
    conn.execute("DELETE FROM profile WHERE id <> ?1", params![first.id])?;
    prune_loose_rows(&conn)?;
    conn.execute(
        "UPDATE profile SET is_active = 1 WHERE id = ?1",
        params![first.id],
    )?;
    conn.pragma_update(None, "wal_checkpoint", "TRUNCATE")?;
    drop(conn);

    let folder = Folder::new(first_folder(&spaces, &first.name));
    let manifest = manifest_for(folder.path(), &first)?;
    move_into(app_dir, folder.path())?;
    let id = registry::add(app, app_dir, &manifest.id, folder.path())?;
    registry::ask_name(app, &id)?;
    registry::touch(app, &id)?;
    for other in &made {
        registry::ask_name(app, other)?;
    }
    crate::log::info(
        "space",
        &format!(
            "the workspace is now {} space(s), the first at {}",
            made.len() + 1,
            folder.path().display()
        ),
    );
    let mut ids = vec![id];
    ids.extend(made);
    Ok(ids)
}

/// A profile that becomes a space.
#[derive(Debug, Clone)]
struct Keeper {
    id: String,
    key: String,
    name: String,
}

/// The profiles with anything in them - a work, a note, a collection, a chat,
/// a card of the focus board - the open one first, then by how many works.
fn profiles_with_content(conn: &Connection) -> Result<Vec<Keeper>> {
    let rows = conn
        .prepare(
            "SELECT p.id, p.key, p.name,
                    (SELECT count(*) FROM work w WHERE w.profile_id = p.id) AS works,
                    (SELECT count(*) FROM work w WHERE w.profile_id = p.id)
                  + (SELECT count(*) FROM note n WHERE n.profile_id = p.id)
                  + (SELECT count(*) FROM collection c WHERE c.profile_id = p.id)
                  + (SELECT count(*) FROM chat h WHERE h.profile_id = p.id)
                  + (SELECT count(*) FROM focus_note f WHERE f.profile_id = p.id) AS held
               FROM profile p
              ORDER BY p.is_active DESC, works DESC, p.created_at, p.key",
        )?
        .query_map([], |row| {
            Ok((
                Keeper {
                    id: row.get(0)?,
                    key: row.get(1)?,
                    name: row.get(2)?,
                },
                row.get::<_, i64>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows
        .into_iter()
        .filter(|(_, held)| *held > 0)
        .map(|(keeper, _)| keeper)
        .collect())
}

/// The assistant's settings, read from the workspace before the step that
/// drops them runs, into `app.db` - unless it already holds some.
fn copy_settings(legacy: &Path, app: &Connection) -> Result<()> {
    let has_own: bool = app
        .query_row("SELECT 1 FROM assistant_settings", [], |_| Ok(true))
        .optional()?
        .unwrap_or(false);
    if has_own {
        return Ok(());
    }
    let raw = Connection::open(legacy)?;
    let has_table: bool = raw
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'assistant_settings'",
            [],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !has_table {
        return Ok(());
    }
    // Every column 0049 could have left, read by name: a workspace stopped
    // at 0048 has the budget alone.
    let columns: Vec<String> = raw
        .prepare("SELECT name FROM pragma_table_info('assistant_settings')")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let has = |name: &str| columns.iter().any(|column| column == name);
    let row = raw
        .query_row(
            &format!(
                "SELECT weekly_budget_usd, {}, {}, {} FROM assistant_settings WHERE id = 1",
                if has("parallel_runs") {
                    "parallel_runs"
                } else {
                    "3"
                },
                if has("model") { "model" } else { "NULL" },
                if has("run_sees") {
                    "run_sees"
                } else {
                    "'nothing'"
                },
            ),
            [],
            |row| {
                Ok((
                    row.get::<_, Option<f64>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    if let Some((budget, runs, model, sees)) = row {
        app.execute(
            "INSERT INTO assistant_settings (id, weekly_budget_usd, parallel_runs, model, run_sees, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5)",
            params![budget, runs, model, sees, crate::time::now()],
        )?;
    }
    Ok(())
}

/// A copy of the workspace as it was, beside it, once: the move rewrites the
/// database, and a person must be able to go back to the day before.
fn keep_a_copy(app_dir: &Path, legacy: &Path) -> Result<()> {
    let taken = std::fs::read_dir(app_dir)?.flatten().any(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .starts_with("kilna.before-spaces.")
    });
    if taken {
        return Ok(());
    }
    let stamp = crate::time::now().replace([':', '.'], "-");
    let copy = app_dir.join(format!("kilna.before-spaces.{stamp}.db"));
    let source = Connection::open(legacy)?;
    source.execute("VACUUM INTO ?1", params![copy.to_string_lossy()])?;
    Ok(())
}

/// Make a space of its own of one profile: a copy of the database with the
/// others taken out, and a copy of the files beside it.
fn split_off(
    app_dir: &Path,
    app: &Connection,
    conn: &Connection,
    spaces: &Path,
    keeper: &Keeper,
) -> Result<String> {
    let folder = Folder::new(super::free_folder(spaces, &keeper.name));
    std::fs::create_dir_all(folder.path())?;
    conn.execute(
        "VACUUM INTO ?1",
        params![folder.database().to_string_lossy()],
    )?;
    let copy = db::open(&folder.database())?;
    copy.execute("DELETE FROM profile WHERE id <> ?1", params![keeper.id])?;
    prune_loose_rows(&copy)?;
    copy.execute(
        "UPDATE profile SET is_active = 1 WHERE id = ?1",
        params![keeper.id],
    )?;
    drop(copy);
    copy_dir(&app_dir.join(MEDIA_DIR), &folder.path().join(MEDIA_DIR))?;
    let manifest = manifest_for(folder.path(), keeper)?;

    // Out of the first: it is a space of its own now.
    conn.execute("DELETE FROM profile WHERE id = ?1", params![keeper.id])?;
    registry::add(app, app_dir, &manifest.id, folder.path())
}

/// The rows that name a profile without a key to it - the trash, the
/// tombstones, the log of operations - for profiles no longer in this
/// database. The log names a profile by its key (ADR 0014), so either is
/// looked for.
fn prune_loose_rows(conn: &Connection) -> Result<()> {
    for table in ["deletion", "tombstone", "operation"] {
        conn.execute(
            &format!(
                "DELETE FROM {table} WHERE profile_id IS NOT NULL \
                   AND profile_id NOT IN (SELECT id FROM profile) \
                   AND profile_id NOT IN (SELECT key FROM profile)"
            ),
            [],
        )?;
    }
    Ok(())
}

/// The first space's folder: one the move began before - a manifest and no
/// database yet - or a free one.
fn first_folder(spaces: &Path, name: &str) -> PathBuf {
    if let Ok(entries) = std::fs::read_dir(spaces) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.join(super::MANIFEST_FILE).is_file() && !path.join(DATABASE_FILE).exists() {
                return path;
            }
        }
    }
    super::free_folder(spaces, name)
}

/// The manifest of a space a profile became, written once: the one already in
/// the folder, from a move begun before, is kept with its id.
fn manifest_for(folder: &Path, keeper: &Keeper) -> Result<Manifest> {
    if let Ok(manifest) = Manifest::read(folder) {
        return Ok(manifest);
    }
    let kind = if crate::profile::builtin()?
        .iter()
        .any(|shipped| shipped.key == keeper.key)
    {
        keeper.key.clone()
    } else {
        crate::profile::DEFAULT_KIND.to_owned()
    };
    let name = super::checked_name(&keeper.name).unwrap_or_else(|_| keeper.key.clone());
    let manifest = Manifest::new(&name, &kind)?;
    manifest.write(folder)?;
    Ok(manifest)
}

/// Move the files of the old layout into the first space's folder - the
/// database last, so a start stopped half-way still finds it where it was and
/// finishes the move.
fn move_into(app_dir: &Path, folder: &Path) -> Result<()> {
    std::fs::create_dir_all(folder)?;
    for dir in [MEDIA_DIR, ASSISTANT_DIR] {
        let from = app_dir.join(dir);
        let to = folder.join(dir);
        if from.is_dir() && !to.exists() {
            std::fs::rename(&from, &to)?;
        }
    }
    for suffix in ["-wal", "-shm"] {
        let from = app_dir.join(format!("{DATABASE_FILE}{suffix}"));
        if from.is_file() {
            // Checkpointed and closed: whatever is left is empty, and a
            // stale one beside the moved database would be read as its own.
            std::fs::remove_file(&from)?;
        }
    }
    std::fs::rename(app_dir.join(DATABASE_FILE), folder.join(DATABASE_FILE))?;
    Ok(())
}

/// Copy a directory of files, one level: `media/` holds files, not folders.
fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    if !from.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)?.flatten() {
        let path = entry.path();
        if path.is_file() {
            std::fs::copy(&path, to.join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    /// A workspace laid out as before v0.109: every shipped profile seeded
    /// into one database in the application's directory, as `seed` did.
    fn before_spaces(app_dir: &Path) -> Connection {
        let conn = fixtures::workspace_file(&app_dir.join(DATABASE_FILE));
        for (n, shipped) in crate::profile::builtin().unwrap().iter().enumerate() {
            conn.execute(
                "INSERT INTO profile (id, key, name, description, config, is_active, is_builtin, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 't', 't')",
                params![
                    format!("p-{}", shipped.key),
                    shipped.key,
                    shipped.name,
                    shipped.description,
                    serde_json::to_string(&shipped.config).unwrap(),
                    i64::from(n == 0),
                ],
            )
            .unwrap();
        }
        std::fs::create_dir_all(app_dir.join(MEDIA_DIR)).unwrap();
        std::fs::create_dir_all(app_dir.join(ASSISTANT_DIR)).unwrap();
        conn
    }

    fn keys(conn: &Connection) -> Vec<String> {
        crate::profile::list(conn)
            .unwrap()
            .into_iter()
            .map(|profile| profile.key)
            .collect()
    }

    #[test]
    fn a_directory_with_no_workspace_moves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let app = db::open_app_in_memory().unwrap();

        assert!(adopt(dir.path(), &app).unwrap().is_empty());
        assert!(registry::list(&app, dir.path(), None).unwrap().is_empty());
    }

    #[test]
    fn the_profile_with_works_becomes_the_first_space_and_the_empty_ones_go() {
        let dir = tempfile::tempdir().unwrap();
        let app = db::open_app_in_memory().unwrap();
        let conn = before_spaces(dir.path());
        let song = fixtures::song(&conn, "p-music", "Harbour lights");
        std::fs::write(dir.path().join(MEDIA_DIR).join("a.png"), b"png").unwrap();
        drop(conn);

        let ids = adopt(dir.path(), &app).unwrap();

        assert_eq!(ids.len(), 1, "the empty crafts are not spaces");
        let space = registry::get(&app, dir.path(), &ids[0], None).unwrap();
        assert_eq!(space.name, "Studio");
        assert_eq!(space.kind.as_deref(), Some("music"));
        assert!(
            space.ask_name,
            "the person names it the first time it opens"
        );
        let folder = Path::new(&space.path);
        assert!(folder.starts_with(dir.path().join(SPACES_DIR)));
        assert!(
            folder.join(MEDIA_DIR).join("a.png").is_file(),
            "the files moved with it"
        );
        assert!(folder.join(ASSISTANT_DIR).is_dir());
        assert!(
            !dir.path().join(DATABASE_FILE).exists(),
            "moved, not copied"
        );

        let moved = db::open(&folder.join(DATABASE_FILE)).unwrap();
        assert_eq!(keys(&moved), ["music"]);
        assert!(crate::work::get(&moved, &song.id).unwrap().is_some());
        assert_eq!(
            registry::newest(&app, dir.path()).unwrap().unwrap().id,
            ids[0]
        );
    }

    #[test]
    fn a_second_profile_with_works_becomes_a_space_of_its_own() {
        let dir = tempfile::tempdir().unwrap();
        let app = db::open_app_in_memory().unwrap();
        let conn = before_spaces(dir.path());
        let song = fixtures::song(&conn, "p-music", "Harbour lights");
        let post = fixtures::work(&conn, "p-blog", "post", "On harbours");
        drop(conn);

        let ids = adopt(dir.path(), &app).unwrap();

        assert_eq!(ids.len(), 2);
        let first = registry::get(&app, dir.path(), &ids[0], None).unwrap();
        let blog = registry::get(&app, dir.path(), &ids[1], None).unwrap();
        assert_eq!(first.kind.as_deref(), Some("music"));
        assert_eq!(blog.kind.as_deref(), Some("blog"));
        assert!(blog.ask_name);

        let first = db::open(&Path::new(&first.path).join(DATABASE_FILE)).unwrap();
        let blog = db::open(&Path::new(&blog.path).join(DATABASE_FILE)).unwrap();
        assert_eq!(keys(&first), ["music"]);
        assert_eq!(keys(&blog), ["blog"]);
        assert!(crate::work::get(&first, &post.id).unwrap().is_none());
        assert!(crate::work::get(&blog, &post.id).unwrap().is_some());
        assert!(crate::work::get(&blog, &song.id).unwrap().is_none());
        assert!(crate::profile::active(&blog).unwrap().unwrap().is_active);
    }

    #[test]
    fn the_settings_move_to_the_application_and_a_copy_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let app = db::open_app_in_memory().unwrap();
        let conn = before_spaces(dir.path());
        fixtures::song(&conn, "p-music", "Harbour lights");
        drop(conn);
        // A workspace from before 0050 still has the table: put it back with
        // the row a person left in it.
        let raw = Connection::open(dir.path().join(DATABASE_FILE)).unwrap();
        raw.execute_batch(
            "CREATE TABLE assistant_settings (
                 id INTEGER PRIMARY KEY, weekly_budget_usd REAL, updated_at TEXT NOT NULL,
                 parallel_runs INTEGER NOT NULL DEFAULT 3, model TEXT, run_sees TEXT NOT NULL DEFAULT 'nothing');
             INSERT INTO assistant_settings VALUES (1, 12.5, 't', 5, 'sonnet', 'work');",
        )
        .unwrap();
        drop(raw);

        adopt(dir.path(), &app).unwrap();

        let settings = crate::assistant::settings::read(&app).unwrap();
        assert_eq!(settings.parallel_runs, 5);
        assert_eq!(settings.model.as_deref(), Some("sonnet"));
        assert_eq!(crate::assistant::spend::budget(&app).unwrap(), Some(12.5));
        let kept = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("kilna.before-spaces.")
            })
            .count();
        assert_eq!(kept, 1, "the workspace as it was, beside it");
    }

    #[test]
    fn a_move_stopped_before_the_database_went_finishes_in_the_same_folder() {
        let dir = tempfile::tempdir().unwrap();
        let app = db::open_app_in_memory().unwrap();
        let conn = before_spaces(dir.path());
        fixtures::song(&conn, "p-music", "Harbour lights");
        drop(conn);
        // A start that wrote the manifest and moved the files, and stopped.
        let begun = dir.path().join(SPACES_DIR).join("studio");
        let manifest = Manifest::new("Studio", "music").unwrap();
        manifest.write(&begun).unwrap();
        std::fs::rename(dir.path().join(MEDIA_DIR), begun.join(MEDIA_DIR)).unwrap();

        let ids = adopt(dir.path(), &app).unwrap();

        assert_eq!(ids, [manifest.id]);
        assert!(begun.join(DATABASE_FILE).is_file());
        assert!(!dir.path().join(SPACES_DIR).join("studio-2").exists());
        assert!(adopt(dir.path(), &app).unwrap().is_empty(), "moved once");
    }
}
