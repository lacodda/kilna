//! A release's numbers, from YouTube Studio's own export (v0.160, FR-22 22.1
//! p.2): the API's quota is gone, but Studio's Advanced mode exports a table
//! of every video - views, impressions, click-through, how much of it was
//! watched, likes, subscribers. Each row is matched to its release by the
//! video's id in the release's link, else by its exact title, and kept on
//! the release as `meta.analytics`; read first, kept only when told to.
//!
//! The headers are read in English and in Russian, as Studio writes them in
//! the language its account speaks; a column the reader does not know is
//! left out, never guessed.

use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::error::{Error, Result};

/// What a column is, by the words its header starts with.
const COLUMNS: [(&str, &[&str]); 11] = [
    (
        "id",
        &["content", "контент", "video id", "идентификатор видео"],
    ),
    ("title", &["video title", "название видео"]),
    ("views", &["views", "просмотры"]),
    (
        "impressions",
        &["impressions click", "impressions", "показы"],
    ),
    ("watch_hours", &["watch time", "время просмотра"]),
    (
        "avg_view_seconds",
        &[
            "average view duration",
            "средняя продолжительность просмотра",
        ],
    ),
    (
        "avg_view_pct",
        &["average percentage viewed", "средний процент просмотра"],
    ),
    (
        "likes",
        &[
            "likes",
            "отметки \"нравится\"",
            "отметки «нравится»",
            "лайки",
        ],
    ),
    (
        "subscribers",
        &["subscribers gained", "subscribers", "подписчики"],
    ),
    ("comments", &["comments added", "comments", "комментарии"]),
    (
        "ctr",
        &["impressions click-through rate", "ctr", "click-through"],
    ),
];

/// Which number a header names, or none.
fn column_of(header: &str) -> Option<&'static str> {
    let header = header.trim().trim_matches('\u{feff}').to_lowercase();
    // Click-through first: its header begins as impressions' does.
    if header.contains("click-through") || header.starts_with("ctr") {
        return Some("ctr");
    }
    COLUMNS
        .iter()
        .find(|(_, starts)| starts.iter().any(|start| header.starts_with(start)))
        .map(|(key, _)| *key)
}

/// A cell as a number: `1,234`, `12.5`, `0:03:05` (seconds), `5,4` (a
/// decimal comma); none for anything else.
fn number_of(cell: &str, key: &str) -> Option<f64> {
    let cell = cell.trim().trim_end_matches('%').trim();
    if cell.is_empty() {
        return None;
    }
    if key == "avg_view_seconds" && cell.contains(':') {
        let mut seconds = 0.0;
        for part in cell.split(':') {
            seconds = seconds * 60.0 + part.trim().parse::<f64>().ok()?;
        }
        return Some(seconds);
    }
    let plain: String = cell
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{a0}')
        .collect();
    // `1,234` is a thousand; `5,4` is five and four tenths.
    let plain = match (plain.contains(','), plain.contains('.')) {
        (true, true) => plain.replace(',', ""),
        (true, false) if plain.split(',').nth(1).is_some_and(|tail| tail.len() == 3) => {
            plain.replace(',', "")
        }
        (true, false) => plain.replace(',', "."),
        _ => plain,
    };
    plain.parse().ok()
}

/// The id of a video in a link to it: `youtu.be/ID`, `watch?v=ID`,
/// `shorts/ID`.
pub fn video_id(link: &str) -> Option<String> {
    let link = link.trim();
    let after = ["youtu.be/", "watch?v=", "shorts/"]
        .iter()
        .find_map(|mark| link.split_once(mark).map(|(_, rest)| rest))?;
    let id: String = after
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    (id.len() == 11).then_some(id)
}

/// One row of the export, as read.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct AnalyticsRow {
    pub line: u32,
    pub video_id: Option<String>,
    pub title: Option<String>,
    pub metrics: BTreeMap<String, f64>,
    /// The release it belongs to, when one was found.
    pub release_id: Option<String>,
    pub work_title: Option<String>,
    /// How it was found: `id` or `title`.
    pub by: Option<String>,
}

/// An export read against the space's releases.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct AnalyticsImport {
    pub rows: Vec<AnalyticsRow>,
    /// The numbers the file has, in the order of its columns.
    pub columns: Vec<String>,
}

/// Read an export's text against the releases of a profile.
pub fn read(conn: &Connection, profile_id: &str, csv: &str) -> Result<AnalyticsImport> {
    let rows = crate::plan_import::rows_of(csv);
    let Some(((_, header), body)) = rows.split_first() else {
        return Err(Error::refused("analytics.empty"));
    };
    let keys: Vec<Option<&'static str>> = header.iter().map(|cell| column_of(cell)).collect();
    if !keys.contains(&Some("id")) && !keys.contains(&Some("title")) {
        return Err(Error::refused("analytics.noVideoColumn"));
    }
    let columns: Vec<String> = keys
        .iter()
        .flatten()
        .filter(|key| **key != "id" && **key != "title")
        .map(|key| (*key).to_owned())
        .collect();

    // Every release of the profile, by the id in its link and by title.
    let mut by_id: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut by_title: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut statement = conn.prepare(
        "SELECT r.id, r.url, w.title, json_extract(r.meta, '$.title')
           FROM release r JOIN work w ON w.id = r.work_id
          WHERE w.profile_id = ?1",
    )?;
    let found = statement.query_map([profile_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;
    for one in found {
        let (id, url, work_title, title) = one?;
        if let Some(video) = url.as_deref().and_then(video_id) {
            by_id.insert(video, (id.clone(), work_title.clone()));
        }
        for name in [Some(work_title.clone()), title].into_iter().flatten() {
            let name = name.trim().to_lowercase();
            if !name.is_empty() {
                by_title
                    .entry(name)
                    .or_default()
                    .push((id.clone(), work_title.clone()));
            }
        }
    }

    let mut out = Vec::new();
    for (line, cells) in body {
        let cell = |key: &str| {
            keys.iter()
                .position(|one| *one == Some(key))
                .and_then(|at| cells.get(at))
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        };
        let video = cell("id");
        let title = cell("title");
        // Studio's first row after the header is the total of them all.
        let total = video
            .as_deref()
            .is_some_and(|id| matches!(id.to_lowercase().as_str(), "total" | "итого" | "всего"));
        if total {
            continue;
        }
        let mut metrics = BTreeMap::new();
        for (at, key) in keys.iter().enumerate() {
            let Some(key) = key.filter(|key| *key != "id" && *key != "title") else {
                continue;
            };
            if let Some(number) = cells.get(at).and_then(|value| number_of(value, key)) {
                metrics.insert(key.to_owned(), number);
            }
        }
        let by_video = video.as_deref().and_then(|id| by_id.get(id));
        let by_name = title
            .as_deref()
            .map(|title| title.trim().to_lowercase())
            .and_then(|name| by_title.get(&name))
            .filter(|matches| {
                // A title two releases share names neither.
                let mut ids: Vec<&String> = matches.iter().map(|(id, _)| id).collect();
                ids.dedup();
                ids.len() == 1
            })
            .and_then(|matches| matches.first());
        let (release_id, work_title, by) = match (by_video, by_name) {
            (Some((id, work)), _) => (Some(id.clone()), Some(work.clone()), Some("id")),
            (None, Some((id, work))) => (Some(id.clone()), Some(work.clone()), Some("title")),
            (None, None) => (None, None, None),
        };
        out.push(AnalyticsRow {
            line: *line,
            video_id: video,
            title,
            metrics,
            release_id,
            work_title,
            by: by.map(str::to_owned),
        });
    }
    Ok(AnalyticsImport { rows: out, columns })
}

/// A release's `meta` with these numbers kept as its analytics, said when
/// they were read and from where.
pub fn with_numbers(
    meta: &Map<String, Value>,
    metrics: &BTreeMap<String, f64>,
    at: &str,
) -> Map<String, Value> {
    let mut out = meta.clone();
    let mut numbers: Map<String, Value> = metrics
        .iter()
        .map(|(key, value)| (key.clone(), json!(value)))
        .collect();
    numbers.insert("read_at".into(), json!(at));
    numbers.insert("from".into(), json!("youtube_studio"));
    out.insert("analytics".into(), Value::Object(numbers));
    out
}

#[cfg(test)]
mod tests;
