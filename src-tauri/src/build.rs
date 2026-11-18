//! What a work's simple video is built from (v0.149, FR-15): the picture
//! that stands for it and the track that plays under it - the same rule the
//! simple-video plugin renders by, so the Build tab previews what the render
//! will make.

use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;

use crate::error::{Error, Result};

/// Files a picture can be.
const STILLS: [&str; 6] = ["jpg", "jpeg", "png", "webp", "bmp", "gif"];
/// Files a loop can be.
const LOOPS: [&str; 4] = ["mp4", "mov", "webm", "mkv"];
/// Files a track can be.
const TRACKS: [&str; 8] = ["wav", "flac", "mp3", "m4a", "aac", "ogg", "opus", "aiff"];

fn extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn is_track(path: &str) -> bool {
    TRACKS.contains(&extension(path).as_str())
}

/// The picture and the track of a work's simple video.
#[derive(Debug, Clone, Default, PartialEq, Serialize, ts_rs::TS)]
pub struct BuildSources {
    /// The work's cover, whole: what stands on the video.
    pub picture: Option<String>,
    /// The cover is a loop that plays over and over, not a still.
    pub looped: bool,
    /// The track, whole: the work's finished file when it is one, else the
    /// newest track attached to it, else the finished track of the first
    /// work it is made from.
    pub track: Option<String>,
    /// What the track is called: the name it arrived under, else its file's.
    pub track_name: Option<String>,
}

/// What `work_id`'s simple video is built from.
pub fn sources(conn: &Connection, work_id: &str) -> Result<BuildSources> {
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let mut out = BuildSources::default();
    if let Some(cover) = crate::asset::cover_of(conn, work_id)? {
        let ext = extension(&cover.path);
        if STILLS.contains(&ext.as_str()) || LOOPS.contains(&ext.as_str()) {
            out.looped = LOOPS.contains(&ext.as_str());
            out.picture = Some(cover.path);
        }
    }
    let finished = crate::folder::locate(conn, &config, &work)?.file.path;
    out.track = finished.filter(|path| is_track(path));
    if out.track.is_none()
        && let Some(asset) = crate::asset::for_work(conn, work_id)?
            .into_iter()
            .rev()
            .find(|asset| {
                is_track(&asset.path) || asset.original_name.as_deref().is_some_and(is_track)
            })
    {
        out.track_name = asset.original_name.clone().or(asset.label.clone());
        out.track = Some(asset.path);
    }
    if out.track.is_none() {
        for link in crate::link::sources(conn, work_id)? {
            let Some(donor) = crate::work::get(conn, &link.source_id)? else {
                continue;
            };
            if let Some(path) = crate::folder::locate(conn, &config, &donor)?
                .file
                .path
                .filter(|path| is_track(path))
            {
                out.track = Some(path);
                break;
            }
        }
    }
    if out.track_name.is_none() {
        out.track_name = out.track.as_deref().and_then(|path| {
            Path::new(path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    #[test]
    fn a_track_attached_to_the_work_and_its_cover_are_what_it_is_built_from() {
        let (conn, profile_id, dir) = fixtures::workspace_with_media();
        let clip = fixtures::video(&conn, &profile_id, "Harbour lights");
        assert_eq!(sources(&conn, &clip.id).unwrap(), BuildSources::default());

        let media = dir.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        for name in ["mix.wav", "notes.txt"] {
            let file = fixtures::file(dir.path(), name);
            crate::actions::asset::attach(
                &conn,
                &media,
                &file.to_string_lossy(),
                crate::asset::NewAsset {
                    work_id: Some(clip.id.clone()),
                    ..crate::asset::NewAsset::default()
                },
            )
            .unwrap();
        }
        fixtures::cover(&conn, &profile_id, &clip.id);

        let found = sources(&conn, &clip.id).unwrap();
        assert!(
            found
                .track
                .as_deref()
                .is_some_and(|path| path.ends_with(".wav")),
            "{found:?}"
        );
        assert!(found.picture.is_some(), "{found:?}");
        assert_eq!(
            found.track_name.as_deref(),
            Some("mix.wav"),
            "the name it arrived under"
        );
        assert!(!found.looped);
    }
}
