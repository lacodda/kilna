//! Days the calendar is closed: a holiday, a fortnight away.
//!
//! A span of days with a name, both ends inside it. It closes the days to the
//! auto-layout - a plan never lands on them - and not to a person: a release
//! put on a closed day lands, with the closure said first (see
//! [`crate::release::preview`]). Spans may overlap; "New Year" inside a
//! fortnight away is two facts, and both are true.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use time::{Date, macros::format_description};

use crate::error::{Error, Result};
use crate::minted::Minted;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct TimeOff {
    pub id: String,
    pub profile_id: String,
    /// The first closed day, `yyyy-mm-dd`.
    pub starts_on: String,
    /// The last closed day; the same as `starts_on` for one day.
    pub ends_on: String,
    /// What the days are: "Away", "New Year". Never empty: the month shows
    /// it on the days it closes.
    pub label: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct NewTimeOff {
    pub starts_on: String,
    pub ends_on: String,
    pub label: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct TimeOffPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starts_on: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_on: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

const SELECT: &str =
    "SELECT id, profile_id, starts_on, ends_on, label, created_at, updated_at FROM time_off";

/// Close a span of days.
pub fn create_minted(
    conn: &Connection,
    profile_id: &str,
    new: NewTimeOff,
    minted: Minted,
) -> Result<TimeOff> {
    check_span(&new.starts_on, &new.ends_on)?;
    let label = named(&new.label)?;
    conn.execute(
        "INSERT INTO time_off (id, profile_id, starts_on, ends_on, label, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            minted.id(),
            profile_id,
            new.starts_on,
            new.ends_on,
            label,
            minted.at(),
        ],
    )?;
    get(conn, minted.id())?
        .ok_or_else(|| Error::Internal("the time off vanished after insert".into()))
}

/// Move a span's ends or rename it. A patch that changes nothing writes
/// nothing, so it leaves no trace for synchronisation to settle.
pub fn update_at(conn: &Connection, id: &str, patch: TimeOffPatch, at: &str) -> Result<TimeOff> {
    let found = get(conn, id)?.ok_or_else(|| unknown(id))?;
    let starts_on = patch.starts_on.unwrap_or_else(|| found.starts_on.clone());
    let ends_on = patch.ends_on.unwrap_or_else(|| found.ends_on.clone());
    let label = match patch.label {
        Some(label) => named(&label)?,
        None => found.label.clone(),
    };
    check_span(&starts_on, &ends_on)?;
    if starts_on == found.starts_on && ends_on == found.ends_on && label == found.label {
        return Ok(found);
    }
    conn.execute(
        "UPDATE time_off SET starts_on = ?2, ends_on = ?3, label = ?4, updated_at = ?5 WHERE id = ?1",
        params![id, starts_on, ends_on, label, at],
    )?;
    get(conn, id)?.ok_or_else(|| unknown(id))
}

/// Open the days again. Nothing hangs off a span, so the undo puts the same
/// row back from the copy the operation recorded, as a relation between
/// cards comes back.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    if conn.execute("DELETE FROM time_off WHERE id = ?1", params![id])? == 0 {
        return Err(unknown(id));
    }
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<TimeOff>> {
    Ok(conn
        .query_row(&format!("{SELECT} WHERE id = ?1"), params![id], read)
        .optional()?)
}

/// Every span of a profile, earliest first.
pub fn list(conn: &Connection, profile_id: &str) -> Result<Vec<TimeOff>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} WHERE profile_id = ?1 ORDER BY starts_on, ends_on, id"
    ))?;
    let rows = statement
        .query_map(params![profile_id], read)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// The span that closes `day`, if one does: the earliest to start, when
/// several overlap, so the same day always names the same span.
pub fn covering(conn: &Connection, profile_id: &str, day: &str) -> Result<Option<TimeOff>> {
    Ok(conn
        .query_row(
            &format!(
                "{SELECT} WHERE profile_id = ?1 AND starts_on <= ?2 AND ends_on >= ?2
                 ORDER BY starts_on, ends_on, id LIMIT 1"
            ),
            params![profile_id, day],
            read,
        )
        .optional()?)
}

/// The closed days of a profile as spans of dates, for a scan that asks
/// about many days and should not ask the database about each.
pub fn spans(conn: &Connection, profile_id: &str) -> Result<Vec<Span>> {
    list(conn, profile_id)?
        .iter()
        .map(|entry| {
            Ok(Span {
                first: parse(&entry.starts_on)?,
                last: parse(&entry.ends_on)?,
            })
        })
        .collect()
}

/// A run of closed days, both ends inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub first: Date,
    pub last: Date,
}

impl Span {
    pub fn contains(&self, day: Date) -> bool {
        self.first <= day && day <= self.last
    }

    /// How many days it closes.
    pub fn days(&self) -> i64 {
        (self.last - self.first).whole_days() + 1
    }
}

/// Whether any of `spans` closes `day`.
pub fn closes(spans: &[Span], day: Date) -> bool {
    spans.iter().any(|span| span.contains(day))
}

/// Both ends must be days that exist, the first no later than the last.
fn check_span(starts_on: &str, ends_on: &str) -> Result<()> {
    for day in [starts_on, ends_on] {
        if !crate::time::is_date(day) {
            return Err(Error::refused("timeOff.badDay").param("value", day));
        }
    }
    if starts_on > ends_on {
        return Err(Error::refused("timeOff.endsBeforeItStarts")
            .param("from", starts_on)
            .param("to", ends_on));
    }
    Ok(())
}

fn parse(day: &str) -> Result<Date> {
    Date::parse(day, format_description!("[year]-[month]-[day]"))
        .map_err(|_| Error::refused("timeOff.badDay").param("value", day))
}

/// A name, trimmed; an empty one is refused rather than stored, since the
/// month shows it on every day the span closes.
fn named(label: &str) -> Result<String> {
    let label = label.trim();
    if label.is_empty() {
        return Err(Error::refused("timeOff.unnamed"));
    }
    Ok(label.to_owned())
}

fn unknown(id: &str) -> Error {
    Error::not_found("time off", id)
}

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<TimeOff> {
    Ok(TimeOff {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        starts_on: row.get(2)?,
        ends_on: row.get(3)?,
        label: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    fn away(conn: &Connection, profile_id: &str, from: &str, to: &str, label: &str) -> TimeOff {
        create_minted(
            conn,
            profile_id,
            NewTimeOff {
                starts_on: from.into(),
                ends_on: to.into(),
                label: label.into(),
            },
            Minted::fresh(),
        )
        .unwrap()
    }

    #[test]
    fn a_span_closes_both_of_its_ends_and_the_days_between() {
        let (conn, profile_id) = fixtures::workspace();
        away(&conn, &profile_id, "2026-10-20", "2026-10-27", " Away ");

        for day in ["2026-10-20", "2026-10-23", "2026-10-27"] {
            let found = covering(&conn, &profile_id, day).unwrap();
            assert_eq!(
                found.map(|span| span.label).as_deref(),
                Some("Away"),
                "{day}"
            );
        }
        for day in ["2026-10-19", "2026-10-28"] {
            assert!(
                covering(&conn, &profile_id, day).unwrap().is_none(),
                "{day}"
            );
        }
    }

    #[test]
    fn one_day_is_a_span_whose_ends_meet() {
        let (conn, profile_id) = fixtures::workspace();
        let day = away(&conn, &profile_id, "2027-01-01", "2027-01-01", "New Year");

        let spans = spans(&conn, &profile_id).unwrap();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].days(), 1);
        assert_eq!(day.starts_on, day.ends_on);
    }

    #[test]
    fn a_span_that_ends_before_it_starts_is_refused_in_words() {
        let (conn, profile_id) = fixtures::workspace();
        let refused = create_minted(
            &conn,
            &profile_id,
            NewTimeOff {
                starts_on: "2026-10-27".into(),
                ends_on: "2026-10-20".into(),
                label: "Away".into(),
            },
            Minted::fresh(),
        )
        .unwrap_err();
        assert_eq!(
            refused.refusal().map(|refusal| refusal.code),
            Some("timeOff.endsBeforeItStarts")
        );

        let refused = create_minted(
            &conn,
            &profile_id,
            NewTimeOff {
                starts_on: "2026-02-30".into(),
                ends_on: "2026-03-02".into(),
                label: "Away".into(),
            },
            Minted::fresh(),
        )
        .unwrap_err();
        assert_eq!(
            refused.refusal().map(|refusal| refusal.code),
            Some("timeOff.badDay")
        );

        let refused = create_minted(
            &conn,
            &profile_id,
            NewTimeOff {
                starts_on: "2026-10-20".into(),
                ends_on: "2026-10-20".into(),
                label: "  ".into(),
            },
            Minted::fresh(),
        )
        .unwrap_err();
        assert_eq!(
            refused.refusal().map(|refusal| refusal.code),
            Some("timeOff.unnamed")
        );
    }

    /// Overlapping spans are both kept, and a day they share always names
    /// the one that starts first.
    #[test]
    fn overlapping_spans_name_a_shared_day_by_the_earlier() {
        let (conn, profile_id) = fixtures::workspace();
        away(&conn, &profile_id, "2026-12-28", "2027-01-05", "Away");
        away(&conn, &profile_id, "2027-01-01", "2027-01-01", "New Year");

        assert_eq!(list(&conn, &profile_id).unwrap().len(), 2);
        let shared = covering(&conn, &profile_id, "2027-01-01").unwrap().unwrap();
        assert_eq!(shared.label, "Away");
    }

    #[test]
    fn an_edit_that_changes_nothing_writes_nothing() {
        let (conn, profile_id) = fixtures::workspace();
        let span = away(&conn, &profile_id, "2026-10-20", "2026-10-27", "Away");

        let same = update_at(
            &conn,
            &span.id,
            TimeOffPatch {
                label: Some("Away".into()),
                ..TimeOffPatch::default()
            },
            "2030-01-01T00:00:00.000Z",
        )
        .unwrap();
        assert_eq!(same.updated_at, span.updated_at);

        let moved = update_at(
            &conn,
            &span.id,
            TimeOffPatch {
                ends_on: Some("2026-10-29".into()),
                ..TimeOffPatch::default()
            },
            "2030-01-01T00:00:00.000Z",
        )
        .unwrap();
        assert_eq!(moved.ends_on, "2026-10-29");
        assert_eq!(moved.starts_on, "2026-10-20");

        let refused = update_at(
            &conn,
            &span.id,
            TimeOffPatch {
                starts_on: Some("2026-11-01".into()),
                ..TimeOffPatch::default()
            },
            "2030-01-01T00:00:00.000Z",
        )
        .unwrap_err();
        assert_eq!(
            refused.refusal().map(|refusal| refusal.code),
            Some("timeOff.endsBeforeItStarts")
        );
    }
}
