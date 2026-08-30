use rusqlite::Connection;

use crate::error::{Error, Result};

/// One versioned schema step. Migrations are embedded in the binary so that a
/// workspace can always be upgraded by the build that opens it.
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

/// Ordered by version; never edited once released, only appended to.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "core_schema",
        sql: include_str!("../../migrations/0001_core_schema.sql"),
    },
    Migration {
        version: 2,
        name: "chat",
        sql: include_str!("../../migrations/0002_chat.sql"),
    },
    Migration {
        version: 3,
        name: "trash",
        sql: include_str!("../../migrations/0003_trash.sql"),
    },
    Migration {
        version: 4,
        name: "journal",
        sql: include_str!("../../migrations/0004_journal.sql"),
    },
    Migration {
        version: 5,
        name: "status_pin",
        sql: include_str!("../../migrations/0005_status_pin.sql"),
    },
    Migration {
        version: 6,
        name: "slot_pin",
        sql: include_str!("../../migrations/0006_slot_pin.sql"),
    },
    Migration {
        version: 7,
        name: "chat_run",
        sql: include_str!("../../migrations/0007_chat_run.sql"),
    },
    Migration {
        version: 8,
        name: "chat_waiting",
        sql: include_str!("../../migrations/0008_chat_waiting.sql"),
    },
    Migration {
        version: 9,
        name: "focus_board",
        sql: include_str!("../../migrations/0009_focus_board.sql"),
    },
    Migration {
        version: 10,
        name: "work_tags_and_marks",
        sql: include_str!("../../migrations/0010_work_tags_and_marks.sql"),
    },
];

/// The newest schema this build understands.
pub fn latest_version() -> i64 {
    MIGRATIONS.last().map_or(0, |m| m.version)
}

/// Schema version currently stored in the database.
pub fn current_version(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

/// Apply every migration the database has not seen yet.
///
/// Each step runs in its own transaction, so a failure leaves the database on
/// the last version that fully applied rather than half-way through one.
pub fn apply(conn: &mut Connection) -> Result<i64> {
    let mut version = current_version(conn)?;
    let latest = latest_version();

    if version > latest {
        return Err(Error::SchemaTooNew {
            found: version,
            supported: latest,
        });
    }

    let from = version;
    for migration in MIGRATIONS.iter().filter(|m| m.version > from) {
        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)?;
        // PRAGMA does not accept bound parameters.
        tx.pragma_update(None, "user_version", migration.version)?;
        tx.commit()?;
        version = migration.version;
    }

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_sequential_and_start_at_one() {
        for (index, migration) in MIGRATIONS.iter().enumerate() {
            assert_eq!(
                migration.version,
                index as i64 + 1,
                "migration `{}` breaks the sequence",
                migration.name
            );
        }
    }

    #[test]
    fn apply_brings_an_empty_database_to_the_latest_version() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(current_version(&conn).unwrap(), 0);

        let version = apply(&mut conn).unwrap();

        assert_eq!(version, latest_version());
        assert_eq!(current_version(&conn).unwrap(), latest_version());
    }

    #[test]
    fn apply_is_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply(&mut conn).unwrap();
        // A second run must not attempt to recreate existing tables.
        assert_eq!(apply(&mut conn).unwrap(), latest_version());
    }

    #[test]
    fn a_database_at_an_older_version_is_carried_forward() {
        let mut conn = Connection::open_in_memory().unwrap();
        // Apply only the first migration, as an older build would have left it.
        let first = &MIGRATIONS[0];
        conn.execute_batch(first.sql).unwrap();
        conn.pragma_update(None, "user_version", first.version)
            .unwrap();

        let version = apply(&mut conn).unwrap();

        assert_eq!(version, latest_version());
        let chat_exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'chat'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(chat_exists, 1, "the later migration must have run");
    }

    #[test]
    fn a_newer_schema_is_refused_rather_than_downgraded() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", latest_version() + 1)
            .unwrap();

        let error = apply(&mut conn).unwrap_err();

        assert!(matches!(error, Error::SchemaTooNew { .. }));
    }
}
