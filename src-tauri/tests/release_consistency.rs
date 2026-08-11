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
