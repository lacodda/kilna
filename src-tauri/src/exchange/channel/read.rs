//! A channel's folder read (v0.154, FR-20 20.5): its README, its register of
//! repeats, its bank, its plan and its month packs, by the shapes the files
//! are written in - headings, tables, fenced blocks - never by what a
//! channel says in them.

use std::path::Path;

use crate::error::{Error, Result};

/// A `## ` section of a page, with its `### ` parts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Section {
    pub heading: String,
    /// The words under the heading before its first `### `.
    pub body: String,
    pub parts: Vec<(String, String)>,
}

/// A row of the register's stop-list: the pattern and its rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub pattern: String,
    pub rule: String,
}

/// An item of the bank's reference tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tradition {
    pub name: String,
    /// The `### ` region its table stands under.
    pub group: String,
    pub attributes: Vec<String>,
    pub note: String,
}

/// A track of the plan: one row of a month's table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlannedTrack {
    /// `YYYY-MM-DD`.
    pub date: String,
    pub title: String,
    pub peak: bool,
    pub vocal: bool,
    /// The row's cells by the column's heading, as written.
    pub cells: Vec<(String, String)>,
}

/// A month of the plan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlannedMonth {
    /// `YYYY-MM`.
    pub month: String,
    /// The month's name in quotes, when the heading gives one.
    pub title: Option<String>,
    pub arc: Option<String>,
    pub tracks: Vec<PlannedTrack>,
}

/// A track of a month pack: its day and the blocks under it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackTrack {
    /// `YYYY-MM-DD`.
    pub date: String,
    /// The `# ` heading after the date, inside its brackets.
    pub heading: String,
    /// `### ` blocks by heading, their fenced text or their words.
    pub blocks: Vec<(String, String)>,
}

/// Everything read off one channel's folder.
#[derive(Debug, Clone, Default)]
pub struct Channel {
    pub readme: Vec<Section>,
    pub patterns: Vec<Pattern>,
    pub traditions: Vec<Tradition>,
    /// Month (`YYYY-MM`) to the traditions taken in it.
    pub taken: Vec<(String, Vec<String>)>,
    pub plan: Vec<PlannedMonth>,
    pub packs: Vec<PackTrack>,
}

/// Read the folder of one channel. Only README.md is required; the rest is
/// read when it is there.
pub fn read(dir: &Path) -> Result<Channel> {
    let page = |name: &str| std::fs::read_to_string(dir.join(name)).ok();
    let readme = page("README.md").ok_or_else(|| {
        Error::refused("channel.noReadme").param("path", dir.display().to_string())
    })?;
    let mut channel = Channel {
        readme: sections(&readme),
        ..Channel::default()
    };
    if let Some(text) = page("Реестр самоповторов.md") {
        channel.patterns = patterns(&text);
    }
    if let Some(text) = page("Банк традиций.md") {
        let (traditions, taken) = bank(&text);
        channel.traditions = traditions;
        channel.taken = taken;
    }
    if let Some(text) = page("План.md") {
        channel.plan = plan(&text);
    }
    if let Ok(entries) = std::fs::read_dir(dir.join("Подборки")) {
        let mut files: Vec<_> = entries
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
            .collect();
        files.sort();
        for file in files {
            if let Ok(text) = std::fs::read_to_string(&file) {
                channel.packs.extend(pack(&text));
            }
        }
    }
    Ok(channel)
}

/// A page's `## ` sections with their `### ` parts.
pub fn sections(text: &str) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            out.push(Section {
                heading: heading.trim().to_owned(),
                ..Section::default()
            });
            continue;
        }
        let Some(section) = out.last_mut() else {
            continue;
        };
        if let Some(heading) = line.strip_prefix("### ") {
            section
                .parts
                .push((heading.trim().to_owned(), String::new()));
            continue;
        }
        let target = match section.parts.last_mut() {
            Some((_, body)) => body,
            None => &mut section.body,
        };
        target.push_str(line);
        target.push('\n');
    }
    for section in &mut out {
        section.body = section.body.trim().to_owned();
        for (_, body) in &mut section.parts {
            *body = body.trim().to_owned();
        }
    }
    out
}

/// The cells of a markdown table row; none for a line that is no row or is
/// the row under the header.
fn cells(line: &str) -> Option<Vec<String>> {
    let line = line.trim();
    if !line.starts_with('|') || !line.ends_with('|') || line.len() < 2 {
        return None;
    }
    let inner = &line[1..line.len() - 1];
    let cells: Vec<String> = inner.split('|').map(|cell| clean(cell.trim())).collect();
    if cells
        .iter()
        .all(|cell| cell.chars().all(|c| c == '-' || c == ':' || c == ' '))
    {
        return None;
    }
    Some(cells)
}

/// A cell's words without markdown emphasis.
fn clean(cell: &str) -> String {
    cell.replace("**", "").replace('`', "").trim().to_owned()
}

/// The tables of a page: each a header and its rows, with the `### `
/// heading it stands under.
fn tables(text: &str) -> Vec<(String, Vec<String>, Vec<Vec<String>>)> {
    let mut out = Vec::new();
    let mut heading = String::new();
    let mut header: Option<Vec<String>> = None;
    let mut rows: Vec<Vec<String>> = Vec::new();
    for line in text.lines() {
        if let Some(found) = line
            .strip_prefix("### ")
            .or_else(|| line.strip_prefix("## "))
        {
            if let Some(done) = header.take() {
                out.push((heading.clone(), done, std::mem::take(&mut rows)));
            }
            heading = found.trim().to_owned();
            continue;
        }
        let is_row = line.trim().starts_with('|');
        match (cells(line), is_row, header.is_some()) {
            (Some(found), _, false) => header = Some(found),
            (Some(found), _, true) => rows.push(found),
            (None, true, _) => {}
            (None, false, true) => {
                if let Some(done) = header.take() {
                    out.push((heading.clone(), done, std::mem::take(&mut rows)));
                }
            }
            (None, false, false) => {}
        }
    }
    if let Some(done) = header {
        out.push((heading, done, rows));
    }
    out
}

/// The register's stop-list: a table whose header names a pattern and a
/// rule.
fn patterns(text: &str) -> Vec<Pattern> {
    let mut out = Vec::new();
    for (_, header, rows) in tables(text) {
        let at = |name: &str| {
            header
                .iter()
                .position(|cell| cell.to_lowercase().contains(name))
        };
        let (Some(pattern), Some(rule)) = (
            at("паттерн").or_else(|| at("pattern")),
            at("правило").or_else(|| at("rule")),
        ) else {
            continue;
        };
        for row in rows {
            let (Some(p), Some(r)) = (row.get(pattern), row.get(rule)) else {
                continue;
            };
            if !p.is_empty() {
                out.push(Pattern {
                    pattern: p.clone(),
                    rule: r.clone(),
                });
            }
        }
    }
    out
}

/// The bank: its reference tables (a region, its instruments, devices and
/// character) and the months its taken traditions are listed under.
fn bank(text: &str) -> (Vec<Tradition>, Vec<(String, Vec<String>)>) {
    let mut traditions = Vec::new();
    for (group, header, rows) in tables(text) {
        if header.len() < 2 {
            continue;
        }
        let note_at = header.len() - 1;
        for row in rows {
            let Some(name) = row.first().filter(|name| !name.is_empty()) else {
                continue;
            };
            let attributes: Vec<String> = row
                .iter()
                .enumerate()
                .filter(|(at, _)| *at != 0 && *at != note_at)
                .flat_map(|(_, cell)| {
                    cell.split(',')
                        .map(|one| one.trim().to_owned())
                        .collect::<Vec<_>>()
                })
                .filter(|one| !one.is_empty())
                .collect();
            traditions.push(Tradition {
                name: name.clone(),
                group: group.clone(),
                attributes,
                note: row.get(note_at).cloned().unwrap_or_default(),
            });
        }
    }
    let mut taken = Vec::new();
    for line in text.lines() {
        // `- **2026-09:** A · B · C`
        let Some(rest) = line.trim().strip_prefix("- **") else {
            continue;
        };
        let Some((month, names)) = rest.split_once(":**") else {
            continue;
        };
        let month = month.trim();
        if month.len() != 7 || month.as_bytes().get(4) != Some(&b'-') {
            continue;
        }
        let names: Vec<String> = names
            .split('·')
            .map(|one| clean(one.trim()))
            .filter(|one| !one.is_empty())
            .collect();
        taken.push((month.to_owned(), names));
    }
    (traditions, taken)
}

/// The plan: each `## YYYY-MM` heading, its `«title»`, its `Арка:` line
/// and the rows of its table, the date's day read with the heading's year.
fn plan(text: &str) -> Vec<PlannedMonth> {
    let mut out: Vec<PlannedMonth> = Vec::new();
    let mut header: Option<Vec<String>> = None;
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            header = None;
            let month: String = heading.chars().take(7).collect();
            if month.len() == 7
                && month.as_bytes().get(4) == Some(&b'-')
                && month[..4].parse::<u16>().is_ok()
            {
                let title = heading
                    .split_once('«')
                    .and_then(|(_, rest)| rest.split_once('»'))
                    .map(|(name, _)| name.trim().to_owned());
                out.push(PlannedMonth {
                    month,
                    title,
                    ..PlannedMonth::default()
                });
            }
            continue;
        }
        let Some(current) = out.last_mut() else {
            continue;
        };
        if let Some(arc) = line
            .trim()
            .strip_prefix("Арка:")
            .or_else(|| line.trim().strip_prefix("Arc:"))
        {
            current.arc = Some(arc.trim().to_owned());
            continue;
        }
        let Some(row) = cells(line) else {
            continue;
        };
        let Some(columns) = &header else {
            header = Some(row);
            continue;
        };
        let Some(first) = row.first() else {
            continue;
        };
        // `08-19 Ср 📌`: the month and day, then anything.
        let day: String = first.chars().take(5).collect();
        if day.len() != 5 || day.as_bytes().get(2) != Some(&b'-') {
            continue;
        }
        let date = format!("{}-{day}", &current.month[..4]);
        let title_raw = row.get(1).cloned().unwrap_or_default();
        let marks = format!("{first} {title_raw}");
        let title = strip_marks(&title_raw);
        current.tracks.push(PlannedTrack {
            date,
            title,
            peak: marks.contains('📌'),
            vocal: marks.contains('🎙'),
            cells: columns
                .iter()
                .cloned()
                .zip(row.iter().map(|cell| strip_marks(cell)))
                .skip(2)
                .collect(),
        });
    }
    out
}

/// Words without the plan's marks: flags, pins, microphones, ticks.
fn strip_marks(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ",.;:·×/()'’&!?-–—+#%".contains(*c))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A month pack: one `# YYYY-MM-DD (...)` a track, its `### ` blocks.
fn pack(text: &str) -> Vec<PackTrack> {
    let mut out: Vec<PackTrack> = Vec::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence && let Some(heading) = line.strip_prefix("# ") {
            let date: String = heading.chars().take(10).collect();
            if date.len() == 10 && crate::time::is_date(&date) {
                let rest = heading[date.len()..].trim();
                let inner = rest
                    .strip_prefix('(')
                    .and_then(|inner| inner.strip_suffix(')'))
                    .unwrap_or(rest);
                out.push(PackTrack {
                    date,
                    heading: inner.trim().to_owned(),
                    blocks: Vec::new(),
                });
            }
            continue;
        }
        let Some(track) = out.last_mut() else {
            continue;
        };
        if !in_fence && let Some(heading) = line.strip_prefix("### ") {
            track
                .blocks
                .push((heading.trim().to_owned(), String::new()));
            continue;
        }
        if let Some((_, body)) = track.blocks.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    for track in &mut out {
        for (_, body) in &mut track.blocks {
            *body = body.trim().to_owned();
        }
    }
    out
}
