//! What each work holds of its media, for the catalogue's filters a batch is
//! chosen by (v0.155, FR-21 21.3): ready for a video, no audio, no cover,
//! ready to go out.
//!
//! Asked apart from the catalogue and only while one of those filters is
//! on: a work's files are looked for in its folder on disk, and the
//! catalogue is read far more often than it is narrowed by media.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::error::Result;

/// A work's media as the catalogue reads it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct WorkMedia {
    /// A cover is chosen for it.
    pub cover: bool,
    /// Its kind goes out under a cover.
    pub wants_cover: bool,
    /// It holds a track: a file attached to it or in its folder that plays.
    pub audio: bool,
    /// It holds a video, attached or in its folder.
    pub video: bool,
}

const AUDIO: [&str; 7] = ["mp3", "wav", "flac", "m4a", "ogg", "aac", "opus"];
const VIDEOS: [&str; 5] = ["mp4", "mov", "webm", "mkv", "avi"];

/// The extension of a path or a name, lower case.
fn extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase()
}

/// Every work of a profile, by id, with what it holds: what is attached to
/// it, its chosen cover, and the files of its folder on disk.
pub fn of_profile(conn: &Connection, profile_id: &str) -> Result<BTreeMap<String, WorkMedia>> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let covered: HashSet<String> = crate::asset::covers_of(conn, profile_id)?
        .into_iter()
        .map(|(work_id, _)| work_id)
        .collect();
    let mut held: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut statement = conn.prepare(
        "SELECT work_id, coalesce(original_name, path) FROM asset
          WHERE profile_id = ?1 AND work_id IS NOT NULL",
    )?;
    let attached = statement.query_map(params![profile_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for one in attached {
        let (work_id, name) = one?;
        held.entry(work_id).or_default().push(extension(&name));
    }

    let mut media = BTreeMap::new();
    for work in super::list(conn, profile_id, &super::WorkFilter::default())? {
        let mut kinds = held.remove(&work.id).unwrap_or_default();
        // What lies in the work's folder, the finished file among it: a
        // track's file is its video, and its audio is a file beside it that
        // nothing points at.
        let folder = crate::folder::of_work(conn, &config, &work)?;
        kinds.extend(folder.files.iter().map(|file| extension(&file.relative)));
        if let Some(path) = folder.file.path {
            kinds.push(extension(&path));
        }
        media.insert(
            work.id.clone(),
            WorkMedia {
                cover: covered.contains(&work.id),
                wants_cover: config.vocabulary(&work.kind).cover,
                audio: kinds.iter().any(|ext| AUDIO.contains(&ext.as_str())),
                video: kinds.iter().any(|ext| VIDEOS.contains(&ext.as_str())),
            },
        );
    }
    Ok(media)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_read_by_its_extension_whatever_its_case() {
        assert_eq!(extension("C:/media/Dawn.MP3"), "mp3");
        assert_eq!(extension("cover"), "");
        assert!(AUDIO.contains(&extension("take.flac").as_str()));
        assert!(VIDEOS.contains(&extension("clip.webm").as_str()));
    }

    /// A track attached to a work makes it hold audio; a work with nothing
    /// attached and no folder holds nothing (v0.155).
    #[test]
    fn a_work_holds_what_is_attached_to_it() {
        let (conn, profile_id, media) = crate::fixtures::workspace_with_media();
        let with_track = crate::fixtures::song(&conn, &profile_id, "Harbour lights").id;
        let bare = crate::fixtures::song(&conn, &profile_id, "Empty pier").id;
        let outside = tempfile::tempdir().unwrap();
        let track = crate::fixtures::file(outside.path(), "harbour.mp3");
        crate::asset::attach(
            &conn,
            &profile_id,
            media.path(),
            &track,
            crate::asset::NewAsset {
                work_id: Some(with_track.clone()),
                ..crate::asset::NewAsset::default()
            },
        )
        .unwrap();

        let held = of_profile(&conn, &profile_id).unwrap();
        assert!(held[&with_track].audio);
        assert!(!held[&with_track].video);
        assert!(!held[&with_track].cover);
        assert_eq!(held[&bare], WorkMedia::default());
    }

    /// A track's file is its video; its audio lies beside it in the folder,
    /// attached to nothing - and holds it all the same.
    #[test]
    fn a_track_in_the_work_folder_is_its_audio() {
        let (conn, profile_id) = crate::fixtures::workspace();
        let root = tempfile::tempdir().unwrap();
        crate::folder::set_root(
            &conn,
            &profile_id,
            Some(&root.path().to_string_lossy()),
            "2030-01-01T00:00:00.000Z",
        )
        .unwrap();
        let work = crate::fixtures::song(&conn, &profile_id, "Harbour lights");
        let config = crate::profile::config_for(&conn, &profile_id).unwrap();
        let folder = crate::folder::ensure(&conn, &config, &work).unwrap();
        let path = std::path::PathBuf::from(folder.path.unwrap());
        std::fs::write(path.join("harbour.MP3"), b"ID3").unwrap();

        let held = of_profile(&conn, &profile_id).unwrap();
        assert!(held[&work.id].audio);
        assert!(!held[&work.id].video);
    }
}
