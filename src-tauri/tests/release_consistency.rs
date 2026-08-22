//! Guards the facts that must agree across every shop window before a release.
//!
//! Versions drift between manifests the moment one of them is bumped by hand,
//! and a second README appears the moment someone needs a shorter one. Both
//! failures are only visible after publishing, so they are checked here.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri always has a parent")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// First `"version": "..."` value in a JSON document.
fn json_version(source: &str) -> String {
    source
        .lines()
        .find_map(|line| {
            let rest = line.trim().strip_prefix("\"version\"")?;
            let value = rest.trim_start_matches([':', ' ']);
            value.trim_matches(['"', ',']).to_owned().into()
        })
        .expect("no `version` field found")
}

#[test]
fn every_manifest_declares_the_same_version() {
    let crate_version = env!("CARGO_PKG_VERSION");

    assert_eq!(
        json_version(&read("package.json")),
        crate_version,
        "package.json disagrees with Cargo.toml"
    );
    assert_eq!(
        json_version(&read("src-tauri/tauri.conf.json")),
        crate_version,
        "tauri.conf.json disagrees with Cargo.toml — the installer would carry the wrong version"
    );

    // Cargo.lock is deliberately not checked here: `cargo test` rewrites it to
    // match Cargo.toml before this test can read it, so the assertion could
    // never fail — it would only look like a check. A stale lockfile is caught
    // by the working tree not being clean after a build, which is where the
    // v0.21 slip was actually found.
}

#[test]
fn the_changelog_documents_the_current_version() {
    let changelog = read("CHANGELOG.md");
    let heading = format!("[{}]", env!("CARGO_PKG_VERSION"));

    assert!(
        changelog.contains(&heading),
        "CHANGELOG.md has no section for {heading}; run `git-cliff --tag v{}`",
        env!("CARGO_PKG_VERSION")
    );
}

#[test]
fn there_is_exactly_one_readme() {
    let extra = ["docs/README.md", "src-tauri/README.md", "npm/README.md"];

    for candidate in extra {
        assert!(
            !repo_root().join(candidate).exists(),
            "{candidate} is a second README; the root one is the only source"
        );
    }
}

#[test]
fn readme_links_are_absolute() {
    let readme = read("README.md");

    // A relative link works on GitHub and breaks everywhere the README is
    // republished — crates.io, npm, the docs site.
    for line in readme.lines() {
        let Some(start) = line.find("](") else {
            continue;
        };
        let target = &line[start + 2..];
        let target = &target[..target.find(')').unwrap_or(target.len())];

        assert!(
            target.starts_with("http") || target.starts_with('#'),
            "relative link `{target}` in README.md"
        );
    }
}

/// The brand assets must be the registry's masters, byte for byte.
///
/// The failure this prevents is subtle: an icon-level tile used where the
/// large one belongs reads as a solid colour block, and nobody notices until
/// the site is live.
#[test]
fn the_brand_assets_match_their_masters() {
    let registry = Path::new("C:/Projects/obsidian/lacodda/Projects/brand/svg");
    if !registry.exists() {
        // The registry is the author's vault; CI checks out only this repo.
        return;
    }

    for (asset, master) in [
        ("assets/logo.svg", "kilna-L.svg"),
        ("assets/logo-m.svg", "kilna-M.svg"),
        ("assets/logo-s.svg", "kilna-S.svg"),
        ("assets/banner.svg", "kilna-banner.svg"),
    ] {
        let ours = std::fs::read(repo_root().join(asset)).expect("asset is missing");
        let theirs = std::fs::read(registry.join(master)).expect("master is missing");

        assert_eq!(
            ours, theirs,
            "{asset} does not match {master} — re-export it from the generator"
        );
    }
}

/// Every ADR is numbered and unique, so a decision cannot quietly overwrite an
/// earlier one by reusing its number.
#[test]
fn adr_numbers_are_unique() {
    let adr = repo_root().join("docs/adr");
    let Ok(entries) = std::fs::read_dir(&adr) else {
        return;
    };

    let mut numbers: Vec<String> = entries
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_owned();
            name.split('-').next().map(str::to_owned)
        })
        .filter(|prefix| prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty())
        .collect();

    let count = numbers.len();
    numbers.sort();
    numbers.dedup();

    assert_eq!(numbers.len(), count, "two ADRs share a number");
}

#[test]
fn the_declared_msrv_is_a_real_number() {
    let manifest = read("src-tauri/Cargo.toml");

    let declared = manifest
        .lines()
        .find_map(|line| line.trim().strip_prefix("rust-version"))
        .map(|rest| {
            rest.trim_start_matches([' ', '='])
                .trim_matches('"')
                .to_owned()
        })
        .expect("Cargo.toml declares no rust-version");

    // The CI `msrv` job proves the build works on it; this only catches a
    // malformed or accidentally cleared value.
    let parts: Vec<_> = declared.split('.').collect();
    assert!(
        parts.len() >= 2 && parts.iter().all(|p| p.parse::<u32>().is_ok()),
        "`{declared}` is not a version number"
    );
}


/// Taking a date must go through the contest, never through a plain edit.
///
/// `update_release` writes a date without asking who holds it, which is right
/// for editing a booking and wrong for claiming one. v0.24 wired the calendar's
/// drag-and-drop to it and shipped a build where dragging one release onto
/// another's day left both there, the rule quietly bypassed.
///
/// Read from the source because the alternative is a DOM test, and the frontend
/// runner has no DOM until the 0.43 block.
#[test]
fn the_calendar_claims_dates_through_the_contest() {
    let source = read("src/components/CalendarView.tsx");

    let handler = source
        .find("onMove=")
        .map(|at| &source[at..(at + 200).min(source.len())])
        .expect("the calendar passes an onMove handler to the grid");

    let mutation = handler
        .find("move.mutate")
        .map(|_| "move")
        .expect("onMove goes through the `move` mutation");

    // The mutation itself must call scheduleRelease, not updateRelease.
    let body = source
        .find(&format!("const {mutation} = useMutation"))
        .map(|at| &source[at..(at + 400).min(source.len())])
        .expect("the move mutation is declared");

    assert!(
        body.contains("scheduleRelease"),
        "dragging a release must claim its date through `scheduleRelease`, so a          held day is contested rather than shared"
    );
    assert!(
        !body.contains("updateRelease"),
        "dragging a release must not write the date through `updateRelease`,          which does not look at who holds the day"
    );
}
