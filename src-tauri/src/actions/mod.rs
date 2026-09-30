//! One function per gesture: the intent, the rows and the journal line, as one
//! unit of work.
//!
//! A gesture - rename a work, plan a release, keep a proposal - has a
//! ceremony: find the profile it acts in, read what the change replaces, pick
//! the moment, write the rows, record the operation that asked for them, say
//! it in the journal, bring the work's status in line. Until v0.83 that
//! ceremony was written out in every Tauri command, sixty-nine times, and
//! again twelve times in the code that applies an assistant's proposal - where
//! the copies had already drifted: a release planned by a package left no line
//! in the history. See ADR 0040.
//!
//! Here it is written once, in [`gesture`], and every action is a function
//! over it. The window's commands are adapters that take the connection and
//! call an action; a proposal applies by calling the same actions; a test
//! drives the workspace the way the window does by calling them too.
//!
//! An action takes a `&Connection` and opens a [unit](crate::db::unit): alone
//! it is its own transaction, and inside another action's unit it becomes
//! part of that one - which is how a whole package lands in one piece.

pub mod asset;
pub mod canon;
pub mod collection;
pub mod comment;
pub mod cut;
pub mod focus;
pub mod link;
pub mod note;
pub mod plugin;
pub mod profile;
pub mod proposal;
pub mod release;
pub mod scene;
pub mod score;
pub mod style;
pub mod trash;
pub mod version;
pub mod work;

use std::cell::{Cell, RefCell};

use rusqlite::Connection;
use serde_json::{Map, Value};

use crate::db::unit::atomically;
use crate::error::{Error, Result};
use crate::journal::{self, Record};
use crate::minted::Minted;
use crate::operation::{self, Intent};

/// A gesture under way: the connection it writes through, the profile it
/// acts in, and the operation it will record.
///
/// Derefs to the connection, so a domain function is handed `&act` exactly
/// as it would be handed a connection.
pub struct Act<'c> {
    conn: &'c Connection,
    profile_id: String,
    profile_key: String,
    kind: &'static str,
    params: RefCell<Map<String, Value>>,
    at: String,
    unchanged: Cell<bool>,
}

impl std::ops::Deref for Act<'_> {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        self.conn
    }
}

impl<'c> Act<'c> {
    /// The profile the gesture acts in.
    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    /// The gesture's one moment.
    ///
    /// Decided once, so the rows and the log agree on it: an edit replayed at
    /// a different instant leaves a different `updated_at`, and the rebuilt
    /// database stops matching (ADR 0014). An operation that needs it carries
    /// it as `at`, by saying so.
    pub fn at(&self) -> &str {
        &self.at
    }

    /// A value the replay or the undo needs.
    pub fn param(&self, key: &str, value: impl Into<Value>) {
        self.params
            .borrow_mut()
            .insert(key.to_owned(), value.into());
    }

    /// A value, serialised: the patch, the new row, the order.
    pub fn json(&self, key: &str, value: &impl serde::Serialize) -> Result<()> {
        self.param(key, serde_json::to_value(value)?);
        Ok(())
    }

    /// The gesture's moment, carried as `at`.
    pub fn stamped(&self) {
        self.param("at", self.at.clone());
    }

    /// A fresh id at the gesture's moment, not yet in the params.
    pub fn fresh(&self) -> Minted {
        Minted::of(uuid::Uuid::new_v4().to_string(), self.at.clone())
    }

    /// A fresh id at the gesture's moment, carried as `id` and `at` so a
    /// replay lands the row under it.
    pub fn mint(&self) -> Minted {
        let minted = self.fresh();
        let mut params = self.params.borrow_mut();
        minted.clone().into_params(&mut params);
        minted
    }

    /// What the fields a patch names held before it - recorded beside the
    /// patch so an undo has something to put back, and only those fields:
    /// see [`crate::reversal`].
    pub fn before<T: serde::Serialize, P: crate::reversal::Patch>(
        &self,
        before: Option<&T>,
        patch: &P,
    ) -> Result<()> {
        self.param("before", crate::reversal::before_of(before, patch)?);
        Ok(())
    }

    /// Nothing changed after all: record no operation.
    ///
    /// An undo entry for a change that did not happen is a step the person
    /// has to walk back past for nothing, and a replay would stamp rows a
    /// second run never touched.
    pub fn unchanged(&self) {
        self.unchanged.set(true);
    }

    /// A line in the history, inside the gesture: it lands with the change
    /// or not at all.
    pub fn journal(&self, record: Record) {
        journal::record(self.conn, &self.profile_id, record);
    }

    /// The title of a work as it stands now, for a journal line.
    pub fn title_of(&self, work_id: &str) -> String {
        journal::work_title(self.conn, work_id).unwrap_or_default()
    }

    /// Bring a work's status in line with the facts after this gesture
    /// changed one of them.
    pub fn restate(&self, work_id: &str) {
        restate(self.conn, &self.profile_id, work_id);
    }

    fn record_the_intent(self) -> Result<()> {
        if self.unchanged.get() {
            return Ok(());
        }
        let logged = Intent::new(self.kind)
            .in_profile(&self.profile_id)
            .param("profile", self.profile_key)
            .params(self.params.into_inner());
        operation::record(self.conn, logged)
    }
}

/// Carry out a gesture in the active profile: `work` writes the rows, and the
/// operation named `kind` is recorded beside them with the params `work`
/// gave it - all in one unit, or none of it.
///
/// The operation is recorded after `work` returns, so there is no way to
/// write through a gesture and forget the log: the only way to record nothing
/// is to say so, with [`Act::unchanged`].
pub fn gesture<T>(
    conn: &Connection,
    kind: &'static str,
    work: impl FnOnce(&Act<'_>) -> Result<T>,
) -> Result<T> {
    let profile_id = active_profile_id(conn)?;
    gesture_in(conn, &profile_id, kind, work)
}

/// [`gesture`] in a profile named by id rather than the active one - the
/// profile editor saves a profile that need not be open.
pub fn gesture_in<T>(
    conn: &Connection,
    profile_id: &str,
    kind: &'static str,
    work: impl FnOnce(&Act<'_>) -> Result<T>,
) -> Result<T> {
    atomically(conn, |conn| {
        let act = Act {
            conn,
            profile_id: profile_id.to_owned(),
            profile_key: profile_key(conn, profile_id)?,
            kind,
            params: RefCell::new(Map::new()),
            at: crate::time::now(),
            unchanged: Cell::new(false),
        };
        let done = work(&act)?;
        act.record_the_intent()?;
        Ok(done)
    })
}

/// The stable key of a profile, for the operations log.
///
/// Operations name profiles by key rather than by id: an id is minted per
/// workspace, so a log replayed into another copy would look for a profile that
/// is not there under that name. See ADR 0014.
pub fn profile_key(conn: &Connection, profile_id: &str) -> Result<String> {
    crate::profile::key_for_id(conn, profile_id)?
        .ok_or_else(|| Error::not_found("profile", profile_id))
}

/// The id of the active profile.
///
/// Every work and note gesture is scoped to it: the window never has to carry
/// the profile id around, and a request cannot land in the wrong profile.
pub fn active_profile_id(conn: &Connection) -> Result<String> {
    crate::profile::active(conn)?
        .map(|profile| profile.id)
        .ok_or_else(|| Error::refused("profile.noneActive"))
}

/// Bring a work's status back in line with the facts, and say so.
///
/// Called by every action that changes one of the facts a status is derived
/// from. Deriving here rather than inside each of those is the whole point:
/// the predecessor wrote the field from four places and it drifted apart from
/// what was true. A pinned work is left alone by [`crate::work::status::refresh`]
/// itself, so a call site never has to remember to check.
pub fn restate(conn: &Connection, profile_id: &str, work_id: &str) {
    let config = match crate::profile::config_for(conn, profile_id) {
        Ok(config) => config,
        Err(cause) => {
            crate::log::error("status", &format!("could not read the profile: {cause}"));
            return;
        }
    };

    match crate::work::status::refresh(conn, &config, work_id) {
        Ok(Some(change)) => journal::record(
            conn,
            profile_id,
            Record::new("work.restated")
                .param("title", change.title)
                .param("from", change.from)
                .param("to", change.to)
                .about("work", work_id.to_owned()),
        ),
        Ok(None) => {}
        Err(cause) => crate::log::error("status", &format!("could not restate the work: {cause}")),
    }
}

/// One item of a batch that was passed over, and why.
///
/// Carried out of the batch rather than counted: "two were skipped" is only
/// useful when a person can see which two, and for what.
#[derive(Debug, Clone, PartialEq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
    /// The work, release or other row the batch was given.
    pub id: String,
    /// What it is called, when it has a name worth reading.
    pub title: Option<String>,
    pub reason: crate::error::Reason,
}

/// What a batch did: how many it reached, and the rest one by one.
#[derive(Debug, Clone, PartialEq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct BulkOutcome {
    /// Items the change actually landed on.
    pub changed: usize,
    /// Items passed over. Not an error: in a batch this is the reason the
    /// number is smaller than the selection, and saying so is kinder than
    /// silence.
    pub skipped: Vec<Skipped>,
}

/// A count for a journal line.
pub(crate) fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    #[test]
    fn a_gesture_records_its_operation_with_the_profile_and_its_params() {
        let (conn, profile_id) = fixtures::workspace();
        let before = operation::count(&conn).unwrap();

        gesture(&conn, "finding.restore", |act| {
            act.param("key", "k");
            Ok(())
        })
        .unwrap();

        assert_eq!(operation::count(&conn).unwrap(), before + 1);
        let written = operation::latest(&conn, 1).unwrap().remove(0);
        assert_eq!(written.kind, "finding.restore");
        assert_eq!(written.profile_id.as_deref(), Some(profile_id.as_str()));
        assert_eq!(written.params["key"], "k");
        assert!(
            written.params.contains_key("profile"),
            "the profile travels by key"
        );
    }

    #[test]
    fn a_gesture_that_fails_records_nothing_and_writes_nothing() {
        let (conn, profile_id) = fixtures::workspace();
        let before = operation::count(&conn).unwrap();

        let failed = gesture(&conn, "work.create", |act| {
            crate::work::create_minted(
                act,
                &profile_id,
                crate::work::NewWork {
                    kind: "song".into(),
                    title: "Half made".into(),
                    ..Default::default()
                },
                act.mint(),
            )?;
            Err::<(), _>(Error::Internal("after the row".into()))
        });

        assert!(failed.is_err());
        assert_eq!(operation::count(&conn).unwrap(), before);
        assert!(
            crate::work::list(&conn, &profile_id, &Default::default())
                .unwrap()
                .is_empty(),
            "the row went with the gesture"
        );
    }

    #[test]
    fn an_unchanged_gesture_records_nothing() {
        let (conn, _) = fixtures::workspace();
        let before = operation::count(&conn).unwrap();

        gesture(&conn, "status.resync", |act| {
            act.unchanged();
            Ok(())
        })
        .unwrap();

        assert_eq!(operation::count(&conn).unwrap(), before);
    }
}
