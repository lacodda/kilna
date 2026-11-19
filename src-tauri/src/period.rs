//! A period laid out ahead (v0.152, FR-20 20.3).
//!
//! A channel plans its year before a track of it exists: thirteen days a
//! month on Tuesday, Thursday and Saturday, a floating peak where a month
//! falls short, nothing on the days off. [`plan`] reads those days off the
//! profile's rhythm for a month, a quarter or a year and says what already
//! stands on them; [`apply`](crate::actions::period::apply) makes, in one
//! gesture, a collection for each month and a draft for each day, planned on
//! it (`work_plan`, FR-11). A draft is an ordinary work at the first stage
//! with nothing written: it has no folder until something of it is kept on
//! disk (FR-04), and its slot is the calendar's dashed line.

use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::Serialize;
use time::{Date, Month};

use crate::error::{Error, Result};

/// The most months laid out at once: a year.
pub const LONGEST: u32 = 12;

/// One day of the period a release is meant for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct PeriodSlot {
    /// `YYYY-MM-DD`.
    pub date: String,
    /// Not one of the grid's days: added for a month short of its target.
    pub floating: bool,
    /// A day off (`time_off`): nothing is made for it.
    pub closed: bool,
    /// What already stands on the day - a release booked, or a work planned
    /// on it: nothing is made for it either.
    pub taken: Option<String>,
}

/// One month of the period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct PeriodMonth {
    /// `YYYY-MM`.
    pub month: String,
    /// What the rhythm means a month to hold, when it says.
    pub target: Option<u32>,
    pub slots: Vec<PeriodSlot>,
}

/// The days of a period, month by month, before anything is made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct PeriodPlan {
    pub months: Vec<PeriodMonth>,
}

/// The days of `months` months from `from` (`YYYY-MM`) by the profile's
/// rhythm: a rhythm of weekdays gives each month its grid, its target and
/// its rules; a rhythm of spacing walks its days apart from the period's
/// first day.
pub fn plan(conn: &Connection, profile_id: &str, from: &str, months: u32) -> Result<PeriodPlan> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let Some(rhythm) = config.rhythm.clone() else {
        return Err(Error::refused("layout.noRhythm"));
    };
    if rhythm.every_days == 0 && !rhythm.by_weekdays() {
        return Err(Error::refused("layout.rhythmTooShort"));
    }
    if months == 0 || months > LONGEST {
        return Err(Error::refused("period.badLength").param("months", i64::from(months)));
    }
    let (year, month) = month_of(from)?;
    let closed = crate::time_off::spans(conn, profile_id)?;
    let taken = taken_days(conn, profile_id, &config)?;

    let mut out = Vec::new();
    let mut walk: Option<Date> = None;
    let (mut y, mut m) = (year, month);
    for _ in 0..months {
        let first = Date::from_calendar_date(y, m, 1)
            .map_err(|_| Error::refused("period.badMonth").param("month", from))?;
        let days: Vec<(Date, bool)> = if rhythm.by_weekdays() {
            rhythm
                .month_slots(y, m)
                .into_iter()
                .map(|slot| (slot.date, slot.floating))
                .collect()
        } else {
            // Days apart, carried across the months from the period's first.
            let mut day = walk.unwrap_or(first);
            let mut found = Vec::new();
            while day.month() == m && day.year() == y {
                found.push((day, false));
                day += time::Duration::days(i64::from(rhythm.every_days));
            }
            walk = Some(day);
            found
        };
        let slots = days
            .into_iter()
            .map(|(date, floating)| {
                let key = iso(date);
                PeriodSlot {
                    closed: crate::time_off::closes(&closed, date),
                    taken: taken.get(&key).cloned(),
                    date: key,
                    floating,
                }
            })
            .collect();
        out.push(PeriodMonth {
            month: format!("{y:04}-{:02}", u8::from(m)),
            target: rhythm.monthly_target.filter(|_| rhythm.by_weekdays()),
            slots,
        });
        (y, m) = next(y, m);
    }
    Ok(PeriodPlan { months: out })
}

/// The days something already stands on - a release booked or out, a work
/// planned - with its title.
fn taken_days(
    conn: &Connection,
    profile_id: &str,
    config: &crate::profile::config::ProfileConfig,
) -> Result<BTreeMap<String, String>> {
    let mut taken = BTreeMap::new();
    for entry in crate::release::calendar(conn, profile_id)? {
        if let Some(at) = entry.release.scheduled_at.as_deref() {
            taken
                .entry(at.get(..10).unwrap_or(at).to_owned())
                .or_insert_with(|| entry.work_title.clone());
        }
    }
    for plan in crate::work_plan::open(conn, profile_id, config.plan_window())? {
        if let Some(work) = crate::work::get(conn, &plan.work_id)? {
            taken.entry(plan.scheduled_at.clone()).or_insert(work.title);
        }
    }
    Ok(taken)
}

/// `YYYY-MM` read.
pub fn month_of(text: &str) -> Result<(i32, Month)> {
    let bad = || Error::refused("period.badMonth").param("month", text);
    let (year, month) = text.trim().split_once('-').ok_or_else(bad)?;
    let year: i32 = year.parse().map_err(|_| bad())?;
    let month: u8 = month.parse().map_err(|_| bad())?;
    let month = Month::try_from(month).map_err(|_| bad())?;
    if !(1900..=9999).contains(&year) {
        return Err(bad());
    }
    Ok((year, month))
}

fn next(year: i32, month: Month) -> (i32, Month) {
    match month {
        Month::December => (year + 1, Month::January),
        other => (year, other.next()),
    }
}

fn iso(day: Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        day.year(),
        u8::from(day.month()),
        day.day()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::period::{PeriodDraft, PeriodLayout, PeriodMonthLayout, apply};

    fn channel() -> (Connection, String) {
        let conn = crate::db::open_in_memory().unwrap();
        crate::profile::seed(&conn, "track-channel").unwrap();
        let profile_id = crate::profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    /// A quarter of Tuesdays, Thursdays and Saturdays, thirteen a month:
    /// a month of fourteen leaves its weakest day out, a month of twelve
    /// gains a floating peak; a day off and a day already planned are said.
    #[test]
    fn a_quarter_is_laid_out_by_the_weekday_grid() {
        let (conn, profile_id) = channel();
        crate::time_off::create_minted(
            &conn,
            &profile_id,
            crate::time_off::NewTimeOff {
                starts_on: "2027-03-02".into(),
                ends_on: "2027-03-02".into(),
                label: "Away".into(),
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap();
        let planned = crate::fixtures::work(&conn, &profile_id, "track", "Already here");
        crate::work_plan::create_minted(
            &conn,
            crate::work_plan::NewWorkPlan {
                work_id: planned.id,
                scheduled_at: "2027-03-04".into(),
                note: None,
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap();

        let plan = plan(&conn, &profile_id, "2027-01", 3).unwrap();
        let months: Vec<&str> = plan.months.iter().map(|m| m.month.as_str()).collect();
        assert_eq!(months, vec!["2027-01", "2027-02", "2027-03"]);
        for month in &plan.months {
            assert_eq!(month.slots.len(), 13, "{}", month.month);
            assert_eq!(month.target, Some(13));
        }
        // February 2027 has twelve of those days: one floating peak.
        assert_eq!(
            plan.months[1].slots.iter().filter(|s| s.floating).count(),
            1
        );
        let march = &plan.months[2];
        let day = |date: &str| march.slots.iter().find(|s| s.date == date).unwrap();
        assert!(day("2027-03-02").closed);
        assert_eq!(day("2027-03-04").taken.as_deref(), Some("Already here"));

        assert!(plan_err(&conn, &profile_id, "2027-13", 1));
        assert!(plan_err(&conn, &profile_id, "2027-01", 13));
    }

    fn plan_err(conn: &Connection, profile_id: &str, from: &str, months: u32) -> bool {
        plan(conn, profile_id, from, months).is_err()
    }

    /// Laid out, a month gathers its drafts in a collection of its kind -
    /// the one already there by that title - each draft planned on its day;
    /// one undo takes the whole period back.
    #[test]
    fn a_period_is_made_in_one_gesture_and_taken_back_in_one() {
        let (conn, profile_id) = channel();
        // A media folder is named: a draft makes no folder in it (FR-04).
        let media = tempfile::tempdir().unwrap();
        crate::folder::set_root(
            &conn,
            &profile_id,
            Some(&media.path().display().to_string()),
            "2026-11-19T10:00:00.000Z",
        )
        .unwrap();
        let made = apply(
            &conn,
            PeriodLayout {
                kind: "track".into(),
                collection_kind: Some("month_pack".into()),
                months: vec![
                    PeriodMonthLayout {
                        month: "2027-01".into(),
                        title: "January 2027".into(),
                        drafts: vec![
                            PeriodDraft {
                                date: "2027-01-05".into(),
                                title: "Track 05.01".into(),
                            },
                            PeriodDraft {
                                date: "2027-01-07".into(),
                                title: "Track 07.01".into(),
                            },
                        ],
                    },
                    PeriodMonthLayout {
                        month: "2027-02".into(),
                        title: "February 2027".into(),
                        drafts: vec![PeriodDraft {
                            date: "2027-02-02".into(),
                            title: "Track 02.02".into(),
                        }],
                    },
                ],
            },
        )
        .unwrap();
        assert_eq!((made.collections, made.works), (2, 3));
        assert_eq!(
            std::fs::read_dir(media.path()).unwrap().count(),
            0,
            "a draft has no folder until something of it is kept"
        );
        let packs = crate::collection::list(&conn, &profile_id).unwrap();
        let january = packs
            .iter()
            .find(|one| one.title == "January 2027")
            .unwrap();
        assert_eq!(january.target_size, Some(13));
        assert_eq!(
            crate::collection::members(&conn, &january.id)
                .unwrap()
                .len(),
            2
        );
        let plans = crate::work_plan::open(&conn, &profile_id, 7).unwrap();
        assert_eq!(plans.len(), 3);

        let offer = crate::undo::last(&conn).unwrap().unwrap();
        crate::undo::undo(&conn, &offer.operation_id).unwrap();
        assert!(
            crate::collection::list(&conn, &profile_id)
                .unwrap()
                .is_empty()
        );
        assert!(
            crate::work_plan::open(&conn, &profile_id, 7)
                .unwrap()
                .is_empty()
        );

        // A day outside its month is refused, and nothing is made.
        let refused = apply(
            &conn,
            PeriodLayout {
                kind: "track".into(),
                collection_kind: None,
                months: vec![PeriodMonthLayout {
                    month: "2027-01".into(),
                    title: "January 2027".into(),
                    drafts: vec![PeriodDraft {
                        date: "2027-02-02".into(),
                        title: "Stray".into(),
                    }],
                }],
            },
        );
        assert_eq!(
            refused.unwrap_err().refusal().unwrap().code,
            "period.badDay"
        );
    }
}
