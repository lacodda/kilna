//! A demo corpus for a space that wants one (v0.129, FR-16): a handful of
//! works at every stage, versions to compare, a score, notes, a card of the
//! canon and releases on the calendar of the coming days - so a new person
//! sees each screen with something on it.
//!
//! Made up, neutral and in English: no real person, no real title, nothing of
//! anyone's own (the privacy rule of the line). Written through the same
//! gestures a person makes, so every bit of it is in the history and can be
//! undone or deleted like anything else; the dates are counted from today, so
//! the calendar is never a page of the past.

use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::actions;
use crate::error::{Error, Result};
use crate::note::NewNote;
use crate::profile::config::ProfileConfig;
use crate::score::NewScore;
use crate::work::version::NewVersion;
use crate::work::{NewWork, WorkPatch};

/// What a demo put into a space.
#[derive(Debug, Clone, Default, Serialize, ts_rs::TS)]
pub struct DemoReport {
    pub works: usize,
    pub notes: usize,
    pub releases: usize,
}

/// One made-up work: its title, how far along it is, and its text - in
/// versions, oldest first, when it has any.
struct Piece {
    title: &'static str,
    stage: i64,
    versions: &'static [&'static str],
}

/// The words of one craft's demo.
struct Corpus {
    pieces: &'static [Piece],
    /// The text of the brief, for the piece still at its idea.
    brief: &'static str,
    notes: &'static [(&'static str, &'static str)],
    /// A card of the canon: its kind, name and what is known of it.
    card: Option<(&'static str, &'static str, &'static str)>,
}

const STUDIO: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Paper Lanterns",
            stage: 100,
            versions: &[
                "We hung paper lanterns on the fence\nand the wind took two of them by ten\n\nCount the ones that stayed\ncount the ones that stayed",
                "We hung paper lanterns on the fence\nthe wind took two before the rain came in\n\nCount the ones that stayed lit\ncount the ones that stayed",
            ],
        },
        Piece {
            title: "Low Tide Hours",
            stage: 75,
            versions: &[
                "The harbour shows its ribs at noon\nboats lean on mud like tired men\nI keep the hours the water keeps\nand wait for it to come again",
            ],
        },
        Piece {
            title: "Glass Orchard",
            stage: 50,
            versions: &[
                "Apples of glass on a wire tree\nring when the bus goes past\n(second verse - something about the frost)",
            ],
        },
        Piece {
            title: "Northbound",
            stage: 25,
            versions: &[],
        },
        Piece {
            title: "A song about the lift",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## What it is about\nA night train going north, and someone who will not say why they are on it.\n\n## Mood\nQuiet, steady, a little cold.",
    notes: &[
        ("phrase", "the ones that stayed lit"),
        ("phrase", "boats lean on mud like tired men"),
        (
            "note",
            "A song in which nothing happens but the light changes. Maybe the lift song.",
        ),
    ],
    card: Some((
        "channel",
        "The demo channel",
        "A small made-up studio that writes quiet songs about ordinary places: harbours, orchards, night trains.",
    )),
};

const NOVEL: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Chapter 1. The Keeper's Daughter",
            stage: 100,
            versions: &[
                "The lamp at Gull Point had not gone out in forty years. Mira checked it anyway, every night, the way her father had.",
                "The lamp at Gull Point had not gone out in forty years. Mira climbed the ninety steps anyway, every night, the way her father had, counting them under her breath.",
            ],
        },
        Piece {
            title: "Chapter 2. The Letter",
            stage: 50,
            versions: &[
                "The letter came on a Tuesday, which was wrong: the boat only came on Fridays.",
            ],
        },
        Piece {
            title: "Chapter 3",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## What happens\nMira finds out who sent the letter.\n\n## What it changes\nShe stops counting the steps.",
    notes: &[
        ("phrase", "counting them under her breath"),
        (
            "note",
            "Does the boat ever come on time? Decide before chapter 4.",
        ),
    ],
    card: Some((
        "character",
        "Mira",
        "Keeps the lighthouse at Gull Point after her father. Counts things when she is afraid.",
    )),
};

const PODCAST: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Episode 1. Starting From Silence",
            stage: 100,
            versions: &[
                "Cold open: the sound of an empty room. HOST: This is what our studio sounded like before the first episode. Today: why we almost didn't start.",
            ],
        },
        Piece {
            title: "Episode 2. The Guest Who Never Came",
            stage: 50,
            versions: &[
                "HOST: We had a guest booked. We had questions. We had coffee. Here is what happened instead.",
            ],
        },
        Piece {
            title: "Episode 3",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## The question\nWhat do you do with the hour a guest leaves empty?\n\n## The way in\nStart with the coffee going cold.",
    notes: &[
        ("phrase", "here is what happened instead"),
        (
            "note",
            "A segment of listener questions, once there are listeners.",
        ),
    ],
    card: Some((
        "guest",
        "A guest to invite",
        "A made-up guest: a lighthouse keeper who records the sea every night.",
    )),
};

const BLOG: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Why Small Tools Win",
            stage: 100,
            versions: &[
                "Small tools do one thing. That is the whole argument, and it is enough.",
                "Small tools do one thing, and you can see all of it. That is the whole argument, and it turns out to be enough.",
            ],
        },
        Piece {
            title: "A Week Without Notifications",
            stage: 50,
            versions: &[
                "Day one: I turned everything off. Day two: I turned one thing back on, and here is why.",
            ],
        },
        Piece {
            title: "Notes on Writing Every Day",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## The point\nWriting every day matters less than writing on the bad days.\n\n## For whom\nAnyone who stopped after a week.",
    notes: &[
        ("phrase", "you can see all of it"),
        (
            "note",
            "A post on what to cut from a draft, with a real draft cut on the page.",
        ),
    ],
    card: None,
};

const POETRY: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Inventory of a Kitchen at Night",
            stage: 100,
            versions: &[
                "the kettle cooling\nthe knife asleep on the board\none cup turned down",
                "the kettle cooling\nthe knife asleep on its board\none cup face down, waiting",
            ],
        },
        Piece {
            title: "Winter Ferry",
            stage: 50,
            versions: &[
                "The ferry leaves at six and no one waves.\nThe water folds the town back into grey.",
            ],
        },
        Piece {
            title: "A poem about the stairwell",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## The image it starts from\nA light left on in an empty stairwell.\n\n## What it turns on\nWho it was left on for.",
    notes: &[
        ("phrase", "one cup face down, waiting"),
        ("note", "Try the ferry poem in couplets."),
    ],
    card: None,
};

const NEWSLETTER: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Issue 1. Why I Started This",
            stage: 100,
            versions: &[
                "Hello. This is a letter about small tools, sent once a week.",
                "Hello. This is a weekly letter about small tools - the ones you can see all of.",
            ],
        },
        Piece {
            title: "Issue 2. Three Tools I Stopped Using",
            stage: 50,
            versions: &[
                "This week: three tools I dropped, and the one thing each of them did better than its replacement.",
            ],
        },
        Piece {
            title: "Issue 3",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## What this issue is about\nThe tool I use every day and never mention.\n\n## The one thing to take away\nBoring tools are the ones that last.",
    notes: &[
        ("phrase", "the ones you can see all of"),
        ("note", "A reader asked about backups - an issue on that."),
    ],
    card: None,
};

/// Put the demo of the space's craft into it. Refused for a space that
/// already holds works: a demo is a way to look around a new space, never
/// something mixed into a person's own.
pub fn seed(conn: &Connection) -> Result<DemoReport> {
    let works: i64 = conn.query_row("SELECT count(*) FROM work", [], |row| row.get(0))?;
    if works > 0 {
        return Err(Error::refused("demo.notEmpty"));
    }
    let profile =
        crate::profile::active(conn)?.ok_or_else(|| Error::refused("profile.noneActive"))?;
    let corpus = match profile.key.as_str() {
        "music" => &STUDIO,
        "novel" => &NOVEL,
        "podcast" => &PODCAST,
        "blog" => &BLOG,
        "poetry" => &POETRY,
        "newsletter" => &NEWSLETTER,
        _ => &BLOG,
    };
    let config = &profile.config;
    let kind = first_writing_kind(config).ok_or_else(|| Error::refused("demo.noKind"))?;
    let mut report = DemoReport::default();
    let mut finished = None;

    for piece in corpus.pieces {
        let work = actions::work::create(
            conn,
            NewWork {
                kind: kind.clone(),
                title: piece.title.to_owned(),
                ..NewWork::default()
            },
        )?;
        report.works += 1;
        let role = config
            .vocabulary(&kind)
            .version_roles
            .first()
            .map(|r| r.key.clone());
        if let Some(role) = &role {
            for (index, body) in piece.versions.iter().enumerate() {
                actions::version::create(
                    conn,
                    &work.id,
                    NewVersion {
                        role: role.clone(),
                        body: (*body).to_owned(),
                        label: None,
                        meta: None,
                        make_current: index + 1 == piece.versions.len(),
                        parent_version_id: None,
                        trial_id: None,
                    },
                )?;
            }
        }
        // The piece still at its idea carries its brief, where the kind
        // prepares a work on a conception.
        if piece.versions.is_empty()
            && piece.stage > 0
            && let Some(conception) = config.conception(&kind)
        {
            actions::version::create(
                conn,
                &work.id,
                NewVersion {
                    role: conception.brief.clone(),
                    body: corpus.brief.to_owned(),
                    label: None,
                    meta: None,
                    make_current: true,
                    parent_version_id: None,
                    trial_id: None,
                },
            )?;
        }
        actions::work::update(
            conn,
            &work.id,
            WorkPatch {
                stage: Some(Some(piece.stage)),
                ..WorkPatch::default()
            },
        )?;
        if piece.stage == 100 && !piece.versions.is_empty() {
            score(conn, config, &kind, &work.id)?;
            finished = Some(work.id.clone());
        }
    }

    for (note_kind, body) in corpus.notes {
        let known = config.note_kind(note_kind).is_some();
        actions::note::create(
            conn,
            NewNote {
                body: (*body).to_owned(),
                kind: known.then(|| (*note_kind).to_owned()),
                ..NewNote::default()
            },
        )?;
        report.notes += 1;
    }
    if let Some((card_kind, name, body)) = corpus.card
        && config.card_kind(card_kind).is_some()
    {
        actions::canon::create_card(
            conn,
            NewNote {
                body: body.to_owned(),
                kind: Some(card_kind.to_owned()),
                title: Some(name.to_owned()),
                ..NewNote::default()
            },
        )?;
        report.notes += 1;
    }

    // What the finished piece goes out as, booked for the coming days.
    if let Some(source) = finished {
        let made_of: Vec<String> = config
            .work_kinds
            .iter()
            .filter(|k| k.key != kind && k.has_doors() && k.lab.is_none())
            .map(|k| k.key.clone())
            .take(2)
            .collect();
        for (offset, made_kind) in made_of.iter().enumerate() {
            let made = actions::work::derive(conn, &source, made_kind, None, Some("en"), None)?;
            report.works += 1;
            if let Some(release) = made.release_id {
                let day = days_from_today(3 + 4 * offset as i64);
                actions::release::schedule(conn, &release, &day)?;
                report.releases += 1;
            }
        }
    }
    Ok(report)
}

/// The kind a craft writes in: the first one with a role to write into.
fn first_writing_kind(config: &ProfileConfig) -> Option<String> {
    config
        .work_kinds
        .iter()
        .find(|kind| !config.vocabulary(&kind.key).version_roles.is_empty() && kind.lab.is_none())
        .or_else(|| config.work_kinds.first())
        .map(|kind| kind.key.clone())
}

/// A judgement of the finished piece: every axis a little above the middle
/// of its scale, said to be the demo's.
fn score(conn: &Connection, config: &ProfileConfig, kind: &str, work_id: &str) -> Result<()> {
    let axes: Map<String, Value> = config
        .vocabulary(kind)
        .axes
        .iter()
        .map(|axis| {
            let value = (axis.scale * 0.7).round();
            (axis.key.clone(), json!(value))
        })
        .collect();
    if axes.is_empty() {
        return Ok(());
    }
    actions::score::create(
        conn,
        work_id,
        NewScore {
            axes,
            version_id: None,
            note: Some("A demo score: every axis a little above the middle.".into()),
            rater: None,
        },
    )?;
    Ok(())
}

/// A day `days` from today, as the calendar writes it.
fn days_from_today(days: i64) -> String {
    let day = time::OffsetDateTime::now_utc().date() + time::Duration::days(days);
    day.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, profile};

    fn space(kind: &str) -> Connection {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn, kind).unwrap();
        conn
    }

    #[test]
    fn every_shipped_craft_gets_a_demo_with_something_on_each_screen() {
        for kind in ["music", "novel", "podcast", "blog", "poetry", "newsletter"] {
            let conn = space(kind);
            let report = seed(&conn).unwrap();
            assert!(report.works >= 3, "{kind}: {report:?}");
            assert!(report.notes >= 2, "{kind}: {report:?}");
            let scored: i64 = conn
                .query_row("SELECT count(*) FROM work_score", [], |row| row.get(0))
                .unwrap();
            assert_eq!(scored, 1, "{kind}: one finished piece is judged");
            let stages: Vec<i64> = conn
                .prepare("SELECT DISTINCT stage FROM work ORDER BY stage")
                .unwrap()
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap();
            assert!(
                stages.contains(&0) && stages.contains(&100),
                "{kind}: {stages:?}"
            );
        }
    }

    #[test]
    fn studio_s_demo_books_its_releases_in_the_coming_days() {
        let conn = space("music");
        let report = seed(&conn).unwrap();
        assert!(report.releases >= 1, "{report:?}");
        let today = time::OffsetDateTime::now_utc().date().to_string();
        let booked: Vec<String> = conn
            .prepare("SELECT scheduled_at FROM release WHERE scheduled_at IS NOT NULL")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(!booked.is_empty());
        assert!(
            booked.iter().all(|day| day.as_str() > today.as_str()),
            "{booked:?}"
        );
    }

    #[test]
    fn a_demo_is_never_mixed_into_a_space_with_works() {
        let conn = space("blog");
        seed(&conn).unwrap();
        assert_eq!(
            seed(&conn).unwrap_err().refusal().map(|r| r.code),
            Some("demo.notEmpty")
        );
    }
}
