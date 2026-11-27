//! Gestures on the register of repeats: a term entered, edited, and the works
//! named as carrying it (ADR 0044). Throwing a term away is the trash's
//! gesture, like every deletion.

use rusqlite::Connection;

use super::gesture;
use crate::error::{Error, Result};
use crate::register::{self, NewTerm, Term, TermPatch};

pub fn create(conn: &Connection, new: NewTerm) -> Result<Term> {
    gesture(conn, "term.create", |act| {
        act.json("term", &new)?;
        let minted = act.mint();
        register::create_minted(act, act.profile_id(), new, minted)
    })
}

pub fn update(conn: &Connection, id: &str, patch: TermPatch) -> Result<Term> {
    gesture(conn, "term.update", |act| {
        let before = register::get(act, id)?;
        act.param("id", id);
        act.json("patch", &patch)?;
        act.before(before.as_ref(), &patch)?;
        act.stamped();
        register::update_at(act, id, patch, act.at())
    })
}

/// Name a work as carrying a term - how a meaning is tied to the works it is
/// in. Naming it twice changes nothing and records nothing.
pub fn link(conn: &Connection, term_id: &str, work_id: &str) -> Result<()> {
    gesture(conn, "term.link", |act| {
        if linked(act, term_id, work_id)? {
            act.unchanged();
            return Ok(());
        }
        act.param("termId", term_id);
        act.param("workId", work_id);
        act.param("title", act.title_of(work_id));
        let minted = act.mint();
        register::link_minted(act, term_id, work_id, minted)
    })
}

/// Let go of a work named as carrying a term. The row goes outright, like a
/// link between works: it is a fact about two things, not a thing with a body.
pub fn unlink(conn: &Connection, term_id: &str, work_id: &str) -> Result<()> {
    gesture(conn, "term.unlink", |act| {
        let created_at: Option<(String, String)> = act
            .query_row(
                "SELECT id, created_at FROM term_work WHERE term_id = ?1 AND work_id = ?2",
                rusqlite::params![term_id, work_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        let Some((id, at)) = created_at else {
            return Err(Error::not_found(
                "term_work",
                format!("{term_id}/{work_id}"),
            ));
        };
        // The row's own id and moment go into the log, so an undo puts back
        // the same row rather than a new one that looks like it.
        act.param("termId", term_id);
        act.param("workId", work_id);
        act.param("title", act.title_of(work_id));
        act.param("id", id);
        act.param("createdAt", at);
        register::unlink(act, term_id, work_id)?;
        Ok(())
    })
}

/// Make one word of the record into several (v0.171): what an agent could
/// propose and a person could only do by hand, word by word. Each part is
/// found or made with what the one held, and the one goes to the trash -
/// every step a gesture of its own in the history, all of them or none.
///
/// The parts are words: trimmed, the empty ones dropped, each once. Fewer
/// than two is no split - one part is a rename, and that is the word's own
/// field.
pub fn split(conn: &Connection, id: &str, into: &[String]) -> Result<Vec<Term>> {
    let mut parts: Vec<String> = Vec::new();
    for part in into {
        let part = part.trim();
        if !part.is_empty()
            && !parts
                .iter()
                .any(|known| crate::words::plain(known) == crate::words::plain(part))
        {
            parts.push(part.to_owned());
        }
    }
    if parts.len() < 2 {
        return Err(Error::refused("term.splitNeedsParts"));
    }
    crate::db::unit::atomically(conn, |conn| {
        let found = register::get(conn, id)?.ok_or_else(|| Error::not_found("term", id))?;
        let made = register::proposal::split(conn, &found.profile_id, &found, &parts)?;
        let mut terms = Vec::with_capacity(made.len());
        for part in made {
            if let Some(term) = register::get(conn, &part)? {
                terms.push(term);
            }
        }
        Ok(terms)
    })
}

/// Make two words of the record one (v0.171): `id` goes into `into_id`. Its
/// word and its forms become forms of the one that stays, its ways of
/// singing are added where the other has none for that spelling, the works
/// named for it are named for the other, and it goes to the trash. What the
/// one that stays says of itself - its kind, how strictly it is spent, its
/// topic and note - is left as it is: merging says they are the same thing,
/// and the one kept is the one whose words about it stand.
pub fn merge(conn: &Connection, id: &str, into_id: &str) -> Result<Term> {
    if id == into_id {
        return Err(Error::refused("term.mergeItself"));
    }
    crate::db::unit::atomically(conn, |conn| {
        let gone = register::get(conn, id)?.ok_or_else(|| Error::not_found("term", id))?;
        let kept =
            register::get(conn, into_id)?.ok_or_else(|| Error::not_found("term", into_id))?;
        let plain = crate::words::plain;

        let mut forms = kept.forms.clone();
        for form in std::iter::once(&gone.word).chain(&gone.forms) {
            let known = plain(&kept.word) == plain(form)
                || forms.iter().any(|one| plain(one) == plain(form));
            if !known {
                forms.push(form.clone());
            }
        }
        let mut sung = kept.sung.clone();
        for one in &gone.sung {
            if !sung
                .iter()
                .any(|known| known.written.to_lowercase() == one.written.to_lowercase())
            {
                sung.push(one.clone());
            }
        }
        let patch = TermPatch {
            forms: Some(forms).filter(|forms| *forms != kept.forms),
            sung: Some(sung).filter(|sung| *sung != kept.sung),
            ..TermPatch::default()
        };
        if patch.forms.is_some() || patch.sung.is_some() {
            update(conn, into_id, patch)?;
        }
        for used in register::uses(conn, id)? {
            if used.named {
                link(conn, into_id, &used.work_id)?;
            }
        }
        super::trash::discard(conn, crate::trash::Entity::Term, id)?;
        register::get(conn, into_id)?.ok_or_else(|| Error::not_found("term", into_id))
    })
}

fn linked(conn: &Connection, term_id: &str, work_id: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM term_work WHERE term_id = ?1 AND work_id = ?2)",
        rusqlite::params![term_id, work_id],
        |row| row.get(0),
    )?)
}

/// "I know, I am keeping it" on a word the guard of repeats found in a song
/// (ADR 0054): the finding stays drawn and stops counting. Said twice, it
/// changes nothing and records nothing.
pub fn keep_repeat(conn: &Connection, work_id: &str, word: &str) -> Result<()> {
    gesture(conn, "repeat.keep", |act| {
        if crate::register::guard::is_kept(act, work_id, word)? {
            act.unchanged();
            return Ok(());
        }
        act.param("workId", work_id);
        act.param("word", word.trim().to_lowercase());
        act.param("title", act.title_of(work_id));
        let minted = act.mint();
        crate::register::guard::keep_minted(act, act.profile_id(), work_id, word, minted)
    })
}

/// Take "I am keeping it" back: the finding counts again. The row's id and
/// moment go into the log, so an undo puts back the same row.
pub fn unkeep_repeat(conn: &Connection, work_id: &str, word: &str) -> Result<()> {
    gesture(conn, "repeat.unkeep", |act| {
        let word = word.trim().to_lowercase();
        let row: Option<(String, String)> = act
            .query_row(
                "SELECT id, created_at FROM repeat_kept WHERE work_id = ?1 AND word = ?2",
                rusqlite::params![work_id, word],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        let Some((id, at)) = row else {
            act.unchanged();
            return Ok(());
        };
        act.param("workId", work_id);
        act.param("word", word.clone());
        act.param("title", act.title_of(work_id));
        act.param("id", id);
        act.param("createdAt", at);
        crate::register::guard::unkeep(act, work_id, &word)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::register::Sung;

    fn term(conn: &Connection, word: &str, forms: &[&str]) -> Term {
        create(
            conn,
            NewTerm {
                word: word.into(),
                forms: forms.iter().map(|form| (*form).to_owned()).collect(),
                ..NewTerm::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn a_word_of_two_is_split_by_a_gesture_of_the_window() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        let both = term(&conn, "harbour, lantern", &[]);
        link(&conn, &both.id, &song.id).unwrap();

        // One part is a rename, not a split; the same word twice is one.
        for few in [
            vec![],
            vec!["harbour".to_owned()],
            vec!["harbour".into(), " Harbour ".into()],
        ] {
            assert_eq!(
                split(&conn, &both.id, &few).unwrap_err().reason().key,
                "refusal.term.splitNeedsParts"
            );
        }

        let parts = split(&conn, &both.id, &["harbour".into(), " lantern ".into()]).unwrap();
        assert_eq!(
            parts
                .iter()
                .map(|part| part.word.as_str())
                .collect::<Vec<_>>(),
            ["harbour", "lantern"]
        );
        assert!(
            register::get(&conn, &both.id).unwrap().is_none(),
            "the whole is in the trash"
        );
        for part in &parts {
            assert_eq!(part.kind, both.kind);
            assert!(
                register::uses(&conn, &part.id)
                    .unwrap()
                    .iter()
                    .any(|used| used.named)
            );
        }
    }

    #[test]
    fn two_words_are_made_one_and_the_one_that_stays_keeps_its_own_say() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        let kept = term(&conn, "lantern", &["lamp"]);
        let gone = create(
            &conn,
            NewTerm {
                word: "lanthorn".into(),
                forms: vec!["lamp".into(), "light".into()],
                sung: vec![Sung {
                    written: "lanthorn".into(),
                    sung: "lant-horn".into(),
                }],
                note: Some("the old spelling".into()),
                ..NewTerm::default()
            },
        )
        .unwrap();
        link(&conn, &gone.id, &song.id).unwrap();

        assert_eq!(
            merge(&conn, &kept.id, &kept.id).unwrap_err().reason().key,
            "refusal.term.mergeItself"
        );

        let one = merge(&conn, &gone.id, &kept.id).unwrap();
        assert_eq!(one.id, kept.id);
        assert_eq!(one.word, "lantern");
        // The word that went and its forms, each once.
        assert_eq!(one.forms, ["lamp", "lanthorn", "light"]);
        assert_eq!(one.sung.len(), 1);
        assert_eq!(
            one.note, kept.note,
            "what the one that stays says of itself stands"
        );
        assert!(register::get(&conn, &gone.id).unwrap().is_none());
        assert!(
            register::uses(&conn, &kept.id)
                .unwrap()
                .iter()
                .any(|used| used.named)
        );
    }
}
