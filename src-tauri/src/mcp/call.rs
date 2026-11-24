//! What every tool reads its call by: the arguments, the work one of them
//! names, the page of a long answer, the answer written out.

use rusqlite::Connection;
use serde_json::{Map, Value, json};

use super::Session;
use crate::error::{Error, Result};
use crate::profile;
use crate::profile::config::ProfileConfig;
use crate::work::{self, WorkFilter};

/// One call of a tool: the space it is answered from, who asks, the craft's
/// vocabulary there and the arguments given.
pub(super) struct Call<'a> {
    pub(super) conn: &'a Connection,
    pub(super) session: &'a Session,
    pub(super) profile: &'a profile::Profile,
    pub(super) config: &'a ProfileConfig,
    pub(super) args: &'a Map<String, Value>,
}

/// The most a page may hold, whatever is asked.
pub(super) const MOST: usize = 500;

/// The part of a long reading an agent asked for (FR-08 8.4).
///
/// The heavy tools gave everything at once: a record of two thousand words,
/// a bank of fifteen hundred notes with their bodies, into a context that
/// holds a few hundred thousand tokens for the whole conversation. A page
/// says how many there are and whether more follow, so an agent can walk the
/// list, read it short first, and take one entry whole.
pub(super) struct Page {
    pub(super) limit: usize,
    pub(super) offset: usize,
    pub(super) summary: bool,
}

impl Page {
    pub(super) fn of(args: &Map<String, Value>, default: usize) -> Self {
        let number = |key: &str| {
            args.get(key)
                .and_then(Value::as_u64)
                .and_then(|n| usize::try_from(n).ok())
        };
        Self {
            limit: number("limit").unwrap_or(default).clamp(1, MOST),
            offset: number("offset").unwrap_or(0),
            summary: args
                .get("summary")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }
    }

    /// This page of `items` under `key`, with how many there are in all and
    /// whether more follow.
    pub(super) fn cut(&self, key: &str, items: Vec<Value>) -> Map<String, Value> {
        let total = items.len();
        let shown: Vec<Value> = items
            .into_iter()
            .skip(self.offset)
            .take(self.limit)
            .collect();
        let mut out = Map::new();
        out.insert("total".into(), json!(total));
        out.insert("limit".into(), json!(self.limit));
        out.insert("offset".into(), json!(self.offset));
        out.insert("has_more".into(), json!(self.offset + shown.len() < total));
        out.insert(key.to_owned(), Value::Array(shown));
        out
    }
}

/// How long a text is, as a summary says it: in characters.
pub(super) fn length_of(text: &str) -> usize {
    text.chars().count()
}

/// The active profile, or the sentence an agent can act on.
pub(super) fn active(conn: &Connection) -> Result<profile::Profile> {
    profile::active(conn)?.ok_or_else(|| Error::refused("profile.noneActive"))
}

pub(super) fn arg<'a>(args: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

pub(super) fn required<'a>(args: &'a Map<String, Value>, key: &str) -> Result<&'a str> {
    arg(args, key).ok_or_else(|| Error::refused("mcp.missingArg").param("key", key))
}

/// The work an argument names: by id first, then by exact title within the
/// active profile. A title shared by two works is refused rather than
/// guessed, with the ids to choose from.
pub(super) fn find_work(conn: &Connection, profile_id: &str, named: &str) -> Result<work::Work> {
    if let Some(found) = work::get(conn, named)? {
        return Ok(found);
    }
    let same_title: Vec<work::Work> = work::list(conn, profile_id, &WorkFilter::default())?
        .into_iter()
        .filter(|w| w.title == named)
        .collect();
    match same_title.len() {
        0 => Err(Error::refused("mcp.unknownWork").param("named", named)),
        1 => Ok(same_title.into_iter().next().expect("one")),
        _ => Err(Error::refused("mcp.ambiguousTitle")
            .param("count", same_title.len())
            .param("named", named)
            .param(
                "ids",
                same_title
                    .iter()
                    .map(|w| w.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            )),
    }
}

pub(super) fn pretty(value: &impl serde::Serialize) -> Result<String> {
    Ok(serde_json::to_string_pretty(value)?)
}
