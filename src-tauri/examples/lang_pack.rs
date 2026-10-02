//! Builds the Russian language pack in `lang/ru/` from open data (ADR 0053).
//!
//!   cargo run --release --example lang_pack -- <sources> <out>
//!
//! `<sources>` holds the files as published:
//!
//! - `accents.json`, `omographs.json`, `yo_words.json`, `yo_homographs.json`:
//!   the dictionaries of ruaccent (MIT), from
//!   <https://huggingface.co/ruaccent/accentuator/tree/main/dictionary>,
//!   unpacked from their `.json.gz`;
//! - `term2freq.dat` - Koziev's word-form frequencies (CC0), from
//!   <https://github.com/Koziev/NLP_Datasets/tree/master/WordformFrequencies>,
//!   unpacked from `term2freq.7z`.
//!
//! It writes `stress.fst` and `frequency.fst`. Rebuild the frequency list
//! whenever the stemmer changes: its keys are the stemmer's.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use fst::MapBuilder;
use kilna_lib::words::pack::{self, Reading};
use kilna_lib::words::{plain, stem};

/// How many stems the frequency list keeps: past the hundred thousandth, a
/// stem is rare whatever the threshold.
const RANKS_KEPT: usize = 100_000;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, sources, out] = args.as_slice() else {
        eprintln!("usage: lang_pack <sources> <out>");
        std::process::exit(2);
    };
    let sources = Path::new(sources);
    let out = Path::new(out);
    fs::create_dir_all(out).expect("the output directory can be made");

    let stress = stress(sources);
    let mut builder = MapBuilder::new(Vec::new()).expect("a builder");
    for (key, reading) in &stress {
        builder
            .insert(key, pack::encode(reading))
            .expect("keys come sorted");
    }
    let bytes = builder.into_inner().expect("the map is written");
    fs::write(out.join("stress.fst"), &bytes).expect("stress.fst is written");
    println!("stress.fst: {} forms, {} bytes", stress.len(), bytes.len());

    let ranks = frequency(sources);
    let mut builder = MapBuilder::new(Vec::new()).expect("a builder");
    for (key, rank) in &ranks {
        builder.insert(key, *rank).expect("keys come sorted");
    }
    let bytes = builder.into_inner().expect("the map is written");
    fs::write(out.join("frequency.fst"), &bytes).expect("frequency.fst is written");
    println!(
        "frequency.fst: {} stems, {} bytes",
        ranks.len(),
        bytes.len()
    );
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|cause| panic!("{} does not read: {cause}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|cause| panic!("{} is not the JSON expected: {cause}", path.display()))
}

/// Where a stressed spelling ("зам+ок") puts its stress, by the place of the
/// vowel among the word's vowels; and which vowels it writes ё.
fn marks_of(stressed: &str) -> (Option<usize>, Vec<usize>) {
    let mut stress = None;
    let mut yo = Vec::new();
    let mut vowel = 0;
    let mut next_is_stressed = false;
    for letter in stressed.chars() {
        if letter == '+' {
            next_is_stressed = true;
            continue;
        }
        if pack::is_vowel(letter) {
            if next_is_stressed {
                stress = Some(vowel);
            }
            if letter.to_lowercase().next() == Some('ё') {
                yo.push(vowel);
            }
            vowel += 1;
        }
        next_is_stressed = false;
    }
    (stress, yo)
}

fn add_stress(reading: &mut Reading, stress: Option<usize>) {
    if let Some(stress) = stress
        && !reading.stresses.contains(&stress)
    {
        reading.stresses.push(stress);
    }
}

/// Every form, keyed as `plain` writes it, with every stress any source gives
/// it and the vowels its spelling writes ё.
fn stress(sources: &Path) -> BTreeMap<String, Reading> {
    let accents: HashMap<String, String> = read_json(&sources.join("accents.json"));
    let homographs: HashMap<String, Vec<String>> = read_json(&sources.join("omographs.json"));
    let yo_words: HashMap<String, String> = read_json(&sources.join("yo_words.json"));
    let yo_homographs: HashMap<String, String> = read_json(&sources.join("yo_homographs.json"));

    let mut out: BTreeMap<String, Reading> = BTreeMap::new();
    for (form, stressed) in &accents {
        // Affixes ("-де") and forms with no letters are not words of a text.
        if form.starts_with('-') || !form.chars().any(char::is_alphabetic) {
            continue;
        }
        let key = plain(form);
        let (stress, yo) = marks_of(stressed);
        let reading = out.entry(key).or_default();
        add_stress(reading, stress);
        for vowel in yo {
            if !reading.yo.contains(&vowel) {
                reading.yo.push(vowel);
            }
        }
    }
    for (form, variants) in &homographs {
        let reading = out.entry(plain(form)).or_default();
        for variant in variants {
            add_stress(reading, marks_of(variant).0);
        }
    }
    for (plainly, with_yo) in &yo_words {
        let reading = out.entry(plain(plainly)).or_default();
        for vowel in marks_of(with_yo).1 {
            if !reading.yo.contains(&vowel) {
                reading.yo.push(vowel);
            }
        }
    }
    // Only the curated list says the е spelling is a word too: the stress
    // dictionary holds "еж" beside "ёж" for a text typed without ё, and that
    // is not "еж" being a word.
    for plainly in yo_homographs.keys() {
        out.entry(plain(plainly)).or_default().yo_optional = true;
    }
    for reading in out.values_mut() {
        reading.stresses.sort_unstable();
        reading.yo.sort_unstable();
    }
    out
}

/// Every stem of the language by rank, 1 the commonest: each word form's
/// count summed under the stem kilna groups it by.
fn frequency(sources: &Path) -> BTreeMap<String, u64> {
    let file = fs::File::open(sources.join("term2freq.dat")).expect("term2freq.dat opens");
    let mut counts: HashMap<String, u64> = HashMap::new();
    for line in BufReader::new(file).lines() {
        let line = line.expect("a line reads");
        let mut parts = line.split('\t');
        let (Some(form), Some(_), Some(count)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        if !pack::is_cyrillic(form) {
            continue;
        }
        let Ok(count) = count.trim().parse::<u64>() else {
            continue;
        };
        *counts.entry(stem(&plain(form))).or_default() += count;
    }
    let mut ranked: Vec<(String, u64)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked
        .into_iter()
        .take(RANKS_KEPT)
        .enumerate()
        .map(|(index, (key, _))| (key, index as u64 + 1))
        .collect()
}
