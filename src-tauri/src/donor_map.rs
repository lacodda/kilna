//! What a plugin heard in a donor's finished file (v0.103, ADR 0071): where
//! its parts begin and the parts worth a short.
//!
//! Kept with the file it was heard in - its path, size and the moment it was
//! last written - and read back only while the donor's finished file is still
//! that file as it stood. A new render is new sound, and what was heard in
//! the old one is not offered over it. A record of a file, not of the
//! workspace: written past the log, never merged or exported (see the
//! migration), rebuilt by listening again.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// A part of a donor offered for a short.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct HeardPart {
    pub starts_at: f64,
    pub ends_at: f64,
    /// Why it is offered: `chorus`, `loudest`, `repeats`, `lift` or `part`,
    /// or a key a newer plugin knows and the window shows as it is.
    pub reason: String,
    /// How strongly, from 0 to 1: the order the plugin offers them in.
    #[serde(default)]
    pub score: f64,
}

/// How a file stood when it was heard: its size and the moment it was last
/// written, to the nanosecond the disk keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    size: i64,
    modified: String,
}

impl Stamp {
    /// How the file at `path` stands now; `None` when it is not there.
    pub fn of(path: &Path) -> Option<Stamp> {
        let meta = std::fs::metadata(path).ok()?;
        let written = meta
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?;
        Some(Stamp {
            size: i64::try_from(meta.len()).ok()?,
            modified: format!("{}.{:09}", written.as_secs(), written.subsec_nanos()),
        })
    }
}

/// What was heard in a donor's file.
#[derive(Debug, Clone, PartialEq)]
pub struct Heard {
    pub file: String,
    pub boundaries: Vec<f64>,
    pub parts: Vec<HeardPart>,
    pub heard_by: String,
    pub heard_at: String,
}

/// Whether what was heard can be offered for the donor's file as it is now.
#[derive(Debug, Clone, PartialEq)]
pub enum Hearing {
    /// Nothing was heard in this donor.
    Unheard,
    /// Heard in the file it has now, as it stands.
    Heard(Heard),
    /// Heard in another file, or in this one before it was written again.
    Stale,
}

/// Keep what was heard in `file` for the donor `work_id`, over whatever was
/// heard in it before.
#[allow(clippy::too_many_arguments)]
pub fn keep_heard(
    conn: &Connection,
    profile_id: &str,
    work_id: &str,
    file: &str,
    stamp: &Stamp,
    boundaries: &[f64],
    parts: &[HeardPart],
    heard_by: &str,
    heard_at: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO donor_map \
             (work_id, profile_id, file, file_size, file_modified, boundaries, parts, heard_by, heard_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
         ON CONFLICT (work_id) DO UPDATE SET \
             file = excluded.file, file_size = excluded.file_size, \
             file_modified = excluded.file_modified, boundaries = excluded.boundaries, \
             parts = excluded.parts, heard_by = excluded.heard_by, heard_at = excluded.heard_at",
        params![
            work_id,
            profile_id,
            file,
            stamp.size,
            stamp.modified,
            serde_json::to_string(boundaries)?,
            serde_json::to_string(parts)?,
            heard_by,
            heard_at,
        ],
    )?;
    Ok(())
}

/// What was heard in the donor `work_id`, for its finished file `file` as it
/// stands now - `None` when it has none.
pub fn hearing(conn: &Connection, work_id: &str, file: Option<&str>) -> Result<Hearing> {
    let row = conn
        .query_row(
            "SELECT file, file_size, file_modified, boundaries, parts, heard_by, heard_at \
             FROM donor_map WHERE work_id = ?1",
            params![work_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    Stamp {
                        size: row.get(1)?,
                        modified: row.get(2)?,
                    },
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .optional()?;
    let Some((heard_file, stamp, boundaries, parts, heard_by, heard_at)) = row else {
        return Ok(Hearing::Unheard);
    };
    let same = file
        .is_some_and(|now| now == heard_file && Stamp::of(Path::new(now)).as_ref() == Some(&stamp));
    if !same {
        return Ok(Hearing::Stale);
    }
    Ok(Hearing::Heard(Heard {
        file: heard_file,
        // Written by this module from typed values; a row that does not read
        // is offered as heard with nothing in it rather than failing the tab.
        boundaries: serde_json::from_str(&boundaries).unwrap_or_default(),
        parts: serde_json::from_str(&parts).unwrap_or_default(),
        heard_by,
        heard_at,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_donor() -> (Connection, String, String) {
        let (conn, profile_id) = crate::fixtures::workspace();
        let work = crate::fixtures::work(&conn, &profile_id, "video", "Tide (video)");
        (conn, profile_id, work.id)
    }

    fn part(starts_at: f64, reason: &str) -> HeardPart {
        HeardPart {
            starts_at,
            ends_at: starts_at + 30.0,
            reason: reason.into(),
            score: 0.5,
        }
    }

    /// What was heard is offered while the file stands as it was heard, and
    /// stops being offered once it is written again or another file is the
    /// donor's.
    #[test]
    fn what_was_heard_is_offered_only_for_the_file_as_it_stood() {
        let (conn, profile_id, donor) = a_donor();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("v1.mp4");
        std::fs::write(&file, b"the first render").unwrap();
        let path = file.to_string_lossy().into_owned();
        assert_eq!(
            hearing(&conn, &donor, Some(&path)).unwrap(),
            Hearing::Unheard
        );

        let stamp = Stamp::of(&file).unwrap();
        keep_heard(
            &conn,
            &profile_id,
            &donor,
            &path,
            &stamp,
            &[0.0, 24.5, 55.0],
            &[part(24.5, "chorus")],
            "cut 0.2.0",
            "2026-10-27T10:00:00Z",
        )
        .unwrap();
        let Hearing::Heard(heard) = hearing(&conn, &donor, Some(&path)).unwrap() else {
            panic!("heard in this file");
        };
        assert_eq!(heard.boundaries, vec![0.0, 24.5, 55.0]);
        assert_eq!(heard.parts, vec![part(24.5, "chorus")]);

        assert_eq!(
            hearing(&conn, &donor, Some("elsewhere/v2.mp4")).unwrap(),
            Hearing::Stale,
            "another file is the donor's now"
        );
        assert_eq!(hearing(&conn, &donor, None).unwrap(), Hearing::Stale);
        std::fs::write(&file, b"the same name, rendered again and longer").unwrap();
        assert_eq!(
            hearing(&conn, &donor, Some(&path)).unwrap(),
            Hearing::Stale,
            "the file was written again"
        );
    }

    /// Hearing a donor again replaces what was heard before.
    #[test]
    fn hearing_again_replaces() {
        let (conn, profile_id, donor) = a_donor();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("v1.mp4");
        std::fs::write(&file, b"a render").unwrap();
        let path = file.to_string_lossy().into_owned();
        let stamp = Stamp::of(&file).unwrap();
        for (boundaries, reason) in [(vec![0.0, 10.0], "part"), (vec![0.0, 30.0], "chorus")] {
            keep_heard(
                &conn,
                &profile_id,
                &donor,
                &path,
                &stamp,
                &boundaries,
                &[part(30.0, reason)],
                "cut 0.2.0",
                "2026-10-27T10:00:00Z",
            )
            .unwrap();
        }

        let Hearing::Heard(heard) = hearing(&conn, &donor, Some(&path)).unwrap() else {
            panic!("heard");
        };
        assert_eq!(heard.boundaries, vec![0.0, 30.0]);
        assert_eq!(heard.parts[0].reason, "chorus");
    }
}
