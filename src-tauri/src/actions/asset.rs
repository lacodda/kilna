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

/// Forget a file and remove the copy the workspace made.
///
/// Not the trash: what the trash promises is that a deletion can be taken
/// back, and a row restored beside bytes that are gone is a promise broken.
/// The original, wherever it came from, was never touched.
pub fn detach(conn: &Connection, id: &str) -> Result<()> {
    gesture(conn, "asset.detach", |act| {
        act.param("id", id);
        asset::delete(act, id)
    })
}
