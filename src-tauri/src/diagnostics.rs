//! What to send when something went wrong (v0.162): the version, the system,
//! the assistant's CLI and the last errors of the log - as one text to paste
//! into an issue - and whether a newer kilna is out.
//!
//! No telemetry: nothing here leaves the machine by itself. The text goes to
//! the clipboard on a press, and the check for a newer version asks the
//! project's public releases and nothing else. A path in a log line that
//! names the person's home is written `~`, so a pasted report does not carry
//! their user name.

use std::path::Path;

use serde::Serialize;

/// How many of the log's last errors a report carries.
const ERRORS: usize = 20;
/// How much of the end of the log is read for them.
const TAIL_BYTES: u64 = 256 * 1024;

/// What a report says about this machine and this kilna.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct Diagnostics {
    pub version: String,
    pub os: String,
    pub arch: String,
    /// The WebView the window is drawn by.
    pub webview: Option<String>,
    /// The assistant's CLI, as it answered `--version`.
    pub cli: Option<String>,
    /// The log's last errors, oldest first, the home written `~`.
    pub errors: Vec<String>,
}

/// A line with the person's home written `~`, however its slashes run.
pub fn redact(line: &str, home: Option<&Path>) -> String {
    let Some(home) = home.map(|home| home.to_string_lossy().into_owned()) else {
        return line.to_owned();
    };
    if home.len() < 3 {
        return line.to_owned();
    }
    let forward = home.replace('\\', "/");
    let back = home.replace('/', "\\");
    line.replace(&back, "~").replace(&forward, "~")
}

/// The last errors of a log file, oldest first.
pub fn recent_errors(log: &Path, home: Option<&Path>) -> Vec<String> {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = std::fs::File::open(log) else {
        return Vec::new();
    };
    let length = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    let _ = file.seek(SeekFrom::Start(length.saturating_sub(TAIL_BYTES)));
    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&bytes);
    let errors: Vec<String> = text
        .lines()
        .filter(|line| line.split(' ').nth(1) == Some("ERROR"))
        .map(|line| redact(line, home))
        .collect();
    errors[errors.len().saturating_sub(ERRORS)..].to_vec()
}

/// A report as text to paste into an issue.
pub fn text(found: &Diagnostics) -> String {
    let mut out = format!(
        "kilna {}\nSystem: {} ({})\nWebView: {}\nClaude Code: {}\n",
        found.version,
        found.os,
        found.arch,
        found.webview.as_deref().unwrap_or("unknown"),
        found.cli.as_deref().unwrap_or("not found"),
    );
    if found.errors.is_empty() {
        out.push_str("\nNo errors in the log.\n");
    } else {
        out.push_str("\nLast errors:\n```\n");
        for line in &found.errors {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str("```\n");
    }
    out
}

/// Whether a newer kilna is out, as the project's releases say.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct UpdateCheck {
    pub current: String,
    /// The newest release's version, when the releases could be read.
    pub latest: Option<String>,
    /// Its page, to download it from.
    pub url: Option<String>,
    pub newer: bool,
}

/// `0.161.0` as numbers to compare; a part that is no number reads as 0.
fn parts(version: &str) -> Vec<u64> {
    version
        .trim()
        .trim_start_matches('v')
        .split(['.', '-', '+'])
        .take(3)
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

/// Whether `latest` is a later version than `current`.
pub fn is_newer(latest: &str, current: &str) -> bool {
    parts(latest) > parts(current)
}

/// The project's public releases.
const RELEASES: &str = "https://api.github.com/repos/lacodda/kilna/releases/latest";

/// Ask the project's releases for the newest one. A machine offline, or a
/// page that cannot be read, says nothing is known rather than failing.
pub fn check(current: &str) -> UpdateCheck {
    let answer = ureq::get(RELEASES)
        .set("User-Agent", "kilna")
        .set("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .ok()
        .and_then(|response| response.into_string().ok())
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    let latest = answer
        .as_ref()
        .and_then(|value| value.get("tag_name"))
        .and_then(serde_json::Value::as_str)
        .map(|tag| tag.trim_start_matches('v').to_owned());
    let url = answer
        .as_ref()
        .and_then(|value| value.get("html_url"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    UpdateCheck {
        current: current.to_owned(),
        newer: latest
            .as_deref()
            .is_some_and(|latest| is_newer(latest, current)),
        latest,
        url,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_home_in_a_line_is_written_as_a_tilde_however_its_slashes_run() {
        let home = Path::new("C:\\Users\\someone");
        assert_eq!(
            redact(
                "could not open C:\\Users\\someone\\media\\a.mp4",
                Some(home)
            ),
            "could not open ~\\media\\a.mp4"
        );
        assert_eq!(
            redact("could not open C:/Users/someone/media/a.mp4", Some(home)),
            "could not open ~/media/a.mp4"
        );
        assert_eq!(redact("nothing personal", None), "nothing personal");
    }

    #[test]
    fn only_the_last_errors_of_the_log_are_taken() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("kilna.log");
        let mut text = String::new();
        for n in 0..30 {
            text.push_str(&format!(
                "2026-11-19T10:00:{n:02}.000Z ERROR trash: failed {n}\n"
            ));
            text.push_str(&format!(
                "2026-11-19T10:00:{n:02}.500Z INFO trash: fine {n}\n"
            ));
        }
        std::fs::write(&log, text).unwrap();
        let errors = recent_errors(&log, None);
        assert_eq!(errors.len(), ERRORS);
        assert!(errors[0].ends_with("failed 10"));
        assert!(errors.last().unwrap().ends_with("failed 29"));
        assert!(recent_errors(&dir.path().join("missing.log"), None).is_empty());
    }

    #[test]
    fn a_later_version_is_newer_and_an_equal_one_is_not() {
        assert!(is_newer("0.162.0", "0.161.0"));
        assert!(is_newer("v1.0.0", "0.166.3"));
        assert!(!is_newer("0.161.0", "0.161.0"));
        assert!(!is_newer("0.160.9", "0.161.0"));
    }

    #[test]
    fn a_report_reads_as_text_to_paste() {
        let report = text(&Diagnostics {
            version: "0.162.0".into(),
            os: "windows".into(),
            arch: "x86_64".into(),
            webview: Some("131.0".into()),
            cli: None,
            errors: vec!["2026-11-19T10:00:00.000Z ERROR trash: failed".into()],
        });
        assert!(report.starts_with("kilna 0.162.0\n"));
        assert!(report.contains("Claude Code: not found"));
        assert!(report.contains("```\n2026-11-19T10:00:00.000Z ERROR trash: failed\n```"));
    }
}
