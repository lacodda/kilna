//! Holds the refusals the backend makes and the sentences the window says for
//! them to the same vocabulary (ADR 0041).
//!
//! A refusal travels as a code; the window looks `refusal.<code>` up in its
//! locale, and the backend reads the English sentence from the same file.
//! Nothing connects them at compile time, so a code with no sentence reaches a
//! person as the bare code - and, over MCP, reaches an agent that way too.
//!
//! Read from the source rather than by running the code, for the reason
//! `journal_keys.rs` gives: a refusal no test happens to trigger contributes
//! nothing, and that is exactly the refusal most likely to be missing words.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{Source, backend, repo_root};

/// Every leaf of a locale as a dotted key.
fn locale(name: &str) -> BTreeMap<String, String> {
    let path = repo_root().join(format!("src/i18n/locales/{name}.json"));
    let raw = std::fs::read_to_string(&path).expect("the locale is readable");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("the locale is JSON");
    let mut out = BTreeMap::new();
    flatten(&value, "", &mut out);
    out
}

fn flatten(value: &serde_json::Value, prefix: &str, out: &mut BTreeMap<String, String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(child, &path, out);
            }
        }
        serde_json::Value::String(text) => {
            out.insert(prefix.to_owned(), text.clone());
        }
        _ => {}
    }
}

/// The `{{name}}` placeholders of a sentence.
fn placeholders(sentence: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = sentence;
    while let Some(open) = rest.find("{{") {
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else {
            break;
        };
        found.insert(after[..close].trim().to_owned());
        rest = &after[close + 2..];
    }
    found
}

/// One place the backend refuses: the code, and the names of the values it
/// passes - read from the call to the end of its statement. Reading on past
/// the refusal's own chain can only find more names, never fewer, so it can
/// hide a missing value but never invent one.
struct Site {
    file: String,
    code: String,
    params: BTreeSet<String>,
}

fn sites(sources: &[Source]) -> Vec<Site> {
    let marker = "refused(";
    let mut found = Vec::new();
    let mut computed = Vec::new();
    for file in sources {
        for (at, _) in file.code.match_indices(marker) {
            let before = file.code[..at].chars().next_back();
            // `fn refused(` is the definition, and `Error::refused(` or
            // `refused(` are calls; a longer name ending in it is not.
            if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            if file.code[..at].trim_end().ends_with("fn") {
                continue;
            }
            let start = at + marker.len();
            let Some(literal) = file.text[start..].strip_prefix('"') else {
                computed.push(format!(
                    "{}: {}",
                    file.path,
                    &file.text[at..(at + 60).min(file.text.len())]
                ));
                continue;
            };
            let Some(end) = literal.find('"') else {
                continue;
            };
            let code = literal[..end].to_owned();
            let statement_end = file.code[start..]
                .find(';')
                .map_or(file.code.len(), |offset| start + offset);
            let window = &file.text[start..statement_end];
            let params = window
                .match_indices(".param(")
                .filter_map(|(offset, _)| {
                    let rest = window[offset + ".param(".len()..].trim_start();
                    let name = rest.strip_prefix('"')?;
                    name.find('"').map(|end| name[..end].to_owned())
                })
                .collect();
            found.push(Site {
                file: file.path.clone(),
                code,
                params,
            });
        }
    }
    assert!(
        computed.is_empty(),
        "a refusal's code must be a string literal, so this gate can see it: {computed:?}"
    );
    found
}

/// The keys the backend names as reasons outright: `Reason::of("skip.…")`.
fn reason_keys(sources: &[Source]) -> BTreeSet<String> {
    common::literal_arguments(sources, "Reason::of")
        .into_iter()
        .map(|(_, key)| key)
        .collect()
}

#[test]
fn every_refusal_has_a_sentence_that_says_what_it_is_given() {
    let sources = backend();
    let sites = sites(&sources);
    // The scan's watchdog: the backend refuses in well over a hundred places.
    assert!(
        sites.len() >= 100,
        "found only {} refusals - the scan has stopped seeing them",
        sites.len()
    );
    let english = locale("en");

    let mut problems = Vec::new();
    for site in &sites {
        let key = format!("refusal.{}", site.code);
        let Some(sentence) = english.get(&key) else {
            problems.push(format!(
                "{}: `{}` has no sentence in en.json",
                site.file, key
            ));
            continue;
        };
        for name in placeholders(sentence) {
            if !site.params.contains(&name) {
                problems.push(format!(
                    "{}: `{key}` says {{{{{name}}}}} and the refusal does not pass it",
                    site.file
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_reason_a_batch_gives_has_a_sentence() {
    let sources = backend();
    let keys = reason_keys(&sources);
    assert!(
        keys.len() >= 4,
        "found only {} reasons - the scan has stopped seeing them",
        keys.len()
    );
    let english = locale("en");
    let missing: Vec<&String> = keys
        .iter()
        .filter(|key| !english.contains_key(*key))
        .collect();
    assert!(
        missing.is_empty(),
        "these reasons have no sentence in en.json: {missing:?}"
    );
}

/// And the other way: a sentence nothing refuses with is a code renamed on
/// one side only, translated and kept in step for no one.
#[test]
fn no_sentence_is_written_for_a_refusal_nobody_makes() {
    let sources = backend();
    let used: BTreeSet<String> = sites(&sources)
        .into_iter()
        .map(|site| format!("refusal.{}", site.code))
        .chain(reason_keys(&sources))
        .collect();
    let stale: Vec<String> = locale("en")
        .into_keys()
        .filter(|key| key.starts_with("refusal.") || key.starts_with("skip."))
        .filter(|key| !used.contains(key))
        .collect();
    assert!(
        stale.is_empty(),
        "these sentences describe refusals nothing makes any more: {stale:?}"
    );
}
