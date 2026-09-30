//! The predecessor's register of repeats and bank of ideas, brought into the
//! register and the notes (ADR 0044, 0045).
//!
//! Both are optional in the source: an older database has neither table, and
//! the import of the works goes on without them. A second import skips what
//! is already here - a term by its word, an idea by its words - so running it
//! twice brings nothing twice.

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, params};

use crate::error::Result;
use crate::note::{self, NewNote, NoteFilter, NoteState};
use crate::register::{self, NewTerm, Strictness, TermKind};

/// What the register and the bank brought in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Brought {
    pub terms: usize,
    /// Works named as carrying a meaning.
    pub named: usize,
    pub notes: usize,
}

fn has_table(legacy: &Connection, name: &str) -> Result<bool> {
    Ok(legacy.query_row(
        "SELECT count(*) > 0 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![name],
        |row| row.get(0),
    )?)
}

/// A term as the predecessor wrote it, read into a word and its forms.
///
/// The register was written for a person to read: "окно / окна" is a word and
/// a form, "мыть(ся)" is two words, "тихий свет (в окне)" is a phrase with
/// a note on where it tends to go. A term with a placeholder in it - "просто
/// X", "тону в …" - is not wording at all but a device, and is kept as a
/// pattern, whole.
pub fn read_term(raw: &str, kind: TermKind) -> (String, Vec<String>, TermKind) {
    let bare = raw.replace(['«', '»', '"'], "");
    let bare = bare.trim();
    let placeholder = bare
        .split_whitespace()
        .any(|token| matches!(token, "X" | "Х" | "…" | "...") || token.ends_with('…'));
    if !kind.is_wording() || placeholder {
        let kind = if kind.is_wording() {
            TermKind::Pattern
        } else {
            kind
        };
        return (bare.to_owned(), Vec::new(), kind);
    }

    let mut words: Vec<String> = Vec::new();
    for alternative in bare.split('/') {
        let alternative = alternative.trim();
        let mut plain = String::new();
        let mut with = String::new();
        let mut rest = alternative;
        // "(ся)" glued to a word is an ending that may be there; "(в голове)"
        // after a space is a gloss, and goes.
        while let Some(open) = rest.find('(') {
            let close = rest[open..].find(')').map_or(rest.len(), |at| open + at);
            let before = &rest[..open];
            let inner = rest.get(open + 1..close).unwrap_or_default();
            plain.push_str(before);
            with.push_str(before);
            if !before.ends_with(char::is_whitespace) && !before.is_empty() {
                with.push_str(inner);
            }
            rest = rest.get(close + 1..).unwrap_or_default();
        }
        plain.push_str(rest);
        with.push_str(rest);
        for word in [plain, with] {
            let word = word.split_whitespace().collect::<Vec<_>>().join(" ");
            if !word.is_empty() && !words.contains(&word) {
                words.push(word);
            }
        }
    }
    let mut words = words.into_iter();
    let word = words.next().unwrap_or_else(|| bare.to_owned());
    (word, words.collect(), kind)
}

fn legacy_kind(raw: &str) -> TermKind {
    TermKind::from_word(raw).unwrap_or(TermKind::Noun)
}

fn legacy_strictness(raw: &str) -> Strictness {
    match raw {
        "ban" => Strictness::Ban,
        "rare" => Strictness::Rare,
        _ => Strictness::Limit,
    }
}

/// The register: its terms, and for its meanings the works it named, matched
/// here by title. For wording the works are not brought: they are read off
/// the texts now, and a list typed years ago would be a second answer.
pub fn register(conn: &Connection, legacy: &Connection, profile_id: &str) -> Result<Brought> {
    let mut brought = Brought::default();
    if !has_table(legacy, "repetition_entries")? {
        return Ok(brought);
    }
    let topics = has_table(legacy, "repetition_categories")?;
    let sql = if topics {
        "SELECT e.id, e.term, e.kind, e.severity, c.name, e.note FROM repetition_entries e
           LEFT JOIN repetition_categories c ON c.id = e.category_id
         ORDER BY e.created_at, e.rowid"
    } else {
        "SELECT id, term, kind, severity, NULL, note FROM repetition_entries
         ORDER BY created_at, rowid"
    };
    let mut statement = legacy.prepare(sql)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let songs = has_table(legacy, "repetition_songs")?;
    let works: HashMap<String, String> = {
        let mut statement = conn.prepare("SELECT title, id FROM work WHERE profile_id = ?1")?;
        let rows =
            statement.query_map(params![profile_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };

    for (legacy_id, raw, kind, severity, topic, note) in rows {
        let (word, forms, kind) = read_term(&raw, legacy_kind(&kind));
        let made = match register::create(
            conn,
            profile_id,
            NewTerm {
                word,
                forms,
                kind: Some(kind),
                strictness: Some(legacy_strictness(&severity)),
                topic,
                note,
            },
        ) {
            Ok(made) => made,
            // Already in the register: the person's own entry wins.
            Err(error) if error.refusal().is_some_and(|r| r.code == "term.exists") => continue,
            Err(error) => return Err(error),
        };
        brought.terms += 1;
        if kind.is_wording() || !songs {
            continue;
        }
        let mut statement =
            legacy.prepare("SELECT title FROM repetition_songs WHERE entry_id = ?1")?;
        let titles = statement
            .query_map(params![legacy_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for title in titles {
            if let Some(work_id) = works.get(&title) {
                register::link(conn, &made.id, work_id)?;
                brought.named += 1;
            }
        }
    }
    Ok(brought)
}

/// The bank of ideas: phrases as phrases, the predecessor's plain notes as
/// notes, every other idea - a song, a clip, a picture - as an idea, each in
/// the state it was left in. A kind the profile does not have falls back to
/// an idea, then to a note.
pub fn bank(conn: &Connection, legacy: &Connection, profile_id: &str) -> Result<Brought> {
    let mut brought = Brought::default();
    if !has_table(legacy, "ideas")? {
        return Ok(brought);
    }
    let config = crate::profile::config_for(conn, profile_id)?;
    let landing = |wanted: &str| -> String {
        [wanted, "idea", "note"]
            .into_iter()
            .find(|key| config.note_kind(key).is_some())
            .unwrap_or("note")
            .to_owned()
    };

    let statuses = has_table(legacy, "statuses")?;
    let sql = if statuses {
        "SELECT i.kind, i.title, i.body, i.tags, s.code FROM ideas i
           LEFT JOIN statuses s ON s.id = i.status_id ORDER BY i.created_at, i.rowid"
    } else {
        "SELECT kind, title, body, tags, NULL FROM ideas ORDER BY created_at, rowid"
    };
    let mut statement = legacy.prepare(sql)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    // What is here already, by kind and words: a second import skips it.
    let mut present: HashSet<(String, Option<String>, String)> =
        note::list(conn, profile_id, &NoteFilter::default())?
            .into_iter()
            .map(|n| (n.kind, n.title, n.body))
            .collect();

    for (kind, title, body, tags, status) in rows {
        let body = body.map(|b| b.trim().to_owned()).filter(|b| !b.is_empty());
        let title = title.trim().to_owned();
        let (kind, title, body) = match kind.as_str() {
            // A phrase is its line: the words are the note, not its title.
            "phrase" => {
                let line = match body {
                    Some(more) => format!("{title}\n\n{more}"),
                    None => title,
                };
                (landing("phrase"), None, line)
            }
            "note" => (landing("note"), Some(title), body.unwrap_or_default()),
            _ => (landing("idea"), Some(title), body.unwrap_or_default()),
        };
        let title = title.filter(|t| !t.is_empty());
        if !present.insert((kind.clone(), title.clone(), body.clone())) {
            continue;
        }
        let state = match status.as_deref() {
            Some("used") => NoteState::Used,
            Some("parked") => NoteState::Parked,
            Some("dropped") => NoteState::Dropped,
            _ => NoteState::Fresh,
        };
        note::create(
            conn,
            profile_id,
            NewNote {
                body,
                kind: Some(kind),
                title,
                tags: tags
                    .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
                    .unwrap_or_default(),
                state: Some(state),
                ..NewNote::default()
            },
        )?;
        brought.notes += 1;
    }
    Ok(brought)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(raw: &str, kind: TermKind) -> (String, Vec<String>, TermKind) {
        read_term(raw, kind)
    }

    #[test]
    fn alternatives_become_a_word_and_its_forms() {
        assert_eq!(
            read("тёплый / горячий / жаркий", TermKind::Adjective),
            (
                "тёплый".into(),
                vec!["горячий".into(), "жаркий".into()],
                TermKind::Adjective
            )
        );
    }

    #[test]
    fn a_glued_ending_is_a_second_form_and_a_gloss_goes() {
        assert_eq!(
            read("мыть(ся)", TermKind::Verb),
            ("мыть".into(), vec!["мыться".into()], TermKind::Verb)
        );
        assert_eq!(
            read("тихий свет (в окне)", TermKind::Phrase),
            ("тихий свет".into(), vec![], TermKind::Phrase)
        );
        assert_eq!(
            read("рвать(ся) / порванный", TermKind::Verb),
            (
                "рвать".into(),
                vec!["рваться".into(), "порванный".into()],
                TermKind::Verb
            )
        );
    }

    #[test]
    fn a_placeholder_makes_a_pattern_kept_whole() {
        assert_eq!(
            read("«всего лишь X»", TermKind::Phrase),
            ("всего лишь X".into(), vec![], TermKind::Pattern)
        );
        assert_eq!(
            read("тону в …", TermKind::Phrase),
            ("тону в …".into(), vec![], TermKind::Pattern)
        );
    }

    #[test]
    fn a_meaning_is_kept_as_written() {
        let image = "Маяк/фонарь, который никого не ведёт";
        assert_eq!(
            read(image, TermKind::Image),
            (image.into(), vec![], TermKind::Image)
        );
    }
}
