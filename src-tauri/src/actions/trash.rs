//! Gestures on the trash: putting something in, taking it back out, and
//! letting it go for good. And undo, which takes the last gesture back.

use rusqlite::Connection;

use super::gesture;
use crate::error::Result;
use crate::journal::Record;
use crate::trash::{self, Entity};

/// Move something to the trash and write one line about it.
///
/// Every deletion in the application is this gesture with a different entity,
/// for the reason `announceDeleted` is one function in the window: several
/// near-copies are several chances for one of them to stop recording and
/// nobody to notice. Returns the trash entry, which is what an undo needs.
///
/// The label is read before the deletion, while there is still something to
/// read; which work it belonged to comes from the entry, which has just
/// snapshotted it.
pub fn discard(conn: &Connection, entity: Entity, id: &str) -> Result<String> {
    gesture(conn, "entity.discard", |act| {
        act.param("entity", entity.as_str());
        act.param("entityId", id);
        // Minted here so the entry a restore names is the same one after a
        // rebuild (ADR 0014).
        let minted = act.mint();
        let entry_id = trash::discard_minted(act, entity, id, minted)?;

        let described = trash::list(act, act.profile_id())?
            .into_iter()
            .find(|entry| entry.id == entry_id);

        // The key follows the entity — `score` deleted is `score.deleted` — so
        // the deletions cannot each invent their own wording, and the gate that
        // checks every recorded key has a sentence has one place to look.
        let mut record = Record::new(format!("{}.deleted", entity.as_str())).param(
            "label",
            described
                .as_ref()
                .map_or_else(|| id.to_owned(), |entry| entry.label.clone()),
        );
        // Filed under the work it came from, so the card's History tab shows
        // what was taken out of it — a deleted score is part of that work's
        // story.
        if let Some(origin) = described.as_ref().and_then(|entry| entry.origin.clone()) {
            record = record.param("origin", origin);
        }
        let behind = work_behind(act, entity, id);
        if let Some(work_id) = behind.clone() {
            record = record.about("work", work_id);
        }
        act.journal(record);

        // Deleting the last score, or the release that held the slot, changes
        // what the work's status should say. The work itself is exempt: it is
        // in the trash, and restating a row nobody can see would only make
        // noise.
        if entity != Entity::Work {
            if let Some(work_id) = behind {
                act.restate(&work_id);
            }
        }
        Ok(entry_id)
    })
}

/// The work a just-deleted child belonged to, read from the trash snapshot.
///
/// The live row is gone by now, so the snapshot is the only place left that
/// still knows.
fn work_behind(conn: &Connection, entity: Entity, entity_id: &str) -> Option<String> {
    match entity {
        Entity::Work => Some(entity_id.to_owned()),
        // Neither hangs off a work: a collection holds works, and a style is
        // the workspace's own dictionary.
        Entity::Collection | Entity::Style => None,
        _ => trash::snapshot_work_id(conn, entity, entity_id),
    }
}

/// Put a trashed entry back where it came from.
pub fn restore(conn: &Connection, id: &str) -> Result<()> {
    gesture(conn, "trash.restore", |act| {
        act.param("id", id);
        // What it was, read before the restore spends the entry.
        let described = trash::list(act, act.profile_id())?
            .into_iter()
            .find(|entry| entry.id == id);
        trash::restore(act, id)?;

        if let Some(entry) = described {
            let mut record = Record::new("trash.restored").param("label", entry.label);
            let behind = work_behind(act, entry.entity, &entry.entity_id);
            if let Some(work_id) = behind.clone() {
                record = record.about("work", work_id);
            }
            act.journal(record);

            // A restored score or release is a fact again, and the status has
            // to answer for it — including for a work that came back whole.
            if let Some(work_id) = behind {
                act.restate(&work_id);
            }
        }
        Ok(())
    })
}

/// Drop one entry for good.
pub fn purge(conn: &Connection, id: &str) -> Result<()> {
    gesture(conn, "trash.purge", |act| {
        act.param("id", id);
        trash::purge(act, id)
    })
}

/// Empty the active profile's trash. Returns how many entries went.
pub fn empty(conn: &Connection) -> Result<usize> {
    gesture(conn, "trash.empty", |act| {
        trash::empty(act, act.profile_id())
    })
}

/// Take back the operation the offer names, and say so in the history.
///
/// The id travels back rather than being implied, so an offer read a moment
/// ago cannot silently reverse something newer — [`crate::undo::undo`]
/// refuses when they disagree, and records the undo as its own operation.
pub fn undo(conn: &Connection, operation: &str) -> Result<crate::undo::Undoable> {
    crate::db::unit::atomically(conn, |conn| {
        let taken = crate::undo::undo(conn, operation)?;
        if let Ok(profile_id) = super::active_profile_id(conn) {
            crate::journal::record(
                conn,
                &profile_id,
                Record::new("undo.done").param("action", taken.action.clone()),
            );
        }
        Ok(taken)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fixtures, operation};

    /// A purge that rolls back with the gesture around it leaves the file on
    /// disk: the file goes only once the purge is committed.
    #[test]
    fn a_purge_rolled_back_keeps_the_file() {
        let (conn, profile_id, media) = fixtures::workspace_with_media();
        let work = fixtures::song(&conn, &profile_id, "Harbour lights");
        let file = fixtures::file(media.path(), "cover.png");
        let attached = crate::asset::attach(
            &conn,
            &profile_id,
            media.path(),
            &file,
            crate::asset::NewAsset {
                work_id: Some(work.id.clone()),
                ..Default::default()
            },
        )
        .unwrap();
        let entry = discard(&conn, Entity::Work, &work.id).unwrap();
        let copied = std::path::PathBuf::from(&attached.path);
        assert!(copied.exists());

        let failed = crate::db::unit::atomically(&conn, |c| {
            purge(c, &entry)?;
            Err::<(), _>(crate::Error::Internal("the larger gesture failed".into()))
        });
        assert!(failed.is_err());
        assert!(
            copied.exists(),
            "the file outlived the purge that never happened"
        );

        purge(&conn, &entry).unwrap();
        assert!(!copied.exists(), "a committed purge lets the file go");
    }

    #[test]
    fn an_undo_is_one_operation_and_one_line() {
        let (conn, profile_id) = fixtures::workspace();
        let work = super::super::work::create(
            &conn,
            crate::work::NewWork {
                kind: "song".into(),
                title: "Harbour lights".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let offer = crate::undo::last(&conn).unwrap().unwrap();

        undo(&conn, &offer.operation_id).unwrap();

        assert!(crate::work::get(&conn, &work.id).unwrap().is_none());
        let written = operation::latest(&conn, 1).unwrap().remove(0);
        assert_eq!(written.kind, "undo.work.create");
        assert!(
            crate::journal::list(&conn, &profile_id)
                .unwrap()
                .iter()
                .any(|entry| entry.action == "undo.done")
        );
    }
}
