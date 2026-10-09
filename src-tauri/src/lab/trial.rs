//! A trial on an experiment's board, and the board itself (ADR 0061).
//!
//! A trial is a row of `trial` (migration 0038): a series, the trial it
//! varies, what it moves, its text, the bricks it was picked from, who to
//! listen to, what came out, and one verdict - kept, dropped, or not judged.
//! The board reads them with what each became: the versions and the phrases
//! that remember it, found by asking rather than stored twice.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::asset::Asset;
use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::profile::config::{Lab, ProfileConfig};
use crate::work::Work;

/// What the person said about a trial once it was heard. None of them is
/// "not judged yet".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename = "TrialVerdict")]
pub enum Verdict {
    /// It works: an example of what is wanted, and what may be harvested.
    Keep,
    /// It does not: an example of what is not.
    Drop,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Keep => "keep",
            Verdict::Drop => "drop",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "keep" => Some(Verdict::Keep),
            "drop" => Some(Verdict::Drop),
            _ => None,
        }
    }
}

/// One trial on an experiment's board.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Trial {
    pub id: String,
    pub profile_id: String,
    /// The experiment whose board it is on.
    pub work_id: String,
    /// The series it stands in: "a sweep of the field", "around the core".
    pub series: String,
    /// Its place in the series.
    pub position: i64,
    /// The trial this one varies, while it is on the board.
    pub parent_id: Option<String>,
    /// What it moves, or where on the field it stands.
    pub angle: String,
    /// The text tried out.
    pub body: String,
    /// The bricks of the dictionary it was picked from, by id: where it came
    /// from, not what it says.
    pub bricks: Vec<String>,
    /// Who to listen to. Never part of the text.
    pub reference: String,
    /// What came out, once it was heard.
    pub outcome: String,
    pub verdict: Option<Verdict>,
    /// The version of a work's text it reworks: a song's style.
    pub source_version_id: Option<String>,
    /// Where a run starts.
    pub run_first: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// A trial to put on a board - what the operations log carries.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct NewTrial {
    pub work_id: String,
    #[serde(default)]
    pub series: Option<String>,
    /// Its place in the series; after the last when absent. Settled before
    /// the operation is logged, so a replay lands it where it landed.
    #[serde(default)]
    pub position: Option<i64>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub angle: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub bricks: Option<Vec<String>>,
    #[serde(default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub verdict: Option<Verdict>,
    #[serde(default)]
    pub source_version_id: Option<String>,
    #[serde(default)]
    pub run_first: Option<bool>,
}

/// What may change about a trial; `None` leaves a part alone, and
/// `Some(None)` takes a nullable part away - the verdict, the parent, the
/// version it reworks.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct TrialPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub parent_id: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bricks: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub verdict: Option<Option<Verdict>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub source_version_id: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_first: Option<bool>,
}

impl TrialPatch {
    /// Whether the patch would change nothing in `trial`.
    pub fn changes_nothing(&self, trial: &Trial) -> bool {
        self.series.as_ref().is_none_or(|v| *v == trial.series)
            && self.position.is_none_or(|v| v == trial.position)
            && self
                .parent_id
                .as_ref()
                .is_none_or(|v| *v == trial.parent_id)
            && self.angle.as_ref().is_none_or(|v| *v == trial.angle)
            && self.body.as_ref().is_none_or(|v| *v == trial.body)
            && self.bricks.as_ref().is_none_or(|v| *v == trial.bricks)
            && self
                .reference
                .as_ref()
                .is_none_or(|v| *v == trial.reference)
            && self.outcome.as_ref().is_none_or(|v| *v == trial.outcome)
            && self.verdict.is_none_or(|v| v == trial.verdict)
            && self
                .source_version_id
                .as_ref()
                .is_none_or(|v| *v == trial.source_version_id)
            && self.run_first.is_none_or(|v| v == trial.run_first)
    }
}

const SELECT: &str = "SELECT id, profile_id, work_id, series, position, parent_id, angle, body, \
                      bricks, reference, outcome, verdict, source_version_id, run_first, \
                      created_at, updated_at FROM trial";

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Trial> {
    let bricks: String = row.get(8)?;
    let verdict: Option<String> = row.get(11)?;
    let run_first: i64 = row.get(13)?;
    Ok(Trial {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        work_id: row.get(2)?,
        series: row.get(3)?,
        position: row.get(4)?,
        parent_id: row.get(5)?,
        angle: row.get(6)?,
        body: row.get(7)?,
        // The schema holds it to a JSON array; a member this build cannot
        // read as an id is left out rather than failing the board.
        bricks: serde_json::from_str::<Vec<serde_json::Value>>(&bricks)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_owned))
            .collect(),
        reference: row.get(9)?,
        outcome: row.get(10)?,
        verdict: verdict.as_deref().and_then(Verdict::parse),
        source_version_id: row.get(12)?,
        run_first: run_first != 0,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

/// The experiment a trial is for, and its lab: a work of the profile whose
/// kind is one.
pub fn a_lab(conn: &Connection, profile_id: &str, work_id: &str) -> Result<(Work, Lab)> {
    let work = crate::work::get(conn, work_id)?
        .filter(|work| work.profile_id == profile_id)
        .ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, profile_id)?;
    match config.lab(&work.kind) {
        Some(lab) => Ok((work, lab.clone())),
        None => Err(Error::refused("trial.noLab").param("title", work.title)),
    }
}

/// The place after the last of a series on a board.
pub fn next_position(conn: &Connection, work_id: &str, series: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT coalesce(max(position), 0) + 1 FROM trial WHERE work_id = ?1 AND series = ?2",
        params![work_id, series],
        |row| row.get(0),
    )?)
}

/// Put a trial on an experiment's board.
///
/// Refused when the work is no experiment, when the trial it varies is not
/// on the same board, when the version it reworks is not one of the role the
/// lab harvests into, and when it names a brick the workspace does not have.
/// An empty trial is a trial: the person writes it on the board.
pub fn create_minted(
    conn: &Connection,
    profile_id: &str,
    new: NewTrial,
    minted: Minted,
) -> Result<Trial> {
    let (work, lab) = a_lab(conn, profile_id, &new.work_id)?;
    let series = new.series.as_deref().map(str::trim).unwrap_or_default();
    if let Some(parent) = new.parent_id.as_deref() {
        check_parent(conn, &work.id, None, parent)?;
    }
    if let Some(version) = new.source_version_id.as_deref() {
        check_source(conn, profile_id, &work, &lab, version)?;
    }
    let bricks = new.bricks.unwrap_or_default();
    check_bricks(conn, profile_id, &bricks)?;
    let position = match new.position {
        Some(position) => position,
        None => next_position(conn, &work.id, series)?,
    };

    conn.execute(
        "INSERT INTO trial (id, profile_id, work_id, series, position, parent_id, angle, body,
                            bricks, reference, outcome, verdict, source_version_id, run_first,
                            created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
        params![
            minted.id(),
            profile_id,
            work.id,
            series,
            position,
            new.parent_id,
            new.angle.as_deref().map(str::trim).unwrap_or_default(),
            new.body.unwrap_or_default(),
            serde_json::to_string(&bricks)?,
            new.reference.as_deref().map(str::trim).unwrap_or_default(),
            new.outcome.unwrap_or_default(),
            new.verdict.map(Verdict::as_str),
            new.source_version_id,
            i64::from(new.run_first.unwrap_or(false)),
            minted.at(),
        ],
    )?;
    get(conn, minted.id())?.ok_or_else(|| Error::Internal("the trial vanished after insert".into()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Trial>> {
    Ok(conn
        .query_row(&format!("{SELECT} WHERE id = ?1"), params![id], read_row)
        .optional()?)
}

/// An experiment's board: series by series, in the order each began, and
/// in order within it.
pub fn for_work(conn: &Connection, work_id: &str) -> Result<Vec<Trial>> {
    let mut statement = conn.prepare(&format!(
        "{SELECT} t WHERE work_id = ?1
         ORDER BY (SELECT min(f.created_at) FROM trial f WHERE f.work_id = t.work_id AND f.series = t.series), series, position, created_at, rowid"
    ))?;
    let rows = statement
        .query_map(params![work_id], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Change a trial: its words, its place, what it varies, the verdict.
pub fn update_at(conn: &Connection, id: &str, patch: TrialPatch, at: &str) -> Result<Trial> {
    let current = get(conn, id)?.ok_or_else(|| Error::not_found("trial", id))?;
    if let Some(Some(parent)) = &patch.parent_id {
        check_parent(conn, &current.work_id, Some(id), parent)?;
    }
    if let Some(Some(version)) = &patch.source_version_id
        && current.source_version_id.as_deref() != Some(version.as_str())
    {
        let (work, lab) = a_lab(conn, &current.profile_id, &current.work_id)?;
        check_source(conn, &current.profile_id, &work, &lab, version)?;
    }
    if let Some(bricks) = &patch.bricks {
        check_bricks(conn, &current.profile_id, bricks)?;
    }
    let series = patch
        .series
        .map(|series| series.trim().to_owned())
        .unwrap_or(current.series);
    let position = patch.position.unwrap_or(current.position);
    let parent_id = patch.parent_id.unwrap_or(current.parent_id);
    let angle = patch
        .angle
        .map(|angle| angle.trim().to_owned())
        .unwrap_or(current.angle);
    let body = patch.body.unwrap_or(current.body);
    let bricks = patch.bricks.unwrap_or(current.bricks);
    let reference = patch
        .reference
        .map(|reference| reference.trim().to_owned())
        .unwrap_or(current.reference);
    let outcome = patch.outcome.unwrap_or(current.outcome);
    let verdict = patch.verdict.unwrap_or(current.verdict);
    let source_version_id = patch.source_version_id.unwrap_or(current.source_version_id);
    let run_first = patch.run_first.unwrap_or(current.run_first);
    conn.execute(
        "UPDATE trial SET series = ?2, position = ?3, parent_id = ?4, angle = ?5, body = ?6,
                          bricks = ?7, reference = ?8, outcome = ?9, verdict = ?10,
                          source_version_id = ?11, run_first = ?12, updated_at = ?13
         WHERE id = ?1",
        params![
            id,
            series,
            position,
            parent_id,
            angle,
            body,
            serde_json::to_string(&bricks)?,
            reference,
            outcome,
            verdict.map(Verdict::as_str),
            source_version_id,
            i64::from(run_first),
            at,
        ],
    )?;
    get(conn, id)?.ok_or_else(|| Error::not_found("trial", id))
}

/// Refuse a parent that is not on the same board, or one that would make a
/// trial its own ancestor.
fn check_parent(conn: &Connection, work_id: &str, own: Option<&str>, parent: &str) -> Result<()> {
    let found = get(conn, parent)?
        .filter(|found| found.work_id == work_id)
        .ok_or_else(|| Error::refused("trial.parentNotOnBoard").param("parent", parent))?;
    let Some(own) = own else {
        return Ok(());
    };
    // Up the line from the parent: reaching the trial itself is a circle.
    let mut step = Some(found);
    let mut seen = 0;
    while let Some(trial) = step {
        if trial.id == own {
            return Err(Error::refused("trial.parentIsDescendant"));
        }
        seen += 1;
        if seen > 1000 {
            break;
        }
        step = match trial.parent_id.as_deref() {
            Some(next) => get(conn, next)?,
            None => None,
        };
    }
    Ok(())
}

/// Refuse a version that is not a text of the role the lab harvests into,
/// of a work of the same workspace: a trial reworks a song's style, never
/// its lyrics.
fn check_source(
    conn: &Connection,
    profile_id: &str,
    experiment: &Work,
    lab: &Lab,
    version_id: &str,
) -> Result<()> {
    let version = crate::work::version::get(conn, version_id)?
        .ok_or_else(|| Error::not_found("version", version_id))?;
    let owner = crate::work::get(conn, &version.work_id)?
        .filter(|work| work.profile_id == profile_id)
        .ok_or_else(|| Error::not_found("version", version_id))?;
    let role = lab.harvest.as_deref().unwrap_or_default();
    if version.role != role || owner.id == experiment.id {
        return Err(Error::refused("trial.sourceNotHarvested")
            .param("title", owner.title)
            .param("role", role));
    }
    Ok(())
}

/// Refuse a brick the workspace does not have.
fn check_bricks(conn: &Connection, profile_id: &str, bricks: &[String]) -> Result<()> {
    for id in bricks {
        let found =
            crate::style_brick::get(conn, id)?.filter(|brick| brick.profile_id == profile_id);
        if found.is_none() {
            return Err(Error::refused("trial.unknownBrick").param("brick", id.as_str()));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The anchors.

/// The anchors of an experiment: the lines of the field its lab reads them
/// from, each trimmed, the empty ones left out.
pub fn anchors_of(lab: &Lab, work: &Work) -> Vec<String> {
    let Some(field) = lab.anchors.as_deref() else {
        return Vec::new();
    };
    let Some(serde_json::Value::String(text)) = work.meta.get(field) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The anchors a text has lost.
///
/// An anchor is a concept the trials keep, and a trial says it its own way:
/// `fuzz bass` as *distorted fuzz bass* or *overdriven bass fuzz*, `galloping
/// toms` as *galloping tom-heavy drums* (found on the owner's own files, where
/// a phrase compared whole marked nearly every trial as lost). So an anchor is
/// kept when each of its words stands somewhere in the text, in any order -
/// a plural or a longer form of a word counting as the word, the little words
/// of a phrase (`in the`, `of a`) not counting at all - and what an anchor
/// line holds in brackets is a note on it, not part of it.
pub fn lost_anchors(anchors: &[String], body: &str) -> Vec<String> {
    let words: Vec<String> = crate::phrase::key_of(body)
        .split(' ')
        .map(stem)
        .filter(|word| !word.is_empty())
        .collect();
    anchors
        .iter()
        .filter(|anchor| {
            let wanted = anchor_words(anchor);
            !wanted.is_empty()
                && !wanted
                    .iter()
                    .all(|word| words.iter().any(|said| same_word(said, word)))
        })
        .cloned()
        .collect()
}

/// Words too small to carry an anchor.
const LITTLE_WORDS: [&str; 12] = [
    "a", "an", "the", "of", "in", "on", "at", "to", "by", "for", "with", "and",
];

/// The words an anchor is kept by: its own, outside brackets, the little
/// ones left out - unless they are all it has.
fn anchor_words(anchor: &str) -> Vec<String> {
    let mut plain = String::new();
    let mut depth = 0usize;
    for ch in anchor.chars() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => plain.push(ch),
            _ => {}
        }
    }
    let all: Vec<String> = crate::phrase::key_of(&plain)
        .split(' ')
        .filter(|word| !word.is_empty())
        .map(stem)
        .collect();
    let big: Vec<String> = all
        .iter()
        .filter(|word| !LITTLE_WORDS.contains(&word.as_str()))
        .cloned()
        .collect();
    if big.is_empty() { all } else { big }
}

/// A word without the plural it may carry: `toms` and `tom` are one word.
fn stem(word: &str) -> String {
    let word = word.to_lowercase();
    match word.strip_suffix('s') {
        Some(rest) if rest.chars().count() >= 3 && !rest.ends_with('s') => rest.to_owned(),
        _ => word,
    }
}

/// Whether a word of the text says a word of the anchor: the same word, or
/// a longer form of it (`fuzzy` says `fuzz`), or - for a word of four
/// letters or more - the start of it (`break` says `breakbeat`).
fn same_word(said: &str, wanted: &str) -> bool {
    said == wanted
        || said.starts_with(wanted)
        || (said.chars().count() >= 4 && wanted.starts_with(said))
}

// ---------------------------------------------------------------------------
// The board.

/// Where a kept trial went: a version of a work's text, or a phrase of the
/// dictionary, each found by the trial it remembers.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
#[serde(tag = "kind", rename_all = "lowercase")]
#[ts(rename = "TrialHarvest")]
pub enum Harvest {
    Version {
        work_id: String,
        title: String,
        version_id: String,
        role: String,
        revision: i64,
        label: Option<String>,
    },
    Brick {
        brick_id: String,
        name: String,
        type_key: String,
    },
}

/// The text a rework starts from: a version of another work.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
#[ts(rename = "TrialSource")]
pub struct Source {
    pub work_id: String,
    pub title: String,
    pub version_id: String,
    pub revision: i64,
    pub label: Option<String>,
}

/// A trial as the board shows it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct TrialCard {
    pub trial: Trial,
    /// The takes it was judged by: the files on it, oldest first.
    pub takes: Vec<Asset>,
    /// Where it went, once kept.
    pub harvest: Vec<Harvest>,
    /// The anchors of the experiment its text has lost.
    pub lost_anchors: Vec<String>,
    /// The text it reworks, while that is there.
    pub source: Option<Source>,
}

/// An experiment's board.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct TrialBoard {
    pub work_id: String,
    /// What every trial must keep, one phrase each.
    pub anchors: Vec<String>,
    /// The role a kept trial goes into, when the lab names one.
    pub harvest_role: Option<String>,
    /// The kinds of work a kept trial may go into.
    pub harvest_kinds: Vec<String>,
    /// The composition a trial is written and read by, when the role has
    /// one: what "From the dictionary" picks from.
    pub composition: Option<String>,
    /// The series, in the order each began.
    pub series: Vec<String>,
    pub trials: Vec<TrialCard>,
}

/// How many trials a board holds, by verdict.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct TrialCounts {
    pub total: i64,
    pub kept: i64,
    pub dropped: i64,
    /// Not judged yet.
    pub open: i64,
}

/// The counts of a board.
pub fn counts(conn: &Connection, work_id: &str) -> Result<TrialCounts> {
    Ok(conn.query_row(
        "SELECT count(*),
                coalesce(sum(verdict = 'keep'), 0),
                coalesce(sum(verdict = 'drop'), 0),
                coalesce(sum(verdict IS NULL), 0)
           FROM trial WHERE work_id = ?1",
        params![work_id],
        |row| {
            Ok(TrialCounts {
                total: row.get(0)?,
                kept: row.get(1)?,
                dropped: row.get(2)?,
                open: row.get(3)?,
            })
        },
    )?)
}

/// What became of a trial: the versions and the bricks that remember it.
pub fn harvest_of(conn: &Connection, trial_id: &str) -> Result<Vec<Harvest>> {
    let mut out = Vec::new();
    let mut versions = conn.prepare(
        "SELECT v.work_id, w.title, v.id, v.role, v.revision, v.label
           FROM work_version v JOIN work w ON w.id = v.work_id
          WHERE v.trial_id = ?1
          ORDER BY v.created_at, v.rowid",
    )?;
    let rows = versions.query_map(params![trial_id], |row| {
        Ok(Harvest::Version {
            work_id: row.get(0)?,
            title: row.get(1)?,
            version_id: row.get(2)?,
            role: row.get(3)?,
            revision: row.get(4)?,
            label: row.get(5)?,
        })
    })?;
    for row in rows {
        out.push(row?);
    }
    let mut bricks = conn.prepare(
        "SELECT id, name, type_key FROM style_brick WHERE trial_id = ?1
          ORDER BY created_at, rowid",
    )?;
    let rows = bricks.query_map(params![trial_id], |row| {
        Ok(Harvest::Brick {
            brick_id: row.get(0)?,
            name: row.get(1)?,
            type_key: row.get(2)?,
        })
    })?;
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn source_of(conn: &Connection, version_id: &str) -> Result<Option<Source>> {
    Ok(conn
        .query_row(
            "SELECT v.work_id, w.title, v.id, v.revision, v.label
               FROM work_version v JOIN work w ON w.id = v.work_id
              WHERE v.id = ?1",
            params![version_id],
            |row| {
                Ok(Source {
                    work_id: row.get(0)?,
                    title: row.get(1)?,
                    version_id: row.get(2)?,
                    revision: row.get(3)?,
                    label: row.get(4)?,
                })
            },
        )
        .optional()?)
}

/// The board of an experiment, every trial with what it became.
pub fn board(conn: &Connection, work_id: &str) -> Result<TrialBoard> {
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let Some(lab) = config.lab(&work.kind).cloned() else {
        return Err(Error::refused("trial.noLab").param("title", work.title));
    };
    let anchors = anchors_of(&lab, &work);
    let harvest_role = lab.harvest.clone();
    let harvest_kinds = harvest_kinds_of(&config, &work.kind);
    let composition = config.trial_composition(&work.kind).map(|c| c.key.clone());

    let mut series: Vec<String> = Vec::new();
    let mut trials = Vec::new();
    for trial in for_work(conn, &work.id)? {
        if !series.contains(&trial.series) {
            series.push(trial.series.clone());
        }
        let source = match trial.source_version_id.as_deref() {
            Some(id) => source_of(conn, id)?,
            None => None,
        };
        trials.push(TrialCard {
            takes: crate::asset::for_trial(conn, &trial.id)?,
            harvest: harvest_of(conn, &trial.id)?,
            lost_anchors: lost_anchors(&anchors, &trial.body),
            source,
            trial,
        });
    }
    Ok(TrialBoard {
        work_id: work.id,
        anchors,
        harvest_role,
        harvest_kinds,
        composition,
        series,
        trials,
    })
}

/// The kinds a kept trial of an experiment of `kind` may go into.
pub fn harvest_kinds_of(config: &ProfileConfig, kind: &str) -> Vec<String> {
    match config.lab(kind).and_then(|lab| lab.harvest.as_deref()) {
        Some(role) => config
            .harvest_kinds(role, kind)
            .map(|found| found.key.clone())
            .collect(),
        None => Vec::new(),
    }
}

/// The parent a version taken from `trial` into `work_id`'s `role` is
/// written from: the version the trial reworks, when it is one of that
/// work's in that role - a rework lands as a branch beside its original.
/// Otherwise none: a trial from a sweep of the field is a text from nothing
/// as far as the song is concerned (ADR 0055).
pub fn harvest_parent(
    conn: &Connection,
    trial: &Trial,
    work_id: &str,
    role: &str,
) -> Result<Option<String>> {
    match trial.source_version_id.as_deref() {
        Some(source) if crate::work::version::in_line(conn, work_id, role, source)? => {
            Ok(Some(source.to_owned()))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_anchor_is_kept_by_its_words_in_any_order() {
        let anchors = vec![
            "fuzz bass".to_owned(),
            "galloping toms".to_owned(),
            "in the red".to_owned(),
            "Tambourine (frame drum with jingles)".to_owned(),
        ];
        // The owner's own trials say their anchors their own way.
        assert!(
            lost_anchors(
                &anchors,
                "post-punk revival, overdriven bass fuzz, galloping tom-heavy drums, \
                 everything in the red, riq tambourine"
            )
            .is_empty()
        );
        assert_eq!(
            lost_anchors(
                &anchors,
                "deep sub bass, galloping drums, red lights, tambourine"
            ),
            vec!["fuzz bass".to_owned(), "galloping toms".to_owned()],
            "a word the text never says loses its anchor"
        );
        assert_eq!(
            lost_anchors(&anchors, "fuzz bass, galloping toms, tambourine"),
            vec!["in the red".to_owned()],
            "the little words never stand for an anchor on their own"
        );
    }
}
