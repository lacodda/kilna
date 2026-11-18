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
    /// The brick of the dictionary this phrase already is, when it has no
    /// explanation yet (v0.132, FR-05 5.4): keeping the item writes what it
    /// means, when to take it and its family onto that brick - only where
    /// the brick says nothing - instead of making a second one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fills: Option<String>,
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
    // whatever type it stands under: one phrase, one meaning. A known phrase
    // with nothing said about it is the one exception: its meaning is filled.
    let known = phrase_bricks(conn, profile_id, config)?;

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
        let unexplained = known
            .get(&key)
            .filter(|(_, explained)| !explained)
            .map(|(id, _)| id.clone());
        if (known.contains_key(&key) && unexplained.is_none()) || !seen.insert(key) {
            package
                .dropped
                .push(Reason::of("refusal.bricks.known").param("phrase", brick.phrase.clone()));
            continue;
        }
        brick.fills = unexplained;
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

/// The dictionary's phrases by their key: the brick's id, and whether it
/// says what it means.
fn phrase_bricks(
    conn: &Connection,
    profile_id: &str,
    config: &ProfileConfig,
) -> Result<std::collections::BTreeMap<String, (String, bool)>> {
    Ok(
        crate::style_brick::list(conn, profile_id, &Default::default())?
            .into_iter()
            .filter(|brick| {
                config
                    .style_type(&brick.type_key)
                    .is_some_and(|style| style.form == StyleForm::Phrase)
            })
            .filter_map(|brick| {
                let explained = brick.explanation.is_some();
                brick
                    .description
                    .map(|text| (key_of(&text), (brick.id, explained)))
            })
            .collect(),
    )
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
    let known = phrase_bricks(conn, &profile_id, &config)?;
    let mut written = Vec::new();
    for (index, brick) in package.bricks.into_iter().enumerate() {
        if !taken(items, index) {
            continue;
        }
        if let Some(id) = &brick.fills {
            if let Some(filled) = fill(conn, &config, id, brick.clone())? {
                written.push(filled);
            }
            continue;
        }
        if known.contains_key(&key_of(&brick.phrase)) {
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

/// What a kept item writes onto a brick the dictionary already has: only the
/// fields it says nothing in, as one `style.update` undo takes back. Nothing
/// when the brick is gone or has come to say all of it since.
fn fill(
    conn: &Connection,
    config: &ProfileConfig,
    id: &str,
    brick: ProposedBrick,
) -> Result<Option<String>> {
    let Some(before) = crate::style_brick::get(conn, id)? else {
        return Ok(None);
    };
    let family = brick.family.filter(|family| {
        before.family.is_none()
            && config
                .style_type(&before.type_key)
                .is_some_and(|style| style.families.iter().any(|f| f.key == *family))
    });
    let patch = crate::style_brick::StyleBrickPatch {
        explanation: before
            .explanation
            .is_none()
            .then_some(Some(brick.explanation)),
        when_to_use: brick
            .when
            .filter(|_| before.when_to_use.is_none())
            .map(Some),
        family: family.map(Some),
        ..Default::default()
    };
    if patch.explanation.is_none() && patch.when_to_use.is_none() && patch.family.is_none() {
        return Ok(None);
    }
    Ok(Some(crate::actions::style::update(conn, id, patch)?.id))
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
                explanation: Some(Label::One("Breaks.".into())),
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
    fn a_phrase_the_dictionary_knows_but_never_explained_is_filled_where_it_says_nothing() {
        let (conn, profile_id) = fixtures::workspace();
        let known = crate::actions::style::create(
            &conn,
            NewStyleBrick {
                type_key: "groove".into(),
                name: "breakbeat".into(),
                description: Some("breakbeat".into()),
                when_to_use: Some("My own steer.".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let read = package(
            &conn,
            &profile_id,
            serde_json::json!({ "bricks": [
                { "type": "groove", "phrase": "Breakbeat", "explanation": { "en": "Chopped drums.", "ru": "Нарезанные барабаны." },
                  "when": "When a rock song needs a rave.", "family": "breaks" }
            ]}),
        );
        assert!(read.dropped.is_empty(), "{:?}", read.dropped);
        assert_eq!(read.bricks[0].fills.as_deref(), Some(known.id.as_str()));

        let written = keep(&conn, read, None).unwrap();
        assert_eq!(
            written,
            std::slice::from_ref(&known.id),
            "the same brick, not a second one"
        );
        let bricks = crate::style_brick::list(&conn, &profile_id, &Default::default()).unwrap();
        assert_eq!(bricks.iter().filter(|b| b.name == "breakbeat").count(), 1);
        let filled = crate::style_brick::get(&conn, &known.id).unwrap().unwrap();
        assert_eq!(
            filled.explanation.as_ref().map(|l| l.in_locale("ru")),
            Some("Нарезанные барабаны.")
        );
        assert_eq!(filled.family.as_deref(), Some("breaks"));
        assert_eq!(
            filled.when_to_use.as_deref(),
            Some("My own steer."),
            "what the owner wrote stays"
        );

        let offer = crate::undo::last(&conn).unwrap().unwrap();
        crate::undo::undo(&conn, &offer.operation_id).unwrap();
        let back = crate::style_brick::get(&conn, &known.id).unwrap().unwrap();
        assert!(back.explanation.is_none() && back.family.is_none());

        // Once it says what it means, the phrase is known like any other.
        let again = package(
            &conn,
            &profile_id,
            serde_json::json!({ "bricks": [
                { "type": "groove", "phrase": "breakbeat", "explanation": { "en": "x", "ru": "x" } }
            ]}),
        );
        assert!(again.bricks[0].fills.is_some());
        crate::actions::style::update(
            &conn,
            &known.id,
            crate::style_brick::StyleBrickPatch {
                explanation: Some(Some(Label::One("Mine.".into()))),
                ..Default::default()
            },
        )
        .unwrap();
        let known_now = package(
            &conn,
            &profile_id,
            serde_json::json!({ "bricks": [
                { "type": "groove", "phrase": "breakbeat", "explanation": { "en": "x", "ru": "x" } }
            ]}),
        );
        assert!(known_now.bricks.is_empty());
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
