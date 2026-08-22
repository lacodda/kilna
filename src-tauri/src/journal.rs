//! The journal: what happened, kept in a form that can still be translated.
//!
//! An entry is an action key and its values, never a finished sentence. The
//! frontend turns `work.created` plus `{"title": "Winter road"}` into a line of
//! text in whatever language the interface is in — including a language chosen
//! long after the entry was written.
//!
//! Two rules shape the rest:
//!
//! * **Writing never fails the caller.** Recording that something happened is
//!   not part of that something happening. [`record`] returns nothing and
//!   swallows its own errors: a full disk must not turn a successful deletion
//!   into a failed one.
//! * **Titles are copied, not referenced.** An entry outlives what it describes,
//!   so "Winter road deleted" has to keep saying that after the work is gone.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::Result;
use crate::time::now;

/// How loudly an entry asks to be noticed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    /// The ordinary record: something was created, edited, released.
    #[default]
    Info,
    /// Something the person should look at — a release that lost its slot, a
    /// plan that no longer holds. Only these are counted as unread.
    Warn,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warn => "warn",
        }
    }

    fn parse(raw: &str) -> Self {
        match raw {
            "warn" => Self::Warn,
            // An unknown level is shown rather than dropped: a line of history
            // is worth more than its severity.
            _ => Self::Info,
        }
    }
}

/// One thing that happened, as the feed shows it.
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub id: String,
    /// The i18n key naming what happened, e.g. `work.created`.
    pub action: String,
    /// Values the key interpolates.
    pub params: Map<String, Value>,
    pub level: Level,
    pub entity: Option<String>,
    pub entity_id: Option<String>,
    /// How many times it happened, counting the first.
    pub occurrences: i64,
    pub created_at: String,
    pub read_at: Option<String>,
}

/// Something about to be written down.
///
/// Built with [`Entry::of`] and the `with_*` methods rather than a struct
/// literal, because almost every call site sets two of the six fields.
#[derive(Debug, Clone)]
pub struct Record {
    action: String,
    params: Map<String, Value>,
    level: Level,
    entity: Option<String>,
    entity_id: Option<String>,
    dedupe_key: Option<String>,
    write_once: bool,
}

impl Record {
    /// An entry for `action`, at the ordinary level, about nothing in
    /// particular.
    pub fn new(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            params: Map::new(),
            level: Level::Info,
            entity: None,
            entity_id: None,
            dedupe_key: None,
            write_once: false,
        }
    }

    /// A value the sentence interpolates. Copied, not referenced.
    pub fn param(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.params.insert(key.to_owned(), value.into());
        self
    }

    /// What it happened to, so the thing's own card can show it.
    pub fn about(mut self, entity: &str, id: impl Into<String>) -> Self {
        self.entity = Some(entity.to_owned());
        self.entity_id = Some(id.into());
        self
    }

    /// Mark it as something the person should look at.
    pub fn warn(mut self) -> Self {
        self.level = Level::Warn;
        self
    }

    /// Collapse repeats into one line instead of filling the feed.
    ///
    /// The key has to name the *situation*, not the moment: `ready:<release id>`
    /// is the same warning however many times it is noticed.
    pub fn deduped(mut self, key: impl Into<String>) -> Self {
        self.dedupe_key = Some(key.into());
        self
    }

    /// Write only the first time this situation is seen; after that, nothing.
    ///
    /// Different from [`Record::deduped`] in what a repeat means. A deduped
    /// repeat is news — someone displaced the same release *again* — so it
    /// counts up and turns unread. A once-entry describes a standing state
    /// noticed by a sweep, and a sweep re-noticing it is not news: turning it
    /// unread on every startup would keep the bell lit until the situation is
    /// fixed, and a bell that is always lit stops meaning anything.
    pub fn once(mut self, key: impl Into<String>) -> Self {
        self.dedupe_key = Some(key.into());
        self.write_once = true;
        self
    }
}

/// Write an entry down, or quietly fail to.
///
/// Deliberately returns nothing. Journalling is a side effect of an action that
/// has already succeeded; letting it report failure would tempt every call site
/// into `?`, and a full disk would start turning completed work into errors.
/// The failure is printed, because a journal that silently stops recording is
/// worse than one that says so on the console.
pub fn record(conn: &Connection, profile_id: &str, entry: Record) {
    if let Err(cause) = write(conn, profile_id, entry) {
        eprintln!("journal: could not record what happened: {cause}");
    }
}

fn write(conn: &Connection, profile_id: &str, entry: Record) -> Result<()> {
    let timestamp = now();

    // A once-entry is written the first time and never touched again — not
    // even to count up, because its repeats are the same standing state being
    // re-noticed, not new occurrences.
    if entry.write_once {
        let key = entry.dedupe_key.as_ref().expect("once always sets the key");
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM journal WHERE profile_id = ?1 AND dedupe_key = ?2",
                params![profile_id, key],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if exists {
            return Ok(());
        }
    }

    // A deduped repeat moves the existing line to now and counts up, so the feed
    // says "three times, most recently just now" rather than repeating itself.
    if let Some(key) = &entry.dedupe_key {
        let updated = conn.execute(
            "UPDATE journal
                SET occurrences = occurrences + 1,
                    created_at = ?1,
                    params = ?2,
                    level = ?3,
                    read_at = NULL
              WHERE profile_id = ?4 AND dedupe_key = ?5",
            params![
                timestamp,
                Value::Object(entry.params.clone()).to_string(),
                entry.level.as_str(),
                profile_id,
                key,
            ],
        )?;
        // Unread again on purpose: a warning that came back is news, even if the
        // person had already dismissed the first one.
        if updated > 0 {
            return Ok(());
        }
    }

    conn.execute(
        "INSERT INTO journal
             (id, profile_id, action, params, level, entity, entity_id, dedupe_key, occurrences, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9)",
        params![
            uuid::Uuid::new_v4().to_string(),
            profile_id,
            entry.action,
            Value::Object(entry.params).to_string(),
            entry.level.as_str(),
            entry.entity,
            entry.entity_id,
            entry.dedupe_key,
            timestamp,
        ],
    )?;

    Ok(())
}

/// How many entries a feed page holds.
///
/// The feed is history, not a work surface: someone looking for something older
/// than this is looking for a search, which the journal does not pretend to be.
const PAGE: i64 = 200;

/// The profile's history, newest first.
pub fn list(conn: &Connection, profile_id: &str) -> Result<Vec<Entry>> {
    query(
        conn,
        "SELECT id, action, params, level, entity, entity_id, occurrences, created_at, read_at
           FROM journal
          WHERE profile_id = ?1
          ORDER BY created_at DESC
          LIMIT ?2",
        params![profile_id, PAGE],
    )
}

/// One thing's own history, newest first — the card's History tab.
pub fn for_entity(conn: &Connection, entity: &str, entity_id: &str) -> Result<Vec<Entry>> {
    query(
        conn,
        "SELECT id, action, params, level, entity, entity_id, occurrences, created_at, read_at
           FROM journal
          WHERE entity = ?1 AND entity_id = ?2
          ORDER BY created_at DESC
          LIMIT ?3",
        params![entity, entity_id, PAGE],
    )
}

/// How many entries are asking to be looked at.
///
/// Only warnings count. If every ordinary edit lit the bell, the bell would be
/// lit permanently and would stop meaning anything.
pub fn unread_count(conn: &Connection, profile_id: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM journal
          WHERE profile_id = ?1 AND read_at IS NULL AND level = 'warn'",
        params![profile_id],
        |row| row.get(0),
    )?)
}

/// Mark everything currently unread as seen.
pub fn mark_read(conn: &Connection, profile_id: &str) -> Result<usize> {
    Ok(conn.execute(
        "UPDATE journal SET read_at = ?1 WHERE profile_id = ?2 AND read_at IS NULL",
        params![now(), profile_id],
    )?)
}

/// Days a read entry is kept before it is swept.
pub const RETENTION_DAYS: i64 = 7;

/// Drop read entries older than [`RETENTION_DAYS`]. Returns how many went.
///
/// Unread entries are never swept, however old: the journal exists so that
/// nothing happens unnoticed, and quietly deleting the notice would be exactly
/// that. Run at startup rather than on a timer — a desktop app that is open for
/// a week can wait a week.
pub fn sweep(conn: &Connection, profile_id: &str) -> Result<usize> {
    // Both sides are put through `strftime` before they are compared. Stored
    // timestamps are RFC 3339 (`2026-08-17T09:00:00Z`) while `datetime()`
    // returns `2026-08-17 09:00:00`, and comparing those as plain strings is
    // wrong in the direction that hides itself: `T` sorts above a space, so
    // every entry would look newer than the cutoff and nothing would ever be
    // swept.
    Ok(conn.execute(
        &format!(
            "DELETE FROM journal
              WHERE profile_id = ?1
                AND read_at IS NOT NULL
                AND strftime('%Y-%m-%dT%H:%M:%SZ', created_at)
                  < strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-{RETENTION_DAYS} days')"
        ),
        params![profile_id],
    )?)
}

fn query(conn: &Connection, sql: &str, args: impl rusqlite::Params) -> Result<Vec<Entry>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement
        .query_map(args, |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(rows
        .into_iter()
        .map(
            |(id, action, params, level, entity, entity_id, occurrences, created_at, read_at)| {
                Entry {
                    id,
                    action,
                    // A malformed params blob costs the values, not the line.
                    params: serde_json::from_str(&params).unwrap_or_default(),
                    level: Level::parse(&level),
                    entity,
                    entity_id,
                    occurrences,
                    created_at,
                    read_at,
                }
            },
        )
        .collect())
}

/// Look up a work's title for an entry that will outlive it.
///
/// Returns `None` when the work is not there, which is not an error: an entry
/// about something already gone is written without a title rather than not
/// written at all.
pub fn work_title(conn: &Connection, work_id: &str) -> Option<String> {
    conn.query_row(
        "SELECT title FROM work WHERE id = ?1",
        params![work_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .ok()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    #[test]
    fn an_entry_keeps_its_key_and_values_rather_than_a_sentence() {
        let (conn, profile_id) = workspace();

        record(
            &conn,
            &profile_id,
            Record::new("work.created")
                .param("title", "Winter road")
                .about("work", "w1"),
        );

        let entries = list(&conn, &profile_id).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, "work.created");
        assert_eq!(entries[0].params["title"], "Winter road");
        assert_eq!(entries[0].entity.as_deref(), Some("work"));
        assert_eq!(entries[0].occurrences, 1);
        assert!(entries[0].read_at.is_none());
    }

    #[test]
    fn a_deduped_repeat_counts_up_instead_of_filling_the_feed() {
        let (conn, profile_id) = workspace();
        let warning = || {
            Record::new("release.notReady")
                .param("title", "Winter road")
                .deduped("ready:r1")
                .warn()
        };

        record(&conn, &profile_id, warning());
        record(&conn, &profile_id, warning());
        record(&conn, &profile_id, warning());

        let entries = list(&conn, &profile_id).unwrap();
        assert_eq!(entries.len(), 1, "one line, not three: {entries:?}");
        assert_eq!(entries[0].occurrences, 3);
    }

    #[test]
    fn a_warning_that_comes_back_is_unread_again() {
        let (conn, profile_id) = workspace();
        let warning = || Record::new("release.notReady").deduped("ready:r1").warn();

        record(&conn, &profile_id, warning());
        mark_read(&conn, &profile_id).unwrap();
        assert_eq!(unread_count(&conn, &profile_id).unwrap(), 0);

        record(&conn, &profile_id, warning());

        assert_eq!(
            unread_count(&conn, &profile_id).unwrap(),
            1,
            "a repeat of a dismissed warning is news again"
        );
    }

    /// A once-entry stays exactly as written: a sweep re-noticing the same
    /// standing state must not count up and must not wake a dismissed warning,
    /// or every startup would relight the bell until the situation is fixed.
    #[test]
    fn a_once_entry_is_written_once_and_left_alone() {
        let (conn, profile_id) = workspace();
        let warning = || {
            Record::new("release.notReady")
                .param("title", "Winter road")
                .once("ready:r1:2026-09-01")
                .warn()
        };

        record(&conn, &profile_id, warning());
        mark_read(&conn, &profile_id).unwrap();

        record(&conn, &profile_id, warning());
        record(&conn, &profile_id, warning());

        let entries = list(&conn, &profile_id).unwrap();
        assert_eq!(entries.len(), 1, "one line however often it is noticed");
        assert_eq!(entries[0].occurrences, 1, "repeats are not occurrences");
        assert_eq!(
            unread_count(&conn, &profile_id).unwrap(),
            0,
            "a re-noticed standing state is not news"
        );

        // A different key is a different situation: the same release moved to
        // another date warns afresh.
        record(
            &conn,
            &profile_id,
            Record::new("release.notReady")
                .once("ready:r1:2026-09-08")
                .warn(),
        );
        assert_eq!(list(&conn, &profile_id).unwrap().len(), 2);
        assert_eq!(unread_count(&conn, &profile_id).unwrap(), 1);
    }

    #[test]
    fn only_warnings_are_counted_unread() {
        let (conn, profile_id) = workspace();

        record(&conn, &profile_id, Record::new("work.created"));
        record(&conn, &profile_id, Record::new("work.updated"));
        record(&conn, &profile_id, Record::new("release.notReady").warn());

        assert_eq!(
            unread_count(&conn, &profile_id).unwrap(),
            1,
            "ordinary edits must not light the bell"
        );
        assert_eq!(list(&conn, &profile_id).unwrap().len(), 3, "but are kept");
    }

    #[test]
    fn a_things_own_history_is_only_its_own() {
        let (conn, profile_id) = workspace();

        record(
            &conn,
            &profile_id,
            Record::new("work.created").about("work", "w1"),
        );
        record(
            &conn,
            &profile_id,
            Record::new("work.created").about("work", "w2"),
        );
        record(&conn, &profile_id, Record::new("profile.switched"));

        let mine = for_entity(&conn, "work", "w1").unwrap();
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].entity_id.as_deref(), Some("w1"));
    }

    #[test]
    fn the_feed_is_scoped_to_its_profile() {
        let (conn, profile_id) = workspace();
        conn.execute(
            "INSERT INTO profile (id, key, name, config, is_active, is_builtin, created_at, updated_at)
             SELECT 'other', 'other', 'Other', config, 0, 0, created_at, updated_at FROM profile LIMIT 1",
            [],
        )
        .unwrap();

        record(&conn, &profile_id, Record::new("work.created"));
        record(&conn, "other", Record::new("work.created"));

        assert_eq!(list(&conn, &profile_id).unwrap().len(), 1);
    }

    #[test]
    fn the_same_dedupe_key_in_another_profile_is_a_different_situation() {
        let (conn, profile_id) = workspace();
        conn.execute(
            "INSERT INTO profile (id, key, name, config, is_active, is_builtin, created_at, updated_at)
             SELECT 'other', 'other', 'Other', config, 0, 0, created_at, updated_at FROM profile LIMIT 1",
            [],
        )
        .unwrap();

        record(&conn, &profile_id, Record::new("x").deduped("same"));
        record(&conn, "other", Record::new("x").deduped("same"));

        assert_eq!(list(&conn, &profile_id).unwrap()[0].occurrences, 1);
        assert_eq!(list(&conn, "other").unwrap()[0].occurrences, 1);
    }

    #[test]
    fn sweeping_takes_old_read_entries_and_spares_unread_ones() {
        let (conn, profile_id) = workspace();

        // Three entries, all old enough to sweep; only some of them read. The
        // timestamp is written in the format `record` uses — RFC 3339 with a `T`
        // and a `Z` — because a fixture in SQLite's own `datetime()` shape would
        // compare differently from every real row and prove nothing.
        for (id, level, read) in [
            ("old-read", "info", true),
            ("old-unread", "warn", false),
            ("old-read-warn", "warn", true),
        ] {
            conn.execute(
                "INSERT INTO journal (id, profile_id, action, params, level, occurrences, created_at, read_at)
                 VALUES (?1, ?2, 'x', '{}', ?3,
                         1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-30 days'), ?4)",
                params![id, profile_id, level, read.then(now)],
            )
            .unwrap();
        }
        // And one read entry that is still young.
        conn.execute(
            "INSERT INTO journal (id, profile_id, action, params, level, occurrences, created_at, read_at)
             VALUES ('fresh-read', ?1, 'x', '{}', 'info', 1, ?2, ?2)",
            params![profile_id, now()],
        )
        .unwrap();

        // One a minute past the cutoff — the case that decides whether the two
        // timestamp formats are actually being reconciled. Stored timestamps
        // carry `T` and `Z`; SQLite's own `datetime()` uses a space. Those only
        // differ from position ten onwards, so any entry from a different day is
        // classified correctly by either comparison, and a test using one would
        // pass over a broken cutoff. On the cutoff's own day the separator is
        // what is compared, `T` sorts above a space, and a naive cutoff decides
        // this entry is newer than it is and leaves it lying there.
        conn.execute(
            "INSERT INTO journal (id, profile_id, action, params, level, occurrences, created_at, read_at)
             VALUES ('just-over-read', ?1, 'x', '{}', 'info', 1,
                     strftime('%Y-%m-%dT%H:%M:%SZ', 'now', ?3, '-1 minutes'), ?2)",
            params![profile_id, now(), format!("-{RETENTION_DAYS} days")],
        )
        .unwrap();

        assert_eq!(sweep(&conn, &profile_id).unwrap(), 3);

        let left: Vec<String> = list(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .map(|entry| entry.id)
            .collect();
        assert!(
            left.contains(&"old-unread".to_owned()),
            "an unread entry is never swept, however old: {left:?}"
        );
        assert!(left.contains(&"fresh-read".to_owned()));
        assert_eq!(left.len(), 2);
    }

    #[test]
    fn recording_into_a_broken_journal_does_not_raise() {
        let (conn, profile_id) = workspace();
        conn.execute_batch("DROP TABLE journal").unwrap();

        // The point of the signature: this compiles without a `?` and the caller
        // has nothing to handle. A deletion must not fail because the note about
        // it could not be written.
        record(&conn, &profile_id, Record::new("work.created"));
    }

    #[test]
    fn a_title_is_read_while_the_work_is_still_there() {
        let (conn, profile_id) = workspace();
        let work = crate::work::create(
            &conn,
            &profile_id,
            crate::work::NewWork {
                kind: "song".into(),
                title: "Winter road".into(),
                status: None,
                collection_id: None,
                meta: None,
            },
        )
        .unwrap();

        assert_eq!(work_title(&conn, &work.id).as_deref(), Some("Winter road"));
        assert_eq!(
            work_title(&conn, "gone"),
            None,
            "and missing is not an error"
        );
    }
}
