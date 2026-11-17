//! A work's folder on disk: where a person keeps what they make for a work
//! outside kilna - the renders, the takes, the stills a generator gave back.
//!
//! The opposite arrangement to an asset, and on purpose (ADR 0057). An asset
//! is *copied into* the workspace (ADR 0027): kilna owns the copy, a backup
//! carries it, and the original can be tidied away. A folder is *looked at*:
//! it belongs to the person and to whatever tools write into it, it can hold
//! fifty gigabytes of video no workspace should swallow, and kilna never
//! writes, moves or deletes a byte in it. Two things kilna does there, and
//! both to the folder itself, never to a file (FR-04, v0.120): it makes the
//! folder when it is first needed - a person opens it - and it takes away a
//! folder that holds nothing when its work goes to the trash. A folder with
//! one file anywhere under it is never removed by anything.
//!
//! Where a work's folder is comes from two facts kept apart:
//!
//! * **the root** - a folder on this machine, one per workspace profile, kept
//!   in `media_root` and never carried anywhere else (it is `D:\` here and
//!   `/Volumes/media` on the next device);
//! * **the template** - the kind's `folder` in the profile: how a work's
//!   folder is named under the root. `{id}` reads the work's id and `{kind}`
//!   its kind - `{kind}s/{id}`, what the shipped profiles name, is a folder
//!   a rename never moves; `{title}` reads the work's title, `{key}` one of
//!   its fields, and `{origin.title}` / `{origin.key}` the same of what it
//!   is all made from - the song, for its clip and its shorts - so the clip
//!   of a song can look into the song's folder.
//!
//! Inside a work's folder the top is for what is finished - the render, the
//! master, the cover - and what it was made out of lies below, in `raw/`,
//! `scenes/`, `images/`, `drafts/`, `candidates/`. Nothing enforces the
//! names; what holds is that a kind's rule for its finished file, `*.mp4`,
//! reads the top of the folder and never a folder below, so a take in
//! `scenes/` is never taken for the finished clip.
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

/// The placeholder for the work's id: the one name a work has that nothing
/// ever changes (FR-04). A folder named by it stays where it is when the
/// work is renamed.
const ID: &str = "id";

/// The placeholder for the work's kind, as its key: `{kind}s/{id}`.
const KIND: &str = "kind";

/// The placeholder for a work's number among the works of its kind made from
/// the same origin - the 2 of "Song (short 2)" (ADR 0070): `shorts/v{n}`.
const NUMBER: &str = "n";

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

/// What a placeholder reads off a work: its title, its number among the works
/// of its kind made from the same origin, or a field that holds a word or a
/// number. A list, a yes-or-no or an empty field names no folder.
fn value_of(work: &Work, name: &str) -> Option<String> {
    let raw = if name == TITLE {
        work.title.clone()
    } else if name == ID {
        work.id.clone()
    } else if name == KIND {
        work.kind.clone()
    } else if name == NUMBER {
        work.ordinal?.to_string()
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
    /// The work's finished file, and how it was found.
    pub file: FinishedFile,
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
            file: FinishedFile::stated(FileState::NoFolder, None),
        }
    }
}

/// How a work's finished file was found, or why it was not (ADR 0070).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum FileState {
    /// The work's folder is not there to look in.
    NoFolder,
    /// The kind names no rule, and nothing was chosen by hand.
    NoRule,
    /// The rule matches nothing in the folder yet.
    NotYet,
    /// Found by the kind's rule: the newest file the pattern matches.
    Found,
    /// Chosen by hand in the Files tab.
    Chosen,
    /// Chosen by hand, and no longer there. Not replaced by what the rule
    /// finds: the person said which file, and cutting another one in its name
    /// would be a guess.
    ChosenMissing,
}

/// The file a work is finished as - the clip a short is cut from, the short
/// a plugin made - found in the work's folder, never copied (ADR 0070).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct FinishedFile {
    pub state: FileState,
    /// The kind's rule, for saying what was looked for.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub rule: Option<String>,
    /// Where it is inside the work's folder, `/` between the steps - also
    /// the choice that is no longer there.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub relative: Option<String>,
    /// Where it is, whole, when it is there.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub path: Option<String>,
}

impl FinishedFile {
    fn stated(state: FileState, rule: Option<String>) -> Self {
        Self {
            state,
            rule,
            relative: None,
            path: None,
        }
    }
}

/// Find a work's folder and list it.
pub fn of_work(conn: &Connection, config: &ProfileConfig, work: &Work) -> Result<WorkFolder> {
    let mut found = locate(conn, config, work)?;
    if found.state == FolderState::Found
        && let Some(path) = found.path.as_deref()
    {
        let (files, truncated) = list(Path::new(path));
        found.files = files;
        found.truncated = truncated;
    }
    Ok(found)
}

/// A work's finished file: the one chosen by hand when there is one, else the
/// newest file the kind's rule matches in the work's folder.
pub fn finished(conn: &Connection, config: &ProfileConfig, work: &Work) -> Result<FinishedFile> {
    Ok(locate(conn, config, work)?.file)
}

/// Where a work's folder is and what its finished file is, without listing
/// the folder.
pub fn locate(conn: &Connection, config: &ProfileConfig, work: &Work) -> Result<WorkFolder> {
    let mut found = place(conn, config, work)?;
    let rule = config
        .work_kinds
        .iter()
        .find(|kind| kind.key == work.kind)
        .and_then(|kind| kind.file.clone());
    found.file = match (found.state, found.path.as_deref()) {
        (FolderState::Found, Some(folder)) => finished_in(Path::new(folder), work, rule),
        _ => FinishedFile::stated(FileState::NoFolder, rule),
    };
    Ok(found)
}

/// The finished file inside a folder that is there.
fn finished_in(folder: &Path, work: &Work, rule: Option<String>) -> FinishedFile {
    if let Some(chosen) = &work.file {
        let path = chosen
            .split('/')
            .fold(folder.to_path_buf(), |path, step| path.join(step));
        let there = path.is_file();
        return FinishedFile {
            state: if there {
                FileState::Chosen
            } else {
                FileState::ChosenMissing
            },
            rule,
            relative: Some(chosen.clone()),
            path: there.then(|| path.to_string_lossy().into_owned()),
        };
    }
    let Some(pattern) = rule.as_deref().and_then(|rule| steps_of(rule).ok()) else {
        return FinishedFile::stated(FileState::NoRule, rule);
    };
    match newest(folder, &pattern) {
        Some((path, relative)) => FinishedFile {
            state: FileState::Found,
            rule,
            relative: Some(relative),
            path: Some(path.to_string_lossy().into_owned()),
        },
        None => FinishedFile::stated(FileState::NotYet, rule),
    }
}

/// The steps of a rule for a finished file - `video/*.mp4` - or what is
/// wrong with it. A step may hold `*` for any run of characters and `?` for
/// one; a rule names a file inside the work's folder and nothing else.
fn steps_of(rule: &str) -> std::result::Result<Vec<String>, String> {
    let rule = rule.trim();
    if rule.is_empty() {
        return Err("is empty".into());
    }
    if rule.starts_with(['/', '\\']) || rule.as_bytes().get(1) == Some(&b':') {
        return Err(
            "is a place on a disk; it names a file inside the work's folder, as `video/*.mp4`"
                .into(),
        );
    }
    let mut steps = Vec::new();
    for step in rule.split(['/', '\\']) {
        if step.is_empty() {
            return Err("has an empty step between two slashes".into());
        }
        if step == "." || step == ".." {
            return Err(format!(
                "steps `{step}`; a finished file lies inside the work's folder"
            ));
        }
        if step
            .chars()
            .any(|c| c != '*' && c != '?' && is_forbidden(c))
        {
            return Err(format!(
                "writes `{step}`, which no file can be called on every system"
            ));
        }
        steps.push(step.to_owned());
    }
    Ok(steps)
}

/// What is wrong with a rule for a finished file, for the profile's
/// validation; `None` when it can name one.
pub fn rule_problem(rule: &str) -> Option<String> {
    steps_of(rule).err()
}

/// Whether a name matches one step of a rule: `*` any run of characters, `?`
/// any one, everything else itself whatever its case - the systems a person
/// keeps media on mostly do not tell `V1.MP4` from `v1.mp4`.
fn step_matches(step: &str, name: &str) -> bool {
    let step: Vec<char> = step.to_lowercase().chars().collect();
    let name: Vec<char> = name.to_lowercase().chars().collect();
    // The classic two-pointer match with one remembered star: linear, and no
    // recursion for a name full of stars to blow up.
    let (mut s, mut n) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while n < name.len() {
        if s < step.len() && (step[s] == '?' || step[s] == name[n]) {
            s += 1;
            n += 1;
        } else if s < step.len() && step[s] == '*' {
            star = Some((s, n));
            s += 1;
        } else if let Some((at, from)) = star {
            s = at + 1;
            n = from + 1;
            star = Some((at, from + 1));
        } else {
            return false;
        }
    }
    step[s..].iter().all(|c| *c == '*')
}

/// The newest file under `folder` the steps match, and where it is inside the
/// folder: last changed first, the greater path when two changed at once.
fn newest(folder: &Path, steps: &[String]) -> Option<(PathBuf, String)> {
    let mut places = vec![(folder.to_path_buf(), String::new())];
    for (index, step) in steps.iter().enumerate() {
        let last = index + 1 == steps.len();
        let mut next = Vec::new();
        for (dir, relative) in places {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if is_the_systems(&name) || !step_matches(step, &name) {
                    continue;
                }
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if (last && kind.is_file()) || (!last && kind.is_dir()) {
                    let relative = if relative.is_empty() {
                        name
                    } else {
                        format!("{relative}/{name}")
                    };
                    next.push((entry.path(), relative));
                }
            }
        }
        places = next;
    }
    places
        .into_iter()
        .map(|(path, relative)| {
            let changed = std::fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .ok();
            (changed, relative, path)
        })
        .max_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
        .map(|(_, relative, path)| (path, relative))
}

/// A path a person chose inside a work's folder, as it is kept: the steps
/// joined by `/`. Refused when it is a place on a disk or climbs out - the
/// choice names a file of the folder and nothing else.
pub fn inside(path: &str) -> Result<String> {
    let outside = || Error::refused("folder.fileOutside").param("path", path);
    let path = path.trim();
    if path.is_empty() || path.starts_with(['/', '\\']) || path.as_bytes().get(1) == Some(&b':') {
        return Err(outside());
    }
    let mut steps = Vec::new();
    for step in path.split(['/', '\\']) {
        if step.is_empty() || step == "." || step == ".." || step.chars().any(is_forbidden) {
            return Err(outside());
        }
        steps.push(step);
    }
    Ok(steps.join("/"))
}

/// Where a work's folder is, and whether it is there.
fn place(conn: &Connection, config: &ProfileConfig, work: &Work) -> Result<WorkFolder> {
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
            } else {
                found.state = FolderState::Absent;
            }
        }
    }
    Ok(found)
}

/// Make a work's folder if it is not there, so that it is there to be
/// filled - called when the folder is first needed, never when the work is
/// made (FR-04): a work with no files has no folder, and a year of planned
/// works does not leave a graveyard of empty ones on the disk. Answers with
/// the folder as it now stands.
pub fn ensure(conn: &Connection, config: &ProfileConfig, work: &Work) -> Result<WorkFolder> {
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

/// Where a work's folder is, when it is there - read while the work still
/// is, for [`remove_if_empty`] once it has gone to the trash. Nothing when
/// there is no folder, or no telling where it would be.
pub fn standing(conn: &Connection, work_id: &str) -> Option<PathBuf> {
    let work = crate::work::get(conn, work_id).ok()??;
    let config = crate::profile::config_for(conn, &work.profile_id).ok()?;
    let found = place(conn, &config, &work).ok()?;
    match (found.state, found.path) {
        (FolderState::Found, Some(path)) => Some(PathBuf::from(path)),
        _ => None,
    }
}

/// Take a folder away if nothing is in it - what happens to a work's folder
/// when the work goes to the trash, deleted or its making undone. Answers
/// whether it was removed.
///
/// Only ever a folder, never a file: a tree with one file anywhere in it,
/// whatever the file is, is left exactly as it stands. The folders are
/// removed with the call that refuses a folder holding anything, so a file
/// that arrives while this runs keeps its folder. A work that comes back
/// from the trash gets its folder again when it is next opened.
pub fn remove_if_empty(folder: &Path) -> bool {
    fn holds_a_file(folder: &Path, depth: usize) -> bool {
        // Deeper than a work's folder goes, or unreadable: not known to be
        // empty, so it is not.
        if depth > DEEPEST {
            return true;
        }
        let Ok(entries) = std::fs::read_dir(folder) else {
            return true;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                return true;
            };
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => {
                    if holds_a_file(&entry.path(), depth + 1) {
                        return true;
                    }
                }
                // A file, a link, or something that cannot be told.
                _ => return true,
            }
        }
        false
    }
    fn remove(folder: &Path) -> bool {
        if let Ok(entries) = std::fs::read_dir(folder) {
            for entry in entries.flatten() {
                remove(&entry.path());
            }
        }
        std::fs::remove_dir(folder).is_ok()
    }
    !holds_a_file(folder, 0) && remove(folder)
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
                    .and_then(stamp),
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

/// The endings of what is read inside the window as text: a lyric, a
/// subtitle track, notes, a table, a log.
pub const TEXTS: &[&str] = &[
    "txt", "md", "srt", "vtt", "lrc", "json", "csv", "log", "yaml", "yml",
];

/// How much of a text the window is given: enough for a long lyric and a
/// film's subtitles, not a log of a week.
pub const MOST_TEXT: u64 = 512 * 1024;

/// A text read for the viewer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct TextFile {
    pub text: String,
    /// Whether the file is longer than what was read.
    pub truncated: bool,
    /// The file's whole size in bytes.
    pub size: u64,
}

/// Whether `path` lies under one of `places` with no step that climbs: the
/// folders kilna looks at, and nothing beside them.
fn within(path: &Path, places: &[PathBuf]) -> bool {
    !path
        .components()
        .any(|step| matches!(step, Component::ParentDir | Component::CurDir))
        && places.iter().any(|place| path.starts_with(place))
}

/// Read a text for the viewer (FR-14): only a file under one of `places`,
/// only one of [`TEXTS`], and no more than [`MOST_TEXT`] of it. Bytes that
/// are not UTF-8 are shown as the mark for one rather than refusing the
/// file - a subtitle track in an old encoding is still worth a look.
pub fn read_text(path: &Path, places: &[PathBuf]) -> Result<TextFile> {
    use std::io::Read as _;
    let ending = path
        .extension()
        .map(|ending| ending.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let shown = path.to_string_lossy().into_owned();
    if !within(path, places) || !TEXTS.contains(&ending.as_str()) {
        return Err(Error::refused("folder.notReadable").param("path", shown));
    }
    let failed = |cause: std::io::Error| {
        Error::refused("folder.readFailed")
            .param("path", path.to_string_lossy().into_owned())
            .param("cause", cause.to_string())
    };
    let file = std::fs::File::open(path).map_err(failed)?;
    let size = file.metadata().map_err(failed)?.len();
    let mut bytes = Vec::new();
    file.take(MOST_TEXT)
        .read_to_end(&mut bytes)
        .map_err(failed)?;
    Ok(TextFile {
        text: String::from_utf8_lossy(&bytes).into_owned(),
        truncated: size > MOST_TEXT,
        size,
    })
}

/// A file of a work's folder, by its path inside it: there, and a file.
pub fn file_in(folder: &Path, relative: &str) -> Result<PathBuf> {
    let relative = inside(relative)?;
    let path = folder.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    if !path.is_file() {
        return Err(Error::refused("folder.fileMissing").param("path", relative));
    }
    Ok(path)
}

/// Give a file of a work's folder another name, in the folder it lies in
/// (FR-14). Answers with its new path inside the work's folder.
///
/// A name, not a place: no step, no character a file may not hold, no name
/// the system keeps for a device. Never onto another file - a rename that
/// lands on a take already there would be a take lost. The one thing kilna
/// changes in a folder it otherwise only looks at, and only when asked to.
pub fn rename_file(folder: &Path, relative: &str, name: &str) -> Result<String> {
    let from = file_in(folder, relative)?;
    let name = name.trim();
    let stem = name.split('.').next().unwrap_or_default();
    let bad = name.is_empty()
        || name.chars().any(is_forbidden)
        || name.ends_with(['.', ' '])
        || name.chars().all(|c| c == '.')
        || name.chars().count() > LONGEST_VALUE
        || RESERVED
            .iter()
            .any(|reserved| stem.eq_ignore_ascii_case(reserved));
    if bad {
        return Err(Error::refused("folder.badName").param("name", name));
    }
    let to = from.with_file_name(name);
    let relative = inside(relative)?;
    let renamed = match relative.rfind('/') {
        Some(cut) => format!("{}/{name}", &relative[..cut]),
        None => name.to_owned(),
    };
    if to == from {
        return Ok(renamed);
    }
    // The same file under another case is this file, where names are told
    // apart by case or not; any other file there is somebody else's.
    let same_file = to.to_string_lossy().to_lowercase() == from.to_string_lossy().to_lowercase();
    if to.exists() && !same_file {
        return Err(Error::refused("folder.nameTaken").param("name", name));
    }
    std::fs::rename(&from, &to).map_err(|cause| {
        Error::refused("folder.renameFailed")
            .param("name", name)
            .param("cause", cause.to_string())
    })?;
    Ok(renamed)
}

/// Copy a file kilna looks at to a place the person chose - "save a copy".
/// Only a file under one of `places`; where it goes is the person's answer
/// to the system's own dialog, which has already asked about writing over.
pub fn save_copy(source: &Path, places: &[PathBuf], destination: &Path) -> Result<()> {
    if !within(source, places) || !source.is_file() {
        return Err(Error::refused("folder.notReadable")
            .param("path", source.to_string_lossy().into_owned()));
    }
    std::fs::copy(source, destination)
        .map(|_| ())
        .map_err(|cause| {
            Error::refused("folder.copyFailed")
                .param("path", destination.to_string_lossy().into_owned())
                .param("cause", cause.to_string())
        })
}

/// When a file last changed, as RFC 3339 - what a listing says of a file in
/// a work's folder and of one kept in the workspace alike.
pub fn stamp(moment: std::time::SystemTime) -> Option<String> {
    time::OffsetDateTime::from(moment)
        .format(&time::format_description::well_known::Rfc3339)
        .ok()
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

        let made = ensure(&conn, &config, &song).unwrap();
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
        let refused = ensure(&conn, &config, &song).unwrap_err();
        assert_eq!(refused.refusal().map(|r| r.code), Some("folder.noTemplate"));
    }

    /// `{id}` and `{kind}` name a folder a rename never moves (FR-04): the
    /// shipped template, `{kind}s/{id}`.
    #[test]
    fn a_folder_named_by_id_stays_where_it_is_when_the_work_is_renamed() {
        let root = Path::new("media-root");
        let mut song = work("Harbour lights", json!({}));
        let id = song.id.clone();

        let before = resolve(root, "{kind}s/{id}", &song, &song).unwrap();
        assert_eq!(before, at(root, &["songs", &id]));

        song.title = "Harbour lights (the long one)".into();
        assert_eq!(resolve(root, "{kind}s/{id}", &song, &song).unwrap(), before);

        // What a clip is made from has an id and a kind too.
        let mut clip = work("Clip", json!({}));
        clip.kind = "video".into();
        assert_eq!(
            resolve(root, "{origin.kind}s/{origin.id}/{kind}", &clip, &song).unwrap(),
            at(root, &["songs", &id, "video"])
        );
        assert_eq!(problem("{kind}s/{id}"), None);
    }

    /// A work is made without a folder and gets one when it is first needed;
    /// in the trash it takes an empty folder with it and leaves one that
    /// holds a file exactly as it stands (FR-04).
    #[test]
    fn a_folder_is_made_when_needed_and_only_an_empty_one_goes_with_its_work() {
        let (conn, profile_id) = crate::fixtures::workspace();
        let dir = tempfile::tempdir().unwrap();
        set_root(
            &conn,
            &profile_id,
            Some(&dir.path().to_string_lossy()),
            "2030-01-01T00:00:00.000Z",
        )
        .unwrap();
        let mut config = crate::profile::config_for(&conn, &profile_id).unwrap();
        for kind in &mut config.work_kinds {
            kind.folder = Some("{kind}s/{id}".into());
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![profile_id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        // Made, and nothing on disk: no graveyard of empty folders.
        let empty = crate::fixtures::song(&conn, &profile_id, "Never opened");
        let opened = crate::fixtures::song(&conn, &profile_id, "Opened and left");
        let filled = crate::fixtures::song(&conn, &profile_id, "Holds a take");
        let songs = dir.path().join("songs");
        assert!(!songs.exists(), "making a work makes no folder");
        assert_eq!(standing(&conn, &empty.id), None);

        // Needed: opened.
        for work in [&opened, &filled] {
            assert_eq!(
                ensure(&conn, &config, work).unwrap().state,
                FolderState::Found
            );
        }
        let opened_at = songs.join(&opened.id);
        let filled_at = songs.join(&filled.id);
        assert_eq!(standing(&conn, &opened.id), Some(opened_at.clone()));
        // Empty folders below do not make a folder hold anything; a file
        // three folders down does.
        std::fs::create_dir_all(opened_at.join("raw").join("old")).unwrap();
        std::fs::create_dir_all(filled_at.join("scenes").join("take 1")).unwrap();
        std::fs::write(
            filled_at.join("scenes").join("take 1").join("v1.mp4"),
            b"take",
        )
        .unwrap();

        for work in [&empty, &opened, &filled] {
            crate::trash::discard(&conn, crate::trash::Entity::Work, &work.id).unwrap();
        }
        assert!(!opened_at.exists(), "an empty folder goes with its work");
        assert!(
            filled_at
                .join("scenes")
                .join("take 1")
                .join("v1.mp4")
                .is_file(),
            "a folder with a file in it is never removed"
        );
        assert!(songs.is_dir(), "and nothing above the work's folder is");

        // Emptying the trash does not reach the disk either.
        crate::trash::empty(&conn, &profile_id).unwrap();
        assert!(
            filled_at
                .join("scenes")
                .join("take 1")
                .join("v1.mp4")
                .is_file()
        );
    }

    /// The top of a work's folder is for what is finished: `*.mp4` reads it
    /// and never a folder below, so a newer take in `scenes/` or `raw/` is
    /// not taken for the finished clip (FR-04).
    #[test]
    fn a_take_in_a_subfolder_is_never_the_finished_file() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path();
        std::fs::write(folder.join("final.mp4"), b"final").unwrap();
        for below in ["raw", "scenes", "images", "drafts", "candidates"] {
            std::fs::create_dir_all(folder.join(below)).unwrap();
            std::fs::write(folder.join(below).join("newer.mp4"), b"take").unwrap();
        }
        // Written after the finished one, so "newest" alone would pick one.
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
        for below in ["raw", "scenes"] {
            let take = std::fs::File::options()
                .write(true)
                .open(folder.join(below).join("newer.mp4"))
                .unwrap();
            take.set_modified(later).unwrap();
        }

        let steps = steps_of("*.mp4").unwrap();
        let (_, relative) = newest(folder, &steps).unwrap();
        assert_eq!(relative, "final.mp4");
    }

    /// A text is read for the viewer only where kilna looks, only when it is
    /// one, and only so much of it (FR-14).
    #[test]
    fn a_text_is_read_under_the_media_folder_and_nowhere_else() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let places = [dir.path().to_path_buf()];
        let lyric = dir.path().join("lyrics.txt");
        std::fs::write(&lyric, "Tide\nturns").unwrap();

        let read = read_text(&lyric, &places).unwrap();
        assert_eq!(
            (read.text.as_str(), read.truncated, read.size),
            ("Tide\nturns", false, 10)
        );

        // Not UTF-8: shown, with the mark where a letter could not be read.
        let old = dir.path().join("subs.srt");
        std::fs::write(&old, [b'a', 0xFF, b'b']).unwrap();
        assert_eq!(read_text(&old, &places).unwrap().text, "a\u{FFFD}b");

        // Longer than the viewer shows: the beginning, and that there is more.
        let long = dir.path().join("week.log");
        std::fs::write(&long, vec![b'x'; usize::try_from(MOST_TEXT).unwrap() + 10]).unwrap();
        let read = read_text(&long, &places).unwrap();
        assert!(read.truncated);
        assert_eq!(read.text.len() as u64, MOST_TEXT);
        assert_eq!(read.size, MOST_TEXT + 10);

        let refused = |path: &Path| {
            read_text(path, &places)
                .unwrap_err()
                .refusal()
                .map(|refusal| refusal.code)
        };
        // Beside the media folder, a path that climbs out of it, and a file
        // that is not a text.
        let elsewhere = outside.path().join("secret.txt");
        std::fs::write(&elsewhere, "no").unwrap();
        assert_eq!(refused(&elsewhere), Some("folder.notReadable"));
        assert_eq!(
            refused(&dir.path().join("..").join("secret.txt")),
            Some("folder.notReadable")
        );
        let program = dir.path().join("run.exe");
        std::fs::write(&program, "MZ").unwrap();
        assert_eq!(refused(&program), Some("folder.notReadable"));
        assert_eq!(
            refused(&dir.path().join("gone.txt")),
            Some("folder.readFailed")
        );
    }

    /// A file of the folder is renamed where it lies, to a name and not a
    /// place, and never onto another file (FR-14).
    #[test]
    fn a_file_is_renamed_in_its_folder_and_never_onto_another() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path();
        std::fs::create_dir_all(folder.join("scenes")).unwrap();
        std::fs::write(folder.join("scenes").join("take 1.mp4"), b"one").unwrap();
        std::fs::write(folder.join("scenes").join("take 2.mp4"), b"two").unwrap();
        std::fs::write(folder.join("final.mp4"), b"final").unwrap();

        assert_eq!(
            rename_file(folder, "scenes/take 1.mp4", "  opening.mp4 ").unwrap(),
            "scenes/opening.mp4"
        );
        assert!(folder.join("scenes").join("opening.mp4").is_file());
        assert!(!folder.join("scenes").join("take 1.mp4").exists());
        assert_eq!(
            rename_file(folder, "final.mp4", "Final.mp4").unwrap(),
            "Final.mp4"
        );
        // To the name it has: nothing to do, and nothing wrong.
        assert_eq!(
            rename_file(folder, "Final.mp4", "Final.mp4").unwrap(),
            "Final.mp4"
        );

        let refused = |relative: &str, name: &str| {
            rename_file(folder, relative, name)
                .unwrap_err()
                .refusal()
                .map(|refusal| refusal.code)
        };
        // Onto the take beside it: both stay as they are.
        assert_eq!(
            refused("scenes/opening.mp4", "take 2.mp4"),
            Some("folder.nameTaken")
        );
        assert_eq!(
            std::fs::read(folder.join("scenes").join("take 2.mp4")).unwrap(),
            b"two"
        );
        for name in [
            "",
            "  ",
            "a/b.mp4",
            "..\\up.mp4",
            "what?.mp4",
            "CON.mp4",
            "ends.",
            "..",
        ] {
            assert_eq!(
                refused("scenes/opening.mp4", name),
                Some("folder.badName"),
                "{name:?}"
            );
        }
        assert_eq!(
            refused("scenes/gone.mp4", "x.mp4"),
            Some("folder.fileMissing")
        );
        assert_eq!(
            refused("../outside.mp4", "x.mp4"),
            Some("folder.fileOutside")
        );
        assert_eq!(refused("scenes", "x"), Some("folder.fileMissing"));
    }

    /// A copy is saved of what kilna looks at, and of nothing else.
    #[test]
    fn a_copy_is_saved_only_of_a_file_under_the_media_folder() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let places = [dir.path().to_path_buf()];
        let mix = dir.path().join("mix.wav");
        std::fs::write(&mix, b"RIFF").unwrap();
        let to = outside.path().join("copy.wav");

        save_copy(&mix, &places, &to).unwrap();
        assert_eq!(std::fs::read(&to).unwrap(), b"RIFF");
        assert!(mix.is_file(), "a copy: the file stays where it was");

        let refused = save_copy(&to, &places, &dir.path().join("in.wav")).unwrap_err();
        assert_eq!(
            refused.refusal().map(|refusal| refusal.code),
            Some("folder.notReadable")
        );
    }

    /// `{n}` reads the work's number among its kind made from the same
    /// origin: the second short of a song lies in `shorts/v2`, and a work
    /// with no number says the placeholder it could not fill.
    #[test]
    fn a_number_names_a_folder_of_its_own() {
        let root = Path::new("/media");
        let song = work("Tide", json!({}));
        let mut short = work("Tide (short 2)", json!({}));
        short.ordinal = Some(2);

        assert_eq!(
            resolve(root, "songs/{origin.title}/shorts/v{n}", &short, &song).unwrap(),
            at(root, &["songs", "Tide", "shorts", "v2"])
        );
        short.ordinal = None;
        assert_eq!(
            resolve(root, "songs/{origin.title}/shorts/v{n}", &short, &song).unwrap(),
            Resolved::Unfilled("n".into()),
            "a hole in the path would show the shorts folder of every short"
        );
    }

    /// A step of a rule holds `*` and `?`, whatever the case of the name.
    #[test]
    fn a_rule_step_matches_by_stars_and_marks() {
        assert!(step_matches("*.mp4", "v1.mp4"));
        assert!(
            step_matches("*.mp4", "TAKE 2.MP4"),
            "case is not a difference"
        );
        assert!(step_matches("v*", "v12"));
        assert!(step_matches("v?", "v3"));
        assert!(!step_matches("v?", "v12"), "one mark is one character");
        assert!(!step_matches("*.mp4", "v1.mp4.part"));
        assert!(step_matches("*a*b*", "xxaxxbxx"));
        assert!(!step_matches("*a*b", "xxaxxbxxc"));
        assert!(step_matches("*", ""));
    }

    /// A rule names a file inside the work's folder and nothing else.
    #[test]
    fn a_rule_that_leaves_the_folder_is_refused() {
        assert_eq!(rule_problem("v*/video/*.mp4"), None);
        for rule in [
            "",
            "/video/*.mp4",
            "C:/video/*.mp4",
            "../*.mp4",
            "video//*.mp4",
            "a|b",
        ] {
            assert!(rule_problem(rule).is_some(), "{rule:?} is refused");
        }
        assert_eq!(inside(r"v1\video\v2.mp4").unwrap(), "v1/video/v2.mp4");
        for path in ["", "/v1.mp4", r"C:\v1.mp4", "../v1.mp4", "v1/./v2.mp4"] {
            assert_eq!(
                inside(path).unwrap_err().refusal().map(|r| r.code),
                Some("folder.fileOutside"),
                "{path:?} is no file of the folder"
            );
        }
    }

    /// The finished file: the newest the rule matches - only where the rule
    /// says, never an animated scene beside it - and a file chosen by hand
    /// over the rule, which is not replaced by a guess when it goes.
    #[test]
    fn the_finished_file_is_the_newest_the_rule_matches_or_the_one_chosen() {
        let (conn, profile_id) = crate::fixtures::workspace();
        let dir = tempfile::tempdir().unwrap();
        set_root(
            &conn,
            &profile_id,
            Some(&dir.path().to_string_lossy()),
            "2030-01-01T00:00:00.000Z",
        )
        .unwrap();
        let mut config = crate::profile::config_for(&conn, &profile_id).unwrap();
        for kind in &mut config.work_kinds {
            kind.folder = Some("clips/{title}".into());
            kind.file = Some("v*/video/*.mp4".into());
        }
        let mut clip = crate::fixtures::video(&conn, &profile_id, "Harbour lights");
        let folder = dir.path().join("clips").join("Harbour lights");

        assert_eq!(
            finished(&conn, &config, &clip).unwrap().state,
            FileState::NoFolder
        );
        std::fs::create_dir_all(folder.join("v1/video/hd")).unwrap();
        std::fs::create_dir_all(folder.join("v2/video")).unwrap();
        assert_eq!(
            finished(&conn, &config, &clip).unwrap().state,
            FileState::NotYet
        );

        let write = |relative: &str, minutes_ago: u64| {
            let path = folder.join(relative);
            std::fs::write(&path, b"not really an mp4").unwrap();
            let at =
                std::time::SystemTime::now() - std::time::Duration::from_secs(minutes_ago * 60);
            std::fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(at)
                .unwrap();
        };
        write("v1/video/v9.mp4", 30);
        write("v1/video/v2.mp4", 20);
        write("v1/video/hd/scene-4.mp4", 1);
        write("v2/video/notes.txt", 1);

        let found = finished(&conn, &config, &clip).unwrap();
        assert_eq!(found.state, FileState::Found);
        assert_eq!(
            found.relative.as_deref(),
            Some("v1/video/v2.mp4"),
            "the newest render - not the greatest name, nor the scene animated a minute ago one step deeper"
        );

        clip.file = Some("v1/video/v9.mp4".into());
        let chosen = finished(&conn, &config, &clip).unwrap();
        assert_eq!(chosen.state, FileState::Chosen);
        assert_eq!(chosen.relative.as_deref(), Some("v1/video/v9.mp4"));

        std::fs::remove_file(folder.join("v1/video/v9.mp4")).unwrap();
        let gone = finished(&conn, &config, &clip).unwrap();
        assert_eq!(gone.state, FileState::ChosenMissing);
        assert_eq!(gone.path, None, "the rule does not stand in for a choice");

        clip.file = None;
        for kind in &mut config.work_kinds {
            kind.file = None;
        }
        assert_eq!(
            finished(&conn, &config, &clip).unwrap().state,
            FileState::NoRule
        );
    }
}
