//! A channel brought in from a made-up folder: the shapes the owner's files
//! are written in, none of their words.

use rusqlite::Connection;
use serde_json::json;

use super::{apply, cooldown_of, plan, read};

const README: &str = "# DEMO — Demo Channel

## Identity

- A channel of night drives.
- One track every other day.

## Main rule: no tradition twice

Never repeat a tradition within three months.

## Literary register of descriptions

Short, warm, second person.

## Visual engine

### Artists (for cover prompt)

- Painter One
- Painter Two

### Banned in covers

- Neon signs

## Approved playlists

### Regions

- Night Roads
- Coastlines
";

const REGISTER: &str = "# DEMO — register

## Stop-list of patterns

| # | Pattern | Frequency | Rule |
|---|---------|-----------|------|
| 1 | **«cyber-city»** as a default scene | 12 | once a month at most |
| 2 | holo-prefix | 7 | never |
";

const BANK: &str = "# Bank of traditions

> Taken traditions rest 3 months before they return.

## Taken: plan 2027

- **2027-01:** Basque · Atlantis

## Reference

### Europe

| Region | Instruments | Devices | Character |
|--------|-------------|---------|-----------|
| **Basque** | txalaparta, alboka | duels | playful |
| Breton | bombarde, biniou | drones | stormy |
";

const PLAN: &str = "# DEMO — Plan

## 2027-01 ✅ — «Cold start» (2 tracks)

Arc: the month warms up.

| Date | Title | Tradition × Genre | BPM·key·mood | Sound | Cover | Artist | Playlists |
|------|-------|-------------------|--------------|-------|-------|--------|-----------|
| 01-05 Tue | Two Hands 🎙 | Basque (txalaparta) × Broken Beat 🇪🇺 | 126 · dorian · playful | planks duel | A beach at dusk | Painter One | Night Roads |
| 01-07 Thu 📌 | Storm Pipes | Breton × Techno | 130 · minor · stormy | bombarde | A lighthouse | Painter Two | Coastlines |
";

const PACK: &str = "# 2027-01-05 (Demo Channel – Two Hands (Basque x Broken Beat))

### Title

```
Demo Channel – Two Hands (Basque x Broken Beat)
```

### Description

```
Two planks, one beat. #Demo
```

### Duration

```
3:05
```

### Suno prompt

```
txalaparta duel, broken beat, warm sub
```

### Cover prompt

```
two hands striking wooden planks on a beach at dusk
```

### Playlists

- Night Roads
- Coastlines
";

fn folder() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("README.md"), README).unwrap();
    std::fs::write(dir.path().join("Реестр самоповторов.md"), REGISTER).unwrap();
    std::fs::write(dir.path().join("Банк традиций.md"), BANK).unwrap();
    std::fs::write(dir.path().join("План.md"), PLAN).unwrap();
    std::fs::create_dir(dir.path().join("Подборки")).unwrap();
    std::fs::write(dir.path().join("Подборки").join("DEMO - 2027-01.md"), PACK).unwrap();
    dir
}

fn channel_space() -> (Connection, String) {
    let conn = crate::db::open_in_memory().unwrap();
    crate::profile::seed(&conn, "track-channel").unwrap();
    let profile_id = crate::profile::active(&conn).unwrap().unwrap().id;
    (conn, profile_id)
}

/// The files are read by their shapes: sections, tables, months, packs.
#[test]
fn a_channels_folder_is_read_by_its_shapes() {
    let dir = folder();
    let channel = read::read(dir.path()).unwrap();
    assert_eq!(channel.patterns.len(), 2);
    assert_eq!(
        channel.patterns[0].pattern,
        "«cyber-city» as a default scene"
    );
    assert_eq!(channel.traditions.len(), 2);
    assert_eq!(channel.traditions[0].name, "Basque");
    assert_eq!(channel.traditions[0].group, "Europe");
    assert_eq!(
        channel.traditions[0].attributes,
        vec!["txalaparta", "alboka", "duels"]
    );
    assert_eq!(
        channel.taken,
        vec![(
            "2027-01".to_owned(),
            vec!["Basque".to_owned(), "Atlantis".to_owned()]
        )]
    );
    assert_eq!(channel.plan.len(), 1);
    let month = &channel.plan[0];
    assert_eq!(month.title.as_deref(), Some("Cold start"));
    assert_eq!(month.arc.as_deref(), Some("the month warms up."));
    assert_eq!(month.tracks[0].date, "2027-01-05");
    assert_eq!(month.tracks[0].title, "Two Hands");
    assert!(month.tracks[0].vocal);
    assert!(month.tracks[1].peak);
    assert_eq!(channel.packs.len(), 1);
    assert_eq!(cooldown_of(BANK), Some(3));
}

/// Planned and applied: the passport, the register, the bank with its rest
/// and last uses, the month's collection, its tracks planned or booked with
/// what their pack says; applied again, nothing is written twice.
#[test]
fn a_channel_is_planned_and_applied_once() {
    let (conn, profile_id) = channel_space();
    let dir = folder();
    let channel = read::read(dir.path()).unwrap();
    let mut planned = plan(&conn, &profile_id, &channel).unwrap();
    planned.cooldown_months = cooldown_of(BANK);
    assert_eq!(planned.rules, vec!["Main rule: no tradition twice"]);
    assert!(
        planned
            .facts
            .iter()
            .any(|f| f.section == "artists" && f.body == "Painter One")
    );
    assert!(
        planned
            .facts
            .iter()
            .any(|f| f.section == "bans" && f.body == "Neon signs")
    );
    assert!(planned.facts.iter().any(|f| f.section == "playlists"
        && f.body == "Coastlines"
        && f.slot.as_deref() == Some("Regions")));
    assert!(planned.facts.iter().any(|f| f.section == "voice"));
    let two_hands = &planned.months[0].tracks[0];
    assert_eq!(two_hands.fields["tradition"], json!("Basque (txalaparta)"));
    assert_eq!(two_hands.fields["genre"], json!("Broken Beat"));
    assert_eq!(two_hands.fields["bpm"], json!(126.0));
    assert_eq!(two_hands.fields["duration"], json!("3:05"));
    assert_eq!(
        two_hands.suno.as_deref(),
        Some("txalaparta duel, broken beat, warm sub")
    );
    assert_eq!(
        two_hands.release["playlists"],
        json!("Night Roads, Coastlines")
    );
    assert!(planned.conflicts.is_empty(), "{:?}", planned.conflicts);

    let done = apply(&conn, &planned).unwrap();
    assert_eq!(done.collections, 1);
    assert_eq!(done.works, 2);
    assert_eq!(
        done.releases, 1,
        "the packed track is booked; the other is planned"
    );
    assert_eq!(done.terms, 2);
    assert_eq!(done.traditions, 3, "two of the tables and one only taken");

    let works = crate::score::catalogue(&conn, &profile_id).unwrap();
    let storm = works.iter().find(|w| w.title == "Storm Pipes").unwrap();
    assert!(
        crate::work_plan::of_work(&conn, &storm.work_id)
            .unwrap()
            .is_some()
    );
    let root = crate::note::root_card(
        &conn,
        &profile_id,
        &crate::profile::config_for(&conn, &profile_id).unwrap(),
    )
    .unwrap()
    .unwrap();
    let basque = crate::canon::fact::in_section(&conn, &root.id, "bank")
        .unwrap()
        .into_iter()
        .find(|f| f.body == "Basque")
        .unwrap();
    assert_eq!(basque.data["cooldown_months"], json!(3));
    assert_eq!(basque.data["last_used_at"], json!("2027-01-15"));

    // Again: the works are there by title, the facts by their words.
    let again = plan(&conn, &profile_id, &channel).unwrap();
    assert!(again.months[0].tracks.iter().all(|t| t.existing.is_some()));
    let twice = apply(&conn, &again).unwrap();
    assert_eq!(
        (twice.works, twice.facts, twice.terms, twice.traditions),
        (0, 0, 0, 0)
    );
}
