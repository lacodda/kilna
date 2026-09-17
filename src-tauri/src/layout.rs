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
use std::cmp::Ordering;
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

/// What the layout needs to know about a release still waiting for a day.
struct Queued {
    id: String,
    work_id: String,
    work_title: String,
    total: Option<f64>,
    created_at: String,
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
///   days, and neither do two works cut from the same video. Spacing already
///   guarantees this for a rhythm of two days or more; a daily rhythm is
///   where it earns its keep.
///
///   The second half is the shorts rule (v0.73), and it is the same rule
///   rather than a second mechanism: two shorts cut from one video on
///   consecutive days are the same video twice to anyone watching, whatever
///   the two shorts are called. A short with no donor — one shot for itself —
///   has no source to clash on and is scattered by its work alone. There is
///   no separate queue and no shuffle (decision of 2026-09-11): a rule that
///   holds every day beats an order that happens to look spread out.
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

    // Which videos each work is cut from, read once for the whole workspace.
    // Asking per release would repeat the query on every day the scan walks
    // past, and the scan walks past a lot of days.
    let sources = crate::cut::sources_by_work(conn, profile_id)?;

    // The ground: every date any release sits on, each work's own dates, and
    // each donor's. Released entries count too — something that went out
    // yesterday sets the pace exactly as a plan for yesterday would have.
    let mut taken: BTreeSet<Date> = BTreeSet::new();
    let mut work_dates: BTreeMap<String, BTreeSet<Date>> = BTreeMap::new();
    let mut source_dates: BTreeMap<String, BTreeSet<Date>> = BTreeMap::new();
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
        for source in sources.get(&entry.release.work_id).into_iter().flatten() {
            source_dates.entry(source.clone()).or_default().insert(date);
        }
    }

    let mut remaining: Vec<Queued> = release::queue(conn, profile_id)?
        .into_iter()
        .map(|entry| Queued {
            id: entry.release.id,
            work_id: entry.release.work_id,
            work_title: entry.work_title,
            total: entry.total,
            created_at: entry.release.created_at,
        })
        .collect();
    // The queue comes back strongest-first, but two releases with one total
    // and one title tie, and what breaks the tie in SQL is the order the rows
    // were written in — which differs between two workspaces holding the same
    // facts. The plan has to be a function of the facts alone, so after the
    // queue's own order the last word goes to the release id: not an order
    // anyone reads, but one every device computes alike. That is why the
    // tie-break lives here and not in the query.
    remaining.sort_by(|a, b| {
        a.total
            .is_none()
            .cmp(&b.total.is_none())
            .then_with(|| b.total.partial_cmp(&a.total).unwrap_or(Ordering::Equal))
            .then_with(|| a.work_title.cmp(&b.work_title))
            .then_with(|| a.created_at.cmp(&b.created_at))
            .then_with(|| a.id.cmp(&b.id))
    });

    let mut placements = Vec::with_capacity(remaining.len());
    let mut date = next_day(today)?;

    // Provably more days than the layout can need: each placement advances at
    // most `every_days + 2` days past the previous one, and each pre-existing
    // date blocks a window shorter than `2 * every_days`. Running out means a
    // bug in the loop, and an error beats scanning dates forever.
    //
    // The donor rule does not widen this. It blocks the same two-day window
    // around a placement that the work rule blocks, on a different key, so
    // the worst case a placement can push the scan is what it already was.
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
            // The strongest release whose work — and whose donors — keep clear
            // of the neighbouring days. When everything remaining is too
            // close, the day stays empty and the scan moves on: scatter is a
            // rule, not a wish.
            let day_before = date - Duration::DAY;
            let day_after = date + Duration::DAY;
            let clear = |dates: &BTreeMap<String, BTreeSet<Date>>, key: &str| {
                dates
                    .get(key)
                    .is_none_or(|dates| dates.range(day_before..=day_after).next().is_none())
            };
            let pick = remaining.iter().position(|queued| {
                clear(&work_dates, &queued.work_id)
                    && sources
                        .get(&queued.work_id)
                        .into_iter()
                        .flatten()
                        .all(|source| clear(&source_dates, source))
            });
            if let Some(index) = pick {
                let queued = remaining.remove(index);
                taken.insert(date);
                for source in sources.get(&queued.work_id).into_iter().flatten() {
                    source_dates.entry(source.clone()).or_default().insert(date);
                }
                work_dates.entry(queued.work_id).or_default().insert(date);
                placements.push(Placement {
                    release_id: queued.id,
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
///
/// The operation is written inside this function's own transaction rather than
/// by the caller around it, for the reason [`crate::trash::discard_minted`]
/// gives: a log entry committed beside a layout that then failed would replay
/// into placements that never landed. See ADR 0014.
pub fn apply(
    conn: &mut Connection,
    placements: &[Placement],
    logged: Option<crate::operation::Intent>,
) -> Result<usize> {
    apply_at(conn, placements, logged, &now())
}

/// Apply a plan with the moment already decided.
///
/// The seam a replay comes back through. One plan is one gesture, so every
/// release it places carries the same instant — live and replayed alike. See
/// ADR 0014.
pub fn apply_at(
    conn: &mut Connection,
    placements: &[Placement],
    logged: Option<crate::operation::Intent>,
    at: &str,
) -> Result<usize> {
    let tx = conn.transaction()?;

    if let Some(logged) = logged {
        crate::operation::record(&tx, logged)?;
    }

    let timestamp = at.to_owned();

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
                rater: None,
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
                        scheduled_time: None,
                        time_zone: None,
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
        let (conn, profile_id) = workspace(3);

        // Booked ahead on the 5th, and released on the 1st.
        let booked = queued(&conn, &profile_id, "Booked", 6.0, 1);
        release::schedule(&conn, &booked[0], "2026-09-05").unwrap();
        let out = queued(&conn, &profile_id, "Out", 6.0, 1);
        release::schedule(&conn, &out[0], "2026-09-01").unwrap();
        release::mark_released(&conn, &out[0], None, None).unwrap();

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

    /// Two shorts cut from one video are the same video twice to anyone
    /// watching, so they scatter as one work's releases do — even though they
    /// are two separate works with two separate titles.
    ///
    /// Mutating the donor half of the `pick` filter must put "Hook" and
    /// "Bridge" on consecutive days and break this.
    #[test]
    fn two_shorts_from_one_video_do_not_land_on_neighbouring_days() {
        let (conn, profile_id) = workspace(1);

        let donor = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "video".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap();

        // Two shorts out of that one video, and one shot for itself. All three
        // are equally strong, so nothing but the rules decides the order.
        let hook = shorts_release(&conn, &profile_id, "Anchor", Some(&donor.id));
        let bridge = shorts_release(&conn, &profile_id, "Bridge", Some(&donor.id));
        let own = shorts_release(&conn, &profile_id, "Zephyr", None);

        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();
        assert_eq!(plan.len(), 3, "everything queued is placed");

        let day_of = |release_id: &str| {
            plan.iter()
                .find(|placement| placement.release_id == release_id)
                .map(|placement| parse_date(&placement.date).unwrap())
                .unwrap()
        };
        let (hook_day, bridge_day) = (day_of(&hook), day_of(&bridge));
        assert!(
            (hook_day - bridge_day).abs() > Duration::DAY,
            "two shorts from one video sat on {hook_day} and {bridge_day}"
        );

        // And the short with no donor is free to take the day between them:
        // the rule is about the video, not about shorts as a category.
        let own_day = day_of(&own);
        assert!(
            own_day > hook_day.min(bridge_day) && own_day < hook_day.max(bridge_day),
            "a short shot for itself has no source to clash on"
        );
    }

    /// A short, its donor link, and one queued release for it. `donor` of
    /// `None` is a short shot for itself.
    fn shorts_release(
        conn: &Connection,
        profile_id: &str,
        title: &str,
        donor: Option<&str>,
    ) -> String {
        let work = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "short".into(),
                title: title.into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        score::create(
            conn,
            &work.id,
            NewScore {
                axes: json!({ "story": 7.0 }).as_object().cloned().unwrap(),
                version_id: None,
                note: None,
                rater: None,
            },
        )
        .unwrap();
        if let Some(donor) = donor {
            crate::cut::create(
                conn,
                profile_id,
                crate::cut::NewCut {
                    work_id: work.id.clone(),
                    source_id: donor.to_owned(),
                    starts_at: 10.0,
                    ends_at: 22.0,
                    position: None,
                    label: None,
                },
            )
            .unwrap();
        }
        release::create(
            conn,
            NewRelease {
                work_id: work.id,
                kind: "short".into(),
                title: Some(title.into()),
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap()
        .id
    }

    /// The same inputs give the same plan, byte for byte. The preview's whole
    /// promise is that approving it books it, and a layout that dices would
    /// book something nobody saw.
    #[test]
    fn the_plan_is_deterministic() {
        let (conn, profile_id) = workspace(2);
        queued(&conn, &profile_id, "One", 6.0, 2);
        queued(&conn, &profile_id, "Two", 6.0, 2);
        let anchor = queued(&conn, &profile_id, "Anchor", 8.0, 1);
        release::schedule(&conn, &anchor[0], "2026-09-10").unwrap();

        let first = plan(&conn, &profile_id, "2026-09-01").unwrap();
        let second = plan(&conn, &profile_id, "2026-09-01").unwrap();

        assert_eq!(first, second);
        assert!(!first.is_empty());
    }

    /// The same facts written in a different order — what two devices that
    /// synchronised would hold. The rows are inserted by hand so the ids can
    /// be the same on both sides; everything else about them ties.
    fn workspace_with(order: &[(&str, &str)]) -> (Connection, String) {
        let (conn, profile_id) = workspace(2);
        for (work_id, release_id) in order {
            conn.execute(
                "INSERT INTO work (id, profile_id, kind, title, status, created_at, updated_at)
                 VALUES (?1, ?2, 'song', 'Same title', 'draft', '2026-09-01T00:00:00.000Z',
                         '2026-09-01T00:00:00.000Z')",
                params![work_id, profile_id],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO release (id, work_id, kind, status, created_at, updated_at)
                 VALUES (?1, ?2, 'clip', ?3, '2026-09-01T00:00:00.000Z',
                         '2026-09-01T00:00:00.000Z')",
                params![release_id, work_id, release::PLANNED],
            )
            .unwrap();
        }
        (conn, profile_id)
    }

    #[test]
    fn the_plan_does_not_depend_on_the_order_the_rows_were_written_in() {
        let pairs = [("w-a", "r-a"), ("w-b", "r-b"), ("w-c", "r-c")];
        let (forward, profile_forward) = workspace_with(&pairs);
        let mut reversed_pairs = pairs;
        reversed_pairs.reverse();
        let (reversed, profile_reversed) = workspace_with(&reversed_pairs);

        let one = plan(&forward, &profile_forward, "2026-09-01").unwrap();
        let two = plan(&reversed, &profile_reversed, "2026-09-01").unwrap();

        assert_eq!(one.len(), 3);
        assert_eq!(
            one, two,
            "two devices with the same facts must lay out the same calendar"
        );
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
        let applied = apply(&mut conn, &plan, None).unwrap();

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
        release::schedule(&conn, &interloper[0], plan[1].date.as_str()).unwrap();

        let refused = apply(&mut conn, &plan, None).unwrap_err();
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

        release::schedule(&conn, &ids[0], "2026-12-24").unwrap();

        let refused = apply(&mut conn, &plan, None).unwrap_err();
        assert_eq!(refused.kind(), "layoutStale", "got {refused}");
        // Its hand-picked date survives the refusal.
        let kept = release::get(&conn, &ids[0]).unwrap().unwrap();
        assert_eq!(kept.scheduled_at.as_deref(), Some("2026-12-24"));
    }
}
