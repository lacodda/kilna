use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Where the CLI is and whether it can be used.
#[derive(Debug, Clone, Serialize)]
pub struct Availability {
    pub available: bool,
    pub version: Option<String>,
    /// What to tell the user when it is not available.
    pub reason: Option<String>,
}

/// The executable kilna shells out to. Users install Claude Code themselves —
/// their subscription, their session, their skills. See the README.
#[cfg(windows)]
const EXECUTABLE: &str = "claude.cmd";
#[cfg(not(windows))]
const EXECUTABLE: &str = "claude";

/// What to say when the CLI is nowhere on the PATH. One sentence, one place:
/// the probe, the blocking turn and a streamed run all report the same thing.
pub const MISSING: &str = "Claude Code is not on the PATH. Install it from                            https://claude.com/claude-code, then reopen kilna.";

/// Look for the CLI and report what was found.
///
/// This never fails: "not installed" is an answer the panel shows, not an error
/// that breaks the app. The rest of kilna works without it.
pub fn probe() -> Availability {
    match version() {
        Ok(version) => Availability {
            available: true,
            version: Some(version),
            reason: None,
        },
        Err(error) => Availability {
            available: false,
            version: None,
            reason: Some(error.to_string()),
        },
    }
}

fn version() -> Result<String> {
    let output = command()
        .arg("--version")
        .output()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => Error::Assistant(
                "Claude Code is not on the PATH. Install it from https://claude.com/claude-code, \
                 then reopen kilna."
                    .into(),
            ),
            _ => Error::Assistant(format!("could not run `{EXECUTABLE}`: {error}")),
        })?;

    if !output.status.success() {
        return Err(Error::Assistant(format!(
            "`{EXECUTABLE} --version` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// What the CLI reports for one non-interactive turn.
///
/// Only the fields kilna uses are named; the CLI's output carries more.
#[derive(Debug, Clone, Deserialize)]
struct CliResult {
    #[serde(default)]
    is_error: bool,
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    total_cost_usd: Option<f64>,
    #[serde(default)]
    duration_ms: Option<u64>,
}

/// One turn of a conversation.
#[derive(Debug, Clone, Serialize)]
pub struct Turn {
    pub body: String,
    pub session_id: Option<String>,
    pub cost_usd: Option<f64>,
    pub duration_ms: Option<u64>,
}

/// Send a prompt and wait for the answer.
///
/// `session_id` continues an existing conversation; without it the CLI starts a
/// new one and reports the id to keep.
///
/// `workdir` is where the CLI starts — the empty directory of ADR 0008. Without
/// it the process inherits kilna's own, which in development is the source tree.
///
/// The prompt goes in over stdin rather than as an argument. On Windows the
/// executable is a `.cmd`, and Rust refuses to pass an argument containing a
/// newline to a batch file — a deliberate guard against argument injection.
/// Lyrics and chapters are full of newlines, so the argument form is unusable.
pub fn ask(
    prompt: &str,
    session_id: Option<&str>,
    workdir: Option<&std::path::Path>,
) -> Result<Turn> {
    let mut command = command();
    command.args(["--print", "--output-format", "json"]);

    if let Some(session_id) = session_id {
        command.args(["--resume", session_id]);
    }
    if let Some(dir) = workdir {
        command.current_dir(dir);
    }

    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|error| Error::Assistant(format!("could not run `{EXECUTABLE}`: {error}")))?;

    {
        use std::io::Write;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Assistant("could not write to Claude Code".into()))?;
        stdin
            .write_all(prompt.as_bytes())
            .map_err(|error| Error::Assistant(format!("could not send the prompt: {error}")))?;
        // Dropping stdin closes it, which is what tells the CLI the prompt ended.
    }

    let output = child
        .wait_with_output()
        .map_err(|error| Error::Assistant(format!("`{EXECUTABLE}` did not finish: {error}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if stdout.trim().is_empty() {
        return Err(Error::Assistant(if stderr.trim().is_empty() {
            "Claude Code returned nothing".into()
        } else {
            stderr.trim().to_owned()
        }));
    }

    let parsed: CliResult = serde_json::from_str(stdout.trim()).map_err(|error| {
        Error::Assistant(format!(
            "could not read the reply from Claude Code ({error}): {}",
            stdout.trim()
        ))
    })?;

    let body = parsed.result.unwrap_or_default();

    if parsed.is_error {
        // The CLI puts its own diagnosis in `result` — pass it through rather
        // than replacing it with something vaguer.
        return Err(Error::Assistant(if body.trim().is_empty() {
            "Claude Code reported an error".into()
        } else {
            body
        }));
    }

    Ok(Turn {
        body,
        session_id: parsed.session_id,
        cost_usd: parsed.total_cost_usd,
        duration_ms: parsed.duration_ms,
    })
}

pub(super) fn command() -> Command {
    // Only the Windows branch below mutates it.
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut command = Command::new(EXECUTABLE);

    // Without this a console window flashes on every call on Windows.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_never_panics_and_always_explains_itself() {
        let availability = probe();

        if availability.available {
            assert!(availability.version.is_some());
        } else {
            assert!(
                availability.reason.is_some(),
                "an unavailable CLI must come with something to show the user"
            );
        }
    }

    #[test]
    fn an_error_result_is_reported_with_the_clis_own_wording() {
        let raw =
            r#"{"is_error":true,"result":"Not logged in · Please run /login","session_id":"x"}"#;
        let parsed: CliResult = serde_json::from_str(raw).unwrap();

        assert!(parsed.is_error);
        assert_eq!(
            parsed.result.as_deref(),
            Some("Not logged in · Please run /login")
        );
    }

    #[test]
    fn a_successful_result_carries_the_session_and_the_cost() {
        let raw = r#"{"is_error":false,"result":"ping","session_id":"abc","total_cost_usd":0.18,"duration_ms":2756}"#;
        let parsed: CliResult = serde_json::from_str(raw).unwrap();

        assert_eq!(parsed.session_id.as_deref(), Some("abc"));
        assert_eq!(parsed.total_cost_usd, Some(0.18));
        assert_eq!(parsed.duration_ms, Some(2756));
    }

    #[test]
    fn unknown_fields_in_the_reply_do_not_break_parsing() {
        // The CLI's output grows over time; kilna must survive that.
        let raw = r#"{"is_error":false,"result":"ok","brand_new_field":{"nested":true}}"#;
        let parsed: CliResult = serde_json::from_str(raw).unwrap();

        assert_eq!(parsed.result.as_deref(), Some("ok"));
    }
}
