//! The style dictionary: the bricks a prompt is built from.

use rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::assistant::proposal::Proposal;
use crate::error::{Error, Result};
use crate::mcp::call::{Call, arg, find_work, pretty};
use crate::mcp::deliver::deliver;

/// `styles`: the dictionary a prompt is built from.
pub(super) fn styles(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        config,
        args,
        ..
    } = *call;
    read_styles(conn, &profile.id, config, args)
}

/// The dictionary as `styles` reads it.
fn read_styles(
    conn: &Connection,
    profile_id: &str,
    config: &crate::profile::config::ProfileConfig,
    args: &Map<String, Value>,
) -> Result<String> {
    let all = crate::style_brick::list(conn, profile_id, &Default::default())?;
    let type_key = arg(args, "type");
    let composition = match arg(args, "composition") {
        Some(key) => Some(
            config
                .composition(key)
                .ok_or_else(|| Error::not_found("composition", key))?,
        ),
        None => None,
    };
    let query = arg(args, "query").map(str::to_lowercase);
    let branch = arg(args, "branch");
    if type_key.is_none() && composition.is_none() && query.is_none() && branch.is_none() {
        let types: Vec<Value> = config
            .style_types
            .iter()
            .map(|style| {
                json!({
                    "type": style.key, "label": style.label, "form": style.form,
                    "branch": config.branch_of(&style.key),
                    "retired": style.retired.is_some(),
                    "bricks": all.iter().filter(|b| b.type_key == style.key).count(),
                })
            })
            .collect();
        return pretty(&json!({ "types": types, "compose": config.compose }));
    }
    let house = crate::phrase::house(conn, profile_id)?;
    let mut rows: Vec<&crate::style_brick::StyleBrick> = all
        .iter()
        .filter(|b| type_key.is_none_or(|key| b.type_key == key))
        .filter(|b| branch.is_none_or(|branch| config.branch_of(&b.type_key) == branch))
        .filter(|b| composition.is_none_or(|c| c.types().any(|key| key == b.type_key)))
        .filter(|b| {
            query.as_ref().is_none_or(|q| {
                b.name.to_lowercase().contains(q)
                    || b.description
                        .as_deref()
                        .is_some_and(|d| d.to_lowercase().contains(q))
                    || b.explanation
                        .as_ref()
                        .is_some_and(|l| l.words().iter().any(|w| w.to_lowercase().contains(q)))
            })
        })
        .collect();
    if let Some(composition) = composition {
        let place = |key: &str| {
            composition
                .types()
                .position(|t| t == key)
                .unwrap_or(usize::MAX)
        };
        rows.sort_by_key(|b| place(&b.type_key));
    }
    let out: Vec<Value> = rows
        .into_iter()
        .map(|b| {
            json!({
                "id": b.id, "type": b.type_key, "name": b.name, "family": b.family,
                "phrase": b.description, "explanation": b.explanation, "when": b.when_to_use,
                "house": house.contains(&b.id), "origin": b.origin, "status": b.status,
                "favorite": b.favorite, "branch": config.branch_of(&b.type_key),
            })
        })
        .collect();
    pretty(&out)
}

/// The bricks an agent named for a composed text, by id or exact name - a
/// phrase is also its name - each of the composition's types. A name two
/// bricks share is refused with their ids, as a title is.
pub(super) fn brick_ids(
    conn: &Connection,
    profile_id: &str,
    composition: &crate::profile::config::Composition,
    named: &[String],
) -> Result<Vec<String>> {
    let bricks: Vec<crate::style_brick::StyleBrick> =
        crate::style_brick::list(conn, profile_id, &Default::default())?
            .into_iter()
            .filter(|brick| composition.types().any(|key| key == brick.type_key))
            .collect();
    let mut ids = Vec::with_capacity(named.len());
    for name in named {
        if let Some(brick) = bricks.iter().find(|b| b.id == *name) {
            ids.push(brick.id.clone());
            continue;
        }
        let key = crate::phrase::key_of(name);
        let matching: Vec<&crate::style_brick::StyleBrick> = bricks
            .iter()
            .filter(|b| {
                crate::phrase::key_of(&b.name) == key
                    || b.description
                        .as_deref()
                        .is_some_and(|d| crate::phrase::key_of(d) == key)
            })
            .collect();
        match matching.as_slice() {
            [one] => ids.push(one.id.clone()),
            [] => return Err(Error::refused("mcp.unknownBrick").param("named", name.clone())),
            many => {
                return Err(Error::refused("mcp.ambiguousBrick")
                    .param("named", name.clone())
                    .param(
                        "ids",
                        many.iter()
                            .map(|b| b.id.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    ));
            }
        }
    }
    Ok(ids)
}

/// `propose_style`: new bricks for the dictionary and rewrites of those it
/// has.
pub(super) fn propose_style(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        config,
        args,
    } = *call;
    let found = match arg(args, "work") {
        Some(named) => Some(find_work(conn, &profile.id, named)?),
        None => None,
    };
    let raw = args
        .get("styles")
        .cloned()
        .ok_or_else(|| Error::refused("mcp.missingArg").param("key", "styles"))?;
    let package = crate::style_proposal::read(conn, &profile.id, config, &raw)?;
    if package.is_empty() {
        return Err(Error::refused("styles.noStyles"));
    }
    let body = crate::style_proposal::render(&package);
    let count = package.styles.len();
    let dropped = package.dropped.len();
    deliver(
        conn,
        &profile.id,
        session,
        found.as_ref(),
        Proposal::Styles { package },
        &body,
        arg(args, "note"),
    )?;
    let mut answer = format!(
        "Proposed {count} style{} for the dictionary. It waits in the chat {}; the person keeps it whole or style by style.",
        if count == 1 { "" } else { "s" },
        if found.is_some() {
            "on the work"
        } else {
            "named after you"
        }
    );
    if dropped > 0 {
        answer.push_str(&format!(" {dropped} left out - the chat says why."));
    }
    Ok(answer)
}
