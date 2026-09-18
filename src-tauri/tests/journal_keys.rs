//! Holds the journal's two halves to the same vocabulary.
//!
//! The backend writes an action key; the frontend looks that key up in the
//! locale to build the sentence. Nothing connects them at compile time, so a key
//! renamed on one side becomes a line of history that reads as `work.created`
//! rather than as a sentence — visible only to whoever happens to scroll the
//! feed afterwards, in a build already shipped.
//!
//! `tools/check-locales.mjs` cannot see this: it compares locales against each
//! other, and a key missing from *both* is consistent.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri always has a parent")
        .to_path_buf()
}

/// Every action key the command layer can write.
///
/// Read from the source rather than by running the commands: a command no test
/// happens to call would otherwise contribute no key, and that is exactly the
/// case most likely to be wrong.
///
/// Deletions do not appear here as literals — `discard_and_record` builds
/// `<entity>.deleted` from the entity itself, so they are added from the list of
/// entities instead. That is deliberate: a key assembled in one place cannot be
/// worded six different ways, and this scanner does not have to guess.
fn keys_written() -> BTreeSet<String> {
    // Every file that writes the journal. `commands.rs` is the window's side;
    // `mcp.rs` records what an agent outside it proposed; `assistant/apply.rs`
    // records what a person applied of it; `doors.rs` records what the
    // upgrade at open moved.
    let source = [
        "src-tauri/src/commands.rs",
        "src-tauri/src/mcp.rs",
        "src-tauri/src/assistant/apply.rs",
        "src-tauri/src/doors.rs",
    ]
    .iter()
    .map(|file| {
        std::fs::read_to_string(repo_root().join(file))
            .unwrap_or_else(|err| panic!("{file} is readable: {err}"))
    })
    .collect::<Vec<_>>()
    .join(
        "
",
    );

    // A key computed in the argument — `Record::new(if x { "a" } else { "b" })`
    // — is invisible to the scan below, and an unseen key reaches the screen
    // raw. Rather than parse Rust, the rule is that the argument must open with
    // a string literal; anything else fails here with instructions.
    let computed: Vec<&str> = source
        .match_indices("Record::new(")
        .filter(|(index, _)| {
            let argument = &source[index + "Record::new(".len()..];
            // `format!("{}.deleted", …)` is the one computed form this gate
            // knows about: the seven trash entities are enumerated below, so its
            // keys are covered.
            !argument.starts_with('"') && !argument.starts_with(r#"format!("{}.deleted""#)
        })
        .map(|(index, _)| {
            let line_start = source[..index].rfind('\n').map_or(0, |at| at + 1);
            let line_end = source[index..]
                .find('\n')
                .map_or(source.len(), |at| index + at);
            source[line_start..line_end].trim()
        })
        .collect();

    assert!(
        computed.is_empty(),
        "these journal keys are computed rather than written out, so this gate          cannot see them — name each key literally: {computed:?}"
    );

    let mut found = BTreeSet::new();
    for (index, _) in source.match_indices("Record::new(\"") {
        let start = index + "Record::new(\"".len();
        let Some(end) = source[start..].find('"') else {
            continue;
        };
        found.insert(source[start..start + end].to_owned());
    }

    for entity in [
        kilna_lib::trash::Entity::Work,
        kilna_lib::trash::Entity::Version,
        kilna_lib::trash::Entity::Score,
        kilna_lib::trash::Entity::Release,
        kilna_lib::trash::Entity::Note,
        kilna_lib::trash::Entity::Collection,
        kilna_lib::trash::Entity::Scene,
        kilna_lib::trash::Entity::Cut,
    ] {
        found.insert(format!("{}.deleted", entity.as_str()));
    }

    found
}

/// Keys under `journal.` in the source locale.
fn keys_translated() -> BTreeSet<String> {
    let source = std::fs::read_to_string(repo_root().join("src/i18n/locales/en.json"))
        .expect("en.json is readable");
    let locale: serde_json::Value = serde_json::from_str(&source).expect("en.json is valid JSON");

    locale["journal"]
        .as_object()
        .expect("the locale has a journal section")
        .keys()
        .map(|key| base_key(key).to_owned())
        .collect()
}

/// A sentence that counts things is written once per plural form —
/// `status.resynced_one`, `status.resynced_other` — but the backend records the
/// one key underneath them. Comparing the suffixed names against what the code
/// writes would report every counted sentence as both missing and stale, so
/// they are folded back to the key the code actually uses.
fn base_key(key: &str) -> &str {
    const FORMS: [&str; 6] = ["_zero", "_one", "_two", "_few", "_many", "_other"];
    FORMS
        .iter()
        .find_map(|form| key.strip_suffix(form))
        .unwrap_or(key)
}

#[test]
fn every_action_the_backend_writes_has_a_sentence() {
    let written = keys_written();
    assert!(
        !written.is_empty(),
        "no `Record::new` calls found — this test has stopped testing anything"
    );

    let translated = keys_translated();
    let missing: Vec<&String> = written.difference(&translated).collect();

    assert!(
        missing.is_empty(),
        "these actions are recorded but have no sentence in en.json: {missing:?}"
    );
}

#[test]
fn no_sentence_is_written_for_an_action_nobody_records() {
    // The journal section also holds the screen's own labels, which are not
    // action keys. Action keys are the dotted ones, by convention: `work.created`
    // names a thing that happened, `title` names a heading.
    let translated: BTreeSet<String> = keys_translated()
        .into_iter()
        .filter(|key| key.contains('.'))
        .collect();
    let written = keys_written();

    let stale: Vec<&String> = translated.difference(&written).collect();

    assert!(
        stale.is_empty(),
        "these sentences describe actions nothing records any more: {stale:?}"
    );
}

/// Every kind `undo` offers to take back, read out of the match arm that
/// decides it.
///
/// Scanned from the source for the reason `keys_written` gives: a kind no test
/// happens to undo would otherwise contribute nothing, and that is the one most
/// likely to be missing its sentence.
fn kinds_reversible() -> BTreeSet<String> {
    let source = std::fs::read_to_string(repo_root().join("src-tauri/src/undo.rs"))
        .expect("undo.rs is readable");
    let start = source
        .find("pub fn reversible(")
        .expect("undo.rs states which kinds are reversible");
    // The arm itself, not the function: `reversible` ends with `)\n}`, and
    // reading past it swallows the next function's prose as a "kind".
    let body = &source[start..];
    let end = body.find("\n    )").expect("the match arm ends");
    let arm = &body[..end];

    let mut found = BTreeSet::new();
    for (index, _) in arm.match_indices('"') {
        let after = &arm[index + 1..];
        let Some(close) = after.find('"') else {
            continue;
        };
        let kind = &after[..close];
        // The arm holds only dotted kind names; anything else is prose.
        if kind.contains('.') {
            found.insert(kind.to_owned());
        }
    }
    found
}

/// The undo toast says "Took back {{what}}", and `what` is looked up as
/// `undo.<kind>` — so a reversible kind with no sentence puts the raw key in
/// front of the person: *Took back undo.cut.create*.
///
/// `check-locales.mjs` cannot see this either: a key missing from both locales
/// is consistent, which is exactly how this shipped unnoticed once.
#[test]
fn every_kind_undo_offers_to_take_back_has_a_sentence() {
    let reversible = kinds_reversible();
    assert!(
        reversible.len() > 10,
        "the scan of `reversible` found almost nothing — it has stopped testing anything"
    );

    let source = std::fs::read_to_string(repo_root().join("src/i18n/locales/en.json"))
        .expect("en.json is readable");
    let locale: serde_json::Value = serde_json::from_str(&source).expect("en.json is valid JSON");
    let translated: BTreeSet<String> = locale["undo"]
        .as_object()
        .expect("the locale has an undo section")
        .keys()
        .map(|key| base_key(key).to_owned())
        .collect();

    let missing: Vec<&String> = reversible.difference(&translated).collect();
    assert!(
        missing.is_empty(),
        "undo offers to take these back but has no sentence for them in en.json: {missing:?}"
    );
}
