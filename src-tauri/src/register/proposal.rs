//! Words proposed for the record, applied with one click (ADR 0052).
//!
//! One package for everything an agent, an action or kilna itself proposes
//! about words: a word kept as a phrase and the shelf it goes on, how a word
//! is sung, a term for the register, and the works a meaning is in. One word
//! is one item: "пульсар" proposed as a phrase to keep, sung "пульсАр", is
//! taken in one click - the phrase among the phrases (ADR 0064), the singing
//! in the record (ADR 0052).
//!
//! The package is read and checked against the workspace as it stands, and
//! kept whole or item by item (`word:N`), the way a package for the canon is
//! (ADR 0018).
//!
//! Since v0.116 an item can change a word the record already keeps, take one
//! out of it, or split one into several (FR-08 8.3). Until then a proposal
//! about a known word added to it and nothing more: a strictness proposed
//! for a word that had one, a kind, a topic, a note were dropped without a
//! word, and the answer said "proposed". What a proposal changes is now read
//! against the record when it is made and said field by field - was, becomes
//! - and a field that could not be written when it is kept is said too.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Strictness, Sung, TermKind, TermPatch};
use crate::error::{Error, Reason, Result};

/// What a proposal does with a word.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum WordAction {
    /// Keep it: a new word, or what is proposed written onto the one the
    /// record keeps.
    #[default]
    Upsert,
    /// Take it out of the record - into the trash.
    Delete,
    /// One word of the record that is several becomes them: "a, b" becomes
    /// "a" and "b", each with what the one held.
    Split,
}

impl WordAction {
    // The signature serde asks of `skip_serializing_if`.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn is_upsert(&self) -> bool {
        *self == Self::Upsert
    }
}

/// One thing keeping a proposal changes in a word the record already keeps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct WordChange {
    /// `strictness`, `forms`, `kind`, `topic`, `note`, `sung`, `works`.
    pub field: String,
    pub was: String,
    pub becomes: String,
}

/// One word as it is proposed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct ProposedWord {
    /// What is done with it; kept, when nothing says otherwise.
    #[serde(default, skip_serializing_if = "WordAction::is_upsert")]
    pub action: WordAction,
    /// The word as it is written: the record's key, matched whatever its case.
    pub word: String,
    /// Other forms counted as the same word.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forms: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub kind: Option<TermKind>,
    /// Kept as a phrase, fresh, for works to come.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub phrase: bool,
    /// The shelf the phrase goes on, by name; made when there is none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub shelf: Option<String>,
    /// How its forms are sung where that is not how they are written.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sung: Vec<Sung>,
    /// Spent: in the register, this strictly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub strictness: Option<Strictness>,
    /// Not spent any more: the word stays in the record for how it is sung.
    /// Written by an agent as `"strictness": null`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unspend: bool,
    /// The forms given take the place of the ones the word has, rather than
    /// joining them.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub replace_forms: bool,
    /// What the word is about, as the register groups by.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub topic: Option<String>,
    /// For a split: the words it becomes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub into: Vec<String>,
    /// The record keeps this word already: the item changes it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub known: bool,
    /// What keeping the item changes in that word, read when it was
    /// proposed: was, becomes. Shown, never read back.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<WordChange>,
    /// The works a meaning is in, by id, as the person reads them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub works: Vec<NamedWork>,
    /// Why: what the proposer saw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub note: Option<String>,
    /// The works it was found written this way in, by title - what kilna
    /// says when it proposes from the owner's own texts. Shown, never kept:
    /// the works a meaning is in are `works`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seen_in: Vec<String>,
}

/// A work a proposed word names, with the title it was read under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct NamedWork {
    pub id: String,
    pub title: String,
}

/// What was proposed, and what of it was left out on reading.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct WordsPackage {
    pub words: Vec<ProposedWord>,
    /// Why an item was not taken into the package: a word with no letters, a
    /// work nobody can find.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dropped: Vec<Reason>,
    /// What a person should know before keeping it: a word that reads as
    /// several.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<Reason>,
}

impl WordsPackage {
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

/// How an answer gives words for the record: appended to an action that
/// produces `words`, as the canon's instruction is to one that produces
/// `canon`.
pub const INSTRUCTION: &str = "\n\nAnswer with your reasoning, then one fenced json block the application reads:\n\n```json\n{\"words\": [{\"word\": \"a lighthouse nobody keeps\", \"kind\": \"image\", \"strictness\": \"limit\", \"works\": [\"<exact title>\", \"<exact title>\"], \"note\": \"<the line here> / <the line there>\"}]}\n```\n\n`word` is the meaning in a few plain words - when the register already names it, its words exactly as the register writes them, so the works are named for that entry. `kind` is image, scene or pattern for a meaning; noun, adjective, verb or phrase for a word. `works` are the titles of every work it is in - this one and the ones it shares it with - exactly as you were given them. Leave the block's list empty when there is nothing to propose.";

/// The item key of the `index`-th word, as an apply names it.
pub fn item(index: usize) -> String {
    format!("word:{index}")
}

fn taken(items: Option<&[String]>, index: usize) -> bool {
    items.is_none_or(|items| items.iter().any(|one| *one == item(index)))
}

/// Read a package from what a proposer wrote: `{"words": [...]}`, or the list
/// itself. A work is named by id or by exact title; one that is not found is
/// left out and said so, the rest of the word kept.
pub fn read(conn: &Connection, profile_id: &str, value: &Value) -> Result<WordsPackage> {
    let list = match value {
        Value::Array(list) => list.clone(),
        Value::Object(object) => match object.get("words") {
            Some(Value::Array(list)) => list.clone(),
            _ => return Err(Error::refused("words.noWords")),
        },
        _ => return Err(Error::refused("words.noWords")),
    };
    let mut package = WordsPackage::default();
    for raw in list {
        let raw = match raw {
            Value::String(word) => serde_json::json!({ "word": word, "phrase": true }),
            other => other,
        };
        let mut works: Vec<String> = Vec::new();
        if let Some(Value::Array(named)) = raw.get("works") {
            for one in named {
                if let Some(text) = one.as_str() {
                    works.push(text.to_owned());
                } else if let Some(id) = one.get("id").and_then(Value::as_str) {
                    works.push(id.to_owned());
                }
            }
        }
        // `"strictness": null` is said on purpose: the word is not spent any
        // more. An absent one says nothing.
        let unspend = raw.get("strictness").is_some_and(Value::is_null);
        let mut stripped = raw.clone();
        if let Some(object) = stripped.as_object_mut() {
            object.remove("works");
            // What kilna reads off the record, not what a proposer says.
            object.remove("known");
            object.remove("changes");
        }
        let mut word: ProposedWord = match serde_json::from_value(stripped) {
            Ok(word) => word,
            Err(why) => {
                package
                    .dropped
                    .push(Reason::of("refusal.words.unread").param("why", why.to_string()));
                continue;
            }
        };
        word.word = word.word.trim().to_owned();
        if crate::words::words(&word.word).is_empty() {
            package
                .dropped
                .push(Reason::of("refusal.words.noLetters").param("word", word.word.clone()));
            continue;
        }
        for named in works {
            match find_work(conn, profile_id, &named)? {
                Some(found) => {
                    if !word.works.iter().any(|known| known.id == found.id) {
                        word.works.push(NamedWork {
                            id: found.id,
                            title: found.title,
                        });
                    }
                }
                None => package.dropped.push(
                    Reason::of("refusal.words.noSuchWork")
                        .param("word", word.word.clone())
                        .param("work", named),
                ),
            }
        }
        word.sung = super::clean_sung(word.sung);
        word.unspend = unspend && word.strictness.is_none();
        word.into = word
            .into
            .iter()
            .map(|part| part.trim().to_owned())
            .filter(|part| !crate::words::words(part).is_empty())
            .collect();
        let found = super::find_word(conn, profile_id, &word.word, None)?;
        word.known = found.is_some();
        match (word.action, &found) {
            (WordAction::Delete | WordAction::Split, None) => {
                package
                    .dropped
                    .push(Reason::of("refusal.words.notInRecord").param("word", word.word.clone()));
                continue;
            }
            (WordAction::Split, Some(_)) if word.into.is_empty() => {
                package.dropped.push(
                    Reason::of("refusal.words.splitIntoNothing").param("word", word.word.clone()),
                );
                continue;
            }
            (WordAction::Upsert, Some(found)) => {
                word.changes = plan(found, &word).changes(found, &word);
            }
            (WordAction::Upsert, None) => {
                // One word of the record is one thing a text can say. Two
                // written as one are counted as neither.
                if word.word.contains([',', '/']) && !word.phrase {
                    package.warnings.push(
                        Reason::of("refusal.words.separator").param("word", word.word.clone()),
                    );
                }
            }
            _ => {}
        }
        package.words.push(word);
    }
    Ok(package)
}

/// What a proposed word writes onto the word the record keeps: each field
/// that would change, with what it changes to.
#[derive(Debug, Default)]
struct Plan {
    strictness: Option<Option<Strictness>>,
    forms: Option<Vec<String>>,
    kind: Option<TermKind>,
    topic: Option<Option<String>>,
    note: Option<Option<String>>,
    sung: Option<Vec<Sung>>,
}

fn plan(found: &super::Term, word: &ProposedWord) -> Plan {
    let mut plan = Plan::default();
    if word.unspend {
        if found.strictness.is_some() {
            plan.strictness = Some(None);
        }
    } else if let Some(strictness) = word.strictness
        && found.strictness != Some(strictness)
    {
        plan.strictness = Some(Some(strictness));
    }
    if word.replace_forms || !word.forms.is_empty() {
        let mut forms = if word.replace_forms {
            Vec::new()
        } else {
            found.forms.clone()
        };
        forms.extend(word.forms.iter().cloned());
        let forms = super::clean_forms(&found.word, forms);
        if forms != found.forms {
            plan.forms = Some(forms);
        }
    }
    if let Some(kind) = word.kind
        && kind != found.kind
    {
        plan.kind = Some(kind);
    }
    let said = |text: &Option<String>| {
        text.as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    };
    if let Some(topic) = said(&word.topic)
        && found.topic.as_deref() != Some(topic.as_str())
    {
        plan.topic = Some(Some(topic));
    }
    if let Some(note) = said(&word.note)
        && found.note.as_deref() != Some(note.as_str())
    {
        plan.note = Some(Some(note));
    }
    if !word.sung.is_empty() {
        let mut sung = found.sung.clone();
        sung.extend(word.sung.iter().cloned());
        let sung = super::clean_sung(sung);
        if sung != found.sung {
            plan.sung = Some(sung);
        }
    }
    plan
}

impl Plan {
    /// The plan as a person reads it: was, becomes.
    fn changes(&self, found: &super::Term, word: &ProposedWord) -> Vec<WordChange> {
        let none = || "-".to_owned();
        let mut out = Vec::new();
        let mut push = |field: &str, was: String, becomes: String| {
            out.push(WordChange {
                field: field.to_owned(),
                was,
                becomes,
            });
        };
        if let Some(strictness) = self.strictness {
            push(
                "strictness",
                found
                    .strictness
                    .map_or_else(none, |s| s.as_str().to_owned()),
                strictness.map_or_else(none, |s| s.as_str().to_owned()),
            );
        }
        if let Some(forms) = &self.forms {
            let list = |forms: &[String]| {
                if forms.is_empty() {
                    none()
                } else {
                    forms.join(", ")
                }
            };
            push("forms", list(&found.forms), list(forms));
        }
        if let Some(kind) = self.kind {
            push(
                "kind",
                found.kind.as_str().to_owned(),
                kind.as_str().to_owned(),
            );
        }
        if let Some(Some(topic)) = &self.topic {
            push(
                "topic",
                found.topic.clone().unwrap_or_else(none),
                topic.clone(),
            );
        }
        if let Some(Some(note)) = &self.note {
            push(
                "note",
                found.note.clone().unwrap_or_else(none),
                note.clone(),
            );
        }
        if let Some(sung) = &self.sung {
            let list = |sung: &[Sung]| {
                if sung.is_empty() {
                    none()
                } else {
                    sung.iter()
                        .map(|one| format!("{} → {}", one.written, one.sung))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            };
            push("sung", list(&found.sung), list(sung));
        }
        if !word.works.is_empty() {
            push(
                "works",
                none(),
                word.works
                    .iter()
                    .map(|work| work.title.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        out
    }

    /// The plan as the patches it is written by, one field each, with the
    /// field's name: a field the register refuses then costs only itself.
    fn patches(self) -> Vec<(&'static str, TermPatch)> {
        let mut out = Vec::new();
        if let Some(strictness) = self.strictness {
            out.push((
                "strictness",
                TermPatch {
                    strictness: Some(strictness),
                    ..TermPatch::default()
                },
            ));
        }
        if let Some(forms) = self.forms {
            out.push((
                "forms",
                TermPatch {
                    forms: Some(forms),
                    ..TermPatch::default()
                },
            ));
        }
        if let Some(kind) = self.kind {
            out.push((
                "kind",
                TermPatch {
                    kind: Some(kind),
                    ..TermPatch::default()
                },
            ));
        }
        if let Some(topic) = self.topic {
            out.push((
                "topic",
                TermPatch {
                    topic: Some(topic),
                    ..TermPatch::default()
                },
            ));
        }
        if let Some(note) = self.note {
            out.push((
                "note",
                TermPatch {
                    note: Some(note),
                    ..TermPatch::default()
                },
            ));
        }
        if let Some(sung) = self.sung {
            out.push((
                "sung",
                TermPatch {
                    sung: Some(sung),
                    ..TermPatch::default()
                },
            ));
        }
        out
    }
}

fn find_work(
    conn: &Connection,
    profile_id: &str,
    named: &str,
) -> Result<Option<crate::work::Work>> {
    if let Some(found) = crate::work::get(conn, named)?.filter(|w| w.profile_id == profile_id) {
        return Ok(Some(found));
    }
    let id: Option<String> = rusqlite::OptionalExtension::optional(conn.query_row(
        "SELECT id FROM work WHERE profile_id = ?1 AND title = ?2 ORDER BY created_at, rowid LIMIT 1",
        rusqlite::params![profile_id, named.trim()],
        |row| row.get(0),
    ))?;
    match id {
        Some(id) => crate::work::get(conn, &id),
        None => Ok(None),
    }
}

/// What stands in the way of keeping the items taken: a work that has gone
/// since the package was read. Said per word, so the rest can still be kept.
pub fn check(
    conn: &Connection,
    package: &WordsPackage,
    items: Option<&[String]>,
) -> Result<Vec<Reason>> {
    let mut problems = Vec::new();
    for (index, word) in package.words.iter().enumerate() {
        if !taken(items, index) {
            continue;
        }
        if word.action != WordAction::Upsert {
            let profile_id = crate::actions::active_profile_id(conn)?;
            if super::find_word(conn, &profile_id, &word.word, None)?.is_none() {
                problems
                    .push(Reason::of("refusal.words.notInRecord").param("word", word.word.clone()));
            }
        }
        for named in &word.works {
            if crate::work::get(conn, &named.id)?.is_none() {
                problems.push(
                    Reason::of("refusal.words.workGone")
                        .param("word", word.word.clone())
                        .param("title", named.title.clone()),
                );
            }
        }
    }
    Ok(problems)
}

/// What keeping a package wrote.
#[derive(Debug, Default)]
pub struct Kept {
    /// The terms written, by id.
    pub terms: Vec<String>,
    /// The phrases kept, by id.
    pub phrases: Vec<String>,
    /// What could not be written, each with the word, the field and why:
    /// said, so that "kept" never covers a field that was not.
    pub unapplied: Vec<Reason>,
}

/// Keep the items taken, as a hand would: the phrase kept and put on its
/// shelf - found or made - and, where the item says something the record
/// keeps, the word found or made and what is proposed written onto it, the
/// works it names named; a word taken out sent to the trash; a word split
/// made into its parts. Each a gesture of its own; the caller holds the unit.
pub fn keep(conn: &Connection, package: WordsPackage, items: Option<&[String]>) -> Result<Kept> {
    let profile_id = crate::actions::active_profile_id(conn)?;
    let mut kept = Kept::default();
    for (index, word) in package.words.into_iter().enumerate() {
        if !taken(items, index) {
            continue;
        }
        match word.action {
            WordAction::Upsert => {}
            WordAction::Delete => {
                if let Some(found) = super::find_word(conn, &profile_id, &word.word, None)? {
                    crate::actions::trash::discard(conn, crate::trash::Entity::Term, &found.id)?;
                    kept.terms.push(found.id);
                }
                continue;
            }
            WordAction::Split => {
                if let Some(found) = super::find_word(conn, &profile_id, &word.word, None)? {
                    kept.terms
                        .extend(split(conn, &profile_id, &found, &word.into)?);
                }
                continue;
            }
        }
        if word.phrase {
            let config = crate::profile::config_for(conn, &profile_id)?;
            let kind = config
                .line_kinds()
                .into_iter()
                .next()
                .map(str::to_owned)
                .ok_or_else(|| crate::error::Error::refused("words.noLineKind"))?;
            let shelf = match word.shelf.as_deref().filter(|name| !name.trim().is_empty()) {
                Some(name) => Some(match crate::shelf::find(conn, &profile_id, name)? {
                    Some(found) => found,
                    None => crate::actions::shelf::create(conn, name)?,
                }),
                None => None,
            };
            let lines = crate::actions::note::keep_lines(
                conn,
                std::slice::from_ref(&word.word),
                &kind,
                shelf.as_ref().map(|shelf| shelf.id.as_str()),
                &[],
            )?;
            kept.phrases
                .extend(lines.notes.into_iter().map(|note| note.id));
        }
        // A word kept only as a phrase says nothing the record holds: no term.
        let says = word.strictness.is_some()
            || word.unspend
            || !word.sung.is_empty()
            || !word.forms.is_empty()
            || word.replace_forms
            || !word.works.is_empty()
            || (!word.phrase
                && (word.kind.is_some() || word.topic.is_some() || word.note.is_some()));
        if !says {
            continue;
        }
        let term = match super::find_word(conn, &profile_id, &word.word, None)? {
            Some(found) => {
                // Field by field, each a write of its own: one the register
                // refuses - a kind the word's works do not allow - is said
                // and costs none of the others.
                for (field, patch) in plan(&found, &word).patches() {
                    let written = crate::db::unit::atomically(conn, |conn| {
                        crate::actions::register::update(conn, &found.id, patch)
                    });
                    if let Err(cause) = written {
                        kept.unapplied.push(
                            Reason::of("refusal.words.fieldNotWritten")
                                .param("word", found.word.clone())
                                .param("field", field)
                                .param("why", cause.reason().english()),
                        );
                    }
                }
                found
            }
            None => crate::actions::register::create(
                conn,
                super::NewTerm {
                    word: word.word.clone(),
                    forms: word.forms.clone(),
                    kind: word.kind,
                    strictness: word.strictness,
                    sung: word.sung.clone(),
                    topic: word.topic.clone(),
                    note: word.note.clone(),
                    ..super::NewTerm::default()
                },
            )?,
        };
        for named in &word.works {
            crate::actions::register::link(conn, &term.id, &named.id)?;
        }
        kept.terms.push(term.id);
    }
    Ok(kept)
}

/// Make one word of the record into several: each part found or made with
/// what the one held - its kind, its strictness, its topic and note, the
/// works named for it - the ways of singing going to the part they are a
/// way of singing, and the one itself to the trash. The ids of the parts.
fn split(
    conn: &Connection,
    profile_id: &str,
    found: &super::Term,
    into: &[String],
) -> Result<Vec<String>> {
    let named: Vec<String> = super::uses(conn, &found.id)?
        .into_iter()
        .filter(|used| used.named)
        .map(|used| used.work_id)
        .collect();
    let mut parts = Vec::new();
    for part in into {
        let plain = crate::words::plain(part);
        // A way of singing belongs to the part it spells.
        let sung: Vec<Sung> = found
            .sung
            .iter()
            .filter(|one| {
                let written = crate::words::plain(&one.written);
                let stem: String = plain.chars().take(4).collect();
                !stem.is_empty() && written.starts_with(&stem)
            })
            .cloned()
            .collect();
        let term = match super::find_word(conn, profile_id, part, Some(&found.id))? {
            Some(known) => known,
            None => crate::actions::register::create(
                conn,
                super::NewTerm {
                    word: part.clone(),
                    forms: Vec::new(),
                    kind: Some(found.kind),
                    strictness: found.strictness,
                    sung,
                    topic: found.topic.clone(),
                    note: found.note.clone(),
                    // A part of a split term reads where the whole did.
                    scope: found.scope,
                    max_per_month: found.max_per_month,
                    cooldown_days: found.cooldown_days,
                },
            )?,
        };
        for work_id in &named {
            crate::actions::register::link(conn, &term.id, work_id)?;
        }
        parts.push(term.id);
    }
    crate::actions::trash::discard(conn, crate::trash::Entity::Term, &found.id)?;
    Ok(parts)
}

/// The package as a person reads it before keeping it: a line per word with
/// what would change.
pub fn render(package: &WordsPackage) -> String {
    let mut out = String::new();
    for word in &package.words {
        let mut line = format!("- **{}**", word.word);
        let mut said: Vec<String> = Vec::new();
        match word.action {
            WordAction::Upsert => {}
            WordAction::Delete => said.push("take out of the record".to_owned()),
            WordAction::Split => said.push(format!("split into {}", word.into.join(", "))),
        }
        // A word the record keeps: what changes, was and becomes. The rest
        // of the line would say again what the changes say.
        if word.known && word.action == WordAction::Upsert {
            for change in &word.changes {
                said.push(format!(
                    "{}: {} → {}",
                    change.field, change.was, change.becomes
                ));
            }
            if word.changes.is_empty() {
                said.push("nothing to change: the record already says this".to_owned());
            }
            line.push_str(&format!(" — {}", said.join("; ")));
            out.push_str(&line);
            out.push('\n');
            continue;
        }
        if word.phrase {
            said.push(match &word.shelf {
                Some(shelf) => format!("phrase · {shelf}"),
                None => "phrase".to_owned(),
            });
        }
        if let Some(strictness) = word.strictness {
            said.push(format!("register · {}", strictness.as_str()));
        }
        for one in &word.sung {
            said.push(format!("{} → {}", one.written, one.sung));
        }
        if !word.works.is_empty() {
            said.push(format!(
                "in {}",
                word.works
                    .iter()
                    .map(|w| format!("“{}”", w.title))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !said.is_empty() {
            line.push_str(&format!(" — {}", said.join("; ")));
        }
        if let Some(note) = &word.note {
            line.push_str(&format!("\n  {}", note.replace('\n', "\n  ")));
        }
        out.push_str(&line);
        out.push('\n');
    }
    if !package.warnings.is_empty() {
        let said: Vec<String> = package
            .warnings
            .iter()
            .map(|reason| {
                let code = reason.key.strip_prefix("refusal.").unwrap_or(&reason.key);
                crate::error::english(code, &reason.params)
            })
            .collect();
        out.push_str(&format!("\nMind: {}\n", said.join(" ")));
    }
    if !package.dropped.is_empty() {
        let said: Vec<String> = package
            .dropped
            .iter()
            .map(|reason| {
                let code = reason.key.strip_prefix("refusal.").unwrap_or(&reason.key);
                crate::error::english(code, &reason.params)
            })
            .collect();
        out.push_str(&format!("\nLeft out: {}.\n", said.join("; ")));
    }
    out.trim_end().to_owned()
}

/// The latest text of every sung role of every work of a profile, with the
/// work it is of: what the owner has written for a singer.
fn sung_texts(conn: &Connection, profile_id: &str) -> Result<Vec<(String, String)>> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let mut statement = conn.prepare(
        "SELECT w.title, w.kind, v.role, v.body FROM work w
           JOIN work_version v ON v.work_id = w.id
          WHERE w.profile_id = ?1
            AND v.id = (SELECT v2.id FROM work_version v2
                         WHERE v2.work_id = w.id AND v2.role = v.role
                         ORDER BY v2.created_at DESC, v2.rowid DESC LIMIT 1)
          ORDER BY w.created_at, w.rowid",
    )?;
    let rows = statement
        .query_map(rusqlite::params![profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows
        .into_iter()
        .filter(|(_, kind, role, _)| {
            config
                .vocabulary(kind)
                .version_roles
                .iter()
                .any(|known| known.key == *role && known.sung)
        })
        .map(|(title, _, _, body)| (title, body))
        .collect())
}

/// The written form a respelling stands for: "марсэль" is "марсель" sung
/// with a hard э, the way the owner keeps a singer off a soft е. None when
/// no э of the word stands for an е of a word the language pack knows.
fn written_for(plain_form: &str) -> Option<String> {
    let letters: Vec<char> = plain_form.chars().collect();
    for (index, letter) in letters.iter().enumerate() {
        if *letter != 'э' {
            continue;
        }
        let mut tried = letters.clone();
        tried[index] = 'е';
        let tried: String = tried.into_iter().collect();
        if crate::words::pack::reading(&tried).is_some() {
            return Some(tried);
        }
    }
    None
}

/// What the owner's sung texts already say about singing, proposed for the
/// record (v0.90): every word they marked with a capital vowel that the
/// language pack cannot stress on its own - a word it does not know, or a
/// stress it does not give - and every respelling of a word it knows
/// ("МарсЭль" for "Марсель"). Not a homograph's mark: which reading a song
/// means is the song's, and a word kept as always sung one way would argue
/// with the next song that means the other.
///
/// A proposal, never a write: the owner keeps what is right and leaves the
/// rest. A word whose way of singing the record already keeps is not
/// proposed again.
pub fn from_texts(conn: &Connection, profile_id: &str) -> Result<WordsPackage> {
    use crate::words::{pack, plain, sung};
    let kept: std::collections::BTreeSet<String> = super::sung_forms(conn, profile_id)?
        .iter()
        .map(|one| plain(&one.written))
        .collect();
    // By the written form: the way it is sung, and the works it was seen in.
    let mut found: std::collections::BTreeMap<String, (String, String, Vec<String>)> =
        std::collections::BTreeMap::new();
    for (title, body) in sung_texts(conn, profile_id)? {
        for word in crate::words::words(&body) {
            if word.bracketed || !pack::is_cyrillic(word.text) {
                continue;
            }
            let Some(mark) = sung::marked(word.text) else {
                continue;
            };
            let lower = word.text.to_lowercase();
            if lower.contains('ё') && !word.text.chars().skip(1).any(char::is_uppercase) {
                // A ё is a spelling, not a mark the owner made.
                continue;
            }
            let key = plain(word.text);
            // A respelling first: "МарсЭль" is "Марсель" sung hard, whether
            // or not some dictionary also lists the э spelling.
            let (written, sung_form) = match (written_for(&key), pack::reading(word.text)) {
                (Some(written), _) => (written, sung::with_stress(&key, mark)),
                (None, Some(reading)) if reading.is_homograph() => continue,
                (None, Some(reading))
                    if reading.stresses.contains(&mark) || reading.stresses.is_empty() =>
                {
                    continue;
                }
                (None, _) => (key.clone(), sung::with_stress(&key, mark)),
            };
            if kept.contains(&plain(&written))
                || plain(&sung_form) == plain(&written) && sung_form == written
            {
                continue;
            }
            let entry = found
                .entry(plain(&written))
                .or_insert_with(|| (written.clone(), sung_form.clone(), Vec::new()));
            if !entry.2.contains(&title) {
                entry.2.push(title.clone());
            }
        }
    }
    let words = found
        .into_values()
        .map(|(written, sung_form, titles)| ProposedWord {
            word: written.clone(),
            sung: vec![Sung {
                written,
                sung: sung_form,
            }],
            seen_in: titles,
            ..ProposedWord::default()
        })
        .collect();
    Ok(WordsPackage {
        words,
        dropped: Vec::new(),
        warnings: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::register::{self, NewTerm};
    use serde_json::json;

    #[test]
    fn the_owners_marks_are_proposed_where_the_book_cannot_say_them() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        fixtures::version(
            &conn,
            &song.id,
            "lyrics",
            "В МарсЭль, где пУльсар,\nсЕти и ещё кваквАкряк\nпульсАр",
        );
        let other = fixtures::song(&conn, &profile_id, "Shore");
        fixtures::version(&conn, &other.id, "lyrics", "и снова МарсЭль");
        fixtures::version(
            &conn,
            &other.id,
            "style",
            "МарсЭль as a style word is not sung",
        );

        let package = from_texts(&conn, &profile_id).unwrap();

        let proposed: Vec<(String, String)> = package
            .words
            .iter()
            .map(|word| (word.sung[0].written.clone(), word.sung[0].sung.clone()))
            .collect();
        assert_eq!(
            proposed,
            [
                ("кваквакряк".to_owned(), "кваквАкряк".to_owned()),
                ("марсель".to_owned(), "марсЭль".to_owned()),
                ("пульсар".to_owned(), "пУльсар".to_owned()),
            ],
            "the homograph's mark stays the song's, a stress the book gives is not news, a ё is a spelling"
        );
        assert_eq!(package.words[1].seen_in, ["Tide", "Shore"]);

        keep(&conn, package, None).unwrap();
        assert!(
            from_texts(&conn, &profile_id).unwrap().is_empty(),
            "kept once, not proposed again"
        );
    }

    #[test]
    fn a_package_is_read_with_its_works_and_what_was_left_out() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        let package = read(
            &conn,
            &profile_id,
            &json!({ "words": [
                "пульсар",
                { "word": "Марсель", "sung": [{ "written": "Марсель", "sung": "МарсЭль" }] },
                { "word": "a lighthouse nobody keeps", "kind": "image", "works": [song.id, "Nowhere"] },
                { "word": "  " }
            ]}),
        )
        .unwrap();

        assert_eq!(package.words.len(), 3);
        assert!(package.words[0].phrase, "a bare word is a phrase to keep");
        assert_eq!(package.words[2].works.len(), 1);
        assert_eq!(package.dropped.len(), 2, "{:?}", package.dropped);
    }

    #[test]
    fn keeping_adds_to_a_word_the_record_holds_and_makes_what_it_does_not() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        let coffee = register::create(
            &conn,
            &profile_id,
            NewTerm {
                word: "кофе".into(),
                ..NewTerm::default()
            },
        )
        .unwrap();
        let package = read(
            &conn,
            &profile_id,
            &json!([
                { "word": "Кофе", "phrase": true, "shelf": "Kitchen" },
                { "word": "пульсар", "phrase": true, "shelf": "kitchen",
                  "sung": [{ "written": "пульсар", "sung": "пульсАр" }] },
                { "word": "a lighthouse nobody keeps", "kind": "image", "strictness": "limit", "works": [song.id] }
            ]),
        )
        .unwrap();

        let kept = keep(&conn, package, Some(&[item(0), item(1)])).unwrap();
        let (terms, phrases) = (kept.terms, kept.phrases);

        assert_eq!(
            (terms.len(), phrases.len()),
            (1, 2),
            "two phrases and one term - the third item was not taken"
        );
        let coffee = register::get(&conn, &coffee.id).unwrap().unwrap();
        assert_eq!(
            coffee.strictness,
            Some(register::Strictness::Limit),
            "a word kept as a phrase leaves the register's term as it was"
        );
        let shelves = crate::shelf::views(&conn, &profile_id).unwrap();
        assert_eq!(shelves.len(), 1, "the shelf is made once and found by name");
        assert_eq!(shelves[0].note_ids.len(), 2);
        let kept: Vec<String> = crate::shelf::notes(&conn, &shelves[0].shelf.id)
            .unwrap()
            .into_iter()
            .map(|note| note.body)
            .collect();
        assert_eq!(kept, ["Кофе", "пульсар"]);
        let pulsar = register::find_word(&conn, &profile_id, "пульсар", None)
            .unwrap()
            .expect("the way it is sung is the record's");
        assert_eq!(pulsar.sung[0].sung, "пульсАр");
        assert_eq!(pulsar.strictness, None);
        assert_eq!(register::list(&conn, &profile_id).unwrap().len(), 2);
    }

    /// A proposal about a word the record keeps changes it, and says what
    /// it changes: was, becomes (FR-08 8.3). Until v0.116 a strictness
    /// proposed for a word that had one, a kind, a topic and a note were
    /// dropped without a word.
    #[test]
    fn a_known_word_is_changed_and_the_change_is_said() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let term = register::create(
            &conn,
            &profile_id,
            register::NewTerm {
                word: "lighthouse".into(),
                forms: vec!["lighthouses".into()],
                strictness: Some(register::Strictness::Rare),
                note: Some("first note".into()),
                ..register::NewTerm::default()
            },
        )
        .unwrap();

        let package = read(
            &conn,
            &profile_id,
            &json!([{
                "word": "Lighthouse", "strictness": "ban", "forms": ["beacon"],
                "replace_forms": true, "topic": "sea", "note": "second note",
                "works": [song.id]
            }]),
        )
        .unwrap();
        let word = &package.words[0];
        assert!(word.known);
        let said: Vec<(&str, &str, &str)> = word
            .changes
            .iter()
            .map(|change| {
                (
                    change.field.as_str(),
                    change.was.as_str(),
                    change.becomes.as_str(),
                )
            })
            .collect();
        assert_eq!(
            said,
            [
                ("strictness", "rare", "ban"),
                ("forms", "lighthouses", "beacon"),
                ("topic", "-", "sea"),
                ("note", "first note", "second note"),
                ("works", "-", "Harbour lights"),
            ]
        );
        let body = render(&package);
        assert!(body.contains("strictness: rare → ban"), "{body}");

        let kept = keep(&conn, package, None).unwrap();
        assert_eq!(kept.unapplied, Vec::new());
        let after = register::get(&conn, &term.id).unwrap().unwrap();
        assert_eq!(after.strictness, Some(register::Strictness::Ban));
        assert_eq!(after.forms, ["beacon"]);
        assert_eq!(after.topic.as_deref(), Some("sea"));
        assert_eq!(after.note.as_deref(), Some("second note"));
        assert!(
            register::uses(&conn, &term.id)
                .unwrap()
                .iter()
                .any(|used| used.work_id == song.id && used.named)
        );

        // `"strictness": null` unspends; a second proposal of what the
        // record now says changes nothing and says so.
        let unspend = read(
            &conn,
            &profile_id,
            &json!([{ "word": "lighthouse", "strictness": null }]),
        )
        .unwrap();
        assert_eq!(unspend.words[0].changes[0].becomes, "-");
        keep(&conn, unspend, None).unwrap();
        assert_eq!(
            register::get(&conn, &term.id).unwrap().unwrap().strictness,
            None
        );
        let same = read(
            &conn,
            &profile_id,
            &json!([{ "word": "lighthouse", "topic": "sea" }]),
        )
        .unwrap();
        assert!(same.words[0].changes.is_empty());
        assert!(render(&same).contains("nothing to change"));
    }

    /// A word is taken out of the record into the trash, and one that is
    /// several is split into them, each with what the one held.
    #[test]
    fn a_word_is_taken_out_and_a_word_of_two_is_split() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let wrong = register::create(
            &conn,
            &profile_id,
            register::NewTerm {
                word: "tpyo".into(),
                ..register::NewTerm::default()
            },
        )
        .unwrap();
        let glued = register::create(
            &conn,
            &profile_id,
            register::NewTerm {
                word: "spacesuit, isotope".into(),
                kind: Some(register::TermKind::Image),
                strictness: Some(register::Strictness::Limit),
                topic: Some("space".into()),
                ..register::NewTerm::default()
            },
        )
        .unwrap();
        register::link(&conn, &glued.id, &song.id).unwrap();

        let package = read(
            &conn,
            &profile_id,
            &json!([
                { "word": "tpyo", "action": "delete" },
                { "word": "spacesuit, isotope", "action": "split", "into": ["spacesuit", " isotope "] },
                { "word": "never kept", "action": "delete" },
                { "word": "tpyo", "action": "split" }
            ]),
        )
        .unwrap();
        // What is not in the record, and a split into nothing, are left out
        // and said.
        assert_eq!(package.words.len(), 2);
        let why: Vec<&str> = package
            .dropped
            .iter()
            .map(|reason| reason.key.as_str())
            .collect();
        assert_eq!(
            why,
            [
                "refusal.words.notInRecord",
                "refusal.words.splitIntoNothing"
            ]
        );

        let kept = keep(&conn, package, None).unwrap();
        assert!(register::get(&conn, &wrong.id).unwrap().is_none());
        assert!(register::get(&conn, &glued.id).unwrap().is_none());
        // In the trash, not destroyed.
        let buried: Vec<String> = crate::trash::list(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .map(|entry| entry.entity_id)
            .collect();
        assert!(buried.contains(&wrong.id) && buried.contains(&glued.id));

        for part in ["spacesuit", "isotope"] {
            let term = register::find_word(&conn, &profile_id, part, None)
                .unwrap()
                .unwrap_or_else(|| panic!("no `{part}` after the split"));
            assert!(kept.terms.contains(&term.id));
            assert_eq!(term.kind, register::TermKind::Image);
            assert_eq!(term.strictness, Some(register::Strictness::Limit));
            assert_eq!(term.topic.as_deref(), Some("space"));
            assert!(
                register::uses(&conn, &term.id)
                    .unwrap()
                    .iter()
                    .any(|used| used.work_id == song.id && used.named),
                "the work named for the one is named for `{part}`"
            );
        }
    }

    /// A new word written with a comma or a slash reads as several: kept as
    /// proposed, with a warning said beside it.
    #[test]
    fn a_word_that_reads_as_several_is_warned_about() {
        let (conn, profile_id) = fixtures::workspace();
        let package = read(
            &conn,
            &profile_id,
            &json!([
                { "word": "candle / candles", "strictness": "limit" },
                { "word": "lantern", "strictness": "limit" },
                "a line, with a comma, kept as a phrase"
            ]),
        )
        .unwrap();
        assert_eq!(package.words.len(), 3);
        assert_eq!(package.warnings.len(), 1);
        assert_eq!(package.warnings[0].key, "refusal.words.separator");
        assert!(render(&package).contains("Mind:"));
    }
}
