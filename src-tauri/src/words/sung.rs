//! A text that is sung: where its stresses fall, and what the public reads
//! (ADR 0053).
//!
//! The owner writes a lyric for a singer that reads letters, not intentions:
//! a homograph is sung whichever way the singer guesses, an unknown name is
//! guessed at, an "е" that is "ё" is sung as "е". So the stress is written
//! into the text, the way the owner always has - a capital vowel ("пульсАр"),
//! or an acute over a first letter that is already a capital ("О́блако") -
//! and a word the singer gets wrong however it is stressed is respelled
//! ("Марсель" sung "МарсЭль"), each respelling kept on the word's record
//! (`Term::sung`).
//!
//! This module reads those marks, checks a sung text against the language
//! pack and the owner's respellings ([`check`]), and takes the marks off for
//! a text the public reads - a description, a copy to paste ([`clean`]).

use serde::Serialize;

use super::pack::{self, Reading};
use super::{Word, plain, words};
use crate::register::Sung;

/// The combining acute: a stress written over a letter.
const ACUTE: char = '\u{0301}';

/// What the check says about one word of a sung text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum StressKind {
    /// A homograph with no mark: the singer may read it either way.
    Homograph,
    /// A word the dictionary does not know, with no mark: the singer guesses.
    Unknown,
    /// A mark on a vowel the dictionary never stresses.
    Against,
    /// "е" where the word is written with "ё".
    Yo,
    /// A word the owner sings another way, written the plain way.
    Unsung,
}

/// One word of a sung text the check has something to say about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct StressNote {
    pub kind: StressKind,
    /// The word as written.
    pub word: String,
    /// Where it stands, in UTF-16 units, as the window counts.
    pub start: usize,
    pub end: usize,
    /// What it could be written as instead, the likeliest first: each reading
    /// of a homograph marked ("зАмок", "замОк"), the dictionary's stress, the
    /// ё spelling, the owner's respelling. Empty for a word nobody knows.
    pub options: Vec<String>,
}

/// What checking a sung text found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct StressCheck {
    pub notes: Vec<StressNote>,
    /// Where the stressed vowel of each word stands, in UTF-16 units: the one
    /// its mark says, or the dictionary's when it has only one. What "show
    /// the stresses" draws an accent over.
    pub accents: Vec<usize>,
}

/// The letters of a word, each with its place in UTF-16 units from the
/// word's start, its marks set apart.
struct Letters {
    /// Each letter, its offset from the word's start, and whether an acute
    /// follows it.
    letters: Vec<(char, usize, bool)>,
}

impl Letters {
    fn of(word: &str) -> Self {
        let mut letters: Vec<(char, usize, bool)> = Vec::new();
        let mut at = 0;
        for letter in word.chars() {
            if super::is_mark(letter) {
                if letter == ACUTE
                    && let Some(last) = letters.last_mut()
                {
                    last.2 = true;
                }
            } else {
                letters.push((letter, at, false));
            }
            at += letter.len_utf16();
        }
        Self { letters }
    }

    /// The vowels, by their place among the letters.
    fn vowels(&self) -> Vec<usize> {
        self.letters
            .iter()
            .enumerate()
            .filter(|(_, (letter, _, _))| pack::is_vowel(*letter))
            .map(|(index, _)| index)
            .collect()
    }
}

/// Which vowel a written word marks as stressed, by its place among the
/// word's vowels: an acute over it; a capital past the first letter, when it
/// is the word's only one (two or more capitals are an abbreviation or a
/// shout, not a mark); or else ё, which is always stressed.
pub fn marked(word: &str) -> Option<usize> {
    let letters = Letters::of(word);
    let vowels = letters.vowels();
    if let Some(place) = marked_by_hand(word) {
        return Some(place);
    }
    vowels
        .iter()
        .position(|&index| matches!(letters.letters[index].0, 'ё' | 'Ё'))
}

/// The stress the owner marked themselves - an acute or a lone capital
/// vowel - and not the ё a word is simply spelled with. A ё can carry a
/// secondary stress in a compound ("трёхсот" is stressed on its last
/// vowel), so it never argues with the dictionary; a hand's mark may.
fn marked_by_hand(word: &str) -> Option<usize> {
    let letters = Letters::of(word);
    let vowels = letters.vowels();
    if let Some(place) = vowels.iter().position(|&index| letters.letters[index].2) {
        return Some(place);
    }
    let capitals: Vec<usize> = letters
        .letters
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, (letter, _, _))| letter.is_uppercase())
        .map(|(index, _)| index)
        .collect();
    if let [only] = capitals.as_slice()
        && let Some(place) = vowels.iter().position(|index| index == only)
    {
        return Some(place);
    }
    None
}

/// Whether a word no dictionary knows is still no stranger to a singer: a
/// sung-out vowel ("любооовь", "а-а-а"), a run of syllables ("ла-ла-ла",
/// "о-у-о"), or a hyphenated word every part of which is known or short
/// ("кем-то", "раз-два-три"). Measured on the owner's lyrics, these were
/// the bulk of what read as unknown.
fn is_familiar(word: &str) -> bool {
    let lower: Vec<char> = word
        .to_lowercase()
        .chars()
        .filter(|c| !super::is_mark(*c))
        .collect();
    if lower
        .windows(3)
        .any(|three| three[0] == three[1] && three[1] == three[2])
    {
        return true;
    }
    if !word.contains('-') {
        return false;
    }
    word.split('-').all(|part| {
        let letters = part.chars().filter(|c| c.is_alphabetic()).count();
        letters <= 3 || pack::reading(part).is_some()
    })
}

/// `word` with its stress on the `vowel`-th vowel, the way the owner writes
/// it: that vowel a capital, every other mark taken off - or, when it is the
/// first letter, an acute over it, the letter's case kept: a capital there
/// already means the start of a line, and "о́блако" mid-line stays small.
pub fn with_stress(word: &str, vowel: usize) -> String {
    let base = unmarked(word);
    let letters = Letters::of(&base);
    let Some(&index) = letters.vowels().get(vowel) else {
        return base;
    };
    let mut out = String::new();
    for (place, (letter, _, _)) in letters.letters.iter().enumerate() {
        if place == index {
            if place == 0 {
                out.push(*letter);
                out.push(ACUTE);
            } else {
                out.extend(letter.to_uppercase());
            }
        } else {
            out.push(*letter);
        }
    }
    out
}

/// `word` with its marks taken off: the acute dropped, and a capital vowel
/// that marks a stress lowered. A first capital, an abbreviation and a shout
/// stay as they are.
fn unmarked(word: &str) -> String {
    let letters = Letters::of(word);
    let capitals: Vec<usize> = letters
        .letters
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, (letter, _, _))| letter.is_uppercase())
        .map(|(index, _)| index)
        .collect();
    let lowered = match capitals.as_slice() {
        [only] if pack::is_vowel(letters.letters[*only].0) => Some(*only),
        _ => None,
    };
    letters
        .letters
        .iter()
        .enumerate()
        .flat_map(|(place, (letter, _, _))| {
            if Some(place) == lowered {
                letter.to_lowercase().collect::<Vec<_>>()
            } else {
                vec![*letter]
            }
        })
        .collect()
}

/// `form` with the case of `like`'s first letter: a respelling found at the
/// start of a line starts with a capital too.
fn cased_like(form: &str, like: &str) -> String {
    let starts_upper = like.chars().next().is_some_and(char::is_uppercase);
    let mut letters = form.chars();
    match letters.next() {
        Some(first) if starts_upper => first.to_uppercase().chain(letters).collect(),
        Some(first) => first.to_lowercase().chain(letters).collect(),
        None => String::new(),
    }
}

/// Where the letter at `place` of a word stands in the text, in UTF-16 units.
fn offset(word: &Word<'_>, letters: &Letters, place: usize) -> usize {
    word.start + letters.letters[place].1
}

/// Check a sung text against the language pack and the owner's respellings.
pub fn check(text: &str, sung: &[Sung]) -> StressCheck {
    let respelled: Vec<String> = sung.iter().map(|one| plain(&one.sung)).collect();
    let mut out = StressCheck::default();
    for word in words(text) {
        if word.bracketed || !pack::is_cyrillic(word.text) {
            continue;
        }
        let key = plain(word.text);
        let letters = Letters::of(word.text);
        let vowels = letters.vowels();
        if vowels.is_empty() {
            continue;
        }
        let note = |kind: StressKind, options: Vec<String>| StressNote {
            kind,
            word: word.text.to_owned(),
            start: word.start,
            end: word.end,
            options,
        };

        // Written the owner's way: what the owner sings, whatever any
        // dictionary says of the letters.
        if respelled.contains(&key) {
            if let Some(place) = marked(word.text) {
                out.accents.push(offset(&word, &letters, vowels[place]));
            }
            continue;
        }
        // Written the plain way where the owner sings it another.
        if let Some(one) = sung.iter().find(|one| plain(&one.written) == key) {
            out.notes.push(note(
                StressKind::Unsung,
                vec![cased_like(&one.sung, word.text)],
            ));
            continue;
        }

        let reading: Option<Reading> = pack::reading(word.text);
        let mark = marked(word.text);

        if let Some(reading) = &reading
            && !reading.yo_optional
        {
            let missing: Vec<usize> = reading
                .yo
                .iter()
                .copied()
                .filter(|&place| {
                    vowels
                        .get(place)
                        .is_some_and(|&index| matches!(letters.letters[index].0, 'е' | 'Е'))
                })
                .collect();
            if !missing.is_empty() {
                let fixed: String = letters
                    .letters
                    .iter()
                    .enumerate()
                    .map(|(index, (letter, _, _))| {
                        let place = vowels.iter().position(|&v| v == index);
                        match (letter, place) {
                            ('е', Some(p)) if missing.contains(&p) => 'ё',
                            ('Е', Some(p)) if missing.contains(&p) => 'Ё',
                            (other, _) => *other,
                        }
                    })
                    .collect();
                out.notes.push(note(StressKind::Yo, vec![fixed]));
            }
        }

        // One vowel has nowhere else to put its stress.
        if vowels.len() < 2 {
            continue;
        }
        match (&reading, mark) {
            (_, Some(place)) => {
                out.accents.push(offset(&word, &letters, vowels[place]));
                if let Some(reading) = &reading
                    && marked_by_hand(word.text) == Some(place)
                    && !reading.stresses.is_empty()
                    && !reading.stresses.contains(&place)
                {
                    let options = reading
                        .stresses
                        .iter()
                        .map(|&stress| with_stress(word.text, stress))
                        .collect();
                    out.notes.push(note(StressKind::Against, options));
                }
            }
            (Some(reading), None) if reading.is_homograph() => {
                let options = reading
                    .stresses
                    .iter()
                    .map(|&stress| with_stress(word.text, stress))
                    .collect();
                out.notes.push(note(StressKind::Homograph, options));
            }
            (Some(reading), None) => {
                if let Some(&stress) = reading.stresses.first()
                    && let Some(&index) = vowels.get(stress)
                {
                    out.accents.push(offset(&word, &letters, index));
                }
            }
            (None, None) if is_familiar(word.text) => {}
            (None, None) => out.notes.push(note(StressKind::Unknown, Vec::new())),
        }
    }
    out.notes.sort_by_key(|note| note.start);
    out.accents.sort_unstable();
    out
}

/// The words of a sung text a singer could get wrong, for `{stress}`: each
/// homograph with its readings, each word the dictionary does not know, each
/// stress marked against it - what an action that marks the stresses is
/// asked about. The ones that need no judgement (ё, the owner's respellings)
/// are left out: the window applies those itself.
pub fn sheet(text: &str, sung: &[Sung]) -> String {
    let checked = check(text, sung);
    let mut lines: Vec<String> = Vec::new();
    for note in &checked.notes {
        let line = match note.kind {
            StressKind::Homograph => format!(
                "- {} - two readings: {}",
                note.word,
                note.options.join(" or ")
            ),
            StressKind::Unknown => format!("- {} - not in the dictionary", note.word),
            StressKind::Against => format!(
                "- {} - marked against the dictionary, which reads {}",
                note.word,
                note.options.join(" or ")
            ),
            StressKind::Yo | StressKind::Unsung => continue,
        };
        if !lines.contains(&line) {
            lines.push(line);
        }
    }
    if lines.is_empty() {
        return "(every word of this text is stressed as the dictionary reads it)".to_owned();
    }
    format!(
        "The words a singer could get wrong, in the order they stand:\n{}",
        lines.join("\n")
    )
}

/// A sung text as the public reads it (ADR 0053): every respelling the owner
/// keeps put back the way the word is written ("МарсЭль" → "Марсель"), every
/// stress mark taken off ("пульсАр" → "пульсар"). Line breaks, punctuation and
/// the words with no marks stay exactly as they are.
pub fn clean(text: &str, sung: &[Sung]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    let base = text.as_ptr() as usize;
    for word in words(text) {
        let start = word.text.as_ptr() as usize - base;
        let end = start + word.text.len();
        out.push_str(&text[at..start]);
        let key = plain(word.text);
        // Marks are Russian: a capital inside "LinkedIn" or "FedEx" is the
        // brand's, not a stress.
        let replaced = match sung.iter().find(|one| plain(&one.sung) == key) {
            Some(one) if !one.written.is_empty() => cased_like(&one.written, word.text),
            _ if pack::is_cyrillic(word.text) => unmarked(word.text),
            _ => word.text.to_owned(),
        };
        out.push_str(&replaced);
        at = end;
    }
    out.push_str(&text[at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str, sung: &[Sung]) -> Vec<(String, StressKind)> {
        check(text, sung)
            .notes
            .into_iter()
            .map(|note| (note.word, note.kind))
            .collect()
    }

    fn marseille() -> Vec<Sung> {
        vec![Sung {
            written: "Марсель".into(),
            sung: "МарсЭль".into(),
        }]
    }

    #[test]
    fn a_mark_is_a_lone_capital_past_the_first_letter_an_acute_or_a_yo() {
        assert_eq!(marked("пульсАр"), Some(1));
        assert_eq!(marked("Окно"), None, "a first capital is a capital");
        assert_eq!(marked("О\u{301}блако"), Some(0));
        assert_eq!(marked("ЗАМОК"), None, "a shout marks nothing");
        assert_eq!(marked("ещё"), Some(1));
        assert_eq!(marked("замок"), None);
    }

    #[test]
    fn a_sung_out_vowel_a_run_of_syllables_and_a_known_compound_are_no_strangers() {
        assert!(
            check("любооовь, ла-ла-ла, кем-то, о-у-о", &[])
                .notes
                .is_empty()
        );
        assert_eq!(
            kinds("кваквакряк-ля", &[]),
            [("кваквакряк-ля".to_owned(), StressKind::Unknown)]
        );
    }

    #[test]
    fn the_sheet_asks_only_about_what_needs_judgement() {
        let said = sheet("замок, еще, кваквакряк, замок", &[]);
        assert!(
            said.contains("- замок - two readings: зАмок or замОк"),
            "{said}"
        );
        assert!(
            said.contains("- кваквакряк - not in the dictionary"),
            "{said}"
        );
        assert!(
            !said.contains("еще"),
            "a missing ё is the window's to apply"
        );
        assert_eq!(said.matches("замок").count(), 1, "each word once: {said}");
        assert!(sheet("окно", &[]).starts_with("(every word"));
    }

    #[test]
    fn a_yo_in_a_compound_does_not_argue_with_the_book() {
        assert!(
            check("трёхсот", &[]).notes.is_empty(),
            "ё spells, it does not mark"
        );
    }

    #[test]
    fn a_stress_is_written_the_owners_way() {
        assert_eq!(with_stress("замок", 0), "зАмок");
        assert_eq!(with_stress("замОк", 0), "зАмок", "one mark at a time");
        assert_eq!(with_stress("Облако", 0), "О\u{301}блако");
        assert_eq!(
            with_stress("облако", 0),
            "о\u{301}блако",
            "mid-line, it stays small"
        );
        assert_eq!(with_stress("окно", 9), "окно");
    }

    #[test]
    fn a_sung_text_is_told_its_homographs_its_strangers_its_yo_and_its_marks_against_the_book() {
        let found = kinds(
            "Старый замок, пУльсар\nеще кваквакряк, мАнтра\n[Припев замок]",
            &[],
        );
        assert_eq!(
            found,
            vec![
                ("замок".to_owned(), StressKind::Homograph),
                ("пУльсар".to_owned(), StressKind::Against),
                ("еще".to_owned(), StressKind::Yo),
                ("кваквакряк".to_owned(), StressKind::Unknown),
            ],
            "мАнтра is stressed as the book says; a section's name is not sung"
        );
        let lock = &check("замок", &[]).notes[0];
        assert_eq!(lock.options, ["зАмок", "замОк"]);
    }

    #[test]
    fn a_word_the_owner_respells_is_found_written_plainly_and_left_alone_respelled() {
        assert_eq!(
            check("Марсель зовёт", &marseille()).notes,
            vec![StressNote {
                kind: StressKind::Unsung,
                word: "Марсель".into(),
                start: 0,
                end: 7,
                options: vec!["МарсЭль".into()],
            }]
        );
        assert!(check("в МарсЭль", &marseille()).notes.is_empty());
    }

    #[test]
    fn the_accents_stand_on_the_marked_vowel_or_the_books_only_one() {
        let text = "окно пульсАр замок";
        let accents = check(text, &[]).accents;
        let units: Vec<u16> = text.encode_utf16().collect();
        let shown: Vec<String> = accents
            .iter()
            .map(|&at| String::from_utf16(&units[at..at + 1]).unwrap())
            .collect();
        assert_eq!(shown, ["о", "А"], "the homograph is shown no stress");
    }

    #[test]
    fn a_public_text_has_the_marks_taken_off_and_the_respellings_put_back() {
        assert_eq!(
            clean(
                "В МарсЭль, где пульсАр\nи NASA, и О\u{301}блако, и iPhone, и LinkedIn",
                &marseille()
            ),
            "В Марсель, где пульсар\nи NASA, и Облако, и iPhone, и LinkedIn"
        );
        assert_eq!(clean("мАрсэль", &marseille()), "марсель");
    }
}
