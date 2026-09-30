//! Words a text uses more than once, and where.
//!
//! Written into a song, "лестница" three times in different cases is one image
//! leaned on three times, and the writer would rather know it before a listener
//! does. The counter groups the words of a text by their light stem, keeps the
//! groups that occur twice or more, and hands back the places so the editor can
//! mark them.
//!
//! Function words are left out: "и", "the" and "не" repeat in every text ever
//! written and a page lit up by them would say nothing. So are stems shorter
//! than three letters, and anything inside square brackets - `[Verse 2]` names a
//! section and repeats by design.

use std::collections::HashMap;

use serde::Serialize;

use super::words;

/// Stems shorter than this are pronouns, particles and noise.
const MIN_STEM: usize = 3;

/// One word the text leans on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct RepeatGroup {
    /// The grouping key. Not for showing.
    pub stem: String,
    /// The word as it first appears, for showing.
    pub word: String,
    pub count: usize,
}

/// The groups, most repeated first, and each place in text order with the
/// index of its group.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Repeats {
    pub groups: Vec<RepeatGroup>,
    /// `(start, end, group)`, in UTF-16 units.
    pub places: Vec<(usize, usize, usize)>,
}

/// A word met in the text: its key, how it first appeared, and every place.
struct Seen {
    stem: String,
    word: String,
    places: Vec<(usize, usize)>,
}

pub fn find(text: &str) -> Repeats {
    // First appearance order is kept by the vector; the map finds the entry.
    let mut order: Vec<Seen> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();

    for word in words(text) {
        if word.bracketed || word.is_stop() {
            continue;
        }
        let key = word.stem();
        if key.chars().count() < MIN_STEM {
            continue;
        }
        let at = *index.entry(key.clone()).or_insert_with(|| {
            order.push(Seen {
                stem: key,
                word: word.text.to_owned(),
                places: Vec::new(),
            });
            order.len() - 1
        });
        order[at].places.push((word.start, word.end));
    }

    let mut repeated: Vec<Seen> = order
        .into_iter()
        .filter(|seen| seen.places.len() > 1)
        .collect();
    // Most repeated first; equal counts in order of first appearance, which
    // a stable sort keeps.
    repeated.sort_by_key(|seen| std::cmp::Reverse(seen.places.len()));

    let mut places: Vec<(usize, usize, usize)> = repeated
        .iter()
        .enumerate()
        .flat_map(|(group, seen)| seen.places.iter().map(move |&(s, e)| (s, e, group)))
        .collect();
    places.sort_unstable();

    Repeats {
        groups: repeated
            .into_iter()
            .map(|seen| RepeatGroup {
                count: seen.places.len(),
                stem: seen.stem,
                word: seen.word,
            })
            .collect(),
        places,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words_of(repeats: &Repeats) -> Vec<(&str, usize)> {
        repeats
            .groups
            .iter()
            .map(|g| (g.word.as_str(), g.count))
            .collect()
    }

    #[test]
    fn a_word_is_counted_across_its_forms() {
        let found = find("Лестница вела вверх.\nПо лестнице шёл дождь.\nЛестницей к небу.");
        assert_eq!(words_of(&found), [("Лестница", 3)]);
        assert!(found.places.iter().all(|&(_, _, group)| group == 0));
    }

    #[test]
    fn the_places_are_exact_and_in_text_order() {
        let text = "rain on the roof, rain in the street";
        let found = find(text);
        let second = text.rfind("rain").unwrap();
        assert_eq!(found.places, [(0, 4, 0), (second, second + 4, 0)]);
    }

    #[test]
    fn function_words_are_left_alone() {
        let found = find("и снова, и снова, и снова — the end and the end");
        let words: Vec<&str> = found.groups.iter().map(|g| g.word.as_str()).collect();
        assert_eq!(words, ["снова", "end"]);
    }

    #[test]
    fn section_labels_are_left_alone() {
        let found = find("[Verse 1]\nverse of the night\n[Verse 2]\nanother night");
        let words: Vec<&str> = found.groups.iter().map(|g| g.word.as_str()).collect();
        assert_eq!(words, ["night"]);
    }

    #[test]
    fn groups_go_most_repeated_first() {
        let found = find("дом дорога дом дорога дом");
        assert_eq!(words_of(&found), [("дом", 3), ("дорога", 2)]);
    }

    #[test]
    fn a_text_that_repeats_nothing_has_no_groups() {
        assert_eq!(find("каждое слово здесь единственное"), Repeats::default());
        assert_eq!(find(""), Repeats::default());
    }
}
