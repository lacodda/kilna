//! A light stemmer for Russian and English words.
//!
//! Enough to say that "лестница", "лестницы" and "лестницей" are one word, and
//! that "ladder" and "ladders" are - which is the whole of what the repeat
//! counter and the register need. Not a lemmatiser: the stem is a key for
//! grouping, never shown as a word, so "лестниц" is a fine answer and
//! "лестница" would cost a dictionary. The Russian side follows the shape of
//! the Snowball algorithm - a region after the first vowel, one suffix class
//! stripped in order - with the derivational step left out; the English side is
//! Porter's first steps.
//!
//! Own rather than a dependency: the available crates cover forty languages
//! this text will never be written in, and the window used to carry a copy of
//! its own (`lib/stem.ts`) that the backend could not share. There is one now,
//! here, and the window asks for what it needs (ADR 0044).

const RU_VOWELS: &[char] = &['а', 'е', 'и', 'о', 'у', 'ы', 'э', 'ю', 'я'];

/// Snowball's RV: everything after the first vowel. Suffixes are only ever
/// taken from inside it, which is what keeps "мыло" from losing its "ло".
///
/// A byte offset: the word is worked on as the `str` it is, so a suffix test
/// is a comparison of bytes rather than a vector of letters per try - the
/// register stems every word of every work, and a hundred allocations a word
/// is what that would cost.
fn rv(word: &str) -> usize {
    word.char_indices()
        .find(|(_, letter)| RU_VOWELS.contains(letter))
        .map_or(word.len(), |(at, letter)| at + letter.len_utf8())
}

/// Takes the first suffix of `suffixes` the word ends with, inside RV.
fn strip<'w>(word: &'w str, from: usize, suffixes: &[&str]) -> Option<&'w str> {
    suffixes
        .iter()
        .find(|suffix| word.len() >= from + suffix.len() && word.ends_with(*suffix))
        .map(|suffix| &word[..word.len() - suffix.len()])
}

/// Suffixes that count only after `а` or `я`: the letter stays.
fn strip_after_a_ya<'w>(word: &'w str, from: usize, suffixes: &[&str]) -> Option<&'w str> {
    // Both letters are two bytes long in UTF-8.
    const A_YA: usize = 2;
    suffixes
        .iter()
        .find(|suffix| {
            word.len() >= from + suffix.len() + A_YA
                && word.ends_with(*suffix)
                && matches!(
                    word[..word.len() - suffix.len()].chars().next_back(),
                    Some('а' | 'я')
                )
        })
        .map(|suffix| &word[..word.len() - suffix.len()])
}

// Longest first within each class, as Snowball lists them: "ившись" must be
// tried before "ив", or the wrong one wins.
const PERFECTIVE_GERUND_AYA: &[&str] = &["вшись", "вши", "в"];
const PERFECTIVE_GERUND: &[&str] = &["ившись", "ывшись", "ивши", "ывши", "ив", "ыв"];
const REFLEXIVE: &[&str] = &["ся", "сь"];
const ADJECTIVE: &[&str] = &[
    "ими", "ыми", "его", "ого", "ему", "ому", "ее", "ие", "ые", "ое", "ей", "ий", "ый", "ой", "ем",
    "им", "ым", "ом", "их", "ых", "ую", "юю", "ая", "яя", "ою", "ею",
];
const PARTICIPLE_AYA: &[&str] = &["ем", "нн", "вш", "ющ", "щ"];
const PARTICIPLE: &[&str] = &["ивш", "ывш", "ующ"];
const VERB_AYA: &[&str] = &[
    "ете", "йте", "ешь", "нно", "ла", "на", "ли", "ем", "ло", "но", "ет", "ют", "ны", "ть", "й",
    "л", "н",
];
const VERB: &[&str] = &[
    "ейте", "уйте", "ила", "ыла", "ена", "ите", "или", "ыли", "ило", "ыло", "ено", "ует", "уют",
    "ены", "ить", "ыть", "ишь", "ей", "уй", "ил", "ыл", "им", "ым", "ен", "ят", "ит", "ыт", "ую",
    "ю",
];
const NOUN: &[&str] = &[
    "иями", "ями", "ами", "ией", "иям", "ием", "иях", "ев", "ов", "ие", "ье", "еи", "ии", "ей",
    "ой", "ий", "ям", "ем", "ам", "ом", "ах", "ях", "ию", "ью", "ия", "ья", "а", "е", "и", "й",
    "о", "у", "ы", "ь", "ю", "я",
];

fn stem_russian(input: &str) -> String {
    let word = input.replace('ё', "е");
    let from = rv(&word);
    let mut out: &str = &word;

    // Step 1: one of the perfective gerund endings; failing that, the
    // reflexive ending and then one adjectival, verbal or noun ending.
    if let Some(gerund) = strip_after_a_ya(out, from, PERFECTIVE_GERUND_AYA)
        .or_else(|| strip(out, from, PERFECTIVE_GERUND))
    {
        out = gerund;
    } else {
        out = strip(out, from, REFLEXIVE).unwrap_or(out);
        if let Some(adjective) = strip(out, from, ADJECTIVE) {
            out = strip_after_a_ya(adjective, from, PARTICIPLE_AYA)
                .or_else(|| strip(adjective, from, PARTICIPLE))
                .unwrap_or(adjective);
        } else if let Some(verb) =
            strip_after_a_ya(out, from, VERB_AYA).or_else(|| strip(out, from, VERB))
        {
            out = verb;
        } else {
            out = strip(out, from, NOUN).unwrap_or(out);
        }
    }

    // Step 2: a trailing и.
    out = strip(out, from, &["и"]).unwrap_or(out);

    // Step 4: a trailing ь; a doubled н; the superlative.
    out = strip(out, from, &["ейше", "ейш"]).unwrap_or(out);
    if out.ends_with("нн") && out.len() - "н".len() >= from {
        out = &out[..out.len() - "н".len()];
    }
    out = strip(out, from, &["ь"]).unwrap_or(out);

    out.to_owned()
}

fn has_vowel(word: &str) -> bool {
    word.chars().any(|letter| "aeiouy".contains(letter))
}

fn stem_english(word: &str) -> String {
    let mut out = word.to_owned();

    // Plurals.
    if out.ends_with("sses") || (out.ends_with("ies") && out.len() > 4) {
        out.truncate(out.len() - 2);
    } else if out.ends_with("ss") || out.ends_with("us") {
        // Stays.
    } else if out.ends_with('s') && out.len() > 3 {
        out.pop();
    }

    // -ed, -ing: only where a vowel remains, so "sing" stays "sing".
    for suffix in ["ing", "ed"] {
        if out.ends_with(suffix) && out.len() - suffix.len() >= 2 {
            let stem = &out[..out.len() - suffix.len()];
            if !has_vowel(stem) {
                continue;
            }
            let bytes = stem.as_bytes();
            let last = bytes[bytes.len() - 1];
            // "hopping" → "hop", but "falling" → "fall" is wrong either way;
            // the undoubling rule Porter uses errs towards short stems, and a
            // short stem groups more, which is the safe direction for a repeat
            // counter.
            out = if stem.len() >= 3 && last == bytes[bytes.len() - 2] && !b"lsz".contains(&last) {
                stem[..stem.len() - 1].to_owned()
            } else if stem.ends_with("at") || stem.ends_with("bl") || stem.ends_with("iz") {
                format!("{stem}e")
            } else {
                stem.to_owned()
            };
            break;
        }
    }

    // Common derivational tails a lyric repeats through.
    for suffix in ["ness", "ment", "ful", "ly", "er"] {
        if out.ends_with(suffix) && out.len() - suffix.len() >= 3 {
            out.truncate(out.len() - suffix.len());
            break;
        }
    }

    // A final y is an i: "happy" and "happier" meet at "happi".
    if out.ends_with('y') && out.len() > 2 {
        out.pop();
        out.push('i');
    }
    // A final e is dropped past three letters: "love" and "loving" meet at "lov".
    if out.ends_with('e') && out.len() > 3 {
        out.pop();
    }

    out
}

/// The grouping key of a word: its light stem, lowercased. Scripts other than
/// Cyrillic and Latin are returned lowercased and whole.
pub fn stem(word: &str) -> String {
    let lower = word.to_lowercase();
    if lower.chars().any(|c| ('а'..='я').contains(&c) || c == 'ё') {
        return stem_russian(&lower);
    }
    if !lower.is_empty()
        && lower
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '\'' || c == '’' || c == '-')
    {
        // "don't" stems as "don": what follows an apostrophe is a clitic.
        let head = lower.split(['\'', '’']).next().unwrap_or_default();
        return stem_english(head);
    }
    lower
}

#[cfg(test)]
mod tests {
    use super::stem;

    // The stemmer is judged on one thing: forms of one word meet, and
    // different words do not. The stems themselves are keys, not words, so the
    // tests say "these agree" rather than "this is the stem".
    fn agree(words: &[&str]) {
        let first = stem(words[0]);
        for word in &words[1..] {
            assert_eq!(stem(word), first, "{} / {word}", words[0]);
        }
    }

    fn differ(a: &str, b: &str) {
        assert_ne!(stem(a), stem(b), "{a} vs {b}");
    }

    #[test]
    fn russian_meets_the_cases_of_a_noun() {
        agree(&[
            "лестница",
            "лестницы",
            "лестнице",
            "лестницу",
            "лестницей",
            "лестниц",
        ]);
        agree(&["окно", "окна", "окну", "окном", "окне"]);
        agree(&["дорога", "дороги", "дорогу", "дорогой", "дорогами"]);
        agree(&["тень", "тени", "тенью"]);
    }

    #[test]
    fn russian_meets_the_forms_of_a_verb() {
        agree(&["читает", "читают", "читаем", "читала", "читать"]);
        agree(&["бежала", "бежали", "бежал", "бежать"]);
        agree(&["строит", "строят", "строил", "строить"]);
        agree(&["собираться", "собираюсь"]);
    }

    #[test]
    fn russian_meets_the_forms_of_an_adjective() {
        agree(&["тёплый", "тёплая", "тёплое", "тёплые", "тёплого", "тёплыми"]);
    }

    #[test]
    fn russian_does_not_merge_different_words() {
        differ("лестница", "лес");
        differ("дорога", "дорого");
        differ("окно", "око");
        differ("чай", "чайка");
    }

    #[test]
    fn russian_treats_yo_as_ye() {
        agree(&["ещё", "еще"]);
    }

    #[test]
    fn english_meets_plurals_and_verb_forms() {
        agree(&["ladder", "ladders"]);
        agree(&["love", "loves", "loved", "loving"]);
        agree(&["hope", "hopes", "hoped", "hoping"]);
        agree(&["city", "cities"]);
    }

    #[test]
    fn english_keeps_short_words_whole() {
        assert_eq!(stem("sing"), "sing");
        assert_eq!(stem("is"), "is");
    }

    #[test]
    fn english_does_not_merge_different_words() {
        differ("sing", "sin");
        differ("night", "nine");
    }

    #[test]
    fn it_lowercases_and_leaves_other_scripts_whole() {
        assert_eq!(stem("Лестница"), stem("лестница"));
        assert_eq!(stem("東京"), "東京");
    }
}
