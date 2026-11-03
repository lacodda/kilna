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

/// A day and a week of runs, every chat of the workspace together: the money
/// is the person's whatever profile asked.
pub fn read(conn: &Connection, since: Since<'_>) -> Result<Spend> {
    let day = moment(since.day)?;
    let week = moment(since.week)?;
    let spent = |from: &str| -> Result<f64> {
        Ok(conn.query_row(
            "SELECT coalesce(sum(cost_usd), 0) FROM chat_run \
              WHERE cost_usd IS NOT NULL AND ended_at >= ?1",
            params![from],
            |row| row.get(0),
        )?)
    };
    Ok(Spend {
        today: spent(&day)?,
        week: spent(&week)?,
        budget: budget(conn)?,
    })
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
pub fn check(conn: &Connection, profile_id: &str, since: Since<'_>) -> Result<SpendCheck> {
    let spend = read(conn, since)?;
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
        let chat_id = chat(&conn, &profile_id);
        run(&conn, &chat_id, "2026-11-01T23:59:59.999Z", Some(5.0)); // last week
        run(&conn, &chat_id, MONDAY, Some(1.0)); // the first moment of the week
        run(&conn, &chat_id, "2026-11-04T02:59:59.999Z", Some(0.5)); // yesterday, here
        run(&conn, &chat_id, TODAY, Some(0.25));
        run(&conn, &chat_id, "2026-11-04T18:00:00.000Z", None); // stopped, unpriced

        let spend = read(&conn, SINCE).unwrap();
        assert!((spend.today - 0.25).abs() < 1e-9, "{spend:?}");
        assert!((spend.week - 1.75).abs() < 1e-9, "{spend:?}");
        assert_eq!(spend.budget, None);
    }

    #[test]
    fn every_profiles_runs_are_the_same_money() {
        let (conn, profile_id) = fixtures::workspace();
        conn.execute(
            "INSERT INTO profile (id, key, name, config, is_active, is_builtin, created_at, updated_at)
             SELECT 'other', 'other', 'Other', config, 0, 0, created_at, updated_at FROM profile LIMIT 1",
            [],
        )
        .unwrap();
        run(&conn, &chat(&conn, &profile_id), TODAY, Some(1.0));
        run(&conn, &chat(&conn, "other"), TODAY, Some(2.0));

        assert!((read(&conn, SINCE).unwrap().today - 3.0).abs() < 1e-9);
    }

    #[test]
    fn a_budget_is_set_changed_and_cleared_and_never_nonsense() {
        let (conn, _) = fixtures::workspace();
        assert_eq!(budget(&conn).unwrap(), None);

        set_budget(&conn, Some(10.0)).unwrap();
        set_budget(&conn, Some(12.5)).unwrap();
        assert_eq!(budget(&conn).unwrap(), Some(12.5));

        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(set_budget(&conn, Some(bad)).is_err(), "{bad}");
        }
        assert_eq!(
            budget(&conn).unwrap(),
            Some(12.5),
            "a refusal changes nothing"
        );

        set_budget(&conn, None).unwrap();
        assert_eq!(budget(&conn).unwrap(), None);
    }

    /// The schema holds the bound too, for a write that does not come
    /// through [`set_budget`].
    #[test]
    fn the_table_refuses_a_budget_of_nothing() {
        let (conn, _) = fixtures::workspace();
        let refused = conn.execute(
            "INSERT INTO assistant_settings (id, weekly_budget_usd, updated_at) VALUES (1, 0, 'x')",
            [],
        );
        assert!(refused.is_err());
        let second = conn.execute(
            "INSERT INTO assistant_settings (id, weekly_budget_usd, updated_at) VALUES (2, 1, 'x')",
            [],
        );
        assert!(second.is_err(), "one row of settings, not two");
    }

    #[test]
    fn crossing_the_budget_is_said_once_a_week_and_budget() {
        let (conn, profile_id) = fixtures::workspace();
        let chat_id = chat(&conn, &profile_id);
        set_budget(&conn, Some(1.0)).unwrap();
        run(&conn, &chat_id, TODAY, Some(0.75));

        let under = check(&conn, &profile_id, SINCE).unwrap();
        assert!(!under.crossed && !under.spend.over());

        run(&conn, &chat_id, TODAY, Some(0.5));
        let over = check(&conn, &profile_id, SINCE).unwrap();
        assert!(over.crossed && over.spend.over());
        let again = check(&conn, &profile_id, SINCE).unwrap();
        assert!(!again.crossed, "the same crossing is said once");

        // A higher budget crossed again is a new thing to say.
        set_budget(&conn, Some(1.2)).unwrap();
        assert!(check(&conn, &profile_id, SINCE).unwrap().crossed);

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
        let bad = Since {
            day: "today",
            week: MONDAY,
        };
        assert!(read(&conn, bad).is_err());
    }
}
