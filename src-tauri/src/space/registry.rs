//! The spaces this machine knows, in the application's database.
//!
//! A row says where a space's folder is and what this machine did with it:
//! when it was added, last opened, put in the trash. What the space is - its
//! name, kind, colour and mark - is read from its folder every time, so there
//! is one place it is written (ADR 0077).

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use super::{Folder, Manifest, Space};
use crate::error::{Error, Result};
use crate::time::now;

/// A row of the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    id: String,
    path: String,
    opened_at: Option<String>,
    trashed_at: Option<String>,
    ask_name: bool,
}

const SELECT: &str = "SELECT id, path, opened_at, trashed_at, ask_name FROM space";

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Row> {
    Ok(Row {
        id: row.get(0)?,
        path: row.get(1)?,
        opened_at: row.get(2)?,
        trashed_at: row.get(3)?,
        ask_name: row.get::<_, i64>(4)? == 1,
    })
}

/// How a folder is written down: relative to the application's directory when
/// it lies under it, so a portable copy on a drive that changes its letter
/// keeps its spaces; whole otherwise. Forward slashes either way.
fn stored(app_dir: &Path, folder: &Path) -> String {
    let relative = folder
        .strip_prefix(app_dir)
        .ok()
        .filter(|rest| !rest.as_os_str().is_empty());
    let path = relative.unwrap_or(folder);
    path.to_string_lossy().replace('\\', "/")
}

/// The folder a stored path names.
fn resolved(app_dir: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        app_dir.join(path)
    }
}

/// Write a space's folder into the registry, and give back the id it is known
/// by.
///
/// A folder already listed keeps its row. A folder whose file carries the id of
/// another row is one of two things: that space moved - the old folder is gone -
/// and the row follows it; or a copy of it, both folders standing - and the copy
/// becomes a space of its own, with a new id written into its file, so that
/// opening one never opens the other.
pub fn add(conn: &Connection, app_dir: &Path, id: &str, folder: &Path) -> Result<String> {
    let path = stored(app_dir, folder);
    if let Some(known) = conn
        .query_row(
            "SELECT id FROM space WHERE path = ?1",
            params![path],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        return Ok(known);
    }

    let at = now();
    if let Some(other) = row(conn, id)? {
        let elsewhere = resolved(app_dir, &other.path);
        if elsewhere.join(super::MANIFEST_FILE).is_file() {
            let mut manifest = Manifest::read(folder)?;
            manifest.id = uuid::Uuid::new_v4().to_string();
            manifest.write(folder)?;
            conn.execute(
                "INSERT INTO space (id, path, added_at) VALUES (?1, ?2, ?3)",
                params![manifest.id, path, at],
            )?;
            return Ok(manifest.id);
        }
        conn.execute(
            "UPDATE space SET path = ?2 WHERE id = ?1",
            params![id, path],
        )?;
        return Ok(id.to_owned());
    }

    conn.execute(
        "INSERT INTO space (id, path, added_at) VALUES (?1, ?2, ?3)",
        params![id, path, at],
    )?;
    Ok(id.to_owned())
}

/// Ask the window to name a space the first time it opens it: its name was
/// given by kilna, not by the person.
pub fn ask_name(conn: &Connection, id: &str) -> Result<()> {
    changed(
        conn.execute("UPDATE space SET ask_name = 1 WHERE id = ?1", params![id])?,
        id,
    )
}

/// The person named the space; the window stops asking.
pub fn named(conn: &Connection, id: &str) -> Result<()> {
    changed(
        conn.execute("UPDATE space SET ask_name = 0 WHERE id = ?1", params![id])?,
        id,
    )
}

/// The space was opened now.
pub fn touch(conn: &Connection, id: &str) -> Result<()> {
    changed(
        conn.execute(
            "UPDATE space SET opened_at = ?2 WHERE id = ?1",
            params![id, now()],
        )?,
        id,
    )
}

/// Put a space aside: off the switcher, its folder untouched. The open one
/// cannot be: there would be nothing left open.
pub fn trash(conn: &Connection, id: &str, open: &str) -> Result<()> {
    if id == open {
        return Err(Error::refused("space.trashOpen"));
    }
    changed(
        conn.execute(
            "UPDATE space SET trashed_at = ?2 WHERE id = ?1 AND trashed_at IS NULL",
            params![id, now()],
        )?,
        id,
    )
}

/// Take a space back out of the trash.
pub fn restore(conn: &Connection, id: &str) -> Result<()> {
    changed(
        conn.execute(
            "UPDATE space SET trashed_at = NULL WHERE id = ?1",
            params![id],
        )?,
        id,
    )
}

/// Forget a space in the trash: its row goes, its folder stays where it is -
/// kilna never deletes a space's files, and the folder can be opened again by
/// its `space.kilna`.
pub fn forget(conn: &Connection, id: &str) -> Result<()> {
    let Some(row) = row(conn, id)? else {
        return Err(Error::not_found("space", id));
    };
    if row.trashed_at.is_none() {
        return Err(Error::refused("space.forgetNotTrashed"));
    }
    conn.execute("DELETE FROM space WHERE id = ?1", params![id])?;
    Ok(())
}

/// The folder of a listed space.
pub fn folder(conn: &Connection, app_dir: &Path, id: &str) -> Result<Folder> {
    let row = row(conn, id)?.ok_or_else(|| Error::not_found("space", id))?;
    Ok(Folder::new(resolved(app_dir, &row.path)))
}

/// One listed space, read with its folder.
pub fn get(conn: &Connection, app_dir: &Path, id: &str, open: Option<&str>) -> Result<Space> {
    let row = row(conn, id)?.ok_or_else(|| Error::not_found("space", id))?;
    Ok(described(app_dir, row, open))
}

/// Every listed space, the trashed ones too, the newest opened first and the
/// never opened after them by when they were added.
pub fn list(conn: &Connection, app_dir: &Path, open: Option<&str>) -> Result<Vec<Space>> {
    let rows = conn
        .prepare(&format!(
            "{SELECT} ORDER BY opened_at IS NULL, opened_at DESC, added_at, id"
        ))?
        .query_map([], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows
        .into_iter()
        .map(|row| described(app_dir, row, open))
        .collect())
}

/// The space a start opens: the newest opened that is not in the trash and
/// can be reached.
pub fn newest(conn: &Connection, app_dir: &Path) -> Result<Option<Space>> {
    Ok(list(conn, app_dir, None)?
        .into_iter()
        .find(|space| space.trashed_at.is_none() && space.reachable))
}

/// A listed space by its name, as an agent names it: exactly, then without
/// regard to case. Two spaces of one name are refused with both folders.
pub fn by_name(conn: &Connection, app_dir: &Path, name: &str) -> Result<Space> {
    let spaces: Vec<Space> = list(conn, app_dir, None)?
        .into_iter()
        .filter(|space| space.trashed_at.is_none())
        .collect();
    if let Some(found) = spaces.iter().find(|space| space.id == name) {
        return Ok(found.clone());
    }
    let wanted = name.trim().to_lowercase();
    let matching: Vec<&Space> = spaces
        .iter()
        .filter(|space| space.name.to_lowercase() == wanted)
        .collect();
    match matching.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err(Error::refused("space.unknown").param("named", name).param(
            "known",
            spaces
                .iter()
                .map(|space| space.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        )),
        many => Err(Error::refused("space.ambiguous")
            .param("named", name)
            .param(
                "paths",
                many.iter()
                    .map(|space| space.path.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
            )),
    }
}

fn row(conn: &Connection, id: &str) -> Result<Option<Row>> {
    Ok(conn
        .query_row(&format!("{SELECT} WHERE id = ?1"), params![id], read_row)
        .optional()?)
}

fn changed(rows: usize, id: &str) -> Result<()> {
    if rows == 0 {
        return Err(Error::not_found("space", id));
    }
    Ok(())
}

fn described(app_dir: &Path, row: Row, open: Option<&str>) -> Space {
    let folder = resolved(app_dir, &row.path);
    let manifest = Manifest::read(&folder).ok();
    let reachable = manifest.is_some() && folder.join(crate::db::DATABASE_FILE).is_file();
    let fallback = folder.file_name().map_or_else(
        || row.path.clone(),
        |name| name.to_string_lossy().into_owned(),
    );
    Space {
        open: open == Some(row.id.as_str()),
        name: manifest
            .as_ref()
            .map_or(fallback, |manifest| manifest.name.clone()),
        kind: manifest.as_ref().map(|manifest| manifest.kind.clone()),
        colour: manifest
            .as_ref()
            .and_then(|manifest| manifest.colour.clone()),
        mark: manifest.as_ref().and_then(|manifest| manifest.mark.clone()),
        path: folder.display().to_string(),
        id: row.id,
        opened_at: row.opened_at,
        trashed_at: row.trashed_at,
        ask_name: row.ask_name,
        reachable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::space::{NewSpace, create};

    fn made(app: &Connection, dir: &Path, name: &str) -> Space {
        create(
            app,
            dir,
            &NewSpace {
                name: name.into(),
                kind: "music".into(),
                colour: None,
                mark: None,
            },
            None,
        )
        .unwrap()
    }

    #[test]
    fn a_folder_under_the_application_is_written_relative_to_it() {
        let app_dir = Path::new("C:/data/kilna");
        assert_eq!(
            stored(app_dir, &app_dir.join("spaces").join("blog")),
            "spaces/blog"
        );
        assert_eq!(
            resolved(app_dir, "spaces/blog"),
            app_dir.join("spaces/blog")
        );
        let elsewhere = std::env::temp_dir().join("blog");
        assert_eq!(resolved(app_dir, &stored(app_dir, &elsewhere)), elsewhere);
    }

    #[test]
    fn the_newest_opened_comes_first_and_a_start_opens_it() {
        let dir = tempfile::tempdir().unwrap();
        let app = crate::db::open_app_in_memory().unwrap();
        let first = made(&app, dir.path(), "First");
        let second = made(&app, dir.path(), "Second");
        touch(&app, &second.id).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        touch(&app, &first.id).unwrap();

        let names: Vec<String> = list(&app, dir.path(), None)
            .unwrap()
            .into_iter()
            .map(|space| space.name)
            .collect();
        assert_eq!(names, ["First", "Second"]);
        assert_eq!(newest(&app, dir.path()).unwrap().unwrap().id, first.id);
    }

    #[test]
    fn a_trashed_space_is_not_opened_and_its_folder_stays_when_forgotten() {
        let dir = tempfile::tempdir().unwrap();
        let app = crate::db::open_app_in_memory().unwrap();
        let kept = made(&app, dir.path(), "Kept");
        let aside = made(&app, dir.path(), "Aside");
        touch(&app, &aside.id).unwrap();

        assert_eq!(
            trash(&app, &kept.id, &kept.id)
                .unwrap_err()
                .refusal()
                .unwrap()
                .code,
            "space.trashOpen"
        );
        assert_eq!(
            forget(&app, &aside.id).unwrap_err().refusal().unwrap().code,
            "space.forgetNotTrashed"
        );
        trash(&app, &aside.id, &kept.id).unwrap();
        assert_eq!(newest(&app, dir.path()).unwrap().unwrap().id, kept.id);

        forget(&app, &aside.id).unwrap();
        assert_eq!(list(&app, dir.path(), None).unwrap().len(), 1);
        assert!(
            Path::new(&aside.path)
                .join(super::super::MANIFEST_FILE)
                .is_file()
        );
    }

    #[test]
    fn a_space_out_of_reach_is_listed_by_its_folder_and_not_opened() {
        let dir = tempfile::tempdir().unwrap();
        let app = crate::db::open_app_in_memory().unwrap();
        let gone = made(&app, dir.path(), "Gone");
        touch(&app, &gone.id).unwrap();
        std::fs::remove_file(Path::new(&gone.path).join(super::super::MANIFEST_FILE)).unwrap();

        let listed = get(&app, dir.path(), &gone.id, None).unwrap();
        assert!(!listed.reachable);
        assert_eq!(listed.name, "gone");
        assert_eq!(newest(&app, dir.path()).unwrap(), None);
    }

    #[test]
    fn a_moved_folder_keeps_its_space_and_a_copy_becomes_another() {
        let dir = tempfile::tempdir().unwrap();
        let app = crate::db::open_app_in_memory().unwrap();
        let space = made(&app, dir.path(), "Moving");
        let copy = dir.path().join("copy");
        std::fs::create_dir(&copy).unwrap();
        std::fs::copy(
            Path::new(&space.path).join(super::super::MANIFEST_FILE),
            copy.join(super::super::MANIFEST_FILE),
        )
        .unwrap();

        let copied = add(&app, dir.path(), &space.id, &copy).unwrap();
        assert_ne!(
            copied, space.id,
            "a copy beside its original is a space of its own"
        );
        assert_eq!(Manifest::read(&copy).unwrap().id, copied);

        let moved = dir.path().join("moved");
        std::fs::rename(&space.path, &moved).unwrap();
        assert_eq!(add(&app, dir.path(), &space.id, &moved).unwrap(), space.id);
        assert_eq!(folder(&app, dir.path(), &space.id).unwrap().path(), moved);
    }

    #[test]
    fn a_space_is_found_by_its_name_and_two_of_one_name_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let app = crate::db::open_app_in_memory().unwrap();
        let night = made(&app, dir.path(), "Night");
        made(&app, dir.path(), "Day");
        made(&app, dir.path(), "Day");

        assert_eq!(by_name(&app, dir.path(), "night").unwrap().id, night.id);
        assert_eq!(by_name(&app, dir.path(), &night.id).unwrap().id, night.id);
        assert_eq!(
            by_name(&app, dir.path(), "Day")
                .unwrap_err()
                .refusal()
                .unwrap()
                .code,
            "space.ambiguous"
        );
        assert_eq!(
            by_name(&app, dir.path(), "Dusk")
                .unwrap_err()
                .refusal()
                .unwrap()
                .code,
            "space.unknown"
        );
    }
}
