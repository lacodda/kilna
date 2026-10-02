//! The language pack: what kilna knows about a language's words without
//! asking anyone (ADR 0053).
//!
//! Two dictionaries for Russian, built once from open data by `cargo run
//! --example lang_pack` and compiled into the binary from `lang/ru/`:
//!
//! - **stress** - for every word form, the vowels it may be stressed on (two
//!   or more is a homograph: "зАмок" and "замОк"), the vowels it is written
//!   with ё, and whether the е spelling is a word too ("все" beside "всё").
//!   From the ruaccent dictionaries (MIT).
//! - **frequency** - the rank of every stem of the language by how often its
//!   forms are met, 1 the commonest. From Koziev's word-form frequencies
//!   (CC0), summed by kilna's own stemmer, so a rank answers for the same
//!   key every other count in kilna groups by (ADR 0044).
//!
//! Both are finite-state maps (`fst`): a lookup is read where the bytes lie,
//! with nothing loaded and nothing copied, so the window's check of a text
//! as it is typed costs a few microseconds a word. The stress map is keyed
//! by the form as [`super::plain`] writes it - lowercase, ё read as е, marks
//! dropped - so a word typed without its ё, or with a capital vowel for the
//! singer, still finds itself.
//!
//! Whether a word is Russian is said by its letters: a Cyrillic word is
//! looked up, any other is not - a language setting would be a choice the
//! text already makes.

use std::sync::OnceLock;

use fst::Map;

/// The stress of every Russian word form the pack knows.
static STRESS_RU: &[u8] = include_bytes!("../../lang/ru/stress.fst");
/// The rank of every Russian stem by frequency.
static FREQUENCY_RU: &[u8] = include_bytes!("../../lang/ru/frequency.fst");

/// The Russian vowels, ё among them: the letters a stress can fall on.
pub const RU_VOWELS: [char; 10] = ['а', 'е', 'ё', 'и', 'о', 'у', 'ы', 'э', 'ю', 'я'];

/// Whether a letter, in either case, is a Russian vowel.
pub fn is_vowel(letter: char) -> bool {
    letter
        .to_lowercase()
        .next()
        .is_some_and(|lower| RU_VOWELS.contains(&lower))
}

/// Whether a word is written in Cyrillic: the words a Russian pack answers
/// for.
pub fn is_cyrillic(word: &str) -> bool {
    let mut letters = word
        .chars()
        .filter(|letter| letter.is_alphabetic())
        .peekable();
    letters.peek().is_some()
        && letters.all(|letter| matches!(letter, 'а'..='я' | 'А'..='Я' | 'ё' | 'Ё'))
}

/// How the dictionary reads a word form.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reading {
    /// The vowels it may be stressed on, by their place among the word's
    /// vowels from 0. Two or more is a homograph; none is a word the
    /// dictionary knows but cannot stress.
    pub stresses: Vec<usize>,
    /// The vowels its spelling writes ё, by the same count.
    pub yo: Vec<usize>,
    /// Whether the spelling with е is a word as well - "все" and "всё": a
    /// text that writes е is not wrong.
    pub yo_optional: bool,
}

impl Reading {
    pub fn is_homograph(&self) -> bool {
        self.stresses.len() > 1
    }
}

const STRESS_BITS: u32 = 6;
const STRESS_SLOTS: u32 = 3;
const YO_SHIFT: u32 = STRESS_BITS * STRESS_SLOTS;
const YO_BITS: u32 = 32;
const YO_OPTIONAL: u64 = 1 << (YO_SHIFT + YO_BITS);

/// A reading as the stress map stores it. Up to three stresses, each one
/// more than its vowel's place so 0 says "none"; the vowels written ё as a
/// mask of the first 32; and a bit for an optional ё. What does not fit -
/// a fourth stress, a ё past the 32nd vowel - is not stored: no Russian word
/// has either.
pub fn encode(reading: &Reading) -> u64 {
    let mut value = 0u64;
    for (slot, stress) in reading
        .stresses
        .iter()
        .take(STRESS_SLOTS as usize)
        .enumerate()
    {
        let stored = u64::try_from(*stress + 1).unwrap_or(0) & ((1 << STRESS_BITS) - 1);
        value |= stored << (STRESS_BITS * u32::try_from(slot).unwrap_or(0));
    }
    for vowel in &reading.yo {
        if let Ok(place) = u32::try_from(*vowel)
            && place < YO_BITS
        {
            value |= 1 << (YO_SHIFT + place);
        }
    }
    if reading.yo_optional {
        value |= YO_OPTIONAL;
    }
    value
}

/// The reading a stored value says.
pub fn decode(value: u64) -> Reading {
    let mut stresses = Vec::new();
    for slot in 0..STRESS_SLOTS {
        let stored = (value >> (STRESS_BITS * slot)) & ((1 << STRESS_BITS) - 1);
        if stored > 0 {
            stresses.push(usize::try_from(stored - 1).unwrap_or(0));
        }
    }
    let yo = (0..YO_BITS)
        .filter(|place| value & (1 << (YO_SHIFT + place)) != 0)
        .map(|place| place as usize)
        .collect();
    Reading {
        stresses,
        yo,
        yo_optional: value & YO_OPTIONAL != 0,
    }
}

fn stress_map() -> Option<&'static Map<&'static [u8]>> {
    static MAP: OnceLock<Option<Map<&'static [u8]>>> = OnceLock::new();
    MAP.get_or_init(|| match Map::new(STRESS_RU) {
        Ok(map) => Some(map),
        Err(cause) => {
            crate::log::warn(
                "words",
                &format!("the stress dictionary does not read: {cause}"),
            );
            None
        }
    })
    .as_ref()
}

fn frequency_map() -> Option<&'static Map<&'static [u8]>> {
    static MAP: OnceLock<Option<Map<&'static [u8]>>> = OnceLock::new();
    MAP.get_or_init(|| match Map::new(FREQUENCY_RU) {
        Ok(map) => Some(map),
        Err(cause) => {
            crate::log::warn(
                "words",
                &format!("the frequency list does not read: {cause}"),
            );
            None
        }
    })
    .as_ref()
}

/// How the dictionary reads `word`, written any way - with capitals for the
/// singer, without its ё. None for a word it does not know, and for a word
/// that is not Cyrillic.
pub fn reading(word: &str) -> Option<Reading> {
    if !is_cyrillic(word) {
        return None;
    }
    stress_map()?.get(super::plain(word)).map(decode)
}

/// How common a stem of the language is: its rank, 1 the commonest. None for
/// a stem the list does not hold - rarer than everything it does.
pub fn rank(stem: &str) -> Option<u64> {
    frequency_map()?.get(stem)
}

/// The vowels of a word, each with its place among the word's letters (in
/// chars, from 0): what a stress index counts.
pub fn vowels(word: &str) -> Vec<(usize, char)> {
    word.chars()
        .enumerate()
        .filter(|(_, letter)| is_vowel(*letter))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reading_survives_its_encoding() {
        for reading in [
            Reading::default(),
            Reading {
                stresses: vec![0, 1],
                yo: vec![],
                yo_optional: false,
            },
            Reading {
                stresses: vec![2],
                yo: vec![2, 31],
                yo_optional: true,
            },
        ] {
            assert_eq!(decode(encode(&reading)), reading);
        }
    }

    #[test]
    fn the_pack_knows_a_homograph_a_yo_and_where_a_stress_falls() {
        let lock = reading("замок").expect("the pack knows the word");
        assert!(lock.is_homograph(), "{lock:?}");
        assert_eq!(reading("ЗАМОК"), Some(lock.clone()), "whatever its case");

        let pulsar = reading("пульсар").unwrap();
        assert_eq!(pulsar.stresses, [1], "пульсАр");

        let still = reading("еще").unwrap();
        assert_eq!(still.yo, [1], "written ещё");
        assert!(!still.yo_optional);
        assert!(
            reading("все").unwrap().yo_optional,
            "все is a word as much as всё"
        );

        assert_eq!(reading("window"), None);
        assert_eq!(reading("кваквакряк"), None);
    }

    #[test]
    fn a_common_stem_ranks_above_a_rare_one() {
        let common = rank(&crate::words::stem("окно")).expect("a common word is listed");
        let rare = rank(&crate::words::stem("пульсар")).unwrap_or(u64::MAX);
        assert!(common < rare, "{common} < {rare}");
        assert!(common < 5_000);
    }

    #[test]
    fn a_word_is_russian_by_its_letters() {
        assert!(is_cyrillic("Ёлка"));
        assert!(is_cyrillic("кто-то"));
        assert!(!is_cyrillic("Vega"));
        assert!(!is_cyrillic("Вега-7b"));
        assert!(is_vowel('Ё') && is_vowel('ы') && !is_vowel('й'));
    }
}
