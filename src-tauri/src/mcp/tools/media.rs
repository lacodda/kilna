//! Files made beside kilna, proposed for the works they belong to.

use serde_json::Value;

use crate::assistant::proposal::{self, Proposal};
use crate::error::{Error, Result};
use crate::mcp::call::{Call, arg, find_work, required};
use crate::mcp::deliver::deliver;

/// `propose_media`: files made beside kilna, each for a work and as what.
pub(super) fn propose_media(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        args,
        ..
    } = *call;
    let raw = args
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::refused("mcp.missingArg").param("key", "items"))?;
    let mut items = Vec::new();
    for one in raw {
        let one = one
            .as_object()
            .ok_or_else(|| Error::refused("mcp.missingArg").param("key", "items"))?;
        let work = find_work(conn, &profile.id, required(one, "work")?)?;
        let path = required(one, "path")?.trim().to_owned();
        let said = required(one, "role")?;
        let role: crate::actions::media::MediaRole =
            serde_json::from_value(Value::String(said.to_owned()))
                .map_err(|_| Error::refused("mcp.badMediaRole").param("role", said))?;
        let file = std::path::Path::new(&path);
        let name = file
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !file.is_absolute() || !file.is_file() {
            return Err(Error::refused("media.notAFile").param("path", path));
        }
        if !role.fits(&name) {
            return Err(Error::refused("media.wrongKind")
                .param("name", name)
                .param("role", said));
        }
        items.push(proposal::MediaItem {
            work_id: work.id,
            work_title: work.title,
            path,
            role,
            rename: one
                .get("rename_to_work")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            reason: arg(one, "reason").map(str::to_owned),
            confidence: one
                .get("confidence")
                .and_then(Value::as_f64)
                .map(|sure| sure.clamp(0.0, 1.0)),
        });
    }
    if items.is_empty() {
        return Err(Error::refused("proposal.nothingTaken"));
    }
    let mut body = String::new();
    for item in &items {
        body.push_str(&format!(
            "- {} -> {} ({})\n",
            item.path,
            item.work_title,
            serde_json::to_value(item.role)?
                .as_str()
                .unwrap_or_default()
        ));
    }
    let count = items.len();
    deliver(
        conn,
        &profile.id,
        session,
        None,
        Proposal::Media { items },
        body.trim_end(),
        None,
    )?;
    Ok(format!(
        "Proposed {count} files to take into their works. They wait in the chat named after you; nothing moves until the person applies it."
    ))
}
