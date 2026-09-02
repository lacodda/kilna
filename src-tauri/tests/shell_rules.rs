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

/// The one box that scrolls must scroll on one axis.
///
/// `overflow-y-auto` on its own leaves the horizontal axis at `auto` as well --
/// the CSS spec forces it -- so the content area was silently scrollable
/// sideways and slid under the sidebar. The tab strip hit the same trap at
/// v0.35; this is the second time, hence a test.
#[test]
fn the_content_scroller_is_clipped_sideways_and_reserves_its_gutter() {
    let app = read("src/App.tsx");
    let scroller = app
        .split("key={screen}")
        .nth(1)
        .expect("the keyed content scroller is gone from App.tsx");
    let class_attr = scroller
        .split("className=")
        .nth(1)
        .expect("the scroller has no className")
        .split('>')
        .next()
        .expect("unterminated element");

    assert!(
        class_attr.contains("overflow-y-auto"),
        "the content area no longer scrolls vertically"
    );
    assert!(
        class_attr.contains("overflow-x-hidden"),
        "the content scroller lost `overflow-x-hidden`; `overflow-y-auto` alone leaves the horizontal axis scrollable"
    );
    assert!(
        class_attr.contains("scrollbar-gutter:stable"),
        "the content scroller lost its stable gutter; every navigation will shift sideways as the scrollbar appears"
    );
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
            "src/components/versions/VersionDiff.tsx",
            "the version diff",
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
