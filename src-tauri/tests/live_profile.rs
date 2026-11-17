//! The profile upgrade of v0.112 run against a real space, by hand.
//!
//! Ignored by default: it needs a database to point at, which CI has none of.
//! Run it before a release with `KILNA_LIVE_DB` set to a *copy* of one. It
//! prints counts and keys only - a real workspace's words are the owner's.

use kilna_lib::profile::{self, config::MetaFieldType};
use kilna_lib::speech::Speech;

#[test]
#[ignore]
fn the_upgrade_of_v0_112_carries_a_real_profile_forward() {
    let Ok(path) = std::env::var("KILNA_LIVE_DB") else {
        panic!("set KILNA_LIVE_DB to a copy of a real workspace");
    };
    let conn = kilna_lib::db::open(std::path::Path::new(&path)).unwrap();
    let before = profile::active(&conn).unwrap().unwrap();
    let plain_before = plain_labels(&before.config);

    profile::seed(&conn, &before.key).unwrap();
    let after = profile::active(&conn).unwrap().unwrap();
    let config = &after.config;

    let problems = config.validate();
    println!("problems after the upgrade: {problems:?}");
    assert!(problems.is_empty());

    println!(
        "plain-string labels on kinds, statuses and axes: {plain_before} -> {}",
        plain_labels(config)
    );
    let critique = config.prompts.iter().find(|p| p.key == "critique").unwrap();
    println!("critique reads: {:?}", critique.placeholders());
    println!(
        "album-place arrived: {}",
        config.prompts.iter().any(|p| p.key == "album-place")
    );
    let anchors = config.work_meta_fields.iter().find(|f| f.key == "anchors");
    println!("anchors is {:?}", anchors.map(|field| field.field_type));
    for kind in &config.work_kinds {
        println!(
            "kind {}: {} layers, label in ru differs from en: {}",
            kind.key,
            kind.layers.len(),
            kind.label.in_locale("ru") != kind.label.in_locale("en")
        );
    }

    // What experiments hold as anchors still reads, entry by entry.
    if anchors.is_some_and(|field| field.field_type == MetaFieldType::List) {
        let mut statement = conn
            .prepare("SELECT json_extract(meta, '$.anchors') FROM work WHERE json_extract(meta, '$.anchors') IS NOT NULL")
            .unwrap();
        let held: Vec<rusqlite::types::Value> = statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let entries: usize = held
            .iter()
            .map(|value| match value {
                rusqlite::types::Value::Text(text) => {
                    let stored = serde_json::from_str(text)
                        .unwrap_or_else(|_| serde_json::Value::String(text.clone()));
                    MetaFieldType::entries(&stored).len()
                }
                _ => 0,
            })
            .sum();
        println!(
            "{} works hold anchors, {entries} entries in all",
            held.len()
        );
    }

    // Every chat reads in Russian without a failure, and how many bodies
    // were written out again from a proposal.
    let chats: Vec<String> = conn
        .prepare("SELECT id FROM chat")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut respoken = 0;
    for chat in &chats {
        let english = kilna_lib::assistant::transcript_in(&conn, chat, Speech::ENGLISH)
            .unwrap()
            .unwrap();
        let russian = kilna_lib::assistant::transcript_in(&conn, chat, Speech::of("ru"))
            .unwrap()
            .unwrap();
        respoken += english
            .messages
            .iter()
            .zip(&russian.messages)
            .filter(|(en, ru)| en.body != ru.body)
            .count();
    }
    println!(
        "{} chats read; {respoken} bodies said again in Russian",
        chats.len()
    );

    // And a second start changes nothing.
    profile::seed(&conn, &before.key).unwrap();
    let again = profile::active(&conn).unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(&again.config).unwrap(),
        serde_json::to_value(config).unwrap()
    );
}

/// How many labels of kinds, statuses and axes are one string.
fn plain_labels(config: &profile::config::ProfileConfig) -> usize {
    let value = serde_json::to_value(config).unwrap();
    let mut count = 0;
    for kind in value["work_kinds"].as_array().into_iter().flatten() {
        count += usize::from(kind["label"].is_string());
        for list in [
            "statuses",
            "axes",
            "tiers",
            "version_roles",
            "release_kinds",
        ] {
            for entry in kind[list].as_array().into_iter().flatten() {
                count += usize::from(entry["label"].is_string());
            }
        }
    }
    count
}
