//! Styles proposed for the dictionary by an agent (v0.134, FR-08 8.1).
//!
//! `propose_style` brings a package of styles of any type - new ones, and
//! rewrites of ones the dictionary has - with their contracts. Read against
//! the dictionary as it stands, shown in the chat, and kept whole or item by
//! item (`style:N`): each kept item is the dictionary's own `style.create` or
//! `style.update`, so undo takes it back and the history says it. A rewrite
//! changes only what it names.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Reason, Result};
use crate::profile::config::{Label, ProfileConfig};
use crate::style_brick::{self, Contract, NewStyleBrick, StyleBrickPatch};

/// One style as it is proposed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct ProposedStyle {
    /// The brick it rewrites, by id. Absent, a new brick.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub id: Option<String>,
    /// A key of the profile's `style_types`: needed for a new brick.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub type_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    /// The words that go into a prompt, in English.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub when: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub explanation: Option<Label>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub colours: Option<Vec<String>>,
    /// When it can go into a picture and what it brings.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub contract: Option<Contract>,
    /// The name the rewritten brick has now, for the chat to say what
    /// changes - filled on reading, never sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub was: Option<String>,
}

/// What was proposed, and what of it was left out on reading.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct StylesPackage {
    pub styles: Vec<ProposedStyle>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dropped: Vec<Reason>,
}

impl StylesPackage {
    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }
}

/// The item key of the `index`-th style, as an apply names it.
pub fn item(index: usize) -> String {
    format!("style:{index}")
}

fn taken(items: Option<&[String]>, index: usize) -> bool {
    items.is_none_or(|items| items.iter().any(|one| *one == item(index)))
}

fn trimmed(text: Option<String>) -> Option<String> {
    text.map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

/// Read a package from what an agent sent - `{"styles": [...]}` or the list
/// itself - against the dictionary as it stands. An item that cannot be kept
/// as it is - a type the profile does not make, a brick that is not there, a
/// new one with no name - is left out and said.
pub fn read(
    conn: &Connection,
    profile_id: &str,
    config: &ProfileConfig,
    value: &Value,
) -> Result<StylesPackage> {
    let list = match value {
        Value::Array(list) => list.clone(),
        Value::Object(object) => match object.get("styles") {
            Some(Value::Array(list)) => list.clone(),
            _ => return Err(Error::refused("styles.noStyles")),
        },
        _ => return Err(Error::refused("styles.noStyles")),
    };
    let mut package = StylesPackage::default();
    for (index, raw) in list.into_iter().enumerate() {
        let at = index + 1;
        let mut raw = raw;
        if let Some(object) = raw.as_object_mut() {
            object.retain(|_, value| !value.is_null());
        }
        let mut style: ProposedStyle = match serde_json::from_value(raw) {
            Ok(style) => style,
            Err(why) => {
                package.dropped.push(
                    Reason::of("refusal.styles.unread")
                        .param("index", at)
                        .param("why", why.to_string()),
                );
                continue;
            }
        };
        style.id = trimmed(style.id);
        style.name = trimmed(style.name);
        style.description = trimmed(style.description);
        style.when = trimmed(style.when);
        style.family = trimmed(style.family);
        style.was = None;

        let type_key = match &style.id {
            Some(id) => {
                let Some(brick) =
                    style_brick::get(conn, id)?.filter(|brick| brick.profile_id == profile_id)
                else {
                    package.dropped.push(
                        Reason::of("refusal.styles.unknownStyle")
                            .param("index", at)
                            .param("id", id.clone()),
                    );
                    continue;
                };
                style.was = Some(brick.name.clone());
                style.type_key.clone().unwrap_or(brick.type_key)
            }
            None => {
                let (Some(type_key), Some(_)) = (&style.type_key, &style.name) else {
                    package
                        .dropped
                        .push(Reason::of("refusal.styles.newNeedsTypeAndName").param("index", at));
                    continue;
                };
                type_key.clone()
            }
        };
        let fits = config
            .style_type(&type_key)
            .is_some_and(|kind| kind.retired.is_none());
        if !fits {
            package.dropped.push(
                Reason::of("refusal.styles.unknownType")
                    .param("index", at)
                    .param("type", type_key),
            );
            continue;
        }
        if let Some(family) = &style.family
            && !config
                .style_type(&type_key)
                .is_some_and(|kind| kind.families.iter().any(|f| f.key == *family))
        {
            // A family the type does not file under is a guess gone wrong,
            // not a reason to lose the style.
            style.family = None;
        }
        package.styles.push(style);
    }
    Ok(package)
}

/// What is wrong with keeping the items named, as the dictionary stands now.
pub fn check(
    conn: &Connection,
    package: &StylesPackage,
    items: Option<&[String]>,
) -> Result<Vec<Reason>> {
    let mut problems = Vec::new();
    for (index, style) in package.styles.iter().enumerate() {
        if !taken(items, index) {
            continue;
        }
        if let Some(id) = &style.id
            && style_brick::get(conn, id)?.is_none()
        {
            problems.push(
                Reason::of("refusal.styles.unknownStyle")
                    .param("index", index + 1)
                    .param("id", id.clone()),
            );
        }
    }
    Ok(problems)
}

/// Keep the items named, or all: each one the dictionary's own gesture. The
/// ids of the bricks written.
pub fn keep(
    conn: &Connection,
    package: StylesPackage,
    items: Option<&[String]>,
) -> Result<Vec<String>> {
    let mut written = Vec::new();
    for (index, style) in package.styles.into_iter().enumerate() {
        if !taken(items, index) {
            continue;
        }
        let made = match style.id {
            Some(id) => crate::actions::style::update(
                conn,
                &id,
                StyleBrickPatch {
                    type_key: style.type_key,
                    name: style.name,
                    description: style.description.map(Some),
                    family: style.family.map(Some),
                    when_to_use: style.when.map(Some),
                    explanation: style.explanation.map(Some),
                    colours: style.colours,
                    contract: style.contract,
                    ..StyleBrickPatch::default()
                },
            )?,
            None => crate::actions::style::create(
                conn,
                NewStyleBrick {
                    type_key: style.type_key.unwrap_or_default(),
                    name: style.name.unwrap_or_default(),
                    description: style.description,
                    family: style.family,
                    when_to_use: style.when,
                    explanation: style.explanation,
                    colours: style.colours,
                    contract: style.contract,
                    ..NewStyleBrick::default()
                },
            )?,
        };
        written.push(made.id);
    }
    Ok(written)
}

/// The package as the chat shows it, in English.
pub fn render(package: &StylesPackage) -> String {
    let mut out = String::new();
    for style in &package.styles {
        let name = style
            .name
            .as_deref()
            .or(style.was.as_deref())
            .unwrap_or_default();
        let head = match &style.was {
            Some(was) if style.name.as_deref().is_some_and(|name| name != was) => {
                format!("**{was}** → **{name}**")
            }
            Some(_) => format!("**{name}** (rewritten)"),
            None => format!("**{name}** ({})", style.type_key.as_deref().unwrap_or("")),
        };
        out.push_str(&format!(
            "- {head}{}\n",
            style
                .description
                .as_deref()
                .map(|text| format!(" — {text}"))
                .unwrap_or_default()
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

    fn package(conn: &Connection, profile_id: &str, value: Value) -> StylesPackage {
        let config = crate::profile::config_for(conn, profile_id).unwrap();
        read(conn, profile_id, &config, &value).unwrap()
    }

    #[test]
    fn a_package_makes_new_styles_and_rewrites_old_ones_by_the_dictionary_s_gestures() {
        let (conn, profile_id) = fixtures::workspace();
        let old = crate::actions::style::create(
            &conn,
            NewStyleBrick {
                type_key: "pose".into(),
                name: "Standing".into(),
                description: Some("standing still".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let read = package(
            &conn,
            &profile_id,
            serde_json::json!({ "styles": [
                { "type": "pose", "name": "Arms crossed", "description": "arms crossed, weight on one leg",
                  "contract": { "needs": ["subject:person", "crop:min:waist"] } },
                { "id": old.id, "description": "standing still, hands in pockets" },
                { "type": "no-such-type", "name": "X" },
                { "description": "a new one with no name" },
                { "id": "nobody", "description": "x" }
            ]}),
        );
        assert_eq!(read.styles.len(), 2);
        assert_eq!(read.styles[1].was.as_deref(), Some("Standing"));
        let codes: Vec<&str> = read.dropped.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(
            codes,
            [
                "refusal.styles.unknownType",
                "refusal.styles.newNeedsTypeAndName",
                "refusal.styles.unknownStyle"
            ]
        );
        assert!(render(&read).contains("**Standing** (rewritten)"));

        let written = keep(&conn, read, None).unwrap();
        assert_eq!(written.len(), 2);
        let made = style_brick::get(&conn, &written[0]).unwrap().unwrap();
        assert_eq!(made.contract.needs, ["subject:person", "crop:min:waist"]);
        let rewritten = style_brick::get(&conn, &old.id).unwrap().unwrap();
        assert_eq!(
            rewritten.name, "Standing",
            "a rewrite changes only what it names"
        );
        assert_eq!(
            rewritten.description.as_deref(),
            Some("standing still, hands in pockets")
        );
        let offer = crate::undo::last(&conn).unwrap().unwrap();
        crate::undo::undo(&conn, &offer.operation_id).unwrap();
        let back = style_brick::get(&conn, &old.id).unwrap().unwrap();
        assert_eq!(back.description.as_deref(), Some("standing still"));
    }

    #[test]
    fn only_the_items_named_are_kept() {
        let (conn, profile_id) = fixtures::workspace();
        let read = package(
            &conn,
            &profile_id,
            serde_json::json!([
                { "type": "pose", "name": "One", "description": "one" },
                { "type": "pose", "name": "Two", "description": "two" }
            ]),
        );
        let written = keep(&conn, read, Some(&[item(1)])).unwrap();
        assert_eq!(written.len(), 1);
        assert_eq!(
            style_brick::get(&conn, &written[0]).unwrap().unwrap().name,
            "Two"
        );
    }
}
