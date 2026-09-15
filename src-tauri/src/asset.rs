//! Files a person attaches: a cover, a reference, a picture of the thing.
//!
//! The bytes live on disk and the row holds the path (ADR 0002): binaries in
//! the database bloat every backup and every future sync of it. The file is
//! *copied* into the workspace's own `media/` directory rather than pointed
//! at where it lies, so that one folder is the whole workspace — a backup
//! takes it with the database, and moving a workspace is moving a folder.
//! Pointing at a file elsewhere on the machine would make a cover that works
//! until the day you tidy your downloads.
//!
//! Inside `media/` a file is named by the asset's id and the extension it
//! arrived with. Nothing outside can rename it into a collision, and nothing
//! inside has to ask what to do when two people attach `cover.png`. The name
//! it arrived under is kept in the row and shown: it is what a person
//! recognises, and what a generator wrote in it.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::minted::Minted;

/// What an asset is for, as the application reads it.
///
/// A word rather than a vocabulary of the profile: unlike a kind of shot or a
/// kind of note, this is not the craft's word for something it judges — it is
/// how the application decides what to *show*. A cover goes in the header, a
/// reference goes in the gallery.
pub const COVER: &str = "cover";

/// A file attached to a work or a release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub profile_id: String,
    pub work_id: Option<String>,
    pub release_id: Option<String>,
    /// What it is for: `cover`, or a plain attachment.
    pub kind: String,
    /// Where the bytes are, absolute, inside the workspace's `media/`.
    pub path: String,
    /// What a person calls it. The original file name when they called it
    /// nothing.
    pub label: Option<String>,
    /// The name the file arrived under — what the world outside calls it.
    pub original_name: Option<String>,
    pub created_at: String,
}

/// What to attach, and to what.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewAsset {
    #[serde(default)]
    pub work_id: Option<String>,
    #[serde(default)]
    pub release_id: Option<String>,
    /// `attachment` when omitted.
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
}

/// The kind an asset takes when nothing says otherwise.
const ATTACHMENT: &str = "attachment";

const SELECT: &str = "SELECT id, profile_id, work_id, release_id, kind, path, label, \
     original_name, created_at FROM asset";

/// Copy a file into the workspace and record it.
pub fn attach(
    conn: &Connection,
    profile_id: &str,
    media_dir: &Path,
    source: &Path,
    new: NewAsset,
) -> Result<Asset> {
    attach_minted(conn, profile_id, media_dir, source, new, Minted::fresh())
}

/// Copy and record with the id and moment already decided — the seam a
/// replay comes back through, see ADR 0014.
///
/// The copy happens before the row: a row pointing at a file that was never
/// written is a broken cover, while a file with no row is a stray byte in a
/// directory nobody reads. Of the two, the stray byte is the one a person
/// never notices.
pub fn attach_minted(
    conn: &Connection,
    profile_id: &str,
    media_dir: &Path,
    source: &Path,
    new: NewAsset,
    minted: Minted,
) -> Result<Asset> {
    if new.work_id.is_none() && new.release_id.is_none() {
        return Err(Error::Other(
            "a file is attached to a work or to a release".into(),
        ));
    }
    if let Some(work_id) = new.work_id.as_deref() {
        let work =
            crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
        if work.profile_id != profile_id {
            return Err(Error::Other(
                "a file is attached to a work of the same profile".into(),
            ));
        }
    }

    if !source.is_file() {
        return Err(Error::Other(format!(
            "there is no file at {}",
            source.display()
        )));
    }
    let original_name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());

    let stored = media_dir.join(stored_name(minted.id(), source));
    // Never over a file that is already there. The name is the asset's id,
    // so a collision means this id is taken — by a row being replayed, or by
    // a mistake — and copying over it would destroy bytes another row points
    // at. Found by the test that refuses the same id twice.
    if stored.exists() {
        return Err(Error::Other(format!(
            "the workspace already holds a file for `{}`",
            minted.id()
        )));
    }
    std::fs::copy(source, &stored).map_err(|cause| {
        Error::Other(format!(
            "could not copy {} into the workspace: {cause}",
            source.display()
        ))
    })?;

    let kind = new
        .kind
        .map(|kind| kind.trim().to_owned())
        .filter(|kind| !kind.is_empty())
        .unwrap_or_else(|| ATTACHMENT.to_owned());
    let label = new
        .label
        .map(|label| label.trim().to_owned())
        .filter(|label| !label.is_empty());

    let written = conn.execute(
        "INSERT INTO asset (id, profile_id, work_id, release_id, kind, path, label, original_name, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            minted.id(),
            profile_id,
            new.work_id,
            new.release_id,
            kind,
            stored.to_string_lossy(),
            label,
            original_name,
            minted.at()
        ],
    );
    if let Err(cause) = written {
        // The row is what makes the copy an asset; without it the file is
        // litter, and litter in the workspace's own directory is litter a
        // backup carries forever.
        let _ = std::fs::remove_file(&stored);
        return Err(cause.into());
    }

    get(conn, minted.id())?.ok_or_else(|| Error::not_found("asset", minted.id()))
}

/// The name a file takes inside `media/`: the asset's id, and the extension
/// it arrived with so that anything reading the directory still knows what it
/// is holding.
fn stored_name(id: &str, source: &Path) -> String {
    match source.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if !ext.is_empty() => format!("{id}.{}", ext.to_lowercase()),
        _ => id.to_owned(),
    }
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Asset>> {
    let mut statement = conn.prepare(&format!("{SELECT} WHERE id = ?1"))?;
    let row = statement.query_row(params![id], read).optional()?;
    Ok(row)
}

/// Everything attached to a work, oldest first.
pub fn for_work(conn: &Connection, work_id: &str) -> Result<Vec<Asset>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE work_id = ?1 ORDER BY created_at, rowid"
    ))?;
    let rows = statement
        .query_map(params![work_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Everything attached to a release, oldest first.
pub fn for_release(conn: &Connection, release_id: &str) -> Result<Vec<Asset>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE release_id = ?1 ORDER BY created_at, rowid"
    ))?;
    let rows = statement
        .query_map(params![release_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// The work's cover: the newest file attached to it as one.
///
/// Newest rather than one marked chosen: attaching a second cover is what a
/// person does to change the first, and a flag to keep in step would be a
/// second truth about the same thing.
pub fn cover_of(conn: &Connection, work_id: &str) -> Result<Option<Asset>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE work_id = ?1 AND kind = ?2 ORDER BY created_at DESC, rowid DESC LIMIT 1"
    ))?;
    let row = statement
        .query_row(params![work_id, COVER], read)
        .optional()?;
    Ok(row)
}

/// The covers of many works at once, by work id.
///
/// The catalogue draws two hundred rows and asking per row is the shape that
/// made the predecessor's screens slow.
pub fn covers_of(conn: &Connection, profile_id: &str) -> Result<Vec<(String, String)>> {
    let mut statement = conn.prepare(
        "SELECT a.work_id, a.path FROM asset a
         WHERE a.profile_id = ?1 AND a.kind = ?2 AND a.work_id IS NOT NULL
           AND a.created_at = (
             SELECT max(b.created_at) FROM asset b
             WHERE b.work_id = a.work_id AND b.kind = a.kind
           )
         ORDER BY a.work_id",
    )?;
    let rows = statement
        .query_map(params![profile_id, COVER], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Forget an asset and remove the file it copied in.
///
/// The row goes first: while it is there the file is an asset, and a row
/// pointing at a file that is already gone is the state a person sees as a
/// broken picture.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    let Some(asset) = get(conn, id)? else {
        return Ok(());
    };
    conn.execute("DELETE FROM asset WHERE id = ?1", params![id])?;
    let path = PathBuf::from(&asset.path);
    if let Err(cause) = std::fs::remove_file(&path) {
        // Said, not raised: the row is gone either way, and a file left
        // behind is not a reason to tell a person their deletion failed.
        eprintln!("asset: could not remove {}: {cause}", path.display());
    }
    Ok(())
}

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Asset> {
    Ok(Asset {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        work_id: row.get(2)?,
        release_id: row.get(3)?,
        kind: row.get(4)?,
        path: row.get(5)?,
        label: row.get(6)?,
        original_name: row.get(7)?,
        created_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::{self, NewWork};
    use crate::{db, profile};

    fn workspace() -> (Connection, String, tempfile::TempDir) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        let media = tempfile::tempdir().unwrap();
        (conn, profile_id, media)
    }

    fn a_work(conn: &Connection, profile_id: &str) -> String {
        work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id
    }

    fn a_file(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, b"not really a picture").unwrap();
        path
    }

    /// The file is copied in under the asset's id, and the name it arrived
    /// under is kept: that is what a person recognises it by.
    #[test]
    fn a_file_is_copied_in_and_remembers_the_name_it_came_with() {
        let (conn, profile_id, media) = workspace();
        let outside = tempfile::tempdir().unwrap();
        let source = a_file(outside.path(), "scene-04-still-v3.PNG");
        let work_id = a_work(&conn, &profile_id);

        let asset = attach(
            &conn,
            &profile_id,
            media.path(),
            &source,
            NewAsset {
                work_id: Some(work_id.clone()),
                kind: Some(COVER.into()),
                ..NewAsset::default()
            },
        )
        .unwrap();

        assert_eq!(
            asset.original_name.as_deref(),
            Some("scene-04-still-v3.PNG")
        );
        assert_eq!(asset.kind, COVER);
        let stored = PathBuf::from(&asset.path);
        assert!(stored.is_file(), "the bytes are in the workspace");
        assert_eq!(
            stored.file_name().unwrap().to_string_lossy(),
            format!("{}.png", asset.id),
            "named by id, extension lowercased"
        );
        assert!(source.is_file(), "the original is copied, not moved");

        // And the workspace can find it again.
        assert_eq!(for_work(&conn, &work_id).unwrap().len(), 1);
        assert_eq!(
            cover_of(&conn, &work_id).unwrap().map(|a| a.id),
            Some(asset.id)
        );
    }

    /// Attaching a second cover is how a person changes the first: the newest
    /// is the cover, and no flag has to be kept in step.
    #[test]
    fn the_newest_cover_is_the_cover() {
        let (conn, profile_id, media) = workspace();
        let outside = tempfile::tempdir().unwrap();
        let work_id = a_work(&conn, &profile_id);

        let first = attach(
            &conn,
            &profile_id,
            media.path(),
            &a_file(outside.path(), "first.png"),
            NewAsset {
                work_id: Some(work_id.clone()),
                kind: Some(COVER.into()),
                ..NewAsset::default()
            },
        )
        .unwrap();
        let second = attach_minted(
            &conn,
            &profile_id,
            media.path(),
            &a_file(outside.path(), "second.png"),
            NewAsset {
                work_id: Some(work_id.clone()),
                kind: Some(COVER.into()),
                ..NewAsset::default()
            },
            Minted::of("second-asset", "2030-01-01T00:00:00.000Z"),
        )
        .unwrap();

        assert_eq!(
            cover_of(&conn, &work_id).unwrap().map(|a| a.id),
            Some(second.id.clone())
        );
        assert_eq!(
            for_work(&conn, &work_id).unwrap().len(),
            2,
            "the one it replaced is still attached, not destroyed"
        );

        let covers = covers_of(&conn, &profile_id).unwrap();
        assert_eq!(covers.len(), 1, "one cover per work, for the catalogue");
        assert_eq!(covers[0].0, work_id);
        assert!(covers[0].1.contains(&second.id), "and it is the newest");
        let _ = first;
    }

    /// A file that is not there, and an attachment to nothing, are refused
    /// before anything is copied.
    #[test]
    fn what_cannot_be_attached_is_refused_before_it_is_copied() {
        let (conn, profile_id, media) = workspace();
        let work_id = a_work(&conn, &profile_id);

        let missing = attach(
            &conn,
            &profile_id,
            media.path(),
            Path::new("nowhere/at/all.png"),
            NewAsset {
                work_id: Some(work_id.clone()),
                ..NewAsset::default()
            },
        );
        assert!(missing.is_err(), "no file, no asset");

        let outside = tempfile::tempdir().unwrap();
        let orphan = attach(
            &conn,
            &profile_id,
            media.path(),
            &a_file(outside.path(), "x.png"),
            NewAsset::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(orphan.contains("work or to a release"), "{orphan}");

        assert_eq!(
            std::fs::read_dir(media.path()).unwrap().count(),
            0,
            "nothing was copied in"
        );
    }

    /// A refused attachment leaves nothing behind in the workspace.
    ///
    /// The copy happens before the row, so a row that cannot be written must
    /// take its copy back — otherwise every failed attach leaves a file in a
    /// directory the backup carries forever and nobody reads. Reached here by
    /// a release id that names nothing: the file copies, and the foreign key
    /// refuses the row.
    #[test]
    fn a_row_that_cannot_be_written_takes_its_copy_back() {
        let (conn, profile_id, media) = workspace();
        let outside = tempfile::tempdir().unwrap();
        let work_id = a_work(&conn, &profile_id);
        let source = a_file(outside.path(), "doomed.png");

        // A release that does not exist: the copy succeeds, the row does not.
        let refused = attach(
            &conn,
            &profile_id,
            media.path(),
            &source,
            NewAsset {
                release_id: Some("no-such-release".into()),
                ..NewAsset::default()
            },
        );
        assert!(refused.is_err(), "a row against nothing is refused");
        assert_eq!(
            std::fs::read_dir(media.path()).unwrap().count(),
            0,
            "and the copy it had already made is taken back"
        );

        // The same id twice cannot overwrite the bytes the first one holds.
        let minted = Minted::of("the-same-id", "2030-01-01T00:00:00.000Z");
        let about = || NewAsset {
            work_id: Some(work_id.clone()),
            ..NewAsset::default()
        };
        attach_minted(
            &conn,
            &profile_id,
            media.path(),
            &source,
            about(),
            minted.clone(),
        )
        .unwrap();
        let taken = attach_minted(&conn, &profile_id, media.path(), &source, about(), minted);
        assert!(taken.is_err(), "the id is taken");
        assert_eq!(
            std::fs::read_dir(media.path()).unwrap().count(),
            1,
            "and the file the first one copied in still stands"
        );
        assert_eq!(for_work(&conn, &work_id).unwrap().len(), 1);
    }

    /// Forgetting an asset takes its file with it.
    #[test]
    fn deleting_an_asset_removes_the_file_it_brought_in() {
        let (conn, profile_id, media) = workspace();
        let outside = tempfile::tempdir().unwrap();
        let work_id = a_work(&conn, &profile_id);
        let asset = attach(
            &conn,
            &profile_id,
            media.path(),
            &a_file(outside.path(), "gone.png"),
            NewAsset {
                work_id: Some(work_id.clone()),
                ..NewAsset::default()
            },
        )
        .unwrap();
        let stored = PathBuf::from(&asset.path);
        assert!(stored.is_file());

        delete(&conn, &asset.id).unwrap();
        assert!(get(&conn, &asset.id).unwrap().is_none());
        assert!(!stored.exists(), "the bytes go with the row");
        assert!(for_work(&conn, &work_id).unwrap().is_empty());
    }
}
