//! Finding anything, from one box.
//!
//! Four kinds of thing are searched — works, version bodies, notes and chat
//! messages — and each answers a different question: *where is that song*,
//! *where did I write that line*, *what did I note about it*, *what did the
//! assistant say*.
//!
//! ## Why FTS5, and why it replaced a loop in Rust
//!
//! This module used to read every version body of the profile into memory on
//! each keystroke and fold it with Rust's `to_lowercase`, because SQLite's
//! `LIKE` folds ASCII only: `лето` did not match `Лето`, and half of this
//! app's text is Russian. That was the right call while the alternative was
//! `LIKE`.
//!
//! FTS5's `unicode61` tokenizer folds the whole of Unicode, so the reason for
//! the loop is gone — and with it the cost of reading a catalogue's worth of
//! text per keystroke. The index is `search_index`, filled and maintained by
//! the triggers of migration 0023.
//!
//! ## The one thing FTS5 does not do for Russian
//!
//! There is no stemmer. A bare `холодильник` matches only that exact word, so
//! *в холодильнике* is missed — which is precisely the search a person means.
//! Every term is therefore turned into a prefix term (`холодильник*`), which
//! finds both. `query_of` is where that happens, and it is also where the
//! query syntax of FTS5 is defused: what the person typed is words to search
//! for, never an expression to run.

use rusqlite::{Connection, params};
use serde::Serialize;

use crate::error::Result;

/// What a hit points at, and what opening it should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Work,
    Version,
    Note,
    Message,
}

impl Kind {
    /// The tag the index stores, and the one a query asks for.
    fn as_entity(self) -> &'static str {
        match self {
            Kind::Work => "work",
            Kind::Version => "version",
            Kind::Note => "note",
            Kind::Message => "message",
        }
    }
}

/// One thing found.
#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub kind: Kind,
    /// The row that matched: a work, a version, a note, a message. A note
    /// opens on the notes screen by it, which is why it travels.
    pub entity_id: String,
    /// The work it belongs to. Absent only for a note on nothing in
    /// particular, which has a screen of its own to open on; a chat without a
    /// work is skipped rather than offered with nowhere to go.
    pub work_id: Option<String>,
    /// The work's title, or empty with no work.
    pub work_title: String,
    /// What to show as the hit's own line: a title, or the matching text.
    pub title: String,
    /// Where it came from: `lyrics · Revision 2`, `note`, `assistant`.
    pub detail: String,
    /// Rank within its kind — lower sorts first.
    pub rank: i64,
}

/// How many hits of each kind are worth showing.
///
/// A palette is for recognising something, not for browsing everything: past
/// half a dozen per kind the list stops being scannable and the answer is a
/// narrower query.
const PER_KIND: usize = 6;

/// Case-folded form used for comparison.
///
/// `to_lowercase` rather than `to_ascii_lowercase`, which is the entire point:
/// it knows that `Л` is `л`. Kept although the search itself now folds in
/// SQLite, because the catalogue's own field filters still compare in Rust.
pub fn fold(text: &str) -> String {
    text.to_lowercase()
}

/// Does `haystack` contain `needle`, ignoring case in any language?
pub fn matches(haystack: &str, needle_folded: &str) -> bool {
    fold(haystack).contains(needle_folded)
}

/// The FTS5 expression for what a person typed, or `None` if it held no words.
///
/// Two jobs, and the second is the one that bites. The first is prefixes: with
/// no Russian stemmer, `холодильник` alone misses *в холодильнике*, so every
/// term becomes `холодильник*`.
///
/// The second is that FTS5's query syntax is a language — `AND`, `NEAR`, `-`,
/// `"`, `*`, `(` all mean something in it. A person typing `don't` or `rock —
/// ballad` is not writing an expression, and a syntax error in a search box
/// reads as "the search is broken". Every term is therefore wrapped in double
/// quotes, with the quote character itself doubled, which makes it a literal
/// string to FTS5; the `*` is appended outside the quotes, where it still
/// means "prefix".
pub fn query_of(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        // Punctuation is never part of a term: `unicode61` splits on it too, so
        // keeping it would only produce terms that cannot match.
        .split(|c: char| !c.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"*", term.replace('"', "\"\"")))
        .collect();

    if terms.is_empty() {
        return None;
    }
    // Every word has to appear: typing a second word narrows a search, it does
    // not widen it.
    Some(terms.join(" AND "))
}

/// Everything matching `query` in one profile, grouped by kind.
pub fn find(conn: &Connection, profile_id: &str, query: &str) -> Result<Vec<Hit>> {
    let Some(expression) = query_of(query) else {
        return Ok(Vec::new());
    };

    let mut hits = Vec::new();
    for kind in [Kind::Work, Kind::Version, Kind::Note, Kind::Message] {
        hits.extend(of_kind(conn, profile_id, &expression, kind)?);
    }
    Ok(hits)
}

/// Which works answer `query`, anywhere in them, best first.
///
/// The palette's question is "show me the thing I am thinking of"; the
/// catalogue's is "which of my works mention a fridge", and that is a different
/// answer — one row per work, however many lines inside it matched, and no cap,
/// because the catalogue is a list to read down rather than a menu to pick
/// from.
///
/// Returned as ids rather than as rows: the catalogue already holds its rows,
/// with their scores, tiers and columns, and a second shape of the same work
/// would be a second truth about it.
pub fn works_matching(conn: &Connection, profile_id: &str, query: &str) -> Result<Vec<String>> {
    let Some(expression) = query_of(query) else {
        return Ok(Vec::new());
    };

    // `min(rank)` is the best line found in the work: a song whose chorus is
    // about the fridge should outrank one that mentions it once in a note.
    let mut statement = conn.prepare(
        "SELECT s.work_id
           FROM search_index s
          WHERE search_index MATCH ?1
            AND s.profile_id = ?2
            AND s.work_id IS NOT NULL
          GROUP BY s.work_id
          ORDER BY min(rank)",
    )?;

    let ids = statement
        .query_map(params![expression, profile_id], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ids)
}

/// The hits of one kind, best first.
///
/// One query per kind rather than one query sorted afterwards: the cap is per
/// kind, and asking for six of each is what lets a single well-matched note
/// survive a thousand matching lyric lines.
fn of_kind(conn: &Connection, profile_id: &str, expression: &str, kind: Kind) -> Result<Vec<Hit>> {
    // `snippet` is the index's own excerpt: the window around the match, with
    // the match itself marked. Rust cannot do better here without reading the
    // body back, and it is what made the old search read every body.
    let mut statement = conn.prepare(
        "SELECT s.entity_id, s.work_id, snippet(search_index, 4, '', '', '…', 12)
           FROM search_index s
          WHERE search_index MATCH ?1
            AND s.profile_id = ?2
            AND s.entity = ?3
          ORDER BY rank
          LIMIT ?4",
    )?;

    let rows = statement
        .query_map(
            params![expression, profile_id, kind.as_entity(), PER_KIND as i64],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut hits = Vec::with_capacity(rows.len());
    for (index, (entity_id, work_id, snippet)) in rows.into_iter().enumerate() {
        // A hit with nowhere to open is worse than no hit: a chat on nothing is
        // skipped rather than offered. A note on nothing opens on the notes
        // screen, so it stays.
        if work_id.is_none() && kind != Kind::Note {
            continue;
        }
        let Some(described) = describe(conn, kind, &entity_id, work_id.as_deref(), &snippet)?
        else {
            continue;
        };
        hits.push(Hit {
            rank: index as i64,
            ..described
        });
    }
    Ok(hits)
}

/// Dress one indexed row as a hit: what it is called, and where it came from.
///
/// The index holds only the text that was searched, so the line and the label
/// are read from the row itself. One small query per hit, and there are at
/// most `PER_KIND` of them per kind.
fn describe(
    conn: &Connection,
    kind: Kind,
    entity_id: &str,
    work_id: Option<&str>,
    snippet: &str,
) -> Result<Option<Hit>> {
    let entity = entity_id.to_owned();
    let work = work_id.map(str::to_owned);
    let found = match kind {
        Kind::Work => conn
            .query_row(
                "SELECT title, kind, status FROM work WHERE id = ?1",
                params![entity_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .map(|(title, kind_key, status)| Hit {
                kind: Kind::Work,
                entity_id: entity.clone(),
                work_id: work.clone(),
                work_title: title.clone(),
                // A work's own line is its title, not the snippet: the point of
                // finding a work is recognising it.
                title,
                detail: format!("{kind_key} · {status}"),
                rank: 0,
            }),
        Kind::Version => conn
            .query_row(
                "SELECT w.title, v.role, v.revision, v.label
                   FROM work_version v JOIN work w ON w.id = v.work_id
                  WHERE v.id = ?1",
                params![entity_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .map(|(work_title, role, revision, label)| Hit {
                kind: Kind::Version,
                entity_id: entity.clone(),
                work_id: work.clone(),
                work_title,
                // The line it was found in, not the version's name: the name is
                // in the detail, and what was searched for is the text.
                title: one_line(snippet),
                detail: format!(
                    "{role} · {}",
                    label.unwrap_or_else(|| format!("v{revision}"))
                ),
                rank: 0,
            }),
        Kind::Note => conn
            .query_row(
                "SELECT coalesce(w.title, ''), n.title, n.kind
                   FROM note n LEFT JOIN work w ON w.id = n.work_id
                  WHERE n.id = ?1",
                params![entity_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .map(|(work_title, title, note_kind)| Hit {
                kind: Kind::Note,
                entity_id: entity.clone(),
                work_id: work.clone(),
                work_title,
                title: title.unwrap_or_else(|| one_line(snippet)),
                detail: note_kind,
                rank: 0,
            }),
        Kind::Message => conn
            .query_row(
                "SELECT w.title, m.role
                   FROM chat_message m
                   JOIN chat c ON c.id = m.chat_id
                   JOIN work w ON w.id = c.work_id
                  WHERE m.id = ?1",
                params![entity_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map(|(work_title, role)| Hit {
                kind: Kind::Message,
                entity_id: entity.clone(),
                work_id: work.clone(),
                work_title,
                title: one_line(snippet),
                detail: role,
                rank: 0,
            }),
    };

    match found {
        Ok(hit) => Ok(Some(hit)),
        // The row went away between the index and this query — a deletion mid
        // search. Dropping it beats failing the whole search for one stale row.
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// A body is many lines; a hit is one line of interface.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::{self, NewChat};
    use crate::db;
    use crate::note::{self, NewNote};
    use crate::profile;
    use crate::work::version::{self, NewVersion};
    use crate::work::{self, NewWork};

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    fn song(conn: &Connection, profile_id: &str, title: &str) -> String {
        work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: title.into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn a_work_is_found_by_part_of_its_title() {
        let (conn, profile_id) = workspace();
        song(&conn, &profile_id, "Harbour lights");
        song(&conn, &profile_id, "Winter shift");

        let hits = find(&conn, &profile_id, "harbour").unwrap();

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, Kind::Work);
        assert_eq!(hits[0].title, "Harbour lights");
    }

    // The reason this module was written by hand for a year. SQLite's own
    // `lower()` and `LIKE` ignore case for ASCII only, so a Russian workspace
    // would find nothing typed in the wrong case — which is most of the time.
    // FTS5's `unicode61` is what now makes this pass.
    #[test]
    fn case_is_ignored_in_russian_too() {
        let (conn, profile_id) = workspace();
        song(&conn, &profile_id, "Гавань огней");

        for query in ["гавань", "ГАВАНЬ", "ГаВаНь", "огней"] {
            let hits = find(&conn, &profile_id, query).unwrap();
            assert_eq!(hits.len(), 1, "`{query}` should have found the work");
        }
    }

    // Russian has no stemmer here, so the word as typed is rarely the word as
    // written: a person looking for a fridge types `холодильник` and the line
    // says `в холодильнике`. Without prefix terms this finds nothing at all,
    // which is the whole feature failing quietly.
    #[test]
    fn a_word_matches_its_russian_inflections() {
        let (conn, profile_id) = workspace();
        let work = song(&conn, &profile_id, "Кухня");
        let mut conn = conn;
        version::create(
            &mut conn,
            &work,
            NewVersion {
                role: "lyrics".into(),
                body: "Тихо гудит в холодильнике свет, и кофейня закрыта".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();

        for query in ["холодильник", "кофе", "ХОЛОДИЛЬНИК"] {
            let hits = find(&conn, &profile_id, query).unwrap();
            assert!(
                hits.iter().any(|hit| hit.kind == Kind::Version),
                "`{query}` should have found the line"
            );
        }
    }

    // Two words narrow, they do not widen: a work matching only one of them is
    // not what the person asked for.
    #[test]
    fn every_word_has_to_appear() {
        let (conn, profile_id) = workspace();
        song(&conn, &profile_id, "Harbour lights");
        song(&conn, &profile_id, "Harbour bells");

        let hits = find(&conn, &profile_id, "harbour lights").unwrap();

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Harbour lights");
    }

    // FTS5's query language is a language, and a search box is not a place to
    // write one. Anything typed is words, never syntax — a stray quote or a
    // dash must not turn into a syntax error the person cannot read.
    #[test]
    fn punctuation_is_searched_for_not_executed() {
        let (conn, profile_id) = workspace();
        song(&conn, &profile_id, "Harbour lights");

        for query in ["\"", "harbour OR", "-harbour", "harbour AND (", "NEAR("] {
            let hits = find(&conn, &profile_id, query);
            assert!(hits.is_ok(), "`{query}` should not have failed: {hits:?}");
        }
        // And the words inside the noise still work.
        assert_eq!(find(&conn, &profile_id, "\"harbour\"").unwrap().len(), 1);
    }

    // The craft fields are where half of a song's description lives, and they
    // were not searched at all before: `meta` was never even selected.
    #[test]
    fn a_work_is_found_by_its_craft_fields_and_tags() {
        let (conn, profile_id) = workspace();
        let work = song(&conn, &profile_id, "Harbour lights");
        work::update(
            &conn,
            &work,
            work::WorkPatch {
                meta: Some(
                    serde_json::json!({ "mood": "melancholy surrealism" })
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
                tags: Some(vec!["winter".into()]),
                ..Default::default()
            },
        )
        .unwrap();

        for query in ["surrealism", "winter"] {
            let hits = find(&conn, &profile_id, query).unwrap();
            assert!(
                hits.iter()
                    .any(|hit| hit.work_id.as_deref() == Some(work.as_str())),
                "`{query}` should have found the work"
            );
        }
    }

    #[test]
    fn a_line_inside_a_version_is_found_and_quoted() {
        let (conn, profile_id) = workspace();
        let work = song(&conn, &profile_id, "Harbour lights");
        let mut conn = conn;
        version::create(
            &mut conn,
            &work,
            NewVersion {
                role: "lyrics".into(),
                body: "The cranes go still at seven.\nI count the lights across the bay.".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();

        let hits = find(&conn, &profile_id, "cranes").unwrap();
        let version = hits.iter().find(|hit| hit.kind == Kind::Version).unwrap();

        assert!(version.title.contains("cranes"), "got: {}", version.title);
        assert_eq!(version.work_id.as_deref(), Some(work.as_str()));
        // One line of interface, whatever the body's line breaks were.
        assert!(!version.title.contains('\n'));
    }

    // An edited body is a different body. The index is kept by triggers, and a
    // trigger that fires only on insert would answer with last week's text.
    #[test]
    fn an_edited_body_is_searched_as_it_now_reads() {
        let (conn, profile_id) = workspace();
        let work = song(&conn, &profile_id, "Harbour lights");
        let mut conn = conn;
        let version = version::create(
            &mut conn,
            &work,
            NewVersion {
                role: "lyrics".into(),
                body: "The cranes go still at seven".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();

        conn.execute(
            "UPDATE work_version SET body = ?1 WHERE id = ?2",
            params!["The gulls go quiet at seven", version.id],
        )
        .unwrap();

        assert!(
            find(&conn, &profile_id, "cranes").unwrap().is_empty(),
            "the old text should be gone from the index"
        );
        assert!(
            find(&conn, &profile_id, "gulls")
                .unwrap()
                .iter()
                .any(|hit| hit.kind == Kind::Version),
            "the new text should be in it"
        );
    }

    // A deleted work must not leave hits pointing at nothing.
    #[test]
    fn a_deleted_work_leaves_the_index() {
        let (conn, profile_id) = workspace();
        let work = song(&conn, &profile_id, "Harbour lights");
        conn.execute("DELETE FROM work WHERE id = ?1", params![work])
            .unwrap();

        assert!(find(&conn, &profile_id, "harbour").unwrap().is_empty());
    }

    #[test]
    fn a_note_is_found_by_its_body() {
        let (conn, profile_id) = workspace();
        let work = song(&conn, &profile_id, "Harbour lights");
        note::create(
            &conn,
            &profile_id,
            NewNote {
                body: "the second verse still explains itself".into(),
                kind: None,
                title: None,
                work_id: Some(work.clone()),
                tags: vec![],
            },
        )
        .unwrap();

        let hits = find(&conn, &profile_id, "explains").unwrap();
        let note = hits.iter().find(|hit| hit.kind == Kind::Note).unwrap();

        assert_eq!(note.work_id.as_deref(), Some(work.as_str()));
    }

    #[test]
    fn a_note_on_nothing_is_found_and_names_itself() {
        let (conn, profile_id) = workspace();
        let loose = note::create(
            &conn,
            &profile_id,
            NewNote {
                body: "graphite marks the layer".into(),
                kind: None,
                title: None,
                work_id: None,
                tags: vec![],
            },
        )
        .unwrap();

        let hits = find(&conn, &profile_id, "graphite").unwrap();
        let note = hits
            .iter()
            .find(|hit| hit.kind == Kind::Note)
            .expect("a note on no work has the notes screen to open on");

        assert_eq!(note.work_id, None);
        assert_eq!(note.entity_id, loose.id);
    }

    #[test]
    fn a_chat_message_is_found_and_carries_its_work() {
        let (conn, profile_id) = workspace();
        let work = song(&conn, &profile_id, "Harbour lights");
        let chat = assistant::create(
            &conn,
            &profile_id,
            NewChat {
                work_id: Some(work.clone()),
                title: None,
                ..Default::default()
            },
        )
        .unwrap();
        assistant::append(
            &conn,
            &chat.id,
            "user",
            "what rhymes with harbour",
            serde_json::Map::new(),
        )
        .unwrap();

        let hits = find(&conn, &profile_id, "rhymes").unwrap();
        let message = hits.iter().find(|hit| hit.kind == Kind::Message).unwrap();

        assert_eq!(message.work_id.as_deref(), Some(work.as_str()));
        assert_eq!(message.work_title, "Harbour lights");
    }

    #[test]
    fn nothing_is_searched_for_when_the_query_is_blank() {
        let (conn, profile_id) = workspace();
        song(&conn, &profile_id, "Harbour lights");

        assert!(find(&conn, &profile_id, "").unwrap().is_empty());
        assert!(find(&conn, &profile_id, "   ").unwrap().is_empty());
        // Punctuation alone holds no words either.
        assert!(find(&conn, &profile_id, " -- ").unwrap().is_empty());
    }

    #[test]
    fn the_search_stays_inside_its_profile() {
        let (conn, profile_id) = workspace();
        conn.execute(
            "INSERT INTO profile (id, key, name, config, is_active, is_builtin, created_at, updated_at)
             SELECT 'other', 'other', 'Other', config, 0, 0, created_at, updated_at FROM profile LIMIT 1",
            [],
        )
        .unwrap();
        song(&conn, &profile_id, "Harbour lights");
        song(&conn, "other", "Harbour bells");

        let hits = find(&conn, &profile_id, "harbour").unwrap();

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Harbour lights");
    }

    #[test]
    fn each_kind_is_capped_so_one_kind_cannot_bury_the_rest() {
        let (conn, profile_id) = workspace();
        for index in 0..PER_KIND + 4 {
            song(&conn, &profile_id, &format!("Harbour {index}"));
        }

        let hits = find(&conn, &profile_id, "harbour").unwrap();

        assert_eq!(hits.len(), PER_KIND);
    }

    // The catalogue's question, and the whole point of the release: a work is
    // found by a word inside its lyric, with nothing of the sort in its title.
    #[test]
    fn a_work_is_listed_for_a_word_only_its_body_holds() {
        let (conn, profile_id) = workspace();
        let kitchen = song(&conn, &profile_id, "Кухня");
        let harbour = song(&conn, &profile_id, "Гавань огней");
        let mut conn = conn;
        version::create(
            &mut conn,
            &kitchen,
            NewVersion {
                role: "lyrics".into(),
                body: "Тихо гудит в холодильнике свет".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();

        let found = works_matching(&conn, &profile_id, "холодильник").unwrap();

        assert_eq!(found, vec![kitchen]);
        assert!(!found.contains(&harbour));
    }

    // One row per work, however many lines inside it matched: a catalogue that
    // listed a song four times because four verses mention the sea would be
    // worse than one that could not search at all.
    #[test]
    fn a_work_is_listed_once_however_often_it_matches() {
        let (conn, profile_id) = workspace();
        let work = song(&conn, &profile_id, "Sea songs");
        let mut conn = conn;
        for index in 0..3 {
            version::create(
                &mut conn,
                &work,
                NewVersion {
                    role: "lyrics".into(),
                    body: format!("the sea again, take {index}"),
                    label: None,
                    meta: None,
                    make_current: false,
                    parent_version_id: None,
                },
            )
            .unwrap();
        }

        let found = works_matching(&conn, &profile_id, "sea").unwrap();

        assert_eq!(found, vec![work]);
    }

    // Unlike the palette, the catalogue is a list to read down: capping it at
    // six would quietly hide the rest of the answer.
    #[test]
    fn the_catalogue_answer_is_not_capped() {
        let (conn, profile_id) = workspace();
        for index in 0..PER_KIND + 4 {
            song(&conn, &profile_id, &format!("Harbour {index}"));
        }

        let found = works_matching(&conn, &profile_id, "harbour").unwrap();

        assert_eq!(found.len(), PER_KIND + 4);
    }

    #[test]
    fn the_catalogue_answer_stays_inside_its_profile() {
        let (conn, profile_id) = workspace();
        conn.execute(
            "INSERT INTO profile (id, key, name, config, is_active, is_builtin, created_at, updated_at)
             SELECT 'other', 'other', 'Other', config, 0, 0, created_at, updated_at FROM profile LIMIT 1",
            [],
        )
        .unwrap();
        let mine = song(&conn, &profile_id, "Harbour lights");
        song(&conn, "other", "Harbour bells");

        assert_eq!(
            works_matching(&conn, &profile_id, "harbour").unwrap(),
            vec![mine]
        );
    }
}
