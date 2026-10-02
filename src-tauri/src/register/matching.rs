//! Finding a term's wording in a text.
//!
//! A term is found by its word and its forms, each read as a run of stems: the
//! word "окно" is found in "окна" and "окном", the phrase "на краю" in "На
//! краю" - and "окон", a form the light stemmer does not reach, only when the
//! term lists it. Whole words only: "чай" is not in "чайка". What stands in
//! square brackets is a section's name and is never a hit.
//!
//! Meanings are not looked for at all (see [`super::TermKind::is_wording`]).

use std::collections::HashMap;

use rusqlite::{Connection, params};

use super::Term;
use crate::error::Result;
use crate::words::{stem, words};

/// Stems already worked out, by the lowercased word. A walk over every work of
/// a workspace meets "и" a thousand times and "окно" a hundred; each is
/// stemmed once.
#[derive(Default)]
pub struct Stems(HashMap<String, String>);

impl Stems {
    pub fn of(&mut self, word: &str) -> String {
        let lower = crate::words::plain(word);
        if let Some(found) = self.0.get(&lower) {
            return found.clone();
        }
        let key = stem(&lower);
        self.0.insert(lower, key.clone());
        key
    }
}

/// One place a term's wording stands in a text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    /// In UTF-16 units, as the window counts.
    pub start: usize,
    pub end: usize,
    /// Index into the terms the matcher was built from.
    pub term: usize,
}

/// The wording of a set of terms, ready to be looked for.
pub struct Matcher {
    /// By a pattern's first stem: the term it belongs to and the whole run.
    by_first: HashMap<String, Vec<(usize, Vec<String>)>>,
}

impl Matcher {
    /// Every form of every term that is wording. A form of no words at all -
    /// "…" - is nothing to look for and is left out.
    pub fn new(terms: &[Term], stems: &mut Stems) -> Self {
        let mut by_first: HashMap<String, Vec<(usize, Vec<String>)>> = HashMap::new();
        for (index, term) in terms.iter().enumerate() {
            if !term.kind.is_wording() {
                continue;
            }
            for form in std::iter::once(&term.word).chain(term.forms.iter()) {
                let run: Vec<String> = words(form).iter().map(|w| stems.of(w.text)).collect();
                let Some(first) = run.first().cloned() else {
                    continue;
                };
                let patterns = by_first.entry(first).or_default();
                if !patterns.iter().any(|(t, p)| *t == index && *p == run) {
                    patterns.push((index, run));
                }
            }
        }
        Self { by_first }
    }

    /// Every place in `text` a term's wording stands, in text order. Where two
    /// forms of one term overlap - "window" and "window pane" - the longer
    /// is the hit.
    pub fn find(&self, text: &str, stems: &mut Stems) -> Vec<Hit> {
        if self.by_first.is_empty() {
            return Vec::new();
        }
        let found: Vec<_> = words(text).into_iter().filter(|w| !w.bracketed).collect();
        let keys: Vec<String> = found.iter().map(|w| stems.of(w.text)).collect();

        let mut hits: Vec<Hit> = Vec::new();
        for (at, key) in keys.iter().enumerate() {
            let Some(patterns) = self.by_first.get(key) else {
                continue;
            };
            for (term, run) in patterns {
                if keys.len() - at >= run.len() && keys[at..at + run.len()] == run[..] {
                    hits.push(Hit {
                        start: found[at].start,
                        end: found[at + run.len() - 1].end,
                        term: *term,
                    });
                }
            }
        }
        hits.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
        let mut kept: Vec<Hit> = Vec::with_capacity(hits.len());
        for hit in hits {
            let overlaps_own = kept
                .iter()
                .any(|k| k.term == hit.term && k.start < hit.end && hit.start < k.end);
            if !overlaps_own {
                kept.push(hit);
            }
        }
        kept
    }
}

/// A work's text as it stands: its current version.
pub struct Current {
    pub work_id: String,
    pub title: String,
    pub kind: String,
    pub body: String,
}

/// The current text of every work of a profile. A work in the trash is not
/// here - the trash takes its rows out of the table.
pub fn corpus(conn: &Connection, profile_id: &str) -> Result<Vec<Current>> {
    let mut statement = conn.prepare(
        "SELECT w.id, w.title, w.kind, v.body FROM work w
           JOIN work_version v ON v.id = w.current_version_id
          WHERE w.profile_id = ?1
          ORDER BY w.created_at, w.rowid",
    )?;
    let rows = statement.query_map(params![profile_id], |row| {
        Ok(Current {
            work_id: row.get(0)?,
            title: row.get(1)?,
            kind: row.get(2)?,
            body: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// For each term, the works whose current text says it and how many times -
/// keyed by the term's id.
pub fn found_in_corpus(
    conn: &Connection,
    profile_id: &str,
    terms: &[Term],
) -> Result<HashMap<String, HashMap<String, usize>>> {
    let mut stems = Stems::default();
    let matcher = Matcher::new(terms, &mut stems);
    let mut out: HashMap<String, HashMap<String, usize>> = HashMap::new();
    if matcher.by_first.is_empty() {
        return Ok(out);
    }
    for text in corpus(conn, profile_id)? {
        for hit in matcher.find(&text.body, &mut stems) {
            *out.entry(terms[hit.term].id.clone())
                .or_default()
                .entry(text.work_id.clone())
                .or_default() += 1;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::register::{Strictness, TermKind};

    fn term(word: &str, forms: &[&str], kind: TermKind) -> Term {
        Term {
            id: word.into(),
            profile_id: String::new(),
            word: word.into(),
            forms: forms.iter().map(|f| (*f).to_owned()).collect(),
            kind,
            strictness: Some(Strictness::Limit),
            bank: None,
            sung: Vec::new(),
            topic: None,
            note: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn found<'t>(terms: &[Term], text: &'t str) -> Vec<&'t str> {
        let mut stems = Stems::default();
        let matcher = Matcher::new(terms, &mut stems);
        let units: Vec<u16> = text.encode_utf16().collect();
        matcher
            .find(text, &mut stems)
            .into_iter()
            .map(|hit| {
                // Back from UTF-16 units to the slice they name.
                let before = String::from_utf16(&units[..hit.start]).unwrap().len();
                let length = String::from_utf16(&units[hit.start..hit.end])
                    .unwrap()
                    .len();
                &text[before..before + length]
            })
            .collect()
    }

    #[test]
    fn a_word_is_found_in_its_cases_and_whole() {
        let terms = [term("окно", &[], TermKind::Noun)];
        assert_eq!(
            found(&terms, "Окна открыты, в окне свет, у окон никого"),
            ["Окна", "окне"]
        );
        let terms = [term("чай", &[], TermKind::Noun)];
        assert!(found(&terms, "чайка над морем").is_empty());
    }

    #[test]
    fn a_form_reaches_what_the_stem_does_not() {
        let terms = [term("окно", &["окон"], TermKind::Noun)];
        assert_eq!(found(&terms, "у окон никого"), ["окон"]);
    }

    #[test]
    fn a_phrase_is_found_as_a_run_of_words() {
        let terms = [term("на краю", &[], TermKind::Phrase)];
        assert_eq!(
            found(&terms, "Стою на краю. На краю света. Краю на"),
            ["на краю", "На краю"]
        );
    }

    #[test]
    fn a_meaning_is_not_looked_for() {
        let terms = [term("остывший чай", &[], TermKind::Image)];
        assert!(found(&terms, "остывший чай на столе").is_empty());
    }

    #[test]
    fn a_section_name_is_never_a_hit() {
        let terms = [term("chorus", &[], TermKind::Noun)];
        assert_eq!(found(&terms, "[Chorus]\nthe chorus again"), ["chorus"]);
    }

    #[test]
    fn overlapping_forms_of_one_term_give_the_longer_hit() {
        let terms = [term("window", &["window pane"], TermKind::Noun)];
        assert_eq!(
            found(&terms, "a window pane, a window"),
            ["window pane", "window"]
        );
    }

    #[test]
    fn places_count_as_the_window_counts_past_a_wide_character() {
        let terms = [term("окно", &[], TermKind::Noun)];
        assert_eq!(found(&terms, "🌙 окно"), ["окно"]);
    }
}
