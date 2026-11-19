//! A file taken into a work (v0.156, FR-21 21.1 p.4 and 21.2 p.1): what a
//! generator saved in Downloads, or what an assistant beside kilna made,
//! moved into the work's folder under a name of the work's and given the
//! role it was taken for.
//!
//! One way in for the armed receiver of the window and for `propose_media`
//! of the assistant: the file is checked against its role, the work's folder
//! is made when it is not there yet, the file is moved - never copied over
//! another, never left half-moved across drives - and then the role says
//! what it is to the work: its finished file, its cover, a take or a cover
//! to choose from, or a file that only lies in its folder.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::asset::{self, NewAsset};
use crate::error::{Error, Result};
use crate::folder::{self, FolderState};

/// What a file is taken into a work as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum MediaRole {
    /// A take of its track, kept beside the others to choose from.
    CandidateAudio,
    /// Its track: the work's finished file where the kind's rule takes it.
    FinalAudio,
    /// A picture for its cover, set beside the cover to choose from.
    CandidateCover,
    /// Its cover.
    FinalCover,
    /// Its video: the work's finished file where the kind's rule takes it.
    Video,
    /// A file of its, lying in its folder and nothing more.
    Asset,
}

const AUDIO: [&str; 7] = ["mp3", "wav", "flac", "m4a", "ogg", "aac", "opus"];
const PICTURES: [&str; 5] = ["png", "jpg", "jpeg", "webp", "avif"];
const VIDEOS: [&str; 5] = ["mp4", "mov", "webm", "mkv", "avi"];

impl MediaRole {
    /// The extensions a file of this role may have; none for any.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::CandidateAudio | Self::FinalAudio => &AUDIO,
            Self::CandidateCover | Self::FinalCover => &PICTURES,
            Self::Video => &VIDEOS,
            Self::Asset => &[],
        }
    }

    /// Whether a file of this name may be taken as this role.
    pub fn fits(self, name: &str) -> bool {
        let allowed = self.extensions();
        allowed.is_empty() || allowed.contains(&extension(name).as_str())
    }

    fn word(self) -> &'static str {
        match self {
            Self::CandidateAudio => "candidate_audio",
            Self::FinalAudio => "final_audio",
            Self::CandidateCover => "candidate_cover",
            Self::FinalCover => "final_cover",
            Self::Video => "video",
            Self::Asset => "asset",
        }
    }
}

/// Where a file went and what it became.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct TakenMedia {
    /// Where the file lies now, whole.
    pub path: String,
    /// Where it lies inside the work's folder.
    pub relative: String,
    /// It is the work's finished file now.
    pub finished: bool,
    /// The picture kept as its cover or a cover to choose from.
    pub asset_id: Option<String>,
}

/// The extension of a name, lower case.
fn extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase()
}

/// A work's title as a file name: what no system refuses, and short.
fn name_of(title: &str) -> String {
    let kept: String = title
        .chars()
        .map(|c| if folder::is_forbidden(c) { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let kept: String = kept.chars().take(80).collect();
    let kept = kept.trim_end_matches(['.', ' ']).to_owned();
    if kept.is_empty() {
        "work".to_owned()
    } else {
        kept
    }
}

/// The name a file is given inside the work's folder, before a free one is
/// looked for: the work's own for what it is finished as and its cover,
/// a numbered take for one to choose from, its own name otherwise.
fn named(role: MediaRole, title: &str, original: &str, rename: bool) -> String {
    let ext = extension(original);
    let dotted = if ext.is_empty() {
        String::new()
    } else {
        format!(".{ext}")
    };
    if !rename {
        return original.to_owned();
    }
    match role {
        MediaRole::FinalAudio | MediaRole::Video => format!("{}{dotted}", name_of(title)),
        MediaRole::FinalCover => format!("cover{dotted}"),
        MediaRole::CandidateAudio => format!("candidates/{} - take{dotted}", name_of(title)),
        MediaRole::CandidateCover => format!("candidates/cover{dotted}"),
        MediaRole::Asset => original.to_owned(),
    }
}

/// A place in `folder` for `relative` that holds nothing yet: `take.mp3`,
/// else `take (2).mp3`, and on.
fn free(folder: &Path, relative: &str) -> (PathBuf, String) {
    let place = |relative: &str| {
        relative
            .split('/')
            .fold(folder.to_path_buf(), |path, step| path.join(step))
    };
    if !place(relative).exists() {
        return (place(relative), relative.to_owned());
    }
    let (stem, dotted) = match relative.rfind('.') {
        Some(at) if at > relative.rfind('/').map_or(0, |slash| slash + 1) => {
            (&relative[..at], &relative[at..])
        }
        _ => (relative, ""),
    };
    (2..)
        .map(|n| format!("{stem} ({n}){dotted}"))
        .find(|candidate| !place(candidate).exists())
        .map(|candidate| (place(&candidate), candidate))
        .expect("some number is free")
}

/// Take the file at `source` into the work as `role` (v0.156). `rename`
/// gives it a name of the work's; without it the file keeps its own.
pub fn take(
    conn: &Connection,
    media: &Path,
    work_id: &str,
    source: &Path,
    role: MediaRole,
    rename: bool,
) -> Result<TakenMedia> {
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            Error::refused("media.notAFile").param("path", source.display().to_string())
        })?
        .to_owned();
    if !source.is_file() {
        return Err(Error::refused("media.notAFile").param("path", source.display().to_string()));
    }
    if !role.fits(&name) {
        return Err(Error::refused("media.wrongKind")
            .param("name", name.as_str())
            .param("role", role.word()));
    }
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let place = folder::ensure(conn, &config, &work)?;
    let at = match (place.state, place.path.as_deref()) {
        (FolderState::Found, Some(path)) => PathBuf::from(path),
        _ => return Err(Error::refused("media.noFolder").param("title", work.title.as_str())),
    };

    // A file already in the work's folder stays where it is.
    let inside = source
        .canonicalize()
        .ok()
        .zip(at.canonicalize().ok())
        .and_then(|(file, folder)| file.strip_prefix(folder).ok().map(Path::to_path_buf));
    let (path, relative) = match inside {
        Some(relative) => (
            source.to_path_buf(),
            relative.to_string_lossy().replace('\\', "/"),
        ),
        None => {
            let (path, relative) = free(&at, &named(role, &work.title, &name, rename));
            folder::move_file(source, &path).map_err(|cause| {
                Error::refused("media.moveFailed")
                    .param("name", name.as_str())
                    .param("cause", cause)
            })?;
            (path, relative)
        }
    };
    let whole = path.to_string_lossy().into_owned();

    let mut taken = TakenMedia {
        path: whole.clone(),
        relative: relative.clone(),
        finished: false,
        asset_id: None,
    };
    match role {
        // What it is finished as - where the kind's rule takes a file like
        // this, or names none. A track's rule takes its video: its audio lies
        // beside it and is not chosen over it.
        MediaRole::FinalAudio | MediaRole::Video => {
            let rule = config.vocabulary(&work.kind).file.clone();
            if rule
                .as_deref()
                .is_none_or(|rule| folder::rule_takes(rule, &relative))
            {
                super::plugin::keep_files(conn, work_id, std::slice::from_ref(&whole))?;
                taken.finished = true;
            }
        }
        MediaRole::FinalCover | MediaRole::CandidateCover => {
            let kind = if role == MediaRole::FinalCover {
                asset::COVER
            } else {
                asset::CANDIDATE
            };
            let attached = super::asset::attach(
                conn,
                media,
                &whole,
                NewAsset {
                    work_id: Some(work_id.to_owned()),
                    kind: Some(kind.to_owned()),
                    ..NewAsset::default()
                },
            )?;
            taken.asset_id = Some(attached.id);
        }
        MediaRole::CandidateAudio | MediaRole::Asset => {}
    }
    Ok(taken)
}

#[cfg(test)]
mod tests;
