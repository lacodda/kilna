use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The protocol version kilna speaks. A plugin declaring anything else is
/// listed but refused, with the mismatch shown — silently ignoring it would
/// look like the plugin was not installed.
///
/// Still 1 after v0.101 added a command's time allowance, the `cuts` target,
/// the files a reply names and progress lines on stderr, and after v0.108 added
/// the `score` target, settings and words by language: every one of them is
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
    /// The description in other languages, by language code.
    #[serde(default)]
    pub descriptions: BTreeMap<String, String>,
    /// Actions the plugin offers. Each becomes a button where its point of
    /// extension is drawn.
    #[serde(default)]
    pub commands: Vec<Command>,
    /// What the person can set for it (v0.108). The window draws them, keeps
    /// what was set, and hands every run the values (ADR 0075).
    #[serde(default)]
    pub settings: Vec<Setting>,
}

impl Manifest {
    /// What is wrong with a manifest that parsed: the mistakes the shape alone
    /// cannot hold. Said once, in the plugin's place on the settings screen,
    /// rather than as a form that cannot be filled in.
    pub fn fault(&self) -> Option<String> {
        let mut commands = BTreeSet::new();
        if let Some(command) = self
            .commands
            .iter()
            .find(|command| !commands.insert(command.key.as_str()))
        {
            return Some(format!("it names the command “{}” twice", command.key));
        }
        let mut settings = BTreeSet::new();
        for setting in &self.settings {
            if !settings.insert(setting.key.as_str()) {
                return Some(format!("it names the setting “{}” twice", setting.key));
            }
            if let Control::Choice { options } = &setting.control
                && options.is_empty()
            {
                return Some(format!(
                    "its setting “{}” offers nothing to choose",
                    setting.key
                ));
            }
            if let Some(default) = &setting.default
                && !setting.accepts(default)
            {
                return Some(format!(
                    "the default of its setting “{}” is not one the setting takes",
                    setting.key
                ));
            }
        }
        None
    }

    /// What a run is told: for each setting declared, what was set in the
    /// window while it is still a value the setting takes, or else the plugin's
    /// own default. A setting with neither is left out.
    pub fn values(&self, kept: &Map<String, Value>) -> Map<String, Value> {
        self.settings
            .iter()
            .filter_map(|setting| {
                kept.get(&setting.key)
                    .filter(|value| setting.accepts(value))
                    .or(setting.default.as_ref())
                    .map(|value| (setting.key.clone(), value.clone()))
            })
            .collect()
    }

    /// What was set in the window that still means something: a key the plugin
    /// no longer declares, or a value it no longer takes, is kept but not read.
    pub fn standing(&self, kept: &Map<String, Value>) -> Map<String, Value> {
        self.settings
            .iter()
            .filter_map(|setting| {
                kept.get(&setting.key)
                    .filter(|value| setting.accepts(value))
                    .map(|value| (setting.key.clone(), value.clone()))
            })
            .collect()
    }
}

/// One thing the person can set for a plugin.
///
/// Not for a secret: settings are kept in the workspace, so they go into every
/// backup and are handed over in the clear. A plugin with a token asks the
/// system's keychain for it.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Setting {
    pub key: String,
    pub label: String,
    /// The label in other languages, by language code.
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    /// The description in other languages, by language code.
    #[serde(default)]
    pub descriptions: BTreeMap<String, String>,
    /// What the plugin does when nothing is set. The form shows it, so what is
    /// on the screen is what a run does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub default: Option<Value>,
    #[serde(flatten)]
    pub control: Control,
}

impl Setting {
    /// Whether `value` is one this setting can hold.
    pub fn accepts(&self, value: &Value) -> bool {
        match &self.control {
            Control::Text => value.is_string(),
            Control::Boolean => value.is_boolean(),
            Control::Number { min, max, step } => value.as_f64().is_some_and(|number| {
                number.is_finite()
                    && min.is_none_or(|min| number >= min)
                    && max.is_none_or(|max| number <= max)
                    && step.is_none_or(|step| on_step(number, min.unwrap_or(0.0), step))
            }),
            Control::Choice { options } => value
                .as_str()
                .is_some_and(|chosen| options.iter().any(|option| option.value == chosen)),
        }
    }
}

/// Whether `number` is a whole count of `step`s from `base`: 18.5 is no value
/// of a setting that goes by ones (wish 6815). A step of nothing or less says
/// nothing about the value.
fn on_step(number: f64, base: f64, step: f64) -> bool {
    if !(step.is_finite() && step > 0.0) {
        return true;
    }
    let steps = (number - base) / step;
    (steps - steps.round()).abs() < 1e-9 * steps.abs().max(1.0)
}

/// How a setting is answered. An unknown type is a manifest that cannot be
/// read, and the plugin is listed with the reason.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Control {
    /// A line of text.
    Text,
    /// A number, within bounds when it has any.
    Number {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        min: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        max: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        step: Option<f64>,
    },
    /// Yes or no.
    Boolean,
    /// One of a list.
    Choice { options: Vec<SettingOption> },
}

/// One answer a `choice` setting offers.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct SettingOption {
    pub value: String,
    pub label: String,
    /// The label in other languages, by language code.
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}

/// One action a plugin can perform.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Command {
    pub key: String,
    pub label: String,
    /// The label in other languages, by language code: `{"ru": "..."}`. The
    /// window takes its own language's and falls back to `label`.
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub description: Option<String>,
    /// The description in other languages, by language code.
    #[serde(default)]
    pub descriptions: BTreeMap<String, String>,
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
    /// Offered on a work's Score tab (v0.108): a judge. Handed what a work is
    /// handed and the axes to judge along; what it returns as `score` is a
    /// proposal, which the person keeps or does not.
    Score,
}

impl Target {
    /// Whether the command acts on a work, whichever tab offers it.
    pub fn is_work(self) -> bool {
        matches!(self, Self::Work | Self::Cuts | Self::Score)
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
    /// Whether the person left it on: one turned off offers nothing and cannot
    /// be run, and stays listed so it can be turned back on.
    pub enabled: bool,
    /// What was set for it in the window that still means something. A setting
    /// not named here reads the manifest's default.
    pub settings: Map<String, Value>,
}

/// Name a plugin executable must have to be discovered.
pub const PREFIX: &str = "kilna-plugin-";

#[cfg(test)]
mod tests {
    use serde_json::json;

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

    /// Every kind of setting, each in the fewest words it can be written in.
    fn a_manifest_with_settings() -> Manifest {
        serde_json::from_str(
            r#"{
                "protocol_version": 1, "name": "cut", "version": "0.3.0",
                "settings": [
                    {"key": "channel", "label": "Channel", "type": "text", "default": "Tide",
                     "labels": {"ru": "Канал"}},
                    {"key": "crf", "label": "Quality", "type": "number", "min": 0, "max": 51, "default": 20},
                    {"key": "frames", "label": "Frames", "type": "number", "min": 1, "step": 1},
                    {"key": "gain", "label": "Gain", "type": "number", "min": -3, "step": 0.5},
                    {"key": "hand", "label": "Hand", "type": "boolean", "default": true},
                    {"key": "preset", "label": "Preset", "type": "choice",
                     "options": [{"value": "fast", "label": "Fast"}, {"value": "slow", "label": "Slow"}]}
                ]
            }"#,
        )
        .unwrap()
    }

    /// A value is held to its setting: the wrong type, a number past its bound
    /// and an answer not on the list are not values it can hold.
    #[test]
    fn a_setting_takes_only_what_it_says_it_takes() {
        let manifest = a_manifest_with_settings();
        let setting = |key: &str| manifest.settings.iter().find(|s| s.key == key).unwrap();

        assert!(setting("channel").accepts(&json!("Sea")));
        assert!(!setting("channel").accepts(&json!(3)));
        assert!(setting("crf").accepts(&json!(0)));
        assert!(setting("crf").accepts(&json!(51)));
        assert!(!setting("crf").accepts(&json!(52)));
        assert!(!setting("crf").accepts(&json!(-1)));
        assert!(!setting("crf").accepts(&json!("20")));
        assert!(setting("crf").accepts(&json!(18.5)), "no step: any number");
        assert!(setting("frames").accepts(&json!(18)));
        assert!(!setting("frames").accepts(&json!(18.5)), "off its step");
        assert!(setting("gain").accepts(&json!(-1.5)));
        assert!(setting("gain").accepts(&json!(2)));
        assert!(!setting("gain").accepts(&json!(0.25)));
        assert!(setting("hand").accepts(&json!(false)));
        assert!(!setting("hand").accepts(&json!("yes")));
        assert!(setting("preset").accepts(&json!("slow")));
        assert!(!setting("preset").accepts(&json!("medium")));
    }

    /// A run reads what was set, or the plugin's default; a key no longer
    /// declared and a value no longer taken are not handed over.
    #[test]
    fn a_run_is_told_what_was_set_and_the_defaults_of_the_rest() {
        let manifest = a_manifest_with_settings();
        let kept: Map<String, Value> = serde_json::from_value(json!({
            "channel": "Sea",
            "crf": 99,
            "gone": "a setting of an older version",
        }))
        .unwrap();

        let values = manifest.values(&kept);

        assert_eq!(values.get("channel"), Some(&json!("Sea")), "what was set");
        assert_eq!(
            values.get("crf"),
            Some(&json!(20)),
            "out of bounds now: the default"
        );
        assert_eq!(values.get("hand"), Some(&json!(true)), "the default");
        assert_eq!(values.get("preset"), None, "no value and no default");
        assert_eq!(values.get("gone"), None, "no longer declared");

        let standing = manifest.standing(&kept);
        assert_eq!(standing.len(), 1, "{standing:?}");
        assert_eq!(standing.get("channel"), Some(&json!("Sea")));
    }

    /// The mistakes only the author can make are said in words.
    #[test]
    fn a_manifest_at_odds_with_itself_says_where() {
        assert_eq!(a_manifest_with_settings().fault(), None);

        let fault = |raw: &str| {
            serde_json::from_str::<Manifest>(raw)
                .unwrap()
                .fault()
                .expect("a fault to name")
        };
        assert!(
            fault(
                r#"{"protocol_version":1,"name":"x","version":"1","commands":[
                {"key":"a","label":"A","target":"work"},
                {"key":"a","label":"B","target":"work"}]}"#
            )
            .contains("“a” twice")
        );
        assert!(
            fault(
                r#"{"protocol_version":1,"name":"x","version":"1","settings":[
                {"key":"a","label":"A","type":"text"},
                {"key":"a","label":"B","type":"boolean"}]}"#
            )
            .contains("“a” twice")
        );
        assert!(
            fault(
                r#"{"protocol_version":1,"name":"x","version":"1","settings":[
                {"key":"a","label":"A","type":"choice","options":[]}]}"#
            )
            .contains("nothing to choose")
        );
        assert!(
            fault(
                r#"{"protocol_version":1,"name":"x","version":"1","settings":[
                {"key":"a","label":"A","type":"number","max":5,"default":9}]}"#
            )
            .contains("default")
        );
    }

    #[test]
    fn a_setting_of_an_unknown_type_is_refused_rather_than_guessed() {
        let raw = r#"{
            "protocol_version": 1, "name": "demo", "version": "0.1.0",
            "settings": [{"key": "x", "label": "X", "type": "colour"}]
        }"#;

        assert!(serde_json::from_str::<Manifest>(raw).is_err());
    }

    /// A judge names the Score tab, and acts on a work like any other command.
    #[test]
    fn a_command_can_stand_on_the_score_tab() {
        let raw = r#"{
            "protocol_version": 1, "name": "judge", "version": "0.1.0",
            "commands": [{"key": "judge", "label": "Judge", "target": "score",
                          "labels": {"ru": "Оценить"}}]
        }"#;

        let manifest: Manifest = serde_json::from_str(raw).unwrap();

        assert_eq!(manifest.commands[0].target, Target::Score);
        assert!(manifest.commands[0].target.is_work());
        assert_eq!(manifest.commands[0].labels["ru"], "Оценить");
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
