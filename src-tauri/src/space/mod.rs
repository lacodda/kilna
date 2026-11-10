//! Spaces (v0.109, ADR 0077): a space is a folder with a database of its own.
//!
//! Until v0.109 one database held every profile, and switching profiles
//! switched which works the window showed. Two channels of the same craft could
//! not be kept apart - the register of repeats, the canon and the calendar of
//! one would have read the other's - and a backup or a move was always all of
//! it. Now a space is a folder:
//!
//! ```text
//! <folder>/
//!   space.kilna   what the space is: its name, its kind, its colour and mark
//!   kilna.db      its works, versions, scores, releases, notes, canon...
//!   media/        the files its rows name, by paths relative to the folder
//!   assistant/    the empty directory its runs start in
//!   kilna.lock    while a window holds it open
//! ```
//!
//! The application keeps, beside its spaces and never inside one, a database of
//! its own (`app.db`): where the folders are, which was opened last, and the
//! settings that are the person's whatever space is open (the assistant's).
//! Backing a space up, or moving it to another machine, is copying its folder;
//! double-clicking its `space.kilna` opens it.

pub mod legacy;
pub mod lock;
pub mod registry;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// The file that says what a space is, in its folder. Its extension is the
/// one the installer ties to kilna, so double-clicking it opens the space.
pub const MANIFEST_FILE: &str = "space.kilna";

/// Where a new space's folder goes, under the application's data directory.
pub const SPACES_DIR: &str = "spaces";

/// The manifest's own format, read before anything else in it.
pub const FORMAT: u32 = 1;

/// The longest name a space may carry, in characters.
const NAME_LIMIT: usize = 80;

/// The marker next to the executable that makes a copy of kilna portable: its
/// data, and the spaces it makes, live beside it (#95).
pub const PORTABLE_MARKER: &str = "kilna.portable";

/// The folder of a portable copy's data, beside the executable.
const PORTABLE_DATA: &str = "data";

/// What a space says about itself, in its folder.
///
/// The name, the colour and the mark are the space's own, not the machine's,
/// so they travel with the folder: a space copied to another machine is called
/// what it was called here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u32,
    pub id: String,
    pub name: String,
    /// The shipped profile the space was made from: `music` (Studio),
    /// `blog`, `novel`, `podcast`. A fact of its making - the vocabulary
    /// itself lives in its database and may be edited there.
    pub kind: String,
    /// A colour as `#RRGGBB`; the kind's own when none was chosen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<String>,
    /// The name of a glyph from the window's set; the kind's own when none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<String>,
}

impl Manifest {
    /// A new space's manifest, with an id of its own.
    pub fn new(name: &str, kind: &str) -> Result<Self> {
        Ok(Self {
            format: FORMAT,
            id: uuid::Uuid::new_v4().to_string(),
            name: checked_name(name)?,
            kind: checked_kind(kind)?,
            colour: None,
            mark: None,
        })
    }

    /// The manifest of the space at `folder`.
    pub fn read(folder: &Path) -> Result<Self> {
        let path = folder.join(MANIFEST_FILE);
        let raw = std::fs::read_to_string(&path).map_err(|cause| {
            Error::refused("space.unreadable")
                .param("path", folder.display().to_string())
                .param("cause", cause.to_string())
        })?;
        let manifest: Self = serde_json::from_str(&raw).map_err(|cause| {
            Error::refused("space.unreadable")
                .param("path", folder.display().to_string())
                .param("cause", cause.to_string())
        })?;
        if manifest.format > FORMAT {
            return Err(Error::refused("space.tooNew")
                .param("path", folder.display().to_string())
                .param("format", manifest.format));
        }
        Ok(manifest)
    }

    /// Write the manifest into `folder`, whole or not at all: to a file
    /// beside it first, then over it, so a crash leaves the old one or the new.
    pub fn write(&self, folder: &Path) -> Result<()> {
        std::fs::create_dir_all(folder)?;
        let path = folder.join(MANIFEST_FILE);
        let draft = folder.join(format!("{MANIFEST_FILE}.new"));
        std::fs::write(&draft, serde_json::to_string_pretty(self)? + "\n")?;
        std::fs::rename(&draft, &path)?;
        Ok(())
    }
}

/// A space as the window lists it.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Space {
    pub id: String,
    /// The space's own name - or its folder's, when its file cannot be read.
    pub name: String,
    /// The kind it was made from; none when its file cannot be read.
    pub kind: Option<String>,
    pub colour: Option<String>,
    pub mark: Option<String>,
    /// Its folder, whole, as this machine reaches it.
    pub path: String,
    pub opened_at: Option<String>,
    pub trashed_at: Option<String>,
    /// The name was given by kilna; the window asks for one, once.
    pub ask_name: bool,
    /// Whether its folder and its file can be read now - a space on a drive
    /// that is not plugged in is listed, and cannot be opened.
    pub reachable: bool,
    /// The one the window has open.
    pub open: bool,
}

/// What a person gives a new space.
#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
pub struct NewSpace {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub colour: Option<String>,
    #[serde(default)]
    pub mark: Option<String>,
}

/// What changes in how a space looks.
#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
pub struct SpaceLook {
    pub colour: Option<String>,
    pub mark: Option<String>,
}

/// The paths of a space, from its folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder(PathBuf);

impl Folder {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn database(&self) -> PathBuf {
        self.0.join(crate::db::DATABASE_FILE)
    }

    pub fn manifest(&self) -> PathBuf {
        self.0.join(MANIFEST_FILE)
    }

    /// The folder a `space.kilna` file stands in - what a double-click hands
    /// kilna - or the folder itself when given one.
    pub fn of(path: &Path) -> Self {
        if path.file_name().is_some_and(|name| name == MANIFEST_FILE) {
            Self(
                path.parent()
                    .map_or_else(|| path.to_path_buf(), Path::to_path_buf),
            )
        } else {
            Self(path.to_path_buf())
        }
    }
}

/// Where the application keeps its own data on this machine: next to the
/// executable when a `kilna.portable` file stands there, the per-user data
/// directory otherwise.
pub fn app_dir() -> Result<PathBuf> {
    if let Some(dir) = portable_dir() {
        return Ok(dir);
    }
    crate::db::default_data_dir()
}

/// The portable data directory, when this copy is portable.
pub fn portable_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let beside = exe.parent()?;
    beside
        .join(PORTABLE_MARKER)
        .is_file()
        .then(|| beside.join(PORTABLE_DATA))
}

/// Make a new space: its folder under `parent` (the application's `spaces/`
/// when none is given), its file, and its database seeded with the kind's
/// profile. Registered, not opened.
pub fn create(
    app: &rusqlite::Connection,
    app_dir: &Path,
    new: &NewSpace,
    parent: Option<&Path>,
) -> Result<Space> {
    let mut manifest = Manifest::new(&new.name, &new.kind)?;
    manifest.colour = checked_colour(new.colour.as_deref())?;
    manifest.mark = checked_mark(new.mark.as_deref())?;

    let parent = parent.map_or_else(|| app_dir.join(SPACES_DIR), Path::to_path_buf);
    let folder = Folder::new(free_folder(&parent, &manifest.name));
    std::fs::create_dir_all(folder.path())?;
    manifest.write(folder.path())?;
    let conn = crate::db::open(&folder.database())?;
    crate::profile::seed(&conn, &manifest.kind)?;
    crate::style_set::seed(&conn)?;
    drop(conn);

    registry::add(app, app_dir, &manifest.id, folder.path())?;
    registry::get(app, app_dir, &manifest.id, None)
}

/// The databases of every other space this machine reaches, for reading only
/// and as they stand - nothing migrated, nothing seeded: what the week's runs
/// cost is counted across them all (one wallet), and a space that cannot be
/// opened is left out of the count.
pub fn others_to_count(
    app: &rusqlite::Connection,
    app_dir: &Path,
    except: &str,
) -> Vec<rusqlite::Connection> {
    let Ok(spaces) = registry::list(app, app_dir, None) else {
        return Vec::new();
    };
    spaces
        .into_iter()
        .filter(|space| space.id != except && space.reachable)
        .filter_map(|space| {
            rusqlite::Connection::open_with_flags(
                Path::new(&space.path).join(crate::db::DATABASE_FILE),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .ok()
        })
        .collect()
}

/// Give a space another name, in its own file.
pub fn rename(folder: &Folder, name: &str) -> Result<Manifest> {
    let mut manifest = Manifest::read(folder.path())?;
    manifest.name = checked_name(name)?;
    manifest.write(folder.path())?;
    Ok(manifest)
}

/// Give a space another colour or mark, in its own file.
pub fn restyle(folder: &Folder, look: &SpaceLook) -> Result<Manifest> {
    let mut manifest = Manifest::read(folder.path())?;
    manifest.colour = checked_colour(look.colour.as_deref())?;
    manifest.mark = checked_mark(look.mark.as_deref())?;
    manifest.write(folder.path())?;
    Ok(manifest)
}

/// A name a space may carry: trimmed, not empty, not longer than a line.
pub fn checked_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::refused("space.noName"));
    }
    if name.chars().count() > NAME_LIMIT || name.chars().any(char::is_control) {
        return Err(Error::refused("space.badName").param("limit", NAME_LIMIT));
    }
    Ok(name.to_owned())
}

/// A kind a space may be made from: a profile kilna ships.
fn checked_kind(kind: &str) -> Result<String> {
    let shipped = crate::profile::builtin()?;
    if shipped.iter().any(|profile| profile.key == kind) {
        return Ok(kind.to_owned());
    }
    Err(Error::refused("space.unknownKind")
        .param("kind", kind)
        .param(
            "known",
            shipped
                .iter()
                .map(|profile| profile.key.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        ))
}

/// A colour as `#RRGGBB`, or none.
fn checked_colour(colour: Option<&str>) -> Result<Option<String>> {
    match colour.map(str::trim).filter(|c| !c.is_empty()) {
        None => Ok(None),
        Some(colour)
            if colour.len() == 7
                && colour.starts_with('#')
                && colour[1..].chars().all(|c| c.is_ascii_hexdigit()) =>
        {
            Ok(Some(colour.to_ascii_uppercase()))
        }
        Some(colour) => Err(Error::refused("space.badColour").param("colour", colour)),
    }
}

/// A glyph's name - lowercase letters, digits and dashes - or none.
fn checked_mark(mark: Option<&str>) -> Result<Option<String>> {
    match mark.map(str::trim).filter(|m| !m.is_empty()) {
        None => Ok(None),
        Some(mark)
            if mark.len() <= 32
                && mark
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') =>
        {
            Ok(Some(mark.to_owned()))
        }
        Some(mark) => Err(Error::refused("space.badMark").param("mark", mark)),
    }
}

/// A folder under `parent` named after `name` that is not taken yet.
pub fn free_folder(parent: &Path, name: &str) -> PathBuf {
    let base = folder_name(name);
    let mut candidate = parent.join(&base);
    let mut n = 2;
    while candidate.exists() {
        candidate = parent.join(format!("{base}-{n}"));
        n += 1;
    }
    candidate
}

/// A folder's name for a space's: its letters and digits, lowercase, the rest
/// dashes - a name any file system takes, in any script.
fn folder_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
        if out.chars().count() >= 48 {
            break;
        }
    }
    let out = out.trim_matches('-').to_owned();
    if out.is_empty() {
        "space".to_owned()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_is_named_after_the_space_in_any_script() {
        assert_eq!(folder_name("Night Shift"), "night-shift");
        assert_eq!(folder_name("Ночная смена!"), "ночная-смена");
        assert_eq!(folder_name("  ...  "), "space");
        assert_eq!(folder_name("a/b\\c:d"), "a-b-c-d");
    }

    #[test]
    fn a_taken_folder_is_not_reused() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("blog")).unwrap();
        std::fs::create_dir(dir.path().join("blog-2")).unwrap();

        assert_eq!(free_folder(dir.path(), "Blog"), dir.path().join("blog-3"));
    }

    #[test]
    fn a_manifest_is_written_whole_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let mut manifest = Manifest::new("Night Shift", "music").unwrap();
        manifest.colour = Some("#8A7DF5".into());

        manifest.write(dir.path()).unwrap();

        assert_eq!(Manifest::read(dir.path()).unwrap(), manifest);
        assert!(!dir.path().join("space.kilna.new").exists());
    }

    #[test]
    fn a_manifest_from_a_newer_kilna_is_refused_not_misread() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(MANIFEST_FILE),
            r#"{"format": 2, "id": "x", "name": "N", "kind": "music"}"#,
        )
        .unwrap();

        let refused = Manifest::read(dir.path()).unwrap_err();
        assert_eq!(refused.refusal().unwrap().code, "space.tooNew");
    }

    #[test]
    fn a_name_a_kind_a_colour_and_a_mark_are_held_to_their_shapes() {
        assert_eq!(checked_name("  Blog  ").unwrap(), "Blog");
        for bad in ["", "   ", "a\nb"] {
            assert!(checked_name(bad).is_err(), "{bad:?}");
        }
        assert!(checked_name(&"x".repeat(81)).is_err());
        assert!(checked_kind("music").is_ok());
        assert_eq!(
            checked_kind("poetry").unwrap_err().refusal().unwrap().code,
            "space.unknownKind"
        );
        assert_eq!(
            checked_colour(Some("#8a7df5")).unwrap().as_deref(),
            Some("#8A7DF5")
        );
        for bad in ["8A7DF5", "#8A7DF", "#GGGGGG", "red"] {
            assert!(checked_colour(Some(bad)).is_err(), "{bad}");
        }
        assert_eq!(checked_colour(Some(" ")).unwrap(), None);
        assert_eq!(
            checked_mark(Some("book-open")).unwrap().as_deref(),
            Some("book-open")
        );
        for bad in ["Book", "book open", "<svg>", &"a".repeat(33)] {
            assert!(checked_mark(Some(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_double_clicked_file_names_its_folder() {
        let folder = Path::new("spaces").join("blog");
        assert_eq!(Folder::of(&folder.join(MANIFEST_FILE)).path(), folder);
        assert_eq!(Folder::of(&folder).path(), folder);
    }

    #[test]
    fn a_space_is_made_with_its_kind_alone_and_registered() {
        let dir = tempfile::tempdir().unwrap();
        let app = crate::db::open_app_in_memory().unwrap();

        let space = create(
            &app,
            dir.path(),
            &NewSpace {
                name: "Second channel".into(),
                kind: "blog".into(),
                colour: Some("#3fa36b".into()),
                mark: Some("feather".into()),
            },
            None,
        )
        .unwrap();

        let folder = dir.path().join(SPACES_DIR).join("second-channel");
        assert_eq!(Path::new(&space.path), folder);
        assert_eq!(space.kind.as_deref(), Some("blog"));
        assert_eq!(space.colour.as_deref(), Some("#3FA36B"));
        assert!(space.reachable);
        let conn = crate::db::open(&folder.join(crate::db::DATABASE_FILE)).unwrap();
        let profiles = crate::profile::list(&conn).unwrap();
        assert_eq!(
            profiles.iter().map(|p| p.key.as_str()).collect::<Vec<_>>(),
            ["blog"],
            "a space holds its own kind, not every craft kilna ships"
        );
        assert!(profiles[0].is_active);
    }

    #[test]
    fn two_spaces_of_one_kind_are_two_folders() {
        let dir = tempfile::tempdir().unwrap();
        let app = crate::db::open_app_in_memory().unwrap();
        let new = NewSpace {
            name: "Studio".into(),
            kind: "music".into(),
            colour: None,
            mark: None,
        };

        let first = create(&app, dir.path(), &new, None).unwrap();
        let second = create(&app, dir.path(), &new, None).unwrap();

        assert_ne!(first.id, second.id);
        assert_ne!(first.path, second.path);
        assert_eq!(registry::list(&app, dir.path(), None).unwrap().len(), 2);
    }
}
