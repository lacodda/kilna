//! A text, checked: the words it leans on, and what it takes from the register.
//!
//! One answer for both, because the window marks both on the same letters and
//! a mark cannot sit inside another: the places come back already cut into
//! runs, each carrying at most one repeated word and at most one term - the
//! strictest, where two terms cover one word.

use serde::Serialize;

use super::matching::{Matcher, Stems};
use super::{Strictness, TermKind};
use crate::error::Result;
use crate::words::repeats::{self, RepeatGroup};

/// What checking a text found.
#[derive(Debug, Clone, Default, PartialEq, Serialize, ts_rs::TS)]
pub struct TextCheck {
    /// The words the text leans on, most repeated first.
    pub repeats: Vec<RepeatGroup>,
    /// The register's terms the text takes, strictest first, then the most
    /// used in it.
    pub terms: Vec<TermHit>,
    /// Where to mark, in text order, never overlapping.
    pub marks: Vec<TextMark>,
}

/// A term of the register a text takes.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct TermHit {
    pub term_id: String,
    pub word: String,
    pub kind: TermKind,
    pub strictness: Strictness,
    /// How many times this text says it.
    pub count: usize,
}

/// A run of the text to mark, in UTF-16 units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct TextMark {
    pub start: usize,
    pub end: usize,
    /// Index into `repeats`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub repeat: Option<usize>,
    /// Index into `terms`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub term: Option<usize>,
}

/// Check `text` against itself and the register of `profile_id`.
pub fn text(conn: &rusqlite::Connection, profile_id: &str, text: &str) -> Result<TextCheck> {
    let terms = super::list(conn, profile_id)?;
    let mut stems = Stems::default();
    let hits = Matcher::new(&terms, &mut stems).find(text, &mut stems);
    let found = repeats::find(text);

    // The terms taken, each once, strictest first.
    let mut taken: Vec<TermHit> = Vec::new();
    let mut slot_of: Vec<Option<usize>> = vec![None; terms.len()];
    for hit in &hits {
        match slot_of[hit.term] {
            Some(slot) => taken[slot].count += 1,
            None => {
                let term = &terms[hit.term];
                slot_of[hit.term] = Some(taken.len());
                taken.push(TermHit {
                    term_id: term.id.clone(),
                    word: term.word.clone(),
                    kind: term.kind,
                    strictness: term.strictness,
                    count: 1,
                });
            }
        }
    }
    let mut order: Vec<usize> = (0..taken.len()).collect();
    order.sort_by(|&a, &b| {
        taken[a]
            .strictness
            .cmp(&taken[b].strictness)
            .then(taken[b].count.cmp(&taken[a].count))
    });
    let mut position = vec![0; taken.len()];
    for (new, &old) in order.iter().enumerate() {
        position[old] = new;
    }
    let spans: Vec<(usize, usize, usize)> = hits
        .iter()
        .filter_map(|hit| slot_of[hit.term].map(|slot| (hit.start, hit.end, position[slot])))
        .collect();
    let mut sorted: Vec<Option<TermHit>> = taken.into_iter().map(Some).collect();
    let terms_out: Vec<TermHit> = order.iter().filter_map(|&old| sorted[old].take()).collect();

    let marks = cut(&found.places, &spans, &terms_out);
    Ok(TextCheck {
        repeats: found.groups,
        terms: terms_out,
        marks,
    })
}

/// The places of repeated words and of terms, cut into runs that carry at
/// most one of each. `terms` is what the term indices point into, so the
/// strictest can win a run two terms cover.
fn cut(
    repeats: &[(usize, usize, usize)],
    spans: &[(usize, usize, usize)],
    terms: &[TermHit],
) -> Vec<TextMark> {
    let mut edges: Vec<usize> = repeats
        .iter()
        .flat_map(|&(s, e, _)| [s, e])
        .chain(spans.iter().flat_map(|&(s, e, _)| [s, e]))
        .collect();
    edges.sort_unstable();
    edges.dedup();

    let mut out: Vec<TextMark> = Vec::new();
    for pair in edges.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        let repeat = repeats
            .iter()
            .find(|&&(s, e, _)| s <= start && end <= e)
            .map(|&(_, _, group)| group);
        let term = spans
            .iter()
            .filter(|&&(s, e, _)| s <= start && end <= e)
            .map(|&(_, _, index)| index)
            // The terms are strictest first, so the lowest index is the
            // strictest of those covering the run.
            .min();
        if repeat.is_none() && term.is_none() {
            continue;
        }
        debug_assert!(term.is_none_or(|index| index < terms.len()));
        match out.last_mut() {
            Some(last) if last.end == start && last.repeat == repeat && last.term == term => {
                last.end = end;
            }
            _ => out.push(TextMark {
                start,
                end,
                repeat,
                term,
            }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::register::{self, NewTerm};

    fn add(conn: &rusqlite::Connection, profile_id: &str, word: &str, strictness: Strictness) {
        register::create(
            conn,
            profile_id,
            NewTerm {
                word: word.into(),
                strictness: Some(strictness),
                ..NewTerm::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn a_text_reports_its_repeats_and_the_terms_it_takes() {
        let (conn, profile_id) = fixtures::workspace();
        add(&conn, &profile_id, "shadow", Strictness::Limit);
        add(&conn, &profile_id, "dust", Strictness::Ban);
        add(&conn, &profile_id, "ember", Strictness::Rare);

        let checked = text(
            &conn,
            &profile_id,
            "shadow and dust, shadow and dust, shadows",
        )
        .unwrap();

        let terms: Vec<(&str, usize)> = checked
            .terms
            .iter()
            .map(|t| (t.word.as_str(), t.count))
            .collect();
        assert_eq!(terms, [("dust", 2), ("shadow", 3)], "strictest first");
        let words: Vec<&str> = checked.repeats.iter().map(|g| g.word.as_str()).collect();
        assert_eq!(words, ["shadow", "dust"]);
    }

    #[test]
    fn marks_never_overlap_and_carry_both_a_repeat_and_a_term() {
        let (conn, profile_id) = fixtures::workspace();
        add(&conn, &profile_id, "white noise", Strictness::Ban);

        let body = "white noise, noise again";
        let checked = text(&conn, &profile_id, body).unwrap();

        // "white " is the phrase alone, "noise" is the phrase and a repeat,
        // the second "noise" a repeat alone.
        let runs: Vec<(&str, Option<usize>, Option<usize>)> = checked
            .marks
            .iter()
            .map(|m| (&body[m.start..m.end], m.repeat, m.term))
            .collect();
        assert_eq!(
            runs,
            [
                ("white ", None, Some(0)),
                ("noise", Some(0), Some(0)),
                ("noise", Some(0), None),
            ]
        );
        assert!(checked.marks.windows(2).all(|p| p[0].end <= p[1].start));
    }

    #[test]
    fn where_two_terms_cover_a_word_the_stricter_marks_it() {
        let (conn, profile_id) = fixtures::workspace();
        add(&conn, &profile_id, "edge", Strictness::Rare);
        add(&conn, &profile_id, "on the edge", Strictness::Ban);

        let body = "on the edge";
        let checked = text(&conn, &profile_id, body).unwrap();

        let ban = checked
            .terms
            .iter()
            .position(|t| t.strictness == Strictness::Ban)
            .unwrap();
        let last = checked.marks.last().unwrap();
        assert_eq!(&body[last.start..last.end], "on the edge");
        assert_eq!(last.term, Some(ban));
    }

    #[test]
    fn an_empty_register_still_counts_the_repeats() {
        let (conn, profile_id) = fixtures::workspace();
        let checked = text(&conn, &profile_id, "дом дорога дом").unwrap();
        assert!(checked.terms.is_empty());
        assert_eq!(checked.repeats.len(), 1);
        assert_eq!(checked.marks.len(), 2);
    }
}
