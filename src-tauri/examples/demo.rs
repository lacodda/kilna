//! Fills a workspace with invented demo content, for screenshots and for
//! trying the app without typing everything by hand.
//!
//!   cargo run --example demo -- <path-to-kilna.db>
//!
//! The content is fictional on purpose: nothing from a real catalogue belongs
//! in this repository.

use kilna_lib::note::NewNote;
use kilna_lib::release::{self, NewRelease};
use kilna_lib::score::{self, NewScore};
use kilna_lib::work::NewWork;
use kilna_lib::work::version::NewVersion;
use kilna_lib::{db, note, profile, work};
use serde_json::json;

struct Demo {
    title: &'static str,
    status: &'static str,
    bpm: i64,
    key: &'static str,
    lyrics: &'static [&'static str],
    style: &'static str,
    note: Option<(&'static str, &'static [&'static str])>,
    /// Axis values, in the order the Music profile declares them. A work with
    /// no scores stays unjudged, which the catalogue shows differently.
    scores: &'static [[f64; 6]],
    /// Release kind to plan, and the slot to claim if any.
    release: Option<(&'static str, Option<&'static str>)>,
}

const AXIS_KEYS: [&str; 6] = [
    "hook",
    "lyrics",
    "emotion",
    "production",
    "originality",
    "visual",
];

const WORKS: &[Demo] = &[
    Demo {
        title: "Harbour lights",
        status: "scored",
        bpm: 96,
        key: "Am",
        lyrics: &[
            "The cranes go still at seven\nand the water keeps the noise.\nI count the lights across the bay\nthe way I count your voice.",
            "The cranes go still at seven.\nThe water keeps the noise.\nI count the lights across the bay\nthe way I once counted your voice.",
        ],
        style: "slow indie folk, brushed drums, upright bass, close vocal, room reverb",
        note: Some((
            "The second verse still explains itself. Cut the last line and let the image stand.",
            &["revision", "lyrics"],
        )),
        // Scored once before the rewrite and once after — the second verse
        // fix is meant to be visible as a jump in the total.
        scores: &[
            [7.0, 6.0, 7.0, 6.0, 7.0, 8.0],
            [8.0, 8.5, 8.0, 6.5, 7.0, 8.0],
        ],
        release: Some(("clip", Some("2026-09-11"))),
    },
    Demo {
        title: "Paper boats",
        status: "draft",
        bpm: 120,
        key: "F",
        lyrics: &["We fold the year in halves\nand set it on the stream."],
        style: "bright synth pop, arpeggiated bass, tape saturation",
        note: None,
        scores: &[],
        // Planned but unscored — it sits at the back of the queue, and cannot
        // take a slot from anything that has been judged.
        release: Some(("audio", None)),
    },
    Demo {
        title: "Winter shift",
        status: "scheduled",
        bpm: 88,
        key: "Dm",
        lyrics: &["Six o'clock, the gate is cold.\nThe shift is somebody else's now."],
        style: "downtempo, felt piano, low strings, sparse percussion",
        note: Some((
            "Works as a picture track — one still frame, no clip.",
            &["release"],
        )),
        scores: &[[6.0, 7.5, 8.0, 7.0, 6.0, 4.0]],
        release: Some(("audio", Some("2026-09-04"))),
    },
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: demo <path-to-kilna.db>")?;

    let mut conn = db::open(std::path::Path::new(&path))?;
    profile::seed(&conn)?;
    let profile_id = profile::active(&conn)?.ok_or("no active profile")?.id;

    for demo in WORKS {
        let created = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: demo.title.into(),
                status: Some(demo.status.into()),
                collection_id: None,
                meta: json!({ "bpm": demo.bpm, "key": demo.key, "language": "English" })
                    .as_object()
                    .cloned(),
            },
        )?;

        // A draft, then the score it earned — in that order, so each snapshot
        // pins to the revision it actually describes.
        for (index, body) in demo.lyrics.iter().enumerate() {
            work::version::create(
                &mut conn,
                &created.id,
                NewVersion {
                    role: "lyrics".into(),
                    body: (*body).into(),
                    label: None,
                    meta: None,
                    make_current: true,
                },
            )?;

            if let Some(values) = demo.scores.get(index) {
                let axes = AXIS_KEYS
                    .iter()
                    .zip(values)
                    .map(|(key, value)| ((*key).to_owned(), json!(value)))
                    .collect();

                score::create(
                    &conn,
                    &created.id,
                    NewScore {
                        axes,
                        version_id: None,
                        note: None,
                    },
                )?;
            }
        }

        work::version::create(
            &mut conn,
            &created.id,
            NewVersion {
                role: "style".into(),
                body: demo.style.into(),
                label: None,
                meta: None,
                // The lyrics stay the work's current version; the style is a
                // parallel body, not a replacement.
                make_current: false,
            },
        )?;

        if let Some((body, tags)) = demo.note {
            note::create(
                &conn,
                &profile_id,
                NewNote {
                    body: body.into(),
                    kind: None,
                    title: None,
                    work_id: Some(created.id.clone()),
                    tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
                },
            )?;
        }

        if let Some((kind, slot)) = demo.release {
            let planned = release::create(
                &conn,
                NewRelease {
                    work_id: created.id.clone(),
                    kind: kind.into(),
                    title: Some(demo.title.into()),
                    scheduled_at: None,
                    meta: None,
                },
            )?;

            if let Some(slot) = slot {
                release::schedule(&mut conn, &planned.id, slot)?;
            }
        }

        println!("added {}", demo.title);
    }

    Ok(())
}
