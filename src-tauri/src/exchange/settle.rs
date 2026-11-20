//! The files a space kept as copies of its own, settled where they belong
//! (v0.164, FR-04 4.3): `kilna --settle-media [--space <name>] [--apply]`.
//!
//! Until v0.164 every file attached to a work was copied into the space's
//! own `media/`, so a cover lived twice - once where the person keeps their
//! work, once inside kilna. Each such copy of a work's file is settled:
//!
//! - when the very same bytes already lie under the media folder, the row
//!   is pointed at that file and kilna's copy goes;
//! - otherwise the copy is moved into the work's folder - its picture into
//!   `images/`, a candidate into `candidates/`, the rest into `raw/`; in a
//!   folder the work shares with others, named after the work.
//!
//! What belongs to no work - a style's picture, a card's - stays in `media/`,
//! as it does for a space with no media folder. A plan and a report unless
//! `--apply` is said; applied, a copy of the database is kept first.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::error::{Error, Result};

/// How a copy is settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum How {
    /// The same bytes lie under the media folder: the row points there.
    Twin,
    /// Moved into the work's folder.
    Moved,
}

/// One copy, and where it goes.
#[derive(Debug, Clone, Serialize)]
pub struct Settled {
    pub asset_id: String,
    pub work_title: String,
    pub from: String,
    pub to: String,
    pub how: How,
    pub bytes: u64,
}

/// A copy left where it is, and why.
#[derive(Debug, Clone, Serialize)]
pub struct Left {
    pub path: String,
    pub reason: String,
}

/// What settling would do.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Plan {
    pub media: String,
    pub root: Option<String>,
    pub settled: Vec<Settled>,
    pub left: Vec<Left>,
    /// Files of `media/` no row names at all: left, said.
    pub strays: Vec<String>,
}

/// What applying did.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Applied {
    pub backup: String,
    pub pointed: usize,
    pub moved: usize,
    pub copies_removed: usize,
    pub failed: Vec<Left>,
}

fn subfolder_for(kind: &str) -> &'static str {
    match kind {
        crate::asset::COVER => "images",
        crate::asset::CANDIDATE => "candidates",
        _ => "raw",
    }
}

/// Every file under `root` by its size - where a twin is looked for.
fn by_size(root: &Path) -> BTreeMap<u64, Vec<PathBuf>> {
    let mut out: BTreeMap<u64, Vec<PathBuf>> = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file()
                && let Ok(metadata) = entry.metadata()
            {
                out.entry(metadata.len()).or_default().push(entry.path());
            }
        }
    }
    out
}

/// Whether two files hold the same bytes, read side by side.
fn same_bytes(a: &Path, b: &Path) -> bool {
    let (Ok(mut a), Ok(mut b)) = (std::fs::File::open(a), std::fs::File::open(b)) else {
        return false;
    };
    let mut left = vec![0u8; 1 << 16];
    let mut right = vec![0u8; 1 << 16];
    loop {
        let Ok(n) = a.read(&mut left) else {
            return false;
        };
        if n == 0 {
            return b.read(&mut right).is_ok_and(|m| m == 0);
        }
        if b.read_exact(&mut right[..n]).is_err() || left[..n] != right[..n] {
            return false;
        }
    }
}

/// An asset as settling reads it: id, work, release, kind, path and the name
/// it came with.
type Row = (
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    Option<String>,
);

/// What settling the copies in `media` would do.
pub fn plan(conn: &Connection, profile_id: &str, media: &Path) -> Result<Plan> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let root = crate::folder::root(conn, profile_id)?.filter(|root| Path::new(root).is_dir());
    let mut out = Plan {
        media: media.to_string_lossy().into_owned(),
        root: root.clone(),
        ..Plan::default()
    };
    let rows: Vec<Row> = conn
        .prepare(
            "SELECT id, work_id, release_id, kind, path, original_name FROM asset
             WHERE profile_id = ?1 ORDER BY created_at, id",
        )?
        .query_map(params![profile_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;

    let named: BTreeSet<String> = rows.iter().map(|row| row.4.to_lowercase()).collect();
    if let Ok(entries) = std::fs::read_dir(media) {
        for entry in entries.flatten() {
            let path = entry.path().to_string_lossy().into_owned();
            if entry.file_type().is_ok_and(|kind| kind.is_file())
                && !named
                    .iter()
                    .any(|one| crate::folder::same_place(Path::new(one), Path::new(&path)))
            {
                out.strays.push(path);
            }
        }
    }

    let sizes = root.as_deref().map(|root| by_size(Path::new(root)));
    let mut taken: BTreeSet<String> = BTreeSet::new();
    let mut seen_paths: BTreeSet<String> = BTreeSet::new();
    for (id, work_id, release_id, kind, path, original) in rows {
        let file = Path::new(&path);
        if !crate::folder::lies_under(file, media) {
            continue;
        }
        // Clones share a file: the first row that names it settles it for
        // every row.
        if !seen_paths.insert(path.to_lowercase()) {
            continue;
        }
        let leave = |reason: &str, out: &mut Plan| {
            out.left.push(Left {
                path: path.clone(),
                reason: reason.to_owned(),
            });
        };
        if !file.is_file() {
            leave("the file is not there", &mut out);
            continue;
        }
        let work_id = match (work_id, release_id) {
            (Some(id), _) => Some(id),
            (None, Some(release)) => crate::release::get(conn, &release)?.map(|one| one.work_id),
            _ => None,
        };
        let Some(work) = work_id
            .as_deref()
            .map(|id| crate::work::get(conn, id))
            .transpose()?
            .flatten()
        else {
            leave("it belongs to no work", &mut out);
            continue;
        };
        let (Some(root), Some(sizes)) = (root.as_deref(), sizes.as_ref()) else {
            leave("the space has no media folder on this machine", &mut out);
            continue;
        };
        let bytes = std::fs::metadata(file).map_or(0, |metadata| metadata.len());
        let twin = sizes
            .get(&bytes)
            .into_iter()
            .flatten()
            .find(|candidate| candidate.as_path() != file && same_bytes(candidate, file));
        if let Some(twin) = twin {
            out.settled.push(Settled {
                asset_id: id,
                work_title: work.title.clone(),
                from: path.clone(),
                to: twin.to_string_lossy().into_owned(),
                how: How::Twin,
                bytes,
            });
            continue;
        }
        let found = crate::folder::locate(conn, &config, &work)?;
        let Some(folder) = found.path.as_deref().map(PathBuf::from) else {
            leave("the work's folder cannot be named yet", &mut out);
            continue;
        };
        if !crate::folder::lies_under(&folder, Path::new(root)) {
            leave("the work's folder is not under the media folder", &mut out);
            continue;
        }
        let name = original
            .filter(|name| !name.trim().is_empty())
            .or_else(|| {
                file.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| id.clone());
        let (dir, name) = match crate::folder::named_prefix(conn, &config, &work) {
            Some(prefix) => (folder, crate::folder::name_after(&prefix, &name)),
            None => (folder.join(subfolder_for(&kind)), name),
        };
        let mut to = crate::folder::free_name(&dir, &name);
        let mut n = 2;
        while !taken.insert(to.to_string_lossy().to_lowercase()) {
            let stem = Path::new(&name)
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            let extension = Path::new(&name)
                .extension()
                .map(|extension| format!(".{}", extension.to_string_lossy()))
                .unwrap_or_default();
            to = crate::folder::free_name(&dir, &format!("{stem} ({n}){extension}"));
            n += 1;
        }
        out.settled.push(Settled {
            asset_id: id,
            work_title: work.title.clone(),
            from: path.clone(),
            to: to.to_string_lossy().into_owned(),
            how: How::Moved,
            bytes,
        });
    }
    Ok(out)
}

/// The rows that name `from` - live and in the trash - pointed at `to`.
fn repoint(conn: &Connection, from: &str, to: &str) -> Result<()> {
    conn.execute(
        "UPDATE asset SET path = ?2 WHERE path = ?1",
        params![from, to],
    )?;
    let quoted = |path: &str| serde_json::to_string(path).unwrap_or_default();
    conn.execute(
        "UPDATE deletion SET snapshot = replace(snapshot, ?1, ?2) WHERE instr(snapshot, ?1) > 0",
        params![quoted(from), quoted(to)],
    )?;
    Ok(())
}

/// Carry a plan out: a copy of the database to `backup` first, then each
/// copy pointed at its twin or moved into its work's folder.
pub fn apply(conn: &Connection, plan: &Plan, backup: &Path) -> Result<Applied> {
    super::backup::write_database(conn, backup)?;
    let mut done = Applied {
        backup: backup.to_string_lossy().into_owned(),
        ..Applied::default()
    };
    for one in &plan.settled {
        match one.how {
            How::Twin => {
                repoint(conn, &one.from, &one.to)?;
                done.pointed += 1;
                match std::fs::remove_file(&one.from) {
                    Ok(()) => done.copies_removed += 1,
                    Err(cause) => crate::log::warn(
                        "settle",
                        &format!("could not remove the copy {}: {cause}", one.from),
                    ),
                }
            }
            How::Moved => {
                let to = Path::new(&one.to);
                if let Some(parent) = to.parent()
                    && let Err(cause) = std::fs::create_dir_all(parent)
                {
                    done.failed.push(Left {
                        path: one.from.clone(),
                        reason: cause.to_string(),
                    });
                    continue;
                }
                match crate::folder::move_file(Path::new(&one.from), to) {
                    Ok(()) => {
                        repoint(conn, &one.from, &one.to)?;
                        done.moved += 1;
                    }
                    Err(reason) => done.failed.push(Left {
                        path: one.from.clone(),
                        reason,
                    }),
                }
            }
        }
    }
    Ok(done)
}

/// Read a space's copies, plan where each goes and write the report; with
/// `apply_it`, keep a copy of the database and apply. The report's folder
/// comes back.
pub fn run(app_dir: &Path, space: Option<&str>, apply_it: bool) -> Result<PathBuf> {
    use crate::space::registry;
    let app = crate::db::open_app(&app_dir.join(crate::db::APP_DATABASE_FILE))?;
    let found = super::target_space(&app, app_dir, space, apply_it)?;
    let folder = registry::folder(&app, app_dir, &found.id)?;
    let (conn, held_by) = crate::state::open_space(&folder, crate::state::Role::Companion)?;
    if apply_it && held_by.is_some() {
        return Err(Error::refused("space.unreachable").param("path", found.path.clone()));
    }
    let profile_id = crate::actions::active_profile_id(&conn)?;
    let media = crate::state::media_dir_beside(&folder.database())?;
    let planned = plan(&conn, &profile_id, &media)?;
    let moment = crate::time::now().replace([':', '.'], "-");
    let dir = app_dir.join("settle-media").join(&moment);
    let applied = if apply_it {
        Some(apply(&conn, &planned, &dir.join("kilna.before-settle.db"))?)
    } else {
        None
    };
    report(&dir, &found.name, &planned, applied.as_ref())?;
    Ok(dir)
}

fn report(dir: &Path, space: &str, plan: &Plan, applied: Option<&Applied>) -> Result<()> {
    use std::fmt::Write as _;
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        dir.join("settle_plan.json"),
        serde_json::to_string_pretty(
            &serde_json::json!({ "space": space, "plan": plan, "applied": applied }),
        )?,
    )?;
    let twins = plan
        .settled
        .iter()
        .filter(|one| one.how == How::Twin)
        .count();
    let mut page = format!("# Settling the files of {space}\n\n");
    let _ = writeln!(
        page,
        "- the space's own copies: `{}`\n- the media folder: `{}`\n- pointed at the same file already under the media folder: {twins}\n- moved into their work's folder: {}\n- left where they are: {}\n- files no row names: {}\n",
        plan.media,
        plan.root.as_deref().unwrap_or("none on this machine"),
        plan.settled.len() - twins,
        plan.left.len(),
        plan.strays.len(),
    );
    match applied {
        Some(done) => {
            let _ = writeln!(
                page,
                "Applied: {} pointed, {} moved, {} copies removed, {} failed. A copy of the database from before is `{}`.\n",
                done.pointed,
                done.moved,
                done.copies_removed,
                done.failed.len(),
                done.backup
            );
        }
        None => {
            page.push_str("A plan: nothing is written until it is run again with `--apply`.\n\n")
        }
    }
    if !plan.settled.is_empty() {
        page.push_str("| Work | From | To | How |\n| --- | --- | --- | --- |\n");
        for one in &plan.settled {
            let _ = writeln!(
                page,
                "| {} | `{}` | `{}` | {} |",
                one.work_title.replace('|', "\\|"),
                one.from,
                one.to,
                match one.how {
                    How::Twin => "the same file",
                    How::Moved => "moved",
                }
            );
        }
        page.push('\n');
    }
    if !plan.left.is_empty() {
        page.push_str("## Left where they are\n\n");
        for one in &plan.left {
            let _ = writeln!(page, "- `{}`: {}", one.path, one.reason);
        }
    }
    std::fs::write(dir.join("settle_plan.md"), page)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    /// A copy whose bytes lie under the media folder points there and goes;
    /// one that does not is moved into its work's folder by its kind; a
    /// style's picture stays.
    #[test]
    fn the_spaces_copies_settle_where_they_belong() {
        let (conn, profile_id, dir) = fixtures::workspace_on_disk();
        let media = crate::state::media_dir_of(&conn).unwrap();
        let root = dir.path().join("library");
        std::fs::create_dir_all(root.join("elsewhere")).unwrap();
        crate::folder::set_root(
            &conn,
            &profile_id,
            Some(&root.to_string_lossy()),
            "2026-11-20T00:00:00Z",
        )
        .unwrap();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");

        // Two copies made the old way, straight into `media/`.
        let twin = root.join("elsewhere").join("cover.png");
        std::fs::write(&twin, b"the same picture").unwrap();
        let copy_of_twin = media.join("a1.png");
        std::fs::write(&copy_of_twin, b"the same picture").unwrap();
        let lone = media.join("a2.wav");
        std::fs::write(&lone, b"a take").unwrap();
        for (id, kind, path, name) in [
            ("a1", "cover", &copy_of_twin, "cover.png"),
            ("a2", "attachment", &lone, "take.wav"),
        ] {
            conn.execute(
                "INSERT INTO asset (id, profile_id, work_id, kind, path, original_name, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, '2026-11-20T00:00:00Z')",
                params![id, profile_id, song.id, kind, path.to_string_lossy(), name],
            )
            .unwrap();
        }

        let planned = plan(&conn, &profile_id, &media).unwrap();
        assert_eq!(planned.settled.len(), 2, "{planned:#?}");
        let pointed = planned
            .settled
            .iter()
            .find(|one| one.asset_id == "a1")
            .unwrap();
        assert_eq!(pointed.how, How::Twin);
        let moved = planned
            .settled
            .iter()
            .find(|one| one.asset_id == "a2")
            .unwrap();
        assert_eq!(moved.how, How::Moved);
        assert!(
            moved.to.ends_with("take.wav") && moved.to.contains("raw"),
            "{}",
            moved.to
        );

        let done = apply(&conn, &planned, &dir.path().join("before.db")).unwrap();
        assert_eq!((done.pointed, done.moved, done.copies_removed), (1, 1, 1));
        assert!(
            !copy_of_twin.exists() && !lone.exists(),
            "kilna's copies are gone"
        );
        assert!(
            twin.is_file() && Path::new(&moved.to).is_file(),
            "the person's files are there"
        );
        let paths: Vec<String> = conn
            .prepare("SELECT path FROM asset ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            paths,
            [twin.to_string_lossy().into_owned(), moved.to.clone()]
        );

        // Settled once, there is nothing left to settle.
        assert!(plan(&conn, &profile_id, &media).unwrap().settled.is_empty());
    }
}
