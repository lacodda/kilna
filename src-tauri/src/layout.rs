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

use crate::db::unit::atomically;
use crate::error::{Error, Result};
use crate::release::{self, Verdict};
use crate::time::now;

/// One line of the plan: this release lands on this day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
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
/// * **Closed days stay empty** ([`crate::time_off`]): a holiday, a fortnight
///   away. The layout steps over them; only a person puts something there.
/// * **Spacing**: a placement keeps at least `every_days` from every release
///   that has a date — existing or placed here, planned or already out. The
///   rhythm is the pace of the whole output, not of one work.
/// * **Scatter**: two works made from the same thing never go out on
///   neighbouring days - the clip and the audio of one song, two shorts cut
///   from one video. Spacing already guarantees this for a rhythm of two
///   days or more; a daily rhythm is where it earns its keep.
///
///   It began as the shorts rule (v0.73): two shorts cut from one video on
///   consecutive days are the same video twice to anyone watching, whatever
///   the two shorts are called. Since a publication goes out once (ADR
///   0051), what used to be two releases of one work - a song's clip and its
///   audio - are two works made from one song, and the same rule keeps them
///   apart: what they are made from, through the links and the cuts, is the
///   key. A work made from nothing has no key to clash on and is scattered
///   by itself alone. There is no separate queue and no shuffle (decision
///   of 2026-09-11): a rule that holds every day beats an order that happens
///   to look spread out.
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
        return Err(Error::refused("layout.noRhythm"));
    };
    if rhythm.every_days == 0 {
        return Err(Error::refused("layout.rhythmTooShort"));
    }
    let spacing = Duration::days(i64::from(rhythm.every_days) - 1);
    let today = parse_date(today)?;
    let closed = crate::time_off::spans(conn, profile_id)?;

    // What each work is made from - the videos it is cut from, and every
    // work up its chain - read once for the whole workspace. Asking per
    // release would repeat the query on every day the scan walks past, and
    // the scan walks past a lot of days.
    let mut sources = crate::cut::sources_by_work(conn, profile_id)?;
    for (work_id, ancestors) in crate::link::ancestors_by_work(conn, profile_id)? {
        let known = sources.entry(work_id).or_default();
        for ancestor in ancestors {
            if !known.contains(&ancestor) {
                known.push(ancestor);
            }
        }
    }

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
    // A closed day pushes it by one day at most, so every closed day counts
    // once more.
    let mut scans_left = (remaining.len() as i64 + 2) * (i64::from(rhythm.every_days) + 2)
        + taken.len() as i64 * 2 * i64::from(rhythm.every_days)
        + closed.iter().map(crate::time_off::Span::days).sum::<i64>()
        + 366;

    while !remaining.is_empty() {
        scans_left -= 1;
        if scans_left < 0 {
            return Err(Error::Internal(
                "the layout could not settle — this is a bug worth reporting".into(),
            ));
        }

        let spaced = taken
            .range(date - spacing..=date + spacing)
            .next()
            .is_none();
        if spaced && !crate::time_off::closes(&closed, date) {
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
/// One unit: a plan half booked is not the plan that was approved.
pub fn apply(conn: &Connection, placements: &[Placement]) -> Result<usize> {
    apply_at(conn, placements, &now())
}

/// Apply a plan with the moment already decided.
///
/// The seam a replay comes back through. One plan is one gesture, so every
/// release it places carries the same instant — live and replayed alike. See
/// ADR 0014.
pub fn apply_at(conn: &Connection, placements: &[Placement], at: &str) -> Result<usize> {
    atomically(conn, |tx| apply_in(tx, placements, at))
}

fn apply_in(tx: &Connection, placements: &[Placement], at: &str) -> Result<usize> {
    let timestamp = at.to_owned();

    for placement in placements {
        // The date is stored as given, so it must be a real date — a plan is
        // trusted to come back from `plan`, but not blindly.
        parse_date(&placement.date)?;
        let found = release::get(tx, &placement.release_id)?
            .ok_or_else(|| stale("a release from the plan is gone"))?;
        if found.status != release::PLANNED || found.scheduled_at.is_some() {
            return Err(stale("a release from the plan is no longer in the queue"));
        }
        let day = release::preview(tx, &placement.release_id, &placement.date)?;
        if day.verdict != Verdict::Empty {
            return Err(stale("a day from the plan is no longer free"));
        }
        if day.closed.is_some() {
            return Err(stale("a day from the plan has been closed"));
        }

        tx.execute(
            "UPDATE release SET scheduled_at = ?2, updated_at = ?3 WHERE id = ?1",
            params![placement.release_id, placement.date, timestamp],
        )?;
    }

    Ok(placements.len())
}

/// Take back a booked plan: every release it placed goes back to the queue.
///
/// The undo of [`apply`]. Only the last operation is ever undone, so each
/// release is where the plan put it; one that has gone out since is left
/// alone - its date is a record of what happened, not a booking.
pub fn take_back_at(conn: &Connection, placements: &[Placement], at: &str) -> Result<()> {
    for placement in placements {
        conn.execute(
            "UPDATE release SET scheduled_at = NULL, slot_pinned_at = NULL, updated_at = ?2
              WHERE id = ?1 AND status = ?3",
            params![placement.release_id, at, release::PLANNED],
        )?;
    }
    Ok(())
}

/// One release going from one day to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Move {
    pub release_id: String,
    pub from: String,
    pub to: String,
}

/// What shifting the calendar would do, before anything moves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct ShiftPlan {
    /// Every release that would move, in calendar order.
    pub moves: Vec<Move>,
    /// Releases in the span that keep their date, because a person settled
    /// it: a pin is the one word that says "this day, whatever else moves".
    pub kept: Vec<String>,
    /// Releases of `moves` that would land on a closed day. Said, not
    /// refused: the person shifting is the one deciding.
    pub closed: Vec<String>,
}

/// The longest shift, either way. A year is already a different plan.
const MAX_SHIFT_DAYS: i64 = 366;

/// Where the calendar would be after moving everything planned from `from`
/// onward by `days` - later when positive, earlier when negative.
///
/// The second of the layout's two halves for a calendar already booked: a
/// week lost to illness, a fortnight away added after the plan was made. It
/// moves nothing; [`apply_shift`] books exactly what it returns.
///
/// * **What moves**: every planned release with a date on or after `from`.
///   What already went out is history and stays; what a person pinned stays,
///   and the plan names it, so "why did that one not move" has an answer
///   before it is asked.
/// * **By plain days.** The gaps between releases are the rhythm, and moving
///   them all by the same number of days keeps it. A release landing on a
///   closed day is named, not stepped over: stepping over would bunch the
///   days after the closure.
/// * **Never into the past.** A shift that would put a release before `today`
///   is refused whole, naming the first one: today's local date, supplied by
///   the window, as for [`plan`].
pub fn plan_shift(
    conn: &Connection,
    profile_id: &str,
    from: &str,
    days: i64,
    today: &str,
) -> Result<ShiftPlan> {
    if days == 0 {
        return Err(Error::refused("layout.shiftByNothing"));
    }
    if days.abs() > MAX_SHIFT_DAYS {
        return Err(Error::refused("layout.shiftTooFar").param("days", MAX_SHIFT_DAYS));
    }
    let start = parse_date(from)?;
    let today = parse_date(today)?;
    let closed = crate::time_off::spans(conn, profile_id)?;
    let by = Duration::days(days);

    let mut plan = ShiftPlan {
        moves: Vec::new(),
        kept: Vec::new(),
        closed: Vec::new(),
    };
    for entry in release::calendar(conn, profile_id)? {
        let Some(on) = entry.release.scheduled_at.as_deref() else {
            continue;
        };
        if entry.release.status != release::PLANNED {
            continue;
        }
        let on_day = parse_date(on)?;
        if on_day < start {
            continue;
        }
        if entry.release.slot_pinned_at.is_some() {
            plan.kept.push(entry.release.id);
            continue;
        }
        let to = on_day
            .checked_add(by)
            .ok_or_else(|| Error::Internal("the shift ran off the end of the calendar".into()))?;
        if to < today {
            return Err(Error::refused("layout.shiftIntoThePast")
                .param("title", entry.work_title)
                .param("date", iso(to)?));
        }
        if crate::time_off::closes(&closed, to) {
            plan.closed.push(entry.release.id.clone());
        }
        plan.moves.push(Move {
            release_id: entry.release.id,
            from: on.to_owned(),
            to: iso(to)?,
        });
    }
    Ok(plan)
}

/// Book exactly the shift a person approved — all of it or none of it.
///
/// Each release must still be planned, on the day the preview saw, and not
/// pinned since; anything else means the calendar moved after the preview,
/// and the whole shift is refused, as a stale layout is.
pub fn apply_shift(conn: &Connection, moves: &[Move]) -> Result<usize> {
    apply_shift_at(conn, moves, &now())
}

/// Apply a shift with the moment already decided: the seam a replay comes
/// back through (ADR 0014).
pub fn apply_shift_at(conn: &Connection, moves: &[Move], at: &str) -> Result<usize> {
    atomically(conn, |tx| apply_shift_in(tx, moves, at))
}

fn apply_shift_in(tx: &Connection, moves: &[Move], at: &str) -> Result<usize> {
    for step in moves {
        parse_date(&step.to)?;
        let found = release::get(tx, &step.release_id)?
            .ok_or_else(|| stale("a release from the shift is gone"))?;
        if found.status != release::PLANNED
            || found.scheduled_at.as_deref() != Some(step.from.as_str())
            || found.slot_pinned_at.is_some()
        {
            return Err(stale("a release from the shift has moved since"));
        }
        tx.execute(
            "UPDATE release SET scheduled_at = ?2, updated_at = ?3 WHERE id = ?1",
            params![step.release_id, step.to, at],
        )?;
    }
    Ok(moves.len())
}

/// Take a shift back: every release it moved returns to the day it left.
///
/// The undo of [`apply_shift`]. Only the last operation is ever undone, so
/// each release is where the shift put it; one that has gone out since keeps
/// the day it went out on.
pub fn shift_back_at(conn: &Connection, moves: &[Move], at: &str) -> Result<()> {
    for step in moves {
        conn.execute(
            "UPDATE release SET scheduled_at = ?2, updated_at = ?3 WHERE id = ?1 AND status = ?4",
            params![step.release_id, step.from, at, release::PLANNED],
        )?;
    }
    Ok(())
}

/// How many days lie from `from` to `to`: negative when `to` is earlier.
pub fn days_between(from: &str, to: &str) -> Result<i64> {
    Ok((parse_date(to)? - parse_date(from)?).whole_days())
}

fn stale(reason: &str) -> Error {
    Error::LayoutStale(format!("the calendar changed since the preview: {reason}"))
}

fn parse_date(date: &str) -> Result<Date> {
    let iso = format_description!("[year]-[month]-[day]");
    Date::parse(date, iso).map_err(|_| Error::refused("layout.badDate").param("value", date))
}

fn iso(date: Date) -> Result<String> {
    let iso = format_description!("[year]-[month]-[day]");
    date.format(iso)
        .map_err(|cause| Error::Internal(format!("could not format a layout date: {cause}")))
}

fn next_day(date: Date) -> Result<Date> {
    date.next_day()
        .ok_or_else(|| Error::Internal("the layout ran off the end of the calendar".into()))
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

    /// A song with a score, and `releases` audio publications made from it,
    /// each with its one queued release - weighed, like any work nobody
    /// judges, by the song's score. All of them bear the song's title, so a
    /// test can tell the songs apart by it.
    fn queued(
        conn: &Connection,
        profile_id: &str,
        title: &str,
        hook: f64,
        releases: usize,
    ) -> Vec<String> {
        let song = work::create(
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
            &song.id,
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
                let made = work::create(
                    conn,
                    profile_id,
                    NewWork {
                        kind: "audio".into(),
                        title: title.into(),
                        ..NewWork::default()
                    },
                )
                .unwrap();
                crate::link::create(
                    conn,
                    profile_id,
                    crate::link::NewLink {
                        work_id: made.id.clone(),
                        source_id: song.id.clone(),
                        role: None,
                        source_version_id: None,
                    },
                )
                .unwrap();
                release::create(
                    conn,
                    NewRelease {
                        work_id: made.id,
                        kind: "youtube".into(),
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
    /// keeps one song's publications from occupying a run of consecutive
    /// days.
    #[test]
    fn a_daily_rhythm_scatters_one_songs_publications_across_alternate_days() {
        let (conn, profile_id) = workspace(1);
        queued(&conn, &profile_id, "Only", 7.0, 3);

        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();

        assert_eq!(dates(&plan), ["2026-09-02", "2026-09-04", "2026-09-06"]);
    }

    /// Two songs on a daily rhythm interleave: the day between two
    /// publications of one song is not wasted when another song can take it.
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
        let (conn, profile_id) = workspace(3);
        queued(&conn, &profile_id, "One", 6.0, 1);
        queued(&conn, &profile_id, "Two", 4.0, 1);

        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();
        let applied = apply(&conn, &plan).unwrap();

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
        let (conn, profile_id) = workspace(3);
        queued(&conn, &profile_id, "One", 6.0, 1);
        queued(&conn, &profile_id, "Two", 4.0, 1);
        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();

        // Someone books the second planned day by hand between the preview
        // and the approval.
        let interloper = queued(&conn, &profile_id, "Interloper", 9.0, 1);
        release::schedule(&conn, &interloper[0], plan[1].date.as_str()).unwrap();

        let refused = apply(&conn, &plan).unwrap_err();
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
        let (conn, profile_id) = workspace(3);
        let ids = queued(&conn, &profile_id, "One", 6.0, 1);
        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();

        release::schedule(&conn, &ids[0], "2026-12-24").unwrap();

        let refused = apply(&conn, &plan).unwrap_err();
        assert_eq!(refused.kind(), "layoutStale", "got {refused}");
        // Its hand-picked date survives the refusal.
        let kept = release::get(&conn, &ids[0]).unwrap().unwrap();
        assert_eq!(kept.scheduled_at.as_deref(), Some("2026-12-24"));
    }

    fn close(conn: &Connection, profile_id: &str, from: &str, to: &str) {
        crate::time_off::create_minted(
            conn,
            profile_id,
            crate::time_off::NewTimeOff {
                starts_on: from.into(),
                ends_on: to.into(),
                label: "Away".into(),
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap();
    }

    /// A fortnight away is stepped over: the plan picks up the rhythm on the
    /// first open day after it. Mutating the `closes` check in `plan` puts
    /// the second release inside the span and breaks this.
    #[test]
    fn the_layout_steps_over_closed_days() {
        let (conn, profile_id) = workspace(3);
        queued(&conn, &profile_id, "Best", 9.0, 1);
        queued(&conn, &profile_id, "Next", 5.0, 1);
        close(&conn, &profile_id, "2026-09-03", "2026-09-10");

        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();

        // The 2nd is open; the 5th would be next, but the 3rd-10th are
        // closed, so the 11th is the first day that is both open and spaced.
        assert_eq!(dates(&plan), ["2026-09-02", "2026-09-11"]);
    }

    /// Tomorrow closed is tomorrow skipped, not tomorrow ignored.
    #[test]
    fn a_closed_tomorrow_moves_the_first_placement() {
        let (conn, profile_id) = workspace(1);
        queued(&conn, &profile_id, "Only", 7.0, 1);
        close(&conn, &profile_id, "2026-09-02", "2026-09-02");

        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();
        assert_eq!(dates(&plan), ["2026-09-03"]);
    }

    /// The preview was drawn before the day was closed: the plan is a
    /// picture of the past, and booking it is refused whole.
    #[test]
    fn a_day_closed_since_the_preview_makes_the_plan_stale() {
        let (conn, profile_id) = workspace(3);
        queued(&conn, &profile_id, "One", 6.0, 1);
        queued(&conn, &profile_id, "Two", 4.0, 1);
        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();

        close(&conn, &profile_id, &plan[1].date, &plan[1].date);

        let refused = apply(&conn, &plan).unwrap_err();
        assert_eq!(refused.kind(), "layoutStale", "got {refused}");
        assert_eq!(release::queue(&conn, &profile_id).unwrap().len(), 2);
    }

    /// A booked plan taken back: every release it placed is queued again.
    #[test]
    fn a_booked_plan_is_taken_back_whole() {
        let (conn, profile_id) = workspace(3);
        queued(&conn, &profile_id, "One", 6.0, 1);
        queued(&conn, &profile_id, "Two", 4.0, 1);
        let plan = plan(&conn, &profile_id, "2026-09-01").unwrap();
        apply(&conn, &plan).unwrap();

        take_back_at(&conn, &plan, "2026-09-01T12:00:00.000Z").unwrap();

        assert_eq!(release::queue(&conn, &profile_id).unwrap().len(), 2);
        assert!(release::calendar(&conn, &profile_id).unwrap().is_empty());
    }

    /// Three releases booked on the 5th, the 8th and the 12th, the 8th pinned.
    fn booked(conn: &Connection, profile_id: &str) -> [String; 3] {
        let first = queued(conn, profile_id, "First", 6.0, 1).remove(0);
        let kept = queued(conn, profile_id, "Kept", 6.0, 1).remove(0);
        let last = queued(conn, profile_id, "Last", 6.0, 1).remove(0);
        release::schedule(conn, &first, "2026-09-05").unwrap();
        release::schedule(conn, &kept, "2026-09-08").unwrap();
        release::set_slot_pin(conn, &kept, true).unwrap();
        release::schedule(conn, &last, "2026-09-12").unwrap();
        [first, kept, last]
    }

    fn scheduled(conn: &Connection, id: &str) -> String {
        release::get(conn, id)
            .unwrap()
            .unwrap()
            .scheduled_at
            .unwrap()
    }

    /// Everything planned from the day on moves by the days asked; a pinned
    /// date is named and stays; what came before the day stays too.
    #[test]
    fn a_shift_moves_what_is_planned_from_the_day_and_keeps_the_pins() {
        let (conn, profile_id) = workspace(3);
        let [first, kept, last] = booked(&conn, &profile_id);

        let plan = plan_shift(&conn, &profile_id, "2026-09-06", 7, "2026-09-01").unwrap();

        assert_eq!(
            plan.moves,
            vec![Move {
                release_id: last.clone(),
                from: "2026-09-12".into(),
                to: "2026-09-19".into(),
            }]
        );
        assert_eq!(plan.kept, vec![kept.clone()]);

        apply_shift(&conn, &plan.moves).unwrap();
        assert_eq!(scheduled(&conn, &first), "2026-09-05", "before the day");
        assert_eq!(scheduled(&conn, &kept), "2026-09-08", "pinned");
        assert_eq!(scheduled(&conn, &last), "2026-09-19");

        shift_back_at(&conn, &plan.moves, "2026-09-02T00:00:00.000Z").unwrap();
        assert_eq!(
            scheduled(&conn, &last),
            "2026-09-12",
            "the undo puts it back"
        );
    }

    /// A shift earlier is the same gesture the other way, and it never puts
    /// anything before today.
    #[test]
    fn a_shift_earlier_stops_at_today() {
        let (conn, profile_id) = workspace(3);
        let [first, _, last] = booked(&conn, &profile_id);

        let plan = plan_shift(&conn, &profile_id, "2026-09-01", -3, "2026-09-01").unwrap();
        assert_eq!(plan.moves.len(), 2);
        apply_shift(&conn, &plan.moves).unwrap();
        assert_eq!(scheduled(&conn, &first), "2026-09-02");
        assert_eq!(scheduled(&conn, &last), "2026-09-09");

        let refused = plan_shift(&conn, &profile_id, "2026-09-01", -2, "2026-09-01").unwrap_err();
        assert_eq!(
            refused.refusal().map(|refusal| refusal.code),
            Some("layout.shiftIntoThePast")
        );
        let refused = plan_shift(&conn, &profile_id, "2026-09-01", 0, "2026-09-01").unwrap_err();
        assert_eq!(
            refused.refusal().map(|refusal| refusal.code),
            Some("layout.shiftByNothing")
        );
    }

    /// Landing on a closed day is said, not stepped over: the rhythm between
    /// the releases is what a shift keeps.
    #[test]
    fn a_shift_names_what_it_would_put_on_a_closed_day() {
        let (conn, profile_id) = workspace(3);
        let [first, _, last] = booked(&conn, &profile_id);
        close(&conn, &profile_id, "2026-09-12", "2026-09-12");

        let plan = plan_shift(&conn, &profile_id, "2026-09-01", 7, "2026-09-01").unwrap();

        assert_eq!(
            plan.closed,
            vec![first.clone()],
            "the 5th lands on the 12th"
        );
        assert!(
            plan.moves
                .iter()
                .any(|step| step.release_id == last && step.to == "2026-09-19")
        );
    }

    /// A release moved by hand between the preview and the approval: the
    /// shift is refused whole.
    #[test]
    fn a_shift_drawn_on_a_calendar_that_moved_is_refused_whole() {
        let (conn, profile_id) = workspace(3);
        let [first, _, last] = booked(&conn, &profile_id);
        let plan = plan_shift(&conn, &profile_id, "2026-09-01", 7, "2026-09-01").unwrap();

        release::schedule(&conn, &last, "2026-09-30").unwrap();

        let refused = apply_shift(&conn, &plan.moves).unwrap_err();
        assert_eq!(refused.kind(), "layoutStale", "got {refused}");
        assert_eq!(scheduled(&conn, &first), "2026-09-05", "nothing half-moved");
    }
}
