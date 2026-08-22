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
    let source = std::fs::read_to_string(repo_root().join("src-tauri/src/commands.rs"))
        .expect("commands.rs is readable");

    // A key computed in the argument — `Record::new(if x { "a" } else { "b" })`
    // — is invisible to the scan below, and an unseen key reaches the screen
    // raw. Rather than parse Rust, the rule is that the argument must open with
    // a string literal; anything else fails here with instructions.
    let computed: Vec<&str> = source
        .match_indices("Record::new(")
        .filter(|(index, _)| {
            let argument = &source[index + "Record::new(".len()..];
            // `format!("{}.deleted", …)` is the one computed form this gate
            // knows about: the six trash entities are enumerated below, so its
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
