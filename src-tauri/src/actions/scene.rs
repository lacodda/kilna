//! Gestures on a storyboard: its scenes, what they are about, and the frames
//! drawn for them.

use std::path::Path;

use rusqlite::Connection;

use super::{count, gesture};
use crate::error::{Error, Result};
use crate::journal::Record;
use crate::scene::{self, NewScene, Scene, ScenePatch};
use crate::scene_frame::{self, SceneFrame};
use crate::scene_note::{self, SceneNote};

/// Add a scene to a work's storyboard — after the last, unless numbered.
pub fn create(conn: &Connection, new: NewScene) -> Result<Scene> {
    gesture(conn, "scene.create", |act| {
        act.json("scene", &new)?;
        let minted = act.mint();
        let created = scene::create_minted(act, act.profile_id(), new, minted)?;
        act.journal(
            Record::new("scene.created")
                .param("title", act.title_of(&created.work_id))
                .param("number", created.position)
                .about("work", created.work_id.clone()),
        );
        Ok(created)
    })
}

/// Edit a scene: one field, or the prompt blocks as a set.
pub fn update(conn: &Connection, id: &str, patch: ScenePatch) -> Result<Scene> {
    gesture(conn, "scene.update", |act| {
        let before = scene::get(act, id)?;
        act.param("id", id);
        act.json("patch", &patch)?;
        act.before(before.as_ref(), &patch)?;
        act.stamped();
        scene::update_at(act, id, patch, act.at())
    })
}

/// Say that a scene is about a note — a character, a place.
pub fn attach_note(conn: &Connection, scene_id: &str, note_id: &str) -> Result<SceneNote> {
    gesture(conn, "scene.attachNote", |act| {
        act.param("sceneId", scene_id);
        act.param("noteId", note_id);
        let minted = act.mint();
        scene_note::attach_minted(act, scene_id, note_id, minted)
    })
}

/// Stop a scene being about a note.
pub fn detach_note(conn: &Connection, scene_id: &str, note_id: &str) -> Result<()> {
    gesture(conn, "scene.detachNote", |act| {
        act.param("sceneId", scene_id);
        act.param("noteId", note_id);
        scene_note::detach(act, scene_id, note_id)
    })
}

/// Build the board's frame from the parts the source text marks out.
///
/// One scene per part, in the text's order, carrying the part's name. The
/// scenes' ids are minted here and travel in the operation, so a workspace
/// rebuilt from the log lands on the same rows.
pub fn frame(conn: &Connection, work_id: &str, role: &str) -> Result<Vec<Scene>> {
    gesture(conn, "scene.frame", |act| {
        // How many parts the text marks out right now: the ids are minted for
        // exactly those, and framing refuses if the text has changed since.
        let parts = scene::parts_of_source(act, work_id, role)?;
        let minted: Vec<crate::minted::Minted> = (0..parts).map(|_| act.fresh()).collect();
        let ids: Vec<String> = minted.iter().map(|one| one.id().to_owned()).collect();
        act.param("workId", work_id);
        act.param("role", role);
        act.json("ids", &ids)?;
        act.stamped();

        let framed = scene::frame_from_text(act, work_id, role, &minted)?;
        act.journal(
            Record::new("scene.framed")
                .param("title", act.title_of(work_id))
                .param("count", count(framed.len()))
                .about("work", work_id.to_owned()),
        );
        Ok(framed)
    })
}

/// Divide the work's length between the scenes of its board.
///
/// The first timing of a board, not the last word on it: every span after
/// this is dragged by hand. One operation for the whole board, with the spans
/// as they stood, so the undo puts back exactly these.
pub fn time(conn: &Connection, work_id: &str) -> Result<Vec<Scene>> {
    gesture(conn, "scene.time", |act| {
        let before: Vec<serde_json::Value> = scene::for_work(act, work_id)?
            .into_iter()
            .map(|scene| {
                serde_json::json!({
                    "id": scene.id,
                    "startsAt": scene.starts_at,
                    "endsAt": scene.ends_at,
                })
            })
            .collect();
        act.param("workId", work_id);
        act.json("before", &before)?;
        act.stamped();

        let timed = scene::time_board_at(act, work_id, act.at())?;
        act.journal(
            Record::new("scene.timed")
                .param("title", act.title_of(work_id))
                .param("count", count(timed.len()))
                .about("work", work_id.to_owned()),
        );
        Ok(timed)
    })
}

/// Number a board in the order given: 1..N, in one change.
///
/// The whole order travels rather than one scene and a target number, because
/// both gestures the screen offers — putting a new scene between two others,
/// and moving one that is already there — are the same thing said twice. The
/// numbers the board held travel too — not a tidy 1..N, which is very likely a
/// board this one has never been — so the undo puts back exactly those.
pub fn renumber(conn: &Connection, work_id: &str, ids: &[String]) -> Result<Vec<Scene>> {
    gesture(conn, "scene.renumber", |act| {
        let before: Vec<serde_json::Value> = scene::for_work(act, work_id)?
            .into_iter()
            .map(|scene| serde_json::json!({ "id": scene.id, "position": scene.position }))
            .collect();
        act.param("workId", work_id);
        act.json("ids", &ids)?;
        act.json("before", &before)?;
        act.stamped();

        let numbered = scene::renumber(act, work_id, ids, act.at())?;
        act.journal(
            Record::new("scene.renumbered")
                .param("title", act.title_of(work_id))
                .param("count", count(numbered.len()))
                .about("work", work_id.to_owned()),
        );
        Ok(numbered)
    })
}

/// Copy a picture into the workspace and hang it on a scene.
///
/// The path comes from the picker, from a drop, or from a pasted image the
/// window wrote to a temporary file; by the time it gets here it is a place
/// on this machine, and the bytes are copied into the workspace's `media/`.
pub fn attach_frame(
    conn: &Connection,
    media: &Path,
    scene_id: &str,
    kind: &str,
    source: &str,
) -> Result<SceneFrame> {
    gesture(conn, "scene.attachFrame", |act| {
        act.param("sceneId", scene_id);
        act.param("kind", kind);
        act.param("source", source);
        let minted = act.mint();
        let attached =
            scene_frame::attach_minted(act, media, scene_id, kind, Path::new(source), minted)?;
        act.journal(
            Record::new("scene.framed")
                .param("name", attached.original_name.clone().unwrap_or_default())
                .about("scene", scene_id.to_owned()),
        );
        Ok(attached)
    })
}

/// Hang a pasted picture on a scene: the clipboard gives bytes, not a path.
///
/// The bytes are not written into the log: an operation carrying a picture
/// would make the log the size of the pictures. It is recorded as the arrival
/// it is, and like `scene.attachFrame` it is not replayed.
pub fn paste_frame(
    conn: &Connection,
    media: &Path,
    scene_id: &str,
    kind: &str,
    bytes: &[u8],
    name: &str,
) -> Result<SceneFrame> {
    gesture(conn, "scene.attachFrame", |act| {
        act.param("sceneId", scene_id);
        act.param("kind", kind);
        act.param("source", format!("<pasted: {name}>"));
        let attached = scene_frame::attach_bytes(act, media, scene_id, kind, bytes, name)?;
        act.journal(
            Record::new("scene.framed")
                .param("name", attached.original_name.clone().unwrap_or_default())
                .about("scene", scene_id.to_owned()),
        );
        Ok(attached)
    })
}

/// Take a frame off a scene, and the copy the workspace made with it.
///
/// Not the trash, for the reason ADR 0027 gives: a row restored beside bytes
/// that are gone is a broken picture, not an undo.
pub fn detach_frame(conn: &Connection, id: &str) -> Result<()> {
    gesture(conn, "scene.detachFrame", |act| {
        act.param("id", id);
        scene_frame::detach(act, id)
    })
}

/// Cut the video from this frame — and from no other of its kind on the
/// scene. Which one was chosen before travels, so an undo puts that one back
/// rather than leaving the scene undecided.
pub fn select_frame(conn: &Connection, id: &str) -> Result<SceneFrame> {
    gesture(conn, "scene.selectFrame", |act| {
        let frame =
            scene_frame::get(act, id)?.ok_or_else(|| Error::not_found("scene_frame", id))?;
        let before = scene_frame::for_scene(act, &frame.scene_id)?
            .into_iter()
            .find(|one| one.is_selected)
            .map(|one| one.id);
        act.param("id", id);
        act.param("sceneId", frame.scene_id.clone());
        act.json("before", &before)?;
        scene_frame::select(act, id)
    })
}

/// Go back to having no frame of a kind chosen for a scene. The verdict being
/// cleared travels, so an undo can put it back; of this kind only - the chosen
/// still is not touched by a change of mind about a clip.
pub fn clear_frame(conn: &Connection, scene_id: &str, kind: &str) -> Result<()> {
    gesture(conn, "scene.clearFrame", |act| {
        let before = scene_frame::selected(act, scene_id, kind)?.map(|one| one.id);
        act.param("sceneId", scene_id);
        act.param("kind", kind);
        act.json("before", &before)?;
        scene_frame::clear_selection(act, scene_id, kind)
    })
}

/// Put a scene's frames of a kind in the order given, first to last.
pub fn reorder_frames(
    conn: &Connection,
    scene_id: &str,
    kind: &str,
    ids: &[String],
) -> Result<Vec<SceneFrame>> {
    gesture(conn, "scene.reorderFrames", |act| {
        let before: Vec<String> = scene_frame::of_kind(act, scene_id, kind)?
            .into_iter()
            .map(|one| one.id)
            .collect();
        act.param("sceneId", scene_id);
        act.param("kind", kind);
        act.json("ids", &ids)?;
        act.json("before", &before)?;
        scene_frame::reorder(act, scene_id, kind, ids)
    })
}
