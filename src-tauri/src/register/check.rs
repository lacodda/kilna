//! A text, checked: the words it leans on, what it takes from the register,
//! and - for a text that is sung - where its stresses fall (ADR 0053).
//!
//! One answer for all three, because the window marks them on the same
//! letters and a mark cannot sit inside another: the places come back already
//! cut into runs, each carrying at most one repeated word, at most one term -
//! the strictest, where two terms cover one word - and at most one note on
//! how the word is sung.

use serde::Serialize;

use super::matching::{Matcher, Stems};
use super::{Strictness, TermKind};
use crate::error::Result;
use crate::words::repeats::{self, RepeatGroup};
use crate::words::sung::{self, StressNote};

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
    /// What a sung text says about its stresses and its respellings, in text
    /// order. Empty for a text that is not sung.
    pub stress: Vec<StressNote>,
    /// Where each word's stressed vowel stands, in UTF-16 units: what "show
    /// the stresses" draws. Empty for a text that is not sung.
    pub accents: Vec<usize>,
    /// The phrases of the dictionary a text of a composed role says, in the
    /// order first said (v0.94). Empty for any other text.
    pub phrases: Vec<crate::phrase::PhraseHit>,
    /// The tags it says that the dictionary does not know.
    pub unknown: Vec<crate::phrase::UnknownPhrase>,
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
    /// Index into `stress`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub stress: Option<usize>,
    /// Index into `phrases`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub phrase: Option<usize>,
    /// Index into `unknown`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub unknown: Option<usize>,
}

/// Check `text` against itself and the register of `profile_id`; a text that
/// is `sung` against the language pack and the owner's respellings too.
pub fn text(
    conn: &rusqlite::Connection,
    profile_id: &str,
    text: &str,
    sung: bool,
) -> Result<TextCheck> {
    let terms = super::spent(conn, profile_id)?;
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
                    strictness: term.strictness.unwrap_or_default(),
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

    let stressed = if sung {
        sung::check(text, &super::sung_forms(conn, profile_id)?)
    } else {
        sung::StressCheck::default()
    };
    let notes: Vec<(usize, usize, usize)> = stressed
        .notes
        .iter()
        .enumerate()
        .map(|(index, note)| (note.start, note.end, index))
        .collect();

    let marks = cut(&found.places, &spans, &terms_out, &notes);
    Ok(TextCheck {
        repeats: found.groups,
        terms: terms_out,
        marks,
        stress: stressed.notes,
        accents: stressed.accents,
        phrases: Vec::new(),
        unknown: Vec::new(),
    })
}

/// Check a version's text the way its role reads (v0.94): a role a
/// composition writes is a line of phrases, read against the dictionary -
/// not a lyric, so neither its repeated words nor the register's terms are
/// looked for in it. Any other role is checked as [`text`] checks it.
///
/// A trial of an experiment is read as the text it would become (v0.95): the
/// role its lab keeps trials in names the composition, though the experiment
/// itself does not have the role.
pub fn version_text(
    conn: &rusqlite::Connection,
    profile_id: &str,
    text: &str,
    sung: bool,
    work: &crate::work::Work,
    role: &str,
) -> Result<TextCheck> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let trial = config
        .lab(&work.kind)
        .and_then(|lab| lab.harvest.as_deref())
        .filter(|harvest| *harvest == role)
        .and_then(|_| config.trial_composition(&work.kind));
    let Some(composition) = config.composition_for(&work.kind, role).or(trial) else {
        return self::text(conn, profile_id, text, sung);
    };
    let dictionary = crate::phrase::Dictionary::of(conn, profile_id, composition)?;
    let fields = crate::phrase::Fields::of(
        &config,
        composition,
        &serde_json::Value::Object(work.meta.clone()),
    );
    let reading = crate::phrase::read(text, &dictionary, &fields);
    let mut marks: Vec<TextMark> = reading
        .phrase_places
        .iter()
        .map(|&(start, end, index)| TextMark {
            start,
            end,
            repeat: None,
            term: None,
            stress: None,
            phrase: Some(index),
            unknown: None,
        })
        .chain(
            reading
                .unknown_places
                .iter()
                .map(|&(start, end, index)| TextMark {
                    start,
                    end,
                    repeat: None,
                    term: None,
                    stress: None,
                    phrase: None,
                    unknown: Some(index),
                }),
        )
        .collect();
    marks.sort_by_key(|mark| mark.start);
    Ok(TextCheck {
        phrases: reading.phrases,
        unknown: reading.unknown,
        marks,
        ..TextCheck::default()
    })
}

/// The places of repeated words, of terms and of notes on singing, cut into
/// runs that carry at most one of each. `terms` is what the term indices
/// point into, so the strictest can win a run two terms cover.
fn cut(
    repeats: &[(usize, usize, usize)],
    spans: &[(usize, usize, usize)],
    terms: &[TermHit],
    notes: &[(usize, usize, usize)],
) -> Vec<TextMark> {
    let mut edges: Vec<usize> = repeats
        .iter()
        .flat_map(|&(s, e, _)| [s, e])
        .chain(spans.iter().flat_map(|&(s, e, _)| [s, e]))
        .chain(notes.iter().flat_map(|&(s, e, _)| [s, e]))
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
        // A note is about one word, and words do not overlap.
        let stress = notes
            .iter()
            .find(|&&(s, e, _)| s <= start && end <= e)
            .map(|&(_, _, index)| index);
        if repeat.is_none() && term.is_none() && stress.is_none() {
            continue;
        }
        debug_assert!(term.is_none_or(|index| index < terms.len()));
        match out.last_mut() {
            Some(last)
                if last.end == start
                    && last.repeat == repeat
                    && last.term == term
                    && last.stress == stress =>
            {
                last.end = end;
            }
            _ => out.push(TextMark {
                start,
                end,
                repeat,
                term,
                stress,
                phrase: None,
                unknown: None,
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
            false,
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
        let checked = text(&conn, &profile_id, body, false).unwrap();

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
        let checked = text(&conn, &profile_id, body, false).unwrap();

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
        let checked = text(&conn, &profile_id, "дом дорога дом", false).unwrap();
        assert!(checked.terms.is_empty());
        assert_eq!(checked.repeats.len(), 1);
        assert_eq!(checked.marks.len(), 2);
        assert!(
            checked.stress.is_empty() && checked.accents.is_empty(),
            "not sung"
        );
    }

    /// A sung text is told how it is sung, on the same runs: a homograph that
    /// is also a repeat carries both, and a word the owner respells is
    /// checked against the owner's record.
    #[test]
    fn a_sung_text_carries_its_notes_on_the_same_runs() {
        let (conn, profile_id) = fixtures::workspace();
        register::create(
            &conn,
            &profile_id,
            NewTerm {
                sung: vec![crate::register::Sung {
                    written: "Марсель".into(),
                    sung: "МарсЭль".into(),
                }],
                ..NewTerm::banked("Марсель")
            },
        )
        .unwrap();

        let body = "замок и замок, Марсель";
        let checked = text(&conn, &profile_id, body, true).unwrap();

        let kinds: Vec<_> = checked.stress.iter().map(|note| note.kind).collect();
        assert_eq!(
            kinds,
            [
                crate::words::sung::StressKind::Homograph,
                crate::words::sung::StressKind::Homograph,
                crate::words::sung::StressKind::Unsung
            ]
        );
        let first = &checked.marks[0];
        assert_eq!((first.repeat, first.stress), (Some(0), Some(0)));
        assert!(checked.marks.windows(2).all(|p| p[0].end <= p[1].start));
    }
}
