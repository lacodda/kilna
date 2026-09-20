//! Guards the handful of rules that make the window behave like an application
//! rather than a web page.
//!
//! None of them can be checked by rendering: they are single declarations whose
//! absence shows up only as a gesture doing something strange on someone's
//! laptop. All four were found that way, on the owner's own machine, after
//! hundreds of green tests said nothing. So they are checked as source text --
//! crude, but it fails when someone deletes the line, which is the failure that
//! actually happens.

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

/// The document itself must not scroll.
///
/// `height: 100%` without this leaves the page scrollable in both directions,
/// and a two-finger swipe found it: the shell slid sideways until the sidebar
/// left the window, and downwards until a blank strip sat above the topbar.
#[test]
fn the_document_is_sealed() {
    let css = read("src/styles.css");
    let root = css
        .split("html,")
        .nth(1)
        .expect("no `html, body, #root` block in styles.css");
    let block = root.split('}').next().expect("unterminated rule");

    assert!(
        block.contains("overflow: hidden"),
        "the html/body/#root rule no longer seals the document; a trackpad swipe will move the whole shell"
    );
}

/// The screen area itself does not scroll: it hands its height to a `<Screen>`.
///
/// The window has a bottom edge and the content must stop at it. When this box
/// scrolled, a long screen ran past the edge the way a web page does — no end
/// in sight, and a wide table's sideways bar parked under two hundred rows.
#[test]
fn the_screen_area_clips_and_hands_its_height_down() {
    let app = read("src/App.tsx");
    let area = app
        .split("key={screen}")
        .nth(1)
        .expect("the keyed content area is gone from App.tsx");
    let class_attr = area
        .split("className=")
        .nth(1)
        .expect("the content area has no className")
        .split('>')
        .next()
        .expect("unterminated element");

    assert!(
        class_attr.contains("overflow-hidden"),
        "the screen area scrolls again; the window must not, a `<Screen>` scrolls inside it"
    );
    assert!(
        class_attr.contains("min-h-0") && class_attr.contains("flex-1"),
        "the screen area stopped handing its height down; every screen will grow past the window"
    );
}

/// Every screen scrolls within the window rather than past it.
#[test]
fn a_flowing_screen_scrolls_itself_and_reserves_its_gutter() {
    let app = read("src/App.tsx");
    let screen = app
        .split("function Screen(")
        .nth(1)
        .expect("the Screen wrapper is gone from App.tsx");
    let body = screen.split("\n}").next().expect("unterminated Screen");

    assert!(
        body.contains("overflow-y-auto"),
        "a flowing screen no longer scrolls; its content will be cut off at the window's edge"
    );
    assert!(
        body.contains("overflow-x-hidden"),
        "a flowing screen lost `overflow-x-hidden`; `overflow-y-auto` alone leaves the sideways axis scrollable"
    );
    assert!(
        body.contains("scrollbar-gutter:stable"),
        "a flowing screen lost its stable gutter; every navigation will shift sideways as the scrollbar appears"
    );
    assert!(
        body.contains("overflow-hidden"),
        "a held screen no longer clips; it is the one that lays out against the window's height"
    );
}

/// Radii come from the scale, not from a number someone picked.
///
/// Four different roundings on one gesture — a row under the pointer — is what
/// the owner read as "strange corners": the rail at 10px, a nav at 9, a stage
/// stop at 5, a calendar chip at 7, none of them a step of the scale. The
/// exceptions below are smaller than the smallest step on purpose: a 2-line
/// badge inside a calendar tile, where `sm` would swallow the tile's own
/// corner.
#[test]
fn roundings_come_from_the_scale() {
    const ALLOWED: [&str; 2] = ["rounded-[4px]", "rounded-[3px]"];

    let mut stray: Vec<String> = Vec::new();
    for entry in walk(&repo_root().join("src")) {
        let text = std::fs::read_to_string(&entry).expect("a source file is readable");
        for (number, line) in text.lines().enumerate() {
            let Some(at) = line.find("rounded-[") else {
                continue;
            };
            let rest = &line[at..];
            let Some(end) = rest.find(']') else { continue };
            let token = &rest[..=end];
            if !token.ends_with("px]") || ALLOWED.contains(&token) {
                continue;
            }
            stray.push(format!(
                "{}:{}: {token}",
                entry.strip_prefix(repo_root()).unwrap_or(&entry).display(),
                number + 1
            ));
        }
    }

    assert!(
        stray.is_empty(),
        "these roundings are hand-picked pixels rather than steps of the scale \
         (xs 4 / sm 6 / md 9 / lg 12 / xl 16 - use `rounded-sm`, `rounded-md`, ...):\n  {}",
        stray.join("\n  ")
    );
}

/// Every `.tsx` under a directory.
fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(walk(&path));
        } else if path.extension().is_some_and(|e| e == "tsx") {
            found.push(path);
        }
    }
    found
}

/// Selection is off by default and handed back to text.
///
/// Both halves matter. Without the first the app reads as a web page caught
/// mid-copy; without the second nobody can copy a lyric out of the very
/// application built for writing them.
#[test]
fn selection_is_off_by_default_and_given_back_to_text() {
    let css = read("src/styles.css");

    assert!(
        css.contains("user-select: none"),
        "the shell no longer switches selection off; dragging across it will paint every label"
    );
    assert!(
        css.contains("user-select: text"),
        "nothing hands selection back; version bodies and notes would be uncopyable"
    );

    for (file, what) in [
        ("src/components/ui/Markdown.tsx", "rendered prose"),
        (
            "src/components/VersionPanel.tsx",
            "a version being read, written or compared",
        ),
    ] {
        assert!(
            read(file).contains("selectable"),
            "{what} is no longer marked selectable, so it cannot be copied"
        );
    }
}

/// Every block that shows somebody's own writing hands selection back.
///
/// The list above names the two that matter most; this one is the rule behind
/// it. `whitespace-pre-wrap` and `<pre>` are how this codebase renders text
/// exactly as it was typed -- a version body, a note, an error to paste into a
/// report -- so anywhere either appears is a place a reader will try to select.
/// Four such blocks were missed on the first pass of v0.43 and found by reading
/// rather than by any test, which is why the rule is enforced rather than the
/// list.
#[test]
fn nothing_shows_verbatim_text_without_making_it_selectable() {
    // Directories rather than a file list: a new screen must be covered by the
    // rule the day it is written, not the day someone remembers this test.
    let root = repo_root().join("src/components");
    let mut offenders = Vec::new();

    fn walk(dir: &Path, offenders: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("components directory is readable") {
            let path = entry.expect("readable entry").path();
            if path.is_dir() {
                walk(&path, offenders);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "tsx") {
                continue;
            }

            // The renderer defines the styling for prose; it carries the mark
            // itself and is not a place text is pasted in raw.
            if path.ends_with("Markdown.tsx") {
                continue;
            }

            // A lowercase file in `ui/` is a copy from dowel's registry and is
            // never edited here: it knows nothing of this application's
            // `selectable` class. The place that grants selection is the
            // product component that uses it, which the walk still reaches.
            let registry_copy = path.parent().is_some_and(|dir| dir.ends_with("ui"))
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| stem.chars().next().is_some_and(char::is_lowercase));
            if registry_copy {
                continue;
            }

            let source = std::fs::read_to_string(&path).expect("readable component");

            // A file, not a line, is the unit. Selection is inherited, and the
            // right place to grant it is often the scroll box around the text
            // rather than each line inside it -- a diff is selected across its
            // lines, not one at a time. So the question this asks is whether a
            // file that renders verbatim text says `selectable` anywhere at
            // all; a file that shows raw text and never mentions it is the
            // failure worth catching.
            let renders_verbatim = source
                .lines()
                .any(|line| line.contains("whitespace-pre-wrap") || line.contains("<pre"));

            if renders_verbatim && !source.contains("selectable") {
                offenders.push(
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?")
                        .to_owned(),
                );
            }
        }
    }

    walk(&root, &mut offenders);

    assert!(
        offenders.is_empty(),
        "these render text verbatim but do not mark it selectable, so it cannot be copied: {}",
        offenders.join(", ")
    );
}

/// Nothing adds `relative` to an overlay that is already positioned.
///
/// `cn` merges classes with `tailwind-merge`, which resolves a conflict by
/// keeping the last class of a group — and `position` is one group. A wrapper
/// adding `relative` so its own absolutely positioned child sits in the corner
/// therefore removes the `fixed` the overlay is built on, and the popup starts
/// measuring `top: 50%` against the document rather than the window.
///
/// That is exactly what happened to the dialog: the wrapper added `relative`,
/// every dialog moved below the fold on a tall screen, and three diagnoses went
/// past it because the class is right there in the stylesheet and missing only
/// after the merge. A `fixed` parent positions an absolute child perfectly
/// well, so the `relative` was never needed in the first place.
#[test]
fn no_wrapper_adds_relative_to_an_overlay() {
    let ui = repo_root().join("src/components/ui");
    let mut offenders = Vec::new();

    for entry in std::fs::read_dir(&ui).expect("the ui directory is readable") {
        let path = entry.expect("readable entry").path();
        if path.extension().is_none_or(|ext| ext != "tsx") {
            continue;
        }

        let source = std::fs::read_to_string(&path).expect("readable component");
        for (number, line) in source.lines().enumerate() {
            // Only where a class list is being handed to one of dowel's popups:
            // `relative` on an element of one's own is ordinary and fine.
            let hands_classes_to_a_popup = line.contains("Popup className=")
                || line.contains("Popup\n")
                || (line.contains("<Dialog") && line.contains("className"));
            if hands_classes_to_a_popup && line.contains("'relative'") {
                offenders.push(format!(
                    "{}:{}",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                    number + 1
                ));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "these add `relative` to an overlay, which drops its `fixed` when the classes merge: {}",
        offenders.join(", ")
    );
}
