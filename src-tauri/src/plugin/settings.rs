//! What the window keeps for a plugin: whether it is on, and what was set for it
//! (v0.108, ADR 0075).
//!
//! Only what was changed is written. A plugin nobody touched has no row, is on
//! and reads its own defaults, so a default the plugin changes in its next
//! release reaches everyone who left it alone. Past the operations log: which
//! plugins stand on a machine and what they are told is that machine's business.

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Map, Value};

use super::manifest::{Manifest, Plugin};
use crate::error::{Error, Result};
use crate::time::now;

/// What is kept for one plugin.
#[derive(Debug, Clone, PartialEq)]
pub struct Kept {
    pub enabled: bool,
    pub settings: Map<String, Value>,
}

impl Default for Kept {
    fn default() -> Self {
        Self {
            enabled: true,
            settings: Map::new(),
        }
    }
}

fn parse(enabled: i64, settings: &str) -> Kept {
    Kept {
        enabled: enabled != 0,
        // The schema holds the column to an object; anything else is a row
        // nobody wrote, and reads as nothing set.
        settings: serde_json::from_str(settings).unwrap_or_default(),
    }
}

/// What is kept for `executable`: a plugin nobody touched is on, with nothing set.
pub fn kept(conn: &Connection, executable: &str) -> Result<Kept> {
    Ok(conn
        .query_row(
            "SELECT enabled, settings FROM plugin_settings WHERE executable = ?1",
            params![executable],
            |row| Ok(parse(row.get(0)?, &row.get::<_, String>(1)?)),
        )
        .optional()?
        .unwrap_or_default())
}

/// What is kept for every plugin that has a row.
fn every(conn: &Connection) -> Result<HashMap<String, Kept>> {
    let mut statement =
        conn.prepare("SELECT executable, enabled, settings FROM plugin_settings")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            parse(row.get(1)?, &row.get::<_, String>(2)?),
        ))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Put what is kept beside what was found on this machine.
pub fn fill_in(conn: &Connection, plugins: &mut [Plugin]) -> Result<()> {
    let kept = every(conn)?;
    for plugin in plugins {
        let Some(found) = kept.get(&plugin.executable) else {
            continue;
        };
        plugin.enabled = found.enabled;
        plugin.settings = plugin
            .manifest
            .as_ref()
            .map_or_else(Map::new, |manifest| manifest.standing(&found.settings));
    }
    Ok(())
}

/// Turn a plugin on or off. What was set for it stays.
pub fn set_enabled(conn: &Connection, executable: &str, enabled: bool) -> Result<()> {
    conn.execute(
        "INSERT INTO plugin_settings (executable, enabled, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT (executable) DO UPDATE SET
             enabled = excluded.enabled, updated_at = excluded.updated_at",
        params![executable, i64::from(enabled), now()],
    )?;
    Ok(())
}

/// Set one setting of a plugin, or put it back to the plugin's default with
/// `None`.
///
/// Held to what the manifest declares: a key it does not have and a value the
/// setting does not take are refused here rather than handed to a program that
/// has no reason to expect them.
pub fn set_value(
    conn: &Connection,
    executable: &str,
    manifest: &Manifest,
    key: &str,
    value: Option<Value>,
) -> Result<()> {
    let Some(setting) = manifest.settings.iter().find(|setting| setting.key == key) else {
        return Err(Error::refused("plugin.noSuchSetting")
            .param("plugin", executable)
            .param("key", key));
    };
    if let Some(value) = &value
        && !setting.accepts(value)
    {
        return Err(Error::refused("plugin.badSetting")
            .param("plugin", executable)
            .param("key", setting.label.as_str()));
    }

    let mut kept = kept(conn, executable)?;
    match value {
        Some(value) => {
            kept.settings.insert(key.to_owned(), value);
        }
        None => {
            kept.settings.remove(key);
        }
    }
    conn.execute(
        "INSERT INTO plugin_settings (executable, enabled, settings, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (executable) DO UPDATE SET
             settings = excluded.settings, updated_at = excluded.updated_at",
        params![
            executable,
            i64::from(kept.enabled),
            serde_json::to_string(&kept.settings)?,
            now()
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::fixtures;

    fn a_manifest() -> Manifest {
        serde_json::from_str(
            r#"{
                "protocol_version": 1, "name": "cut", "version": "0.3.0",
                "settings": [
                    {"key": "channel", "label": "Channel", "type": "text", "default": "Tide"},
                    {"key": "crf", "label": "Quality", "type": "number", "min": 0, "max": 51}
                ]
            }"#,
        )
        .unwrap()
    }

    fn found(name: &str) -> Plugin {
        Plugin {
            executable: name.to_owned(),
            path: format!("plugins/{name}.exe"),
            manifest: Some(a_manifest()),
            usable: true,
            reason: None,
            enabled: true,
            settings: Map::new(),
        }
    }

    #[test]
    fn a_plugin_nobody_touched_is_on_with_nothing_set() {
        let (conn, _profile_id) = fixtures::workspace();

        assert_eq!(kept(&conn, "kilna-plugin-cut").unwrap(), Kept::default());

        let mut plugins = [found("kilna-plugin-cut")];
        fill_in(&conn, &mut plugins).unwrap();
        assert!(plugins[0].enabled);
        assert!(plugins[0].settings.is_empty());
    }

    /// Only what was set is kept, so the rest keep following the plugin's own
    /// defaults; taking a value back leaves the others where they were.
    #[test]
    fn a_setting_is_kept_and_taken_back_on_its_own() {
        let (conn, _profile_id) = fixtures::workspace();
        let manifest = a_manifest();

        set_value(&conn, "p", &manifest, "channel", Some(json!("Sea"))).unwrap();
        set_value(&conn, "p", &manifest, "crf", Some(json!(18))).unwrap();
        assert_eq!(kept(&conn, "p").unwrap().settings.len(), 2);

        set_value(&conn, "p", &manifest, "channel", None).unwrap();

        let settings = kept(&conn, "p").unwrap().settings;
        assert_eq!(settings.get("channel"), None);
        assert_eq!(settings.get("crf"), Some(&json!(18)));
        assert_eq!(
            manifest.values(&settings).get("channel"),
            Some(&json!("Tide")),
            "back to what the plugin does by itself"
        );
    }

    /// A key the plugin does not declare and a value the setting does not take
    /// are refused, and leave what was there.
    #[test]
    fn what_the_manifest_does_not_declare_is_refused() {
        let (conn, _profile_id) = fixtures::workspace();
        let manifest = a_manifest();
        set_value(&conn, "p", &manifest, "crf", Some(json!(18))).unwrap();

        let refused = set_value(&conn, "p", &manifest, "nope", Some(json!(1))).unwrap_err();
        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("plugin.noSuchSetting")
        );
        let refused = set_value(&conn, "p", &manifest, "crf", Some(json!(99))).unwrap_err();
        assert_eq!(refused.refusal().map(|r| r.code), Some("plugin.badSetting"));

        let settings = kept(&conn, "p").unwrap().settings;
        assert_eq!(settings.len(), 1);
        assert_eq!(settings.get("crf"), Some(&json!(18)));
    }

    /// Turning a plugin off and on again costs it nothing, and setting a value
    /// on one that is off does not turn it back on.
    #[test]
    fn being_off_and_being_set_are_two_things() {
        let (conn, _profile_id) = fixtures::workspace();
        let manifest = a_manifest();

        set_value(&conn, "p", &manifest, "crf", Some(json!(18))).unwrap();
        set_enabled(&conn, "p", false).unwrap();
        assert!(!kept(&conn, "p").unwrap().enabled);
        assert_eq!(
            kept(&conn, "p").unwrap().settings.get("crf"),
            Some(&json!(18))
        );

        set_value(&conn, "p", &manifest, "channel", Some(json!("Sea"))).unwrap();
        assert!(!kept(&conn, "p").unwrap().enabled, "still off");

        set_enabled(&conn, "p", true).unwrap();
        assert!(kept(&conn, "p").unwrap().enabled);
        assert_eq!(kept(&conn, "p").unwrap().settings.len(), 2);
    }

    #[test]
    fn the_list_reads_what_is_kept_for_each_plugin() {
        let (conn, _profile_id) = fixtures::workspace();
        let manifest = a_manifest();
        set_enabled(&conn, "off", false).unwrap();
        set_value(&conn, "set", &manifest, "crf", Some(json!(18))).unwrap();

        let mut plugins = [found("off"), found("set"), found("untouched")];
        fill_in(&conn, &mut plugins).unwrap();

        assert!(!plugins[0].enabled);
        assert!(plugins[1].enabled);
        assert_eq!(plugins[1].settings.get("crf"), Some(&json!(18)));
        assert!(plugins[2].enabled);
        assert!(plugins[2].settings.is_empty());
    }
}
