//! How often a kind is meant to go out: the rhythm and its monthly goals.

use serde::{Deserialize, Serialize};

/// The pace releases go out at.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Rhythm {
    /// Days the auto-layout keeps between releases. 1 is daily.
    pub every_days: u32,
    /// Time of day a release usually ships (HH:MM), shown beside the date when
    /// editing a release. Slots themselves stay dates: the contest is per day,
    /// and a time would split it.
    #[serde(default)]
    pub default_time: Option<String>,
    /// What a month should hold, by tier or by kind of work (v0.111): "four of
    /// the top tier a month", "eight shorts a month". `every_days` paces the
    /// whole output and says nothing of what it is made of; a month can keep
    /// the pace on shorts alone. The window says which goal a month falls
    /// short of, and by how many. Empty for a profile that sets none.
    #[serde(default)]
    pub monthly: Vec<MonthlyGoal>,
    /// How many days before a planned slot's day it starts to remind
    /// (v0.138, FR-11), at least one.
    #[serde(default = "seven", skip_serializing_if = "is_seven")]
    #[ts(optional, as = "Option<u32>")]
    pub plan_lead_days: u32,
    /// How near a planned slot's day something made from its work must be
    /// booked to close it, in days either side (v0.138).
    #[serde(default = "seven", skip_serializing_if = "is_seven")]
    #[ts(optional, as = "Option<u32>")]
    pub plan_match_window_days: u32,
    /// The days of the week a release goes out on (v0.150, FR-20): Tuesday,
    /// Thursday and Saturday. When any are named, the layout keeps to them
    /// and `every_days` is not read.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<Weekday>>")]
    pub weekdays: Vec<Weekday>,
    /// How many releases a month is meant to hold on those days - thirteen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub monthly_target: Option<u32>,
    /// What a month with more of those days than the target does.
    #[serde(default, skip_serializing_if = "Overflow::is_keep_all")]
    #[ts(optional, as = "Option<Overflow>")]
    pub overflow: Overflow,
    /// What a month with fewer of those days than the target does.
    #[serde(default, skip_serializing_if = "Shortage::is_keep_grid")]
    #[ts(optional, as = "Option<Shortage>")]
    pub shortage: Shortage,
    /// The channel's time zone, by its IANA name - `America/Asuncion`: the
    /// hour a release goes out is that zone's, summer time and all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub timezone: Option<String>,
}

impl Default for Rhythm {
    /// A release a day, at no set hour, with the reminders' own windows.
    fn default() -> Self {
        Self {
            every_days: 1,
            default_time: None,
            monthly: Vec::new(),
            plan_lead_days: 7,
            plan_match_window_days: 7,
            weekdays: Vec::new(),
            monthly_target: None,
            overflow: Overflow::KeepAll,
            shortage: Shortage::KeepGrid,
            timezone: None,
        }
    }
}

/// A day of the week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    /// The day of the week of `date`.
    pub fn of(date: time::Date) -> Self {
        match date.weekday() {
            time::Weekday::Monday => Self::Mon,
            time::Weekday::Tuesday => Self::Tue,
            time::Weekday::Wednesday => Self::Wed,
            time::Weekday::Thursday => Self::Thu,
            time::Weekday::Friday => Self::Fri,
            time::Weekday::Saturday => Self::Sat,
            time::Weekday::Sunday => Self::Sun,
        }
    }
}

/// What a month with more release days than its target does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Overflow {
    /// Every day of the grid is a slot: fourteen in a month of fourteen.
    #[default]
    KeepAll,
    /// The surplus is left out - the days that stand closest to a slot
    /// before them, the ones a listener misses least.
    SkipWeakest,
}

impl Overflow {
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn is_keep_all(&self) -> bool {
        *self == Self::KeepAll
    }
}

/// What a month with fewer release days than its target does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Shortage {
    /// The grid as it falls: twelve in a month of twelve.
    #[default]
    KeepGrid,
    /// A floating slot is added for each one short, in the middle of the
    /// widest gap between the grid's days - where the month's peak can go.
    AddFloatingPeak,
}

impl Shortage {
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn is_keep_grid(&self) -> bool {
        *self == Self::KeepGrid
    }
}

/// A day of a month a release is meant for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct MonthSlot {
    #[ts(type = "string")]
    pub date: time::Date,
    /// Not one of the grid's days: added for a month short of its target.
    pub floating: bool,
}

impl Rhythm {
    /// Whether the rhythm keeps to days of the week rather than to a spacing.
    pub fn by_weekdays(&self) -> bool {
        !self.weekdays.is_empty()
    }

    /// The days of `month` of `year` a release is meant for, by the grid of
    /// weekdays, the target and its rules. Empty for a rhythm of spacing.
    pub fn month_slots(&self, year: i32, month: time::Month) -> Vec<MonthSlot> {
        if !self.by_weekdays() {
            return Vec::new();
        }
        let Ok(first) = time::Date::from_calendar_date(year, month, 1) else {
            return Vec::new();
        };
        let mut grid: Vec<time::Date> = Vec::new();
        let mut day = first;
        while day.month() == month {
            if self.weekdays.contains(&Weekday::of(day)) {
                grid.push(day);
            }
            match day.next_day() {
                Some(next) => day = next,
                None => break,
            }
        }
        let target = self.monthly_target.map(|target| target as usize);
        if let Some(target) = target
            && grid.len() > target
            && self.overflow == Overflow::SkipWeakest
        {
            while grid.len() > target {
                // The weakest slot: the one closest to the slot before it.
                let weakest = (1..grid.len())
                    .min_by_key(|&at| {
                        (
                            (grid[at] - grid[at - 1]).whole_days(),
                            std::cmp::Reverse(at),
                        )
                    })
                    .unwrap_or(grid.len() - 1);
                grid.remove(weakest);
            }
        }
        let mut slots: Vec<MonthSlot> = grid
            .iter()
            .map(|&date| MonthSlot {
                date,
                floating: false,
            })
            .collect();
        if let Some(target) = target
            && slots.len() < target
            && self.shortage == Shortage::AddFloatingPeak
        {
            let last = first
                .replace_day(time::util::days_in_month(month, year))
                .unwrap_or(first);
            while slots.len() < target {
                // The middle of the widest gap, the month's edges counted.
                let mut edges: Vec<time::Date> = vec![first - time::Duration::DAY];
                edges.extend(slots.iter().map(|slot| slot.date));
                edges.push(last + time::Duration::DAY);
                let Some((at, _)) = edges
                    .windows(2)
                    .enumerate()
                    .max_by_key(|(_, pair)| (pair[1] - pair[0]).whole_days())
                else {
                    break;
                };
                let gap = (edges[at + 1] - edges[at]).whole_days();
                if gap < 2 {
                    break;
                }
                let date = edges[at] + time::Duration::days(gap / 2);
                slots.push(MonthSlot {
                    date,
                    floating: true,
                });
                slots.sort_by_key(|slot| slot.date);
            }
        }
        slots
    }

    /// The offset of the rhythm's time zone on `date` at `clock` (`HH:MM`),
    /// as an ISO offset - `-03:00`; none without a zone or for a name the
    /// zone database does not know.
    pub fn offset_on(&self, date: time::Date, clock: &str) -> Option<String> {
        let zone = jiff::tz::TimeZone::get(self.timezone.as_deref()?).ok()?;
        let (hour, minute) = clock.split_once(':')?;
        let at = jiff::civil::date(
            i16::try_from(date.year()).ok()?,
            i8::try_from(u8::from(date.month())).ok()?,
            i8::try_from(date.day()).ok()?,
        )
        .at(hour.parse().ok()?, minute.parse().ok()?, 0, 0);
        let zoned = at.to_zoned_datetime(zone).ok()?;
        let seconds = zoned.offset().seconds();
        let sign = if seconds < 0 { '-' } else { '+' };
        let minutes = seconds.unsigned_abs() / 60;
        Some(format!("{sign}{:02}:{:02}", minutes / 60, minutes % 60))
    }
}

/// Whether `name` is a time zone the zone database knows.
pub fn is_time_zone(name: &str) -> bool {
    jiff::tz::TimeZone::get(name).is_ok()
}

fn seven() -> u32 {
    7
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_seven(days: &u32) -> bool {
    *days == 7
}

/// How many releases of one sort a month should hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct MonthlyGoal {
    /// What is counted.
    #[serde(flatten)]
    pub of: GoalOf,
    /// How many a month, at least one.
    pub count: u32,
}

/// What a monthly goal counts: one or the other, never both and never
/// neither - a goal "of the top tier and of shorts" is two goals, and a goal
/// of nothing counts nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum GoalOf {
    /// Releases whose tier this is - the tier of what goes out, which a clip,
    /// a short and an audio release take from their song.
    Tier(String),
    /// Releases of works of this kind.
    Kind(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Month, macros::date};

    fn weekly(target: Option<u32>, overflow: Overflow, shortage: Shortage) -> Rhythm {
        Rhythm {
            weekdays: vec![Weekday::Tue, Weekday::Thu, Weekday::Sat],
            monthly_target: target,
            overflow,
            shortage,
            ..Rhythm::default()
        }
    }

    #[test]
    fn a_month_of_fourteen_grid_days_skips_the_weakest_down_to_its_target() {
        // December 2026: five Tuesdays, five Thursdays, four Saturdays.
        let all =
            weekly(None, Overflow::KeepAll, Shortage::KeepGrid).month_slots(2026, Month::December);
        assert_eq!(all.len(), 14);
        let kept = weekly(Some(13), Overflow::SkipWeakest, Shortage::KeepGrid)
            .month_slots(2026, Month::December);
        assert_eq!(kept.len(), 13);
        assert!(kept.iter().all(|slot| !slot.floating));
        // The surplus that goes is a day two days after the slot before it.
        let gone: Vec<_> = all.iter().filter(|slot| !kept.contains(slot)).collect();
        assert_eq!(gone.len(), 1);
        let before = all
            .iter()
            .rev()
            .find(|slot| slot.date < gone[0].date)
            .unwrap();
        assert_eq!((gone[0].date - before.date).whole_days(), 2);
    }

    #[test]
    fn a_month_of_twelve_grid_days_gains_a_floating_slot_in_its_widest_gap() {
        // February 2026: four of each.
        let rhythm = weekly(Some(13), Overflow::KeepAll, Shortage::AddFloatingPeak);
        let slots = rhythm.month_slots(2026, Month::February);
        assert_eq!(slots.len(), 13);
        let floating: Vec<_> = slots.iter().filter(|slot| slot.floating).collect();
        assert_eq!(floating.len(), 1);
        assert!(!rhythm.weekdays.contains(&Weekday::of(floating[0].date)));
        let kept = weekly(Some(13), Overflow::KeepAll, Shortage::KeepGrid)
            .month_slots(2026, Month::February);
        assert_eq!(kept.len(), 12, "the grid as it falls");
        assert!(
            Rhythm::default()
                .month_slots(2026, Month::February)
                .is_empty()
        );
    }

    #[test]
    fn the_zone_gives_the_hour_its_offset_summer_time_and_all() {
        let rhythm = Rhythm {
            timezone: Some("Europe/Berlin".into()),
            ..Rhythm::default()
        };
        assert_eq!(
            rhythm.offset_on(date!(2027 - 01 - 12), "12:00").as_deref(),
            Some("+01:00")
        );
        assert_eq!(
            rhythm.offset_on(date!(2027 - 07 - 12), "12:00").as_deref(),
            Some("+02:00")
        );
        assert_eq!(
            Rhythm::default().offset_on(date!(2027 - 07 - 12), "12:00"),
            None
        );
        assert!(is_time_zone("America/Asuncion"));
        assert!(!is_time_zone("Mars/Olympus"));
    }
}
