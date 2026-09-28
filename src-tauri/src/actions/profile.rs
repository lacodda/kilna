//! Gestures on the profile: the craft's vocabulary, saved as one document.

use rusqlite::Connection;

use super::gesture_in;
use crate::error::Result;
use crate::profile::{self, Profile, config::ProfileConfig};

/// Replace a profile's configuration.
///
/// Existing works keep their status and kind even when the vocabulary that
/// named them is edited away — an old value stays visible rather than being
/// rewritten, because the alternative is silently changing what a work is.
///
/// Logged as `profile.update` with the whole document before and after: the
/// vocabulary is part of the workspace a replay rebuilds - statuses, axes and
/// kinds are read through it - and it used to be written past the log, so a
/// rebuilt workspace came back with the profile as it was first seeded.
pub fn update_config(conn: &Connection, id: &str, config: &ProfileConfig) -> Result<Profile> {
    gesture_in(conn, id, "profile.update", |act| {
        let before = profile::config_for(act, id)?;
        act.param("id", id);
        act.json("config", config)?;
        act.json("before", &before)?;
        act.stamped();
        profile::update_config_at(act, id, config, act.at())
    })
}
