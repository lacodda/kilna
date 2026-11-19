//! A pool of themes used in turn (v0.151, FR-20 20.2).
//!
//! An item of a bank is a fact of a section shaped `bank`: its words are the
//! item's name - a tradition, a region, a rubric - and its data the group it
//! belongs to, what it is made of, how many months it rests after a use, when
//! it was last used and by which works, and whether it is locked for good. A
//! track that takes a tradition from the bank takes it out of the pool for
//! those months: the pool is there so the channel does not reach for the same
//! few every time.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};

/// The keys an item's data may carry.
pub const DATA_KEYS: &[&str] = &[
    "group",
    "attributes",
    "cooldown_months",
    "last_used_at",
    "used_in",
    "locked",
    "note",
];

/// The most months an item may rest: ten years is a lock, said otherwise.
const LONGEST_REST: u64 = 120;

/// A work that used an item, and the day it did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct BankUse {
    pub work_id: String,
    pub title: String,
    /// `YYYY-MM-DD`.
    pub date: String,
}

/// Where an item stands on a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum BankState {
    /// Free to take.
    Available,
    /// Resting after a use, until a month.
    Cooldown,
    /// Taken out of the pool for good.
    Locked,
}

/// An item's data as it is written down: checked, and in one spelling.
pub fn checked(data: &Map<String, Value>) -> Result<Map<String, Value>> {
    let text = |key: &str| {
        data.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    let mut out = Map::new();
    if let Some(group) = text("group") {
        out.insert("group".into(), Value::String(group));
    }
    let attributes: Vec<String> = match data.get("attributes") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::String(line)) => line.split(',').map(|one| one.trim().to_owned()).collect(),
        Some(Value::Array(many)) => many
            .iter()
            .filter_map(Value::as_str)
            .map(|one| one.trim().to_owned())
            .collect(),
        Some(_) => return Err(Error::refused("canon.bankBadAttributes")),
    };
    let attributes: Vec<Value> = attributes
        .into_iter()
        .filter(|one| !one.is_empty())
        .map(Value::String)
        .collect();
    if !attributes.is_empty() {
        out.insert("attributes".into(), Value::Array(attributes));
    }
    match data.get("cooldown_months") {
        None | Some(Value::Null) => {}
        Some(value) => {
            let months = value
                .as_u64()
                .filter(|months| *months <= LONGEST_REST)
                .ok_or_else(|| Error::refused("canon.bankBadCooldown"))?;
            out.insert("cooldown_months".into(), Value::from(months));
        }
    }
    let mut last = match text("last_used_at") {
        None => None,
        Some(day) if crate::time::is_date(&day) => Some(day),
        Some(day) => return Err(Error::refused("canon.bankBadDay").param("day", day)),
    };
    let uses: Vec<BankUse> = match data.get("used_in") {
        None | Some(Value::Null) => Vec::new(),
        Some(value) => {
            serde_json::from_value(value.clone()).map_err(|_| Error::refused("canon.bankBadUse"))?
        }
    };
    let mut kept: Vec<BankUse> = Vec::new();
    for one in uses {
        let date = one.date.trim().to_owned();
        if !crate::time::is_date(&date) {
            return Err(Error::refused("canon.bankBadDay").param("day", date));
        }
        let work_id = one.work_id.trim().to_owned();
        if work_id.is_empty() {
            return Err(Error::refused("canon.bankBadUse"));
        }
        // A work uses an item once: the last word on its day wins.
        kept.retain(|other| other.work_id != work_id);
        kept.push(BankUse {
            work_id,
            title: one.title.trim().to_owned(),
            date,
        });
    }
    kept.sort_by(|a, b| a.date.cmp(&b.date));
    // The last use is never older than the newest work that used it.
    if let Some(newest) = kept.last()
        && last.as_deref().is_none_or(|day| day < newest.date.as_str())
    {
        last = Some(newest.date.clone());
    }
    if let Some(last) = last {
        out.insert("last_used_at".into(), Value::String(last));
    }
    if !kept.is_empty() {
        out.insert("used_in".into(), serde_json::to_value(&kept)?);
    }
    if data.get("locked").and_then(Value::as_bool) == Some(true) {
        out.insert("locked".into(), Value::Bool(true));
    }
    if let Some(note) = text("note") {
        out.insert("note".into(), Value::String(note));
    }
    Ok(out)
}

/// Where an item stands on `today`, and - resting - the month it is free
/// again (`YYYY-MM`).
pub fn state(data: &Map<String, Value>, today: time::Date) -> (BankState, Option<String>) {
    if data.get("locked").and_then(Value::as_bool) == Some(true) {
        return (BankState::Locked, None);
    }
    let months = data
        .get("cooldown_months")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let last = data
        .get("last_used_at")
        .and_then(Value::as_str)
        .and_then(|day| time::Date::parse(day, DAY).ok());
    match last.and_then(|last| plus_months(last, months)) {
        Some(free) if months > 0 && free > today => (
            BankState::Cooldown,
            Some(format!("{:04}-{:02}", free.year(), u8::from(free.month()))),
        ),
        _ => (BankState::Available, None),
    }
}

/// The item's data with `work` marked as using it on `date`: its use added -
/// or moved to that day, when it was there - and its last use the newest.
pub fn used(
    data: &Map<String, Value>,
    work_id: &str,
    title: &str,
    date: &str,
) -> Result<Map<String, Value>> {
    if !crate::time::is_date(date) {
        return Err(Error::refused("canon.bankBadDay").param("day", date));
    }
    let mut next = data.clone();
    let mut uses: Vec<Value> = next
        .get("used_in")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    uses.push(serde_json::to_value(BankUse {
        work_id: work_id.to_owned(),
        title: title.to_owned(),
        date: date.to_owned(),
    })?);
    next.insert("used_in".into(), Value::Array(uses));
    checked(&next)
}

/// Whether `work` is among the item's uses.
pub fn uses(data: &Map<String, Value>, work_id: &str) -> bool {
    data.get("used_in")
        .and_then(Value::as_array)
        .is_some_and(|uses| {
            uses.iter()
                .any(|one| one.get("work_id").and_then(Value::as_str) == Some(work_id))
        })
}

/// The item as a prompt is told it: its name, group and attributes.
pub fn line(name: &str, data: &Map<String, Value>) -> String {
    let mut line = format!("- {name}");
    if let Some(group) = data.get("group").and_then(Value::as_str) {
        line.push_str(&format!(" ({group})"));
    }
    let attributes: Vec<&str> = data
        .get("attributes")
        .and_then(Value::as_array)
        .map(|many| many.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if !attributes.is_empty() {
        line.push_str(&format!(": {}", attributes.join(", ")));
    }
    line
}

/// Today, as the prompts read the pool: the day on this machine's clock in
/// UTC, which a month's rest does not notice.
pub fn today() -> time::Date {
    time::OffsetDateTime::now_utc().date()
}

const DAY: &[time::format_description::FormatItem<'static>] =
    time::macros::format_description!("[year]-[month]-[day]");

/// `day` and `months` later, on the same day of the month or the month's
/// last when it is shorter.
fn plus_months(day: time::Date, months: u64) -> Option<time::Date> {
    let index = i64::from(day.year()) * 12 + i64::from(u8::from(day.month())) - 1
        + i64::try_from(months).ok()?;
    let year = i32::try_from(index.div_euclid(12)).ok()?;
    let month = time::Month::try_from(u8::try_from(index.rem_euclid(12) + 1).ok()?).ok()?;
    let last = time::util::days_in_month(month, year);
    time::Date::from_calendar_date(year, month, day.day().min(last)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn data(value: Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap()
    }

    fn day(text: &str) -> time::Date {
        time::Date::parse(text, DAY).unwrap()
    }

    #[test]
    fn an_item_rests_its_months_after_a_use() {
        let item = data(json!({ "cooldown_months": 6, "last_used_at": "2026-03-31" }));
        assert_eq!(
            state(&item, day("2026-09-29")),
            (BankState::Cooldown, Some("2026-09".into()))
        );
        assert_eq!(
            state(&item, day("2026-09-30")),
            (BankState::Available, None)
        );
        // Never used, or resting no months: free.
        assert_eq!(
            state(&data(json!({ "cooldown_months": 6 })), day("2026-01-01")).0,
            BankState::Available
        );
        assert_eq!(
            state(
                &data(json!({ "last_used_at": "2026-03-31" })),
                day("2026-04-01")
            )
            .0,
            BankState::Available
        );
        // Locked whatever its rest says.
        assert_eq!(
            state(&data(json!({ "locked": true })), day("2026-01-01")).0,
            BankState::Locked
        );
    }

    #[test]
    fn a_use_moves_the_last_day_and_a_work_is_counted_once() {
        let item = checked(&data(
            json!({ "cooldown_months": 3, "attributes": "kora, balafon, " }),
        ))
        .unwrap();
        assert_eq!(item["attributes"], json!(["kora", "balafon"]));
        let once = used(&item, "w1", "Dawn", "2026-05-02").unwrap();
        let twice = used(&once, "w2", "Dusk", "2026-04-10").unwrap();
        assert_eq!(twice["last_used_at"], json!("2026-05-02"));
        assert_eq!(twice["used_in"].as_array().unwrap().len(), 2);
        assert_eq!(twice["used_in"][0]["work_id"], json!("w2"), "sorted by day");
        let again = used(&twice, "w1", "Dawn", "2026-06-01").unwrap();
        assert_eq!(again["used_in"].as_array().unwrap().len(), 2);
        assert_eq!(again["last_used_at"], json!("2026-06-01"));
        assert!(uses(&again, "w2"));
        assert!(!uses(&again, "w3"));
    }

    #[test]
    fn what_does_not_read_is_refused() {
        for bad in [
            json!({ "cooldown_months": -1 }),
            json!({ "cooldown_months": 500 }),
            json!({ "last_used_at": "2026-13-01" }),
            json!({ "used_in": [{ "work_id": "w", "title": "t", "date": "soon" }] }),
            json!({ "used_in": "w" }),
            json!({ "attributes": 3 }),
        ] {
            assert!(checked(&data(bad.clone())).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_prompt_reads_the_name_group_and_attributes() {
        let item = data(json!({ "group": "West Africa", "attributes": ["kora", "griot voice"] }));
        assert_eq!(
            line("Mande", &item),
            "- Mande (West Africa): kora, griot voice"
        );
    }
}
