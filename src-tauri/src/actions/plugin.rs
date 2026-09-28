//! What a plugin is given, and the gesture that keeps what it returns.
//!
//! Running a plugin is three steps with the workspace let go in the middle:
//! read the subject, run the executable - which may take as long as it likes -
//! and merge what it returned. The command holds the lock for the first and
//! the last and not for the wait; the two halves live here.

use rusqlite::Connection;
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::plugin::{self, manifest::Target};
use crate::release::{self, ReleasePatch};
use crate::work::{self, WorkPatch, version};

/// The row a plugin is run against, as it is sent.
///
/// A plugin acting on a work almost always wants its text, so a work travels
/// with the newest body of each role: sending only the row would make every
/// plugin ask for the body back.
pub fn subject(conn: &Connection, target: Target, id: &str) -> Result<Value> {
    match target {
        Target::Release => Ok(serde_json::to_value(
            release::get(conn, id)?.ok_or_else(|| Error::not_found("release", id))?,
        )?),
        Target::Work => {
            let found = work::get(conn, id)?.ok_or_else(|| Error::not_found("work", id))?;
            let mut value = serde_json::to_value(&found)?;
            let mut bodies = Map::new();
            for summary in version::list(conn, id)? {
                if bodies.contains_key(&summary.role) {
                    continue;
                }
                if let Some(full) = version::get(conn, &summary.id)? {
                    bodies.insert(summary.role.clone(), Value::String(full.body));
                }
            }
            if let Some(object) = value.as_object_mut() {
                object.insert("bodies".into(), Value::Object(bodies));
            }
            Ok(value)
        }
    }
}

/// Keep what a plugin returned: merged into the fields as they are NOW.
///
/// Read after the plugin returned rather than taken from what it was sent: a
/// field edited while it ran would otherwise be put back to what it was when
/// it started. Written as the ordinary edit it is - through the log, with what
/// it replaced - so undo takes back what the plugin wrote. It used to write
/// past the log, and Ctrl+Z after a plugin took back the person's previous
/// edit instead.
///
/// A plugin can add and overwrite its own keys but cannot clear the rest —
/// losing unrelated metadata to a third-party integration is not recoverable.
pub fn keep(
    conn: &Connection,
    target: Target,
    id: &str,
    returned: &Map<String, Value>,
) -> Result<()> {
    if returned.is_empty() {
        return Ok(());
    }
    match target {
        Target::Release => {
            let before = release::get(conn, id)?.ok_or_else(|| Error::not_found("release", id))?;
            super::release::update(
                conn,
                id,
                ReleasePatch {
                    meta: Some(plugin::merge_meta(&before.meta, returned)),
                    ..ReleasePatch::default()
                },
            )?;
        }
        Target::Work => {
            // Only the plugin's own keys: a work's fields merge by key, so the
            // rest cannot be cleared and the undo takes back exactly what the
            // plugin wrote.
            super::work::update(
                conn,
                id,
                WorkPatch {
                    meta: Some(returned.clone()),
                    ..WorkPatch::default()
                },
            )?;
        }
    }
    Ok(())
}
