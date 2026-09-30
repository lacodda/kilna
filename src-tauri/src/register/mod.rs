//! The register of repeats: the words, images and scenes a body of work has
//! spent (ADR 0044).
//!
//! A term is either **wording** - a noun, an adjective, a verb, a phrase - or a
//! **meaning** - an image, a scene, a pattern. Wording is found in a text by its
//! forms, compared by stem ([`matching`]); a meaning cannot be found by its
//! words, and the works that carry it are named, as rows of `term_work`. Which
//! works carry a term - the register's "in how many songs" - is read off the
//! works every time it is asked ([`uses`]), never written down: a count typed by
//! hand is out of date the day after the next song.
//!
//! What a text of the work being written already takes from the register is
//! [`check`]; what the assistant is handed is [`sheet`]; the works whose words
//! stand closest to a text are [`neighbours`].

pub mod check;
pub mod matching;
pub mod neighbours;
pub mod sheet;

use std::collections::{BTreeSet, HashMap};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::time::now;

/// How strictly a term is off limits. The code reads it: it decides how a hit
/// is marked in the text and what the assistant is told.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
pub enum Strictness {
    /// Spent: in so many works that one more says nothing new.
    Ban,
    /// Worn: used often enough that each use has to earn its place.
    #[default]
    Limit,
    /// Rare but conspicuous: a word used so seldom that even two or three
    /// uses stand out.
    Rare,
}

impl Strictness {
    pub const ALL: [Strictness; 3] = [Strictness::Ban, Strictness::Limit, Strictness::Rare];

    pub fn as_str(self) -> &'static str {
        match self {
            Strictness::Ban => "ban",
            Strictness::Limit => "limit",
            Strictness::Rare => "rare",
        }
    }

    fn parse(raw: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|one| one.as_str() == raw)
            .ok_or_else(|| Error::Internal(format!("a stored strictness reads `{raw}`")))
    }
}

/// What a term is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum TermKind {
    Noun,
    Adjective,
    Verb,
    /// Words that go together: "on the edge", "burnt bridges".
    Phrase,
    /// A figure: "a lighthouse nobody keeps".
    Image,
    /// A situation: "smoking at the window at night".
    Scene,
    /// A device: "just X", where X belittles.
    Pattern,
}

impl TermKind {
    pub const ALL: [TermKind; 7] = [
        TermKind::Noun,
        TermKind::Adjective,
        TermKind::Verb,
        TermKind::Phrase,
        TermKind::Image,
        TermKind::Scene,
        TermKind::Pattern,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            TermKind::Noun => "noun",
            TermKind::Adjective => "adjective",
            TermKind::Verb => "verb",
            TermKind::Phrase => "phrase",
            TermKind::Image => "image",
            TermKind::Scene => "scene",
            TermKind::Pattern => "pattern",
        }
    }

    fn parse(raw: &str) -> Result<Self> {
        Self::from_word(raw)
            .ok_or_else(|| Error::Internal(format!("a stored term kind reads `{raw}`")))
    }

    /// A kind as a caller names it; none for any other word.
    pub fn from_word(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|one| one.as_str() == raw.trim())
    }

    /// Whether a term of this kind is found in a text by its words. A meaning
    /// is not: its works are named.
    pub fn is_wording(self) -> bool {
        matches!(
            self,
            TermKind::Noun | TermKind::Adjective | TermKind::Verb | TermKind::Phrase
        )
    }

    /// The kind a term written with these words most likely is: one word is a
    /// noun until the person says otherwise, several are a phrase.
    pub fn guess(word: &str) -> Self {
        if crate::words::words(word).len() > 1 {
            TermKind::Phrase
        } else {
            TermKind::Noun
        }
    }
}

/// One entry of the register.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Term {
    pub id: String,
    pub profile_id: String,
    /// The term as a person reads it: "окно", "on the edge", "a lighthouse
    /// nobody keeps".
    pub word: String,
    /// Other words counted as the same term. Cases need none: words are
    /// compared by their stems.
    pub forms: Vec<String>,
    pub kind: TermKind,
    pub strictness: Strictness,
    /// The register's own grouping: "the kitchen", "physics and space".
    pub topic: Option<String>,
    /// Why it is spent, or what to reach for instead.
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct NewTerm {
    pub word: String,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub forms: Vec<String>,
    /// Guessed from the words when absent: see [`TermKind::guess`].
    #[serde(default)]
    pub kind: Option<TermKind>,
    /// A limit when absent.
    #[serde(default)]
    pub strictness: Option<Strictness>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
pub struct TermPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub word: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forms: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<TermKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strictness: Option<Strictness>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub topic: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub note: Option<Option<String>>,
}

/// A term with the works it is in, counted as it stands: the register as the
/// screen and the assistant read it.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct RegisterEntry {
    #[serde(flatten)]
    pub term: Term,
    /// How many works carry it: found in their text, or named.
    pub uses: usize,
}

/// A work a term is in, and how it is known to be.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct TermUse {
    pub work_id: String,
    pub title: String,
    pub kind: String,
    /// How many times the work's current text says it. Zero for a work that
    /// is only named.
    pub found: usize,
    /// Whether a person named the work as carrying it.
    pub named: bool,
}

const SELECT_TERM: &str = "SELECT id, profile_id, word, forms, kind, strictness, topic, note, \
     created_at, updated_at FROM term";

/// The words a term's forms are kept as: trimmed, the empty ones and the term's
/// own word dropped, each once whatever its case.
pub fn clean_forms(word: &str, forms: Vec<String>) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    let own = crate::words::plain(word.trim());
    for form in forms {
        let form = form.trim().to_owned();
        let plain = crate::words::plain(&form);
        if form.is_empty() || plain == own || kept.iter().any(|k| crate::words::plain(k) == plain) {
            continue;
        }
        kept.push(form);
    }
    kept
}

/// A text the person left blank reads as nothing, not as an empty string.
fn blank_is_none(text: Option<String>) -> Option<String> {
    text.map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

pub fn create(conn: &Connection, profile_id: &str, new: NewTerm) -> Result<Term> {
    create_minted(conn, profile_id, new, Minted::fresh())
}

/// Create a term with the id and moment already decided: the seam a replay
/// comes back through (ADR 0014).
pub fn create_minted(
    conn: &Connection,
    profile_id: &str,
    new: NewTerm,
    minted: Minted,
) -> Result<Term> {
    let word = new.word.trim().to_owned();
    if word.is_empty() {
        return Err(Error::refused("term.needsWord"));
    }
    if let Some(existing) = find_word(conn, profile_id, &word, None)? {
        return Err(Error::refused("term.exists").param("word", existing.word));
    }
    let kind = new.kind.unwrap_or_else(|| TermKind::guess(&word));
    let forms = clean_forms(&word, new.forms);

    conn.execute(
        "INSERT INTO term (id, profile_id, word, forms, kind, strictness, topic, note, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![
            minted.id(),
            profile_id,
            word,
            serde_json::to_string(&forms)?,
            kind.as_str(),
            new.strictness.unwrap_or_default().as_str(),
            blank_is_none(new.topic),
            blank_is_none(new.note),
            minted.at(),
        ],
    )?;
    get(conn, minted.id())?.ok_or_else(|| Error::Internal("the term vanished after insert".into()))
}

/// The term of a profile written with these words, whatever their case - the
/// register keeps a word once. `except` is the term being renamed.
fn find_word(
    conn: &Connection,
    profile_id: &str,
    word: &str,
    except: Option<&str>,
) -> Result<Option<Term>> {
    let wanted = crate::words::plain(word);
    Ok(list(conn, profile_id)?
        .into_iter()
        .find(|term| Some(term.id.as_str()) != except && crate::words::plain(&term.word) == wanted))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Term>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_TERM} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?;
    raw.map(RawTerm::into_term).transpose()
}

/// Every term of a profile, in the order the register reads: strictest first,
/// then by word.
pub fn list(conn: &Connection, profile_id: &str) -> Result<Vec<Term>> {
    let mut statement = conn.prepare(&format!("{SELECT_TERM} WHERE profile_id = ?1"))?;
    let raw = statement
        .query_map(params![profile_id], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut terms = raw
        .into_iter()
        .map(RawTerm::into_term)
        .collect::<Result<Vec<_>>>()?;
    terms.sort_by(|a, b| {
        a.strictness
            .cmp(&b.strictness)
            .then_with(|| crate::words::plain(&a.word).cmp(&crate::words::plain(&b.word)))
    });
    Ok(terms)
}

pub fn update(conn: &Connection, id: &str, patch: TermPatch) -> Result<Term> {
    update_at(conn, id, patch, &now())
}

/// Update a term with the change's moment already decided (ADR 0014).
pub fn update_at(conn: &Connection, id: &str, patch: TermPatch, at: &str) -> Result<Term> {
    let found = get(conn, id)?.ok_or_else(|| unknown(id))?;
    let mut assignments: Vec<String> = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    let mut set = |column: &str, value: Box<dyn rusqlite::ToSql>| {
        values.push(value);
        assignments.push(format!("{column} = ?{}", values.len()));
    };

    let word = match patch.word {
        Some(word) => {
            let word = word.trim().to_owned();
            if word.is_empty() {
                return Err(Error::refused("term.needsWord"));
            }
            if let Some(existing) = find_word(conn, &found.profile_id, &word, Some(id))? {
                return Err(Error::refused("term.exists").param("word", existing.word));
            }
            set("word", Box::new(word.clone()));
            word
        }
        None => found.word.clone(),
    };
    // Forms are kept against the word they stand beside, so a rename that
    // makes a form the word itself drops the form.
    if let Some(forms) = patch
        .forms
        .or_else(|| (word != found.word).then(|| found.forms.clone()))
    {
        set(
            "forms",
            Box::new(serde_json::to_string(&clean_forms(&word, forms))?),
        );
    }
    if let Some(kind) = patch.kind {
        set("kind", Box::new(kind.as_str().to_owned()));
    }
    if let Some(strictness) = patch.strictness {
        set("strictness", Box::new(strictness.as_str().to_owned()));
    }
    if let Some(topic) = patch.topic {
        set("topic", Box::new(blank_is_none(topic)));
    }
    if let Some(note) = patch.note {
        set("note", Box::new(blank_is_none(note)));
    }

    if assignments.is_empty() {
        return Ok(found);
    }
    values.push(Box::new(at.to_owned()));
    assignments.push(format!("updated_at = ?{}", values.len()));
    values.push(Box::new(id.to_owned()));
    let sql = format!(
        "UPDATE term SET {} WHERE id = ?{}",
        assignments.join(", "),
        values.len()
    );
    conn.execute(
        &sql,
        rusqlite::params_from_iter(values.iter().map(AsRef::as_ref)),
    )?;
    get(conn, id)?.ok_or_else(|| unknown(id))
}

/// Every topic in use in a profile, with how many terms it groups.
pub fn topics(conn: &Connection, profile_id: &str) -> Result<Vec<(String, i64)>> {
    let mut statement = conn.prepare(
        "SELECT topic, count(*) FROM term WHERE profile_id = ?1 AND topic IS NOT NULL
         GROUP BY topic ORDER BY count(*) DESC, topic",
    )?;
    let rows = statement.query_map(params![profile_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Name a work as carrying a term.
pub fn link(conn: &Connection, term_id: &str, work_id: &str) -> Result<()> {
    link_minted(conn, term_id, work_id, Minted::fresh())
}

/// [`link`] with the id and moment already decided (ADR 0014). Naming a work
/// twice is naming it once.
pub fn link_minted(conn: &Connection, term_id: &str, work_id: &str, minted: Minted) -> Result<()> {
    let term = get(conn, term_id)?.ok_or_else(|| unknown(term_id))?;
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    if work.profile_id != term.profile_id {
        return Err(Error::not_found("work", work_id));
    }
    conn.execute(
        "INSERT INTO term_work (id, profile_id, term_id, work_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT (term_id, work_id) DO NOTHING",
        params![minted.id(), term.profile_id, term_id, work_id, minted.at()],
    )?;
    Ok(())
}

/// Let go of a work named as carrying a term. Whether a row was there.
pub fn unlink(conn: &Connection, term_id: &str, work_id: &str) -> Result<bool> {
    Ok(conn.execute(
        "DELETE FROM term_work WHERE term_id = ?1 AND work_id = ?2",
        params![term_id, work_id],
    )? > 0)
}

/// The works named as carrying each term of a profile.
fn named(conn: &Connection, profile_id: &str) -> Result<HashMap<String, BTreeSet<String>>> {
    let mut statement =
        conn.prepare("SELECT term_id, work_id FROM term_work WHERE profile_id = ?1")?;
    let rows = statement.query_map(params![profile_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut out: HashMap<String, BTreeSet<String>> = HashMap::new();
    for row in rows {
        let (term, work) = row?;
        out.entry(term).or_default().insert(work);
    }
    Ok(out)
}

/// The register as it stands: every term with how many works carry it now.
///
/// One walk over the works' current texts for all the terms, not one per
/// term: the screen asks for the whole register at once, and the texts are
/// the expensive half.
pub fn entries(conn: &Connection, profile_id: &str) -> Result<Vec<RegisterEntry>> {
    let terms = list(conn, profile_id)?;
    let found = matching::found_in_corpus(conn, profile_id, &terms)?;
    let named = named(conn, profile_id)?;
    Ok(terms
        .into_iter()
        .map(|term| {
            let mut works: BTreeSet<&str> = BTreeSet::new();
            if let Some(found) = found.get(&term.id) {
                works.extend(found.keys().map(String::as_str));
            }
            if let Some(named) = named.get(&term.id) {
                works.extend(named.iter().map(String::as_str));
            }
            let uses = works.len();
            RegisterEntry { term, uses }
        })
        .collect())
}

/// The works a term is in: found in their current text, or named - the
/// strongest first.
pub fn uses(conn: &Connection, term_id: &str) -> Result<Vec<TermUse>> {
    let term = get(conn, term_id)?.ok_or_else(|| unknown(term_id))?;
    let found = matching::found_in_corpus(conn, &term.profile_id, std::slice::from_ref(&term))?
        .remove(&term.id)
        .unwrap_or_default();
    let named: BTreeSet<String> = named(conn, &term.profile_id)?
        .remove(&term.id)
        .unwrap_or_default();

    let mut ids: BTreeSet<&str> = found.keys().map(String::as_str).collect();
    ids.extend(named.iter().map(String::as_str));
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        // A work in the trash is not where a term is used any more.
        let Some(work) = crate::work::get(conn, id)? else {
            continue;
        };
        out.push(TermUse {
            found: found.get(id).copied().unwrap_or(0),
            named: named.contains(id),
            work_id: work.id,
            title: work.title,
            kind: work.kind,
        });
    }
    out.sort_by(|a, b| {
        b.found
            .cmp(&a.found)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    Ok(out)
}

/// How many works these words would be found in, before they are a term: what
/// the form for a new term says while it is being typed.
pub fn preview(conn: &Connection, profile_id: &str, word: &str, forms: &[String]) -> Result<usize> {
    if word.trim().is_empty() {
        return Ok(0);
    }
    let draft = Term {
        id: String::new(),
        profile_id: profile_id.to_owned(),
        word: word.trim().to_owned(),
        forms: forms.to_vec(),
        kind: TermKind::guess(word),
        strictness: Strictness::default(),
        topic: None,
        note: None,
        created_at: String::new(),
        updated_at: String::new(),
    };
    Ok(
        matching::found_in_corpus(conn, profile_id, std::slice::from_ref(&draft))?
            .remove("")
            .map_or(0, |works| works.len()),
    )
}

fn unknown(id: &str) -> Error {
    Error::not_found("term", id)
}

struct RawTerm {
    id: String,
    profile_id: String,
    word: String,
    forms: String,
    kind: String,
    strictness: String,
    topic: Option<String>,
    note: Option<String>,
    created_at: String,
    updated_at: String,
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawTerm> {
    Ok(RawTerm {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        word: row.get(2)?,
        forms: row.get(3)?,
        kind: row.get(4)?,
        strictness: row.get(5)?,
        topic: row.get(6)?,
        note: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

impl RawTerm {
    fn into_term(self) -> Result<Term> {
        Ok(Term {
            forms: serde_json::from_str(&self.forms)?,
            kind: TermKind::parse(&self.kind)?,
            strictness: Strictness::parse(&self.strictness)?,
            id: self.id,
            profile_id: self.profile_id,
            word: self.word,
            topic: self.topic,
            note: self.note,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::fixtures;
    use crate::work::{self, NewWork, version};

    pub(crate) fn song(conn: &Connection, profile_id: &str, title: &str, text: &str) -> String {
        let made = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: title.into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        version::create(
            conn,
            &made.id,
            version::NewVersion {
                role: "lyrics".into(),
                body: text.into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
        made.id
    }

    fn term(word: &str) -> NewTerm {
        NewTerm {
            word: word.into(),
            ..NewTerm::default()
        }
    }

    #[test]
    fn a_term_keeps_its_word_once_whatever_the_case() {
        let (conn, profile_id) = fixtures::workspace();
        create(&conn, &profile_id, term("Window")).unwrap();

        let error = create(&conn, &profile_id, term("  window ")).unwrap_err();
        assert_eq!(error.refusal().map(|r| r.code), Some("term.exists"));
        assert!(create(&conn, &profile_id, term("   ")).is_err());
    }

    #[test]
    fn a_new_term_guesses_its_kind_and_is_a_limit_until_said_otherwise() {
        let (conn, profile_id) = fixtures::workspace();
        let one = create(&conn, &profile_id, term("shadow")).unwrap();
        let two = create(&conn, &profile_id, term("burnt bridges")).unwrap();

        assert_eq!(one.kind, TermKind::Noun);
        assert_eq!(two.kind, TermKind::Phrase);
        assert_eq!(one.strictness, Strictness::Limit);
    }

    #[test]
    fn forms_drop_the_word_itself_and_repeat_nothing() {
        let forms = clean_forms(
            "окно",
            vec!["Окно".into(), "окон".into(), " окон ".into(), "".into()],
        );
        assert_eq!(forms, ["окон"]);
    }

    #[test]
    fn the_count_is_read_off_the_current_texts_and_follows_them() {
        let (conn, profile_id) = fixtures::workspace();
        let first = song(&conn, &profile_id, "One", "a shadow on the wall");
        song(&conn, &profile_id, "Two", "shadows fall at noon");
        song(&conn, &profile_id, "Three", "nothing of the kind");
        let made = create(&conn, &profile_id, term("shadow")).unwrap();

        let count = |conn: &Connection| {
            entries(conn, &profile_id)
                .unwrap()
                .into_iter()
                .find(|entry| entry.term.id == made.id)
                .unwrap()
                .uses
        };
        assert_eq!(count(&conn), 2);

        // A rewrite that drops the word drops the work from the count.
        version::create(
            &conn,
            &first,
            version::NewVersion {
                role: "lyrics".into(),
                body: "a light on the wall".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
        assert_eq!(count(&conn), 1);
    }

    #[test]
    fn a_meaning_is_in_the_works_named_and_only_there() {
        let (conn, profile_id) = fixtures::workspace();
        let named = song(&conn, &profile_id, "Named", "the tea went cold");
        song(&conn, &profile_id, "Other", "a lighthouse nobody keeps");
        let image = create(
            &conn,
            &profile_id,
            NewTerm {
                word: "a lighthouse nobody keeps".into(),
                kind: Some(TermKind::Image),
                ..NewTerm::default()
            },
        )
        .unwrap();

        // Not looked for in the texts, even word for word: a meaning is told
        // a hundred ways, and the one way that matches is no evidence.
        assert!(uses(&conn, &image.id).unwrap().is_empty());

        link(&conn, &image.id, &named).unwrap();
        link(&conn, &image.id, &named).unwrap();
        let found = uses(&conn, &image.id).unwrap();
        assert_eq!(found.len(), 1, "named twice is named once");
        assert!(found[0].named);
        assert_eq!(found[0].found, 0);

        assert!(unlink(&conn, &image.id, &named).unwrap());
        assert!(uses(&conn, &image.id).unwrap().is_empty());
    }

    #[test]
    fn a_work_found_and_named_counts_once() {
        let (conn, profile_id) = fixtures::workspace();
        let one = song(&conn, &profile_id, "One", "the window, the window");
        let made = create(&conn, &profile_id, term("window")).unwrap();
        link(&conn, &made.id, &one).unwrap();

        let entry = entries(&conn, &profile_id).unwrap().remove(0);
        assert_eq!(entry.uses, 1);
        let found = uses(&conn, &made.id).unwrap();
        assert_eq!((found[0].found, found[0].named), (2, true));
    }

    #[test]
    fn a_preview_counts_words_before_they_are_a_term() {
        let (conn, profile_id) = fixtures::workspace();
        song(&conn, &profile_id, "One", "окно открыто");
        song(&conn, &profile_id, "Two", "в окне свет");
        song(&conn, &profile_id, "Three", "окон не видно");

        assert_eq!(preview(&conn, &profile_id, "окно", &[]).unwrap(), 2);
        assert_eq!(
            preview(&conn, &profile_id, "окно", &["окон".into()]).unwrap(),
            3
        );
        assert_eq!(preview(&conn, &profile_id, "  ", &[]).unwrap(), 0);
    }

    #[test]
    fn a_rename_is_refused_onto_a_word_already_kept() {
        let (conn, profile_id) = fixtures::workspace();
        create(&conn, &profile_id, term("dust")).unwrap();
        let ash = create(&conn, &profile_id, term("ash")).unwrap();

        let error = update(
            &conn,
            &ash.id,
            TermPatch {
                word: Some("Dust".into()),
                ..TermPatch::default()
            },
        )
        .unwrap_err();
        assert_eq!(error.refusal().map(|r| r.code), Some("term.exists"));
    }

    #[test]
    fn the_register_reads_strictest_first() {
        let (conn, profile_id) = fixtures::workspace();
        for (word, strictness) in [
            ("b", Strictness::Rare),
            ("a", Strictness::Limit),
            ("c", Strictness::Ban),
        ] {
            create(
                &conn,
                &profile_id,
                NewTerm {
                    word: word.into(),
                    strictness: Some(strictness),
                    ..NewTerm::default()
                },
            )
            .unwrap();
        }
        let words: Vec<String> = list(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .map(|t| t.word)
            .collect();
        assert_eq!(words, ["c", "a", "b"]);
    }
}
