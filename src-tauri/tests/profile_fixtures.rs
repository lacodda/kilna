//! The shipped profiles, as the window receives them, kept beside the
//! frontend's tests.
//!
//! The frontend's component tests draw a workspace in the vocabulary of a real
//! craft - its kinds, roles, axes, stages - and that vocabulary is not the
//! file in `profiles/`: the file is read through `RawProfileConfig`, which
//! migrates older shapes, and what reaches the window is the document after
//! that. A copy written by hand would be a third shape, right on the day it
//! was typed and quietly wrong from the first change to either side.
//!
//! So the copy is generated from the reading itself and checked here: a change
//! to a shipped profile or to how it is read fails this test until the copy is
//! written again, with
//!
//!     KILNA_BLESS=1 cargo test --test profile_fixtures

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use kilna_lib::profile;
use kilna_lib::profile::config::ProfileConfig;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri always has a parent")
        .join("src/test/fixtures/profiles")
}

/// A profile's document the way the window gets it: stored as text, read back
/// through the same deserialiser, and serialised again on the way out.
fn as_the_window_reads_it(shipped: &profile::BuiltinProfile) -> String {
    let stored = serde_json::to_string(&shipped.config).expect("a shipped config serialises");
    let read: ProfileConfig = serde_json::from_str(&stored).expect("a stored config reads back");
    let document = serde_json::json!({
        "key": shipped.key,
        "name": shipped.name,
        "description": shipped.description,
        "config": read,
    });
    serde_json::to_string_pretty(&document).expect("a document serialises") + "\n"
}

#[test]
fn the_frontend_fixtures_are_the_shipped_profiles() {
    let dir = fixtures_dir();
    let bless = std::env::var_os("KILNA_BLESS").is_some();
    if bless {
        std::fs::create_dir_all(&dir).expect("the fixtures directory can be made");
    }

    let shipped = profile::builtin().expect("the shipped profiles read");
    assert!(
        shipped.len() >= 4,
        "read only {} shipped profiles - the fixtures would cover less than the app ships",
        shipped.len()
    );

    let mut stale = Vec::new();
    for profile in &shipped {
        let path = dir.join(format!("{}.json", profile.key));
        let expected = as_the_window_reads_it(profile);
        if bless {
            std::fs::write(&path, &expected).expect("the fixture is writable");
            continue;
        }
        let written = std::fs::read_to_string(&path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        if written != expected {
            stale.push(profile.key.clone());
        }
    }

    assert!(
        stale.is_empty(),
        "the frontend's copies of these profiles no longer match what the window receives: \
         {stale:?}\nWrite them again with `KILNA_BLESS=1 cargo test --test profile_fixtures`."
    );

    // And nothing there that the app does not ship: a profile removed from
    // the binary would otherwise go on being tested as if it were real.
    let keys: BTreeSet<String> = shipped.iter().map(|p| format!("{}.json", p.key)).collect();
    let strays: Vec<String> = std::fs::read_dir(&dir)
        .expect("the fixtures directory is readable")
        .flatten()
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .filter(|name| !keys.contains(name))
        .collect();
    assert!(
        strays.is_empty(),
        "these profile fixtures are not shipped profiles: {strays:?}"
    );
}
