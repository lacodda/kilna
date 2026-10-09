//! What a kept trial may become, checked where it lands (ADR 0061).
//!
//! A version of a work's text and a phrase of the dictionary each remember
//! the trial they were taken from. These are the checks that make the
//! memory mean something: only a kept trial is harvested, a text goes only
//! into the role the lab keeps its trials in, and a phrase only into a type
//! the trial is read against. Called by the version and the brick on their
//! own way in, so every path - the board, a package, a replay - goes through
//! them.

use rusqlite::Connection;

use super::trial::{self, Trial, Verdict};
use crate::error::{Error, Result};
use crate::profile::config::ProfileConfig;

/// The trial named, kept, of the workspace.
fn kept(conn: &Connection, profile_id: Option<&str>, trial_id: &str) -> Result<Trial> {
    let found = trial::get(conn, trial_id)?
        .filter(|found| profile_id.is_none_or(|profile| found.profile_id == profile))
        .ok_or_else(|| Error::not_found("trial", trial_id))?;
    if found.verdict != Some(Verdict::Keep) {
        return Err(Error::refused("trial.notKept"));
    }
    Ok(found)
}

/// Refuse a version of `work_id` in `role` taken from `trial_id` unless the
/// trial was kept, its lab keeps trials in that role, and the work is not
/// the experiment itself.
pub fn check_into(conn: &Connection, trial_id: &str, work_id: &str, role: &str) -> Result<()> {
    let found = kept(conn, None, trial_id)?;
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    if work.profile_id != found.profile_id || work.id == found.work_id {
        return Err(Error::refused("trial.notIntoItself"));
    }
    let experiment = crate::work::get(conn, &found.work_id)?
        .ok_or_else(|| Error::not_found("work", &found.work_id))?;
    let config = crate::profile::config_for(conn, &found.profile_id)?;
    let harvest = config
        .lab(&experiment.kind)
        .and_then(|lab| lab.harvest.as_deref());
    if harvest != Some(role) {
        return Err(Error::refused("trial.wrongRole")
            .param("role", role)
            .param("harvest", harvest.unwrap_or_default()));
    }
    Ok(())
}

/// Refuse a brick of `type_key` cut from `trial_id` unless the trial was
/// kept and the type is one its experiment's trials are read against.
pub fn check_phrase(
    conn: &Connection,
    profile_id: &str,
    config: &ProfileConfig,
    trial_id: &str,
    type_key: &str,
) -> Result<()> {
    let found = kept(conn, Some(profile_id), trial_id)?;
    let experiment = crate::work::get(conn, &found.work_id)?
        .ok_or_else(|| Error::not_found("work", &found.work_id))?;
    let read_against = config
        .trial_composition(&experiment.kind)
        .is_some_and(|composition| composition.types().any(|key| key == type_key));
    if !read_against {
        return Err(Error::refused("trial.notAPhraseType").param("type", type_key));
    }
    Ok(())
}
