//! Types of space (v0.112, ADR 0079): a profile kept as what a new space is
//! made of, and a profile as a document that travels in a file.
//!
//! A type is a profile's document under a name, kept in the application's
//! database. It is made by saving the profile of the open space, or by
//! reading a document from a file; a space made from it starts with that
//! vocabulary rather than with the shipped one. The document a type holds and
//! the document a file holds are one shape - the shape kilna's own profiles
//! ship in - so a profile exported from one machine is a type on another.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::profile::config::ProfileConfig;
use crate::time::now;

/// A profile as a document: what a file holds, and what a type is made of.
///
/// The shape of a shipped profile's file, so the four crafts kilna ships, a
/// profile exported from a space and one written by hand are read by one
/// parser - which also brings a document written in an older format to the
/// current one (`RawProfileConfig`).
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct ProfileDocument {
    /// The shipped profile the document grew from. A key kilna does not
    /// ship - a document from a build that shipped another craft, or one
    /// written by hand - grows from the empty type.
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub config: ProfileConfig,
}

impl ProfileDocument {
    /// The document as the text of its file: indented, ending in a newline,
    /// its words as they are rather than escaped.
    pub fn to_text(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)? + "\n")
    }

    /// The shipped kind a space made of this document is: its own key when
    /// kilna ships it, the empty type's otherwise.
    pub fn kind(&self) -> Result<String> {
        let shipped = crate::profile::builtin()?;
        Ok(
            if shipped.iter().any(|profile| profile.key == self.key.trim()) {
                self.key.trim().to_owned()
            } else {
                crate::profile::BLANK_KIND.to_owned()
            },
        )
    }

    /// The document, held to what a profile must be: a name, and a
    /// vocabulary with nothing wrong in it. Refused whole, every problem
    /// named, the way a profile saved from the editor is.
    pub fn checked(mut self) -> Result<Self> {
        self.name = super::checked_name(&self.name)?;
        self.description = self
            .description
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty());
        crate::profile::check(&self.config)?;
        Ok(self)
    }
}

/// Read a profile document from a file a person picked.
///
/// A file that is not there, is not JSON or is not a profile is refused with
/// the reason the parser gave: a document written by hand is fixed by reading
/// where it broke.
pub fn read_document(path: &Path) -> Result<ProfileDocument> {
    let unreadable = |cause: String| {
        Error::refused("profile.unreadable")
            .param("path", path.display().to_string())
            .param("cause", cause)
    };
    let raw = std::fs::read_to_string(path).map_err(|cause| unreadable(cause.to_string()))?;
    let document: ProfileDocument = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|cause| unreadable(cause.to_string()))?;
    document.checked()
}

/// A type as the window lists it: what it is called and what it grew from.
/// The document itself is read only when a space is made of it or it goes
/// out to a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct SpaceType {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// The shipped profile it grew from, and so the kind of a space made of
    /// it.
    pub kind: String,
    /// How many kinds of work it names, for the line under its name.
    pub work_kinds: usize,
    pub created_at: String,
}

/// Every type this machine keeps, in the order they were made.
pub fn list(app: &Connection) -> Result<Vec<SpaceType>> {
    let mut statement = app.prepare(
        "SELECT id, name, description, kind, config, created_at
           FROM space_type ORDER BY created_at, rowid",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|(id, name, description, kind, config, created_at)| {
            let config: ProfileConfig = serde_json::from_str(&config)?;
            Ok(SpaceType {
                id,
                name,
                description,
                kind,
                work_kinds: config.work_kinds.len(),
                created_at,
            })
        })
        .collect()
}

/// Keep a document as a type. The same document kept twice is two types:
/// a type is named by the person, and two names are two things to pick from.
pub fn add(app: &Connection, document: ProfileDocument) -> Result<SpaceType> {
    let document = document.checked()?;
    let kind = document.kind()?;
    let id = uuid::Uuid::new_v4().to_string();
    app.execute(
        "INSERT INTO space_type (id, name, description, kind, config, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            document.name,
            document.description,
            kind,
            serde_json::to_string(&document.config)?,
            now(),
        ],
    )?;
    get(app, &id)
}

/// One type, as the window lists it.
pub fn get(app: &Connection, id: &str) -> Result<SpaceType> {
    list(app)?
        .into_iter()
        .find(|known| known.id == id)
        .ok_or_else(|| Error::not_found("space type", id))
}

/// The document a type holds: what a space is made of, and what goes out to
/// a file.
pub fn document(app: &Connection, id: &str) -> Result<ProfileDocument> {
    let row: Option<(String, Option<String>, String, String)> = app
        .query_row(
            "SELECT name, description, kind, config FROM space_type WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    let (name, description, kind, config) =
        row.ok_or_else(|| Error::not_found("space type", id))?;
    Ok(ProfileDocument {
        key: kind,
        name,
        description,
        config: serde_json::from_str(&config)?,
    })
}

/// Call a type something else.
pub fn rename(app: &Connection, id: &str, name: &str) -> Result<SpaceType> {
    let name = super::checked_name(name)?;
    let changed = app.execute(
        "UPDATE space_type SET name = ?2 WHERE id = ?1",
        params![id, name],
    )?;
    if changed == 0 {
        return Err(Error::not_found("space type", id));
    }
    get(app, id)
}

/// Let a type go. The spaces made of it keep what they hold: each has had
/// its own copy since it was made.
pub fn delete(app: &Connection, id: &str) -> Result<()> {
    let changed = app.execute("DELETE FROM space_type WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(Error::not_found("space type", id));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::space::{self, NewSpace, SPACES_DIR};

    fn studio() -> ProfileDocument {
        let shipped = crate::profile::builtin()
            .unwrap()
            .into_iter()
            .find(|profile| profile.key == "music")
            .unwrap();
        ProfileDocument {
            key: shipped.key,
            name: "Night Shift".into(),
            description: Some("  Songs of the second channel.  ".into()),
            config: shipped.config,
        }
    }

    #[test]
    fn a_document_kept_as_a_type_is_listed_and_read_back() {
        let app = crate::db::open_app_in_memory().unwrap();
        let mut document = studio();
        document.config.work_kinds[0].label = "Track".into();

        let kept = add(&app, document).unwrap();

        assert_eq!(kept.name, "Night Shift");
        assert_eq!(kept.kind, "music");
        assert_eq!(
            kept.description.as_deref(),
            Some("Songs of the second channel.")
        );
        assert_eq!(list(&app).unwrap(), vec![kept.clone()]);
        let read = self::document(&app, &kept.id).unwrap();
        assert_eq!(read.config.work_kinds[0].label, "Track");
    }

    #[test]
    fn a_document_with_a_broken_vocabulary_is_not_kept() {
        let app = crate::db::open_app_in_memory().unwrap();
        let mut document = studio();
        document.config.work_kinds[0].statuses.clear();

        let refused = add(&app, document).unwrap_err();

        assert_eq!(refused.refusal().unwrap().code, "profile.invalid");
        assert!(list(&app).unwrap().is_empty());
    }

    #[test]
    fn a_document_of_a_craft_kilna_does_not_ship_grows_from_the_empty_type() {
        let app = crate::db::open_app_in_memory().unwrap();
        let mut document = studio();
        document.key = "opera".into();

        assert_eq!(
            add(&app, document).unwrap().kind,
            crate::profile::BLANK_KIND
        );
    }

    #[test]
    fn a_space_made_of_a_type_starts_with_its_vocabulary_and_is_of_its_kind() {
        let dir = tempfile::tempdir().unwrap();
        let app = crate::db::open_app_in_memory().unwrap();
        let mut document = studio();
        document.config.work_kinds[0].label = "Track".into();
        document
            .config
            .prompts
            .retain(|prompt| prompt.key != "titles");
        let kept = add(&app, document).unwrap();

        let made = space::create(
            &app,
            dir.path(),
            &NewSpace {
                name: "Second channel".into(),
                kind: "blog".into(),
                colour: None,
                mark: None,
                type_id: Some(kept.id),
            },
            None,
        )
        .unwrap();

        assert_eq!(
            made.kind.as_deref(),
            Some("music"),
            "the kind is the type's, whatever the request named"
        );
        let folder = dir.path().join(SPACES_DIR).join("second-channel");
        let conn = crate::db::open(&folder.join(crate::db::DATABASE_FILE)).unwrap();
        let profiles = crate::profile::list(&conn).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].key, "music");
        assert_eq!(profiles[0].name, "Night Shift");
        assert!(profiles[0].is_active);
        assert_eq!(profiles[0].config.work_kinds[0].label, "Track");
        assert!(
            profiles[0]
                .config
                .prompts
                .iter()
                .any(|prompt| prompt.key == "titles"),
            "a type is carried forward as its kind is: what it lacks of the shipped profile arrives"
        );
    }

    #[test]
    fn a_document_goes_out_to_a_file_and_comes_back_the_same() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("night-shift.kilna-profile.json");
        let document = studio().checked().unwrap();

        std::fs::write(&path, document.to_text().unwrap()).unwrap();
        let read = read_document(&path).unwrap();

        assert_eq!(read.name, document.name);
        assert_eq!(
            serde_json::to_value(&read.config).unwrap(),
            serde_json::to_value(&document.config).unwrap()
        );
    }

    #[test]
    fn a_file_that_is_not_a_profile_says_where_it_broke() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        std::fs::write(&path, r#"{ "name": "Half a document" }"#).unwrap();

        let refused = read_document(&path).unwrap_err();

        assert_eq!(refused.refusal().unwrap().code, "profile.unreadable");
        assert!(
            read_document(&dir.path().join("nowhere.json"))
                .unwrap_err()
                .refusal()
                .is_some()
        );
    }

    #[test]
    fn a_type_is_renamed_and_let_go_without_touching_a_space() {
        let app = crate::db::open_app_in_memory().unwrap();
        let kept = add(&app, studio()).unwrap();

        assert_eq!(
            rename(&app, &kept.id, " Late Shift ").unwrap().name,
            "Late Shift"
        );
        assert!(rename(&app, &kept.id, "  ").is_err());
        delete(&app, &kept.id).unwrap();
        assert!(list(&app).unwrap().is_empty());
        assert!(delete(&app, &kept.id).is_err());
    }
}
