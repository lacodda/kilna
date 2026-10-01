//! Gestures on the style dictionary: bricks, their descriptions, and the
//! pictures they were described from.

use std::path::Path;

use rusqlite::Connection;

use super::gesture;
use crate::asset::{self, Asset};
use crate::error::{Error, Result};
use crate::style_brick::{self, NewStyleBrick, StyleBrick, StyleBrickPatch};

pub fn create(conn: &Connection, new: NewStyleBrick) -> Result<StyleBrick> {
    gesture(conn, "style.create", |act| {
        act.json("brick", &new)?;
        let minted = act.mint();
        style_brick::create_minted(act, act.profile_id(), new, minted)
    })
}

pub fn update(conn: &Connection, id: &str, patch: StyleBrickPatch) -> Result<StyleBrick> {
    gesture(conn, "style.update", |act| {
        let before = style_brick::get(act, id)?;
        let patch = match &before {
            Some(brick) => patch.completed(brick),
            None => patch,
        };
        act.param("id", id);
        act.json("patch", &patch)?;
        act.before(before.as_ref(), &patch)?;
        act.stamped();
        style_brick::update_at(act, id, patch, act.at())
    })
}

/// "Restore as in the set": the set's words back on a brick the owner changed,
/// as one `style.update` carrying every field the set speaks for.
pub fn restore(conn: &Connection, id: &str) -> Result<StyleBrick> {
    let brick = style_brick::get(conn, id)?.ok_or_else(|| Error::not_found("style", id))?;
    let patch = crate::style_set::restoring(conn, &brick)?;
    update(conn, id, patch)
}

/// A brick's description, written as the edit a person would make: the same
/// `style.update` the dictionary records, so it is taken back by undo and
/// played back by a rebuild like any other. A kept description lets the brick
/// out of draft, unless it was dropped.
pub fn describe(conn: &Connection, id: &str, text: &str) -> Result<StyleBrick> {
    let before = style_brick::get(conn, id)?.ok_or_else(|| Error::not_found("style", id))?;
    update(
        conn,
        id,
        StyleBrickPatch {
            description: Some(Some(text.to_owned())),
            status: (before.status != style_brick::DROPPED).then(|| style_brick::READY.to_owned()),
            ..StyleBrickPatch::default()
        },
    )
}

/// A reference picture pasted straight onto a brick.
///
/// The bytes are not written into the log, for the reason a pasted frame's are
/// not: an operation carrying a picture would make the log the size of the
/// pictures. It is recorded as the arrival it is, and not replayed.
pub fn paste_reference(
    conn: &Connection,
    media: &Path,
    id: &str,
    bytes: &[u8],
    name: &str,
) -> Result<Asset> {
    gesture(conn, "style.attachReference", |act| {
        act.param("styleBrickId", id);
        act.param("source", format!("<pasted: {name}>"));
        asset::attach_bytes(
            act,
            act.profile_id(),
            media,
            bytes,
            name,
            asset::NewAsset {
                style_brick_id: Some(id.to_owned()),
                ..asset::NewAsset::default()
            },
        )
    })
}
