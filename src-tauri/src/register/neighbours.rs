//! The works whose words stand closest to a text.
//!
//! By shared words, weighed by how rare each is across the works: two songs
//! that both say "window" share little, two that both say "barometer" share a
//! lot. This is not meaning - a text that says the same thing in other words
//! is not found here - and it is not offered as meaning. It is what the
//! assistant's "neighbours in meaning" action is handed to read, so it reads
//! the few works worth comparing rather than all of them (ADR 0044).

use std::collections::HashMap;

use super::matching::{Stems, corpus};
use crate::error::Result;
use crate::words::words;
use rusqlite::Connection;

/// Stems shorter than this say too little to make two texts neighbours.
const MIN_STEM: usize = 3;

/// A work near a text.
#[derive(Debug, Clone, PartialEq)]
pub struct Neighbour {
    pub work_id: String,
    pub title: String,
    pub kind: String,
    /// How close, from 0 to 1.
    pub score: f64,
    /// The words both say that weigh the most, as the text writes them.
    pub shared: Vec<String>,
    /// The work's current text.
    pub body: String,
}

/// The weight of each stem of a text, and the word it first appears as.
struct Bag {
    counts: HashMap<String, usize>,
    first: HashMap<String, String>,
}

fn bag(text: &str, stems: &mut Stems) -> Bag {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut first: HashMap<String, String> = HashMap::new();
    for word in words(text) {
        if word.bracketed || word.is_stop() {
            continue;
        }
        let key = stems.of(word.text);
        if key.chars().count() < MIN_STEM {
            continue;
        }
        *counts.entry(key.clone()).or_default() += 1;
        first.entry(key).or_insert_with(|| word.lower());
    }
    Bag { counts, first }
}

/// The `limit` works of a profile nearest to `text`, nearest first, leaving
/// out `except` - the work the text is of.
pub fn of_text(
    conn: &Connection,
    profile_id: &str,
    except: Option<&str>,
    text: &str,
    limit: usize,
) -> Result<Vec<Neighbour>> {
    let mut stems = Stems::default();
    let query = bag(text, &mut stems);
    if query.counts.is_empty() {
        return Ok(Vec::new());
    }
    let others: Vec<_> = corpus(conn, profile_id)?
        .into_iter()
        .filter(|one| Some(one.work_id.as_str()) != except)
        .map(|one| {
            let bag = bag(&one.body, &mut stems);
            (one, bag)
        })
        .filter(|(_, bag)| !bag.counts.is_empty())
        .collect();

    // How many texts say each stem, the query among them.
    let mut df: HashMap<&str, usize> = HashMap::new();
    for key in query.counts.keys() {
        *df.entry(key).or_default() += 1;
    }
    for (_, bag) in &others {
        for key in bag.counts.keys() {
            *df.entry(key).or_default() += 1;
        }
    }
    let total = (others.len() + 1) as f64;
    let idf =
        |key: &str| ((total + 1.0) / (df.get(key).copied().unwrap_or(0) as f64 + 1.0)).ln() + 1.0;
    // A word said five times is not five times the evidence.
    let weight = |count: usize| 1.0 + (count as f64).ln();

    fn vector<'b>(bag: &'b Bag, weigh: &dyn Fn(&str, usize) -> f64) -> HashMap<&'b str, f64> {
        bag.counts
            .iter()
            .map(|(key, &count)| (key.as_str(), weigh(key, count)))
            .collect()
    }
    let weigh = |key: &str, count: usize| weight(count) * idf(key);
    let vector = |bag| vector(bag, &weigh);
    let norm = |v: &HashMap<&str, f64>| v.values().map(|x| x * x).sum::<f64>().sqrt();

    let q = vector(&query);
    let q_norm = norm(&q);
    let mut out: Vec<Neighbour> = Vec::new();
    for (one, bag) in &others {
        let d = vector(bag);
        let mut shared: Vec<(&str, f64)> = q
            .iter()
            .filter_map(|(key, a)| d.get(key).map(|b| (*key, a * b)))
            .collect();
        if shared.is_empty() {
            continue;
        }
        let dot: f64 = shared.iter().map(|(_, product)| product).sum();
        let score = dot / (q_norm * norm(&d));
        shared.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(b.0)));
        out.push(Neighbour {
            work_id: one.work_id.clone(),
            title: one.title.clone(),
            kind: one.kind.clone(),
            score,
            shared: shared
                .iter()
                .take(5)
                .filter_map(|(key, _)| query.first.get(*key).cloned())
                .collect(),
            body: one.body.clone(),
        });
    }
    out.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.title.cmp(&b.title)));
    out.truncate(limit);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::register::tests::song;

    #[test]
    fn the_nearest_work_shares_the_rarest_words() {
        let (conn, profile_id) = fixtures::workspace();
        let me = song(
            &conn,
            &profile_id,
            "Me",
            "the barometer falls, the window shakes",
        );
        // "window" is in three texts, "barometer" in two: sharing the rarer
        // word makes the nearer neighbour.
        song(&conn, &profile_id, "Common", "a window of light");
        song(&conn, &profile_id, "Also", "the window and the rain");
        song(&conn, &profile_id, "Rare", "my barometer, the cloudberries");
        song(&conn, &profile_id, "Stranger", "nothing here is alike");

        let found = of_text(
            &conn,
            &profile_id,
            Some(&me),
            "the barometer falls, the window shakes",
            5,
        )
        .unwrap();

        let titles: Vec<&str> = found.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(
            titles,
            ["Rare", "Also", "Common"],
            "the stranger shares nothing"
        );
        assert_eq!(found[0].shared, ["barometer"]);
        assert!(
            found.iter().all(|n| n.work_id != me),
            "a work is not its own neighbour"
        );
        assert!(found[0].score > found[1].score);
        assert!(found[0].score <= 1.0 + f64::EPSILON);
    }

    #[test]
    fn a_text_of_function_words_has_no_neighbours() {
        let (conn, profile_id) = fixtures::workspace();
        song(&conn, &profile_id, "Any", "and the of it");
        assert!(
            of_text(&conn, &profile_id, None, "and the of", 5)
                .unwrap()
                .is_empty()
        );
    }
}
