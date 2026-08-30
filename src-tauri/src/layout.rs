//! Laying the queue out to the profile's rhythm.
//!
//! Two halves, deliberately split: [`plan`] computes where everything would
//! land and moves nothing, [`apply`] takes a plan back and books exactly it.
//! What the person approved is what happens — the preview is not a sketch of
//! an algorithm that will run again later on different data. If the calendar
//! changed between the two calls, `apply` refuses whole rather than booking
//! half of an outdated picture.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use time::{Date, Duration, macros::format_description};

use crate::error::{Error, Result};
use crate::release::{self, Verdict};
use crate::time::now;

/// One line of the plan: this release lands on this day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub release_id: String,
    pub date: String,
}

/// Where the queue would land, laid out to the rhythm.
///
/// The rules, in the order they bind:
///
/// * **Nothing already on the calendar moves.** Booked days — pinned or not —
///   are ground the layout builds around, never material it rearranges.
/// * **Spacing**: a placement keeps at least `every_days` from every release
///   that has a date — existing or placed here, planned or already out. The
///   rhythm is the pace of the whole output, not of one work.
/// * **Scatter**: two releases of the same work never sit on neighbouring
///   days. Spacing already guarantees this for a rhythm of two days or more;
///   a daily rhythm is where it earns its keep.
/// * **Order**: the queue is walked strongest-first, each release taking the
///   earliest day the rules allow. Everything queued is placed — the preview
///   is where a person decides whether they meant that.
///
/// Placement starts tomorrow: today is already underway, and a slot invented
/// for it would arrive pre-warned as "not ready".
///
/// `today` is the user's local date, supplied by the frontend — the backend
/// only knows UTC, which at a negative offset is already tomorrow.
pub fn plan(conn: &Connection, profile_id: &str, today: &str) -> Result<Vec<Placement>> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let Some(rhythm) = config.rhythm else {
        return Err(Error::Other(
            "the profile has no release rhythm — set one in the profile editor".into(),
        ));
    };
    if rhythm.every_days == 0 {
        return Err(Error::Other("the rhythm must be at least one day".into()));
    }
    let spacing = Duration::days(i64::from(rhythm.every_days) - 1);
    let today = parse_date(today)?;

    // The ground: every date any release sits on, and each work's own dates.
    // Released entries count too — something that went out yesterday sets the
    // pace exactly as a plan for yesterday would have.
    let mut taken: BTreeSet<Date> = BTreeSet::new();
    let mut work_dates: BTreeMap<String, BTreeSet<Date>> = BTreeMap::new();
    for entry in release::calendar(conn, profile_id)? {
        let Some(date) = entry.release.scheduled_at.as_deref() else {
            continue;
        };
        let date = parse_date(date)?;
        taken.insert(date);
        work_dates
            .entry(entry.release.work_id.clone())
            .or_default()
            .insert(date);
    }

    let mut remaining: Vec<(String, String)> = release::queue(conn, profile_id)?
        .into_iter()
        .map(|entry| (entry.release.id.clone(), entry.release.work_id.clone()))
        .collect();

    let mut placements = Vec::with_capacity(remaining.len());
    let mut date = next_day(today)?;

    // Provably more days than the layout can need: each placement advances at
    // most `every_days + 2` days past the previous one, and each pre-existing
    // date blocks a window shorter than `2 * every_days`. Running out means a
    // bug in the loop, and an error beats scanning dates forever.
    let mut scans_left = (remaining.len() as i64 + 2) * (i64::from(rhythm.every_days) + 2)
        + taken.len() as i64 * 2 * i64::from(rhythm.every_days)
        + 366;

    while !remaining.is_empty() {
        scans_left -= 1;
        if scans_left < 0 {
            return Err(Error::Other(
                "the layout could not settle — this is a bug worth reporting".into(),
            ));
        }

        let spaced = taken
            .range(date - spacing..=date + spacing)
            .next()
            .is_none();
        if spaced {
            // The strongest release whose work keeps clear of the neighbouring
            // days. When every remaining work is too close, the day stays
            // empty and the scan moves on — scatter is a rule, not a wish.
            let day_before = date - Duration::DAY;
            let day_after = date + Duration::DAY;
            let pick = remaining.iter().position(|(_, work_id)| {
                work_dates
                    .get(work_id)
                    .is_none_or(|dates| dates.range(day_before..=day_after).next().is_none())
            });
            if let Some(index) = pick {
                let (release_id, work_id) = remaining.remove(index);
                taken.insert(date);
                work_dates.entry(work_id).or_default().insert(date);
                placements.push(Placement {
                    release_id,
                    date: iso(date)?,
                });
            }
        }

        date = next_day(date)?;
    }

    Ok(placements)
}

/// Book exactly what [`plan`] proposed and a person approved.
///
/// Every placement is re-judged by the same rule a drop is — the day must
/// still read [`Verdict::Empty`] — and the release must still be waiting in
/// the queue. Anything else means the calendar moved since the preview, and
/// the whole plan is refused rather than partially applied: the person
/// approved a picture, not its surviving fragments.
pub fn apply(conn: &mut Connection, placements: &[Placement]) -> Result<usize> {
    let tx = conn.transaction()?;
    let timestamp = now();

    for placement in placements {
        // The date is stored as given, so it must be a real date — a plan is
        // trusted to come back from `plan`, but not blindly.
        parse_date(&placement.date)?;
        let found = release::get(&tx, &placement.release_id)?
            .ok_or_else(|| stale("a release from the plan is gone"))?;
        if found.status != release::PLANNED || found.scheduled_at.is_some() {
            return Err(stale("a release from the plan is no longer in the queue"));
        }
        if release::preview(&tx, &placement.release_id, &placement.date)?.verdict != Verdict::Empty
        {
            return Err(stale("a day from the plan is no longer free"));
        }

        tx.execute(
            "UPDATE release SET scheduled_at = ?2, updated_at = ?3 WHERE id = ?1",
            params![placement.release_id, placement.date, timestamp],
        )?;
    }

    tx.commit()?;
    Ok(placements.len())
}

fn stale(reason: &str) -> Error {
    Error::LayoutStale(format!("the calendar changed since the preview: {reason}"))
}

fn parse_date(date: &str) -> Result<Date> {
    let iso = format_description!("[year]-[month]-[day]");
    Date::parse(date, iso).map_err(|_| Error::Other(format!("`{date}` is not an ISO date")))
}

fn iso(date: Date) -> Result<String> {
    let iso = format_description!("[year]-[month]-[day]");
    date.format(iso)
        .map_err(|cause| Error::Other(cause.to_string()))
}

fn next_day(date: Date) -> Result<Date> {
    date.next_day()
        .ok_or_else(|| Error::Other("the layout ran off the end of the calendar".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{self, config::Rhythm};
    use crate::release::NewRelease;
    use crate::score::{self, NewScore};
    use crate::work::{self, NewWork};
    use serde_json::json;

    fn workspace(every_days: u32) -> (Connection, String) {
        let conn = crate::db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        set_rhythm(&conn, &profile_id, Some(every_days));
        (conn, profile_id)
    }

    fn set_rhythm(conn: &Connection, profile_id: &str, every_days: Option<u32>) {
        let mut config = profile::config_for(conn, profile_id).unwrap();
        config.rhythm = every_days.map(|every_days| Rhythm {
            every_days,
            default_time: None,
        });
        profile::update_config(conn, profile_id, &config).unwrap();
    }

    /// A work with a score, and one queued clip release per `releases`.
    fn queued(
        conn: &Connection,
        profile_id: &str,
        title: &str,
        hook: f64,
        releases: usize,
    ) -> Vec<String> {
        let work = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: title.into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        score::create(
            conn,
            &work.id,
            NewScore {
                axes: json!({ "hook": hook }).as_object().cloned().unwrap(),
                version_id: None,
                note: None,
            },
        )
        .unwrap();

        (0..releases)
            .map(|_| {
                release::create(
                    conn,
                    NewRelease {
                        work_id: work.id.clone(),
                        kind: "clip".into(),
                        title: Some(title.into()),
                        scheduled_at: None,
                        meta: None,
                    },
                )
                .unwrap()
                .id
            })
            .collect()
    }

    fn dates(placements: &[Placement]) -> Vec<&str> {
        placements
            .iter()
            .map(|placement| placement.date.as_str())
            .collect()
    }

    #[test]
    fn the_queue_lands_strongest_first_to_the_rhythm() {
        let (conn, profile_id) = workspace(3);
        queued(&conn, &profile_id, "Middle", 5.0, 1);
        queued(&conn, &profile_id, "Best", 9.0, 1);

        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();

        // Tomorrow first, then three days on; the stronger work takes the
        // earlier day.
        assert_eq!(dates(&plan), ["2026-09-02", "2026-09-05"]);
        let best = release::queue(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.work_title == "Best")
            .unwrap();
        assert_eq!(plan[0].release_id, best.release.id);
    }

    /// The rhythm counts every release with a date, not only plans: something
    /// that just went out sets the pace exactly as a booked slot would.
    #[test]
    fn spacing_is_kept_from_booked_and_released_days_alike() {
        let (mut conn, profile_id) = workspace(3);

        // Booked ahead on the 5th, and released on the 1st.
        let booked = queued(&conn, &profile_id, "Booked", 6.0, 1);
        release::schedule(&mut conn, &booked[0], "2026-09-05").unwrap();
        let out = queued(&conn, &profile_id, "Out", 6.0, 1);
        release::schedule(&mut conn, &out[0], "2026-09-01").unwrap();
        release::mark_released(&conn, &out[0], None).unwrap();

        queued(&conn, &profile_id, "Waiting", 5.0, 1);

        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();

        // Days 2–7 all come within three days of the 1st or the 5th; the 8th
        // is the first that keeps the rhythm from both.
        assert_eq!(dates(&plan), ["2026-09-08"]);
    }

    /// With a daily rhythm the spacing rule says nothing, and scatter is what
    /// keeps one work from occupying a run of consecutive days.
    #[test]
    fn a_daily_rhythm_scatters_one_works_releases_across_alternate_days() {
        let (conn, profile_id) = workspace(1);
        queued(&conn, &profile_id, "Only", 7.0, 3);

        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();

        assert_eq!(dates(&plan), ["2026-09-02", "2026-09-04", "2026-09-06"]);
    }

    /// Two works on a daily rhythm interleave: the day between two releases of
    /// one work is not wasted when another work can take it.
    #[test]
    fn a_daily_rhythm_interleaves_works() {
        let (conn, profile_id) = workspace(1);
        queued(&conn, &profile_id, "Strong", 9.0, 2);
        queued(&conn, &profile_id, "Weak", 4.0, 2);

        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();

        let by_date: Vec<(&str, &str)> = plan
            .iter()
            .map(|placement| {
                let entry = release::get(&conn, &placement.release_id).unwrap().unwrap();
                let title = crate::journal::work_title(&conn, &entry.work_id).unwrap();
                (
                    placement.date.as_str(),
                    if title == "Strong" { "s" } else { "w" },
                )
            })
            .collect();
        assert_eq!(
            by_date,
            [
                ("2026-09-02", "s"),
                ("2026-09-03", "w"),
                ("2026-09-04", "s"),
                ("2026-09-05", "w"),
            ]
        );
    }

    /// The same inputs give the same plan, byte for byte. The preview's whole
    /// promise is that approving it books it, and a layout that dices would
    /// book something nobody saw.
    #[test]
    fn the_plan_is_deterministic() {
        let (mut conn, profile_id) = workspace(2);
        queued(&conn, &profile_id, "One", 6.0, 2);
        queued(&conn, &profile_id, "Two", 6.0, 2);
        let anchor = queued(&conn, &profile_id, "Anchor", 8.0, 1);
        release::schedule(&mut conn, &anchor[0], "2026-09-10").unwrap();

        let first = plan(&conn, &profile_id, "2026-09-01").unwrap();
        let second = plan(&conn, &profile_id, "2026-09-01").unwrap();

        assert_eq!(first, second);
        assert!(!first.is_empty());
    }

    #[test]
    fn a_profile_without_a_rhythm_refuses_to_plan() {
        let (conn, profile_id) = workspace(3);
        set_rhythm(&conn, &profile_id, None);
        queued(&conn, &profile_id, "Waiting", 5.0, 1);

        assert!(plan(&conn, &profile_id, "2026-09-01").is_err());
    }

    #[test]
    fn an_empty_queue_plans_nothing() {
        let (conn, profile_id) = workspace(3);
        assert!(plan(&conn, &profile_id, "2026-09-01").unwrap().is_empty());
    }

    #[test]
    fn a_malformed_today_is_an_error_not_an_empty_plan() {
        let (conn, profile_id) = workspace(3);
        assert!(plan(&conn, &profile_id, "someday").is_err());
    }

    #[test]
    fn applying_the_plan_books_exactly_what_it_says() {
        let (mut conn, profile_id) = workspace(3);
        queued(&conn, &profile_id, "One", 6.0, 1);
        queued(&conn, &profile_id, "Two", 4.0, 1);

        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();
        let applied = apply(&mut conn, &plan).unwrap();

        assert_eq!(applied, 2);
        assert!(release::queue(&conn, &profile_id).unwrap().is_empty());
        let booked: Vec<(String, String)> = release::calendar(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .map(|entry| {
                (
                    entry.release.id.clone(),
                    entry.release.scheduled_at.clone().unwrap(),
                )
            })
            .collect();
        for placement in &plan {
            assert!(booked.contains(&(placement.release_id.clone(), placement.date.clone())));
        }
    }

    /// The plan was drawn against a calendar that has since moved: the whole
    /// application is refused, and nothing from it is booked.
    #[test]
    fn a_stale_plan_is_refused_whole() {
        let (mut conn, profile_id) = workspace(3);
        queued(&conn, &profile_id, "One", 6.0, 1);
        queued(&conn, &profile_id, "Two", 4.0, 1);
        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();

        // Someone books the second planned day by hand between the preview
        // and the approval.
        let interloper = queued(&conn, &profile_id, "Interloper", 9.0, 1);
        release::schedule(&mut conn, &interloper[0], plan[1].date.as_str()).unwrap();

        let refused = apply(&mut conn, &plan).unwrap_err();
        assert_eq!(refused.kind(), "layoutStale", "got {refused}");

        // The first placement would have succeeded on its own; atomicity is
        // the point.
        let still_queued = release::queue(&conn, &profile_id).unwrap();
        assert_eq!(still_queued.len(), 2, "nothing may be half-applied");
    }

    /// A release that found a date some other way is no longer the queue entry
    /// the person approved a plan for.
    #[test]
    fn a_release_scheduled_since_the_preview_makes_the_plan_stale() {
        let (mut conn, profile_id) = workspace(3);
        let ids = queued(&conn, &profile_id, "One", 6.0, 1);
        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();

        release::schedule(&mut conn, &ids[0], "2026-12-24").unwrap();

        let refused = apply(&mut conn, &plan).unwrap_err();
        assert_eq!(refused.kind(), "layoutStale", "got {refused}");
        // Its hand-picked date survives the refusal.
        let kept = release::get(&conn, &ids[0]).unwrap().unwrap();
        assert_eq!(kept.scheduled_at.as_deref(), Some("2026-12-24"));
    }
}
