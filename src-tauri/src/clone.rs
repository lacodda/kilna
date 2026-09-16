//! A second attempt at a video: the same donor, the same board, its own life.
//!
//! A person who is not happy with a video does not want to lose the first one
//! — they want to try again from the same starting point and compare. The
//! predecessor answered that with "versions of the whole video", which is the
//! shape this product turned down on 2026-09-11: a scene belongs to a work,
//! and a second attempt is a second work made from the same donor.
//!
//! So cloning makes a work, hangs it on the same donor, and copies the board
//! into it: every scene with its number, its timing, its shot type and its
//! prompt blocks, and every picture and clip those scenes had chosen or were
//! choosing between.
//!
//! What it does NOT do is copy the bytes. A cloned board points its own rows
//! at the same files, because the second attempt usually reuses most of the
//! first one's pictures — a board of fifty scenes with four candidates each
//! would otherwise write two hundred duplicate files per click, and nobody
//! asked for a second copy of a picture they are about to replace anyway.
//! Ownership and kind belong to the row, which is the clone's own; the path
//! is shared, and `asset::delete` since v0.69 removes the bytes only when the
//! last row naming them is gone.

use rusqlite::Connection;

use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::work::{self, NewWork, Work};
use crate::{link, scene, scene_frame};

/// What a clone came out as, for the sentence the window says afterwards.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Cloned {
    pub work: Work,
    /// How many scenes the board brought across.
    pub scenes: usize,
    /// How many stills and clips came with them.
    pub materials: usize,
}

/// Copy a work's board into a new work of the same kind.
///
/// The title is the caller's: the window offers "<title> (2)" and the person
/// may say something better before agreeing, which is the only part of this
/// anyone has an opinion about.
pub fn clone_work(conn: &Connection, work_id: &str, title: &str) -> Result<Cloned> {
    clone_work_minted(conn, work_id, title, Minted::fresh())
}

/// Clone with the new work's id and moment already decided — the seam a
/// replay comes back through, and what lets an undo name what to discard.
pub fn clone_work_minted(
    conn: &Connection,
    work_id: &str,
    title: &str,
    minted: Minted,
) -> Result<Cloned> {
    let source = work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;

    let title = title.trim();
    if title.is_empty() {
        return Err(Error::Other("a clone is given a title".into()));
    }

    // The kind, the tags and the marks come across; the score and the
    // releases do not. A clone has not been judged and has not gone out —
    // carrying a verdict over would be the copy claiming the original's
    // reception.
    let made = work::create_minted(
        conn,
        &source.profile_id,
        NewWork {
            kind: source.kind.clone(),
            title: title.to_owned(),
            tags: source.tags.clone(),
            marks: source.marks.clone(),
            collection_id: source.collection_id.clone(),
            meta: Some(source.meta.clone()),
            ..NewWork::default()
        },
        minted,
    )?;

    // The same donor, in the same role. A clip's text is the song's, and the
    // second attempt is at the same song — a clone hanging on nothing would
    // be a board that cannot be reframed.
    for source_link in link::sources(conn, &source.id)? {
        link::create(
            conn,
            &source.profile_id,
            link::NewLink {
                work_id: made.id.clone(),
                source_id: source_link.source_id.clone(),
                role: Some(source_link.role.clone()),
                source_version_id: source_link.source_version_id.clone(),
            },
        )?;
    }

    let mut materials = 0usize;
    let scenes = scene::for_work(conn, &source.id)?;
    for original in &scenes {
        let copy = scene::create(
            conn,
            &source.profile_id,
            scene::NewScene {
                work_id: made.id.clone(),
                position: Some(original.position),
                section: original.section.clone(),
                starts_at: original.starts_at,
                ends_at: original.ends_at,
                shot_type: original.shot_type.clone(),
                description: Some(original.description.clone()),
                blocks: Some(original.blocks.clone()),
            },
        )?;

        // Every candidate, not only the chosen one: choosing between them is
        // the work, and a clone that kept only the verdict would throw away
        // the three pictures the person is still deciding against.
        for held in scene_frame::for_scene(conn, &original.id)? {
            scene_frame::hang_existing(
                conn,
                &copy.id,
                &held,
                &source.profile_id,
                &made.id,
                Minted::fresh(),
            )?;
            materials += 1;
        }
    }

    Ok(Cloned {
        work: made,
        scenes: scenes.len(),
        materials,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work::NewWork;
    use crate::{asset, db, profile, scene, scene_frame};

    fn workspace() -> (Connection, String, tempfile::TempDir) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id, tempfile::tempdir().unwrap())
    }

    /// A video with one scene, one chosen still and one candidate beside it.
    fn a_board(conn: &Connection, profile_id: &str, media: &std::path::Path) -> String {
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
                description: Some("a gull over the water".into()),
                ..scene::NewScene::default()
            },
        )
        .unwrap()
        .id;

        let holding = std::path::Path::new(media);
        for name in ["a.png", "b.png"] {
            let file = holding.join(name);
            std::fs::write(&file, name.as_bytes()).unwrap();
            scene_frame::attach(conn, media, &scene_id, scene_frame::FRAME, &file).unwrap();
        }
        let chosen = scene_frame::of_kind(conn, &scene_id, scene_frame::FRAME).unwrap()[1]
            .id
            .clone();
        scene_frame::select(conn, &chosen).unwrap();
        work_id
    }

    #[test]
    fn a_clone_brings_the_board_and_every_candidate_across() {
        let (conn, profile_id, media) = workspace();
        let source = a_board(&conn, &profile_id, media.path());

        let made = clone_work(&conn, &source, "Harbour lights (2)").unwrap();

        assert_eq!(made.scenes, 1);
        assert_eq!(
            made.materials, 2,
            "both candidates, not only the chosen one — deciding between them is the work"
        );

        let scenes = scene::for_work(&conn, &made.work.id).unwrap();
        assert_eq!(scenes[0].description, "a gull over the water");

        let copied = scene_frame::of_kind(&conn, &scenes[0].id, scene_frame::FRAME).unwrap();
        assert_eq!(copied.len(), 2);
        assert!(
            copied.iter().any(|one| one.is_selected),
            "a clone of a board that had chosen its still has chosen its still"
        );
    }

    #[test]
    fn a_clone_shares_the_bytes_rather_than_copying_them() {
        let (conn, profile_id, media) = workspace();
        let source = a_board(&conn, &profile_id, media.path());
        let before = std::fs::read_dir(media.path()).unwrap().count();

        let made = clone_work(&conn, &source, "second attempt").unwrap();

        assert_eq!(
            std::fs::read_dir(media.path()).unwrap().count(),
            before,
            "cloning writes no new files: fifty scenes of four candidates would be two hundred"
        );

        let scenes = scene::for_work(&conn, &made.work.id).unwrap();
        let copied = scene_frame::for_scene(&conn, &scenes[0].id).unwrap();
        let original_scene = &scene::for_work(&conn, &source).unwrap()[0];
        let originals = scene_frame::for_scene(&conn, &original_scene.id).unwrap();
        assert_eq!(copied[0].path, originals[0].path, "the same file");
        assert_ne!(copied[0].asset_id, originals[0].asset_id, "its own row");
    }

    #[test]
    fn removing_a_clones_picture_leaves_the_original_looking_at_its_file() {
        // The trap the shared path opens, and the reason `asset::delete`
        // counts rows before it removes bytes.
        let (conn, profile_id, media) = workspace();
        let source = a_board(&conn, &profile_id, media.path());
        let made = clone_work(&conn, &source, "second attempt").unwrap();

        let scenes = scene::for_work(&conn, &made.work.id).unwrap();
        let copied = scene_frame::for_scene(&conn, &scenes[0].id).unwrap();
        let shared = copied[0].path.clone();
        scene_frame::detach(&conn, &copied[0].id).unwrap();

        assert!(
            std::path::Path::new(&shared).is_file(),
            "the original still points at this file"
        );

        // And when the last row goes, so do the bytes.
        let original_scene = &scene::for_work(&conn, &source).unwrap()[0];
        let originals = scene_frame::for_scene(&conn, &original_scene.id).unwrap();
        let last = originals.iter().find(|one| one.path == shared).unwrap();
        scene_frame::detach(&conn, &last.id).unwrap();
        assert!(
            !std::path::Path::new(&shared).is_file(),
            "nothing names it now"
        );
    }

    #[test]
    fn a_clone_hangs_on_the_same_donor() {
        let (conn, profile_id, media) = workspace();
        let song = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Harbour".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        let source = a_board(&conn, &profile_id, media.path());
        link::create(
            &conn,
            &profile_id,
            link::NewLink {
                work_id: source.clone(),
                source_id: song.id.clone(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();

        let made = clone_work(&conn, &source, "second attempt").unwrap();

        let donors = link::sources(&conn, &made.work.id).unwrap();
        assert_eq!(
            donors.first().map(|one| one.source_id.as_str()),
            Some(song.id.as_str()),
            "a board that cannot name its song cannot be reframed from it"
        );
    }

    #[test]
    fn discarding_a_clone_takes_its_board_with_it_and_leaves_the_original() {
        // What an undo of a clone does: the work goes to the trash, and the
        // scenes and material it made go with it. The original must not
        // notice — a copy taken back is not an edit to what it copied.
        let (mut conn, profile_id, media) = workspace();
        let source = a_board(&conn, &profile_id, media.path());
        let made = clone_work(&conn, &source, "second attempt").unwrap();

        crate::trash::discard(&mut conn, crate::trash::Entity::Work, &made.work.id).unwrap();

        assert!(
            scene::for_work(&conn, &made.work.id).unwrap().is_empty(),
            "the copied board went with the work"
        );

        let original_scene = &scene::for_work(&conn, &source).unwrap()[0];
        assert_eq!(
            scene_frame::for_scene(&conn, &original_scene.id)
                .unwrap()
                .len(),
            2,
            "the original kept both candidates"
        );
        assert!(
            scene_frame::of_kind(&conn, &original_scene.id, scene_frame::FRAME)
                .unwrap()
                .iter()
                .all(|one| std::path::Path::new(&one.path).is_file()),
            "and its files"
        );
    }

    #[test]
    fn a_clone_is_given_a_title() {
        let (conn, profile_id, media) = workspace();
        let source = a_board(&conn, &profile_id, media.path());

        assert!(clone_work(&conn, &source, "   ").is_err());
    }

    #[test]
    fn a_clone_carries_no_verdict_of_its_own() {
        // The score and the releases stay with the original: a copy has not
        // been judged and has not gone out.
        let (conn, profile_id, media) = workspace();
        let source = a_board(&conn, &profile_id, media.path());
        let made = clone_work(&conn, &source, "second attempt").unwrap();

        assert!(
            crate::score::latest(&conn, &made.work.id)
                .unwrap()
                .is_none(),
            "a clone has not been judged"
        );
        assert!(
            asset::for_work(&conn, &made.work.id)
                .unwrap()
                .iter()
                .all(|one| one.kind != "cover"),
            "and wears no cover it did not earn"
        );
    }
}
