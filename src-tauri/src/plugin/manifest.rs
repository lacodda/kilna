use serde::{Deserialize, Serialize};

/// The protocol version kilna speaks. A plugin declaring anything else is
/// listed but refused, with the mismatch shown — silently ignoring it would
/// look like the plugin was not installed.
///
/// Still 1 after v0.101 added a command's time allowance, the `cuts` target,
/// the files a reply names and progress lines on stderr: every one of them is
/// optional on both sides, so a plugin written for the first protocol runs
/// unchanged and a new one runs on a kilna that ignores what it adds.
pub const PROTOCOL_VERSION: u32 = 1;

/// What a plugin says about itself when run with `--manifest`.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
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
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Command {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Where in kilna this command belongs.
    pub target: Target,
    /// How many seconds the command may run before kilna stops it. Absent is
    /// [`DEFAULT_TIMEOUT_SECONDS`] - enough for a lookup or a rename; a
    /// command that renders a video says what it needs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub timeout: Option<u64>,
}

/// What a command may take when its manifest names no allowance: two minutes,
/// generous for a word count, a lookup, a rename.
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 120;

/// The longest allowance a manifest may claim: an hour. A plugin is someone
/// else's program, and one that says it needs a day has made a typo or means
/// to be left alone; it can still be stopped by hand before then.
pub const LONGEST_TIMEOUT_SECONDS: u64 = 3600;

impl Command {
    /// How long this command may run.
    pub fn allowance(&self) -> std::time::Duration {
        let seconds = self
            .timeout
            .filter(|seconds| *seconds > 0)
            .unwrap_or(DEFAULT_TIMEOUT_SECONDS)
            .min(LONGEST_TIMEOUT_SECONDS);
        std::time::Duration::from_secs(seconds)
    }
}

/// The points a plugin may extend.
///
/// Deliberately few: every target is a place the core promises to keep calling,
/// so each one added is a commitment. More can be added; none can be quietly
/// removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    /// Offered on a release. Returns metadata merged into `release.meta`.
    Release,
    /// Offered on a work. Returns metadata merged into `work.meta`.
    Work,
    /// Offered on a work's Cuts tab, beside the stretches it is spliced from
    /// (v0.101): a cutter. Handed what a work is handed - the shot list, the
    /// folders, the finished files - and may name the files it made.
    Cuts,
}

impl Target {
    /// Whether the command acts on a work, whichever tab offers it.
    pub fn is_work(self) -> bool {
        matches!(self, Self::Work | Self::Cuts)
    }
}

/// A plugin found on this machine, whether or not it can be used.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
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

    /// A cutter names the Cuts tab and the time it needs; a command that
    /// names none has the default, and one that claims a day has an hour.
    #[test]
    fn a_command_carries_its_allowance() {
        let raw = r#"{
            "protocol_version": 1, "name": "cut", "version": "0.1.0",
            "commands": [
                {"key": "cut", "label": "Cut", "target": "cuts", "timeout": 900},
                {"key": "count", "label": "Count", "target": "work"},
                {"key": "slow", "label": "Slow", "target": "work", "timeout": 86400}
            ]
        }"#;

        let manifest: Manifest = serde_json::from_str(raw).unwrap();

        assert_eq!(manifest.commands[0].target, Target::Cuts);
        assert_eq!(manifest.commands[0].allowance().as_secs(), 900);
        assert_eq!(
            manifest.commands[1].allowance().as_secs(),
            DEFAULT_TIMEOUT_SECONDS
        );
        assert_eq!(
            manifest.commands[2].allowance().as_secs(),
            LONGEST_TIMEOUT_SECONDS
        );
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
