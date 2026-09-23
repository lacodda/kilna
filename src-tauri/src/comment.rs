//! What the audience said, and the reply to it.
//!
//! One table for every channel a work went out through (migration 0027): the
//! channel is the word the person uses for it, the work is a key, and where a
//! comment stands is one of three states. A reply is drafted here — by hand or
//! by the assistant — and always posted by hand: kilna does not speak for
//! anyone on their channel.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::minted::Minted;

/// Waiting for the person.
pub const OPEN: &str = "open";
/// Answered where it was written.
pub const POSTED: &str = "posted";
/// Needs nothing.
pub const ARCHIVED: &str = "archived";

/// Every state a comment can be in, in the order a person meets them.
pub const STATES: [&str; 3] = [OPEN, POSTED, ARCHIVED];

#[derive(Debug, Clone, Serialize)]
pub struct Comment {
    pub id: String,
    pub profile_id: String,
    pub channel: String,
    pub work_id: Option<String>,
    pub author: Option<String>,
    pub body: String,
    pub reply: Option<String>,
    pub state: String,
    /// The day the viewer wrote it, `YYYY-MM-DD`.
    pub commented_on: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewComment {
    pub channel: String,
    pub body: String,
    #[serde(default)]
    pub work_id: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub reply: Option<String>,
    #[serde(default)]
    pub commented_on: Option<String>,
}

/// Fields that may be changed; `None` leaves one alone, `Some(None)` clears a
/// field that can be empty.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommentPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub work_id: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub author: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub reply: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub commented_on: Option<Option<String>>,
}

/// Narrowing applied to a listing.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CommentFilter {
    pub work_id: Option<String>,
    pub channel: Option<String>,
    /// One state. Absent means everything that still asks for something or
    /// was answered — every state but `archived`, which is the inbox's own
    /// "done with it" and stays out unless asked for.
    pub state: Option<String>,
    /// Case-insensitive substring of the text, the reply or the author.
    pub search: Option<String>,
}

const SELECT_COMMENT: &str = "SELECT id, profile_id, channel, work_id, author, body, reply, state, \
     commented_on, created_at, updated_at FROM comment";

/// Add a comment with the id and timestamp already decided.
///
/// The seam a replay comes back through, as every creation is (ADR 0014).
pub fn create_minted(
    conn: &Connection,
    profile_id: &str,
    new: NewComment,
    minted: Minted,
) -> Result<Comment> {
    let channel = required_text(&new.channel, "channel")?;
    let body = required_text(&new.body, "text")?;
    let commented_on = day(new.commented_on.as_deref())?;
    if let Some(work_id) = &new.work_id {
        in_profile(conn, profile_id, work_id)?;
    }

    conn.execute(
        "INSERT INTO comment (id, profile_id, channel, work_id, author, body, reply, state,
                              commented_on, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![
            minted.id(),
            profile_id,
            channel,
            new.work_id,
            optional_text(new.author.as_deref()),
            body,
            optional_text(new.reply.as_deref()),
            OPEN,
            commented_on,
            minted.at(),
        ],
    )?;

    get(conn, minted.id())?.ok_or_else(|| Error::Other("the comment vanished after insert".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Comment>> {
    Ok(conn
        .query_row(
            &format!("{SELECT_COMMENT} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?)
}

/// A profile's comments, newest first.
///
/// Newest by when the person kept it rather than when the viewer wrote it:
/// the inbox is read in the order things arrived in it, and a comment from
/// last year kept today is today's errand.
pub fn list(conn: &Connection, profile_id: &str, filter: &CommentFilter) -> Result<Vec<Comment>> {
    let mut sql = format!("{SELECT_COMMENT} WHERE profile_id = ?1");
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(profile_id.to_owned())];

    if let Some(work_id) = &filter.work_id {
        values.push(Box::new(work_id.clone()));
        sql.push_str(&format!(" AND work_id = ?{}", values.len()));
    }
    if let Some(channel) = &filter.channel {
        values.push(Box::new(channel.clone()));
        sql.push_str(&format!(" AND channel = ?{}", values.len()));
    }
    match &filter.state {
        Some(state) => {
            values.push(Box::new(state.clone()));
            sql.push_str(&format!(" AND state = ?{}", values.len()));
        }
        None => sql.push_str(&format!(" AND state <> '{ARCHIVED}'")),
    }
    if let Some(search) = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        values.push(Box::new(format!("%{search}%")));
        let n = values.len();
        sql.push_str(&format!(
            " AND (body LIKE ?{n} OR coalesce(reply, '') LIKE ?{n} OR coalesce(author, '') LIKE ?{n})"
        ));
    }

    sql.push_str(" ORDER BY created_at DESC, rowid DESC");

    let mut statement = conn.prepare(&sql)?;
    let params = rusqlite::params_from_iter(values.iter().map(AsRef::as_ref));
    let rows = statement
        .query_map(params, read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Update a comment with the change's moment already decided (ADR 0014).
pub fn update_at(conn: &Connection, id: &str, patch: CommentPatch, at: &str) -> Result<Comment> {
    let current = get(conn, id)?.ok_or_else(|| unknown(id))?;

    let mut assignments: Vec<String> = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    fn put(
        assignments: &mut Vec<String>,
        values: &mut Vec<Box<dyn rusqlite::ToSql>>,
        column: &str,
        value: Box<dyn rusqlite::ToSql>,
    ) {
        values.push(value);
        assignments.push(format!("{column} = ?{}", values.len()));
    }

    if let Some(channel) = patch.channel {
        put(
            &mut assignments,
            &mut values,
            "channel",
            Box::new(required_text(&channel, "channel")?),
        );
    }
    if let Some(work_id) = patch.work_id {
        if let Some(work_id) = &work_id {
            in_profile(conn, &current.profile_id, work_id)?;
        }
        put(&mut assignments, &mut values, "work_id", Box::new(work_id));
    }
    if let Some(author) = patch.author {
        put(
            &mut assignments,
            &mut values,
            "author",
            Box::new(optional_text(author.as_deref())),
        );
    }
    if let Some(body) = patch.body {
        put(
            &mut assignments,
            &mut values,
            "body",
            Box::new(required_text(&body, "text")?),
        );
    }
    if let Some(reply) = patch.reply {
        put(
            &mut assignments,
            &mut values,
            "reply",
            Box::new(optional_text(reply.as_deref())),
        );
    }
    if let Some(state) = patch.state {
        if !STATES.contains(&state.as_str()) {
            return Err(Error::Other(format!(
                "`{state}` is not a state a comment can be in"
            )));
        }
        put(&mut assignments, &mut values, "state", Box::new(state));
    }
    if let Some(commented_on) = patch.commented_on {
        put(
            &mut assignments,
            &mut values,
            "commented_on",
            Box::new(day(commented_on.as_deref())?),
        );
    }

    if assignments.is_empty() {
        return Ok(current);
    }
    put(
        &mut assignments,
        &mut values,
        "updated_at",
        Box::new(at.to_owned()),
    );
    values.push(Box::new(id.to_owned()));

    let sql = format!(
        "UPDATE comment SET {} WHERE id = ?{}",
        assignments.join(", "),
        values.len()
    );
    conn.execute(
        &sql,
        rusqlite::params_from_iter(values.iter().map(AsRef::as_ref)),
    )?;

    get(conn, id)?.ok_or_else(|| unknown(id))
}

/// Every channel the profile's comments came through, with how many of them
/// still wait — the chips of the inbox. Channels whose comments are all dealt
/// with stay in the list at zero: they are still where comments come from,
/// and the next one is filed under the same word rather than a near-miss.
pub fn channels(conn: &Connection, profile_id: &str) -> Result<Vec<(String, i64)>> {
    let mut statement = conn.prepare(
        "SELECT channel, sum(state = 'open') AS waiting
           FROM comment
          WHERE profile_id = ?1
          GROUP BY channel
          ORDER BY waiting DESC, count(*) DESC, channel",
    )?;
    let rows = statement
        .query_map(params![profile_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// How many comments a work has that are not archived, and how many of those
/// still wait: the counter on its card.
pub fn count_for_work(conn: &Connection, work_id: &str) -> Result<(i64, i64)> {
    Ok(conn.query_row(
        "SELECT count(*), coalesce(sum(state = 'open'), 0)
           FROM comment WHERE work_id = ?1 AND state <> 'archived'",
        params![work_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?)
}

/// The replies already posted on a channel, newest first: what "the voice of
/// the channel" is made of when a reply is drafted. The voice is read off
/// what was said there rather than kept as a setting, because the channel is
/// only a word and a setting would need somewhere to live.
pub fn posted_on(
    conn: &Connection,
    profile_id: &str,
    channel: &str,
    except: &str,
    limit: usize,
) -> Result<Vec<(String, String)>> {
    let mut statement = conn.prepare(
        "SELECT body, reply FROM comment
          WHERE profile_id = ?1 AND channel = ?2 AND id <> ?3
            AND state = 'posted' AND coalesce(trim(reply), '') <> ''
          ORDER BY updated_at DESC, rowid DESC
          LIMIT ?4",
    )?;
    let rows = statement
        .query_map(params![profile_id, channel, except, limit as i64], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// A field that must say something, trimmed.
fn required_text(value: &str, what: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::Other(format!("a comment needs its {what}")));
    }
    Ok(trimmed.to_owned())
}

/// A field that may be empty: blank is stored as nothing, so "no author" has
/// one spelling.
fn optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// A day as `YYYY-MM-DD`, checked to be a real one. Blank is no day.
fn day(value: Option<&str>) -> Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    if !is_day(value) {
        return Err(Error::Other(format!(
            "`{value}` is not a day; write it as YYYY-MM-DD"
        )));
    }
    Ok(Some(value.to_owned()))
}

/// Whether a string is a real day written `YYYY-MM-DD` — the 30th of
/// February is not one. Public because the answer that reads a comment off a
/// screenshot is held to the same rule as a comment typed in.
pub fn is_day(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return false;
    };
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) =
        (year.parse::<i32>(), month.parse::<u8>(), day.parse::<u8>())
    else {
        return false;
    };
    ::time::Month::try_from(month)
        .ok()
        .and_then(|month| ::time::Date::from_calendar_date(year, month, day).ok())
        .is_some()
}

/// Refuse a work of another profile: a comment filed there would be invisible
/// from both sides.
fn in_profile(conn: &Connection, profile_id: &str, work_id: &str) -> Result<()> {
    let found: Option<String> = conn
        .query_row(
            "SELECT profile_id FROM work WHERE id = ?1",
            params![work_id],
            |row| row.get(0),
        )
        .optional()?;
    match found {
        Some(owner) if owner == profile_id => Ok(()),
        _ => Err(Error::not_found("work", work_id)),
    }
}

fn unknown(id: &str) -> Error {
    Error::not_found("comment", id)
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Comment> {
    Ok(Comment {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        channel: row.get(2)?,
        work_id: row.get(3)?,
        author: row.get(4)?,
        body: row.get(5)?,
        reply: row.get(6)?,
        state: row.get(7)?,
        commented_on: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
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

    fn song(conn: &Connection, profile_id: &str) -> String {
        work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id
    }

    fn said(channel: &str, body: &str) -> NewComment {
        NewComment {
            channel: channel.into(),
            body: body.into(),
            ..NewComment::default()
        }
    }

    fn keep(conn: &Connection, profile_id: &str, new: NewComment) -> Result<Comment> {
        create_minted(conn, profile_id, new, Minted::fresh())
    }

    #[test]
    fn a_comment_arrives_open_and_trimmed() {
        let (conn, profile_id) = workspace();

        let kept = keep(
            &conn,
            &profile_id,
            NewComment {
                author: Some("  ".into()),
                ..said(" main ", "  loved the bridge ")
            },
        )
        .unwrap();

        assert_eq!(kept.state, OPEN);
        assert_eq!(kept.channel, "main");
        assert_eq!(kept.body, "loved the bridge");
        assert_eq!(kept.author, None, "a blank author is no author");
    }

    #[test]
    fn a_comment_without_words_or_a_channel_is_refused() {
        let (conn, profile_id) = workspace();

        assert!(keep(&conn, &profile_id, said("main", "   ")).is_err());
        assert!(keep(&conn, &profile_id, said("  ", "hello")).is_err());
    }

    #[test]
    fn the_day_must_be_a_real_day() {
        let (conn, profile_id) = workspace();
        let with_day = |day: &str| NewComment {
            commented_on: Some(day.into()),
            ..said("main", "hi")
        };

        assert!(keep(&conn, &profile_id, with_day("2026-02-30")).is_err());
        assert!(keep(&conn, &profile_id, with_day("3 weeks ago")).is_err());
        let kept = keep(&conn, &profile_id, with_day("2026-09-01")).unwrap();
        assert_eq!(kept.commented_on.as_deref(), Some("2026-09-01"));
    }

    #[test]
    fn an_archived_comment_stays_out_of_the_inbox_until_asked_for() {
        let (conn, profile_id) = workspace();
        let first = keep(&conn, &profile_id, said("main", "one")).unwrap();
        keep(&conn, &profile_id, said("main", "two")).unwrap();
        update_at(
            &conn,
            &first.id,
            CommentPatch {
                state: Some(ARCHIVED.into()),
                ..CommentPatch::default()
            },
            "2026-09-22T00:00:00Z",
        )
        .unwrap();

        let inbox = list(&conn, &profile_id, &CommentFilter::default()).unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].body, "two");

        let archived = list(
            &conn,
            &profile_id,
            &CommentFilter {
                state: Some(ARCHIVED.into()),
                ..CommentFilter::default()
            },
        )
        .unwrap();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].body, "one");
    }

    #[test]
    fn a_state_outside_the_three_is_refused() {
        let (conn, profile_id) = workspace();
        let kept = keep(&conn, &profile_id, said("main", "hi")).unwrap();

        let refused = update_at(
            &conn,
            &kept.id,
            CommentPatch {
                state: Some("answered".into()),
                ..CommentPatch::default()
            },
            "2026-09-22T00:00:00Z",
        );

        assert!(
            refused.is_err(),
            "a drafted reply is read off `reply`, not stored"
        );
    }

    #[test]
    fn channels_are_the_words_already_used_with_what_waits_on_each() {
        let (conn, profile_id) = workspace();
        keep(&conn, &profile_id, said("main", "a")).unwrap();
        keep(&conn, &profile_id, said("main", "b")).unwrap();
        let done = keep(&conn, &profile_id, said("second", "c")).unwrap();
        update_at(
            &conn,
            &done.id,
            CommentPatch {
                state: Some(POSTED.into()),
                ..CommentPatch::default()
            },
            "2026-09-22T00:00:00Z",
        )
        .unwrap();

        let channels = channels(&conn, &profile_id).unwrap();

        assert_eq!(
            channels,
            vec![("main".to_owned(), 2), ("second".to_owned(), 0)],
            "a channel with nothing waiting is still a channel"
        );
    }

    #[test]
    fn a_work_counts_its_comments_and_what_still_waits() {
        let (conn, profile_id) = workspace();
        let work_id = song(&conn, &profile_id);
        let about = |body: &str| NewComment {
            work_id: Some(work_id.clone()),
            ..said("main", body)
        };
        keep(&conn, &profile_id, about("one")).unwrap();
        let answered = keep(&conn, &profile_id, about("two")).unwrap();
        let archived = keep(&conn, &profile_id, about("three")).unwrap();
        keep(&conn, &profile_id, said("main", "about nothing")).unwrap();
        for (id, state) in [(&answered.id, POSTED), (&archived.id, ARCHIVED)] {
            update_at(
                &conn,
                id,
                CommentPatch {
                    state: Some(state.into()),
                    ..CommentPatch::default()
                },
                "2026-09-22T00:00:00Z",
            )
            .unwrap();
        }

        assert_eq!(count_for_work(&conn, &work_id).unwrap(), (2, 1));
    }

    #[test]
    fn the_voice_of_a_channel_is_what_was_posted_there() {
        let (conn, profile_id) = workspace();
        let post = |channel: &str, body: &str, reply: &str| {
            let kept = keep(&conn, &profile_id, said(channel, body)).unwrap();
            update_at(
                &conn,
                &kept.id,
                CommentPatch {
                    reply: Some(Some(reply.into())),
                    state: Some(POSTED.into()),
                    ..CommentPatch::default()
                },
                "2026-09-22T00:00:00Z",
            )
            .unwrap();
            kept
        };
        post("main", "q1", "thank you!");
        post("second", "q2", "cheers");
        let drafted = keep(
            &conn,
            &profile_id,
            NewComment {
                reply: Some("not sent yet".into()),
                ..said("main", "q3")
            },
        )
        .unwrap();

        let voice = posted_on(&conn, &profile_id, "main", &drafted.id, 10).unwrap();

        assert_eq!(voice, vec![("q1".to_owned(), "thank you!".to_owned())]);
    }

    #[test]
    fn a_work_of_another_profile_is_refused() {
        let (conn, profile_id) = workspace();

        let refused = keep(
            &conn,
            &profile_id,
            NewComment {
                work_id: Some("somewhere-else".into()),
                ..said("main", "hi")
            },
        );

        assert!(refused.is_err());
    }

    #[test]
    fn a_comment_goes_to_the_trash_with_its_work_and_comes_back_with_it() {
        let (mut conn, profile_id) = workspace();
        let work_id = song(&conn, &profile_id);
        let kept = keep(
            &conn,
            &profile_id,
            NewComment {
                work_id: Some(work_id.clone()),
                ..said("main", "loved it")
            },
        )
        .unwrap();

        let entry = crate::trash::discard(&mut conn, crate::trash::Entity::Work, &work_id).unwrap();
        assert!(get(&conn, &kept.id).unwrap().is_none());

        crate::trash::restore(&mut conn, &entry, None).unwrap();
        assert_eq!(get(&conn, &kept.id).unwrap().unwrap().body, "loved it");
    }
}
