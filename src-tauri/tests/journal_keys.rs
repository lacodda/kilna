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

use std::collections::{BTreeMap, BTreeSet};
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

/// Every `Record::new("key")` in the source, with the names it is given.
///
/// Two shapes have to be read, because the code uses both:
///
/// - the chained one, `Record::new("x").param("a", …).param("b", …)`, ending
///   at the statement's `;`;
/// - the built-up one, where the record is put in a variable and added to
///   over several statements — `let mut record = Record::new("x"); … record =
///   record.param("a", …);` — which is what every branching case does.
///
/// For the second, the scan continues to the end of the enclosing block
/// rather than to the first semicolon. That over-reads: a `.param` on some
/// *other* record later in the same function is counted as belonging to this
/// one. Deliberate, and in the safe direction — this gate exists to catch a
/// hole nothing fills, and over-reading can only ever hide a hole, never
/// invent one. A gate that cried wolf on thirty correct sentences would be
/// turned off within a week.
///
/// `.param(` is also written across two lines when the value is long, so the
/// name is looked for after the bracket rather than tight against it.
fn params_written() -> BTreeMap<String, BTreeSet<String>> {
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
    .join("\n");

    let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (index, _) in source.match_indices("Record::new(\"") {
        let start = index + "Record::new(\"".len();
        let Some(end) = source[start..].find('"') else {
            continue;
        };
        let key = source[start..start + end].to_owned();

        let rest = &source[start + end..];
        // A record handed straight to `record(…)` ends at the `;`. One put in
        // a variable is added to afterwards, so the window runs to the end of
        // the block — see the note above on why reading too much is the safe
        // direction here.
        // Is this record being put in a variable? Either the `let` is on this
        // line, or the `Record::new` is an arm of a `match` that a `let` a few
        // lines up is binding. Looking back a short way covers both without
        // parsing Rust.
        let before = &source[..index];
        let assigned = before
            .rsplit('\n')
            .take(12)
            .any(|line| line.contains("let ") && !line.trim_start().starts_with("//"));
        let window = if assigned {
            &rest[..rest.find("\n}").unwrap_or(rest.len())]
        } else {
            &rest[..rest.find(';').unwrap_or(rest.len())]
        };

        let names = found.entry(key).or_default();
        for (at, _) in window.match_indices(".param(") {
            let after = &window[at + ".param(".len()..];
            // The name is the first string literal after the bracket, on this
            // line or the next: a long value pushes it onto its own line.
            let Some(open) = after.find('"') else {
                continue;
            };
            // ...but only if nothing but whitespace stands between, or a
            // `.param(some_variable)` would swallow the next literal it finds.
            if !after[..open].trim().is_empty() {
                continue;
            }
            let from = open + 1;
            let Some(to) = after[from..].find('"') else {
                continue;
            };
            names.insert(after[from..from + to].to_owned());
        }
    }
    found
}

/// The `{{name}}` holes in a sentence, across all its plural forms.
fn placeholders_of(key: &str, locale: &serde_json::Value) -> BTreeSet<String> {
    let journal = locale["journal"]
        .as_object()
        .expect("the locale has a journal section");

    let mut holes = BTreeSet::new();
    for (name, value) in journal {
        if base_key(name) != key {
            continue;
        }
        let Some(text) = value.as_str() else { continue };
        let mut rest = text;
        while let Some(open) = rest.find("{{") {
            let after = &rest[open + 2..];
            let Some(close) = after.find("}}") else { break };
            holes.insert(after[..close].trim().to_owned());
            rest = &after[close + 2..];
        }
    }
    holes
}

#[test]
fn every_hole_in_a_sentence_is_filled_by_what_records_it() {
    // The defect this exists for: the owner read «{{title}}» удалено in his
    // own history. i18next leaves a hole it has no value for exactly as
    // written, so a sentence that interpolates `{{title}}` and a `Record`
    // that never passes `title` produce a line with braces in it — and the
    // rows are already written by the time anyone sees one.
    //
    // `count` is left out: it is i18next's own, the number that picks the
    // plural form, and it is passed as a param like any other where a
    // sentence counts something.
    let written = params_written();
    let source = std::fs::read_to_string(repo_root().join("src/i18n/locales/en.json"))
        .expect("en.json is readable");
    let locale: serde_json::Value = serde_json::from_str(&source).expect("en.json is valid JSON");

    let mut holes = Vec::new();
    for (key, passed) in &written {
        for hole in placeholders_of(key, &locale) {
            if !passed.contains(&hole) {
                holes.push(format!(
                    "journal.{key} says {{{{{hole}}}}}, which nothing passes"
                ));
            }
        }
    }

    assert!(
        holes.is_empty(),
        "these sentences have holes no `Record` fills, and print the braces: {holes:#?}"
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
