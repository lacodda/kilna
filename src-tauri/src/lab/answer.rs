//! What the assistant is asked for a board of trials, and what it answered
//! (ADR 0061).
//!
//! Three things are asked of it: a sweep of the field (trials spread as far
//! apart as the direction allows), variations around a core (each moving one
//! thing of a trial named), and the fix for a trial that was heard (one
//! variation answering what came out). The request sheet - `{trials}` - says
//! which, with the anchors and the board as it stands; the choices -
//! `{choices}` - are the phrases a trial is written from, with what each
//! means and when to take it.
//!
//! The answer, and an agent's proposal, are read by [`read`]: every brick is
//! looked up by id or by its phrase, every parent on the board or earlier in
//! the same answer, and what is not there is left out and said rather than
//! refusing the whole answer for one misspelt name.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::trial::{self, Trial, Verdict};
use crate::error::{Error, Result};
use crate::profile::config::ProfileConfig;
use crate::style_brick::{self, StyleBrick};
use crate::work::Work;

/// What a person asked the board for.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct TrialRequest {
    /// How many trials: one to [`MOST`]. A fix is one, whatever is asked.
    pub count: u8,
    /// The series the new trials stand in, in the person's words.
    pub series: String,
    /// The trial asked about: the core to vary, or the one heard to fix.
    pub around: Option<String>,
    /// Asks for the fix of `around` - what came out of it is heard - rather
    /// than variations of it.
    pub fix: bool,
}

/// The most trials one run is asked for.
pub const MOST: u8 = 12;

impl TrialRequest {
    /// How many trials the answer should hold.
    pub fn total(&self) -> usize {
        if self.fix { 1 } else { usize::from(self.count) }
    }

    /// Refuse a request that asks for nothing, for more than one run gives,
    /// or for a fix of nothing.
    pub fn check(&self) -> Result<()> {
        if self.count > MOST {
            return Err(Error::refused("trial.tooMany").param("most", MOST));
        }
        if self.total() == 0 {
            return Err(Error::refused("trial.nothingAsked"));
        }
        if self.fix && self.around.is_none() {
            return Err(Error::refused("trial.fixOfNothing"));
        }
        Ok(())
    }
}

/// The trial a proposed one varies: one on the board, or one earlier in the
/// same answer, by its index from 0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename = "TrialParent")]
pub enum Parent {
    Trial(String),
    Earlier(usize),
}

/// A trial inside a proposal: what it is, already checked against the
/// workspace.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(rename = "PackagedTrial")]
pub struct Packaged {
    #[serde(default)]
    pub series: String,
    #[serde(default)]
    pub angle: String,
    pub body: String,
    #[serde(default)]
    pub bricks: Vec<String>,
    #[serde(default)]
    pub reference: String,
    #[serde(default)]
    pub outcome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Verdict>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Parent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_version_id: Option<String>,
    #[serde(default)]
    pub run_first: bool,
}

/// A part of an answer's trial left out because the workspace has nothing
/// by that name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(rename = "TrialDropped")]
pub struct Dropped {
    /// The trial's number in the answer, from 1.
    pub trial: usize,
    /// `brick`, `parent`, `source`, `verdict`, or `trial` for one with no
    /// text, left out whole.
    pub part: String,
    pub value: String,
}

/// What an answer's block came to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Read {
    pub trials: Vec<Packaged>,
    pub dropped: Vec<Dropped>,
}

/// The board a proposal is read for: an experiment of `kind` in the
/// workspace `profile_id` - one that exists, or one a package is about to
/// make, which has no board yet.
#[derive(Debug, Clone, Copy)]
pub struct For<'a> {
    pub profile_id: &'a str,
    pub kind: &'a str,
    pub work_id: Option<&'a str>,
}

impl<'a> For<'a> {
    pub fn work(work: &'a Work) -> Self {
        Self {
            profile_id: &work.profile_id,
            kind: &work.kind,
            work_id: Some(&work.id),
        }
    }
}

/// The phrases a trial of an experiment of `kind` may be picked from: the
/// bricks of its composition's types that say something, ready ones first.
fn phrases_of(
    conn: &Connection,
    config: &ProfileConfig,
    profile_id: &str,
    kind: &str,
) -> Result<Vec<StyleBrick>> {
    let Some(composition) = config.trial_composition(kind) else {
        return Ok(Vec::new());
    };
    let types: Vec<&str> = composition.types().collect();
    let mut bricks: Vec<StyleBrick> =
        style_brick::list(conn, profile_id, &style_brick::StyleBrickFilter::default())?
            .into_iter()
            .filter(|brick| types.contains(&brick.type_key.as_str()))
            .filter(|brick| {
                brick
                    .description
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty())
            })
            .collect();
    bricks.sort_by_key(|brick| brick.status != style_brick::READY);
    Ok(bricks)
}

/// A brick named by its id, its phrase or its name.
fn brick_named<'a>(bricks: &'a [StyleBrick], named: &str) -> Option<&'a StyleBrick> {
    let named = named.trim();
    let key = crate::phrase::key_of(named);
    bricks.iter().find(|brick| brick.id == named).or_else(|| {
        bricks.iter().find(|brick| {
            brick
                .description
                .as_deref()
                .is_some_and(|phrase| crate::phrase::key_of(phrase) == key)
                || crate::phrase::key_of(&brick.name) == key
        })
    })
}

fn text(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// Read the trials of an answer's block - or of an agent's proposal - for
/// the board `board` names.
///
/// The block is `{"trials": [...]}` or the list alone. A trial with no text
/// is left out whole; a brick, a parent or a version the workspace does not
/// have is left out of its trial and said in `dropped`. When the request
/// names a trial to vary or to fix, every trial read is its child, whatever
/// the answer says. Refused only when nothing at all could be read.
pub fn read(
    conn: &Connection,
    config: &ProfileConfig,
    board: For<'_>,
    raw: &Value,
    request: Option<&TrialRequest>,
) -> Result<Read> {
    let list = match raw {
        Value::Array(list) => list,
        Value::Object(object) => object
            .get("trials")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::refused("trial.noTrials"))?,
        _ => return Err(Error::refused("trial.noTrials")),
    };
    let bricks = phrases_of(conn, config, board.profile_id, board.kind)?;
    let on_board = match board.work_id {
        Some(id) => trial::for_work(conn, id)?,
        None => Vec::new(),
    };
    let around = request.and_then(|request| request.around.as_deref());
    let series = request
        .map(|request| request.series.trim().to_owned())
        .unwrap_or_default();
    let mut out = Read::default();
    // The answer's own numbering, read as it stands: a trial left out still
    // holds its number, so a later one naming it is told so.
    let mut kept_at: Vec<Option<usize>> = Vec::with_capacity(list.len());

    for (index, raw) in list.iter().enumerate() {
        let number = index + 1;
        let mut leave = |part: &str, value: &str| {
            out.dropped.push(Dropped {
                trial: number,
                part: part.to_owned(),
                value: value.to_owned(),
            });
        };
        let object = match raw {
            Value::Object(object) => object.clone(),
            // A bare string is a trial's text and nothing else.
            Value::String(body) if !body.trim().is_empty() => {
                let mut object = serde_json::Map::new();
                object.insert("body".into(), Value::String(body.clone()));
                object
            }
            other => {
                leave("trial", &other.to_string());
                kept_at.push(None);
                continue;
            }
        };
        let Some(body) = text(&object, "body").or_else(|| text(&object, "prompt")) else {
            leave("trial", &text(&object, "angle").unwrap_or_default());
            kept_at.push(None);
            continue;
        };

        let mut picked = Vec::new();
        for named in object
            .get("bricks")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            match brick_named(&bricks, named) {
                Some(brick) if !picked.contains(&brick.id) => picked.push(brick.id.clone()),
                Some(_) => {}
                None => leave("brick", named),
            }
        }

        let parent = match around {
            Some(id) => Some(Parent::Trial(id.to_owned())),
            None => match object.get("parent") {
                None | Some(Value::Null) => None,
                Some(Value::Number(n)) => {
                    let earlier = n
                        .as_u64()
                        .and_then(|n| usize::try_from(n).ok())
                        .filter(|&n| n >= 1 && n < number)
                        .and_then(|n| kept_at[n - 1]);
                    if earlier.is_none() {
                        leave("parent", &n.to_string());
                    }
                    earlier.map(Parent::Earlier)
                }
                Some(Value::String(id)) => {
                    let id = id.trim();
                    if on_board.iter().any(|trial| trial.id == id) {
                        Some(Parent::Trial(id.to_owned()))
                    } else {
                        leave("parent", id);
                        None
                    }
                }
                Some(other) => {
                    leave("parent", &other.to_string());
                    None
                }
            },
        };

        let source_version_id = match text(&object, "source").or_else(|| text(&object, "from")) {
            Some(id) => {
                if source_is_harvested(conn, config, board, &id)? {
                    Some(id)
                } else {
                    leave("source", &id);
                    None
                }
            }
            None => None,
        };

        let verdict = match text(&object, "verdict").as_deref() {
            None => None,
            Some("keep" | "kept") => Some(Verdict::Keep),
            Some("drop" | "dropped") => Some(Verdict::Drop),
            Some(other) => {
                leave("verdict", other);
                None
            }
        };

        kept_at.push(Some(out.trials.len()));
        out.trials.push(Packaged {
            series: text(&object, "series").unwrap_or_else(|| series.clone()),
            angle: text(&object, "angle").unwrap_or_default(),
            body,
            bricks: picked,
            reference: text(&object, "reference").unwrap_or_default(),
            outcome: text(&object, "outcome").unwrap_or_default(),
            verdict,
            parent,
            source_version_id,
            run_first: object.get("run_first").and_then(Value::as_bool) == Some(true),
        });
    }

    if out.trials.is_empty() {
        return Err(Error::refused("trial.noTrials"));
    }
    Ok(out)
}

/// Whether `version_id` is a text a trial of the board may rework: a version
/// of the role its lab keeps trials in, of another work of the workspace.
fn source_is_harvested(
    conn: &Connection,
    config: &ProfileConfig,
    board: For<'_>,
    version_id: &str,
) -> Result<bool> {
    let Some(role) = config
        .lab(board.kind)
        .and_then(|lab| lab.harvest.as_deref())
    else {
        return Ok(false);
    };
    let Some(version) = crate::work::version::get(conn, version_id)? else {
        return Ok(false);
    };
    let Some(owner) = crate::work::get(conn, &version.work_id)? else {
        return Ok(false);
    };
    Ok(version.role == role
        && owner.profile_id == board.profile_id
        && Some(owner.id.as_str()) != board.work_id)
}

// ---------------------------------------------------------------------------
// What the assistant is asked.

/// A trial on one line, for the request.
fn trial_line(trial: &Trial) -> String {
    let mut line = String::new();
    if !trial.angle.is_empty() {
        line.push_str(&format!("({}) ", one_line(&trial.angle)));
    }
    line.push_str(&format!("`{}`", one_line(&trial.body)));
    if !trial.reference.is_empty() {
        line.push_str(&format!(" - listen to: {}", one_line(&trial.reference)));
    }
    if !trial.outcome.is_empty() {
        line.push_str(&format!(" - heard: {}", one_line(&trial.outcome)));
    }
    line
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// What the board asks for and what stands on it: the `{trials}` of an
/// action about an experiment.
pub fn request_sheet(conn: &Connection, work: &Work, request: &TrialRequest) -> Result<String> {
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let lab = config
        .lab(&work.kind)
        .ok_or_else(|| Error::refused("trial.noLab").param("title", work.title.clone()))?;
    let board = trial::for_work(conn, &work.id)?;
    let around = match request.around.as_deref() {
        Some(id) => Some(
            board
                .iter()
                .find(|trial| trial.id == id)
                .ok_or_else(|| Error::refused("trial.parentNotOnBoard").param("parent", id))?,
        ),
        None => None,
    };
    let count = request.total();
    let plural = if count == 1 { "" } else { "s" };
    let mut out = match (around, request.fix) {
        (Some(trial), true) => format!(
            "Asked for: what went wrong. The trial below was heard; propose ONE trial that fixes it - its core kept, one line changed or added.\n\nThe trial: {}\n",
            trial_line(trial)
        ),
        (Some(trial), false) => format!(
            "Asked for: {count} variation{plural} around a core. Each moves ONE thing of the trial below and keeps the rest word for word.\n\nThe core: {}\n",
            trial_line(trial)
        ),
        (None, _) => format!(
            "Asked for: {count} trial{plural} sweeping the field - spread as far apart as the direction allows, no two neighbours.\n"
        ),
    };
    if !request.series.trim().is_empty() {
        out.push_str(&format!(
            "\nThey stand in the series “{}”.\n",
            request.series.trim()
        ));
    }

    let anchors = trial::anchors_of(lab, work);
    if !anchors.is_empty() {
        out.push_str("\nThe anchors - every trial keeps each of them, word for word:\n");
        for anchor in &anchors {
            out.push_str(&format!("- {anchor}\n"));
        }
    }

    let mut list = |heading: &str, verdict: Option<Verdict>| {
        let trials: Vec<&Trial> = board
            .iter()
            .filter(|trial| trial.verdict == verdict)
            .filter(|trial| around.is_none_or(|core| core.id != trial.id))
            .collect();
        if trials.is_empty() {
            return;
        }
        out.push_str(&format!("\n{heading}\n"));
        for trial in trials {
            out.push_str(&format!("- {}\n", trial_line(trial)));
        }
    };
    list("Kept - what works; build on it:", Some(Verdict::Keep));
    list(
        "Dropped - what does not; stay away from it:",
        Some(Verdict::Drop),
    );
    list("On the board, not judged yet - do not repeat these:", None);

    // The experiment's own texts - its brief, what was found so far - each
    // newest, under the craft's name for it: what the trials are for.
    for role in &config.vocabulary(&work.kind).version_roles {
        if let Some(body) = crate::work::version::latest(conn, &work.id, &role.key)?
            .map(|version| version.body)
            .filter(|body| !body.trim().is_empty())
        {
            out.push_str(&format!(
                "\nThe experiment's {}:\n\n{}\n",
                role.label.in_locale("en").to_lowercase(),
                body.trim()
            ));
        }
    }
    Ok(out.trim_end().to_owned())
}

/// Everything a trial may be written from, with the ids to name it by: the
/// `{choices}` of an action about an experiment.
pub fn choices_sheet(conn: &Connection, work: &Work) -> Result<String> {
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let Some(composition) = config.trial_composition(&work.kind) else {
        return Ok("The dictionary holds no phrases a trial is read against: write each trial in your own words.".into());
    };
    let bricks = phrases_of(conn, &config, &work.profile_id, &work.kind)?;
    let house = crate::phrase::house(conn, &work.profile_id)?;
    let mut out = String::from(
        "The phrases of the dictionary, block by block in the order a line is usually built; ★ marks the channel's own. Name the ones you use by id in `bricks`.\n",
    );
    for part in &composition.parts {
        let Some(style) = config.style_type(&part.type_key) else {
            continue;
        };
        let mut of_type: Vec<&StyleBrick> = bricks
            .iter()
            .filter(|brick| brick.type_key == part.type_key && brick.status == style_brick::READY)
            .collect();
        if of_type.is_empty() {
            continue;
        }
        of_type.sort_by_key(|brick| !house.contains(&brick.id));
        out.push_str(&format!("\n{} (`{}`)", style.label.as_str(), style.key));
        if let Some(rule) = &part.rule {
            out.push_str(&format!(" - {}", rule.as_str()));
        }
        out.push_str(":\n");
        for brick in of_type {
            let mut line = format!(
                "- `{}` {}",
                brick.id,
                brick.description.as_deref().unwrap_or_default().trim()
            );
            if house.contains(&brick.id) {
                line.push_str(" ★");
            }
            if let Some(explanation) = &brick.explanation {
                line.push_str(&format!(" - {}", one_line(explanation.in_locale("en"))));
            }
            if let Some(when) = brick
                .when_to_use
                .as_deref()
                .filter(|when| !when.trim().is_empty())
            {
                line.push_str(&format!(" When: {}", one_line(when)));
            }
            out.push_str(&line);
            out.push('\n');
        }
    }
    if !composition.fields.is_empty() {
        let mut closing = Vec::new();
        for field in &composition.fields {
            if let Some(value) =
                crate::phrase::compose::field_value(&Value::Object(work.meta.clone()), &field.field)
            {
                closing.push(field.write(&value));
            }
        }
        if !closing.is_empty() {
            out.push_str(&format!(
                "\nThe line closes with the experiment's own fields: {}.\n",
                closing.join(", ")
            ));
        }
    }
    if let Some(limit) = composition.limit {
        out.push_str(&format!(
            "\nThe generator reads at most {limit} characters of a line.\n"
        ));
    }
    Ok(out.trim_end().to_owned())
}

/// What an action about an experiment appends to its prompt: the block to
/// answer with, spelled out.
pub fn instruction(request: &TrialRequest) -> String {
    let total = request.total();
    let parent = if request.around.is_some() {
        "\n- Every trial varies the one named above: do not give `parent`."
    } else {
        "\n- `parent` is optional: the number of an earlier trial in this answer that this one varies."
    };
    format!(
        "\n\nEnd your answer with one fenced ```json block holding exactly {total} trial{plural}:\n\n\
```json\n\
{{\"trials\": [\n  {{\n    \"angle\": \"what this one moves, or where on the field it stands - a few words\",\n    \"body\": \"the line the generator reads, the most important phrase first\",\n    \"bricks\": [\"the id of each phrase of the dictionary it uses\"],\n    \"reference\": \"who to listen to, or leave it out\"\n  }}\n]}}\n```\n\n\
- `body` is required; every other key may be left out.\n\
- No artist or band names in `body`.{parent}",
        plural = if total == 1 { "" } else { "s" },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_asks_for_something_and_not_too_much() {
        let none = TrialRequest::default();
        assert_eq!(
            none.check().unwrap_err().refusal().unwrap().code,
            "trial.nothingAsked"
        );
        let many = TrialRequest {
            count: MOST + 1,
            ..TrialRequest::default()
        };
        assert_eq!(
            many.check().unwrap_err().refusal().unwrap().code,
            "trial.tooMany"
        );
        let fix = TrialRequest {
            fix: true,
            ..TrialRequest::default()
        };
        assert_eq!(
            fix.check().unwrap_err().refusal().unwrap().code,
            "trial.fixOfNothing"
        );
        let one = TrialRequest {
            count: 5,
            fix: true,
            around: Some("t".into()),
            ..TrialRequest::default()
        };
        assert_eq!(one.total(), 1, "a fix is one trial, whatever was asked");
        one.check().unwrap();
    }
}
