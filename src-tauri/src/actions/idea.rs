//! Gestures on a cover's board of ideas (ADR 0050).
//!
//! An idea is put on the board, judged - starred onto the shortlist, turned
//! down, or the verdict taken back - and taken into the constructor. The
//! first two are the idea's own operations; taking one in is an edit of the
//! publication's cover, logged and undone as every edit of a work is.

use rusqlite::Connection;

use super::gesture;
use crate::cover::idea::{self, CoverIdea, IdeaPatch, NewIdea, Source, Verdict};
use crate::error::{Error, Result};
use crate::work::{Work, WorkPatch};

/// Put an idea on a board.
pub fn create(conn: &Connection, new: NewIdea) -> Result<CoverIdea> {
    gesture(conn, "idea.create", |act| {
        act.json("idea", &new)?;
        let minted = act.mint();
        idea::create_minted(act, act.profile_id(), new, minted)
    })
}

/// Change an idea, with what the fields held before, so it can be taken back.
pub fn update(conn: &Connection, id: &str, patch: IdeaPatch) -> Result<CoverIdea> {
    gesture(conn, "idea.update", |act| {
        let before = idea::get(act, id)?;
        act.param("id", id);
        act.json("patch", &patch)?;
        act.before(before.as_ref(), &patch)?;
        act.stamped();
        idea::update_at(act, id, patch, act.at())
    })
}

/// The person's own idea, in their words, on the board.
pub fn add_own(conn: &Connection, work_id: &str, words: &str) -> Result<CoverIdea> {
    let words = words.trim();
    if words.is_empty() {
        return Err(Error::refused("idea.empty"));
    }
    create(
        conn,
        NewIdea {
            work_id: work_id.to_owned(),
            source: Source::Own,
            from_work_id: None,
            angle: String::new(),
            headline: String::new(),
            concept: crate::cover::Cover {
                idea: words.to_owned(),
                ..crate::cover::Cover::default()
            },
            verdict: None,
        },
    )
}

/// Star an idea, turn it down, or take the verdict back. Saying the verdict
/// it already has changes nothing, and records nothing.
pub fn judge(conn: &Connection, id: &str, verdict: Option<Verdict>) -> Result<CoverIdea> {
    let current = idea::get(conn, id)?.ok_or_else(|| Error::not_found("idea", id))?;
    if current.verdict == verdict {
        return Ok(current);
    }
    update(
        conn,
        id,
        IdeaPatch {
            verdict: Some(verdict),
            ..IdeaPatch::default()
        },
    )
}

/// Judge a neighbour's cover: it is copied onto the board with the verdict,
/// fitted to this publication's shape - the copy is what was judged, and it
/// stays as it is when the neighbour's cover changes later.
pub fn judge_sibling(
    conn: &Connection,
    work_id: &str,
    sibling_id: &str,
    verdict: Verdict,
) -> Result<CoverIdea> {
    let mut new = idea::from_sibling(conn, work_id, sibling_id)?;
    new.verdict = Some(verdict);
    create(conn, new)
}

/// Take an idea into its publication's constructor: the cover becomes what
/// the idea decides (see [`idea::taken_into`]).
pub fn take(conn: &Connection, id: &str) -> Result<Work> {
    let found = idea::get(conn, id)?.ok_or_else(|| Error::not_found("idea", id))?;
    take_concept(conn, &found.work_id, &found.concept)
}

/// Take a neighbour's cover into this publication's constructor, fitted to
/// its shape, without putting it on the board.
pub fn take_sibling(conn: &Connection, work_id: &str, sibling_id: &str) -> Result<Work> {
    let new = idea::from_sibling(conn, work_id, sibling_id)?;
    take_concept(conn, work_id, &new.concept)
}

fn take_concept(conn: &Connection, work_id: &str, concept: &crate::cover::Cover) -> Result<Work> {
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let cover = idea::taken_into(&work.cover, concept);
    super::work::update(
        conn,
        work_id,
        WorkPatch {
            cover: Some(cover),
            ..WorkPatch::default()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cover::framing::Layout;
    use crate::fixtures;
    use crate::link::{self, NewLink};

    fn made_from(conn: &Connection, profile_id: &str, work_id: &str, source_id: &str) {
        link::create(
            conn,
            profile_id,
            NewLink {
                work_id: work_id.to_owned(),
                source_id: source_id.to_owned(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();
    }

    /// A star, a "not that", and back - each one operation, each undone to
    /// the verdict before it; and saying the same verdict twice is nothing.
    #[test]
    fn a_verdict_is_logged_and_taken_back() {
        let (conn, profile_id) = fixtures::workspace();
        let audio = fixtures::work(&conn, &profile_id, "audio", "Harbour lights — audio");
        let own = add_own(&conn, &audio.id, "the harbour at dawn").unwrap();
        assert_eq!(own.source, Source::Own);

        let before = crate::operation::count(&conn).unwrap();
        judge(&conn, &own.id, Some(Verdict::Star)).unwrap();
        judge(&conn, &own.id, Some(Verdict::Star)).unwrap();
        assert_eq!(crate::operation::count(&conn).unwrap(), before + 1);

        judge(&conn, &own.id, Some(Verdict::Rejected)).unwrap();
        let offer = crate::undo::last(&conn).unwrap().unwrap();
        crate::undo::undo(&conn, &offer.operation_id).unwrap();
        assert_eq!(
            idea::get(&conn, &own.id).unwrap().unwrap().verdict,
            Some(Verdict::Star)
        );
    }

    /// Undoing an idea put on the board takes it to the trash, and it comes
    /// back from there.
    #[test]
    fn an_idea_put_on_the_board_is_undone_into_the_trash() {
        let (conn, profile_id) = fixtures::workspace();
        let audio = fixtures::work(&conn, &profile_id, "audio", "Harbour lights — audio");
        let own = add_own(&conn, &audio.id, "the harbour at dawn").unwrap();
        let offer = crate::undo::last(&conn).unwrap().unwrap();
        assert_eq!(offer.action, "undo.idea.create");
        crate::undo::undo(&conn, &offer.operation_id).unwrap();
        assert!(idea::get(&conn, &own.id).unwrap().is_none());
        let entry = crate::trash::list(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.entity_id == own.id)
            .expect("the idea is in the trash");
        assert_eq!(entry.label, "the harbour at dawn");
        crate::trash::restore(&conn, &entry.id).unwrap();
        assert!(idea::get(&conn, &own.id).unwrap().is_some());
    }

    /// A song without a cover has no board; neither has anything that is not
    /// a publication with a cover.
    #[test]
    fn only_a_publication_with_a_cover_has_a_board() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let refused = add_own(&conn, &song.id, "an idea").unwrap_err();
        assert_eq!(refused.refusal().unwrap().code, "idea.noCover");
        let audio = fixtures::work(&conn, &profile_id, "audio", "Harbour lights — audio");
        assert_eq!(
            add_own(&conn, &audio.id, "   ")
                .unwrap_err()
                .refusal()
                .unwrap()
                .code,
            "idea.empty"
        );
    }

    /// The clip's cover lies on the short's board, fitted to the short's tall
    /// shape; a star copies it onto the board and it leaves the neighbours;
    /// taking it in builds the short's cover from it.
    #[test]
    fn a_neighbours_cover_is_offered_fitted_and_copied_when_starred() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let clip = fixtures::video(&conn, &profile_id, "Harbour lights — clip");
        let short = fixtures::work(&conn, &profile_id, "short", "Harbour lights · short 1");
        made_from(&conn, &profile_id, &clip.id, &song.id);
        made_from(&conn, &profile_id, &short.id, &clip.id);
        fixtures::release(&conn, &short.id, "short", None);

        let clip_cover = crate::cover::Cover {
            idea: "a lighthouse keeper".into(),
            scene: "a keeper at the lamp".into(),
            framing: Some(Layout::EmblemRight.defaults()),
            ..crate::cover::Cover::default()
        };
        super::super::work::update(
            &conn,
            &clip.id,
            WorkPatch {
                cover: Some(clip_cover),
                ..WorkPatch::default()
            },
        )
        .unwrap();

        let board = idea::board(&conn, &short.id).unwrap();
        assert_eq!(board.format, "9:16");
        assert_eq!(board.siblings.len(), 1);
        let offered = &board.siblings[0];
        assert_eq!(offered.work_id, clip.id);
        assert_eq!(offered.concept.framing.unwrap().layout, Layout::Centre);

        let copied = judge_sibling(&conn, &short.id, &clip.id, Verdict::Star).unwrap();
        assert_eq!(copied.source, Source::Sibling);
        assert_eq!(copied.from_work_id.as_deref(), Some(clip.id.as_str()));
        let board = idea::board(&conn, &short.id).unwrap();
        assert!(
            board.siblings.is_empty(),
            "a copied cover leaves the neighbours"
        );
        assert_eq!(board.ideas.len(), 1);
        assert_eq!(
            board.ideas[0].from_title.as_deref(),
            Some(clip.title.as_str())
        );

        let short = take(&conn, &copied.id).unwrap();
        assert_eq!(short.cover.scene, "a keeper at the lamp");
        assert_eq!(short.cover.framing.unwrap().layout, Layout::Centre);
    }
}
