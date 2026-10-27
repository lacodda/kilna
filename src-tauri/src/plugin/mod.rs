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
    /// The files the command made, whole paths (v0.101). A file inside the
    /// work's folder becomes its finished file when the kind's rule would not
    /// find it by itself (ADR 0070).
    #[serde(default)]
    pub files: Vec<String>,
}

/// One line of progress a plugin writes to stderr while it runs:
/// `{"progress": 0.4, "message": "cutting 2 of 3"}` (v0.101). Both are
/// optional; a line that is not a JSON object naming either is ordinary
/// stderr, kept for the message if the plugin fails.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Progress {
    /// How far along, from 0 to 1, when the plugin can say.
    #[serde(default)]
    pub progress: Option<f64>,
    /// What it is doing now.
    #[serde(default)]
    pub message: Option<String>,
}

impl Progress {
    /// A line of stderr read as progress, when it is one. A share outside 0..1
    /// is held to it: a bar past its end says nothing a full bar does not.
    pub fn of_line(line: &str) -> Option<Self> {
        let line = line.trim();
        if !line.starts_with('{') {
            return None;
        }
        let mut progress: Self = serde_json::from_str(line).ok()?;
        if progress.progress.is_none() && progress.message.is_none() {
            return None;
        }
        progress.progress = progress
            .progress
            .filter(|share| share.is_finite())
            .map(|share| share.clamp(0.0, 1.0));
        Some(progress)
    }
}

/// How long a plugin may take before it is stopped, when its command names no
/// allowance of its own (see [`manifest::Command::allowance`]).
pub const PLUGIN_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(manifest::DEFAULT_TIMEOUT_SECONDS);

/// Wait for a child to finish, reading its output as it goes, and stop it if
/// it runs past `limit` or `cancel` is raised.
///
/// The output is read on threads of its own: a child that writes more than a
/// pipe holds blocks until someone reads, and waiting for it to exit first
/// would wait for ever. Lines of stderr that are progress are handed to
/// `on_progress` on this thread as they come; the rest are kept as the
/// child's stderr.
fn wait_within(
    mut child: std::process::Child,
    limit: std::time::Duration,
    cancel: &std::sync::atomic::AtomicBool,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<std::process::Output> {
    use std::io::{BufRead, Read};
    use std::sync::atomic::Ordering;

    let mut stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let read_out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(pipe) = stdout.as_mut() {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    });
    let (sender, progress) = std::sync::mpsc::channel::<Progress>();
    let read_err = std::thread::spawn(move || {
        let mut kept = Vec::new();
        let Some(pipe) = stderr else {
            return kept;
        };
        let mut reader = std::io::BufReader::new(pipe);
        let mut line = Vec::new();
        // Split on the byte, not read as text: a plugin's stderr is in
        // whatever encoding its system speaks, and one bad byte must not end
        // the reading of the rest.
        while let Ok(read) = reader.read_until(b'\n', &mut line) {
            if read == 0 {
                break;
            }
            match Progress::of_line(&String::from_utf8_lossy(&line)) {
                Some(step) => {
                    let _ = sender.send(step);
                }
                None => kept.extend_from_slice(&line),
            }
            line.clear();
        }
        kept
    });

    let started = std::time::Instant::now();
    let status = loop {
        while let Ok(step) = progress.try_recv() {
            on_progress(step);
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if cancel.load(Ordering::SeqCst) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::refused("plugin.cancelled"));
            }
            Ok(None) if started.elapsed() >= limit => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::refused("plugin.timedOut").param("seconds", limit.as_secs()));
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
            Err(error) => {
                return Err(Error::Internal(format!(
                    "the plugin process could not be waited on: {error}"
                )));
            }
        }
    };

    let stderr = read_err.join().unwrap_or_default();
    // What the child said in its last moments, after the loop stopped
    // looking: a bar that ends at 90 % reads as a plugin that gave up.
    while let Ok(step) = progress.try_recv() {
        on_progress(step);
    }
    Ok(std::process::Output {
        status,
        stdout: read_out.join().unwrap_or_default(),
        stderr,
    })
}

/// How a run is let go of: how long it may take, and how it is stopped or
/// followed while it runs.
pub struct Run<'a> {
    /// How long the command may take - its manifest's allowance.
    pub limit: std::time::Duration,
    /// Raised to stop the plugin before it finishes.
    pub cancel: &'a std::sync::atomic::AtomicBool,
    /// Told each line of progress the plugin writes.
    pub on_progress: &'a mut dyn FnMut(Progress),
}

/// Run one command of one plugin.
///
/// The invocation goes in over stdin as JSON and the outcome comes back over
/// stdout, so a plugin can be written in any language. Plugins live for the
/// duration of a call; nothing here starts a service.
pub fn invoke(path: &Path, invocation: &Invocation<'_>, run: Run<'_>) -> Result<Outcome> {
    let payload = serde_json::to_vec(invocation)?;

    let mut child = command(path)
        .arg("run")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| Error::refused("plugin.spawnFailed").param("cause", error.to_string()))?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Internal("the plugin's stdin was not piped".into()))?;
        if let Err(error) = stdin.write_all(&payload) {
            // A plugin is allowed to answer without reading its input; if it
            // exits first, the write sees a closed pipe. The verdict still
            // arrives through stdout and the exit status, so only a failure
            // other than the closed pipe is real.
            if error.kind() != std::io::ErrorKind::BrokenPipe {
                return Err(Error::refused("plugin.sendFailed").param("cause", error.to_string()));
            }
        }
    }

    let output = wait_within(child, run.limit, run.cancel, run.on_progress)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stderr.trim().is_empty() {
            format!("failed with {}", output.status)
        } else {
            stderr.trim().to_owned()
        };
        return Err(Error::refused("plugin.failed").param("detail", detail));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        // Silence is success with nothing to say.
        return Ok(Outcome {
            meta: Map::new(),
            message: None,
            error: None,
            files: Vec::new(),
        });
    }

    let outcome: Outcome = serde_json::from_str(stdout.trim()).map_err(|error| {
        Error::refused("plugin.badReply")
            .param("cause", error.to_string())
            .param("output", stdout.trim().to_owned())
    })?;

    if let Some(message) = outcome.error {
        return Err(Error::refused("plugin.reported").param("message", message));
    }

    Ok(outcome)
}

// Name a plugin is known by: the file name without its extension.
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

    /// A child that runs for a while and says nothing.
    fn a_sleeper() -> std::process::Child {
        #[cfg(windows)]
        let mut command = {
            let mut command = std::process::Command::new("cmd");
            command.args(["/C", "ping -n 6 127.0.0.1 >NUL"]);
            command
        };
        #[cfg(not(windows))]
        let mut command = {
            let mut command = std::process::Command::new("sleep");
            command.arg("5");
            command
        };
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    }

    /// A plugin that never answers is stopped, and says so, instead of
    /// holding the command for as long as it likes.
    #[test]
    fn a_plugin_that_does_not_answer_is_stopped() {
        let started = std::time::Instant::now();
        let refused = wait_within(
            a_sleeper(),
            std::time::Duration::from_millis(300),
            &std::sync::atomic::AtomicBool::new(false),
            &mut |_| {},
        );

        let error = refused.expect_err("the wait gives up");
        assert_eq!(error.refusal().map(|r| r.code), Some("plugin.timedOut"));
        assert!(
            started.elapsed() < std::time::Duration::from_secs(3),
            "stopped at the limit, not when the child finished on its own"
        );
    }

    /// Output larger than a pipe holds is read while the child runs, so the
    /// child is never left blocked on a full pipe with nobody reading.
    ///
    /// Larger than any pipe's buffer (64 KiB on Linux, less on Windows) is
    /// all it takes. It used to print 8,000 lines through `cmd`, which alone
    /// takes a moment and under the whole suite's load overran the 30 seconds
    /// `wait_within` allows - failing a test about deadlock for being slow.
    #[test]
    fn a_plugin_that_says_a_lot_is_read_to_the_end() {
        #[cfg(windows)]
        let mut command = {
            let mut command = std::process::Command::new("cmd");
            command.args([
                "/C",
                "for /L %i in (1,1,3000) do @echo 0123456789012345678901234567890123456789",
            ]);
            command
        };
        #[cfg(not(windows))]
        let mut command = {
            let mut command = std::process::Command::new("sh");
            command.args(["-c", "head -c 300000 /dev/zero"]);
            command
        };
        let child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let output = wait_within(
            child,
            std::time::Duration::from_secs(30),
            &std::sync::atomic::AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        assert!(output.status.success());
        assert!(
            output.stdout.len() > 100_000,
            "read {} bytes",
            output.stdout.len()
        );
    }

    /// A plugin stopped by hand is stopped at once, and says so, rather than
    /// running out its allowance.
    #[test]
    fn a_plugin_is_stopped_when_asked() {
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let raise = {
            let cancel = cancel.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(200));
                cancel.store(true, std::sync::atomic::Ordering::SeqCst);
            })
        };
        let started = std::time::Instant::now();

        let stopped = wait_within(
            a_sleeper(),
            std::time::Duration::from_secs(60),
            &cancel,
            &mut |_| {},
        );
        raise.join().unwrap();

        let error = stopped.expect_err("the wait gives up when asked");
        assert_eq!(error.refusal().map(|r| r.code), Some("plugin.cancelled"));
        assert!(
            started.elapsed() < std::time::Duration::from_secs(4),
            "stopped when asked, not when the child finished on its own"
        );
    }

    /// Lines of progress on stderr are handed on as they come, and only the
    /// other lines are kept as what the plugin said.
    #[test]
    fn progress_on_stderr_is_followed_and_the_rest_kept() {
        // The lines are written to a file and printed from there: cmd's `echo`
        // mangles quotes, and this is about kilna, not batch escaping.
        let dir = tempfile::tempdir().unwrap();
        let said = dir.path().join("stderr.txt");
        std::fs::write(
            &said,
            "{\"progress\":0.5,\"message\":\"half\"}\nffmpeg says hello\n{\"progress\":7}\n",
        )
        .unwrap();
        #[cfg(windows)]
        let mut command = {
            let mut command = std::process::Command::new("cmd");
            command.args(["/C", "type", &said.to_string_lossy(), "1>&2"]);
            command
        };
        #[cfg(not(windows))]
        let mut command = {
            let mut command = std::process::Command::new("sh");
            command.args(["-c", "cat \"$0\" >&2", &said.to_string_lossy()]);
            command
        };
        let child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut seen = Vec::new();

        let output = wait_within(
            child,
            std::time::Duration::from_secs(30),
            &std::sync::atomic::AtomicBool::new(false),
            &mut |step| seen.push(step),
        )
        .unwrap();

        assert_eq!(
            seen,
            vec![
                Progress {
                    progress: Some(0.5),
                    message: Some("half".into())
                },
                Progress {
                    progress: Some(1.0),
                    message: None
                },
            ],
            "a share past the end is a full bar"
        );
        let said = String::from_utf8_lossy(&output.stderr);
        assert!(said.contains("ffmpeg says hello"), "kept: {said:?}");
        assert!(!said.contains("progress"), "progress is not kept: {said:?}");
    }

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

    /// [`discover`], retried while a fake plugin is still "Text file busy".
    ///
    /// On Unix a test spawning a process in parallel can inherit the write
    /// descriptor of a script another test has just created — descriptors only
    /// close when exec flips CLOEXEC — and executing that script in the window
    /// answers ETXTBSY. The window is microseconds and nothing about kilna is
    /// wrong: a real plugin is not being written while it is being run. Caught
    /// by CI on a release commit; the same flake and the same cure as turnout.
    fn discovered(dir: &Path) -> Vec<Plugin> {
        for _ in 0..50 {
            let found = discover(dir);
            let busy = found.iter().any(|plugin| {
                plugin
                    .reason
                    .as_deref()
                    .unwrap_or_default()
                    .contains("Text file busy")
            });
            if !busy {
                return found;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("a fake plugin stayed busy far longer than the exec window");
    }

    #[test]
    fn an_empty_workspace_directory_contributes_no_plugins() {
        let dir = tempfile::tempdir().unwrap();
        let plugins = dir.path().join("plugins");
        std::fs::create_dir_all(&plugins).unwrap();
        let prefix = plugins.display().to_string();

        // Anything else on this machine's PATH is not this test's business.
        let from_here = discovered(dir.path())
            .into_iter()
            .filter(|plugin| plugin.path.starts_with(&prefix))
            .count();

        assert_eq!(from_here, 0);
    }

    #[test]
    fn a_well_formed_plugin_is_discovered_and_usable() {
        let dir = tempfile::tempdir().unwrap();
        fake_plugin(dir.path(), "kilna-plugin-demo", GOOD_MANIFEST);

        let plugin = discovered(dir.path())
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

        let plugin = discovered(dir.path())
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

        let plugin = discovered(dir.path())
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

        let found = discovered(dir.path());

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

        // Retried for the same reason as `discovered`: a script written moments
        // ago can still be busy in another test's fork/exec window.
        let outcome = {
            let invocation = Invocation {
                command: "fetch",
                target: Target::Release,
                subject: json!({ "id": "r1" }),
            };
            let cancel = std::sync::atomic::AtomicBool::new(false);
            let run = || {
                invoke(
                    &path,
                    &invocation,
                    Run {
                        limit: PLUGIN_TIMEOUT,
                        cancel: &cancel,
                        on_progress: &mut |_| {},
                    },
                )
            };
            let mut attempt = run();
            for _ in 0..50 {
                match &attempt {
                    Err(error) if error.to_string().contains("Text file busy") => {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        attempt = run();
                    }
                    _ => break,
                }
            }
            attempt.unwrap()
        };

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
