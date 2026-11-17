//! Gestures on files: copied into the workspace, and let go of.

use std::path::Path;

use rusqlite::Connection;

use super::gesture;
use crate::asset::{self, Asset, NewAsset};
use crate::error::Result;
use crate::journal::Record;

/// Copy a file into the workspace and attach it to a work, a release or a
/// style.
///
/// The path comes from the file picker, so it is a place on this machine; the
/// bytes are copied into the workspace's own `media/` directory and the row
/// holds where they landed.
pub fn attach(conn: &Connection, media: &Path, source: &str, new: NewAsset) -> Result<Asset> {
    gesture(conn, "asset.attach", |act| {
        act.param("source", source);
        act.json("asset", &new)?;
        let minted = act.mint();
        let attached =
            asset::attach_minted(act, act.profile_id(), media, Path::new(source), new, minted)?;

        let named = Record::new("asset.attached").param(
            "name",
            attached
                .original_name
                .clone()
                .or_else(|| attached.label.clone())
                .unwrap_or_default(),
        );
        // A file attached to a release belongs to no work, and the feed's
        // line is about the file either way.
        act.journal(match attached.work_id.as_deref() {
            Some(work_id) => named.about("work", work_id.to_owned()),
            None => named,
        });
        Ok(attached)
    })
}

/// Take a picture straight from the clipboard and attach it to a work - a
/// candidate for its cover, pasted from a generator's page.
pub fn paste(
    conn: &Connection,
    media: &Path,
    bytes: &[u8],
    name: &str,
    new: NewAsset,
) -> Result<Asset> {
    gesture(conn, "asset.attach", |act| {
        act.param("source", format!("<pasted: {name}>"));
        act.json("asset", &new)?;
        let attached = asset::attach_bytes(act, act.profile_id(), media, bytes, name, new)?;
        let named = Record::new("asset.attached").param(
            "name",
            attached
                .original_name
                .clone()
                .or_else(|| attached.label.clone())
                .unwrap_or_default(),
        );
        act.journal(match attached.work_id.as_deref() {
            Some(work_id) => named.about("work", work_id.to_owned()),
            None => named,
        });
        Ok(attached)
    })
}

/// Make a picture the work's cover: the final one, shown in the catalogue,
/// the calendar and the header; the cover it replaces is a candidate again.
pub fn choose_cover(conn: &Connection, id: &str) -> Result<Asset> {
    gesture(conn, "asset.chooseCover", |act| {
        let chosen =
            asset::get(act, id)?.ok_or_else(|| crate::error::Error::not_found("asset", id))?;
        act.param("id", id);
        act.param(
            "title",
            chosen
                .work_id
                .as_deref()
                .map(|work_id| act.title_of(work_id))
                .unwrap_or_default(),
        );
        let before = asset::choose_cover(act, id)?;
        if before.is_empty() {
            act.unchanged();
        }
        act.json("before", &before)?;
        asset::get(act, id)?.ok_or_else(|| crate::error::Error::not_found("asset", id))
    })
}

/// Take a file off what it was attached to: into the trash, where it waits
/// with its bytes and comes back with them (v0.114).
///
/// Until then this forgot the row and removed the copy at once - "not the
/// trash, because a row restored beside bytes that are gone is a promise
/// broken". The trash has kept the bytes of what it holds since it learned
/// to hold a style's pictures (v0.76.1), so the promise can be kept, and a
/// reference taken off a style by a slip of the pointer is one undo away.
/// The original, wherever it came from, was never touched.
pub fn detach(conn: &Connection, id: &str) -> Result<()> {
    // Already gone is what was asked for.
    if asset::get(conn, id)?.is_none() {
        return Ok(());
    }
    super::trash::discard(conn, crate::trash::Entity::File, id).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    /// A file taken off a work waits in the trash with its bytes, comes back
    /// with them, and loses them only when the entry is deleted for good.
    #[test]
    fn a_file_taken_off_waits_in_the_trash_with_its_bytes() {
        let (conn, profile_id, dir) = fixtures::workspace_on_disk();
        let media = crate::state::media_dir_of(&conn).unwrap();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let source = fixtures::file(dir.path(), "take.png");
        let kept = attach(
            &conn,
            &media,
            &source.display().to_string(),
            NewAsset {
                work_id: Some(song.id.clone()),
                ..NewAsset::default()
            },
        )
        .unwrap();
        let bytes = Path::new(&kept.path);

        detach(&conn, &kept.id).unwrap();
        assert!(asset::get(&conn, &kept.id).unwrap().is_none());
        assert!(bytes.is_file(), "the bytes went with the row");

        // One undo away, bytes and all.
        let offer = crate::undo::last(&conn).unwrap().expect("offered");
        assert_eq!(offer.action, "undo.entity.discard");
        crate::undo::undo(&conn, &offer.operation_id).unwrap();
        assert!(asset::get(&conn, &kept.id).unwrap().is_some());
        assert!(bytes.is_file());

        // Deleted for good, the copy goes: nothing names it any more.
        detach(&conn, &kept.id).unwrap();
        let entry = crate::trash::list(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.entity_id == kept.id)
            .expect("the file waits in the trash");
        assert_eq!(entry.entity, crate::trash::Entity::File);
        crate::actions::trash::purge(&conn, &entry.id).unwrap();
        assert!(!bytes.is_file(), "a purged file left its bytes behind");

        // And taking off what is already gone is no failure.
        detach(&conn, &kept.id).unwrap();
    }
}
