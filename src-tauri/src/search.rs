//! Finding anything, from one box.
//!
//! Four kinds of thing are searched — works, version bodies, notes and chat
//! messages — and each answers a different question: *where is that song*,
//! *where did I write that line*, *what did I note about it*, *what did the
//! assistant say*.
//!
//! ## Why the matching is done here rather than in SQL
//!
//! SQLite's `LIKE` is case-insensitive for ASCII and nothing else: `лето` does
//! not match `Лето`, because the built-in `lower()` leaves every non-ASCII byte
//! alone. Half of this app's text is Russian, so a search that only works in
//! English is not a search.
//!
//! Rust has the whole of Unicode, so rows are folded and compared here. The
//! query still narrows in SQL where it can — by profile, by kind — so the loop
//! only ever sees one profile's text.
//!
//! FTS5 is the eventual answer for a large workspace. The seam for it is this
//! module: `find` is the only thing the commands call, and what it does inside
//! is nobody else's business.

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

/// One thing found.
#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub kind: Kind,
    /// The work to open. Every hit belongs to one — a note or a chat without a
    /// work is skipped rather than offered with nowhere to go.
    pub work_id: String,
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
/// it knows that `Л` is `л`. Allocation per row is fine at this size — the
/// alternative is a false negative in half the workspace.
pub fn fold(text: &str) -> String {
    text.to_lowercase()
}

/// Does `haystack` contain `needle`, ignoring case in any language?
pub fn matches(haystack: &str, needle_folded: &str) -> bool {
    fold(haystack).contains(needle_folded)
}

/// A window of text around the first match, for showing what was found.
///
/// Character-based rather than byte-based: slicing a Cyrillic body by bytes
/// panics on a boundary, and the panic would be in the search box.
fn excerpt(body: &str, needle_folded: &str, width: usize) -> String {
    let folded = fold(body);
    let Some(byte_at) = folded.find(needle_folded) else {
        return body.chars().take(width).collect();
    };

    // Byte offset in the folded string is not a character offset in the
    // original — folding can change length — so the count of characters before
    // the match is what carries over.
    let chars_before = folded[..byte_at].chars().count();
    let start = chars_before.saturating_sub(width / 3);

    let mut out: String = body.chars().skip(start).take(width).collect();
    if start > 0 {
        out.insert(0, '…');
    }
    if body.chars().count() > start + width {
        out.push('…');
    }
    // A body is many lines; a hit is one line of interface.
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Everything matching `query` in one profile, grouped by kind.
pub fn find(conn: &Connection, profile_id: &str, query: &str) -> Result<Vec<Hit>> {
    let needle = fold(query.trim());
    if needle.is_empty() {
        return Ok(Vec::new());
    }

    let mut hits = Vec::new();
    hits.extend(works(conn, profile_id, &needle)?);
    hits.extend(versions(conn, profile_id, &needle)?);
    hits.extend(notes(conn, profile_id, &needle)?);
    hits.extend(messages(conn, profile_id, &needle)?);
    Ok(hits)
}

fn works(conn: &Connection, profile_id: &str, needle: &str) -> Result<Vec<Hit>> {
    let mut statement = conn.prepare(
        "SELECT id, title, kind, status FROM work
          WHERE profile_id = ?1
          ORDER BY updated_at DESC, rowid DESC",
    )?;

    let rows = statement
        .query_map(params![profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(rows
        .into_iter()
        .filter(|(_, title, _, _)| matches(title, needle))
        .take(PER_KIND)
        .enumerate()
        .map(|(index, (id, title, kind, status))| Hit {
            kind: Kind::Work,
            work_id: id,
            work_title: title.clone(),
            title,
            detail: format!("{kind} · {status}"),
            rank: index as i64,
        })
        .collect())
}

fn versions(conn: &Connection, profile_id: &str, needle: &str) -> Result<Vec<Hit>> {
    let mut statement = conn.prepare(
        "SELECT v.work_id, w.title, v.role, v.revision, v.label, v.body
           FROM work_version v
           JOIN work w ON w.id = v.work_id
          WHERE w.profile_id = ?1
          ORDER BY v.created_at DESC, v.rowid DESC",
    )?;

    let rows = statement
        .query_map(params![profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(rows
        .into_iter()
        .filter(|(_, _, _, _, _, body)| matches(body, needle))
        .take(PER_KIND)
        .enumerate()
        .map(
            |(index, (work_id, work_title, role, revision, label, body))| Hit {
                kind: Kind::Version,
                work_id,
                work_title,
                // The line it was found in, not the version's name: the name is in
                // the detail, and what was searched for is the text.
                title: excerpt(&body, needle, 90),
                detail: format!(
                    "{role} · {}",
                    label.unwrap_or_else(|| format!("v{revision}"))
                ),
                rank: index as i64,
            },
        )
        .collect())
}

fn notes(conn: &Connection, profile_id: &str, needle: &str) -> Result<Vec<Hit>> {
    let mut statement = conn.prepare(
        "SELECT n.work_id, w.title, n.title, n.body, n.kind
           FROM note n
           JOIN work w ON w.id = n.work_id
          WHERE n.profile_id = ?1
          ORDER BY n.updated_at DESC, n.rowid DESC",
    )?;

    let rows = statement
        .query_map(params![profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(rows
        .into_iter()
        .filter(|(_, _, title, body, _)| {
            matches(body, needle) || title.as_deref().is_some_and(|t| matches(t, needle))
        })
        .take(PER_KIND)
        .enumerate()
        .map(|(index, (work_id, work_title, title, body, kind))| Hit {
            kind: Kind::Note,
            work_id,
            work_title,
            title: title.unwrap_or_else(|| excerpt(&body, needle, 90)),
            detail: kind,
            rank: index as i64,
        })
        .collect())
}

fn messages(conn: &Connection, profile_id: &str, needle: &str) -> Result<Vec<Hit>> {
    let mut statement = conn.prepare(
        "SELECT c.work_id, w.title, m.role, m.body
           FROM chat_message m
           JOIN chat c ON c.id = m.chat_id
           JOIN work w ON w.id = c.work_id
          WHERE c.profile_id = ?1
          ORDER BY m.created_at DESC, m.rowid DESC",
    )?;

    let rows = statement
        .query_map(params![profile_id], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(rows
        .into_iter()
        // A chat not attached to a work has nowhere to open; skipping it beats
        // offering a hit that goes nowhere.
        .filter_map(|(work_id, work_title, role, body)| {
            work_id.map(|id| (id, work_title, role, body))
        })
        .filter(|(_, _, _, body)| matches(body, needle))
        .take(PER_KIND)
        .enumerate()
        .map(|(index, (work_id, work_title, role, body))| Hit {
            kind: Kind::Message,
            work_id,
            work_title,
            title: excerpt(&body, needle, 90),
            detail: role,
            rank: index as i64,
        })
        .collect())
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

    // The reason this module exists. SQLite's own `lower()` and `LIKE` ignore
    // case for ASCII only, so a Russian workspace would find nothing typed in
    // the wrong case — which is most of the time.
    #[test]
    fn case_is_ignored_in_russian_too() {
        let (conn, profile_id) = workspace();
        song(&conn, &profile_id, "Гавань огней");

        for query in ["гавань", "ГАВАНЬ", "ГаВаНь", "огней"] {
            let hits = find(&conn, &profile_id, query).unwrap();
            assert_eq!(hits.len(), 1, "`{query}` should have found the work");
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
            },
        )
        .unwrap();

        let hits = find(&conn, &profile_id, "cranes").unwrap();
        let version = hits.iter().find(|hit| hit.kind == Kind::Version).unwrap();

        assert!(version.title.contains("cranes"), "got: {}", version.title);
        assert_eq!(version.work_id, work);
        // One line of interface, whatever the body's line breaks were.
        assert!(!version.title.contains('\n'));
    }

    #[test]
    fn an_excerpt_around_a_late_match_is_trimmed_on_both_sides() {
        let body = "a ".repeat(80) + "needle" + &" b".repeat(80);
        let cut = excerpt(&body, "needle", 40);

        assert!(cut.contains("needle"), "got: {cut}");
        assert!(cut.starts_with('…'), "got: {cut}");
        assert!(cut.ends_with('…'), "got: {cut}");
    }

    // Slicing a Cyrillic body by byte offsets panics on a character boundary,
    // and the panic would happen inside the search box.
    #[test]
    fn an_excerpt_of_cyrillic_text_does_not_panic() {
        let body = "Гавань огней, где вода держит шум и я считаю огни на том берегу".to_owned();
        let cut = excerpt(&body, &fold("огни"), 20);
        assert!(cut.contains("огни"), "got: {cut}");
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

        assert_eq!(note.work_id, work);
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

        assert_eq!(message.work_id, work);
        assert_eq!(message.work_title, "Harbour lights");
    }

    #[test]
    fn nothing_is_searched_for_when_the_query_is_blank() {
        let (conn, profile_id) = workspace();
        song(&conn, &profile_id, "Harbour lights");

        assert!(find(&conn, &profile_id, "").unwrap().is_empty());
        assert!(find(&conn, &profile_id, "   ").unwrap().is_empty());
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
}
