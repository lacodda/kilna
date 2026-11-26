//! Shorts: what a space's look like, and which variant one wears.

use serde_json::Value;

use crate::assistant::proposal::Proposal;
use crate::error::{Error, Result};
use crate::mcp::call::{Call, arg, find_work};
use crate::mcp::deliver::deliver;

/// `propose_short`: the style of the space's shorts, the variant a short
/// wears, or both.
pub(super) fn propose_short(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        config,
        args,
    } = *call;
    let current = config.short_style();
    let style = match args.get("style").filter(|raw| !raw.is_null()) {
        Some(raw) => Some(crate::short::merged(&current, raw)?),
        None => None,
    };
    let found = match arg(args, "work") {
        Some(named) => Some(find_work(conn, &profile.id, named)?),
        None => None,
    };
    let variant = arg(args, "variant");
    if style.is_none() && variant.is_none() {
        return Err(Error::refused("short.nothingProposed"));
    }
    let worn = match (variant, &found) {
        (None, _) => None,
        (Some(_), None) => return Err(Error::refused("short.variantNeedsWork")),
        (Some(key), Some(short)) => {
            if !config.vocabulary(&short.kind).excerpt {
                return Err(Error::refused("short.notAShort").param("title", short.title.clone()));
            }
            let known = style.as_ref().unwrap_or(&current);
            let Some(named) = known.variant(key) else {
                return Err(Error::refused("short.unknownVariant")
                    .param("variant", key)
                    .param(
                        "known",
                        known
                            .variants
                            .iter()
                            .map(|v| v.key.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    ));
            };
            Some((short, named.key.clone(), named.name().to_owned()))
        }
    };

    let mut body = String::new();
    if let Some(style) = &style {
        body.push_str(&crate::short::render(style));
    }
    if let Some((short, key, name)) = &worn {
        if !body.is_empty() {
            body.push_str("\n\n");
        }
        body.push_str(&format!("“{}” wears **{name}** (`{key}`).", short.title));
    }
    let count = style.as_ref().map_or(0, |style| style.variants.len());
    let proposal = Proposal::Short {
        style: style.map(Box::new),
        work_id: worn.as_ref().map(|(short, _, _)| short.id.clone()),
        variant: worn.as_ref().map(|(_, key, _)| key.clone()),
    };
    let styled = matches!(&proposal, Proposal::Short { style: Some(_), .. });
    deliver(
        conn,
        &profile.id,
        session,
        found.as_ref(),
        proposal,
        &body,
        arg(args, "note"),
    )?;
    let mut said: Vec<String> = Vec::new();
    if styled {
        said.push(format!(
            "Proposed the short style of the space, with {count} variant{}.",
            if count == 1 { "" } else { "s" }
        ));
    }
    if let Some((short, key, _)) = &worn {
        said.push(format!(
            "Proposed the variant `{key}` for “{}”.",
            short.title
        ));
    }
    said.push(format!(
        "It waits in the chat {}; one click takes it. The build itself is started by the person, on the short's Build tab.",
        if found.is_some() {
            "on the work"
        } else {
            "named after you"
        }
    ));
    Ok(said.join(" "))
}

/// The style of the space's shorts as an agent reads it, when its craft cuts
/// shorts at all.
pub(super) fn style_sheet(call: &Call) -> Result<Option<Value>> {
    if !call.config.work_kinds.iter().any(|kind| kind.excerpt) {
        return Ok(None);
    }
    crate::short::readable(&call.config.short_style()).map(Some)
}
