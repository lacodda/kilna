//! A text written out of the dictionary (v0.94).
//!
//! The person picks phrases block by block and kilna writes the line: the
//! phrases in the order they were picked - a generator that weighs its words
//! from the left hears the first as the most important - and then the work's
//! own fields as the composition's templates write them. Nothing is invented
//! and nothing is reordered, so the same picks give the same text every time,
//! here, in the window and over MCP.
//!
//! What the composition says of a block - at least one groove, at most four
//! instruments - is said back as a problem, never a refusal: a style without
//! a bass is a choice, and the person is the one who knows.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::profile::config::Composition;
use crate::style_brick;

/// What to write.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct ComposeRequest {
    /// A key of the profile's `compose`.
    pub composition: String,
    /// The bricks picked, by id, in the order they were picked.
    pub bricks: Vec<String>,
    /// The work whose fields close the text, when there is one.
    #[serde(default)]
    pub work_id: Option<String>,
}

/// The text, and what the composition says about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct ComposedText {
    pub text: String,
    /// In characters, as the generator counts them.
    pub length: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub limit: Option<u32>,
    /// A block picked fewer or more times than the composition says.
    pub problems: Vec<ComposeProblem>,
}

/// A block of the composition, picked out of its bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct ComposeProblem {
    pub type_key: String,
    pub picked: usize,
    pub min: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max: Option<u8>,
}

/// The value of a work's field as text: what a template writes.
pub fn field_value(meta: &serde_json::Value, key: &str) -> Option<String> {
    match meta.get(key)? {
        serde_json::Value::String(text) => {
            Some(text.trim().to_owned()).filter(|text| !text.is_empty())
        }
        serde_json::Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

/// Write the text.
pub fn write(
    conn: &Connection,
    profile_id: &str,
    request: &ComposeRequest,
) -> Result<ComposedText> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let composition = config
        .composition(&request.composition)
        .ok_or_else(|| Error::not_found("composition", request.composition.clone()))?;

    let meta = match &request.work_id {
        Some(id) => {
            let work = crate::work::get(conn, id)?.ok_or_else(|| Error::not_found("work", id))?;
            if !composition.kinds.contains(&work.kind) {
                return Err(Error::refused("compose.wrongKind")
                    .param("kind", work.kind.clone())
                    .param("title", work.title.clone()));
            }
            serde_json::Value::Object(work.meta)
        }
        None => serde_json::Value::Null,
    };

    let mut phrases: Vec<String> = Vec::with_capacity(request.bricks.len());
    let mut types: Vec<String> = Vec::with_capacity(request.bricks.len());
    for id in &request.bricks {
        let brick = style_brick::get(conn, id)?.ok_or_else(|| Error::not_found("style", id))?;
        if brick.profile_id != profile_id {
            return Err(Error::refused("asset.styleOtherWorkspace").param("name", brick.name));
        }
        if !composition.types().any(|key| key == brick.type_key) {
            return Err(Error::refused("compose.notOfComposition")
                .param("name", brick.name)
                .param("type", brick.type_key));
        }
        let Some(phrase) = brick
            .description
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
        else {
            return Err(Error::refused("compose.saysNothing").param("name", brick.name));
        };
        // The same phrase picked twice is said once: a generator reads a
        // repeat as weight nobody meant.
        if phrases.iter().any(|p| p == phrase) {
            continue;
        }
        phrases.push(phrase.to_owned());
        types.push(brick.type_key);
    }

    let fields: Vec<String> = composition
        .fields
        .iter()
        .filter_map(|field| field_value(&meta, &field.field).map(|value| field.write(&value)))
        .collect();
    let text = phrases
        .into_iter()
        .chain(fields)
        .collect::<Vec<_>>()
        .join(composition.separator());

    Ok(ComposedText {
        length: text.chars().count(),
        text,
        limit: composition.limit,
        problems: problems(composition, &types),
    })
}

fn problems(composition: &Composition, picked: &[String]) -> Vec<ComposeProblem> {
    composition
        .parts
        .iter()
        .filter_map(|part| {
            let count = picked.iter().filter(|key| **key == part.type_key).count();
            let short = count < usize::from(part.min);
            let over = part.max.is_some_and(|max| count > usize::from(max));
            (short || over).then(|| ComposeProblem {
                type_key: part.type_key.clone(),
                picked: count,
                min: part.min,
                max: part.max,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::style_brick::{NewStyleBrick, create};
    use crate::work::NewWork;

    fn phrase(conn: &Connection, profile_id: &str, type_key: &str, text: &str) -> String {
        create(
            conn,
            profile_id,
            NewStyleBrick {
                type_key: type_key.into(),
                name: text.into(),
                description: Some(text.into()),
                ..Default::default()
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn the_text_is_the_picks_in_their_order_and_then_the_fields() {
        let (conn, profile_id) = fixtures::workspace();
        let vocal = phrase(&conn, &profile_id, "vocal", "breathy whisper vocal");
        let genre = phrase(&conn, &profile_id, "genre", "dream pop");
        let groove = phrase(&conn, &profile_id, "groove", "half-time drums");
        let meta = serde_json::json!({ "bpm": 84, "key": "D minor" });
        let song = crate::work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Paper lanterns".into(),
                meta: meta.as_object().cloned(),
                ..Default::default()
            },
        )
        .unwrap();

        let composed = write(
            &conn,
            &profile_id,
            &ComposeRequest {
                composition: "sound".into(),
                bricks: vec![vocal.clone(), genre, groove, vocal],
                work_id: Some(song.id),
            },
        )
        .unwrap();
        assert_eq!(
            composed.text, "breathy whisper vocal, dream pop, half-time drums, 84 bpm, D minor",
            "the order picked is the order written, a repeat is said once, the fields close it"
        );
        assert_eq!(composed.length, composed.text.chars().count());
        assert_eq!(composed.limit, Some(1000));
        let short: Vec<&str> = composed
            .problems
            .iter()
            .map(|p| p.type_key.as_str())
            .collect();
        assert_eq!(
            short,
            ["instrument", "mood"],
            "a block the composition needs and the picks lack is said"
        );
    }

    #[test]
    fn a_block_picked_past_its_bound_is_said_and_the_text_still_written() {
        let (conn, profile_id) = fixtures::workspace();
        let a = phrase(&conn, &profile_id, "groove", "breakbeat");
        let b = phrase(&conn, &profile_id, "groove", "punchy drums");
        let composed = write(
            &conn,
            &profile_id,
            &ComposeRequest {
                composition: "sound".into(),
                bricks: vec![a, b],
                work_id: None,
            },
        )
        .unwrap();
        assert_eq!(composed.text, "breakbeat, punchy drums");
        let groove = composed
            .problems
            .iter()
            .find(|p| p.type_key == "groove")
            .expect("two grooves where the composition takes one");
        assert_eq!((groove.picked, groove.max), (2, Some(1)));
    }

    #[test]
    fn a_brick_of_another_half_of_the_dictionary_is_refused() {
        let (conn, profile_id) = fixtures::workspace();
        let picture = create(
            &conn,
            &profile_id,
            NewStyleBrick {
                type_key: "image-style".into(),
                name: "Grain".into(),
                description: Some("Grainy film.".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let refused = write(
            &conn,
            &profile_id,
            &ComposeRequest {
                composition: "sound".into(),
                bricks: vec![picture.id],
                work_id: None,
            },
        )
        .unwrap_err();
        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("compose.notOfComposition")
        );
    }

    #[test]
    fn a_draft_with_no_phrase_is_refused_by_name() {
        let (conn, profile_id) = fixtures::workspace();
        let draft = create(
            &conn,
            &profile_id,
            NewStyleBrick {
                type_key: "knob".into(),
                name: "Something loud".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let refused = write(
            &conn,
            &profile_id,
            &ComposeRequest {
                composition: "sound".into(),
                bricks: vec![draft.id],
                work_id: None,
            },
        )
        .unwrap_err();
        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("compose.saysNothing")
        );
    }
}
