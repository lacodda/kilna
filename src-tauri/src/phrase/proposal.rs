//! Bricks proposed for the dictionary, kept with one click (v0.94).
//!
//! What "Explain" answers: for each phrase a text says that the dictionary
//! does not know, the brick it would be - its type, the phrase word for word,
//! what it means in every language the window speaks, when to reach for it.
//! Read and checked against the dictionary as it stands, and kept whole or
//! item by item (`brick:N`), the way a package of words is (ADR 0052): each
//! kept item is an ordinary `style.create`, so undo takes it back.

use std::collections::BTreeSet;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::key_of;
use crate::error::{Error, Reason, Result};
use crate::profile::config::{Label, ProfileConfig, StyleForm};
use crate::style_brick::NewStyleBrick;

/// One brick as it is proposed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct ProposedBrick {
    /// A key of the profile's `style_types`, of the form `phrase`.
    #[serde(rename = "type")]
    pub type_key: String,
    /// The phrase, as the texts write it: the brick's name and its words.
    pub phrase: String,
    /// What it is and what it gives, per language.
    pub explanation: Label,
    /// When to reach for it, in English.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub when: Option<String>,
    /// A key of its type's `families`, where the type files its bricks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub family: Option<String>,
}

/// What was proposed, and what of it was left out on reading.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct BricksPackage {
    pub bricks: Vec<ProposedBrick>,
    /// Why an item was not taken into the package: a type the dictionary does
    /// not have, a phrase it already knows.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dropped: Vec<Reason>,
}

impl BricksPackage {
    pub fn is_empty(&self) -> bool {
        self.bricks.is_empty()
    }
}

/// How an answer gives bricks: appended to an action that produces `bricks`.
pub const INSTRUCTION: &str = "\n\nAnswer with one line on what you left out, if anything, then one fenced json block the application reads:\n\n```json\n{\"bricks\": [{\"type\": \"knob\", \"phrase\": \"noise guitar bursts\", \"explanation\": {\"en\": \"Short bursts of guitar noise between the phrases: grit and nerve that break up a smooth mix.\", \"ru\": \"Короткие всплески гитарного шума между фразами: грязь и нервность, рвут гладкость микса.\"}, \"when\": \"When a clean mix needs to feel dangerous.\", \"family\": null}]}\n```\n\n`type` is the key of one of the types given. `phrase` is the phrase exactly as it was given. `explanation` has an `en` and a `ru` sentence. `family` is a key of the type's families where it has them, otherwise null. Leave the list empty when nothing given is a phrase of sound.";

/// The item key of the `index`-th brick, as an apply names it.
pub fn item(index: usize) -> String {
    format!("brick:{index}")
}

fn taken(items: Option<&[String]>, index: usize) -> bool {
    items.is_none_or(|items| items.iter().any(|one| *one == item(index)))
}

/// Read a package from what a proposer wrote - `{"bricks": [...]}` or the
/// list itself - against the types of phrase `types` names (every type of
/// phrase the profile has, when empty) and the dictionary as it stands.
pub fn read(
    conn: &Connection,
    profile_id: &str,
    config: &ProfileConfig,
    types: &[String],
    value: &Value,
) -> Result<BricksPackage> {
    let list = match value {
        Value::Array(list) => list.clone(),
        Value::Object(object) => match object.get("bricks") {
            Some(Value::Array(list)) => list.clone(),
            _ => return Err(Error::refused("bricks.noBricks")),
        },
        _ => return Err(Error::refused("bricks.noBricks")),
    };
    let allowed = |key: &str| {
        config.style_type(key).is_some_and(|style| {
            style.form == StyleForm::Phrase
                && style.retired.is_none()
                && (types.is_empty() || types.iter().any(|t| t == key))
        })
    };
    // What the dictionary already says, so a phrase is not proposed twice -
    // whatever type it stands under: one phrase, one meaning.
    let known: BTreeSet<String> = crate::style_brick::list(conn, profile_id, &Default::default())?
        .into_iter()
        .filter(|brick| {
            config
                .style_type(&brick.type_key)
                .is_some_and(|style| style.form == StyleForm::Phrase)
        })
        .filter_map(|brick| brick.description.map(|text| key_of(&text)))
        .collect();

    let mut package = BricksPackage::default();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for raw in list {
        // `"family": null` reads as no family.
        let mut raw = raw;
        if let Some(object) = raw.as_object_mut() {
            object.retain(|_, value| !value.is_null());
        }
        let mut brick: ProposedBrick = match serde_json::from_value(raw) {
            Ok(brick) => brick,
            Err(why) => {
                package
                    .dropped
                    .push(Reason::of("refusal.bricks.unread").param("why", why.to_string()));
                continue;
            }
        };
        brick.phrase = brick.phrase.trim().to_owned();
        brick.when = brick
            .when
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty());
        let key = key_of(&brick.phrase);
        if key.is_empty() {
            package.dropped.push(Reason::of("refusal.bricks.noPhrase"));
            continue;
        }
        if !allowed(&brick.type_key) {
            package.dropped.push(
                Reason::of("refusal.bricks.unknownType")
                    .param("phrase", brick.phrase.clone())
                    .param("type", brick.type_key.clone()),
            );
            continue;
        }
        if known.contains(&key) || !seen.insert(key) {
            package
                .dropped
                .push(Reason::of("refusal.bricks.known").param("phrase", brick.phrase.clone()));
            continue;
        }
        if let Some(family) = &brick.family
            && !config
                .style_type(&brick.type_key)
                .is_some_and(|style| style.families.iter().any(|f| f.key == *family))
        {
            // A family the type does not file under is a guess gone wrong,
            // not a reason to lose the brick.
            brick.family = None;
        }
        package.bricks.push(brick);
    }
    Ok(package)
}

/// What is wrong with keeping the items named, as the dictionary stands now.
pub fn check(
    conn: &Connection,
    package: &BricksPackage,
    items: Option<&[String]>,
) -> Result<Vec<Reason>> {
    let profile_id = crate::actions::active_profile_id(conn)?;
    let config = crate::profile::config_for(conn, &profile_id)?;
    let mut problems = Vec::new();
    for (index, brick) in package.bricks.iter().enumerate() {
        if !taken(items, index) {
            continue;
        }
        let kept = config
            .style_type(&brick.type_key)
            .is_some_and(|style| style.form == StyleForm::Phrase && style.retired.is_none());
        if !kept {
            problems.push(
                Reason::of("refusal.bricks.unknownType")
                    .param("phrase", brick.phrase.clone())
                    .param("type", brick.type_key.clone()),
            );
        }
    }
    Ok(problems)
}

/// Keep the items named, or all: each one brick, made by the dictionary's own
/// gesture. A phrase the dictionary came to know since is left as it is.
/// The ids of the bricks written.
pub fn keep(
    conn: &Connection,
    package: BricksPackage,
    items: Option<&[String]>,
) -> Result<Vec<String>> {
    let profile_id = crate::actions::active_profile_id(conn)?;
    let config = crate::profile::config_for(conn, &profile_id)?;
    let known: BTreeSet<String> = crate::style_brick::list(conn, &profile_id, &Default::default())?
        .into_iter()
        .filter(|brick| {
            config
                .style_type(&brick.type_key)
                .is_some_and(|style| style.form == StyleForm::Phrase)
        })
        .filter_map(|brick| brick.description.map(|text| key_of(&text)))
        .collect();
    let mut written = Vec::new();
    for (index, brick) in package.bricks.into_iter().enumerate() {
        if !taken(items, index) || known.contains(&key_of(&brick.phrase)) {
            continue;
        }
        let made = crate::actions::style::create(
            conn,
            NewStyleBrick {
                type_key: brick.type_key,
                name: brick.phrase.clone(),
                description: Some(brick.phrase),
                family: brick.family,
                when_to_use: brick.when,
                explanation: Some(brick.explanation),
                ..NewStyleBrick::default()
            },
        )?;
        written.push(made.id);
    }
    Ok(written)
}

/// The package as the chat shows it, in English.
pub fn render(package: &BricksPackage) -> String {
    let mut out = String::new();
    for brick in &package.bricks {
        out.push_str(&format!(
            "- **{}** ({}) — {}\n",
            brick.phrase,
            brick.type_key,
            brick.explanation.as_str()
        ));
    }
    if !package.dropped.is_empty() {
        let said: Vec<String> = package
            .dropped
            .iter()
            .map(|reason| {
                let code = reason.key.strip_prefix("refusal.").unwrap_or(&reason.key);
                crate::error::english(code, &reason.params)
            })
            .collect();
        out.push_str(&format!("\nLeft out: {}.\n", said.join("; ")));
    }
    out.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    fn package(conn: &Connection, profile_id: &str, value: Value) -> BricksPackage {
        let config = crate::profile::config_for(conn, profile_id).unwrap();
        read(conn, profile_id, &config, &[], &value).unwrap()
    }

    #[test]
    fn a_proposal_is_read_against_the_dictionary_as_it_stands() {
        let (conn, profile_id) = fixtures::workspace();
        crate::actions::style::create(
            &conn,
            NewStyleBrick {
                type_key: "groove".into(),
                name: "breakbeat".into(),
                description: Some("breakbeat".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let read = package(
            &conn,
            &profile_id,
            serde_json::json!({ "bricks": [
                { "type": "knob", "phrase": " noise guitar bursts ", "explanation": { "en": "Grit.", "ru": "Грязь." }, "family": null },
                { "type": "groove", "phrase": "Breakbeat", "explanation": { "en": "Breaks.", "ru": "Брейки." } },
                { "type": "image-style", "phrase": "grainy film", "explanation": { "en": "Grain.", "ru": "Зерно." } },
                { "type": "genre", "phrase": "jungle grunge", "explanation": { "en": "Hybrid.", "ru": "Гибрид." }, "family": "no-such-family" },
                { "type": "knob", "phrase": "noise guitar bursts", "explanation": { "en": "Again.", "ru": "Снова." } }
            ]}),
        );
        let phrases: Vec<&str> = read.bricks.iter().map(|b| b.phrase.as_str()).collect();
        assert_eq!(
            phrases,
            ["noise guitar bursts", "jungle grunge"],
            "a known phrase, a picture's type and a repeat are left out"
        );
        assert_eq!(
            read.bricks[1].family, None,
            "a family the type lacks is let go"
        );
        let codes: Vec<&str> = read.dropped.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(
            codes,
            [
                "refusal.bricks.known",
                "refusal.bricks.unknownType",
                "refusal.bricks.known"
            ]
        );
    }

    #[test]
    fn keeping_an_item_makes_a_brick_by_the_dictionarys_own_gesture() {
        let (conn, profile_id) = fixtures::workspace();
        let read = package(
            &conn,
            &profile_id,
            serde_json::json!([
                { "type": "knob", "phrase": "everything clipping", "explanation": { "en": "Pushed into distortion.", "ru": "Всё в перегрузе." }, "when": "When it has to hurt." },
                { "type": "knob", "phrase": "tape-stop drops", "explanation": { "en": "The tape slows to a halt.", "ru": "Лента замедляется до остановки." } }
            ]),
        );
        let written = keep(&conn, read, Some(&[item(0)])).unwrap();
        assert_eq!(written.len(), 1, "only the item named is kept");
        let brick = crate::style_brick::get(&conn, &written[0])
            .unwrap()
            .unwrap();
        assert_eq!(brick.name, "everything clipping");
        assert_eq!(brick.description.as_deref(), Some("everything clipping"));
        assert_eq!(brick.status, crate::style_brick::READY);
        assert_eq!(
            brick.explanation.as_ref().map(|l| l.in_locale("ru")),
            Some("Всё в перегрузе.")
        );
        let logged: i64 = conn
            .query_row(
                "SELECT count(*) FROM operation WHERE kind = 'style.create'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(logged, 1, "a kept brick is a gesture undo can take back");
    }
}
