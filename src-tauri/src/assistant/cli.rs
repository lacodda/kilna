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
}
