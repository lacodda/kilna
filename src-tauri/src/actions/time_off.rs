//! Gestures on the calendar's closed days: closing a span, moving or renaming
//! it, opening it again.

use rusqlite::Connection;

use super::gesture;
use crate::error::{Error, Result};
use crate::journal::Record;
use crate::time_off::{self, NewTimeOff, TimeOff, TimeOffPatch};

/// Close a span of days to the auto-layout.
pub fn create(conn: &Connection, new: NewTimeOff) -> Result<TimeOff> {
    gesture(conn, "timeOff.create", |act| {
        act.json("timeOff", &new)?;
        let minted = act.mint();
        let created = time_off::create_minted(act, act.profile_id(), new, minted)?;
        act.param("title", created.label.clone());
        act.journal(
            Record::new("timeOff.created")
                .param("title", created.label.clone())
                .param("from", created.starts_on.clone())
                .param("to", created.ends_on.clone()),
        );
        Ok(created)
    })
}

/// Move a span's ends, or rename it.
pub fn update(conn: &Connection, id: &str, patch: TimeOffPatch) -> Result<TimeOff> {
    gesture(conn, "timeOff.update", |act| {
        let before = time_off::get(act, id)?.ok_or_else(|| Error::not_found("time off", id))?;
        act.param("id", id);
        act.param("title", before.label.clone());
        act.json("patch", &patch)?;
        act.before(Some(&before), &patch)?;
        act.stamped();
        let after = time_off::update_at(act, id, patch, act.at())?;
        if after == before {
            act.unchanged();
        } else {
            act.journal(
                Record::new("timeOff.changed")
                    .param("title", after.label.clone())
                    .param("from", after.starts_on.clone())
                    .param("to", after.ends_on.clone()),
            );
        }
        Ok(after)
    })
}

/// Open the days again. The row goes outright; the log keeps a copy for the
/// undo.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    gesture(conn, "timeOff.delete", |act| {
        let before = time_off::get(act, id)?.ok_or_else(|| Error::not_found("time off", id))?;
        act.param("id", id);
        act.param("title", before.label.clone());
        act.json("before", &before)?;
        time_off::delete(act, id)?;
        act.journal(
            Record::new("timeOff.deleted")
                .param("title", before.label.clone())
                .param("from", before.starts_on.clone())
                .param("to", before.ends_on.clone()),
        );
        Ok(())
    })
}
