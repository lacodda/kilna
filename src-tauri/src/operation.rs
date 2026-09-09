//! The operations log: what the person asked for, in the order they asked.
//!
//! The schema already records what *became* of every row — migration 0011 put a
//! tombstone under each deletion and a clock on each edited field. What it
//! cannot record is the intent above them: three updates look the same whether
//! they were one gesture or three, and only the command layer knows which.
//!
//! Two rules run against the grain of [`crate::journal`], which this module
//! otherwise resembles, and both are deliberate:
//!
//! * **A failed write fails the caller.** The journal swallows its errors,
//!   because a line of history is worth less than the action it describes. An
//!   operation is worth more: a log with a hole in it replays to a database
//!   that never existed. [`record`] returns a `Result` and every caller carries
//!   it, inside the same transaction as the change wherever there is one.
//! * **Nothing is folded or swept.** No dedupe key, no retention. Two identical
//!   operations are two operations; the hundredth edit of one title is still
//!   part of how the database got here.

use rusqlite::{Connection, params};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::error::Result;
use crate::time::now;

/// One thing the person asked for, as the log stores it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    /// Position in this device's log. Ordering is `(recorded_at, seq)`.
    pub seq: i64,
    pub id: String,
    pub device_id: Option<String>,
    pub profile_id: Option<String>,
    /// What was asked, as a stable machine name: `work.create`.
    pub kind: String,
    /// Everything a replay needs — the input and what the code generated.
    pub params: Map<String, Value>,
    pub recorded_at: String,
}

/// An operation about to be written down.
///
/// Named for what it holds rather than for the act of writing it: the log stores
/// the person's intent. The distance from [`crate::journal::Record`] is also
/// deliberate — the two were briefly both called `Record`, and the journal's own
/// gate scanned this one's calls as if they were its keys.
#[derive(Debug, Clone)]
pub struct Intent {
    kind: String,
    params: Map<String, Value>,
    profile_id: Option<String>,
}

impl Intent {
    /// An operation of `kind`, acting on the workspace rather than a profile.
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            params: Map::new(),
            profile_id: None,
        }
    }

    /// The profile it acted in.
    pub fn in_profile(mut self, profile_id: impl Into<String>) -> Self {
        self.profile_id = Some(profile_id.into());
        self
    }

    /// A value the replay needs.
    ///
    /// Both halves belong here: what the person supplied, and what the code
    /// generated while carrying it out. A `work.create` that logs only the
    /// title replays into a different uuid, and every release, score and
    /// version pointing at the original parts ways with the row it names.
    pub fn param(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.params.insert(key.to_owned(), value.into());
        self
    }

    /// Several values at once, for an operation whose input is already a map.
    pub fn params(mut self, params: Map<String, Value>) -> Self {
        self.params.extend(params);
        self
    }

    /// The values carrying this operation out generated, so a replay can put
    /// them back. Always paired with a domain call taking the same [`Minted`].
    ///
    /// [`Minted`]: crate::minted::Minted
    pub fn minted(mut self, minted: &crate::minted::Minted) -> Self {
        minted.clone().into_params(&mut self.params);
        self
    }

    /// What was asked, as a stable machine name.
    pub fn kind(&self) -> &str {
        &self.kind
    }
}

/// Write an operation down.
///
/// Unlike [`crate::journal::record`], a failure here is the caller's problem:
/// see the module note. Call it inside the transaction that makes the change
/// whenever the change has one, so that a log entry and the rows it describes
/// arrive together or not at all.
pub fn record(conn: &Connection, entry: Intent) -> Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let device_id = crate::device::id(conn)?;

    conn.execute(
        "INSERT INTO operation (id, device_id, profile_id, kind, params, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            device_id,
            entry.profile_id,
            entry.kind,
            Value::Object(entry.params).to_string(),
            now(),
        ],
    )?;

    Ok(())
}

/// Every operation, oldest first — the order a replay reads them in.
pub fn all(conn: &Connection) -> Result<Vec<Operation>> {
    read(
        conn,
        "SELECT seq, id, device_id, profile_id, kind, params, recorded_at
                  FROM operation ORDER BY recorded_at, seq",
        params![],
    )
}

/// The most recent operations, newest first.
pub fn latest(conn: &Connection, limit: i64) -> Result<Vec<Operation>> {
    read(
        conn,
        "SELECT seq, id, device_id, profile_id, kind, params, recorded_at
           FROM operation ORDER BY recorded_at DESC, seq DESC LIMIT ?1",
        params![limit],
    )
}

/// One operation by its stable id.
pub fn by_id(conn: &Connection, id: &str) -> Result<Option<Operation>> {
    Ok(read(
        conn,
        "SELECT seq, id, device_id, profile_id, kind, params, recorded_at
           FROM operation WHERE id = ?1",
        params![id],
    )?
    .into_iter()
    .next())
}

/// How many operations the log holds.
pub fn count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT count(*) FROM operation", [], |row| row.get(0))?)
}

fn read(conn: &Connection, sql: &str, args: impl rusqlite::Params) -> Result<Vec<Operation>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map(args, |row| {
        let raw: String = row.get(5)?;
        Ok(Operation {
            seq: row.get(0)?,
            id: row.get(1)?,
            device_id: row.get(2)?,
            profile_id: row.get(3)?,
            kind: row.get(4)?,
            // A body that will not parse is shown as an empty one rather than
            // failing the read: the log is evidence, and evidence with one
            // unreadable line is still evidence.
            params: serde_json::from_str::<Value>(&raw)
                .ok()
                .and_then(|value| value.as_object().cloned())
                .unwrap_or_default(),
            recorded_at: row.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}
