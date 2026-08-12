pub mod manifest;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::error::{Error, Result};
use manifest::{Manifest, PREFIX, PROTOCOL_VERSION, Plugin, Target};

/// Where kilna looks for plugins, in order.
///
/// Following the line's convention: an external executable named by prefix,
/// found on the PATH or in the workspace's own plugins directory. No dynamic
/// libraries — Rust has no stable ABI — and no sandboxed runtime, because the
/// integrations plugins exist for are exactly the ones a sandbox forbids.
pub fn search_paths(app_data_dir: &Path) -> Vec<PathBuf> {
    let mut paths = vec![app_data_dir.join("plugins")];

    if let Some(system) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&system));
    }

    paths
}

/// Everything installed, usable or not.
///
/// A plugin that fails to describe itself is still listed, with the reason —
/// an integration that silently does not appear is indistinguishable from one
/// that was never installed.
pub fn discover(app_data_dir: &Path) -> Vec<Plugin> {
    let mut found: Vec<Plugin> = Vec::new();

    for directory in search_paths(app_data_dir) {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };

        for entry in entries.filter_map(std::result::Result::ok) {
            let path = entry.path();
            let Some(name) = executable_name(&path) else {
                continue;
            };
            if !name.starts_with(PREFIX) {
                continue;
            }
            // The first copy on the path wins, as with any other command.
            if found.iter().any(|plugin| plugin.executable == name) {
                continue;
            }

            found.push(describe(&path, &name));
        }
    }

    found.sort_by(|a, b| a.executable.cmp(&b.executable));
    found
}

/// Ask a plugin what it is.
fn describe(path: &Path, name: &str) -> Plugin {
    let base = Plugin {
        executable: name.to_owned(),
        path: path.display().to_string(),
        manifest: None,
        usable: false,
        reason: None,
    };

    let output = match command(path).arg("--manifest").output() {
        Ok(output) => output,
        Err(error) => {
            return Plugin {
                reason: Some(format!("could not run it: {error}")),
                ..base
            };
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Plugin {
            reason: Some(if stderr.trim().is_empty() {
                "it refused to describe itself".into()
            } else {
                stderr.trim().to_owned()
            }),
            ..base
        };
    }

    let manifest: Manifest = match serde_json::from_slice(&output.stdout) {
        Ok(manifest) => manifest,
        Err(error) => {
            return Plugin {
                reason: Some(format!("its manifest could not be read: {error}")),
                ..base
            };
        }
    };

    if manifest.protocol_version != PROTOCOL_VERSION {
        return Plugin {
            reason: Some(format!(
                "it speaks protocol {} and this build speaks {PROTOCOL_VERSION}",
                manifest.protocol_version
            )),
            manifest: Some(manifest),
            ..base
        };
    }

    Plugin {
        manifest: Some(manifest),
        usable: true,
        ..base
    }
}

/// What a plugin is given when a command runs.
#[derive(Debug, Clone, Serialize)]
pub struct Invocation<'a> {
    pub command: &'a str,
    pub target: Target,
    /// The row the command was invoked on, as the frontend sees it.
    pub subject: Value,
}

/// What a plugin may send back.
#[derive(Debug, Clone, Deserialize)]
pub struct Outcome {
    /// Fields to merge into the subject's `meta`.
    #[serde(default)]
    pub meta: Map<String, Value>,
    /// A line to show the user.
    #[serde(default)]
    pub message: Option<String>,
    /// Set by a plugin that wants to report failure without a non-zero exit.
    #[serde(default)]
    pub error: Option<String>,
}

/// Run one command of one plugin.
///
/// The invocation goes in over stdin as JSON and the outcome comes back over
/// stdout, so a plugin can be written in any language. Plugins live for the
/// duration of a call; nothing here starts a service.
pub fn invoke(path: &Path, invocation: &Invocation<'_>) -> Result<Outcome> {
    let payload = serde_json::to_vec(invocation)?;

    let mut child = command(path)
        .arg("run")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| Error::Other(format!("could not run the plugin: {error}")))?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Other("could not write to the plugin".into()))?;
        stdin
            .write_all(&payload)
            .map_err(|error| Error::Other(format!("could not send the request: {error}")))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| Error::Other(format!("the plugin did not finish: {error}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Other(if stderr.trim().is_empty() {
            format!("the plugin failed with {}", output.status)
        } else {
            stderr.trim().to_owned()
        }));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        // Silence is success with nothing to say.
        return Ok(Outcome {
            meta: Map::new(),
            message: None,
            error: None,
        });
    }

    let outcome: Outcome = serde_json::from_str(stdout.trim()).map_err(|error| {
        Error::Other(format!(
            "could not read the plugin's reply ({error}): {}",
            stdout.trim()
        ))
    })?;

    if let Some(message) = outcome.error {
        return Err(Error::Other(message));
    }

    Ok(outcome)
}

/// Name a plugin is known by: the file name without its extension.
fn executable_name(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }

    let name = path.file_stem()?.to_str()?;

    // On Windows only these are runnable; without the check, a stray
    // `kilna-plugin-notes.txt` would be treated as a program.
    #[cfg(windows)]
    {
        let runnable = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "exe" | "cmd" | "bat"))
            .unwrap_or(false);
        if !runnable {
            return None;
        }
    }

    Some(name.to_owned())
}

fn command(path: &Path) -> Command {
    // Only the Windows branch below mutates it.
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut command = Command::new(path);

    // Otherwise a console window flashes on every call on Windows.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
}

/// Merge a plugin's returned fields into an existing `meta` object.
///
/// A plugin adds and overwrites its own keys; it cannot clear the rest. Losing
/// unrelated metadata to a third-party integration is not a recoverable
/// mistake.
pub fn merge_meta(
    existing: &Map<String, Value>,
    returned: &Map<String, Value>,
) -> Map<String, Value> {
    let mut merged = existing.clone();
    for (key, value) in returned {
        merged.insert(key.clone(), value.clone());
    }
    merged
}

/// A plugin's own settings, stored per workspace under its executable name.
pub fn settings_key(executable: &str) -> String {
    format!("plugin:{executable}")
}

/// Description of the plugin system for the UI when nothing is installed.
pub fn convention() -> Value {
    json!({
        "prefix": PREFIX,
        "protocol_version": PROTOCOL_VERSION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_starts_with_the_workspace_plugins_directory() {
        let paths = search_paths(Path::new("C:/workspace"));

        assert_eq!(paths[0], Path::new("C:/workspace/plugins"));
    }

    /// Write a runnable stand-in for a plugin into `dir/plugins`.
    ///
    /// A batch file on Windows, a shell script elsewhere: the point of the
    /// convention is that a plugin is any executable, not a Rust artefact.
    fn fake_plugin(dir: &Path, name: &str, manifest: &str) -> PathBuf {
        let plugins = dir.join("plugins");
        std::fs::create_dir_all(&plugins).unwrap();

        #[cfg(windows)]
        {
            // The manifest is written beside the script and printed from there:
            // cmd's `echo` mangles quotes, and this is about testing kilna, not
            // batch escaping.
            let manifest_path = plugins.join(format!("{name}.manifest.json"));
            std::fs::write(&manifest_path, manifest).unwrap();

            let outcome_path = plugins.join(format!("{name}.outcome.json"));
            std::fs::write(&outcome_path, r#"{"message":"ran"}"#).unwrap();

            let path = plugins.join(format!("{name}.cmd"));
            std::fs::write(
                &path,
                format!(
                    "@echo off\r\nif \"%1\"==\"--manifest\" (type \"{}\") else (type \"{}\")\r\n",
                    manifest_path.display(),
                    outcome_path.display()
                ),
            )
            .unwrap();
            path
        }

        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = plugins.join(name);
            std::fs::write(
                &path,
                format!(
                    "#!/bin/sh\nif [ \"$1\" = \"--manifest\" ]; then\n  cat <<'EOF'\n{manifest}\nEOF\nelse\n  echo '{{\"message\":\"ran\"}}'\nfi\n"
                ),
            )
            .unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        }
    }

    const GOOD_MANIFEST: &str = r#"{"protocol_version":1,"name":"demo","version":"0.1.0","commands":[{"key":"fetch","label":"Fetch","target":"release"}]}"#;

    #[test]
    fn an_empty_workspace_directory_contributes_no_plugins() {
        let dir = tempfile::tempdir().unwrap();
        let plugins = dir.path().join("plugins");
        std::fs::create_dir_all(&plugins).unwrap();
        let prefix = plugins.display().to_string();

        // Anything else on this machine's PATH is not this test's business.
        let from_here = discover(dir.path())
            .into_iter()
            .filter(|plugin| plugin.path.starts_with(&prefix))
            .count();

        assert_eq!(from_here, 0);
    }

    #[test]
    fn a_well_formed_plugin_is_discovered_and_usable() {
        let dir = tempfile::tempdir().unwrap();
        fake_plugin(dir.path(), "kilna-plugin-demo", GOOD_MANIFEST);

        let plugin = discover(dir.path())
            .into_iter()
            .find(|plugin| plugin.executable == "kilna-plugin-demo")
            .expect("the plugin must be discovered");

        assert!(plugin.usable, "reason: {:?}", plugin.reason);
        let manifest = plugin.manifest.unwrap();
        assert_eq!(manifest.name, "demo");
        assert_eq!(manifest.commands[0].target, Target::Release);
    }

    #[test]
    fn a_plugin_speaking_another_protocol_is_listed_but_refused() {
        let dir = tempfile::tempdir().unwrap();
        fake_plugin(
            dir.path(),
            "kilna-plugin-future",
            r#"{"protocol_version":99,"name":"future","version":"9.0.0"}"#,
        );

        let plugin = discover(dir.path())
            .into_iter()
            .find(|plugin| plugin.executable == "kilna-plugin-future")
            .expect("it must still be listed");

        assert!(!plugin.usable);
        assert!(
            plugin.reason.as_deref().unwrap_or_default().contains("99"),
            "the mismatch must be shown, not hidden"
        );
    }

    #[test]
    fn a_plugin_with_an_unreadable_manifest_says_why() {
        let dir = tempfile::tempdir().unwrap();
        fake_plugin(dir.path(), "kilna-plugin-broken", "this is not json");

        let plugin = discover(dir.path())
            .into_iter()
            .find(|plugin| plugin.executable == "kilna-plugin-broken")
            .expect("a broken plugin must still be listed");

        assert!(!plugin.usable);
        assert!(plugin.reason.is_some());
    }

    #[test]
    fn a_file_without_the_prefix_is_not_a_plugin() {
        let dir = tempfile::tempdir().unwrap();
        fake_plugin(dir.path(), "something-else", GOOD_MANIFEST);

        let found = discover(dir.path());

        assert!(
            !found
                .iter()
                .any(|plugin| plugin.executable == "something-else")
        );
    }

    #[test]
    fn a_command_runs_and_its_outcome_comes_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = fake_plugin(dir.path(), "kilna-plugin-demo", GOOD_MANIFEST);

        let outcome = invoke(
            &path,
            &Invocation {
                command: "fetch",
                target: Target::Release,
                subject: json!({ "id": "r1" }),
            },
        )
        .unwrap();

        assert_eq!(outcome.message.as_deref(), Some("ran"));
    }

    #[test]
    fn merging_adds_and_overwrites_but_never_clears() {
        let existing = json!({ "keep": 1, "replace": "old" })
            .as_object()
            .cloned()
            .unwrap();
        let returned = json!({ "replace": "new", "add": true })
            .as_object()
            .cloned()
            .unwrap();

        let merged = merge_meta(&existing, &returned);

        assert_eq!(merged.get("keep").unwrap(), &json!(1));
        assert_eq!(merged.get("replace").unwrap(), &json!("new"));
        assert_eq!(merged.get("add").unwrap(), &json!(true));
    }

    #[test]
    fn an_outcome_may_be_empty() {
        let outcome: Outcome = serde_json::from_str("{}").unwrap();

        assert!(outcome.meta.is_empty());
        assert!(outcome.message.is_none());
    }

    #[test]
    fn an_outcome_can_report_failure_without_a_nonzero_exit() {
        let outcome: Outcome = serde_json::from_str(r#"{"error":"no credentials"}"#).unwrap();

        assert_eq!(outcome.error.as_deref(), Some("no credentials"));
    }

    #[test]
    fn a_settings_key_is_namespaced_by_the_executable() {
        assert_eq!(settings_key("kilna-plugin-x"), "plugin:kilna-plugin-x");
    }
}
