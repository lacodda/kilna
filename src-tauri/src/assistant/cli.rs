use std::process::Command;

use serde::Serialize;

use crate::error::{Error, Result};

/// Where the CLI is and whether it can be used.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
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
/// the probe and a streamed run report the same thing.
pub const MISSING: &str = "Claude Code is not on the PATH. Install it from \
 https://claude.com/claude-code, then reopen kilna.";

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

/// How long an answer of [`probe`] is believed by [`probe_lately`].
const BELIEVED_FOR: std::time::Duration = std::time::Duration::from_secs(300);

/// What [`probe`] last found, if it was asked within five minutes; asked
/// again otherwise.
///
/// For what asks often and can be wrong for a while: a text joining the line
/// to be scored by itself paid about a second for a process each time
/// (v0.170). A CLI installed or removed meanwhile is noticed within five
/// minutes - and a run that finds it gone says so itself. A panel that shows
/// the answer to a person asks [`probe`].
pub fn probe_lately() -> Availability {
    static LAST: std::sync::Mutex<Option<(std::time::Instant, Availability)>> =
        std::sync::Mutex::new(None);
    let mut last = LAST
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((at, found)) = last.as_ref()
        && at.elapsed() < BELIEVED_FOR
    {
        return found.clone();
    }
    let found = probe();
    *last = Some((std::time::Instant::now(), found.clone()));
    found
}

fn version() -> Result<String> {
    let output = command()
        .arg("--version")
        .output()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => Error::Assistant(MISSING.into()),
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

/// A folder the executable is taken from instead of the PATH, when a
/// rehearsal named one ([`stand_in`]).
static STAND_IN: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);

/// Take the executable from `dir` rather than from the PATH, or from the
/// PATH again with `None`.
///
/// For a rehearsal of a whole run against a CLI that is not Claude Code
/// (`tests/auto_score_rehearsal.rs`): the PATH is the process's, and
/// changing it under running threads is not something a test may do.
/// Nothing in the application calls this.
#[doc(hidden)]
pub fn stand_in(dir: Option<&std::path::Path>) {
    *STAND_IN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = dir.map(std::path::Path::to_path_buf);
}

pub(super) fn command() -> Command {
    let named = STAND_IN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    // Only the Windows branch below mutates it.
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut command = match named {
        Some(dir) => Command::new(dir.join(EXECUTABLE)),
        None => Command::new(EXECUTABLE),
    };

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
}
