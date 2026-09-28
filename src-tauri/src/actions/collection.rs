//! Gestures on collections: an album, a book, a season.

use rusqlite::Connection;

use super::gesture;
use crate::collection::{self, Collection, CollectionPatch, NewCollection};
use crate::error::Result;

pub fn create(conn: &Connection, new: NewCollection) -> Result<Collection> {
    gesture(conn, "collection.create", |act| {
        act.json("collection", &new)?;
        let minted = act.mint();
        collection::create_minted(act, act.profile_id(), new, minted)
    })
}

pub fn update(conn: &Connection, id: &str, patch: CollectionPatch) -> Result<Collection> {
    gesture(conn, "collection.update", |act| {
        let before = collection::get(act, id)?;
        act.param("id", id);
        act.json("patch", &patch)?;
        act.before(before.as_ref(), &patch)?;
        act.stamped();
        collection::update_at(act, id, patch, act.at())
    })
}

/// Put works in a collection in the order given; works not listed leave it.
pub fn set_contents(conn: &Connection, id: &str, work_ids: &[String]) -> Result<()> {
    gesture(conn, "collection.setContents", |act| {
        act.param("id", id);
        act.json("workIds", &work_ids)?;
        act.stamped();
        collection::set_contents_at(act, id, work_ids, act.at())
    })
}
