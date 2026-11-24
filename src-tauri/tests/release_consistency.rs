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
///
/// The registry is the author's, outside this repository, and is named by
/// `KILNA_BRAND_REGISTRY` - the folder of the line's master SVGs. Without it
/// (CI checks out only this repository) there is nothing to compare against.
#[test]
fn the_brand_assets_match_their_masters() {
    let Some(registry) = std::env::var_os("KILNA_BRAND_REGISTRY").map(std::path::PathBuf::from)
    else {
        return;
    };
    assert!(
        registry.is_dir(),
        "KILNA_BRAND_REGISTRY names {}, which is no folder",
        registry.display()
    );

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

/// The icon the application ships with is the one in `assets/`.
///
/// The masters gate above compares SVGs against the vault, which CI cannot see.
/// This compares two files that are both in the repo, and it exists because the
/// thing it checks actually happened: `src-tauri/icons/` was generated once by
/// the Tauri template and never regenerated, so v0.28 through v0.32 shipped the
/// pre-identity orange tile in the taskbar while every other surface carried the
/// approved mark. Both are drawn by `lyrn assets` now; if this
/// fails, run it.
#[test]
fn the_application_icon_matches_the_brand_icon() {
    let root = repo_root();
    let ours = std::fs::read(root.join("src-tauri/icons/icon.ico"))
        .expect("the application icon is missing");
    let brand = std::fs::read(root.join("assets/icon.ico")).expect("the brand icon is missing");

    assert_eq!(
        ours, brand,
        "src-tauri/icons/icon.ico has fallen behind assets/icon.ico — \
         run `lyrn assets`"
    );
}

/// The icon's largest image comes first.
///
/// Windows itself does not care — it picks the entry closest to the size it
/// asked for. `tauri-codegen` does: it takes `entries()[0]` verbatim as the
/// window icon, so whatever is first is what the titlebar is scaled from. With
/// 16×16 first, the titlebar was stretched from sixteen pixels and looked
/// smeared, which is what the owner saw in v0.32.1.
#[test]
fn the_largest_icon_image_comes_first() {
    let ico = std::fs::read(repo_root().join("src-tauri/icons/icon.ico"))
        .expect("the application icon is missing");

    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    assert!(count > 1, "an icon with one size cannot serve every place");

    // Byte 0 of each 16-byte directory entry is the width; 0 encodes 256.
    let widths: Vec<u32> = (0..count)
        .map(|index| match ico[6 + 16 * index] {
            0 => 256,
            width => u32::from(width),
        })
        .collect();

    let mut descending = widths.clone();
    descending.sort_unstable_by(|a, b| b.cmp(a));

    assert_eq!(
        widths, descending,
        "icon.ico must list its images largest first — the window icon is \
         taken from the first entry. Run `lyrn assets`"
    );
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

/// Where the documentation site says it lives, read from the places that say it.
struct DocsAddress<'a> {
    /// `site` in `docs/astro.config.mjs`.
    site: Option<&'a str>,
    /// `base` there, when it has one.
    base: Option<&'a str>,
    /// The domain in `docs/public/CNAME`, when the file exists.
    cname: Option<&'a str>,
    /// Every absolute path the config writes by hand, such as an icon's `href`.
    paths: Vec<&'a str>,
    /// `homepage` of each manifest, with the manifest's name.
    homepages: Vec<(&'a str, Option<String>)>,
}

/// The value of `key: '...'` in a JavaScript object, the first one written.
fn js_string<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    source.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(key)?.strip_prefix(':')?.trim();
        let quote = rest.chars().next().filter(|c| matches!(c, '\'' | '"'))?;
        rest[1..].split(quote).next()
    })
}

/// Every `href: '/...'` of the config: a path from the domain's root, which
/// Astro does not put the base in front of.
fn rooted_paths(source: &str) -> Vec<&str> {
    source
        .split("href: '")
        .skip(1)
        .filter_map(|rest| rest.split('\'').next())
        .filter(|href| href.starts_with('/'))
        .collect()
}

/// What is wrong with the address, or nothing.
///
/// There are two ways to live on GitHub Pages and no third. With a domain of
/// its own the site is the domain and has no base; without one it is a project
/// site, `https://<owner>.github.io` with the repository as its base. A mix of
/// the two builds, deploys and answers 200 with every stylesheet a 404.
fn docs_address_problems(address: &DocsAddress, owner: &str, repo: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let Some(site) = address.site else {
        return vec!["docs/astro.config.mjs names no `site`".to_owned()];
    };
    let under_repo = format!("/{repo}/");

    let (root, prefix) = match address.cname {
        Some(domain) => {
            let expected = format!("https://{domain}");
            if site != expected {
                problems.push(format!(
                    "docs/public/CNAME says `{domain}`, so `site` is `{expected}`, not `{site}`"
                ));
            }
            if let Some(base) = address.base {
                problems.push(format!(
                    "a site on its own domain has no `base`, and the config has `{base}`"
                ));
            }
            (format!("{expected}/"), "/".to_owned())
        }
        None => {
            let expected = format!("https://{owner}.github.io");
            let base = format!("/{repo}");
            if site != expected {
                problems.push(format!(
                    "without docs/public/CNAME the site is `{expected}`, not `{site}`"
                ));
            }
            if address.base != Some(base.as_str()) {
                problems.push(format!(
                    "a project site needs `base: '{base}'`, and the config has {:?}",
                    address.base
                ));
            }
            (format!("{expected}{base}/"), under_repo.clone())
        }
    };

    for path in &address.paths {
        // On a domain of its own every path starts with `/`, so what gives a
        // left-behind one away is the repository's name in front of it.
        let left_behind = address.cname.is_some() && path.starts_with(&under_repo);
        if !path.starts_with(&prefix) || left_behind {
            problems.push(format!(
                "`{path}` in the config does not start where the site does, at `{prefix}`"
            ));
        }
    }

    for (manifest, homepage) in &address.homepages {
        if homepage.as_deref() != Some(root.as_str()) {
            problems.push(format!(
                "{manifest} has homepage {homepage:?}, and the site is `{root}`"
            ));
        }
    }
    problems
}

/// `homepage` of a JSON or TOML manifest: the first line that sets it.
fn homepage(source: &str) -> Option<String> {
    source.lines().find_map(|line| {
        let line = line.trim();
        let rest = line
            .strip_prefix("\"homepage\"")
            .or_else(|| line.strip_prefix("homepage"))?;
        let value = rest.trim_start_matches([':', '=', ' ']);
        Some(value.trim_matches(['"', ',']).to_owned())
    })
}

/// The documentation site agrees with itself about where it lives.
///
/// The site's address is written in four places - `site` and `base` of the
/// Astro config, the paths the config writes by hand, `docs/public/CNAME` and
/// the manifests' `homepage` - and a move to a domain of its own changes all of
/// them. A sibling project moved half of them and served a bare page for a
/// month, with every gate green.
#[test]
fn the_docs_site_agrees_with_itself_about_where_it_lives() {
    let config = read("docs/astro.config.mjs");
    let cname = std::fs::read_to_string(repo_root().join("docs/public/CNAME")).ok();
    let package = read("package.json");
    let manifest = read("src-tauri/Cargo.toml");

    let address = DocsAddress {
        site: js_string(&config, "site"),
        base: js_string(&config, "base"),
        cname: cname.as_deref().map(str::trim).filter(|c| !c.is_empty()),
        paths: rooted_paths(&config),
        homepages: vec![
            ("package.json", homepage(&package)),
            ("src-tauri/Cargo.toml", homepage(&manifest)),
        ],
    };

    let problems = docs_address_problems(&address, "lacodda", "kilna");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The gate above is only worth having if each half-made move fails it.
#[test]
fn a_half_moved_docs_site_is_refused() {
    fn home(url: &str) -> Vec<(&'static str, Option<String>)> {
        vec![("package.json", Some(url.to_owned()))]
    }
    fn project() -> DocsAddress<'static> {
        DocsAddress {
            site: Some("https://lacodda.github.io"),
            base: Some("/kilna"),
            cname: None,
            paths: vec!["/kilna/apple-touch-icon.png"],
            homepages: home("https://lacodda.github.io/kilna/"),
        }
    }
    fn domain() -> DocsAddress<'static> {
        DocsAddress {
            site: Some("https://kilna.example"),
            base: None,
            cname: Some("kilna.example"),
            paths: vec!["/apple-touch-icon.png"],
            homepages: home("https://kilna.example/"),
        }
    }
    fn refused(address: DocsAddress) -> bool {
        !docs_address_problems(&address, "lacodda", "kilna").is_empty()
    }

    assert!(!refused(project()), "a project site is one of the two ways");
    assert!(!refused(domain()), "a domain of its own is the other");

    // Each of these is one place moved and the rest left behind.
    let moved = [
        DocsAddress {
            cname: Some("kilna.example"),
            ..project()
        },
        DocsAddress {
            base: None,
            ..project()
        },
        DocsAddress {
            site: Some("https://kilna.example"),
            ..project()
        },
        DocsAddress {
            paths: vec!["/apple-touch-icon.png"],
            ..project()
        },
        DocsAddress {
            homepages: home("https://kilna.example/"),
            ..project()
        },
        DocsAddress {
            homepages: vec![("package.json", None)],
            ..project()
        },
        DocsAddress {
            site: None,
            ..project()
        },
        DocsAddress {
            base: Some("/kilna"),
            ..domain()
        },
        DocsAddress {
            cname: None,
            ..domain()
        },
        DocsAddress {
            paths: vec!["/kilna/apple-touch-icon.png"],
            ..domain()
        },
    ];
    for (index, address) in moved.into_iter().enumerate() {
        assert!(refused(address), "half-made move {index} passed");
    }
}
