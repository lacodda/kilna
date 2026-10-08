//! A work's folder on disk: where a person keeps what they make for a work
//! outside kilna - the renders, the takes, the stills a generator gave back.
//!
//! The opposite arrangement to an asset, and on purpose (ADR 0057). An asset
//! is *copied into* the workspace (ADR 0027): kilna owns the copy, a backup
//! carries it, and the original can be tidied away. A folder is *looked at*:
//! it belongs to the person and to whatever tools write into it, it can hold
//! fifty gigabytes of video no workspace should swallow, and kilna never
//! writes, moves or deletes a byte in it. The one thing kilna does there is
//! create the folder when a person asks it to, so that it exists to be filled.
//!
//! Where a work's folder is comes from two facts kept apart:
//!
//! * **the root** - a folder on this machine, one per workspace profile, kept
//!   in `media_root` and never carried anywhere else (it is `D:\` here and
//!   `/Volumes/media` on the next device);
//! * **the template** - the kind's `folder` in the profile: how a work's
//!   folder is named under the root. `{title}` reads the work's title,
//!   `{key}` one of its fields, and `{origin.title}` / `{origin.key}` the
//!   same of what it is all made from - the song, for its clip and its
//!   shorts - so the clip of a song can look into the song's folder.
//!
//! Found, not attached: nothing records which folder a work has, so a
//! template changed today points every work at its new place at once, and a
//! folder made by hand is seen the moment it has the right name.

use std::path::{Component, Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::profile::config::ProfileConfig;
use crate::work::Work;

/// What a placeholder reads off the work it is about: its title, or a field.
const TITLE: &str = "title";

/// The prefix that turns a placeholder to what the work is all made from.
const ORIGIN: &str = "origin.";

/// How many files a listing goes to. A song's folder holds a few dozen; a
/// template that resolved to the root of a drive would otherwise walk all of
/// it before the tab could draw.
const MOST_FILES: usize = 2000;

/// How deep a listing goes below the work's folder: `clip/v1/sd` is three.
const DEEPEST: usize = 6;

/// Where a workspace profile keeps its media on this machine, if anywhere.
pub fn root(conn: &Connection, profile_id: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT path FROM media_root WHERE profile_id = ?1",
            params![profile_id],
            |row| row.get(0),
        )
        .optional()?)
}

/// Every root on this machine - for the window's permission to read them,
/// granted for all profiles at start.
pub fn roots(conn: &Connection) -> Result<Vec<String>> {
    let mut statement = conn.prepare("SELECT path FROM media_root ORDER BY profile_id")?;
    let rows = statement
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(rows)
}

/// Set the root, or forget it with `None`. Returns the root it replaced.
///
/// The path is taken as a person chose it, but must be a folder that is
/// there: a root pointing at nothing makes every work's folder "missing",
/// which reads as a disk that lost its files.
pub fn set_root(
    conn: &Connection,
    profile_id: &str,
    path: Option<&str>,
    at: &str,
) -> Result<Option<String>> {
    let path = path.map(str::trim).filter(|path| !path.is_empty());
    if let Some(path) = path {
        if !Path::new(path).is_absolute() {
            return Err(Error::refused("folder.rootNotAbsolute").param("path", path));
        }
        if !Path::new(path).is_dir() {
            return Err(Error::refused("folder.rootMissing").param("path", path));
        }
    }
    let before = root(conn, profile_id)?;
    match path {
        None => {
            conn.execute(
                "DELETE FROM media_root WHERE profile_id = ?1",
                params![profile_id],
            )?;
        }
        Some(path) => {
            conn.execute(
                "INSERT INTO media_root (profile_id, path, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT (profile_id) DO UPDATE SET path = excluded.path,
                     updated_at = excluded.updated_at",
                params![profile_id, path, at],
            )?;
        }
    }
    Ok(before)
}

/// One placeholder of a template, as written between the braces.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Placeholder {
    /// Read off what the work is all made from rather than the work.
    origin: bool,
    /// `title`, or the key of a field.
    name: String,
}

impl Placeholder {
    fn parse(inner: &str) -> Option<Self> {
        let (origin, name) = match inner.strip_prefix(ORIGIN) {
            Some(rest) => (true, rest),
            None => (false, inner),
        };
        let well_formed = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        well_formed.then(|| Self {
            origin,
            name: name.to_owned(),
        })
    }

    /// The placeholder as a person wrote it, for a message about it.
    fn written(&self) -> String {
        if self.origin {
            format!("{ORIGIN}{}", self.name)
        } else {
            self.name.clone()
        }
    }
}

/// A piece of one segment of a template: words kept as written, or a
/// placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    Text(String),
    Value(Placeholder),
}

/// A template read into its segments, or what is wrong with it.
fn parse(template: &str) -> std::result::Result<Vec<Vec<Piece>>, String> {
    let template = template.trim();
    if template.is_empty() {
        return Err("is empty".into());
    }
    if template.starts_with(['/', '\\'])
        || template.as_bytes().get(1) == Some(&b':')
        || Path::new(template).is_absolute()
    {
        return Err(
            "is a place on a disk; it names a folder under the media folder, as `songs/{title}`"
                .into(),
        );
    }

    let mut segments = Vec::new();
    for segment in template.split(['/', '\\']) {
        let segment = segment.trim();
        if segment.is_empty() {
            return Err("has an empty step between two slashes".into());
        }
        if segment == "." || segment == ".." {
            return Err(format!(
                "steps `{segment}`; a work's folder lies under the media folder"
            ));
        }
        let mut pieces = Vec::new();
        let mut rest = segment;
        while let Some(open) = rest.find('{') {
            if open > 0 {
                pieces.push(Piece::Text(rest[..open].to_owned()));
            }
            let after = &rest[open + 1..];
            let Some(close) = after.find('}') else {
                return Err(format!("opens `{{` in `{segment}` and never closes it"));
            };
            let inner = &after[..close];
            let Some(placeholder) = Placeholder::parse(inner) else {
                return Err(format!(
                    "reads `{{{inner}}}`; a placeholder is `{{title}}`, a field's key, or either after `origin.`"
                ));
            };
            pieces.push(Piece::Value(placeholder));
            rest = &after[close + 1..];
        }
        if rest.contains('}') {
            return Err(format!("closes `}}` in `{segment}` it never opened"));
        }
        if !rest.is_empty() {
            pieces.push(Piece::Text(rest.to_owned()));
        }
        for piece in &pieces {
            if let Piece::Text(text) = piece
                && text.chars().any(is_forbidden)
            {
                return Err(format!(
                    "writes `{text}`, which no folder can be called on every system"
                ));
            }
        }
        segments.push(pieces);
    }
    Ok(segments)
}

/// What is wrong with a template, for the profile's validation; `None` when
/// it can name a folder.
pub fn problem(template: &str) -> Option<String> {
    parse(template).err()
}

/// A character no folder name may hold on Windows, where the rules are the
/// tightest - a template written on one machine names folders on all of them.
fn is_forbidden(c: char) -> bool {
    matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control()
}

/// Names Windows keeps for devices: a folder called `CON` cannot be made.
const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// The longest a value may run inside one folder name.
const LONGEST_VALUE: usize = 120;

/// A title or a field as part of a folder's name: the characters no folder
/// may hold taken out, runs of space made one, trailing dots and spaces
/// dropped (Windows drops them itself, and then cannot find the folder it
/// was asked for). Empty when nothing is left.
fn as_name(value: &str) -> String {
    let kept: String = value
        .chars()
        .filter(|c| !is_forbidden(*c))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut name: String = kept.chars().take(LONGEST_VALUE).collect();
    while name.ends_with(['.', ' ']) {
        name.pop();
    }
    if name.chars().all(|c| c == '.') {
        return String::new();
    }
    if RESERVED
        .iter()
        .any(|reserved| name.eq_ignore_ascii_case(reserved))
    {
        name.push('_');
    }
    name
}

/// What a placeholder reads off a work: its title, or a field that holds a
/// word or a number. A list, a yes-or-no or an empty field names no folder.
fn value_of(work: &Work, name: &str) -> Option<String> {
    let raw = if name == TITLE {
        work.title.clone()
    } else {
        match work.meta.get(name)? {
            Value::String(text) => text.clone(),
            Value::Number(number) => number.to_string(),
            _ => return None,
        }
    };
    Some(as_name(&raw)).filter(|name| !name.is_empty())
}

/// Why a work has no folder to look into, or where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// The folder, under the root - whether or not it exists yet.
    At(PathBuf),
    /// A placeholder the work cannot fill: an empty field, or one it has not
    /// got. Carries the placeholder as written.
    Unfilled(String),
}

/// Where a template puts a work's folder under `root`.
///
/// `origin` is what the work is all made from, or the work itself when it is
/// made from nothing - the reading `{origin}` has everywhere else, so a song
/// and its clip can share one template.
pub fn resolve(root: &Path, template: &str, work: &Work, origin: &Work) -> Result<Resolved> {
    let segments = parse(template).map_err(|problem| {
        Error::refused("folder.badTemplate")
            .param("template", template)
            .param("problem", problem)
    })?;
    let mut path = root.to_path_buf();
    for pieces in segments {
        let mut name = String::new();
        for piece in pieces {
            match piece {
                Piece::Text(text) => name.push_str(&text),
                Piece::Value(placeholder) => {
                    let about = if placeholder.origin { origin } else { work };
                    match value_of(about, &placeholder.name) {
                        Some(value) => name.push_str(&value),
                        None => return Ok(Resolved::Unfilled(placeholder.written())),
                    }
                }
            }
        }
        let name = as_name(&name);
        if name.is_empty() {
            return Ok(Resolved::Unfilled(String::new()));
        }
        path.push(name);
    }
    // Held, not trusted: every value has had its separators taken out, so
    // nothing here can climb - and if a change ever lets one through, this
    // is the line that says so instead of a listing of somebody's home.
    if path.strip_prefix(root).map_or(true, |under| {
        under
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    }) {
        return Err(Error::Internal(format!(
            "a folder template climbed out of the media folder: {}",
            path.display()
        )));
    }
    Ok(Resolved::At(path))
}

/// Where a work stands with its folder - what the Files tab says above the
/// files, or instead of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum FolderState {
    /// No media folder is set for this workspace on this machine.
    NoRoot,
    /// The media folder is set but is not there: a drive not plugged in, a
    /// folder renamed.
    RootMissing,
    /// The work's kind names no folder.
    NoTemplate,
    /// The template reads a field this work leaves empty.
    Unfilled,
    /// The folder would be here, and is not yet.
    Absent,
    /// The folder is there, and its files are listed.
    Found,
}

/// One file in a work's folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct FolderFile {
    /// Where it is, whole - what the window loads it by.
    pub path: String,
    /// Where it is inside the work's folder, with `/` between the steps on
    /// every system: `clip/v1/take-2.mp4`.
    pub relative: String,
    /// Its size in bytes.
    pub size: u64,
    /// When it last changed, as RFC 3339; absent when the system does not
    /// say.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub modified: Option<String>,
}

/// A work's folder, and what is in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct WorkFolder {
    pub state: FolderState,
    /// The media folder, when one is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub root: Option<String>,
    /// The work's folder, once the template could name it - whether or not it
    /// exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub path: Option<String>,
    /// The template the kind names, for saying where the folder was looked
    /// for.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub template: Option<String>,
    /// The placeholder the work could not fill, when that is the state.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub unfilled: Option<String>,
    /// The files, sorted by where they are; empty unless found.
    pub files: Vec<FolderFile>,
    /// Whether there were more files than a listing goes to.
    pub truncated: bool,
}

impl WorkFolder {
    fn stated(state: FolderState) -> Self {
        Self {
            state,
            root: None,
            path: None,
            template: None,
            unfilled: None,
            files: Vec::new(),
            truncated: false,
        }
    }
}

/// Find a work's folder and list it.
pub fn of_work(conn: &Connection, config: &ProfileConfig, work: &Work) -> Result<WorkFolder> {
    let Some(root) = root(conn, &work.profile_id)? else {
        return Ok(WorkFolder::stated(FolderState::NoRoot));
    };
    let template = config
        .work_kinds
        .iter()
        .find(|kind| kind.key == work.kind)
        .and_then(|kind| kind.folder.clone());
    let mut found = WorkFolder {
        root: Some(root.clone()),
        template: template.clone(),
        ..WorkFolder::stated(FolderState::NoTemplate)
    };
    if !Path::new(&root).is_dir() {
        found.state = FolderState::RootMissing;
        return Ok(found);
    }
    let Some(template) = template else {
        return Ok(found);
    };

    let origin = crate::publication::origin(conn, config, &work.id)?;
    let origin = origin.as_ref().unwrap_or(work);
    match resolve(Path::new(&root), &template, work, origin)? {
        Resolved::Unfilled(placeholder) => {
            found.state = FolderState::Unfilled;
            found.unfilled = Some(placeholder);
        }
        Resolved::At(path) => {
            found.path = Some(path.to_string_lossy().into_owned());
            if path.is_dir() {
                found.state = FolderState::Found;
                let (files, truncated) = list(&path);
                found.files = files;
                found.truncated = truncated;
            } else {
                found.state = FolderState::Absent;
            }
        }
    }
    Ok(found)
}

/// Make a work's folder, so that it is there to be filled - the one thing
/// kilna writes under the media folder. Answers with the folder as it now
/// stands.
pub fn create(conn: &Connection, config: &ProfileConfig, work: &Work) -> Result<WorkFolder> {
    let folder = of_work(conn, config, work)?;
    match (folder.state, folder.path.as_deref()) {
        (FolderState::Found, _) => Ok(folder),
        (FolderState::Absent, Some(path)) => {
            std::fs::create_dir_all(path).map_err(|cause| {
                Error::refused("folder.createFailed")
                    .param("path", path)
                    .param("cause", cause.to_string())
            })?;
            of_work(conn, config, work)
        }
        (FolderState::NoRoot, _) => Err(Error::refused("folder.noRoot")),
        (FolderState::RootMissing, _) => {
            Err(Error::refused("folder.rootMissing").param("path", folder.root.unwrap_or_default()))
        }
        (FolderState::NoTemplate, _) => Err(Error::refused("folder.noTemplate")),
        (FolderState::Unfilled, _) | (FolderState::Absent, None) => {
            Err(Error::refused("folder.unfilled")
                .param("placeholder", folder.unfilled.unwrap_or_default()))
        }
    }
}

/// The files under `folder`, sorted by where they are, and whether the
/// listing stopped short.
///
/// What the system keeps for itself is left out - hidden files, a folder's
/// thumbnail cache, its view settings - and so is anything that cannot be
/// read: a listing is a look, and a look at a folder with one locked
/// subfolder still shows the rest.
fn list(folder: &Path) -> (Vec<FolderFile>, bool) {
    let mut files = Vec::new();
    let mut truncated = false;
    let mut pending = vec![(folder.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_the_systems(&name) {
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                if depth + 1 < DEEPEST {
                    pending.push((path, depth + 1));
                }
                continue;
            }
            if !kind.is_file() {
                continue;
            }
            if files.len() == MOST_FILES {
                truncated = true;
                continue;
            }
            let metadata = entry.metadata().ok();
            let relative = path
                .strip_prefix(folder)
                .unwrap_or(&path)
                .components()
                .map(|step| step.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            files.push(FolderFile {
                path: path.to_string_lossy().into_owned(),
                relative,
                size: metadata.as_ref().map_or(0, std::fs::Metadata::len),
                modified: metadata
                    .and_then(|metadata| metadata.modified().ok())
                    .and_then(|moment| {
                        time::OffsetDateTime::from(moment)
                            .format(&time::format_description::well_known::Rfc3339)
                            .ok()
                    }),
            });
        }
    }
    // Folders first at each level would need a tree; sorted by path, a
    // folder's files stand together and the tab groups them by it.
    files.sort_by(|a, b| {
        a.relative
            .to_lowercase()
            .cmp(&b.relative.to_lowercase())
            .then_with(|| a.relative.cmp(&b.relative))
    });
    (files, truncated)
}

/// The endings a file may have for kilna to hand it to the program the
/// system opens it with: pictures, clips, sounds and plain documents - what
/// a person keeps for a work. Never a program or a script: a click on a tile
/// is a look, and a look must not run anything.
pub const OPENABLE: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "gif", "avif", "mp4", "webm", "mov", "m4v", "mp3", "wav", "flac",
    "m4a", "aac", "ogg", "oga", "opus", "pdf", "txt", "md",
];

/// Whether `path` is a file kilna may open for a person: under one of
/// `places`, with no step that climbs, and ending in one of [`OPENABLE`].
pub fn may_open(path: &Path, places: &[PathBuf]) -> bool {
    let climbs = path
        .components()
        .any(|step| matches!(step, Component::ParentDir | Component::CurDir));
    let ending = path
        .extension()
        .map(|ending| ending.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    !climbs
        && OPENABLE.contains(&ending.as_str())
        && places.iter().any(|place| path.starts_with(place))
}

/// Whether a name is one the system keeps for itself in a folder.
fn is_the_systems(name: &str) -> bool {
    name.starts_with('.')
        || name.eq_ignore_ascii_case("desktop.ini")
        || name.eq_ignore_ascii_case("thumbs.db")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, json};

    /// A work with this title and these fields, as a template reads it.
    fn work(title: &str, meta: Value) -> Work {
        thread_local! {
            static BASE: Work = {
                let (conn, profile_id) = crate::fixtures::workspace();
                crate::fixtures::song(&conn, &profile_id, "base")
            };
        }
        let meta: Map<String, Value> = match meta {
            Value::Object(map) => map,
            _ => Map::new(),
        };
        let mut work = BASE.with(Clone::clone);
        work.title = title.into();
        work.meta = meta;
        work
    }

    fn at(root: &Path, steps: &[&str]) -> Resolved {
        let mut path = root.to_path_buf();
        for step in steps {
            path.push(step);
        }
        Resolved::At(path)
    }

    /// A template names a folder under the root from the work's title and
    /// fields, and from what it is made from.
    #[test]
    fn a_template_reads_the_work_and_what_it_is_made_from() {
        let root = Path::new("media-root");
        let song = work("Harbour lights", json!({ "code": "h042", "bpm": 92 }));
        let clip = work("Harbour lights (video)", json!({}));

        assert_eq!(
            resolve(root, "songs/{title}", &song, &song).unwrap(),
            at(root, &["songs", "Harbour lights"])
        );
        assert_eq!(
            resolve(root, "{code}-{bpm}", &song, &song).unwrap(),
            at(root, &["h042-92"]),
            "a number names a folder as written"
        );
        assert_eq!(
            resolve(root, "songs/{origin.code}/clip", &clip, &song).unwrap(),
            at(root, &["songs", "h042", "clip"]),
            "the clip looks into its song's folder"
        );
    }

    /// A placeholder the work cannot fill is said by name, rather than
    /// leaving a hole in the path that lands on the folder above.
    #[test]
    fn an_empty_field_names_no_folder_and_says_which() {
        let root = Path::new("media-root");
        let song = work("Harbour lights", json!({ "code": "", "tags": ["a"] }));

        assert_eq!(
            resolve(root, "songs/{code}", &song, &song).unwrap(),
            Resolved::Unfilled("code".into())
        );
        assert_eq!(
            resolve(root, "songs/{tags}", &song, &song).unwrap(),
            Resolved::Unfilled("tags".into()),
            "a list names no folder"
        );
        assert_eq!(
            resolve(root, "{origin.missing}", &song, &song).unwrap(),
            Resolved::Unfilled("origin.missing".into())
        );
    }

    /// A title is a person's words; a folder name has rules. What breaks the
    /// rules is taken out, never turned into a step of the path.
    #[test]
    fn a_title_cannot_climb_or_break_a_name() {
        let root = Path::new("media-root");
        for (title, expected) in [
            ("AC/DC: Live?", "ACDC Live"),
            ("..\\..\\escape", "....escape"),
            ("Wait...  ", "Wait"),
            ("con", "con_"),
            ("  spaced   out  ", "spaced out"),
        ] {
            let song = work(title, json!({}));
            assert_eq!(
                resolve(root, "{title}", &song, &song).unwrap(),
                at(root, &[expected]),
                "{title:?}"
            );
        }
        let dots = work("..", json!({}));
        assert_eq!(
            resolve(root, "{title}", &dots, &dots).unwrap(),
            Resolved::Unfilled("title".into()),
            "a title of dots names nothing, and the title is what says so"
        );
    }

    /// A template that could name a folder outside the root, or no folder at
    /// all, is refused where the profile is checked.
    #[test]
    fn a_template_that_leaves_the_root_is_refused() {
        for template in [
            "",
            "/songs/{title}",
            "C:/songs",
            "\\\\server\\share",
            "songs/../{title}",
            "./{title}",
            "songs//{title}",
            "songs/{title",
            "songs/title}",
            "songs/{a b}",
            "songs/{}",
            "songs?/{title}",
        ] {
            assert!(
                problem(template).is_some(),
                "{template:?} should be refused"
            );
        }
        for template in ["{title}", "songs/{origin.code}/clip", "a\\{title} (draft)"] {
            assert_eq!(problem(template), None, "{template:?}");
        }
    }

    /// The listing walks the folder, leaves out what the system keeps, and
    /// says where each file is with forward slashes on every system.
    #[test]
    fn a_listing_walks_the_folder_and_skips_the_systems_files() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path();
        std::fs::create_dir_all(folder.join("clip").join("v1")).unwrap();
        std::fs::create_dir_all(folder.join(".cache")).unwrap();
        std::fs::write(folder.join("cover.png"), b"png").unwrap();
        std::fs::write(folder.join("clip").join("v1").join("take.mp4"), b"mp4").unwrap();
        std::fs::write(folder.join("Thumbs.db"), b"x").unwrap();
        std::fs::write(folder.join(".cache").join("hidden.bin"), b"x").unwrap();

        let (files, truncated) = list(folder);
        let names: Vec<&str> = files.iter().map(|f| f.relative.as_str()).collect();
        assert_eq!(names, ["clip/v1/take.mp4", "cover.png"]);
        assert!(!truncated);
        assert_eq!(files[1].size, 3);
        assert!(files[1].modified.is_some());
    }

    /// Only a media file under a place kilna looks at may be opened: never a
    /// program, never a file elsewhere, never a path that climbs out.
    #[test]
    fn only_a_media_file_under_a_looked_at_place_may_be_opened() {
        let root = PathBuf::from("media-root");
        let places = [root.clone()];
        assert!(may_open(&root.join("songs").join("mix.WAV"), &places));
        assert!(may_open(&root.join("cover.png"), &places));
        assert!(!may_open(&root.join("setup.exe"), &places), "a program");
        assert!(!may_open(&root.join("run.bat"), &places), "a script");
        assert!(!may_open(&root.join("no-ending"), &places));
        assert!(!may_open(Path::new("elsewhere/cover.png"), &places));
        assert!(
            !may_open(&root.join("..").join("cover.png"), &places),
            "a path that climbs out"
        );
    }

    /// A workspace with no root says so before anything else, and a root set
    /// to a folder that is not there is refused.
    #[test]
    fn the_root_is_kept_per_profile_and_must_be_there() {
        let (conn, profile_id) = crate::fixtures::workspace();
        assert_eq!(root(&conn, &profile_id).unwrap(), None);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_string_lossy().into_owned();
        let before = set_root(&conn, &profile_id, Some(&path), "2030-01-01T00:00:00.000Z").unwrap();
        assert_eq!(before, None);
        assert_eq!(root(&conn, &profile_id).unwrap(), Some(path.clone()));
        assert_eq!(roots(&conn).unwrap(), std::slice::from_ref(&path));

        let missing = dir.path().join("not-there").to_string_lossy().into_owned();
        let refused = set_root(
            &conn,
            &profile_id,
            Some(&missing),
            "2030-01-01T00:00:00.000Z",
        )
        .unwrap_err();
        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("folder.rootMissing")
        );
        let relative = set_root(
            &conn,
            &profile_id,
            Some("media"),
            "2030-01-01T00:00:00.000Z",
        )
        .unwrap_err();
        assert_eq!(
            relative.refusal().map(|r| r.code),
            Some("folder.rootNotAbsolute")
        );

        let before = set_root(&conn, &profile_id, None, "2030-01-01T00:00:00.000Z").unwrap();
        assert_eq!(before, Some(path), "forgetting answers with what it forgot");
        assert_eq!(root(&conn, &profile_id).unwrap(), None);
    }

    /// The whole way through: a song finds its folder by its field, a clip
    /// made from it finds the song's, and a folder asked for is made.
    #[test]
    fn a_work_finds_its_folder_and_a_missing_one_can_be_made() {
        let (conn, profile_id) = crate::fixtures::workspace();
        let dir = tempfile::tempdir().unwrap();
        let root_path = dir.path().to_string_lossy().into_owned();

        let mut config = crate::profile::config_for(&conn, &profile_id).unwrap();
        let song = crate::fixtures::song(&conn, &profile_id, "Harbour lights");
        for kind in &mut config.work_kinds {
            kind.folder = Some("songs/{origin.title}".into());
        }

        assert_eq!(
            of_work(&conn, &config, &song).unwrap().state,
            FolderState::NoRoot
        );
        set_root(
            &conn,
            &profile_id,
            Some(&root_path),
            "2030-01-01T00:00:00.000Z",
        )
        .unwrap();

        let folder = of_work(&conn, &config, &song).unwrap();
        assert_eq!(folder.state, FolderState::Absent);
        let expected = dir.path().join("songs").join("Harbour lights");
        assert_eq!(folder.path.as_deref(), Some(expected.to_str().unwrap()));

        let made = create(&conn, &config, &song).unwrap();
        assert_eq!(made.state, FolderState::Found);
        assert!(expected.is_dir(), "the folder is on disk");
        std::fs::write(expected.join("mix.wav"), b"RIFF").unwrap();
        let listed = of_work(&conn, &config, &song).unwrap();
        assert_eq!(listed.files.len(), 1);
        assert_eq!(listed.files[0].relative, "mix.wav");

        for kind in &mut config.work_kinds {
            kind.folder = None;
        }
        assert_eq!(
            of_work(&conn, &config, &song).unwrap().state,
            FolderState::NoTemplate
        );
        let refused = create(&conn, &config, &song).unwrap_err();
        assert_eq!(refused.refusal().map(|r| r.code), Some("folder.noTemplate"));
    }
}
