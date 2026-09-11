//! A work made from another: the link, its role, and the version it was
//! taken at.
//!
//! A video is made from a song, a video from an article, a video from three
//! sources or from none. The link says which work was made from which and in
//! what role — `donor` is the first role, and the table is one table for
//! every role to come (decision of 2026-09-11), so a remix or a sequel is a
//! value in `role`, not a schema. The link also remembers the source's
//! version at the moment it was made: the video's scenes were written against
//! that text, and when the song moves on, the card can say so and point at
//! the diff, rather than guess or stay silent. Nothing is copied over on
//! that account — the person decides. See ADR 0019.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::minted::Minted;

/// The first role a link can have: the work this one was made from.
pub const DONOR: &str = "donor";

/// A link as a card reads it: the source beside the link, and whether the
/// source has moved on since the link was made.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub id: String,
    /// The work that was made from the other.
    pub work_id: String,
    /// The work it was made from.
    pub source_id: String,
    pub role: String,
    /// The source's current version when the link was made; none when the
    /// source had none, or that version was deleted since.
    pub source_version_id: Option<String>,
    pub created_at: String,
    pub source_title: String,
    pub source_kind: String,
    pub source_status: String,
    /// The source's current version now.
    pub source_current_version_id: Option<String>,
    /// The revision the link was taken at, and the one the source is on now,
    /// for the card to say "taken at r2, now r4".
    pub taken_revision: Option<i64>,
    pub current_revision: Option<i64>,
    /// The source has moved on since the link was made: another version is
    /// current, or the very version it was taken at was edited in place
    /// (ADR 0015 lets a body change until something judges it). The fact,
    /// not a verdict — the card marks it and the person decides.
    pub drifted: bool,
}

/// A work made from this one, as its card lists them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Derived {
    pub link_id: String,
    pub work_id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub role: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewLink {
    pub work_id: String,
    pub source_id: String,
    /// `donor` when omitted.
    #[serde(default)]
    pub role: Option<String>,
    /// The source's current version when omitted.
    #[serde(default)]
    pub source_version_id: Option<String>,
}

/// Everything a card shows about a work's links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Links {
    /// What this work was made from.
    pub sources: Vec<Link>,
    /// What was made from this work.
    pub derived: Vec<Derived>,
}

const SELECT_LINK: &str = "SELECT l.id, l.work_id, l.source_id, l.role, l.source_version_id, l.created_at, \
     s.title, s.kind, s.status, s.current_version_id, \
     (SELECT revision FROM work_version WHERE id = l.source_version_id), \
     (SELECT revision FROM work_version WHERE id = s.current_version_id), \
     (SELECT changed_at FROM field_clock \
        WHERE entity = 'work_version' AND entity_id = l.source_version_id AND field = 'body') \
     FROM work_link l JOIN work s ON s.id = l.source_id";

pub fn create(conn: &Connection, profile_id: &str, new: NewLink) -> Result<Link> {
    create_minted(conn, profile_id, new, Minted::fresh())
}

/// Make a link with the id and moment already decided — the seam a replay
/// comes back through, see ADR 0014.
///
/// Both works must be in the profile, and the version, when named, must be
/// the source's: a link taken at another work's version would be a fact
/// about nothing. Linking a work to itself is refused by the schema.
pub fn create_minted(
    conn: &Connection,
    profile_id: &str,
    new: NewLink,
    minted: Minted,
) -> Result<Link> {
    let role = new
        .role
        .map(|role| role.trim().to_owned())
        .filter(|role| !role.is_empty())
        .unwrap_or_else(|| DONOR.to_owned());

    let work = crate::work::get(conn, &new.work_id)?
        .ok_or_else(|| Error::not_found("work", &new.work_id))?;
    let source = crate::work::get(conn, &new.source_id)?
        .ok_or_else(|| Error::not_found("work", &new.source_id))?;
    if work.profile_id != profile_id || source.profile_id != profile_id {
        return Err(Error::Other(
            "a link joins two works of the same profile".into(),
        ));
    }
    if work.id == source.id {
        return Err(Error::Other("a work cannot be made from itself".into()));
    }

    let source_version_id = match new.source_version_id {
        Some(id) => {
            let belongs: bool = conn
                .query_row(
                    "SELECT 1 FROM work_version WHERE id = ?1 AND work_id = ?2",
                    params![id, source.id],
                    |_| Ok(true),
                )
                .optional()?
                .unwrap_or(false);
            if !belongs {
                return Err(Error::Other(format!(
                    "version `{id}` is not a version of “{}”",
                    source.title
                )));
            }
            Some(id)
        }
        None => source.current_version_id,
    };

    let id = minted.id().to_owned();
    conn.execute(
        "INSERT INTO work_link (id, profile_id, work_id, source_id, role, source_version_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            profile_id,
            work.id,
            source.id,
            role,
            source_version_id,
            minted.at()
        ],
    )
    .map_err(|cause| match cause {
        rusqlite::Error::SqliteFailure(error, _)
            if error.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Error::Other(format!(
                "“{}” is already made from “{}” as {role}",
                work.title, source.title
            ))
        }
        other => other.into(),
    })?;

    get(conn, &id)?.ok_or_else(|| Error::Other("the link vanished after insert".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Link>> {
    conn.query_row(
        &format!("{SELECT_LINK} WHERE l.id = ?1"),
        params![id],
        read_link,
    )
    .optional()
    .map_err(Into::into)
}

/// What a work was made from, oldest link first.
pub fn sources(conn: &Connection, work_id: &str) -> Result<Vec<Link>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT_LINK} WHERE l.work_id = ?1 ORDER BY l.created_at, l.rowid"
    ))?;
    let rows = statement
        .query_map(params![work_id], read_link)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// What was made from a work, newest first: the clip you just started is
/// the one you are looking for.
pub fn derived(conn: &Connection, source_id: &str) -> Result<Vec<Derived>> {
    let mut statement = conn.prepare(
        "SELECT l.id, w.id, w.title, w.kind, w.status, l.role, l.created_at
         FROM work_link l JOIN work w ON w.id = l.work_id
         WHERE l.source_id = ?1 ORDER BY l.created_at DESC, l.rowid DESC",
    )?;
    let rows = statement
        .query_map(params![source_id], |row| {
            Ok(Derived {
                link_id: row.get(0)?,
                work_id: row.get(1)?,
                title: row.get(2)?,
                kind: row.get(3)?,
                status: row.get(4)?,
                role: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Both directions at once, for the card.
pub fn for_work(conn: &Connection, work_id: &str) -> Result<Links> {
    Ok(Links {
        sources: sources(conn, work_id)?,
        derived: derived(conn, work_id)?,
    })
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    if conn.execute("DELETE FROM work_link WHERE id = ?1", params![id])? == 0 {
        return Err(Error::not_found("link", id));
    }
    Ok(())
}

fn read_link(row: &rusqlite::Row<'_>) -> rusqlite::Result<Link> {
    let source_version_id: Option<String> = row.get(4)?;
    let created_at: String = row.get(5)?;
    let source_current_version_id: Option<String> = row.get(9)?;
    let edited_at: Option<String> = row.get(12)?;
    // Moved on: another version is current, or the version taken was edited
    // after the link was made. A link taken when the source had no version
    // has nothing to have moved from, and reads as still.
    let drifted = source_version_id.is_some()
        && (source_current_version_id != source_version_id
            || edited_at
                .as_deref()
                .is_some_and(|at| at > created_at.as_str()));
    Ok(Link {
        id: row.get(0)?,
        work_id: row.get(1)?,
        source_id: row.get(2)?,
        role: row.get(3)?,
        source_version_id,
        created_at,
        source_title: row.get(6)?,
        source_kind: row.get(7)?,
        source_status: row.get(8)?,
        source_current_version_id,
        taken_revision: row.get(10)?,
        current_revision: row.get(11)?,
        drifted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::work::version::{self, NewVersion};
    use crate::work::{self, NewWork};

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    fn work(conn: &Connection, profile_id: &str, kind: &str, title: &str) -> String {
        work::create(
            conn,
            profile_id,
            NewWork {
                kind: kind.into(),
                title: title.into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id
    }

    fn lyrics(conn: &mut Connection, work_id: &str, body: &str) -> String {
        version::create(
            conn,
            work_id,
            NewVersion {
                role: "lyrics".into(),
                body: body.into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn a_link_remembers_the_sources_version_and_reads_the_source_beside_it() {
        let (mut conn, profile_id) = workspace();
        let song = work(&conn, &profile_id, "song", "Harbour lights");
        let v1 = lyrics(&mut conn, &song, "one line");
        let video = work(&conn, &profile_id, "video", "Harbour lights");

        let link = create(
            &conn,
            &profile_id,
            NewLink {
                work_id: video.clone(),
                source_id: song.clone(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();

        assert_eq!(link.role, DONOR);
        assert_eq!(link.source_version_id.as_deref(), Some(v1.as_str()));
        assert_eq!(link.source_title, "Harbour lights");
        assert_eq!(link.source_kind, "song");
        assert_eq!(link.taken_revision, Some(1));
        assert!(!link.drifted, "nothing has moved yet");

        let links = for_work(&conn, &video).unwrap();
        assert_eq!(links.sources.len(), 1);
        assert!(links.derived.is_empty());
        let from_song = for_work(&conn, &song).unwrap();
        assert_eq!(from_song.derived.len(), 1);
        assert_eq!(from_song.derived[0].work_id, video);
        assert_eq!(from_song.derived[0].kind, "video");
    }

    #[test]
    fn the_link_drifts_when_the_source_moves_on_to_another_version() {
        let (mut conn, profile_id) = workspace();
        let song = work(&conn, &profile_id, "song", "S");
        lyrics(&mut conn, &song, "one line");
        let video = work(&conn, &profile_id, "video", "V");
        let link = create(
            &conn,
            &profile_id,
            NewLink {
                work_id: video,
                source_id: song.clone(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();

        lyrics(&mut conn, &song, "one line\ntwo lines");

        let read = get(&conn, &link.id).unwrap().unwrap();
        assert!(read.drifted);
        assert_eq!(read.taken_revision, Some(1));
        assert_eq!(read.current_revision, Some(2));
    }

    #[test]
    fn the_link_drifts_when_the_version_it_was_taken_at_is_edited_in_place() {
        let (mut conn, profile_id) = workspace();
        let song = work(&conn, &profile_id, "song", "S");
        let v1 = lyrics(&mut conn, &song, "one line");
        let video = work(&conn, &profile_id, "video", "V");
        let link = create(
            &conn,
            &profile_id,
            NewLink {
                work_id: video,
                source_id: song,
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();
        // The clock has millisecond resolution; the edit must land after
        // the link's moment, not within the same millisecond.
        std::thread::sleep(std::time::Duration::from_millis(5));

        conn.execute(
            "UPDATE work_version SET body = 'one line, changed' WHERE id = ?1",
            params![v1],
        )
        .unwrap();

        let read = get(&conn, &link.id).unwrap().unwrap();
        assert!(read.drifted, "an in-place edit is a change too");
        assert_eq!(read.current_revision, Some(1), "still the same revision");
    }

    #[test]
    fn a_link_taken_when_the_source_had_no_version_never_drifts() {
        let (mut conn, profile_id) = workspace();
        let song = work(&conn, &profile_id, "song", "S");
        let video = work(&conn, &profile_id, "video", "V");
        let link = create(
            &conn,
            &profile_id,
            NewLink {
                work_id: video,
                source_id: song.clone(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();
        assert!(link.source_version_id.is_none());

        lyrics(&mut conn, &song, "later");

        assert!(!get(&conn, &link.id).unwrap().unwrap().drifted);
    }

    #[test]
    fn a_work_is_not_made_from_itself_nor_twice_from_the_same_source() {
        let (conn, profile_id) = workspace();
        let song = work(&conn, &profile_id, "song", "S");
        let video = work(&conn, &profile_id, "video", "V");

        let err = create(
            &conn,
            &profile_id,
            NewLink {
                work_id: song.clone(),
                source_id: song.clone(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("itself"), "{err}");

        let new = NewLink {
            work_id: video,
            source_id: song,
            role: None,
            source_version_id: None,
        };
        create(&conn, &profile_id, new.clone()).unwrap();
        let err = create(&conn, &profile_id, new).unwrap_err();
        assert!(err.to_string().contains("already made from"), "{err}");
    }

    #[test]
    fn a_version_of_another_work_is_refused() {
        let (mut conn, profile_id) = workspace();
        let song = work(&conn, &profile_id, "song", "S");
        let other = work(&conn, &profile_id, "song", "O");
        let foreign = lyrics(&mut conn, &other, "x");
        let video = work(&conn, &profile_id, "video", "V");

        let err = create(
            &conn,
            &profile_id,
            NewLink {
                work_id: video,
                source_id: song,
                role: None,
                source_version_id: Some(foreign),
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("not a version of"), "{err}");
    }

    #[test]
    fn deleting_the_source_takes_the_link_and_leaves_a_tombstone() {
        let (conn, profile_id) = workspace();
        let song = work(&conn, &profile_id, "song", "S");
        let video = work(&conn, &profile_id, "video", "V");
        let link = create(
            &conn,
            &profile_id,
            NewLink {
                work_id: video.clone(),
                source_id: song.clone(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();

        conn.execute("DELETE FROM work WHERE id = ?1", params![song])
            .unwrap();

        assert!(get(&conn, &link.id).unwrap().is_none());
        assert!(for_work(&conn, &video).unwrap().sources.is_empty());
        let stones: i64 = conn
            .query_row(
                "SELECT count(*) FROM tombstone WHERE entity = 'work_link' AND entity_id = ?1",
                params![link.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stones, 1);
    }
}
