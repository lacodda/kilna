//! The space itself: what it is and what is in it, found by its words.

use serde_json::{Value, json};

use crate::error::Result;
use crate::mcp::call::{Call, pretty, required};
use crate::mcp::skills::guides;
use crate::search;

/// `workspace`: the space served and the craft's vocabulary in it, per kind
/// of work - what every other tool speaks in.
pub(super) fn workspace(call: &Call) -> Result<String> {
    let Call {
        conn,
        session,
        profile,
        config,
        ..
    } = *call;
    let works: i64 = conn.query_row(
        "SELECT count(*) FROM work WHERE profile_id = ?1",
        [&profile.id],
        |row| row.get(0),
    )?;
    // One entry per kind, each with its own vocabulary: the keys an
    // agent scores or proposes in belong to the work's kind.
    let fields: Vec<Value> = config
        .work_meta_fields
        .iter()
        .map(|f| {
            let mut field = json!({
                "key": f.key, "label": f.label, "type": f.field_type,
                "kinds": f.kinds, "options": f.options,
            });
            // A numbered field is given by kilna when a work is made:
            // an agent that proposes a work leaves it out.
            if let Some(first) = &f.numbered_from {
                field["numbered_from"] = json!(first);
            }
            field
        })
        .collect();
    let kinds: Vec<Value> = config
        .work_kinds
        .iter()
        .map(|kind| {
            json!({
                "key": kind.key,
                "label": kind.label,
                "version_roles": kind.version_roles,
                // Whole: the rubric's marks are the anchors a judge
                // places a work against (FR-17).
                "axes": kind.axes,
                "tiers": kind.tiers,
                "statuses": kind.statuses.iter().map(|s| json!({ "key": s.key, "label": s.label })).collect::<Vec<_>>(),
                "release_kinds": kind.release_kinds.iter().map(|k| json!({
                    "key": k.key, "label": k.label, "cover_format": k.cover_format,
                    "fields": k.fields.iter().map(|f| json!({
                        "key": f.key, "label": f.label, "type": f.field_type,
                        "hint": f.hint, "limit": f.limit,
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
                "goes_out": kind.has_doors(),
                "shot_types": kind.shot_types,
                "layers": kind.layers,
                "scene_blocks": kind.scene_blocks,
                "cover": kind.cover,
                "frame": kind.frame,
                "lab": kind.lab,
                "conception": kind.conception,
            })
        })
        .collect();
    let style_types: Vec<Value> = config
        .style_types
        .iter()
        .filter(|style| style.retired.is_none())
        .map(|style| {
            json!({
                "key": style.key, "label": style.label, "form": style.form,
                "hint": style.hint,
                "families": style.families.iter().map(|f| json!({ "key": f.key, "label": f.label })).collect::<Vec<_>>(),
            })
        })
        .collect();
    // The kinds of note a work's material is kept as - a phrase, a
    // note (ADR 0064) - and the cards of the canon apart: those are
    // read through `canon`.
    let note_kinds: Vec<Value> = config
        .note_kinds
        .iter()
        .filter(|kind| !kind.is_card())
        .map(|kind| {
            let mut said = json!({
                "key": kind.key, "label": kind.label,
                "line": kind.line, "material": kind.material,
            });
            if kind.library {
                said["library"] = json!(true);
                said["media"] = json!(kind.media);
            }
            said
        })
        .collect();
    pretty(&json!({
        "space": session.space,
        "profile": { "key": profile.key, "name": profile.name, "description": profile.description },
        "works": works,
        "fields": fields,
        "work_kinds": kinds,
        "note_kinds": note_kinds,
        "style_types": style_types,
        "compose": config.compose,
        "boards": config.boards,
        // The three lines a work's feel is said along (FR-02).
        "vibe": config.vibe(),
        // What the space's shorts look like (v0.168): the blocks of the
        // style and its variants, without their pictures.
        "short_style": super::shorts::style_sheet(call)?,
        // The skills of the profile, without their methods: what
        // `guide` opens one by one (FR-17).
        "guides": guides(config, None, None),
    }))
}

/// `search`: anything by its text, each hit with the work it belongs to.
pub(super) fn search(call: &Call) -> Result<String> {
    let Call {
        conn,
        profile,
        args,
        ..
    } = *call;
    pretty(&search::find(conn, &profile.id, required(args, "query")?)?)
}
