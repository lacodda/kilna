//! The codes a work is numbered by (v0.93.1, ADR 0059): `CAT-001`, `CAT-002`.
//!
//! A field the profile numbers (`MetaField::numbered_from`) is given to a new
//! work as the next code after the greatest one of its shape. The greatest is
//! read off every work of the workspace and every work in its trash, so a work
//! put back from the trash never finds its code handed to another - a folder
//! on disk named after the code (ADR 0057) would then belong to two works.
//!
//! The code is written into the work, not worked out from where the work
//! stands among the others: a number derived from an order would change when
//! a work before it was deleted, and every folder named after one would be
//! lost.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::Result;
use crate::profile::config::MetaField;

/// What the first code says about all of them: the letters before the
/// number, how many digits the number is written with, and where counting
/// starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    prefix: String,
    width: usize,
    first: u64,
}

impl Pattern {
    /// The shape `first` gives its codes: `CAT-007` is `CAT-`, three digits,
    /// counting from seven. None when it does not end in digits, or ends in
    /// more of them than a count holds.
    pub fn of(first: &str) -> Option<Self> {
        let prefix = first.trim_end_matches(|c: char| c.is_ascii_digit());
        let digits = &first[prefix.len()..];
        Some(Self {
            prefix: prefix.to_owned(),
            width: digits.len(),
            first: digits.parse().ok()?,
        })
    }

    /// The number `value` is the code of, when it is a code of this shape.
    fn number_in(&self, value: &str) -> Option<u64> {
        let digits = value.strip_prefix(self.prefix.as_str())?;
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        digits.parse().ok()
    }

    /// The code `number` is written as: padded to the first code's width,
    /// and longer once the count outgrows it - `CAT-999`, then `CAT-1000`.
    pub fn code(&self, number: u64) -> String {
        format!("{}{number:0width$}", self.prefix, width = self.width)
    }

    /// The number after the greatest code among `taken`, and never before
    /// the first: codes given by hand below it do not pull counting back.
    pub fn next_after<'a>(&self, taken: impl IntoIterator<Item = &'a str>) -> u64 {
        taken
            .into_iter()
            .filter_map(|value| self.number_in(value))
            .max()
            .map_or(self.first, |greatest| {
                greatest.saturating_add(1).max(self.first)
            })
    }
}

/// Where a numbered field stands in a workspace: the codes it gives next,
/// counted off one at a time - a batch gives each work its own.
#[derive(Debug, Clone)]
pub struct Numbering {
    pattern: Pattern,
    next: u64,
}

impl Numbering {
    /// How `field` numbers in the workspace of `profile_id`. None when the
    /// field is not numbered.
    pub fn of(conn: &Connection, profile_id: &str, field: &MetaField) -> Result<Option<Self>> {
        let Some(pattern) = field.numbered_from.as_deref().and_then(Pattern::of) else {
            return Ok(None);
        };
        let taken = taken(conn, profile_id, &field.key)?;
        let next = pattern.next_after(taken.iter().map(String::as_str));
        Ok(Some(Self { pattern, next }))
    }

    /// The next code, counted off.
    pub fn take(&mut self) -> String {
        let code = self.pattern.code(self.next);
        self.next = self.next.saturating_add(1);
        code
    }
}

/// The values `key` holds across the workspace: on its works, and on the
/// works in its trash, whose snapshots keep the fields they had.
fn taken(conn: &Connection, profile_id: &str, key: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT meta FROM work WHERE profile_id = ?1
         UNION ALL
         SELECT json_extract(row.value, '$.meta')
           FROM deletion, json_each(deletion.snapshot, '$.work') AS row
          WHERE deletion.profile_id = ?1",
    )?;
    let rows = statement.query_map(params![profile_id], |row| row.get::<_, Option<String>>(0))?;
    let mut values = Vec::new();
    for meta in rows {
        let Some(meta) = meta? else { continue };
        // A row whose fields do not read holds no code; it is not a reason
        // to refuse making a work.
        let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(&meta) else {
            continue;
        };
        if let Some(text) = text_of(fields.get(key)) {
            values.push(text);
        }
    }
    Ok(values)
}

/// A stored value as the text a code is: a number a plugin wrote counts as
/// its digits. Nothing for none, and nothing for a blank.
fn text_of(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_owned()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

/// Whether a work's fields leave it without a value for `key`: absent, null
/// or blank - the works a code is given to.
pub fn lacks(meta: &Map<String, Value>, key: &str) -> bool {
    text_of(meta.get(key)).is_none()
}

/// A code a batch gave a work, as the operations log keeps it: what a replay
/// gives again and an undo takes back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Given {
    pub id: String,
    pub code: String,
}

/// Give each work its code, at `at`: the replay of a batch, word for word.
pub fn give_at(conn: &Connection, key: &str, given: &[Given], at: &str) -> Result<()> {
    for one in given {
        let patch = super::WorkPatch {
            meta: Some(Map::from_iter([(
                key.to_owned(),
                Value::String(one.code.clone()),
            )])),
            ..super::WorkPatch::default()
        };
        super::update_at(conn, &one.id, patch, at)?;
    }
    Ok(())
}

/// Take each work's code back off it, at `at`: the undo of a batch. The batch
/// gave codes only to works that had none, so none is what they go back to.
pub fn take_back_at(conn: &Connection, key: &str, given: &[Given], at: &str) -> Result<()> {
    for one in given {
        let patch = super::WorkPatch {
            meta: Some(Map::from_iter([(key.to_owned(), Value::Null)])),
            ..super::WorkPatch::default()
        };
        super::update_at(conn, &one.id, patch, at)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(first: &str) -> Pattern {
        Pattern::of(first).expect("a code that ends in digits")
    }

    #[test]
    fn the_first_code_says_the_letters_the_width_and_the_start() {
        assert_eq!(
            Pattern::of("CAT-007"),
            Some(Pattern {
                prefix: "CAT-".into(),
                width: 3,
                first: 7
            })
        );
        assert_eq!(pattern("001").prefix, "", "digits alone are a code too");
        assert_eq!(
            pattern("A1B2").prefix,
            "A1B",
            "only the trailing digits count"
        );
        assert_eq!(Pattern::of("CAT"), None, "nothing to count on");
        assert_eq!(Pattern::of(""), None);
        assert_eq!(
            Pattern::of("X99999999999999999999999"),
            None,
            "more digits than a count holds"
        );
    }

    #[test]
    fn the_next_code_follows_the_greatest_of_its_shape() {
        let codes = pattern("CAT-001");
        assert_eq!(
            codes.next_after([]),
            1,
            "an empty workspace starts at the first"
        );
        assert_eq!(
            codes.next_after(["CAT-001", "CAT-009", "CAT-004"]),
            10,
            "past the greatest, not the count: a gap is not filled"
        );
        assert_eq!(
            codes.next_after(["DOG-500", "CAT-x1", "CAT-", "500"]),
            1,
            "values of another shape are not codes of this one"
        );
        assert_eq!(pattern("CAT-010").next_after(["CAT-002"]), 10);
        assert_eq!(codes.code(12), "CAT-012");
        assert_eq!(codes.code(1000), "CAT-1000", "the count outgrows the width");
    }

    #[test]
    fn a_blank_value_is_no_code() {
        let meta: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "blank": "  ", "none": null, "code": "CAT-001", "number": 12
        }))
        .unwrap();
        assert!(lacks(&meta, "blank"));
        assert!(lacks(&meta, "none"));
        assert!(lacks(&meta, "absent"));
        assert!(!lacks(&meta, "code"));
        assert!(!lacks(&meta, "number"));
    }
}
