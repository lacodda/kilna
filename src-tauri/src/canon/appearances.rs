//! Where a card appears - counted from the works, never written down.
//!
//! A list typed by hand is out of date the day after a new song; this one is
//! read off what the works already say. A card appears in a work that it is
//! the hero of, in whose scenes it stands, whose text names it by one of its
//! names, whose cover prompt names it, and which its facts were read from or
//! hold for.

use std::collections::BTreeMap;

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::error::Result;
use crate::note::Note;

/// One work a card appears in, and how.
#[derive(Debug, Clone, Default, PartialEq, Serialize, ts_rs::TS)]
pub struct Appearance {
    pub work_id: String,
    pub title: String,
    pub kind: String,
    /// The card lives at this work: the hero of this one song.
    pub hero: bool,
    /// The numbers of the work's scenes the card stands in.
    pub scenes: Vec<i64>,
    /// Whether a version of the work names the card by one of its names.
    pub named: bool,
    /// Whether the work's cover prompt names it.
    pub cover: bool,
    /// How many of the card's facts were read from this work, or hold for it
    /// alone.
    pub facts: usize,
}

/// Every work `card` appears in: its own work first, then by title.
pub fn of_card(conn: &Connection, card: &Note) -> Result<Vec<Appearance>> {
    let mut found: BTreeMap<String, Appearance> = BTreeMap::new();
    fn entry<'m>(found: &'m mut BTreeMap<String, Appearance>, work_id: &str) -> &'m mut Appearance {
        found
            .entry(work_id.to_owned())
            .or_insert_with(|| Appearance {
                work_id: work_id.to_owned(),
                ..Appearance::default()
            })
    }

    if let Some(work_id) = card.work_id.as_deref() {
        entry(&mut found, work_id).hero = true;
    }

    let mut statement = conn.prepare(
        "SELECT s.work_id, s.position FROM scene_note sn JOIN scene s ON s.id = sn.scene_id
          WHERE sn.note_id = ?1 ORDER BY s.work_id, s.position",
    )?;
    let scenes = statement
        .query_map(params![card.id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (work_id, position) in scenes {
        entry(&mut found, &work_id).scenes.push(position);
    }

    let mut statement = conn.prepare(
        "SELECT w, count(*) FROM (
             SELECT source_work_id AS w FROM canon_fact WHERE note_id = ?1 AND source_work_id IS NOT NULL
             UNION ALL
             SELECT scope_work_id AS w FROM canon_fact WHERE note_id = ?1 AND scope_work_id IS NOT NULL
         ) GROUP BY w",
    )?;
    let cited = statement
        .query_map(params![card.id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (work_id, count) in cited {
        entry(&mut found, &work_id).facts = usize::try_from(count).unwrap_or(0);
    }

    let names = names_of(card);
    for work_id in works_naming(conn, &card.profile_id, &names)? {
        entry(&mut found, &work_id).named = true;
    }
    for work_id in covers_naming(conn, &card.profile_id, &card.id, &names)? {
        entry(&mut found, &work_id).cover = true;
    }

    // Dressed with the work as it stands. A work gone to the trash is not a
    // place the card appears any more, whatever a fact still remembers.
    let mut out = Vec::with_capacity(found.len());
    for (work_id, mut appearance) in found {
        let Some(work) = crate::work::get(conn, &work_id)? else {
            continue;
        };
        appearance.title = work.title;
        appearance.kind = work.kind;
        out.push(appearance);
    }
    out.sort_by(|a, b| {
        b.hero
            .cmp(&a.hero)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    Ok(out)
}

/// The names a card is found by: its title and its aliases, each once.
pub fn names_of(card: &Note) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for name in card.title.iter().chain(card.aliases.iter()) {
        let name = name.trim();
        if name.is_empty()
            || names
                .iter()
                .any(|n| n.to_lowercase() == name.to_lowercase())
        {
            continue;
        }
        names.push(name.to_owned());
    }
    names
}

/// The works whose versions name one of `names` as whole words.
///
/// Whole words, not prefixes: "Otto" must not find "Ottoline". A Russian name
/// is declined, and there is no stemmer to find "Льва" from "Лев" - which is
/// what a card's aliases are for.
fn works_naming(conn: &Connection, profile_id: &str, names: &[String]) -> Result<Vec<String>> {
    let mut found: Vec<String> = Vec::new();
    for name in names {
        let Some(phrase) = phrase_of(name) else {
            continue;
        };
        let mut statement = conn.prepare(
            "SELECT DISTINCT s.work_id FROM search_index s
              WHERE search_index MATCH ?1 AND s.profile_id = ?2 AND s.entity = 'version'
                AND s.work_id IS NOT NULL",
        )?;
        let works = statement
            .query_map(params![phrase, profile_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for work_id in works {
            if !found.contains(&work_id) {
                found.push(work_id);
            }
        }
    }
    Ok(found)
}

/// A name as one FTS5 phrase: its words in order, each a literal.
fn phrase_of(name: &str) -> Option<String> {
    let words: Vec<&str> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    if words.is_empty() {
        return None;
    }
    Some(format!("\"{}\"", words.join(" ").replace('"', "\"\"")))
}

/// The works whose cover names the card: as its hero (v0.88), by a
/// `[[card:id]]` reference in its words, or by one of its names as a whole
/// word of them. The words are the cover's own - its idea, its scene, what
/// the person wrote for each block - not the ids and settings beside them.
fn covers_naming(
    conn: &Connection,
    profile_id: &str,
    card_id: &str,
    names: &[String],
) -> Result<Vec<String>> {
    let reference = format!("[[card:{card_id}]]");
    let mut statement =
        conn.prepare("SELECT id, cover FROM work WHERE profile_id = ?1 AND cover <> '{}'")?;
    let covers = statement
        .query_map(params![profile_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(covers
        .into_iter()
        .filter(|(_, raw)| {
            let Ok(cover) = serde_json::from_str::<crate::cover::Cover>(raw) else {
                return false;
            };
            if cover.hero_card() == Some(card_id) {
                return true;
            }
            let words = [
                cover.idea.as_str(),
                cover.scene.as_str(),
                cover.picture.as_str(),
                cover.negative.as_str(),
                cover.typography.as_str(),
            ]
            .join("\n");
            words.contains(&reference) || names.iter().any(|name| names_word(&words, name))
        })
        .map(|(id, _)| id)
        .collect())
}

/// Whether `text` holds `name` as a whole word, in any case.
pub fn names_word(text: &str, name: &str) -> bool {
    let text: Vec<char> = text.to_lowercase().chars().collect();
    let name: Vec<char> = name.trim().to_lowercase().chars().collect();
    if name.is_empty() || name.len() > text.len() {
        return false;
    }
    let boundary = |at: Option<&char>| at.is_none_or(|c| !c.is_alphanumeric());
    (0..=text.len() - name.len()).any(|start| {
        text[start..start + name.len()] == name[..]
            && boundary(start.checked_sub(1).and_then(|i| text.get(i)))
            && boundary(text.get(start + name.len()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_found_as_a_whole_word_only() {
        assert!(names_word("Otto left the kiln on", "otto"));
        assert!(names_word("— Лев, ты?", "Лев"));
        assert!(!names_word("Ottoline left", "Otto"));
        assert!(
            !names_word("Льва нет", "Лев"),
            "a declined form is an alias of its own"
        );
        assert!(!names_word("", "Otto"));
    }

    #[test]
    fn a_name_becomes_one_literal_phrase() {
        assert_eq!(
            phrase_of("Semyon Arkadyevich").as_deref(),
            Some("\"Semyon Arkadyevich\"")
        );
        assert_eq!(phrase_of("«Штиль»").as_deref(), Some("\"Штиль\""));
        assert_eq!(phrase_of("—"), None);
    }
}
