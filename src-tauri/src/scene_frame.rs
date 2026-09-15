//! The pictures drawn for a scene, in order, and the one it is cut from.
//!
//! A generator answers a prompt with four pictures, not one, and choosing
//! between them *is* the work. So a scene keeps them all: four rows, an
//! order, and at most one of them marked as the scene's own.
//!
//! A frame points at an asset rather than repeating it. The bytes were
//! already copied into the workspace by `asset` (v0.67) and named by their
//! id there; this table says only whose frame that file is and where it
//! stands. What the schema adds over `scene_note` is what a link never
//! needed — a position, and a mark — and the mark is kept honest by a
//! partial unique index rather than by remembering to check.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::asset::{self, NewAsset};
use crate::error::{Error, Result};
use crate::minted::Minted;

/// What an asset is for when it arrives as a frame of a scene.
///
/// A word of the application, like `cover`: it decides what gets shown where,
/// not what the craft judges.
pub const FRAME: &str = "frame";

/// A picture drawn for a scene, as the board shows it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneFrame {
    pub id: String,
    pub scene_id: String,
    pub asset_id: String,
    /// Its place among the scene's frames, from 1.
    pub position: i64,
    /// Whether the video is cut from this one.
    pub is_selected: bool,
    /// Where the bytes are — the board draws the picture without a second read.
    pub path: String,
    /// The name the file arrived under: `scene-04-still-v3.png` is what the
    /// person recognises among four near-identical pictures.
    pub original_name: Option<String>,
    pub created_at: String,
}

const SELECT: &str = "SELECT f.id, f.scene_id, f.asset_id, f.position, f.is_selected, \
     a.path, a.original_name, f.created_at \
     FROM scene_frame f JOIN asset a ON a.id = f.asset_id";

/// Copy a picture into the workspace and hang it on a scene.
pub fn attach(
    conn: &Connection,
    media_dir: &Path,
    scene_id: &str,
    source: &Path,
) -> Result<SceneFrame> {
    attach_minted(conn, media_dir, scene_id, source, Minted::fresh())
}

/// Copy and hang with the id and moment already decided — the seam a replay
/// comes back through, see ADR 0014.
///
/// The asset is minted from the same moment: one act of the person is one id
/// to replay, and a frame whose asset was minted separately would come back
/// as a frame pointing at nothing.
pub fn attach_minted(
    conn: &Connection,
    media_dir: &Path,
    scene_id: &str,
    source: &Path,
    minted: Minted,
) -> Result<SceneFrame> {
    let scene =
        crate::scene::get(conn, scene_id)?.ok_or_else(|| Error::not_found("scene", scene_id))?;

    let stored = asset::attach_minted(
        conn,
        &scene.profile_id,
        media_dir,
        source,
        NewAsset {
            work_id: Some(scene.work_id.clone()),
            kind: Some(FRAME.into()),
            ..NewAsset::default()
        },
        minted.clone(),
    )?;

    // The frame goes last: the order a person sees is the order the pictures
    // arrived in, until they say otherwise.
    let next = next_position(conn, scene_id)?;
    let frame_id = format!("{}-frame", minted.id());
    conn.execute(
        "INSERT INTO scene_frame (id, profile_id, scene_id, asset_id, position, is_selected, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
        params![
            frame_id,
            scene.profile_id,
            scene_id,
            stored.id,
            next,
            minted.at()
        ],
    )?;

    get(conn, &frame_id)?.ok_or_else(|| Error::not_found("scene_frame", &frame_id))
}

/// Hang a picture that arrived as bytes rather than as a file — a paste.
///
/// The clipboard hands the window pixels, not a path, and the window has no
/// business writing to the disk itself: it would need a filesystem plugin and
/// the permissions that come with it, for one picture on its way into a
/// directory this process already owns. So the bytes come here, land in a
/// temporary file, and take exactly the same road as a picked file — one copy
/// path, not two, and nothing to keep in step later.
pub fn attach_bytes(
    conn: &Connection,
    media_dir: &Path,
    scene_id: &str,
    bytes: &[u8],
    name: &str,
) -> Result<SceneFrame> {
    let safe = Path::new(name)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "pasted.png".to_owned());

    let holding = tempfile::Builder::new()
        .prefix("kilna-paste-")
        .tempdir()
        .map_err(|cause| Error::Other(format!("could not hold the pasted picture: {cause}")))?;
    let source = holding.path().join(&safe);
    std::fs::write(&source, bytes)
        .map_err(|cause| Error::Other(format!("could not write the pasted picture: {cause}")))?;

    attach(conn, media_dir, scene_id, &source)
}

/// Take a frame off a scene, bytes and all.
///
/// Irreversible, and for the reason ADR 0027 gives about detaching: a row
/// restored beside deleted bytes is a broken picture, not an undo.
pub fn detach(conn: &Connection, id: &str) -> Result<()> {
    let Some(frame) = get(conn, id)? else {
        return Ok(());
    };
    // Deleting the asset takes the frame with it — the schema cascades — and
    // takes the bytes, which is what `asset::delete` is for.
    asset::delete(conn, &frame.asset_id)?;
    Ok(())
}

/// Cut the video from this frame, and from no other of its scene.
///
/// The old mark is cleared first: the schema allows one chosen frame per
/// scene, and setting the new one before clearing the old would collide with
/// the index rather than replace it.
pub fn select(conn: &Connection, id: &str) -> Result<SceneFrame> {
    let frame = get(conn, id)?.ok_or_else(|| Error::not_found("scene_frame", id))?;
    conn.execute(
        "UPDATE scene_frame SET is_selected = 0 WHERE scene_id = ?1 AND is_selected = 1",
        params![frame.scene_id],
    )?;
    conn.execute(
        "UPDATE scene_frame SET is_selected = 1 WHERE id = ?1",
        params![id],
    )?;
    get(conn, id)?.ok_or_else(|| Error::not_found("scene_frame", id))
}

/// Stop cutting from any frame of this scene: four candidates and no verdict
/// is the ordinary middle of the work, and a person may go back to it.
pub fn clear_selection(conn: &Connection, scene_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE scene_frame SET is_selected = 0 WHERE scene_id = ?1",
        params![scene_id],
    )?;
    Ok(())
}

/// Put the scene's frames in the order given, first to last.
///
/// Positions are rewritten from 1 rather than swapped: a list the person
/// dragged into shape is the answer, and arithmetic on neighbours is how
/// gaps and duplicates get in.
pub fn reorder(conn: &Connection, scene_id: &str, ids: &[String]) -> Result<Vec<SceneFrame>> {
    let known: Vec<String> = for_scene(conn, scene_id)?
        .into_iter()
        .map(|frame| frame.id)
        .collect();
    if ids.len() != known.len() || !known.iter().all(|id| ids.contains(id)) {
        return Err(Error::Other(
            "reordering a scene's frames names each of them exactly once".into(),
        ));
    }
    for (index, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE scene_frame SET position = ?1 WHERE id = ?2 AND scene_id = ?3",
            params![index as i64 + 1, id, scene_id],
        )?;
    }
    for_scene(conn, scene_id)
}

/// One frame.
pub fn get(conn: &Connection, id: &str) -> Result<Option<SceneFrame>> {
    let mut statement = conn.prepare(&format!("{SELECT} WHERE f.id = ?1"))?;
    let row = statement.query_row(params![id], read).optional()?;
    Ok(row)
}

/// The frames of one scene, in order.
pub fn for_scene(conn: &Connection, scene_id: &str) -> Result<Vec<SceneFrame>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE f.scene_id = ?1 ORDER BY f.position, f.rowid"
    ))?;
    let rows = statement
        .query_map(params![scene_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// The frames of every scene of a board, in one read: fifty scenes drawn with
/// fifty queries is the shape that made the predecessor's screens slow.
pub fn for_work(conn: &Connection, work_id: &str) -> Result<Vec<SceneFrame>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} JOIN scene s ON s.id = f.scene_id
         WHERE s.work_id = ?1 ORDER BY s.position, f.position, f.rowid"
    ))?;
    let rows = statement
        .query_map(params![work_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// The scene a frame belongs to — the board answers from the frame's end too,
/// which is what the predecessor could not do.
pub fn scene_of(conn: &Connection, asset_id: &str) -> Result<Option<String>> {
    let row = conn
        .query_row(
            "SELECT scene_id FROM scene_frame WHERE asset_id = ?1",
            params![asset_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(row)
}

fn next_position(conn: &Connection, scene_id: &str) -> Result<i64> {
    let last: Option<i64> = conn.query_row(
        "SELECT MAX(position) FROM scene_frame WHERE scene_id = ?1",
        params![scene_id],
        |row| row.get(0),
    )?;
    Ok(last.unwrap_or(0) + 1)
}

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<SceneFrame> {
    Ok(SceneFrame {
        id: row.get(0)?,
        scene_id: row.get(1)?,
        asset_id: row.get(2)?,
        position: row.get(3)?,
        is_selected: row.get::<_, i64>(4)? == 1,
        path: row.get(5)?,
        original_name: row.get(6)?,
        created_at: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::{self, NewWork};
    use crate::{db, profile, scene};

    fn workspace() -> (Connection, String, tempfile::TempDir) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        let media = tempfile::tempdir().unwrap();
        (conn, profile_id, media)
    }

    fn a_scene(conn: &Connection, profile_id: &str) -> (String, String) {
        let work_id = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "video".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id;
        let scene_id = scene::create(
            conn,
            profile_id,
            scene::NewScene {
                work_id: work_id.clone(),
                ..scene::NewScene::default()
            },
        )
        .unwrap()
        .id;
        (work_id, scene_id)
    }

    fn a_picture(dir: &Path, name: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, b"not really a png").unwrap();
        path
    }

    /// Four pictures for one prompt: all four are kept, in the order they
    /// arrived, and the board reads them from either end.
    #[test]
    fn a_scene_keeps_every_picture_drawn_for_it() {
        let (conn, profile_id, media) = workspace();
        let (work_id, scene_id) = a_scene(&conn, &profile_id);
        let source = tempfile::tempdir().unwrap();

        for n in 1..=4 {
            let file = a_picture(source.path(), &format!("still-v{n}.png"));
            attach(&conn, media.path(), &scene_id, &file).unwrap();
        }

        let frames = for_scene(&conn, &scene_id).unwrap();
        assert_eq!(frames.len(), 4);
        assert_eq!(
            frames.iter().map(|f| f.position).collect::<Vec<_>>(),
            vec![1, 2, 3, 4],
            "the order is the order they arrived in"
        );
        assert_eq!(
            frames[0].original_name.as_deref(),
            Some("still-v1.png"),
            "the name a generator wrote is what the person recognises"
        );
        assert_eq!(for_work(&conn, &work_id).unwrap().len(), 4);
        assert_eq!(
            scene_of(&conn, &frames[0].asset_id).unwrap().as_deref(),
            Some(scene_id.as_str()),
            "a frame names its scene, not only the other way round"
        );
    }

    /// One chosen frame per scene. Choosing a second one moves the mark
    /// rather than adding it — and the schema would not allow otherwise.
    #[test]
    fn choosing_a_second_frame_moves_the_mark() {
        let (conn, profile_id, media) = workspace();
        let (_, scene_id) = a_scene(&conn, &profile_id);
        let source = tempfile::tempdir().unwrap();
        let first = attach(
            &conn,
            media.path(),
            &scene_id,
            &a_picture(source.path(), "a.png"),
        )
        .unwrap();
        let second = attach(
            &conn,
            media.path(),
            &scene_id,
            &a_picture(source.path(), "b.png"),
        )
        .unwrap();

        select(&conn, &first.id).unwrap();
        select(&conn, &second.id).unwrap();

        let chosen: Vec<_> = for_scene(&conn, &scene_id)
            .unwrap()
            .into_iter()
            .filter(|frame| frame.is_selected)
            .collect();
        assert_eq!(chosen.len(), 1, "one scene, one verdict");
        assert_eq!(chosen[0].id, second.id);

        clear_selection(&conn, &scene_id).unwrap();
        assert!(
            for_scene(&conn, &scene_id)
                .unwrap()
                .iter()
                .all(|frame| !frame.is_selected),
            "no verdict is an ordinary state to go back to"
        );
    }

    /// The database refuses two chosen frames even when nothing asks politely:
    /// the mark is kept by the schema, not by this module remembering to
    /// clear it. Mutating `select` to skip the clearing must break this.
    #[test]
    fn two_chosen_frames_are_not_a_state_the_schema_allows() {
        let (conn, profile_id, media) = workspace();
        let (_, scene_id) = a_scene(&conn, &profile_id);
        let source = tempfile::tempdir().unwrap();
        let first = attach(
            &conn,
            media.path(),
            &scene_id,
            &a_picture(source.path(), "a.png"),
        )
        .unwrap();
        let second = attach(
            &conn,
            media.path(),
            &scene_id,
            &a_picture(source.path(), "b.png"),
        )
        .unwrap();

        conn.execute(
            "UPDATE scene_frame SET is_selected = 1 WHERE id = ?1",
            params![first.id],
        )
        .unwrap();
        let refused = conn.execute(
            "UPDATE scene_frame SET is_selected = 1 WHERE id = ?1",
            params![second.id],
        );
        assert!(
            refused.is_err(),
            "a second chosen frame must not be writable at all"
        );
    }

    /// Dragging the list into shape rewrites the order; naming a frame twice,
    /// or leaving one out, is refused rather than half-applied.
    #[test]
    fn frames_are_reordered_by_naming_all_of_them() {
        let (conn, profile_id, media) = workspace();
        let (_, scene_id) = a_scene(&conn, &profile_id);
        let source = tempfile::tempdir().unwrap();
        let a = attach(
            &conn,
            media.path(),
            &scene_id,
            &a_picture(source.path(), "a.png"),
        )
        .unwrap();
        let b = attach(
            &conn,
            media.path(),
            &scene_id,
            &a_picture(source.path(), "b.png"),
        )
        .unwrap();

        let reordered = reorder(&conn, &scene_id, &[b.id.clone(), a.id.clone()]).unwrap();
        assert_eq!(reordered[0].id, b.id);
        assert_eq!(reordered[0].position, 1);
        assert_eq!(reordered[1].position, 2);

        assert!(
            reorder(&conn, &scene_id, std::slice::from_ref(&a.id)).is_err(),
            "a list missing a frame is refused"
        );
        assert!(
            reorder(&conn, &scene_id, &[a.id.clone(), a.id.clone()]).is_err(),
            "a list naming one twice is refused"
        );
    }

    /// Taking a frame off takes its bytes: a row restored beside deleted
    /// bytes would be a broken picture, not an undo.
    #[test]
    fn detaching_a_frame_takes_the_file_with_it() {
        let (conn, profile_id, media) = workspace();
        let (_, scene_id) = a_scene(&conn, &profile_id);
        let source = tempfile::tempdir().unwrap();
        let frame = attach(
            &conn,
            media.path(),
            &scene_id,
            &a_picture(source.path(), "a.png"),
        )
        .unwrap();
        let stored = std::path::PathBuf::from(&frame.path);
        assert!(stored.is_file(), "the copy is in the workspace");

        detach(&conn, &frame.id).unwrap();
        assert!(for_scene(&conn, &scene_id).unwrap().is_empty());
        assert!(!stored.exists(), "and the bytes are gone with the row");
    }

    /// Deleting the scene takes its frames; deleting the asset takes the
    /// frame that pointed at it. Neither leaves a frame pointing at nothing.
    #[test]
    fn a_frame_does_not_outlive_its_scene_or_its_file() {
        let (conn, profile_id, media) = workspace();
        let (_, scene_id) = a_scene(&conn, &profile_id);
        let source = tempfile::tempdir().unwrap();
        let frame = attach(
            &conn,
            media.path(),
            &scene_id,
            &a_picture(source.path(), "a.png"),
        )
        .unwrap();

        asset::delete(&conn, &frame.asset_id).unwrap();
        assert!(
            get(&conn, &frame.id).unwrap().is_none(),
            "the frame goes with the file it pointed at"
        );

        let again = attach(
            &conn,
            media.path(),
            &scene_id,
            &a_picture(source.path(), "b.png"),
        )
        .unwrap();
        scene::delete(&conn, &scene_id).unwrap();
        assert!(
            get(&conn, &again.id).unwrap().is_none(),
            "and with the scene it was drawn for"
        );
    }
}
