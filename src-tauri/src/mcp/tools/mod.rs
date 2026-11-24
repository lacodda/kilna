//! The tools, each a function of its own name, filed by what it is about.
//!
//! A tool reads, or it proposes: nothing here writes a version, a score or a
//! note. What a client is told about each is in [`super::schema`].

use rusqlite::Connection;
use serde_json::{Map, Value};

use super::Session;
use super::call::{Call, active};
use crate::error::{Error, Result};
use crate::profile;

mod calendar;
mod canon;
mod conception;
mod cover;
mod lab;
mod media;
mod notes;
mod space;
mod storyboard;
mod styles;
mod words;
mod works;

/// Runs the tool a client named, in the space the call is served from, and
/// gives its answer as the text the client is handed.
pub fn run_tool(
    conn: &Connection,
    session: &Session,
    name: &str,
    args: &Map<String, Value>,
) -> Result<String> {
    let profile = active(conn)?;
    let config = profile::config_for(conn, &profile.id)?;
    let call = Call {
        conn,
        session,
        profile: &profile,
        config: &config,
        args,
    };

    match name {
        "workspace" => space::workspace(&call),
        "guide" => super::skills::guide(&call),
        "catalogue" => works::catalogue(&call),
        "work" => works::work(&call),
        "text" => works::text(&call),
        "scores" => works::scores(&call),
        "calendar" => calendar::calendar(&call),
        "propose_batch_prompts" => calendar::propose_batch_prompts(&call),
        "propose_media" => media::propose_media(&call),
        "quotas" => calendar::quotas(&call),
        "notes" => notes::notes(&call),
        "register" => words::register(&call),
        "scenes" => storyboard::scenes(&call),
        "styles" => styles::styles(&call),
        "propose_style" => styles::propose_style(&call),
        "propose_words" => words::propose_words(&call),
        "search" => space::search(&call),
        "canon" => canon::canon(&call),
        "propose_canon" => canon::propose_canon(&call),
        "propose_version" => works::propose_version(&call),
        "propose_score" => works::propose_score(&call),
        "propose_work" => works::propose_work(&call),
        "cover" => cover::cover(&call),
        "propose_cover" => cover::propose_cover(&call),
        "brief" => conception::brief(&call),
        "propose_cards" => conception::propose_cards(&call),
        "trials" => lab::trials(&call),
        "propose_trials" => lab::propose_trials(&call),
        "propose_scenes" => storyboard::propose_scenes(&call),
        "propose_release" => calendar::propose_release(&call),
        "propose_note" => notes::propose_note(&call),
        other => Err(Error::refused("mcp.unknownTool").param("name", other)),
    }
}
