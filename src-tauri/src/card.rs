//! What a card's tabs count.
//!
//! The tab bar puts a number beside every tab that has something to count.
//! Until v0.80 the card fetched four whole lists - every release, link, scene
//! and stretch the work had - only to read their lengths, asked for the
//! comments' counter separately, and left six tabs without a number. One
//! question now, answered in one statement, and the lists are fetched only by
//! the tab that draws them.
//!
//! Each number counts what its tab lists, by the tab's own rule: comments
//! without the archived ones, history no further than a page of it.

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::error::Result;

/// The number beside each of a work's tabs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct Counts {
    /// Versions of every role.
    pub versions: i64,
    pub scores: i64,
    pub releases: i64,
    /// Everything attached to the work, the cover among them.
    pub files: i64,
    /// What the work was made from. Apart from `derived` because the card
    /// asks it alone: a work with a donor is one that can be cut.
    pub sources: i64,
    /// What was made from the work.
    pub derived: i64,
    pub notes: i64,
    /// The work's comments, the archived ones aside, and how many of them
    /// still wait for an answer - the part of the counter worth a mark. For
    /// a work that never goes out itself, the comments under what was made
    /// from it: its Comments tab sums them up (v0.86).
    pub comments: i64,
    pub comments_waiting: i64,
    pub scenes: i64,
    /// The stretches the work is spliced from.
    pub cuts: i64,
    /// The stretches cut out of the work into others: a donor's Cuts tab
    /// names what was taken from it, so a work only ever cut from still has
    /// the tab.
    pub cut_from: i64,
    /// Lines of the work's own history, as many as its tab shows.
    pub history: i64,
}

/// Count everything a work's card has tabs for.
///
/// A work that does not exist counts nothing rather than failing: the card
/// that asks is already saying the work is gone, and a second error beside
/// that would say it twice.
pub fn counts(conn: &Connection, work_id: &str) -> Result<Counts> {
    // The comments a work's tab lists: its own, or - for a song, which the
    // audience never meets as the song - those under its publications. The
    // same works a status is read from.
    let heard = match crate::work::get(conn, work_id)? {
        Some(work) => {
            let config = crate::profile::config_for(conn, &work.profile_id)?;
            crate::work::status::speaking_for(
                conn,
                work_id,
                config.vocabulary(&work.kind).has_doors(),
            )?
        }
        None => vec![work_id.to_owned()],
    };
    let (mut comments, mut comments_waiting) = (0, 0);
    for id in &heard {
        let (listed, waiting) = crate::comment::count_for_work(conn, id)?;
        comments += listed;
        comments_waiting += waiting;
    }
    let history = crate::journal::count_for_entity(conn, "work", work_id)?;

    Ok(conn.query_row(
        "SELECT (SELECT count(*) FROM work_version WHERE work_id = ?1),
                (SELECT count(*) FROM work_score WHERE work_id = ?1),
                (SELECT count(*) FROM release WHERE work_id = ?1),
                (SELECT count(*) FROM asset WHERE work_id = ?1),
                (SELECT count(*) FROM work_link WHERE work_id = ?1),
                (SELECT count(*) FROM work_link WHERE source_id = ?1),
                (SELECT count(*) FROM note WHERE work_id = ?1),
                (SELECT count(*) FROM scene WHERE work_id = ?1),
                (SELECT count(*) FROM cut WHERE work_id = ?1),
                (SELECT count(*) FROM cut WHERE source_id = ?1)",
        params![work_id],
        |row| {
            Ok(Counts {
                versions: row.get(0)?,
                scores: row.get(1)?,
                releases: row.get(2)?,
                files: row.get(3)?,
                sources: row.get(4)?,
                derived: row.get(5)?,
                notes: row.get(6)?,
                comments,
                comments_waiting,
                scenes: row.get(7)?,
                cuts: row.get(8)?,
                cut_from: row.get(9)?,
                history,
            })
        },
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::{self, NewComment};
    use crate::fixtures;
    use crate::journal::{self, Record};
    use crate::link::{self, NewLink};
    use crate::minted::Minted;
    use crate::note::{self, NewNote};
    use crate::release::{self, NewRelease};
    use crate::scene::{self, NewScene};
    use crate::work::version::{self, NewVersion};

    fn work(conn: &Connection, profile_id: &str, kind: &str, title: &str) -> String {
        fixtures::work(conn, profile_id, kind, title).id
    }

    fn lyrics(conn: &mut Connection, work_id: &str, body: &str) {
        version::create(
            conn,
            work_id,
            NewVersion {
                role: "lyrics".into(),
                body: body.into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
    }

    fn comment(conn: &Connection, profile_id: &str, work_id: &str, body: &str) -> String {
        comment::create_minted(
            conn,
            profile_id,
            NewComment {
                channel: "youtube".into(),
                body: body.into(),
                work_id: Some(work_id.into()),
                author: None,
                reply: None,
                commented_on: None,
            },
            Minted::fresh(),
        )
        .unwrap()
        .id
    }

    #[test]
    fn a_new_work_counts_nothing_and_neither_does_a_missing_one() {
        let (conn, profile_id) = fixtures::workspace();
        let song = work(&conn, &profile_id, "song", "Tide");

        let fresh = counts(&conn, &song).unwrap();
        // Creating a work may say so in its history; nothing else is there.
        assert_eq!(
            Counts {
                history: fresh.history,
                ..Counts::default()
            },
            fresh
        );
        assert_eq!(counts(&conn, "no-such-work").unwrap(), Counts::default());
    }

    #[test]
    fn each_tab_counts_what_it_lists() {
        let (mut conn, profile_id) = fixtures::workspace();
        let song = work(&conn, &profile_id, "song", "Tide");
        let video = work(&conn, &profile_id, "video", "Tide (clip)");
        let other = work(&conn, &profile_id, "song", "Elsewhere");

        lyrics(&mut conn, &song, "one line");
        lyrics(&mut conn, &song, "one line\ntwo lines");
        lyrics(&mut conn, &other, "not this one");

        release::create(
            &conn,
            NewRelease {
                work_id: video.clone(),
                kind: "youtube".into(),
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap();

        // The song is a source of the clip: one link, counted on both cards
        // and in opposite directions.
        link::create(
            &conn,
            &profile_id,
            NewLink {
                work_id: video.clone(),
                source_id: song.clone(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();

        note::create(
            &conn,
            &profile_id,
            NewNote {
                body: "the chorus comes too late".into(),
                kind: None,
                title: None,
                work_id: Some(song.clone()),
                tags: vec![],
                ..Default::default()
            },
        )
        .unwrap();

        comment(&conn, &profile_id, &video, "lovely");
        let posted = comment(&conn, &profile_id, &video, "when is the clip out?");
        let archived = comment(&conn, &profile_id, &video, "spam");
        for (id, state) in [(posted, comment::POSTED), (archived, comment::ARCHIVED)] {
            conn.execute(
                "UPDATE comment SET state = ?2 WHERE id = ?1",
                params![id, state],
            )
            .unwrap();
        }

        scene::create(
            &conn,
            &profile_id,
            NewScene {
                work_id: video.clone(),
                ..NewScene::default()
            },
        )
        .unwrap();

        let before = counts(&conn, &song).unwrap().history;
        journal::record(
            &conn,
            &profile_id,
            Record::new("work.renamed").about("work", &song),
        );

        let song_counts = counts(&conn, &song).unwrap();
        assert_eq!(
            song_counts.versions, 2,
            "the other song's version is its own"
        );
        assert_eq!(song_counts.releases, 0, "a song goes out as its clip");
        assert_eq!(song_counts.sources, 0);
        assert_eq!(song_counts.derived, 1);
        assert_eq!(song_counts.notes, 1);
        assert_eq!(
            song_counts.comments, 2,
            "a song sums up the comments under its clip; the archived one is not listed"
        );
        assert_eq!(
            song_counts.comments_waiting, 1,
            "the one answered waits no more"
        );
        assert_eq!(song_counts.scenes, 0);
        assert_eq!(song_counts.history, before + 1);

        let video_counts = counts(&conn, &video).unwrap();
        assert_eq!(video_counts.releases, 1);
        assert_eq!(video_counts.comments, 2, "the clip's own");
        assert_eq!(video_counts.sources, 1);
        assert_eq!(video_counts.derived, 0);
        assert_eq!(video_counts.scenes, 1);
    }

    #[test]
    fn a_donor_counts_what_was_cut_from_it() {
        let (conn, profile_id) = fixtures::workspace();
        let film = work(&conn, &profile_id, "video", "Film");
        let short = work(&conn, &profile_id, "short", "Trailer");
        // Written straight in: the counter reads rows, and the stretch's own
        // rules (a donor link, the track's length) are the cut module's.
        conn.execute(
            "INSERT INTO cut (id, profile_id, work_id, source_id, starts_at, ends_at, position,
                              created_at, updated_at)
             VALUES ('c1', ?1, ?2, ?3, 0, 10, 1, '2026-09-28', '2026-09-28')",
            params![profile_id, short, film],
        )
        .unwrap();

        let donor = counts(&conn, &film).unwrap();
        assert_eq!(donor.cut_from, 1);
        assert_eq!(donor.cuts, 0, "the film is spliced from nothing");
        let spliced = counts(&conn, &short).unwrap();
        assert_eq!(spliced.cuts, 1);
        assert_eq!(spliced.cut_from, 0);
    }

    #[test]
    fn history_counts_no_further_than_its_tab_lists() {
        let (conn, profile_id) = fixtures::workspace();
        let song = work(&conn, &profile_id, "song", "Tide");
        let page = journal::for_entity(&conn, "work", &song).unwrap().len();

        for n in 0..(journal::PAGE as usize + 5 - page) {
            journal::record(
                &conn,
                &profile_id,
                Record::new("work.renamed")
                    .param("n", n)
                    .about("work", &song),
            );
        }

        let listed = journal::for_entity(&conn, "work", &song).unwrap().len();
        assert_eq!(listed as i64, journal::PAGE);
        assert_eq!(counts(&conn, &song).unwrap().history, journal::PAGE);
    }
}
