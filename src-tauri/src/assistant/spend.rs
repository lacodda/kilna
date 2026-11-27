//! What the assistant's runs cost, by the day and by the week, against a
//! budget the person sets for the week (v0.107).
//!
//! The price of a run is the CLI's own `total_cost_usd`, kept on the run's
//! row ([`super::run`]). For a person on a subscription it is what the turns
//! would have cost through the API - a measure of how much was asked, not a
//! bill; for a person paying by the token it is the bill. Either way it is
//! the number the CLI gives, said as it gives it.
//!
//! The budget is soft. Nothing is refused for crossing it: the week's line in
//! the assistant's head turns, the history gets one line the first time the
//! week goes over, and the window says so once - inside itself or, when it is
//! not in front, through the system.
//!
//! The day and the week are the person's, not UTC's. The window sends the
//! moments its own midnight and its own Monday began, and the runs are
//! counted from there: after sunset here UTC is already tomorrow (v0.23).

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::error::{Error, Result};
use crate::journal::{self, Record};
use crate::time::now;

/// What the runs have cost, and what the week may.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, ts_rs::TS)]
pub struct Spend {
    /// Since the person's midnight, in US dollars.
    pub today: f64,
    /// Since the person's Monday.
    pub week: f64,
    /// What the week may cost before the window says so; none when unset.
    pub budget: Option<f64>,
}

impl Spend {
    /// Whether the week has cost more than its budget.
    pub fn over(&self) -> bool {
        self.budget.is_some_and(|budget| self.week > budget)
    }
}

/// The span the window counts from: its midnight and its Monday, as moments.
#[derive(Debug, Clone, Copy)]
pub struct Since<'a> {
    pub day: &'a str,
    pub week: &'a str,
}

/// A day and a week of runs, every chat of every space together: the money is
/// the person's whatever space asked, and the budget is theirs, in the
/// application's database (ADR 0077). `spaces` are the databases to count -
/// the open one first; a space that cannot be counted is said in the log and
/// left out rather than failing the sum.
pub fn read(spaces: &[&Connection], app: &Connection, since: Since<'_>) -> Result<Spend> {
    let day = moment(since.day)?;
    let week = moment(since.week)?;
    let spent = |from: &str| -> Result<f64> {
        let mut total = 0.0;
        for (n, conn) in spaces.iter().enumerate() {
            let counted: rusqlite::Result<f64> = conn.query_row(
                "SELECT coalesce(sum(cost_usd), 0) FROM chat_run \
                  WHERE cost_usd IS NOT NULL AND ended_at >= ?1",
                params![from],
                |row| row.get(0),
            );
            match counted {
                Ok(spent) => total += spent,
                // The open space must count: its failure is a fault.
                Err(cause) if n == 0 => return Err(cause.into()),
                Err(cause) => crate::log::warn(
                    "assistant",
                    &format!("a space's runs could not be counted: {cause}"),
                ),
            }
        }
        Ok(total)
    };
    Ok(Spend {
        today: spent(&day)?,
        week: spent(&week)?,
        budget: budget(app)?,
    })
}

/// Whether the person's week has cost more than it may: what a run nobody
/// asked for checks before it starts (FR-01). Every space counts, as in
/// [`read`] - the money is one person's whichever space spent it.
///
/// `week_from` is when the week began, as the window last said it. A window
/// left open over a Monday still says last week's, so a week older than
/// seven days is moved on by whole weeks; with none said - nothing has asked
/// yet - the week is the last seven days.
pub fn over_lately(
    spaces: &[&Connection],
    app: &Connection,
    week_from: Option<&str>,
) -> Result<bool> {
    let Some(budget) = budget(app)? else {
        return Ok(false);
    };
    let now = time::OffsetDateTime::now_utc();
    // In the shape every stored moment has: they are compared as text.
    let since = week_start(week_from, now)
        .format(&time::format_description::well_known::Rfc3339)
        .ok()
        .and_then(|text| crate::time::instant(&text))
        .unwrap_or_default();
    let mut spent = 0.0;
    for (n, conn) in spaces.iter().enumerate() {
        let counted: rusqlite::Result<f64> = conn.query_row(
            "SELECT coalesce(sum(cost_usd), 0) FROM chat_run \
              WHERE cost_usd IS NOT NULL AND ended_at >= ?1",
            params![since],
            |row| row.get(0),
        );
        match counted {
            Ok(cost) => spent += cost,
            // The open space must count: its failure is a fault.
            Err(cause) if n == 0 => return Err(cause.into()),
            Err(cause) => crate::log::warn(
                "assistant",
                &format!("a space's runs could not be counted: {cause}"),
            ),
        }
    }
    Ok(spent > budget)
}

/// The start of the week `now` falls in, given a start the window once
/// said: that moment, moved on by whole weeks until `now` is within seven
/// days of it. Seven days back when none was said, it cannot be read, or it
/// lies ahead.
fn week_start(said: Option<&str>, now: time::OffsetDateTime) -> time::OffsetDateTime {
    let week = time::Duration::days(7);
    let said = said.and_then(|text| {
        time::OffsetDateTime::parse(text, &time::format_description::well_known::Rfc3339).ok()
    });
    match said {
        Some(start) if start <= now => {
            let whole_weeks = (now - start).whole_days() / 7;
            start + week * i32::try_from(whole_weeks).unwrap_or(0)
        }
        _ => now - week,
    }
}

/// What one chat's runs have cost, all of them.
pub fn of_chat(conn: &Connection, chat_id: &str) -> Result<f64> {
    Ok(conn.query_row(
        "SELECT coalesce(sum(cost_usd), 0) FROM chat_run WHERE chat_id = ?1",
        params![chat_id],
        |row| row.get(0),
    )?)
}

/// The week's budget, when one is set.
pub fn budget(conn: &Connection) -> Result<Option<f64>> {
    Ok(conn
        .query_row(
            "SELECT weekly_budget_usd FROM assistant_settings WHERE id = 1",
            [],
            |row| row.get::<_, Option<f64>>(0),
        )
        .optional()?
        .flatten())
}

/// Set the week's budget, or clear it with `None`. A budget of nothing, of
/// less than nothing or of no number at all is refused rather than stored:
/// the column would refuse it too, in words nobody should read.
pub fn set_budget(conn: &Connection, budget: Option<f64>) -> Result<Option<f64>> {
    if let Some(amount) = budget
        && !(amount.is_finite() && amount > 0.0)
    {
        return Err(Error::refused("spend.badBudget").param("value", amount.to_string()));
    }
    conn.execute(
        "INSERT INTO assistant_settings (id, weekly_budget_usd, updated_at) VALUES (1, ?1, ?2)
         ON CONFLICT (id) DO UPDATE SET
             weekly_budget_usd = excluded.weekly_budget_usd, updated_at = excluded.updated_at",
        params![budget, now()],
    )?;
    Ok(budget)
}

/// What [`check`] found.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, ts_rs::TS)]
pub struct SpendCheck {
    pub spend: Spend,
    /// The week went over its budget, and this call was the first to see it:
    /// the history gained its line, and the window says it once.
    pub crossed: bool,
}

/// Read the spend, and write the week's crossing into the history the first
/// time it is seen. Asked by the window when a run ends and at startup.
///
/// Once per week and budget: raising the budget and going over it again is a
/// new crossing, the same one noticed by the next run is not.
///
/// The line goes into the history of the open space - `spaces[0]` - the one
/// the window shows when it says so.
pub fn check(
    spaces: &[&Connection],
    app: &Connection,
    profile_id: &str,
    since: Since<'_>,
) -> Result<SpendCheck> {
    let spend = read(spaces, app, since)?;
    let Some(conn) = spaces.first() else {
        return Ok(SpendCheck {
            spend,
            crossed: false,
        });
    };
    let crossed = match spend.budget {
        Some(budget) if spend.over() => journal::record_news(
            conn,
            profile_id,
            Record::new("assistant.overBudget")
                .param("spent", spend.week)
                .param("budget", budget)
                .once(format!("budget:{}:{budget}", moment(since.week)?))
                .warn(),
        ),
        _ => false,
    };
    Ok(SpendCheck { spend, crossed })
}

fn moment(text: &str) -> Result<String> {
    crate::time::instant(text).ok_or_else(|| Error::refused("spend.badMoment").param("value", text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::{self, NewChat};
    use crate::fixtures;

    const MONDAY: &str = "2026-11-02T03:00:00.000Z";
    const TODAY: &str = "2026-11-04T03:00:00.000Z";
    const SINCE: Since<'static> = Since {
        day: TODAY,
        week: MONDAY,
    };

    /// A run that ended at `ended_at`, priced at `cost` (or not priced).
    fn run(conn: &Connection, chat_id: &str, ended_at: &str, cost: Option<f64>) {
        conn.execute(
            "INSERT INTO chat_run (id, chat_id, prompt, state, events, started_at, ended_at, cost_usd)
             VALUES (?1, ?2, 'q', 'done', '[]', ?3, ?3, ?4)",
            params![uuid::Uuid::new_v4().to_string(), chat_id, ended_at, cost],
        )
        .unwrap();
    }

    fn chat(conn: &Connection, profile_id: &str) -> String {
        assistant::create(conn, profile_id, NewChat::default())
            .unwrap()
            .id
    }

    #[test]
    fn the_day_and_the_week_count_from_the_windows_own_midnight_and_monday() {
        let (conn, profile_id) = fixtures::workspace();
        let app = crate::db::open_app_in_memory().unwrap();
        let chat_id = chat(&conn, &profile_id);
        run(&conn, &chat_id, "2026-11-01T23:59:59.999Z", Some(5.0)); // last week
        run(&conn, &chat_id, MONDAY, Some(1.0)); // the first moment of the week
        run(&conn, &chat_id, "2026-11-04T02:59:59.999Z", Some(0.5)); // yesterday, here
        run(&conn, &chat_id, TODAY, Some(0.25));
        run(&conn, &chat_id, "2026-11-04T18:00:00.000Z", None); // stopped, unpriced

        let spend = read(&[&conn], &app, SINCE).unwrap();
        assert!((spend.today - 0.25).abs() < 1e-9, "{spend:?}");
        assert!((spend.week - 1.75).abs() < 1e-9, "{spend:?}");
        assert_eq!(spend.budget, None);
    }

    /// Two spaces are one wallet: what a run of the other cost counts with
    /// this one's, against the one budget.
    #[test]
    fn every_spaces_runs_are_the_same_money() {
        let (here, here_profile) = fixtures::workspace();
        let (there, there_profile) = fixtures::workspace();
        let app = crate::db::open_app_in_memory().unwrap();
        run(&here, &chat(&here, &here_profile), TODAY, Some(1.0));
        run(&there, &chat(&there, &there_profile), TODAY, Some(2.0));

        assert!((read(&[&here, &there], &app, SINCE).unwrap().today - 3.0).abs() < 1e-9);
    }

    /// A space that cannot be counted - a database from before the price was
    /// kept - is left out of the sum, not a reason to refuse it; the open
    /// one is.
    #[test]
    fn a_space_that_cannot_be_counted_is_left_out() {
        let (here, profile_id) = fixtures::workspace();
        let app = crate::db::open_app_in_memory().unwrap();
        let odd = Connection::open_in_memory().unwrap();
        run(&here, &chat(&here, &profile_id), TODAY, Some(1.0));

        assert!((read(&[&here, &odd], &app, SINCE).unwrap().today - 1.0).abs() < 1e-9);
        assert!(read(&[&odd, &here], &app, SINCE).is_err());
    }

    #[test]
    fn a_budget_is_set_changed_and_cleared_and_never_nonsense() {
        let app = crate::db::open_app_in_memory().unwrap();
        assert_eq!(budget(&app).unwrap(), None);

        set_budget(&app, Some(10.0)).unwrap();
        set_budget(&app, Some(12.5)).unwrap();
        assert_eq!(budget(&app).unwrap(), Some(12.5));

        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(set_budget(&app, Some(bad)).is_err(), "{bad}");
        }
        assert_eq!(
            budget(&app).unwrap(),
            Some(12.5),
            "a refusal changes nothing"
        );

        set_budget(&app, None).unwrap();
        assert_eq!(budget(&app).unwrap(), None);
    }

    /// The schema holds the bound too, for a write that does not come
    /// through [`set_budget`].
    #[test]
    fn the_table_refuses_a_budget_of_nothing() {
        let app = crate::db::open_app_in_memory().unwrap();
        let refused = app.execute(
            "INSERT INTO assistant_settings (id, weekly_budget_usd, updated_at) VALUES (1, 0, 'x')",
            [],
        );
        assert!(refused.is_err());
        let second = app.execute(
            "INSERT INTO assistant_settings (id, weekly_budget_usd, updated_at) VALUES (2, 1, 'x')",
            [],
        );
        assert!(second.is_err(), "one row of settings, not two");
    }

    #[test]
    fn crossing_the_budget_is_said_once_a_week_and_budget() {
        let (conn, profile_id) = fixtures::workspace();
        let app = crate::db::open_app_in_memory().unwrap();
        let chat_id = chat(&conn, &profile_id);
        set_budget(&app, Some(1.0)).unwrap();
        run(&conn, &chat_id, TODAY, Some(0.75));

        let under = check(&[&conn], &app, &profile_id, SINCE).unwrap();
        assert!(!under.crossed && !under.spend.over());

        run(&conn, &chat_id, TODAY, Some(0.5));
        let over = check(&[&conn], &app, &profile_id, SINCE).unwrap();
        assert!(over.crossed && over.spend.over());
        let again = check(&[&conn], &app, &profile_id, SINCE).unwrap();
        assert!(!again.crossed, "the same crossing is said once");

        // A higher budget crossed again is a new thing to say.
        set_budget(&app, Some(1.2)).unwrap();
        assert!(check(&[&conn], &app, &profile_id, SINCE).unwrap().crossed);

        let lines = journal::list(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .filter(|entry| entry.action == "assistant.overBudget")
            .count();
        assert_eq!(lines, 2);
    }

    #[test]
    fn a_moment_that_is_not_one_is_refused() {
        let (conn, _) = fixtures::workspace();
        let app = crate::db::open_app_in_memory().unwrap();
        let bad = Since {
            day: "today",
            week: MONDAY,
        };
        assert!(read(&[&conn], &app, bad).is_err());
    }

    #[test]
    fn a_week_said_long_ago_is_moved_on_by_whole_weeks() {
        let at = |text: &str| {
            time::OffsetDateTime::parse(text, &time::format_description::well_known::Rfc3339)
                .unwrap()
        };
        let now = at("2026-11-18T12:00:00Z");
        // This week's Monday stands.
        assert_eq!(
            week_start(Some(MONDAY), at("2026-11-04T12:00:00Z")),
            at(MONDAY)
        );
        // A window left open over two Mondays: the week moved on with it.
        assert_eq!(week_start(Some(MONDAY), now), at("2026-11-16T03:00:00Z"));
        // Nothing said, nonsense, or a moment ahead: the last seven days.
        let back = now - time::Duration::days(7);
        assert_eq!(week_start(None, now), back);
        assert_eq!(week_start(Some("monday"), now), back);
        assert_eq!(week_start(Some("2027-01-01T00:00:00Z"), now), back);
    }

    #[test]
    fn a_run_nobody_asked_for_counts_every_space_since_the_persons_week() {
        let (open, profile_id) = fixtures::workspace();
        let (other, other_profile) = fixtures::workspace();
        let app = crate::db::open_app_in_memory().unwrap();
        let spaces = [&open, &other];
        let here = chat(&open, &profile_id);
        let there = chat(&other, &other_profile);
        let now = time::OffsetDateTime::now_utc();
        let ago = |days: i64| {
            crate::time::instant(
                &(now - time::Duration::days(days))
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap(),
            )
            .unwrap()
        };

        // No budget: nothing is over it.
        run(&open, &here, &ago(1), Some(50.0));
        assert!(!over_lately(&spaces, &app, None).unwrap());

        set_budget(&app, Some(60.0)).unwrap();
        assert!(!over_lately(&spaces, &app, None).unwrap());
        // The other space's runs are the same money.
        run(&other, &there, &ago(1), Some(20.0));
        assert!(over_lately(&spaces, &app, None).unwrap());
        assert!(!over_lately(&[&open], &app, None).unwrap());
        // A week that began after those runs has cost nothing yet.
        assert!(!over_lately(&spaces, &app, Some(&ago(0))).unwrap());
        // And one that began three days ago counts them.
        assert!(over_lately(&spaces, &app, Some(&ago(3))).unwrap());
    }
}
