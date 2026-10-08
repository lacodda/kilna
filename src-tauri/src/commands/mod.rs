//! The window's side of the backend: one Tauri command per thing the window
//! asks for, grouped by what it is about.
//!
//! A command is an adapter. It takes the workspace's connection and hands the
//! request to a read in the domain or to a gesture in [`crate::actions`] -
//! it does not write a row, record an operation or journal a line itself, and
//! `tests/operation_coverage.rs` holds it to that. Until v0.83 the commands
//! were one file of 4,400 lines in the order they were written, each carrying
//! its own copy of the gesture's ceremony (ADR 0040).

pub mod assets;
pub mod assistant;
pub mod canon;
pub mod collections;
pub mod comments;
pub mod cuts;
pub mod data;
pub mod focus;
pub mod folders;
pub mod ideas;
pub mod journal;
pub mod links;
pub mod notes;
pub mod plugins;
pub mod register;
pub mod releases;
pub mod scenes;
pub mod scores;
pub mod search;
pub mod styles;
pub mod trash;
pub mod versions;
pub mod works;
pub mod workspace;

use crate::error::Result;

/// The id of the active profile: what every read of the workspace is scoped
/// to, so the window never carries a profile id and a request cannot land in
/// the wrong profile.
fn active(conn: &rusqlite::Connection) -> Result<String> {
    crate::actions::active_profile_id(conn)
}
