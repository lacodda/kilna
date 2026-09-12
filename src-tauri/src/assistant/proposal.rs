//! Reading a structured result out of an answer.
//!
//! An action can ask for something the application knows how to act on — a
//! score along the profile's axes, say — rather than for prose. The assistant
//! still answers in text; what changes is that part of that text is a fenced
//! JSON block this module knows how to find.
//!
//! **The assistant never writes to the workspace.** It proposes, kilna shows
//! what was proposed, and a person applies it. That rule is the reason this is
//! a parser and not a writer: everything here turns text into something the
//! frontend can display next to an "apply" button, and nothing here touches the
//! database. Decided in v0.28 for versions, and it holds unchanged for scores.
//! What a person applies becomes rows in [`super::apply`] — the one place a
//! proposal is written, and the place that marks it applied.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::profile::config::{ProfileConfig, WorkKind};

/// What an answer proposed, if anything.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Proposal {
    /// Values along the profile's scoring axes.
    Score {
        /// Axis key to value, already checked against the profile.
        axes: Map<String, Value>,
        /// The assistant's reasoning, when it gave any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
        /// Axes the answer named that the profile does not have, and axes of
        /// the profile the answer skipped. Shown rather than hidden: a
        /// proposal that only half fits is worth applying, but not silently.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        unknown: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        missing: Vec<String>,
    },
    /// A new version in a role. The text itself is the message body — that
    /// is what *insert as version* keeps, verbatim — so the proposal carries
    /// only where it goes. Made by an agent outside the window (`kilna --mcp`).
    Version {
        role: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    /// A note, on the chat's work or on nothing in particular; the body is
    /// the message body. Made by an agent outside the window.
    Note {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    /// A whole work, or a package of changes to one: the fields of the
    /// overview, versions by role, a score, notes — applied together with one
    /// click. Made by an agent outside the window.
    ///
    /// Which of the two it is comes from the chat the message is in: a chat
    /// on a work receives packages for that work, a chat on nothing receives
    /// new works, which then need `title` and `kind`. The texts travel here
    /// rather than in the message body, because there are several; the body
    /// is a rendering of the package a person reads before applying it.
    Work {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// The kind of the new work. Not `kind`: that name is the tag that
        /// says which variant this is.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        work_kind: Option<String>,
        /// Overview field key to value, already checked against the profile.
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        fields: Map<String, Value>,
        /// Field keys the package named that the profile does not have.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        unknown_fields: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        versions: Vec<PackagedVersion>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        score: Option<Marks>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        notes: Vec<PackagedNote>,
        /// The storyboard of a new video, or scenes added to an existing
        /// one's board. Only for a kind that has a storyboard.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        scenes: Vec<PackagedScene>,
    },
    /// A storyboard for the chat's work: scenes added after the last, or
    /// the whole board replaced. Made by an agent outside the window.
    ///
    /// Its own variant rather than a package with only scenes, because the
    /// decision is different: *add to the board* keeps what is there and
    /// *replace the board* takes it to the trash, and the button has to say
    /// which. The body is a rendering of the board a person reads first.
    Scenes {
        scenes: Vec<PackagedScene>,
        /// Replace the board rather than add to it: a scene with the same
        /// number is rewritten in place, the rest of the old board goes to
        /// the trash, and numbers without a scene yet are created.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        replace: bool,
    },
}

/// A scene inside a proposal: the fields of a row, already checked against
/// the kind's words — the shot type and the block keys are the kind's, and
/// the span is a span.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackagedScene {
    /// The number on the board; after the last when omitted on an added
    /// scene, and in order from 1 on a replaced board.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shot_type: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Prompt blocks by the kind's `scene_blocks` key.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub blocks: Map<String, Value>,
}

/// A version inside a package: the text travels with it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackagedVersion {
    pub role: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A note inside a package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackagedNote {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub body: String,
}

/// Marks along the axes, checked against the kind — the inside of a score
/// proposal, reused by a package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Marks {
    pub axes: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknown: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing: Vec<String>,
}

/// The shape an answer is asked to produce.
#[derive(Debug, Clone, Deserialize)]
struct RawScore {
    #[serde(default)]
    axes: Map<String, Value>,
    #[serde(default)]
    note: Option<String>,
}

/// What a scoring action appends to its prompt.
///
/// Spelled out rather than left to the model's judgement: the block has to be
/// findable, and "reply with JSON" produces a different shape every time. The
/// axes are named with their scales because a number without its ceiling is
/// meaningless — 7 out of 10 and 7 out of 100 are different opinions. And
/// they are named with everything else the profile says about them — the
/// label, the question the axis asks, the marks of its rubric, its weight —
/// because that is the judgement the author wrote down, and a model handed
/// only `"hook": <0-10>` judges by its own. The tiers follow, so the answer
/// knows what its total means.
pub fn scoring_instruction(config: &ProfileConfig, kind: &str) -> String {
    let vocabulary = config.vocabulary(kind);

    let mut guide = String::new();
    for axis in &vocabulary.axes {
        guide.push_str(&format!(
            "- `{}` — {} (0 to {}, weight {})",
            axis.key, axis.label, axis.scale, axis.weight
        ));
        if let Some(description) = axis.description.as_deref().filter(|d| !d.trim().is_empty()) {
            guide.push_str(&format!(": {}", description.trim()));
        }
        guide.push('\n');
        if !axis.options.is_empty() {
            let options = axis
                .options
                .iter()
                .map(|option| format!("{} = {}", option.value, option.label))
                .collect::<Vec<_>>()
                .join("; ");
            guide.push_str(&format!("  answers: {options}\n"));
        }
        for mark in &axis.rubric {
            guide.push_str(&format!("  {} — {}\n", mark.at, mark.label));
        }
    }

    let tiers = if vocabulary.tiers.is_empty() {
        String::new()
    } else {
        let mut sorted: Vec<_> = vocabulary.tiers.iter().collect();
        sorted.sort_by(|a, b| {
            b.min
                .partial_cmp(&a.min)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        format!(
            "\nThe total is the weighted mean of the axes on a 0–100 scale; the tiers are: {}.\n",
            sorted
                .iter()
                .map(|tier| format!("{} from {}", tier.label, tier.min))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let shape = vocabulary
        .axes
        .iter()
        .map(|axis| format!("  \"{}\": <0-{}>", axis.key, axis.scale))
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        "\n\nJudge it along these axes, exactly as the profile defines them:\n\n{guide}{tiers}\n\
         End your reply with a fenced json block, exactly this shape and \
         nothing else inside it:\n\n```json\n{{\n  \"axes\": {{\n{shape}\n  }},\n  \
         \"note\": \"one sentence on why\"\n}}\n```\n\nSay whatever you like \
         above the block. Use every axis listed and no others."
    )
}

/// What an action that produces a version appends to its prompt.
///
/// The whole answer is what gets kept, verbatim, so the model is told not to
/// wrap it: a preamble or a closing question would become part of the text.
pub fn version_instruction(role_label: &str) -> String {
    format!(
        "\n\nYour whole reply is kept as the {role_label}, word for word: write only it — no \
         preamble, no closing question, no fences around the whole."
    )
}

/// Find a scoring proposal in an answer, if it holds one.
///
/// Returns `None` rather than an error when there is no block: an action can be
/// asked for a score and answer in prose anyway, and that is a reply to read,
/// not a failure to report.
pub fn read_score(body: &str, config: &ProfileConfig, kind: &str) -> Option<Proposal> {
    let raw: RawScore = serde_json::from_str(&fenced_json(body)?).ok()?;
    score_from(raw.axes, raw.note, config.vocabulary(kind))
}

/// A scoring proposal out of marks by axis key, checked against the profile.
///
/// The check the fenced block gets, and the same one an agent's `propose_score`
/// gets: unknown axes are named rather than dropped silently, marks are clamped
/// to the axis scale, and marks on no known axis at all are nothing to propose.
pub fn score_from(
    raw: Map<String, Value>,
    note: Option<String>,
    config: &WorkKind,
) -> Option<Proposal> {
    let Marks {
        axes,
        note,
        unknown,
        missing,
    } = marks_from(raw, note, config)?;
    Some(Proposal::Score {
        axes,
        note,
        unknown,
        missing,
    })
}

/// Marks by axis key, checked against a kind — see [`score_from`].
pub fn marks_from(
    raw: Map<String, Value>,
    note: Option<String>,
    config: &WorkKind,
) -> Option<Marks> {
    let mut axes = Map::new();
    let mut unknown = Vec::new();

    for (key, value) in raw {
        match config.axes.iter().find(|axis| axis.key == key) {
            // Clamped rather than refused: an answer that says 11 out of 10
            // means "as high as it goes", and throwing the whole proposal away
            // over one number would waste the rest of it.
            Some(axis) => {
                if let Some(number) = value.as_f64() {
                    let bounded = number.clamp(0.0, axis.scale);
                    axes.insert(key, serde_json::json!(bounded));
                } else {
                    unknown.push(key);
                }
            }
            None => unknown.push(key),
        }
    }

    let missing: Vec<String> = config
        .axes
        .iter()
        .filter(|axis| !axes.contains_key(&axis.key))
        .map(|axis| axis.key.clone())
        .collect();

    // Nothing usable is nothing to propose. A block naming only axes the
    // profile dropped is not a score, and offering to apply it would be
    // offering to write an empty snapshot.
    if axes.is_empty() {
        return None;
    }

    unknown.sort();

    Some(Marks {
        axes,
        note: note.filter(|note| !note.trim().is_empty()),
        unknown,
        missing,
    })
}

/// Overview fields out of a map by key, checked against the profile.
///
/// The same bargain as the axes: a key the profile has no field for is named
/// rather than written — a value under an unknown key would sit in `meta`
/// where no screen shows it. Values are kept as given; the overview stores
/// what is typed and a number that is not one yet stays as typed there too.
/// Empty values are dropped: an agent that sends `""` for a field it has
/// nothing to say about must not blank what is there.
pub fn fields_from(
    raw: Map<String, Value>,
    config: &ProfileConfig,
) -> (Map<String, Value>, Vec<String>) {
    let mut fields = Map::new();
    let mut unknown = Vec::new();

    for (key, value) in raw {
        if !config.work_meta_fields.iter().any(|field| field.key == key) {
            unknown.push(key);
            continue;
        }
        let empty = match &value {
            Value::Null => true,
            Value::String(text) => text.trim().is_empty(),
            _ => false,
        };
        if !empty {
            fields.insert(key, value);
        }
    }

    unknown.sort();
    (fields, unknown)
}

/// Scenes out of what an agent sent, checked against the kind — the check
/// the storyboard's own writer makes, made here so the package is refused
/// whole before it lands, the way an unknown role refuses a package.
///
/// A shot type or a block key the kind does not name is a refusal, not an
/// omission: a scene typed as `closeup` when the vocabulary says `close`
/// would never be found by the board's filter, and a block under a key no
/// template reads would never be shown. A kind with no storyboard at all
/// takes no scenes. Numbers, when given, start at 1; a scene cannot end
/// before it starts.
pub fn scenes_from(raw: &[Value], kind: &WorkKind) -> crate::error::Result<Vec<PackagedScene>> {
    use crate::error::Error;

    if kind.shot_types.is_empty() && kind.scene_blocks.is_empty() {
        return Err(Error::Other(format!(
            "`{}` has no storyboard: the kind names no kinds of shot and no prompt blocks",
            kind.key
        )));
    }
    let names = |keys: &mut dyn Iterator<Item = &str>| {
        let listed: Vec<String> = keys.map(|key| format!("`{key}`")).collect();
        if listed.is_empty() {
            "none".to_owned()
        } else {
            listed.join(", ")
        }
    };

    let mut scenes = Vec::with_capacity(raw.len());
    for (index, item) in raw.iter().enumerate() {
        let Some(item) = item.as_object() else {
            return Err(Error::Other(format!(
                "scene {} is not an object",
                index + 1
            )));
        };
        let text = |key: &str| {
            item.get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        let number = |key: &str| item.get(key).and_then(Value::as_f64);

        let position = match item.get("position") {
            None | Some(Value::Null) => None,
            Some(value) => match value.as_i64() {
                Some(position) if position >= 1 => Some(position),
                _ => {
                    return Err(Error::Other(format!(
                        "scene {}: a scene is numbered from 1",
                        index + 1
                    )));
                }
            },
        };

        let shot_type = text("shot_type");
        if let Some(shot) = shot_type.as_deref() {
            if !kind.shot_types.iter().any(|s| s.key == shot) {
                return Err(Error::Other(format!(
                    "scene {}: no kind of shot `{shot}` for `{}`; its kinds of shot are {}",
                    index + 1,
                    kind.key,
                    names(&mut kind.shot_types.iter().map(|s| s.key.as_str()))
                )));
            }
        }

        let mut blocks = Map::new();
        if let Some(given) = item.get("blocks") {
            let Some(given) = given.as_object() else {
                return Err(Error::Other(format!(
                    "scene {}: `blocks` must be an object of block key to text",
                    index + 1
                )));
            };
            for (key, value) in given {
                if !kind.scene_blocks.iter().any(|b| b.key == *key) {
                    return Err(Error::Other(format!(
                        "scene {}: no prompt block `{key}` for `{}`; its blocks are {}",
                        index + 1,
                        kind.key,
                        names(&mut kind.scene_blocks.iter().map(|b| b.key.as_str()))
                    )));
                }
                // A block is text; a blank one is no block, so an agent that
                // sends "" for what it has nothing to say about leaves the box
                // empty rather than filling it with nothing.
                match value {
                    Value::String(body) if !body.trim().is_empty() => {
                        blocks.insert(key.clone(), Value::String(body.clone()));
                    }
                    Value::String(_) | Value::Null => {}
                    other => {
                        blocks.insert(key.clone(), Value::String(other.to_string()));
                    }
                }
            }
        }

        let starts_at = number("starts_at");
        let ends_at = number("ends_at");
        if let (Some(from), Some(to)) = (starts_at, ends_at) {
            if to < from {
                return Err(Error::Other(format!(
                    "scene {}: it ends at {to} before it starts at {from}",
                    index + 1
                )));
            }
        }
        for at in [starts_at, ends_at].into_iter().flatten() {
            if !(at.is_finite() && at >= 0.0) {
                return Err(Error::Other(format!(
                    "scene {}: seconds are counted from 0",
                    index + 1
                )));
            }
        }

        scenes.push(PackagedScene {
            position,
            section: text("section"),
            starts_at,
            ends_at,
            shot_type,
            description: text("description").unwrap_or_default(),
            blocks,
        });
    }
    Ok(scenes)
}

/// The contents of the last fenced json block in a body.
///
/// The last, not the first: an answer may show an example of the shape before
/// filling it in, and what it settled on is what comes last. The fence is
/// required — a bare object in prose is too easy to find by accident.
fn fenced_json(body: &str) -> Option<String> {
    let mut found: Option<String> = None;
    let mut current: Option<Vec<&str>> = None;

    for line in body.lines() {
        let trimmed = line.trim();
        match &mut current {
            None => {
                // ```json, ```JSON, or ``` json. Anything else — prose, or a
                // fence of another language — is skipped rather than ending
                // the search: the block worth reading usually comes after
                // several lines that are neither.
                if let Some(rest) = trimmed.strip_prefix("```") {
                    if rest.trim().eq_ignore_ascii_case("json") {
                        current = Some(Vec::new());
                    }
                }
            }
            Some(collected) => {
                if trimmed.starts_with("```") {
                    found = Some(collected.join("\n"));
                    current = None;
                } else {
                    collected.push(line);
                }
            }
        }
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;

    fn config() -> ProfileConfig {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        profile::active(&conn).unwrap().unwrap().config
    }

    fn block(inner: &str) -> String {
        format!("Here is what I think.\n\n```json\n{inner}\n```")
    }

    fn score_of(
        proposal: Proposal,
    ) -> (Map<String, Value>, Option<String>, Vec<String>, Vec<String>) {
        let Proposal::Score {
            axes,
            note,
            unknown,
            missing,
        } = proposal
        else {
            panic!("a fenced block reads as a score");
        };
        (axes, note, unknown, missing)
    }

    #[test]
    fn a_well_formed_block_is_read() {
        let config = config();
        let first = config.work_kinds[0].axes[0].key.clone();
        let body = block(&format!(
            r#"{{"axes": {{"{first}": 7}}, "note": "the chorus carries it"}}"#
        ));

        let (axes, note, _, _) = score_of(read_score(&body, &config, "song").unwrap());

        assert_eq!(axes.get(&first).and_then(Value::as_f64), Some(7.0));
        assert_eq!(note.as_deref(), Some("the chorus carries it"));
    }

    #[test]
    fn prose_without_a_block_proposes_nothing() {
        let config = config();

        assert_eq!(
            read_score("I would call it strong, maybe a 7.", &config, "song"),
            None,
            "an answer in prose is a reply to read, not a failure"
        );
    }

    #[test]
    fn an_axis_the_profile_does_not_have_is_named_not_applied() {
        let config = config();
        let first = config.work_kinds[0].axes[0].key.clone();
        let body = block(&format!(r#"{{"axes": {{"{first}": 5, "vibes": 9}}}}"#));

        let (axes, _, unknown, _) = score_of(read_score(&body, &config, "song").unwrap());

        assert!(!axes.contains_key("vibes"));
        assert_eq!(unknown, vec!["vibes".to_owned()]);
    }

    #[test]
    fn axes_the_answer_skipped_are_reported() {
        let config = config();
        let first = config.work_kinds[0].axes[0].key.clone();
        let body = block(&format!(r#"{{"axes": {{"{first}": 5}}}}"#));

        let (_, _, _, missing) = score_of(read_score(&body, &config, "song").unwrap());

        assert_eq!(missing.len(), config.work_kinds[0].axes.len() - 1);
        assert!(!missing.contains(&first));
    }

    #[test]
    fn a_value_past_the_scale_is_clamped_not_refused() {
        let config = config();
        let axis = config.work_kinds[0].axes[0].clone();
        let body = block(&format!(
            r#"{{"axes": {{"{}": {}}}}}"#,
            axis.key,
            axis.scale + 4.0
        ));

        let (axes, _, _, _) = score_of(read_score(&body, &config, "song").unwrap());

        assert_eq!(
            axes.get(&axis.key).and_then(Value::as_f64),
            Some(axis.scale),
            "\"as high as it goes\" is an opinion worth keeping"
        );
    }

    #[test]
    fn a_negative_value_is_clamped_to_zero() {
        let config = config();
        let first = config.work_kinds[0].axes[0].key.clone();
        let body = block(&format!(r#"{{"axes": {{"{first}": -3}}}}"#));

        let (axes, _, _, _) = score_of(read_score(&body, &config, "song").unwrap());

        assert_eq!(axes.get(&first).and_then(Value::as_f64), Some(0.0));
    }

    #[test]
    fn a_block_naming_only_unknown_axes_proposes_nothing() {
        let config = config();
        let body = block(r#"{"axes": {"vibes": 9, "energy": 4}}"#);

        assert_eq!(
            read_score(&body, &config, "song"),
            None,
            "an empty snapshot is not worth offering to write"
        );
    }

    #[test]
    fn the_last_block_wins_over_an_example() {
        let config = config();
        let axis = config.work_kinds[0].axes[0].key.clone();
        let body = format!(
            "The shape looks like this:\n\n```json\n{{\"axes\": {{\"{axis}\": 0}}}}\n```\n\n\
             And here is my actual answer:\n\n```json\n{{\"axes\": {{\"{axis}\": 8}}}}\n```"
        );

        let (axes, _, _, _) = score_of(read_score(&body, &config, "song").unwrap());

        assert_eq!(
            axes.get(&axis).and_then(Value::as_f64),
            Some(8.0),
            "what the answer settled on comes last"
        );
    }

    #[test]
    fn an_unfenced_object_is_not_a_proposal() {
        let config = config();
        let axis = config.work_kinds[0].axes[0].key.clone();
        let body = format!("I would write {{\"axes\": {{\"{axis}\": 7}}}} if you asked.");

        assert_eq!(read_score(&body, &config, "song"), None);
    }

    #[test]
    fn a_block_of_another_language_is_ignored() {
        let config = config();
        let axis = config.work_kinds[0].axes[0].key.clone();
        let body = format!("```python\n{{\"axes\": {{\"{axis}\": 7}}}}\n```");

        assert_eq!(read_score(&body, &config, "song"), None);
    }

    #[test]
    fn broken_json_inside_the_fence_proposes_nothing() {
        let config = config();

        assert_eq!(read_score(&block("{axes: oops"), &config, "song"), None);
    }

    #[test]
    fn an_unclosed_fence_proposes_nothing() {
        let config = config();
        let axis = config.work_kinds[0].axes[0].key.clone();
        let body = format!("```json\n{{\"axes\": {{\"{axis}\": 7}}}}");

        assert_eq!(
            read_score(&body, &config, "song"),
            None,
            "a block that never ended may be a truncated answer"
        );
    }

    #[test]
    fn a_blank_note_is_dropped_rather_than_stored_empty() {
        let config = config();
        let first = config.work_kinds[0].axes[0].key.clone();
        let body = block(&format!(r#"{{"axes": {{"{first}": 5}}, "note": "   "}}"#));

        let (_, note, _, _) = score_of(read_score(&body, &config, "song").unwrap());

        assert_eq!(note, None);
    }

    #[test]
    fn the_instruction_names_every_axis_with_its_scale() {
        let config = config();

        let instruction = scoring_instruction(&config, "song");

        for axis in &config.work_kinds[0].axes {
            assert!(
                instruction.contains(&axis.key),
                "every axis must be named: {}",
                axis.key
            );
        }
        assert!(
            instruction.contains("```json"),
            "the block has to be findable"
        );
    }

    #[test]
    fn fields_are_checked_against_the_profile_and_blanks_are_dropped() {
        let config = config();
        let first = config.work_meta_fields[0].key.clone();
        let mut raw = Map::new();
        raw.insert(first.clone(), Value::from("three"));
        raw.insert("colour".into(), Value::from("blue"));
        raw.insert("premise".into(), Value::from("   "));

        let (fields, unknown) = fields_from(raw, &config);

        assert_eq!(fields.get(&first).and_then(Value::as_str), Some("three"));
        assert!(
            !fields.contains_key("premise"),
            "a blank value must not blank a field"
        );
        assert_eq!(unknown, vec!["colour".to_owned()]);
    }

    #[test]
    fn a_stored_proposal_reads_back_as_it_was_written() {
        let proposal = Proposal::Work {
            title: Some("Harbour lights".into()),
            work_kind: Some("song".into()),
            fields: Map::new(),
            unknown_fields: Vec::new(),
            versions: vec![PackagedVersion {
                role: "lyrics".into(),
                body: "one line".into(),
                label: None,
            }],
            score: None,
            notes: Vec::new(),
            scenes: Vec::new(),
        };
        let stored = serde_json::to_value(&proposal).unwrap();
        assert_eq!(stored["kind"], "work");
        assert!(
            stored.get("fields").is_none(),
            "empty parts are not written"
        );
        let read: Proposal = serde_json::from_value(stored).unwrap();
        assert_eq!(read, proposal);
    }
}

#[cfg(test)]
mod instruction_tests {
    use super::*;
    use crate::profile::config::{Axis, AxisKind, AxisMark, Tier, WorkKind};

    /// The model is told what the author wrote about each axis — the label,
    /// the question, the marks of the rubric, the weight — and what the
    /// total means; not only a key and a ceiling.
    #[test]
    fn the_scoring_instruction_carries_labels_rubrics_and_tiers() {
        let mut kind = WorkKind::new("song", "Song");
        kind.axes.push(Axis {
            key: "imagery".into(),
            label: "Imagery".into(),
            weight: 2.0,
            scale: 10.0,
            description: Some("Can the picture be seen?".into()),
            kind: AxisKind::Scale,
            options: Vec::new(),
            rubric: vec![
                AxisMark {
                    at: 3.0,
                    label: "worn images".into(),
                },
                AxisMark {
                    at: 8.0,
                    label: "one fresh image per verse".into(),
                },
            ],
        });
        kind.tiers.push(Tier {
            key: "hold".into(),
            label: "Hold".into(),
            min: 55.0,
        });
        kind.tiers.push(Tier {
            key: "clip".into(),
            label: "Clip".into(),
            min: 80.0,
        });
        let config = ProfileConfig {
            format: crate::profile::config::FORMAT,
            work_kinds: vec![kind],
            collection_kinds: Vec::new(),
            work_meta_fields: Vec::new(),
            marks: Vec::new(),
            prompts: Vec::new(),
            rhythm: None,
            catalogue_columns: None,
            catalogue_columns_by_kind: None,
        };

        let text = scoring_instruction(&config, "song");

        for expected in [
            "`imagery` — Imagery (0 to 10, weight 2)",
            "Can the picture be seen?",
            "3 — worn images",
            "8 — one fresh image per verse",
            "Clip from 80, Hold from 55",
            "\"imagery\": <0-10>",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
        }
    }

    #[test]
    fn a_version_instruction_names_the_role() {
        let text = version_instruction("Critique");
        assert!(text.contains("kept as the Critique"), "{text}");
    }
}
