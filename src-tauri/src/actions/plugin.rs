//! What a plugin is given, and the gesture that keeps what it returns.
//!
//! Running a plugin is three steps with the workspace let go in the middle:
//! read the subject, run the executable - which may take as long as it likes -
//! and merge what it returned. The command holds the lock for the first and
//! the last and not for the wait; the two halves live here.

use std::path::Path;

use rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::error::{Error, Result};
use crate::folder::{FileState, WorkFolder};
use crate::plugin::{self, manifest::Target};
use crate::profile::config::ProfileConfig;
use crate::release::{self, ReleasePatch};
use crate::work::{self, Work, WorkPatch, version};

/// The row a plugin is run against, as it is sent.
///
/// A plugin acting on a work almost always wants its text, so a work travels
/// with the newest body of each role: sending only the row would make every
/// plugin ask for the body back. Since v0.101 it also travels with everything
/// a cutter needs, so no plugin has to ask for anything back (ADR 0070):
///
/// - `folder` - the work's folder on disk, when the template names one;
/// - `file` - its finished file, and how it was found;
/// - `shots` - the stretches it is spliced from, each beside the donor's
///   finished file;
/// - `donors` - what it was made from, each with its folder and file;
/// - `chain` - the titles up the chain "made from", nearest first, and
///   `origin` - what it is all made from (the song, for a short);
/// - `cover` - the picture it goes out under, when it has one.
pub fn subject(conn: &Connection, target: Target, id: &str) -> Result<Value> {
    match target {
        Target::Release => Ok(serde_json::to_value(
            release::get(conn, id)?.ok_or_else(|| Error::not_found("release", id))?,
        )?),
        Target::Work | Target::Cuts => {
            let found = work::get(conn, id)?.ok_or_else(|| Error::not_found("work", id))?;
            let config = crate::profile::config_for(conn, &found.profile_id)?;
            let mut value = serde_json::to_value(&found)?;
            let mut bodies = Map::new();
            for summary in version::list(conn, id)? {
                if bodies.contains_key(&summary.role) {
                    continue;
                }
                if let Some(full) = version::get(conn, &summary.id)? {
                    bodies.insert(summary.role.clone(), Value::String(full.body));
                }
            }

            let place = crate::folder::locate(conn, &config, &found)?;
            let mut donors = Vec::new();
            for link in crate::link::sources(conn, id)? {
                if link.role == crate::link::LAB {
                    continue;
                }
                let Some(donor) = work::get(conn, &link.source_id)? else {
                    continue;
                };
                let at = crate::folder::locate(conn, &config, &donor)?;
                donors.push(json!({
                    "id": donor.id,
                    "title": donor.title,
                    "kind": donor.kind,
                    "folder": folder_of(&at),
                    "file": at.file,
                }));
            }
            let mut chain = Vec::new();
            for ancestor in crate::link::ancestors(conn, id)? {
                if let Some(up) = work::get(conn, &ancestor)? {
                    chain.push(named(&up));
                }
            }
            let origin = crate::publication::origin(conn, &config, id)?;

            if let Some(object) = value.as_object_mut() {
                object.insert("bodies".into(), Value::Object(bodies));
                object.insert("folder".into(), folder_of(&place));
                object.insert("file".into(), serde_json::to_value(&place.file)?);
                object.insert(
                    "shots".into(),
                    serde_json::to_value(crate::cut::shot_list(conn, &config, id)?)?,
                );
                object.insert("donors".into(), Value::Array(donors));
                object.insert("chain".into(), Value::Array(chain));
                object.insert("origin".into(), origin.as_ref().map_or(Value::Null, named));
                object.insert(
                    "cover".into(),
                    crate::asset::cover_of(conn, id)?
                        .map_or(Value::Null, |asset| Value::String(asset.path)),
                );
            }
            Ok(value)
        }
    }
}

/// A work as the chain names it.
fn named(work: &Work) -> Value {
    json!({ "id": work.id, "title": work.title, "kind": work.kind })
}

/// The work's folder, when the template could name it - made or not, since a
/// cutter may be the one to make it.
fn folder_of(place: &WorkFolder) -> Value {
    place.path.clone().map_or(Value::Null, Value::String)
}

/// Keep what a plugin returned: merged into the fields as they are NOW.
///
/// Read after the plugin returned rather than taken from what it was sent: a
/// field edited while it ran would otherwise be put back to what it was when
/// it started. Written as the ordinary edit it is - through the log, with what
/// it replaced - so undo takes back what the plugin wrote. It used to write
/// past the log, and Ctrl+Z after a plugin took back the person's previous
/// edit instead.
///
/// A plugin can add and overwrite its own keys but cannot clear the rest —
/// losing unrelated metadata to a third-party integration is not recoverable.
pub fn keep(
    conn: &Connection,
    target: Target,
    id: &str,
    returned: &Map<String, Value>,
) -> Result<()> {
    if returned.is_empty() {
        return Ok(());
    }
    match target {
        Target::Release => {
            let before = release::get(conn, id)?.ok_or_else(|| Error::not_found("release", id))?;
            super::release::update(
                conn,
                id,
                ReleasePatch {
                    meta: Some(plugin::merge_meta(&before.meta, returned)),
                    ..ReleasePatch::default()
                },
            )?;
        }
        Target::Work | Target::Cuts => {
            // Only the plugin's own keys: a work's fields merge by key, so the
            // rest cannot be cleared and the undo takes back exactly what the
            // plugin wrote.
            super::work::update(
                conn,
                id,
                WorkPatch {
                    meta: Some(returned.clone()),
                    ..WorkPatch::default()
                },
            )?;
        }
    }
    Ok(())
}

/// Make the file a plugin says it made the work's finished file, when the
/// kind's rule would not find it by itself (ADR 0070).
///
/// The first file named that lies inside the work's folder is the one: a
/// cutter that also wrote a log beside it names the video first. When the rule
/// already finds that file - the ordinary case, a new `v2.mp4` beside `v1` -
/// nothing is written, so the next render made outside kilna is found too.
/// When the person had chosen a file by hand, or the rule finds another, the
/// new file becomes the choice, as the ordinary edit it is: undo takes it
/// back. A file outside the folder is the plugin's business and changes
/// nothing. Returns the file made that is now the work's finished file,
/// found by the rule or chosen for it - what the window says once a cut is
/// done.
pub fn keep_files(conn: &Connection, id: &str, files: &[String]) -> Result<Option<String>> {
    if files.is_empty() {
        return Ok(None);
    }
    let found = work::get(conn, id)?.ok_or_else(|| Error::not_found("work", id))?;
    let config: ProfileConfig = crate::profile::config_for(conn, &found.profile_id)?;
    let place = crate::folder::locate(conn, &config, &found)?;
    let Some(folder) = place.path.as_deref() else {
        return Ok(None);
    };
    let Some((made, relative)) = files.iter().find_map(|file| {
        inside_folder(Path::new(folder), Path::new(file)).map(|relative| (file.clone(), relative))
    }) else {
        return Ok(None);
    };
    let found_by_rule =
        place.file.state == FileState::Found && place.file.relative.as_deref() == Some(&relative);
    let chosen_already =
        place.file.state == FileState::Chosen && place.file.relative.as_deref() == Some(&relative);
    if found_by_rule || chosen_already {
        return Ok(Some(made));
    }
    super::work::update(
        conn,
        id,
        WorkPatch {
            file: Some(Some(relative)),
            ..WorkPatch::default()
        },
    )?;
    Ok(Some(made))
}

/// Where `file` lies inside `folder`, `/` between the steps; `None` when it
/// lies elsewhere or is not there.
fn inside_folder(folder: &Path, file: &Path) -> Option<String> {
    if !file.is_file() {
        return None;
    }
    let under = file.strip_prefix(folder).ok()?;
    let steps: Vec<String> = under
        .components()
        .map(|step| match step {
            std::path::Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Option<_>>()?;
    (!steps.is_empty()).then(|| steps.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    /// A short made from a clip made from a song, with a media folder: the
    /// shape a cutter is run on.
    fn a_family() -> (Connection, String, tempfile::TempDir, Work, Work, Work) {
        let (conn, profile_id) = fixtures::workspace();
        let root = tempfile::tempdir().unwrap();
        crate::folder::set_root(
            &conn,
            &profile_id,
            Some(&root.path().to_string_lossy()),
            "2026-10-26T00:00:00.000Z",
        )
        .unwrap();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        let clip = super::super::work::derive(&conn, &song.id, "video", None, None, None)
            .unwrap()
            .work;
        let short = super::super::work::derive(&conn, &clip.id, "short", None, None, None)
            .unwrap()
            .work;
        (conn, profile_id, root, song, clip, short)
    }

    /// A cutter is handed everything at once: the shot list with the donor's
    /// file, both folders, the chain up to the song and the cover.
    #[test]
    fn a_cutter_is_handed_everything_it_needs() {
        let (conn, profile_id, root, song, clip, short) = a_family();
        let clip_folder = root.path().join("songs").join("Tide").join("video");
        std::fs::create_dir_all(&clip_folder).unwrap();
        std::fs::write(clip_folder.join("v1.mp4"), b"not really an mp4").unwrap();
        crate::cut::create(
            &conn,
            &profile_id,
            crate::cut::NewCut {
                work_id: short.id.clone(),
                source_id: clip.id.clone(),
                starts_at: 12.0,
                ends_at: 20.0,
                position: None,
                label: None,
            },
        )
        .unwrap();

        let subject = subject(&conn, Target::Cuts, &short.id).unwrap();

        let shot = &subject["shots"][0];
        assert_eq!(shot["starts_at"], json!(12.0));
        assert_eq!(
            shot["path"],
            json!(clip_folder.join("v1.mp4").to_string_lossy()),
            "the donor's finished file beside the stretch"
        );
        assert_eq!(
            subject["folder"],
            json!(
                root.path()
                    .join("songs")
                    .join("Tide")
                    .join("shorts")
                    .join("1")
                    .to_string_lossy()
            ),
            "the short's own folder, numbered"
        );
        assert_eq!(subject["donors"][0]["title"], json!("Tide (video)"));
        assert_eq!(
            subject["donors"][0]["folder"],
            json!(clip_folder.to_string_lossy())
        );
        assert_eq!(subject["origin"]["title"], json!(song.title));
        assert_eq!(
            subject["chain"]
                .as_array()
                .unwrap()
                .iter()
                .map(|up| up["title"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["Tide (video)", "Tide"],
            "nearest first"
        );
        assert_eq!(subject["cover"], Value::Null, "no cover yet");
        assert_eq!(subject["ordinal"], json!(1));
    }

    /// A file the plugin made is chosen as the finished file only when the
    /// rule would not find it, and is the finished file either way; a file
    /// elsewhere changes nothing.
    #[test]
    fn a_made_file_becomes_the_finished_file_when_the_rule_would_miss_it() {
        let (conn, _profile_id, root, _song, _clip, short) = a_family();
        let folder = root
            .path()
            .join("songs")
            .join("Tide")
            .join("shorts")
            .join("1");
        std::fs::create_dir_all(folder.join("drafts")).unwrap();
        let by_rule = folder.join("v1.mp4");
        std::fs::write(&by_rule, b"not really an mp4").unwrap();

        let made = by_rule.to_string_lossy().into_owned();
        assert_eq!(
            keep_files(&conn, &short.id, std::slice::from_ref(&made)).unwrap(),
            Some(made),
            "the rule finds it by itself: it is the finished file"
        );
        assert_eq!(work::get(&conn, &short.id).unwrap().unwrap().file, None);

        let deeper = folder.join("drafts").join("cut.mp4");
        std::fs::write(&deeper, b"not really an mp4").unwrap();
        let log = folder.join("cut.log");
        std::fs::write(&log, b"").unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let note = elsewhere.path().join("note.txt");
        std::fs::write(&note, b"").unwrap();
        let made = deeper.to_string_lossy().into_owned();
        assert_eq!(
            keep_files(
                &conn,
                &short.id,
                &[
                    note.to_string_lossy().into_owned(),
                    made.clone(),
                    log.to_string_lossy().into_owned()
                ]
            )
            .unwrap(),
            Some(made),
            "the first file inside the folder is the one"
        );
        assert_eq!(
            work::get(&conn, &short.id)
                .unwrap()
                .unwrap()
                .file
                .as_deref(),
            Some("drafts/cut.mp4"),
            "a file the rule would not find is chosen"
        );

        let outside = elsewhere.path().join("cut.mp4");
        std::fs::write(&outside, b"not really an mp4").unwrap();
        assert_eq!(
            keep_files(&conn, &short.id, &[outside.to_string_lossy().into_owned()]).unwrap(),
            None
        );
    }
}
