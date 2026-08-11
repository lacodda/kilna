use serde::{Deserialize, Serialize};

/// The protocol version kilna speaks. A plugin declaring anything else is
/// listed but refused, with the mismatch shown — silently ignoring it would
/// look like the plugin was not installed.
pub const PROTOCOL_VERSION: u32 = 1;

/// What a plugin says about itself when run with `--manifest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub protocol_version: u32,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Actions the plugin offers. Each becomes a button where its point of
    /// extension is drawn.
    #[serde(default)]
    pub commands: Vec<Command>,
}

/// One action a plugin can perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Where in kilna this command belongs.
    pub target: Target,
}

/// The points a plugin may extend.
///
/// Deliberately few: every target is a place the core promises to keep calling,
/// so each one added is a commitment. More can be added; none can be quietly
/// removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    /// Offered on a release. Returns metadata merged into `release.meta`.
    Release,
    /// Offered on a work. Returns metadata merged into `work.meta`.
    Work,
}

/// A plugin found on this machine, whether or not it can be used.
#[derive(Debug, Clone, Serialize)]
pub struct Plugin {
    /// Executable name, e.g. `kilna-plugin-youtube`.
    pub executable: String,
    pub path: String,
    /// `None` when the manifest could not be read; `reason` says why.
    pub manifest: Option<Manifest>,
    pub usable: bool,
    pub reason: Option<String>,
}

/// Name a plugin executable must have to be discovered.
pub const PREFIX: &str = "kilna-plugin-";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_parses_with_only_its_required_fields() {
        let raw = r#"{"protocol_version":1,"name":"demo","version":"0.1.0"}"#;

        let manifest: Manifest = serde_json::from_str(raw).unwrap();

        assert_eq!(manifest.protocol_version, 1);
        assert!(manifest.commands.is_empty());
    }

    #[test]
    fn commands_carry_their_target() {
        let raw = r#"{
            "protocol_version": 1, "name": "demo", "version": "0.1.0",
            "commands": [{"key": "fetch", "label": "Fetch stats", "target": "release"}]
        }"#;

        let manifest: Manifest = serde_json::from_str(raw).unwrap();

        assert_eq!(manifest.commands[0].target, Target::Release);
    }

    #[test]
    fn an_unknown_target_is_refused_rather_than_guessed() {
        let raw = r#"{
            "protocol_version": 1, "name": "demo", "version": "0.1.0",
            "commands": [{"key": "x", "label": "X", "target": "somewhere_else"}]
        }"#;

        assert!(serde_json::from_str::<Manifest>(raw).is_err());
    }
}
