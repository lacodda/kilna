//! Fills a workspace with invented demo content, for screenshots and for
//! trying the app without typing everything by hand.
//!
//!   cargo run --example demo -- <path-to-kilna.db>
//!
//! The content is fictional on purpose: nothing from a real catalogue belongs
//! in this repository.

use kilna_lib::note::NewNote;
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
}

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
    },
    Demo {
        title: "Paper boats",
        status: "draft",
        bpm: 120,
        key: "F",
        lyrics: &["We fold the year in halves\nand set it on the stream."],
        style: "bright synth pop, arpeggiated bass, tape saturation",
        note: None,
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

        for body in demo.lyrics {
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

        println!("added {}", demo.title);
    }

    Ok(())
}
