//! Words proposed for the record, applied with one click (ADR 0052).
//!
//! One package for everything an agent, an action or kilna itself proposes
//! about words: words for the bank and the block they go in, how a word is
//! sung, a term for the register, and the works a meaning is in. The words of
//! the bank and the register's terms are one record, so they are one
//! proposal: "пульсар" proposed for the bank, sung "пульсАр", is one item,
//! and taking it writes one row.
//!
//! The package is read and checked against the workspace as it stands, and
//! kept whole or item by item (`word:N`), the way a package for the canon is
//! (ADR 0018).

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Bank, Strictness, Sung, TermKind, TermPatch};
use crate::error::{Error, Reason, Result};

/// One word as it is proposed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct ProposedWord {
    /// The word as it is written: the record's key, matched whatever its case.
    pub word: String,
    /// Other forms counted as the same word.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forms: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub kind: Option<TermKind>,
    /// Kept in the bank, as fresh.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bank: bool,
    /// The block of the bank it goes in, by name; made when there is none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub block: Option<String>,
    /// How its forms are sung where that is not how they are written.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sung: Vec<Sung>,
    /// Spent: in the register, this strictly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub strictness: Option<Strictness>,
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
            Value::String(word) => serde_json::json!({ "word": word, "bank": true }),
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
        let mut stripped = raw.clone();
        if let Some(object) = stripped.as_object_mut() {
            object.remove("works");
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
        package.words.push(word);
    }
    Ok(package)
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

/// Keep the items taken, as a hand would: the word found or made, its
/// facets added - never taken away - its block found or made, the works it
/// names named. Each a gesture of its own; the caller holds the unit. The
/// ids of the words written.
pub fn keep(
    conn: &Connection,
    package: WordsPackage,
    items: Option<&[String]>,
) -> Result<Vec<String>> {
    let profile_id = crate::actions::active_profile_id(conn)?;
    let mut written = Vec::new();
    for (index, word) in package.words.into_iter().enumerate() {
        if !taken(items, index) {
            continue;
        }
        let term = match super::find_word(conn, &profile_id, &word.word, None)? {
            Some(found) => {
                let mut patch = TermPatch::default();
                if word.bank && found.bank.is_none() {
                    patch.bank = Some(Some(Bank::Fresh));
                }
                if let Some(strictness) = word.strictness
                    && found.strictness.is_none()
                {
                    patch.strictness = Some(Some(strictness));
                }
                if !word.sung.is_empty() {
                    let mut sung = found.sung.clone();
                    sung.extend(word.sung.iter().cloned());
                    let sung = super::clean_sung(sung);
                    if sung != found.sung {
                        patch.sung = Some(sung);
                    }
                }
                let mut forms = found.forms.clone();
                forms.extend(word.forms.iter().cloned());
                let forms = super::clean_forms(&found.word, forms);
                if forms != found.forms {
                    patch.forms = Some(forms);
                }
                if patch.bank.is_none()
                    && patch.strictness.is_none()
                    && patch.sung.is_none()
                    && patch.forms.is_none()
                {
                    found
                } else {
                    crate::actions::register::update(conn, &found.id, patch)?
                }
            }
            None => crate::actions::register::create(
                conn,
                super::NewTerm {
                    word: word.word.clone(),
                    forms: word.forms.clone(),
                    kind: word.kind,
                    strictness: word.strictness,
                    bank: word.bank.then_some(Bank::Fresh),
                    sung: word.sung.clone(),
                    topic: None,
                    note: word.note.clone(),
                },
            )?,
        };
        if let Some(name) = word.block.as_deref().filter(|name| !name.trim().is_empty()) {
            let block = match super::block::find(conn, &profile_id, name)? {
                Some(found) => found,
                None => crate::actions::register::create_block(conn, name)?,
            };
            crate::actions::register::add_to_block(conn, &block.id, &term.id)?;
        }
        for named in &word.works {
            crate::actions::register::link(conn, &term.id, &named.id)?;
        }
        written.push(term.id);
    }
    Ok(written)
}

/// The package as a person reads it before keeping it: a line per word with
/// what would change.
pub fn render(package: &WordsPackage) -> String {
    let mut out = String::new();
    for word in &package.words {
        let mut line = format!("- **{}**", word.word);
        let mut said: Vec<String> = Vec::new();
        if word.bank {
            said.push(match &word.block {
                Some(block) => format!("bank · {block}"),
                None => "bank".to_owned(),
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
        assert!(package.words[0].bank, "a bare word is a word for the bank");
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
                { "word": "Кофе", "bank": true, "block": "Kitchen" },
                { "word": "пульсар", "bank": true, "block": "kitchen",
                  "sung": [{ "written": "пульсар", "sung": "пульсАр" }] },
                { "word": "a lighthouse nobody keeps", "kind": "image", "strictness": "limit", "works": [song.id] }
            ]),
        )
        .unwrap();

        let written = keep(&conn, package, Some(&[item(0), item(1)])).unwrap();

        assert_eq!(written.len(), 2, "the third item was not taken");
        let coffee = register::get(&conn, &coffee.id).unwrap().unwrap();
        assert_eq!(
            (coffee.strictness, coffee.bank),
            (Some(register::Strictness::Limit), Some(Bank::Fresh)),
            "one word, one record: the register's term joins the bank"
        );
        let blocks = register::block::views(&conn, &profile_id).unwrap();
        assert_eq!(blocks.len(), 1, "the block is made once and found by name");
        assert_eq!(blocks[0].term_ids.len(), 2);
        let pulsar = register::get(&conn, &written[1]).unwrap().unwrap();
        assert_eq!(pulsar.sung[0].sung, "пульсАр");
        assert_eq!(pulsar.strictness, None);
        assert_eq!(register::list(&conn, &profile_id).unwrap().len(), 2);
    }
}
