//! The focus board: what the person decided about what the workspace noticed.
//!
//! Findings themselves are derived on the front end and stored nowhere — a
//! finding appears when its complaint becomes true and leaves when it stops
//! being true, without anything having to remember it. What derivation cannot
//! know is here: a complaint that has already been answered, and a line the
//! person wrote themselves.
//!
//! The back end has no opinion about what a complaint *says*. It stores the
//! string and matches it whole, which is what lets a new finding kind ship on
//! the front end without a migration here.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::time::now;

/// A complaint the person has heard and put away.
#[derive(Debug, Clone, Serialize)]
pub struct Dismissal {
    pub kind: String,
    pub work_id: String,
    pub complaint: String,
    pub dismissed_at: String,
}

/// What identifies a dismissal: the kind, the work, and what was said.
#[derive(Debug, Clone, Deserialize)]
pub struct DismissalKey {
    pub kind: String,
    pub work_id: String,
    pub complaint: String,
}

/// A line the person put on the board themselves.
#[derive(Debug, Clone, Serialize)]
pub struct FocusNote {
    pub id: String,
    pub profile_id: String,
    pub body: String,
    pub work_id: Option<String>,
    pub position: i64,
    pub pinned_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewFocusNote {
    pub body: String,
    #[serde(default)]
    pub work_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FocusNotePatch {
    pub body: Option<String>,
    pub work_id: Option<Option<String>>,
    /// Whether it is kept at the top. `Some(true)` stamps the moment and
    /// `Some(false)` clears it; leaving it out changes nothing.
    pub pinned: Option<bool>,
}

/// The gap left between positions, so a note can be dropped between two others
/// without renumbering the board.
const STEP: i64 = 1024;

const SELECT_NOTE: &str = "SELECT id, profile_id, body, work_id, position, pinned_at, created_at, \
                           updated_at FROM focus_note";

/// Dismiss a complaint, or refresh the moment it was dismissed.
///
/// Dismissing the same complaint twice is not an error: the board can only
/// offer the button while the complaint stands, and a second press is the
/// person saying the same thing again.
pub fn dismiss(conn: &Connection, profile_id: &str, key: &DismissalKey) -> Result<Dismissal> {
    let dismissed_at = now();

    conn.execute(
        "INSERT INTO focus_dismissal (id, profile_id, kind, work_id, complaint, dismissed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (profile_id, kind, work_id, complaint)
         DO UPDATE SET dismissed_at = excluded.dismissed_at",
        params![
            uuid::Uuid::new_v4().to_string(),
            profile_id,
            key.kind,
            key.work_id,
            key.complaint,
            dismissed_at,
        ],
    )?;

    Ok(Dismissal {
        kind: key.kind.clone(),
        work_id: key.work_id.clone(),
        complaint: key.complaint.clone(),
        dismissed_at,
    })
}

/// Bring a dismissed complaint back.
///
/// Removing a dismissal that is not there is not an error: undoing something
/// already visible leaves the board exactly as the person wants it.
pub fn restore(conn: &Connection, profile_id: &str, key: &DismissalKey) -> Result<()> {
    conn.execute(
        "DELETE FROM focus_dismissal
         WHERE profile_id = ?1 AND kind = ?2 AND work_id = ?3 AND complaint = ?4",
        params![profile_id, key.kind, key.work_id, key.complaint],
    )?;
    Ok(())
}

/// Every complaint currently put away in this profile.
pub fn dismissals(conn: &Connection, profile_id: &str) -> Result<Vec<Dismissal>> {
    let mut statement = conn.prepare(
        "SELECT kind, work_id, complaint, dismissed_at FROM focus_dismissal
         WHERE profile_id = ?1
         ORDER BY dismissed_at DESC, rowid DESC",
    )?;

    let rows = statement.query_map(params![profile_id], |row| {
        Ok(Dismissal {
            kind: row.get(0)?,
            work_id: row.get(1)?,
            complaint: row.get(2)?,
            dismissed_at: row.get(3)?,
        })
    })?;

    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Forget dismissals for works that no longer exist.
///
/// The table holds no foreign key on purpose — a dismissal is a decision, not a
/// fact about a work — so nothing removes these when a work is deleted. Left
/// alone they are harmless, since a complaint about a missing work is never
/// raised again, but they accumulate for as long as the workspace lives.
pub fn sweep(conn: &Connection) -> Result<usize> {
    Ok(conn.execute(
        "DELETE FROM focus_dismissal WHERE work_id NOT IN (SELECT id FROM work)",
        [],
    )?)
}

/// Add a note at the end of the board.
pub fn add_note(conn: &Connection, profile_id: &str, new: NewFocusNote) -> Result<FocusNote> {
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = now();

    let last: Option<i64> = conn.query_row(
        "SELECT max(position) FROM focus_note WHERE profile_id = ?1",
        params![profile_id],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT INTO focus_note (id, profile_id, body, work_id, position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            id,
            profile_id,
            new.body,
            new.work_id,
            last.unwrap_or(0) + STEP,
            timestamp,
        ],
    )?;

    get_note(conn, &id)?.ok_or_else(|| Error::Other("the board note vanished after insert".into()))
}

pub fn get_note(conn: &Connection, id: &str) -> Result<Option<FocusNote>> {
    Ok(conn
        .query_row(
            &format!("{SELECT_NOTE} WHERE id = ?1"),
            params![id],
            read_note,
        )
        .optional()?)
}

/// The board's notes: pinned first, then in the order they were arranged.
pub fn notes(conn: &Connection, profile_id: &str) -> Result<Vec<FocusNote>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT_NOTE} WHERE profile_id = ?1
         ORDER BY pinned_at IS NULL, pinned_at, position, rowid"
    ))?;

    let rows = statement.query_map(params![profile_id], read_note)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn update_note(conn: &Connection, id: &str, patch: FocusNotePatch) -> Result<FocusNote> {
    let mut assignments: Vec<String> = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    fn set(
        assignments: &mut Vec<String>,
        values: &mut Vec<Box<dyn rusqlite::ToSql>>,
        column: &str,
        value: Box<dyn rusqlite::ToSql>,
    ) {
        values.push(value);
        assignments.push(format!("{column} = ?{}", values.len()));
    }

    if let Some(body) = patch.body {
        set(&mut assignments, &mut values, "body", Box::new(body));
    }
    if let Some(work_id) = patch.work_id {
        set(&mut assignments, &mut values, "work_id", Box::new(work_id));
    }
    if let Some(pinned) = patch.pinned {
        set(
            &mut assignments,
            &mut values,
            "pinned_at",
            Box::new(pinned.then(now)),
        );
    }

    if assignments.is_empty() {
        return get_note(conn, id)?.ok_or_else(|| unknown_note(id));
    }

    set(&mut assignments, &mut values, "updated_at", Box::new(now()));
    values.push(Box::new(id.to_owned()));

    let sql = format!(
        "UPDATE focus_note SET {} WHERE id = ?{}",
        assignments.join(", "),
        values.len()
    );
    let params = rusqlite::params_from_iter(values.iter().map(AsRef::as_ref));

    if conn.execute(&sql, params)? == 0 {
        return Err(unknown_note(id));
    }

    get_note(conn, id)?.ok_or_else(|| unknown_note(id))
}

/// Put the board's notes in the order given.
///
/// The caller sends the whole arrangement it is showing rather than a pair of
/// neighbours: a board is short, and a full list cannot disagree with itself
/// the way "after that one" can when two moves race. Ids belonging to another
/// profile are ignored, and any note left out of the list keeps a position
/// after the ones named.
pub fn reorder_notes(conn: &mut Connection, profile_id: &str, order: &[String]) -> Result<()> {
    let transaction = conn.transaction()?;

    for (index, id) in order.iter().enumerate() {
        let position = i64::try_from(index).unwrap_or(0) * STEP + STEP;
        transaction.execute(
            "UPDATE focus_note SET position = ?1 WHERE id = ?2 AND profile_id = ?3",
            params![position, id, profile_id],
        )?;
    }

    transaction.commit()?;
    Ok(())
}

pub fn delete_note(conn: &Connection, id: &str) -> Result<()> {
    if conn.execute("DELETE FROM focus_note WHERE id = ?1", params![id])? == 0 {
        return Err(unknown_note(id));
    }
    Ok(())
}

fn unknown_note(id: &str) -> Error {
    Error::not_found("focus note", id)
}

fn read_note(row: &rusqlite::Row<'_>) -> rusqlite::Result<FocusNote> {
    Ok(FocusNote {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        body: row.get(2)?,
        work_id: row.get(3)?,
        position: row.get(4)?,
        pinned_at: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::work::{self, NewWork};

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    fn a_work(conn: &Connection, profile_id: &str, title: &str) -> String {
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

    fn key(kind: &str, work_id: &str, complaint: &str) -> DismissalKey {
        DismissalKey {
            kind: kind.into(),
            work_id: work_id.into(),
            complaint: complaint.into(),
        }
    }

    #[test]
    fn a_dismissed_complaint_is_remembered() {
        let (conn, profile_id) = workspace();

        dismiss(
            &conn,
            &profile_id,
            &key("stale-draft", "w1", "stale-draft:1"),
        )
        .unwrap();
        let hidden = dismissals(&conn, &profile_id).unwrap();

        assert_eq!(hidden.len(), 1);
        assert_eq!(hidden[0].complaint, "stale-draft:1");
    }

    /// The point of the whole stage: hiding is answering *this* complaint, not
    /// silencing the work. A draft that has now sat four months is a different
    /// thing to hear than one that had sat one, and it has to come back.
    #[test]
    fn a_changed_complaint_is_news_again() {
        let (conn, profile_id) = workspace();

        dismiss(
            &conn,
            &profile_id,
            &key("stale-draft", "w1", "stale-draft:1"),
        )
        .unwrap();
        let hidden = dismissals(&conn, &profile_id).unwrap();

        assert!(
            !hidden
                .iter()
                .any(|row| row.complaint == "stale-draft:4" && row.work_id == "w1"),
            "a longer silence is a different complaint and must be raised again"
        );
    }

    #[test]
    fn dismissing_the_same_complaint_twice_keeps_one_row() {
        let (conn, profile_id) = workspace();

        dismiss(&conn, &profile_id, &key("unscored", "w1", "unscored")).unwrap();
        dismiss(&conn, &profile_id, &key("unscored", "w1", "unscored")).unwrap();

        assert_eq!(dismissals(&conn, &profile_id).unwrap().len(), 1);
    }

    #[test]
    fn restoring_brings_a_complaint_back() {
        let (conn, profile_id) = workspace();
        let subject = key("unscored", "w1", "unscored");

        dismiss(&conn, &profile_id, &subject).unwrap();
        restore(&conn, &profile_id, &subject).unwrap();

        assert!(dismissals(&conn, &profile_id).unwrap().is_empty());
    }

    /// Restoring something that is not hidden is a no-op, not a failure: the
    /// board would otherwise raise an error for asking to see what it is
    /// already showing.
    #[test]
    fn restoring_what_was_never_hidden_is_not_an_error() {
        let (conn, profile_id) = workspace();

        restore(&conn, &profile_id, &key("unscored", "w1", "unscored")).unwrap();
    }

    #[test]
    fn a_dismissal_for_a_deleted_work_is_swept() {
        let (conn, profile_id) = workspace();
        let kept = a_work(&conn, &profile_id, "Harbour lights");
        let going = a_work(&conn, &profile_id, "Winter road");

        dismiss(&conn, &profile_id, &key("unscored", &kept, "unscored")).unwrap();
        dismiss(&conn, &profile_id, &key("unscored", &going, "unscored")).unwrap();
        work::delete(&conn, &going).unwrap();

        assert_eq!(sweep(&conn).unwrap(), 1);
        let left = dismissals(&conn, &profile_id).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].work_id, kept);
    }

    #[test]
    fn a_new_note_lands_at_the_end() {
        let (conn, profile_id) = workspace();

        add_note(&conn, &profile_id, note("first")).unwrap();
        add_note(&conn, &profile_id, note("second")).unwrap();

        let board = notes(&conn, &profile_id).unwrap();
        assert_eq!(
            board.iter().map(|n| n.body.as_str()).collect::<Vec<_>>(),
            vec!["first", "second"]
        );
    }

    #[test]
    fn a_pinned_note_rises_to_the_top() {
        let (conn, profile_id) = workspace();
        add_note(&conn, &profile_id, note("first")).unwrap();
        let second = add_note(&conn, &profile_id, note("second")).unwrap();

        update_note(
            &conn,
            &second.id,
            FocusNotePatch {
                pinned: Some(true),
                ..Default::default()
            },
        )
        .unwrap();

        let board = notes(&conn, &profile_id).unwrap();
        assert_eq!(board[0].body, "second");
        assert!(board[0].pinned_at.is_some());
    }

    #[test]
    fn unpinning_returns_a_note_to_its_place() {
        let (conn, profile_id) = workspace();
        let first = add_note(&conn, &profile_id, note("first")).unwrap();
        add_note(&conn, &profile_id, note("second")).unwrap();

        let pin = |pinned| FocusNotePatch {
            pinned: Some(pinned),
            ..Default::default()
        };
        update_note(&conn, &first.id, pin(true)).unwrap();
        update_note(&conn, &first.id, pin(false)).unwrap();

        let board = notes(&conn, &profile_id).unwrap();
        assert_eq!(board[0].body, "first");
        assert!(board[0].pinned_at.is_none());
    }

    #[test]
    fn reordering_rearranges_the_board() {
        let (mut conn, profile_id) = workspace();
        let first = add_note(&conn, &profile_id, note("first")).unwrap();
        let second = add_note(&conn, &profile_id, note("second")).unwrap();
        let third = add_note(&conn, &profile_id, note("third")).unwrap();

        reorder_notes(
            &mut conn,
            &profile_id,
            &[third.id.clone(), first.id.clone(), second.id.clone()],
        )
        .unwrap();

        let board = notes(&conn, &profile_id).unwrap();
        assert_eq!(
            board.iter().map(|n| n.body.as_str()).collect::<Vec<_>>(),
            vec!["third", "first", "second"]
        );
    }

    /// A reorder names only what the board is showing. Anything filtered out of
    /// that view has to keep its place rather than being scattered through it.
    #[test]
    fn reordering_leaves_notes_it_does_not_name_after_the_rest() {
        let (mut conn, profile_id) = workspace();
        let first = add_note(&conn, &profile_id, note("first")).unwrap();
        let second = add_note(&conn, &profile_id, note("second")).unwrap();
        add_note(&conn, &profile_id, note("unnamed")).unwrap();

        reorder_notes(
            &mut conn,
            &profile_id,
            &[second.id.clone(), first.id.clone()],
        )
        .unwrap();

        let board = notes(&conn, &profile_id).unwrap();
        assert_eq!(
            board.iter().map(|n| n.body.as_str()).collect::<Vec<_>>(),
            vec!["second", "first", "unnamed"]
        );
    }

    /// A board belongs to one profile, and a reorder names ids the *showing*
    /// board holds. Without the profile in the WHERE clause an id from another
    /// craft would be renumbered by a board that cannot even see it — the kind
    /// of damage nothing on screen would explain.
    #[test]
    fn reordering_leaves_another_profiles_board_alone() {
        let (mut conn, profile_id) = workspace();
        let other = profile::list(&conn)
            .unwrap()
            .into_iter()
            .find(|p| p.id != profile_id)
            .expect("the seed ships more than one profile");

        let mine = add_note(&conn, &profile_id, note("mine")).unwrap();
        // Two on the other board, so the one named sits at a position the
        // reorder would not hand it by coincidence: a single note would land
        // back on 1024 either way and the check would pass on a broken query.
        add_note(&conn, &other.id, note("theirs first")).unwrap();
        let theirs = add_note(&conn, &other.id, note("theirs second")).unwrap();
        let before = theirs.position;

        reorder_notes(
            &mut conn,
            &profile_id,
            &[theirs.id.clone(), mine.id.clone()],
        )
        .unwrap();

        let after = get_note(&conn, &theirs.id).unwrap().unwrap();
        assert_eq!(
            after.position, before,
            "a reorder must not move a note on another profile's board"
        );
    }

    #[test]
    fn deleting_an_unknown_note_says_so() {
        let (conn, _) = workspace();

        assert!(delete_note(&conn, "nothing").is_err());
    }

    fn note(body: &str) -> NewFocusNote {
        NewFocusNote {
            body: body.into(),
            work_id: None,
        }
    }
}
