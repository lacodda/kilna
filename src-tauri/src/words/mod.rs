//! The words of a text: where each one is, and the key it groups by.
//!
//! What the repeat counter, the register and the neighbours all start from. A
//! word is a run of letters, with an apostrophe or a hyphen inside it allowed
//! ("don't", "кто-то"); its place is given in UTF-16 code units, because the
//! window marks it by offsets into a JavaScript string and a Cyrillic letter is
//! two bytes here and one unit there.

pub mod pack;
pub mod repeats;
pub mod stem;
pub mod sung;

pub use stem::stem;

/// One word of a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word<'t> {
    /// As written.
    pub text: &'t str,
    /// Where it starts, in UTF-16 code units.
    pub start: usize,
    /// Where it ends, in UTF-16 code units.
    pub end: usize,
    /// Whether it stands inside square brackets: `[Verse 2]` names a section
    /// and repeats by design, so nothing counts it.
    pub bracketed: bool,
}

impl Word<'_> {
    /// The word lowercased, with `ё` read as `е` and stress marks dropped.
    pub fn lower(&self) -> String {
        plain(self.text)
    }

    /// The key it groups by. See [`stem()`].
    pub fn stem(&self) -> String {
        stem(&self.lower())
    }

    /// Whether it says nothing on its own: "и", "the", "не".
    pub fn is_stop(&self) -> bool {
        is_stop(&self.lower())
    }
}

/// A combining mark: the stress a lyric writes over a vowel ("замо́к").
pub(crate) fn is_mark(letter: char) -> bool {
    matches!(letter, '\u{0300}'..='\u{036F}')
}

fn is_letter(letter: char) -> bool {
    letter.is_alphabetic() || is_mark(letter)
}

/// Lowercased, `ё` as `е`, stress marks dropped.
pub fn plain(word: &str) -> String {
    word.chars()
        .filter(|&letter| !is_mark(letter))
        .flat_map(char::to_lowercase)
        .map(|letter| if letter == 'ё' { 'е' } else { letter })
        .collect()
}

/// Every word of `text`, in order.
pub fn words(text: &str) -> Vec<Word<'_>> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = Vec::new();
    // Where each char starts, in UTF-16 units: the same walk, counted as the
    // window counts.
    let mut units = Vec::with_capacity(chars.len() + 1);
    let mut unit = 0;
    for &(_, letter) in &chars {
        units.push(unit);
        unit += letter.len_utf16();
    }
    units.push(unit);

    // The spans of `[...]` closed on their own line: section labels. A
    // bracket left open is punctuation, not a label that swallows the verse.
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut open: Option<usize> = None;
    for (index, &(_, letter)) in chars.iter().enumerate() {
        match letter {
            '[' => open = Some(index),
            ']' => {
                if let Some(start) = open.take() {
                    spans.push((start, index));
                }
            }
            '\n' => open = None,
            _ => {}
        }
    }
    let inside = |index: usize| {
        spans
            .iter()
            .any(|&(start, end)| index > start && index < end)
    };

    let mut at = 0;
    while at < chars.len() {
        let letter = chars[at].1;
        if !is_letter(letter) {
            at += 1;
            continue;
        }
        let first = at;
        at += 1;
        loop {
            while at < chars.len() && is_letter(chars[at].1) {
                at += 1;
            }
            // An apostrophe or a hyphen joins two runs of letters, and only
            // two: a trailing hyphen is punctuation.
            let joins = at + 1 < chars.len()
                && matches!(chars[at].1, '\'' | '’' | '-')
                && is_letter(chars[at + 1].1);
            if !joins {
                break;
            }
            at += 1;
        }
        let byte_start = chars[first].0;
        let byte_end = chars.get(at).map_or(text.len(), |&(byte, _)| byte);
        out.push(Word {
            text: &text[byte_start..byte_end],
            start: units[first],
            end: units[at],
            bracketed: inside(first),
        });
    }
    out
}

/// Words so common that a text lit up by them would say nothing.
pub fn is_stop(lower: &str) -> bool {
    STOP.binary_search(&lower).is_ok()
}

/// Sorted, for the binary search. The Russian and English function words the
/// repeat counter has always left alone.
const STOP: &[&str] = &{
    let mut words = [
        "и",
        "в",
        "во",
        "не",
        "что",
        "он",
        "на",
        "я",
        "с",
        "со",
        "как",
        "а",
        "то",
        "все",
        "она",
        "так",
        "его",
        "но",
        "да",
        "ты",
        "к",
        "у",
        "же",
        "вы",
        "за",
        "бы",
        "по",
        "только",
        "ее",
        "мне",
        "было",
        "вот",
        "от",
        "меня",
        "еще",
        "нет",
        "о",
        "из",
        "ему",
        "теперь",
        "когда",
        "даже",
        "ну",
        "вдруг",
        "ли",
        "если",
        "уже",
        "или",
        "ни",
        "быть",
        "был",
        "него",
        "до",
        "вас",
        "нибудь",
        "опять",
        "уж",
        "вам",
        "ведь",
        "там",
        "потом",
        "себя",
        "ничего",
        "ей",
        "может",
        "они",
        "тут",
        "где",
        "есть",
        "надо",
        "ней",
        "для",
        "мы",
        "тебя",
        "их",
        "чем",
        "была",
        "сам",
        "чтоб",
        "без",
        "будто",
        "чего",
        "раз",
        "тоже",
        "себе",
        "под",
        "будет",
        "ж",
        "тогда",
        "кто",
        "этот",
        "того",
        "потому",
        "этого",
        "какой",
        "совсем",
        "ним",
        "здесь",
        "этом",
        "один",
        "почти",
        "мой",
        "тем",
        "чтобы",
        "нее",
        "сейчас",
        "были",
        "куда",
        "зачем",
        "всех",
        "никогда",
        "можно",
        "при",
        "наконец",
        "два",
        "об",
        "другой",
        "хоть",
        "после",
        "над",
        "больше",
        "тот",
        "через",
        "эти",
        "нас",
        "про",
        "всего",
        "них",
        "какая",
        "много",
        "разве",
        "три",
        "эту",
        "моя",
        "впрочем",
        "хорошо",
        "свою",
        "этой",
        "перед",
        "иногда",
        "лучше",
        "чуть",
        "том",
        "нельзя",
        "такой",
        "им",
        "более",
        "всегда",
        "конечно",
        "всю",
        "между",
        "это",
        "эта",
        "будут",
        "буду",
        "мою",
        "твой",
        "твоя",
        "твою",
        "свой",
        "своя",
        "the",
        "a",
        "an",
        "and",
        "or",
        "but",
        "of",
        "to",
        "in",
        "on",
        "at",
        "by",
        "for",
        "with",
        "from",
        "as",
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "being",
        "it",
        "its",
        "this",
        "that",
        "these",
        "those",
        "i",
        "you",
        "he",
        "she",
        "we",
        "they",
        "me",
        "him",
        "her",
        "us",
        "them",
        "my",
        "your",
        "his",
        "our",
        "their",
        "not",
        "no",
        "so",
        "if",
        "then",
        "than",
        "too",
        "very",
        "just",
        "do",
        "does",
        "did",
        "have",
        "has",
        "had",
        "will",
        "would",
        "can",
        "could",
        "should",
        "may",
        "might",
        "there",
        "here",
        "what",
        "which",
        "who",
        "whom",
        "when",
        "where",
        "why",
        "how",
        "all",
        "any",
        "each",
        "every",
        "some",
        "more",
        "most",
        "other",
        "into",
        "out",
        "up",
        "down",
        "over",
        "under",
        "again",
        "off",
        "only",
        "own",
        "same",
        "am",
        "oh",
        "yeah",
        "ll",
        "ve",
        "re",
        "свое",
    ];
    // A const sort: the list reads as it was written, and is searched sorted.
    let mut swapped = true;
    while swapped {
        swapped = false;
        let mut at = 1;
        while at < words.len() {
            if less(words[at], words[at - 1]) {
                let before = words[at - 1];
                words[at - 1] = words[at];
                words[at] = before;
                swapped = true;
            }
            at += 1;
        }
    }
    words
};

/// `a < b` as `str` orders them, in a const context.
const fn less(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut at = 0;
    while at < a.len() && at < b.len() {
        if a[at] != b[at] {
            return a[at] < b[at];
        }
        at += 1;
    }
    a.len() < b.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(text: &str) -> Vec<&str> {
        words(text).into_iter().map(|word| word.text).collect()
    }

    #[test]
    fn words_are_runs_of_letters_joined_by_an_apostrophe_or_a_hyphen() {
        assert_eq!(
            texts("Don't stop — кто-то ждёт, rain-soaked- streets"),
            ["Don't", "stop", "кто-то", "ждёт", "rain-soaked", "streets"]
        );
    }

    #[test]
    fn places_are_utf16_units_as_the_window_counts_them() {
        let text = "ночь 🌙 окно";
        let found = words(text);
        assert_eq!(found[0].start, 0);
        assert_eq!(found[0].end, 4);
        // The moon is two units in JavaScript, one char here.
        assert_eq!(found[1].start, 8);
        assert_eq!(found[1].end, 12);
    }

    #[test]
    fn a_stress_mark_stays_in_the_word_and_out_of_its_key() {
        let found = words("замо́к");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].stem(), stem("замок"));
    }

    #[test]
    fn section_labels_are_bracketed_and_an_open_bracket_is_not_a_label() {
        let found = words("[Verse 1] night\n[unclosed\nday]");
        let flags: Vec<(&str, bool)> = found.iter().map(|w| (w.text, w.bracketed)).collect();
        assert_eq!(
            flags,
            [
                ("Verse", true),
                ("night", false),
                ("unclosed", false),
                ("day", false)
            ]
        );
    }

    #[test]
    fn the_stop_list_is_searchable() {
        assert!(STOP.windows(2).all(|pair| pair[0] <= pair[1]), "sorted");
        assert!(is_stop("и"));
        assert!(is_stop("the"));
        assert!(!is_stop("лестница"));
    }
}
