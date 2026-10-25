//! Gestures on a work's versions: a new draft, and an edit of one.

use rusqlite::Connection;

use super::gesture;
use crate::error::Result;
use crate::journal::Record;
use crate::work::version::{self, NewVersion, Version, VersionSummary};

/// Add a version to a work.
///
/// Journalled under the work rather than the version: the History tab people
/// want is the work's, and a version's own tab would hold one line saying it
/// was made.
pub fn create(conn: &Connection, work_id: &str, new: NewVersion) -> Result<Version> {
    gesture(conn, "version.create", |act| {
        act.param("workId", work_id);
        act.json("version", &new)?;
        let minted = act.mint();
        let created = version::create_minted(act, work_id, new, minted)?;
        act.journal(
            Record::new("version.created")
                .param("role", created.role.clone())
                .param("revision", created.revision)
                .param("title", act.title_of(work_id))
                .about("work", work_id.to_owned()),
        );
        Ok(created)
    })
}

/// Mark a version the main one of its role - the style prompt the song is
/// made with (v0.100.3, ADR 0069).
///
/// What was main before travels in the log, so an undo puts it back, or puts
/// back that nothing was. Not journalled: like making a draft current, it is
/// a choice among versions the History tab already lists.
pub fn set_main(conn: &Connection, work_id: &str, version_id: &str) -> Result<VersionSummary> {
    gesture(conn, "version.setMain", |act| {
        act.param("workId", work_id);
        act.param("id", version_id);
        act.stamped();
        let before = version::set_main_at(act, work_id, version_id, act.at())?;
        if before.as_deref() == Some(version_id) {
            act.unchanged();
        }
        act.json("before", &before)?;
        version::list(act, work_id)?
            .into_iter()
            .find(|summary| summary.id == version_id)
            .ok_or_else(|| crate::error::Error::not_found("version", version_id))
    })
}

/// Rewrite a version's body in place.
///
/// The body before travels whole, so an undo has something to put back. Not a
/// diff: a body is one field, and a diff would be a second way of storing
/// text (ADR 0002). Nothing is journalled - the session's edits are one change
/// to the person, and the version's creation already made its line.
pub fn edit(conn: &Connection, id: &str, body: &str) -> Result<Version> {
    gesture(conn, "version.edit", |act| {
        let before = version::get(act, id)?
            .map(|found| found.body)
            .unwrap_or_default();
        act.param("id", id);
        act.param("body", body);
        act.param("before", before);
        act.stamped();
        version::update_body_at(act, id, body, act.at())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    fn style(conn: &Connection, work_id: &str, body: &str) -> Version {
        create(
            conn,
            work_id,
            NewVersion {
                role: "style".into(),
                body: body.into(),
                ..NewVersion::default()
            },
        )
        .unwrap()
    }

    fn main_of(conn: &Connection, work_id: &str) -> Option<String> {
        version::list(conn, work_id)
            .unwrap()
            .into_iter()
            .find(|summary| summary.is_main)
            .map(|summary| summary.id)
    }

    fn undo_last(conn: &Connection) {
        let offer = crate::undo::last(conn).unwrap().unwrap();
        crate::undo::undo(conn, &offer.operation_id).unwrap();
    }

    /// An undo takes back one choice: the main version goes back to the one
    /// marked before it.
    #[test]
    fn an_undo_puts_back_the_main_version_before_it() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let sung = style(&conn, &song.id, "the one we sing");
        let tried = style(&conn, &song.id, "a second try");
        set_main(&conn, &song.id, &sung.id).unwrap();
        set_main(&conn, &song.id, &tried.id).unwrap();

        undo_last(&conn);

        assert_eq!(main_of(&conn, &song.id), Some(sung.id));
    }

    /// The first choice taken back leaves the role with none: it reads its
    /// newest again.
    #[test]
    fn an_undo_of_the_first_choice_leaves_no_main_version() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let sung = style(&conn, &song.id, "the one we sing");
        style(&conn, &song.id, "a second try");
        set_main(&conn, &song.id, &sung.id).unwrap();

        undo_last(&conn);

        assert_eq!(main_of(&conn, &song.id), None);
    }

    /// A main version thrown away and brought back after another was marked
    /// comes back without the mark: the choice taken since stands.
    #[test]
    fn a_restored_main_version_does_not_overrule_the_choice_taken_since() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let first = style(&conn, &song.id, "one");
        let second = style(&conn, &song.id, "two");
        set_main(&conn, &song.id, &first.id).unwrap();
        let entry = crate::actions::trash::discard(&conn, crate::trash::Entity::Version, &first.id)
            .unwrap();
        set_main(&conn, &song.id, &second.id).unwrap();

        crate::actions::trash::restore(&conn, &entry).unwrap();

        assert!(version::get(&conn, &first.id).unwrap().is_some());
        assert_eq!(main_of(&conn, &song.id), Some(second.id));
    }

    /// Restored with nothing chosen since, it is main again.
    #[test]
    fn a_restored_main_version_is_main_again_when_nothing_was_chosen() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let first = style(&conn, &song.id, "one");
        style(&conn, &song.id, "two");
        set_main(&conn, &song.id, &first.id).unwrap();
        let entry = crate::actions::trash::discard(&conn, crate::trash::Entity::Version, &first.id)
            .unwrap();
        assert_eq!(main_of(&conn, &song.id), None);

        crate::actions::trash::restore(&conn, &entry).unwrap();

        assert_eq!(main_of(&conn, &song.id), Some(first.id));
    }
}
