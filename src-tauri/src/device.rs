//! Who this workspace is, for the day two of them meet.
//!
//! A merge between devices needs a tie-break that both sides compute the same
//! way, and wall clocks alone do not give one: two edits of one field a
//! millisecond apart on two machines have to settle in the same direction
//! everywhere. The device id is that tie-break. It is written once, when a
//! build that knows about it first opens the workspace, and never changes —
//! the triggers of migration 0011 read it on every deletion and every edit.

use rusqlite::{Connection, OptionalExtension};

use crate::error::Result;
use crate::time::now;

/// Make sure the workspace has an identity, and return it.
///
/// Called from [`crate::db::open`] after the migrations, so no update can run
/// before there is a device for its clock to name.
pub fn ensure(conn: &Connection) -> Result<String> {
    if let Some(id) = id(conn)? {
        return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO device (id, created_at) VALUES (?1, ?2)",
        rusqlite::params![id, now()],
    )?;
    Ok(id)
}

/// The workspace's identity, if it has one yet.
pub fn id(conn: &Connection) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT id FROM device", [], |row| row.get(0))
        .optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn opening_a_workspace_gives_it_one_identity() {
        let conn = db::open_in_memory().unwrap();

        let first = id(&conn).unwrap().expect("open assigns an id");
        assert_eq!(
            ensure(&conn).unwrap(),
            first,
            "ensure must not mint a second"
        );

        let rows: i64 = conn
            .query_row("SELECT count(*) FROM device", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn the_identity_survives_a_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = db::default_path(dir.path());

        let first = id(&db::open(&path).unwrap()).unwrap().unwrap();
        let again = id(&db::open(&path).unwrap()).unwrap().unwrap();

        assert_eq!(first, again);
    }

    #[test]
    fn two_workspaces_are_two_devices() {
        let one = id(&db::open_in_memory().unwrap()).unwrap().unwrap();
        let two = id(&db::open_in_memory().unwrap()).unwrap().unwrap();
        assert_ne!(one, two);
    }
}
