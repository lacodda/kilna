//! Holds the window's hand-written unions to the backend's enums.
//!
//! `src/lib/api.ts` mirrors several Rust enums as TypeScript string unions,
//! by hand. Nothing connects the two at compile time, and two had already come
//! apart:
//!
//! - the trash kinds: a comment deleted in v0.76 reached the trash as a kind
//!   the window's `DeletedEntity` did not have, so its row showed no word at
//!   all - while the union's own comment promised a new kind would be a type
//!   error until it had one. It could not be: the type error only fires on the
//!   side that was told;
//! - the verdict of a day: the window still spelled it `displaces` and `held`
//!   from before v0.44, the backend sent `taken`, and a day that merely had
//!   something on it was painted in the colour of a refusal.
//!
//! Each test reads the union out of `api.ts` and compares it with the enum.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use kilna_lib::release::Verdict;
use kilna_lib::trash::Entity;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri always has a parent")
        .to_path_buf()
}

/// The members of a union declared in `api.ts` as `export type Name =`
/// followed by one `| 'member'` per line, or on the same line as `'a' | 'b'`.
fn union_members(name: &str) -> BTreeSet<String> {
    let source =
        std::fs::read_to_string(repo_root().join("src/lib/api.ts")).expect("api.ts is readable");
    let head = format!("export type {name} =");
    let start = source
        .find(&head)
        .unwrap_or_else(|| panic!("api.ts declares {name}"))
        + head.len();
    let mut members = BTreeSet::new();
    let mut rest = source[start..].lines();
    // The members on the declaring line itself, then one per following line
    // while lines start with `|`.
    let mut line = rest.next().unwrap_or_default().to_owned();
    loop {
        for part in line.split('|') {
            let part = part.trim();
            if let Some(member) = part
                .strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
            {
                members.insert(member.to_owned());
            }
        }
        match rest.next() {
            Some(next) if next.trim_start().starts_with('|') => line = next.to_owned(),
            _ => break,
        }
    }
    members
}

fn wire_name<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .expect("serialises")
        .as_str()
        .expect("serialises to a string")
        .to_owned()
}

/// Every verdict, walked through an exhaustive match: a variant added to
/// `Verdict` does not compile here until it is given a place in the walk, so
/// the list cannot be forgotten the way a list written beside the enum can.
fn every_verdict() -> Vec<Verdict> {
    let after = |verdict: Option<Verdict>| match verdict {
        None => Some(Verdict::Empty),
        Some(Verdict::Empty) => Some(Verdict::Taken),
        Some(Verdict::Taken) => Some(Verdict::Pinned),
        Some(Verdict::Pinned) => None,
    };
    let mut all = Vec::new();
    let mut current = after(None);
    while let Some(verdict) = current {
        all.push(verdict);
        current = after(Some(verdict));
    }
    all
}

#[test]
fn the_window_knows_every_kind_the_trash_holds() {
    let sent: BTreeSet<String> = Entity::ALL.iter().map(wire_name).collect();
    let known = union_members("DeletedEntity");
    // The reader's own watchdog: a union written another way would read as
    // empty, and then the comparison would say only that the reader went blind.
    assert!(
        known.len() >= 10,
        "read only {} members of DeletedEntity: {known:?}",
        known.len()
    );
    assert_eq!(
        known, sent,
        "DeletedEntity in src/lib/api.ts and trash::Entity::ALL disagree"
    );
}

#[test]
fn every_kind_the_trash_holds_has_a_word() {
    for locale in ["en", "ru"] {
        let path = repo_root().join(format!("src/i18n/locales/{locale}.json"));
        let source = std::fs::read_to_string(&path).expect("the locale is readable");
        let json: serde_json::Value =
            serde_json::from_str(&source).expect("the locale is valid JSON");
        let words = json["trash"]["entity"]
            .as_object()
            .unwrap_or_else(|| panic!("{locale}.json has no trash.entity section"));
        for entity in Entity::ALL {
            let kind = wire_name(&entity);
            assert!(
                words
                    .get(&kind)
                    .and_then(|word| word.as_str())
                    .is_some_and(|word| !word.is_empty()),
                "{locale}.json has no trash.entity.{kind}: that kind shows in the trash with no word"
            );
        }
    }
}

#[test]
fn the_window_knows_every_verdict_on_a_day() {
    let sent: BTreeSet<String> = every_verdict().iter().map(wire_name).collect();
    let known = union_members("SlotVerdict");
    assert!(
        known.len() >= 3,
        "read only {} members of SlotVerdict: {known:?}",
        known.len()
    );
    assert_eq!(
        known, sent,
        "SlotVerdict in src/lib/api.ts and release::Verdict disagree"
    );
}
