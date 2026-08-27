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

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::profile::config::ProfileConfig;

/// What an answer proposed, if anything.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Proposal {
    /// Values along the profile's scoring axes.
    Score {
        /// Axis key to value, already checked against the profile.
        axes: Map<String, Value>,
        /// The assistant's reasoning, when it gave any.
        #[serde(skip_serializing_if = "Option::is_none")]
        note: Option<String>,
        /// Axes the answer named that the profile does not have, and axes of
        /// the profile the answer skipped. Shown rather than hidden: a
        /// proposal that only half fits is worth applying, but not silently.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        unknown: Vec<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        missing: Vec<String>,
    },
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
/// meaningless — 7 out of 10 and 7 out of 100 are different opinions.
pub fn scoring_instruction(config: &ProfileConfig) -> String {
    let axes = config
        .axes
        .iter()
        .map(|axis| format!("  \"{}\": <0-{}>", axis.key, axis.scale))
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        "\n\nEnd your reply with a fenced json block, exactly this shape and \
         nothing else inside it:\n\n```json\n{{\n  \"axes\": {{\n{axes}\n  }},\n  \
         \"note\": \"one sentence on why\"\n}}\n```\n\nSay whatever you like \
         above the block. Use every axis listed and no others."
    )
}

/// Find a scoring proposal in an answer, if it holds one.
///
/// Returns `None` rather than an error when there is no block: an action can be
/// asked for a score and answer in prose anyway, and that is a reply to read,
/// not a failure to report.
pub fn read_score(body: &str, config: &ProfileConfig) -> Option<Proposal> {
    let raw: RawScore = serde_json::from_str(&fenced_json(body)?).ok()?;

    let mut axes = Map::new();
    let mut unknown = Vec::new();

    for (key, value) in raw.axes {
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

    Some(Proposal::Score {
        axes,
        note: raw.note.filter(|note| !note.trim().is_empty()),
        unknown,
        missing,
    })
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
        } = proposal;
        (axes, note, unknown, missing)
    }

    #[test]
    fn a_well_formed_block_is_read() {
        let config = config();
        let first = config.axes[0].key.clone();
        let body = block(&format!(
            r#"{{"axes": {{"{first}": 7}}, "note": "the chorus carries it"}}"#
        ));

        let (axes, note, _, _) = score_of(read_score(&body, &config).unwrap());

        assert_eq!(axes.get(&first).and_then(Value::as_f64), Some(7.0));
        assert_eq!(note.as_deref(), Some("the chorus carries it"));
    }

    #[test]
    fn prose_without_a_block_proposes_nothing() {
        let config = config();

        assert_eq!(
            read_score("I would call it strong, maybe a 7.", &config),
            None,
            "an answer in prose is a reply to read, not a failure"
        );
    }

    #[test]
    fn an_axis_the_profile_does_not_have_is_named_not_applied() {
        let config = config();
        let first = config.axes[0].key.clone();
        let body = block(&format!(r#"{{"axes": {{"{first}": 5, "vibes": 9}}}}"#));

        let (axes, _, unknown, _) = score_of(read_score(&body, &config).unwrap());

        assert!(!axes.contains_key("vibes"));
        assert_eq!(unknown, vec!["vibes".to_owned()]);
    }

    #[test]
    fn axes_the_answer_skipped_are_reported() {
        let config = config();
        let first = config.axes[0].key.clone();
        let body = block(&format!(r#"{{"axes": {{"{first}": 5}}}}"#));

        let (_, _, _, missing) = score_of(read_score(&body, &config).unwrap());

        assert_eq!(missing.len(), config.axes.len() - 1);
        assert!(!missing.contains(&first));
    }

    #[test]
    fn a_value_past_the_scale_is_clamped_not_refused() {
        let config = config();
        let axis = config.axes[0].clone();
        let body = block(&format!(
            r#"{{"axes": {{"{}": {}}}}}"#,
            axis.key,
            axis.scale + 4.0
        ));

        let (axes, _, _, _) = score_of(read_score(&body, &config).unwrap());

        assert_eq!(
            axes.get(&axis.key).and_then(Value::as_f64),
            Some(axis.scale),
            "\"as high as it goes\" is an opinion worth keeping"
        );
    }

    #[test]
    fn a_negative_value_is_clamped_to_zero() {
        let config = config();
        let first = config.axes[0].key.clone();
        let body = block(&format!(r#"{{"axes": {{"{first}": -3}}}}"#));

        let (axes, _, _, _) = score_of(read_score(&body, &config).unwrap());

        assert_eq!(axes.get(&first).and_then(Value::as_f64), Some(0.0));
    }

    #[test]
    fn a_block_naming_only_unknown_axes_proposes_nothing() {
        let config = config();
        let body = block(r#"{"axes": {"vibes": 9, "energy": 4}}"#);

        assert_eq!(
            read_score(&body, &config),
            None,
            "an empty snapshot is not worth offering to write"
        );
    }

    #[test]
    fn the_last_block_wins_over_an_example() {
        let config = config();
        let axis = config.axes[0].key.clone();
        let body = format!(
            "The shape looks like this:\n\n```json\n{{\"axes\": {{\"{axis}\": 0}}}}\n```\n\n\
             And here is my actual answer:\n\n```json\n{{\"axes\": {{\"{axis}\": 8}}}}\n```"
        );

        let (axes, _, _, _) = score_of(read_score(&body, &config).unwrap());

        assert_eq!(
            axes.get(&axis).and_then(Value::as_f64),
            Some(8.0),
            "what the answer settled on comes last"
        );
    }

    #[test]
    fn an_unfenced_object_is_not_a_proposal() {
        let config = config();
        let axis = config.axes[0].key.clone();
        let body = format!("I would write {{\"axes\": {{\"{axis}\": 7}}}} if you asked.");

        assert_eq!(read_score(&body, &config), None);
    }

    #[test]
    fn a_block_of_another_language_is_ignored() {
        let config = config();
        let axis = config.axes[0].key.clone();
        let body = format!("```python\n{{\"axes\": {{\"{axis}\": 7}}}}\n```");

        assert_eq!(read_score(&body, &config), None);
    }

    #[test]
    fn broken_json_inside_the_fence_proposes_nothing() {
        let config = config();

        assert_eq!(read_score(&block("{axes: oops"), &config), None);
    }

    #[test]
    fn an_unclosed_fence_proposes_nothing() {
        let config = config();
        let axis = config.axes[0].key.clone();
        let body = format!("```json\n{{\"axes\": {{\"{axis}\": 7}}}}");

        assert_eq!(
            read_score(&body, &config),
            None,
            "a block that never ended may be a truncated answer"
        );
    }

    #[test]
    fn a_blank_note_is_dropped_rather_than_stored_empty() {
        let config = config();
        let first = config.axes[0].key.clone();
        let body = block(&format!(r#"{{"axes": {{"{first}": 5}}, "note": "   "}}"#));

        let (_, note, _, _) = score_of(read_score(&body, &config).unwrap());

        assert_eq!(note, None);
    }

    #[test]
    fn the_instruction_names_every_axis_with_its_scale() {
        let config = config();

        let instruction = scoring_instruction(&config);

        for axis in &config.axes {
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
}
