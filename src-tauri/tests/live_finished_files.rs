//! Every work's finished file, found in a real workspace's folders, by hand.
//!
//! Ignored by default: it needs a workspace to point at, which CI has none of.
//! `KILNA_LIVE_DB` points at a *copy* of one; it is opened the way the window
//! opens it - migrations, the profile's upgrade, the numbers given to works
//! made before v0.101 - and every work of `KILNA_LIVE_KIND` (`short` unless
//! said) is asked for its finished file and its shot list. `KILNA_LIVE_OUT`
//! is a file the findings are written to as tab-separated lines - work,
//! number, finished file, the stretches' sum, every shot's donor file - for
//! checking the files against what is in them; nothing of the workspace is
//! printed but the counts.

use std::collections::BTreeMap;

#[test]
#[ignore]
fn every_work_finds_its_own_finished_file() {
    let Ok(path) = std::env::var("KILNA_LIVE_DB") else {
        panic!("set KILNA_LIVE_DB to a copy of a real workspace");
    };
    let kind = std::env::var("KILNA_LIVE_KIND").unwrap_or_else(|_| "short".into());
    let state = kilna_lib::state::AppState::open(std::path::Path::new(&path)).unwrap();
    let conn = state.conn();
    let profile = kilna_lib::profile::active(&conn).unwrap().unwrap();
    let config = kilna_lib::profile::config_for(&conn, &profile.id).unwrap();

    let works = kilna_lib::work::list(
        &conn,
        &profile.id,
        &kilna_lib::work::WorkFilter {
            kind: Some(kind.clone()),
            ..Default::default()
        },
    )
    .unwrap();

    let mut lines = Vec::new();
    let mut found = 0;
    let mut numbered = 0;
    let mut cuttable = 0;
    let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for work in &works {
        if work.ordinal.is_some() {
            numbered += 1;
        }
        let file = kilna_lib::folder::finished(&conn, &config, work).unwrap();
        let shots = kilna_lib::cut::shot_list(&conn, &config, &work.id).unwrap();
        if let Some(path) = &file.path {
            found += 1;
            owners
                .entry(path.clone())
                .or_default()
                .push(work.id.clone());
        }
        if !shots.is_empty() && shots.iter().all(|shot| shot.path.is_some()) {
            cuttable += 1;
        }
        let spliced: f64 = shots.iter().map(|shot| shot.ends_at - shot.starts_at).sum();
        let donors: Vec<String> = shots
            .iter()
            .map(|shot| {
                shot.path
                    .clone()
                    .unwrap_or_else(|| format!("{:?}", shot.file_state))
            })
            .collect();
        lines.push(format!(
            "{}\t{}\t{}\t{:?}\t{}\t{:.3}\t{}",
            work.id,
            work.ordinal.map_or_else(|| "-".into(), |n| n.to_string()),
            file.path.clone().unwrap_or_default(),
            file.state,
            shots.len(),
            spliced,
            donors.join(" | ")
        ));
    }

    let shared: Vec<_> = owners.iter().filter(|(_, works)| works.len() > 1).collect();
    println!(
        "{} works of kind {kind}: {numbered} numbered, {found} with a finished file, {cuttable} cuttable, {} files shared",
        works.len(),
        shared.len()
    );
    if let Ok(out) = std::env::var("KILNA_LIVE_OUT") {
        std::fs::write(out, lines.join("\n") + "\n").unwrap();
    }
    assert!(shared.is_empty(), "no two works are finished as one file");
}

/// Give works the numbers their folders on disk say, by hand.
///
/// Ignored by default, for the reason above. `KILNA_LIVE_DB` is the workspace,
/// a copy first and then the real one with the window closed, and
/// `KILNA_LIVE_NUMBERS` a file of tab-separated lines, a work's id and its
/// number. Each change is the ordinary edit of the work's number, logged and
/// undoable; numbers that trade places pass through a number no sibling holds
/// first, since a number a sibling holds is refused.
#[test]
#[ignore]
fn works_take_the_numbers_their_folders_say() {
    let (Ok(path), Ok(numbers)) = (
        std::env::var("KILNA_LIVE_DB"),
        std::env::var("KILNA_LIVE_NUMBERS"),
    ) else {
        panic!(
            "set KILNA_LIVE_DB to a workspace and KILNA_LIVE_NUMBERS to a file of id<TAB>number lines"
        );
    };
    let state = kilna_lib::state::AppState::open(std::path::Path::new(&path)).unwrap();
    let conn = state.conn();
    let wanted: Vec<(String, i64)> = std::fs::read_to_string(numbers)
        .unwrap()
        .lines()
        .filter_map(|line| {
            let (id, number) = line.split_once('\t')?;
            Some((id.trim().to_owned(), number.trim().parse().ok()?))
        })
        .collect();
    let renumber = |id: &str, number: i64| {
        kilna_lib::actions::work::update(
            &conn,
            id,
            kilna_lib::work::WorkPatch {
                ordinal: Some(Some(number)),
                ..Default::default()
            },
        )
        .unwrap();
    };
    for (id, number) in &wanted {
        renumber(id, 100_000 + number);
    }
    for (id, number) in &wanted {
        renumber(id, *number);
    }
    println!("{} works renumbered", wanted.len());
}
