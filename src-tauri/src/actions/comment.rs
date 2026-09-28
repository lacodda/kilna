//! Gestures on the audience's comments and the replies to them.

use rusqlite::Connection;

use super::gesture;
use crate::comment::{self, Comment, CommentPatch, NewComment};
use crate::error::{Error, Result};

pub fn create(conn: &Connection, new: NewComment) -> Result<Comment> {
    gesture(conn, "comment.create", |act| {
        act.json("comment", &new)?;
        let minted = act.mint();
        comment::create_minted(act, act.profile_id(), new, minted)
    })
}

pub fn update(conn: &Connection, id: &str, patch: CommentPatch) -> Result<Comment> {
    gesture(conn, "comment.update", |act| {
        let before = comment::get(act, id)?;
        act.param("id", id);
        act.json("patch", &patch)?;
        act.before(before.as_ref(), &patch)?;
        act.stamped();
        comment::update_at(act, id, patch, act.at())
    })
}

/// Write a comment's reply: an edit with what stood there before, so it can
/// be taken back.
pub fn reply(conn: &Connection, id: &str, text: &str) -> Result<Comment> {
    comment::get(conn, id)?.ok_or_else(|| Error::not_found("comment", id))?;
    update(
        conn,
        id,
        CommentPatch {
            reply: Some(Some(text.to_owned())),
            ..CommentPatch::default()
        },
    )
}
