//! What tests start from: a workspace with the shipped profile, and the rows
//! most of them need first.
//!
//! Every module used to carry its own `workspace()` and `a_work()` - thirty-five
//! copies of the same six lines, drifting one field at a time. They live here
//! now, compiled into the library rather than behind `cfg(test)` so the
//! integration tests under `tests/` can reach them too. Nothing in the
//! application calls this module.
//!
//! The rows are written through the domain, not through the actions: a fixture
//! is the ground a test stands on, not a gesture the test is about, and it
//! leaves the operations log empty for the test to read.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde_json::Value;

use crate::release::{self, NewRelease, Release};
use crate::score::{self, NewScore, Score};
use crate::work::version::{self, NewVersion, Version};
use crate::work::{self, NewWork, Work};
use crate::{db, profile};

/// An empty workspace in memory, with the shipped profile active, and that
/// profile's id.
pub fn workspace() -> (Connection, String) {
    let conn = db::open_in_memory().expect("an in-memory workspace opens");
    profile::seed(&conn).expect("the shipped profiles seed");
    let profile_id = profile::active(&conn)
        .expect("the active profile reads")
        .expect("seeding activates a profile")
        .id;
    (conn, profile_id)
}

/// [`workspace`] and a directory for the files a test attaches.
pub fn workspace_with_media() -> (Connection, String, tempfile::TempDir) {
    let (conn, profile_id) = workspace();
    let media = tempfile::tempdir().expect("a temporary directory");
    (conn, profile_id, media)
}

/// A workspace on disk, with the shipped profile active - for a gesture that
/// finds its files beside the database, as the window's workspace has them.
pub fn workspace_on_disk() -> (Connection, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let conn = db::open(&dir.path().join("kilna.db")).expect("a workspace on disk opens");
    profile::seed(&conn).expect("the shipped profiles seed");
    let profile_id = profile::active(&conn)
        .expect("the active profile reads")
        .expect("seeding activates a profile")
        .id;
    (conn, profile_id, dir)
}

/// A work of `kind` titled `title`.
pub fn work(conn: &Connection, profile_id: &str, kind: &str, title: &str) -> Work {
    work::create(
        conn,
        profile_id,
        NewWork {
            kind: kind.into(),
            title: title.into(),
            ..NewWork::default()
        },
    )
    .expect("the work is created")
}

/// A song titled `title`: the kind most tests are about.
pub fn song(conn: &Connection, profile_id: &str, title: &str) -> Work {
    work(conn, profile_id, "song", title)
}

/// A video titled `title`: a kind with a storyboard.
pub fn video(conn: &Connection, profile_id: &str, title: &str) -> Work {
    work(conn, profile_id, "video", title)
}

/// A version of a work in `role`, holding `body`.
pub fn version(conn: &Connection, work_id: &str, role: &str, body: &str) -> Version {
    version::create(
        conn,
        work_id,
        NewVersion {
            role: role.into(),
            body: body.into(),
            ..NewVersion::default()
        },
    )
    .expect("the version is created")
}

/// A score of a work: `axes` is an object of axis key to value.
pub fn score(conn: &Connection, work_id: &str, axes: Value) -> Score {
    score::create(
        conn,
        work_id,
        NewScore {
            axes: axes.as_object().cloned().expect("axes are an object"),
            version_id: None,
            note: None,
            rater: None,
        },
    )
    .expect("the score is created")
}

/// A release of a work, planned for `date` or queued without one.
pub fn release(conn: &Connection, work_id: &str, kind: &str, date: Option<&str>) -> Release {
    release::create(
        conn,
        NewRelease {
            work_id: work_id.into(),
            kind: kind.into(),
            scheduled_at: date.map(str::to_owned),
            meta: None,
            scheduled_time: None,
            time_zone: None,
        },
    )
    .expect("the release is created")
}

/// A file with a few bytes in it, to attach.
pub fn file(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, b"not really a picture").expect("the file is written");
    path
}

/// A card of the canon: a note of `kind`, which the shipped profile names
/// with sections, called `title`.
pub fn card(conn: &Connection, profile_id: &str, kind: &str, title: &str) -> crate::note::Note {
    crate::note::create(
        conn,
        profile_id,
        crate::note::NewNote {
            kind: Some(kind.into()),
            title: Some(title.into()),
            ..crate::note::NewNote::default()
        },
    )
    .expect("the card is created")
}

/// A fact of a card, in `section`, saying `body`, in the public layer.
pub fn fact(conn: &Connection, note_id: &str, section: &str, body: &str) -> crate::canon::Fact {
    crate::canon::fact::create_minted(
        conn,
        crate::canon::NewFact {
            note_id: note_id.into(),
            section: section.into(),
            body: body.into(),
            ..crate::canon::NewFact::default()
        },
        crate::minted::Minted::fresh(),
    )
    .expect("the fact is written")
}
