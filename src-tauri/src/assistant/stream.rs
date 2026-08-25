//! Reading one CLI turn as it happens.
//!
//! The blocking form in [`super::cli::ask`] waits for the whole answer and
//! returns it at once. A run that may take minutes cannot be watched that way,
//! so this module asks the CLI for `stream-json` and turns each line it prints
//! into an [`Event`] the panel can show while the run is still going.

use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStdout, Stdio};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};

/// Something worth showing while a run is in flight.
///
/// Deliberately coarser than the CLI's own output: kilna shows blocks of an
/// answer and the names of tools being used, not every token. The shapes the
/// CLI prints grow over time, and everything unrecognised is ignored rather
/// than shown as noise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Event {
    /// The CLI accepted the prompt and named the session. Arrives first.
    Started { session_id: String },
    /// A block of the answer. Blocks arrive whole, in order.
    Text { body: String },
    /// The assistant is using a tool. `detail` is the most telling argument —
    /// a path for a file read, a pattern for a search — or empty when none of
    /// them is short enough to be worth a line.
    Tool { name: String, detail: String },
    /// The turn finished. Carries what the message row needs.
    Finished {
        body: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cost_usd: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
    },
    /// The turn ended badly. The CLI's own wording where there is one.
    Failed { message: String },
    /// The run was stopped by hand. Carries no words: this is not a failure,
    /// and the panel says it in the reader's language.
    Stopped,
}

/// Turn one line of `stream-json` into an event, if it says anything.
///
/// Unreadable lines are not errors: the CLI is free to print shapes this build
/// has never seen, and a run must not die because of one.
pub fn parse_line(line: &str) -> Option<Event> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let value: Value = serde_json::from_str(line).ok()?;

    match value.get("type").and_then(Value::as_str)? {
        "system" if value.get("subtype").and_then(Value::as_str) == Some("init") => {
            let session_id = value.get("session_id").and_then(Value::as_str)?;
            Some(Event::Started {
                session_id: session_id.to_owned(),
            })
        }
        "assistant" => {
            // One assistant message may carry several blocks; the caller gets
            // the first that says something, which in practice is all there is
            // per line for text and per line for a tool call.
            let content = value.pointer("/message/content")?.as_array()?;
            content.iter().find_map(block_event)
        }
        "result" => Some(result_event(&value)),
        _ => None,
    }
}

fn block_event(block: &Value) -> Option<Event> {
    match block.get("type").and_then(Value::as_str)? {
        "text" => {
            let body = block.get("text").and_then(Value::as_str)?;
            (!body.trim().is_empty()).then(|| Event::Text {
                body: body.to_owned(),
            })
        }
        "tool_use" => {
            let name = block.get("name").and_then(Value::as_str)?;
            Some(Event::Tool {
                name: name.to_owned(),
                detail: tool_detail(block.get("input")),
            })
        }
        _ => None,
    }
}

/// The one argument worth putting next to a tool's name.
///
/// Preference order matches what a person scanning the panel wants to know:
/// which file, which pattern, which command. Long values are cut — this is a
/// caption, not a transcript.
fn tool_detail(input: Option<&Value>) -> String {
    const KEYS: [&str; 5] = ["file_path", "pattern", "command", "path", "query"];
    const LIMIT: usize = 80;

    let Some(Value::Object(input)) = input else {
        return String::new();
    };

    let Some(raw) = KEYS
        .iter()
        .find_map(|key| input.get(*key).and_then(Value::as_str))
    else {
        return String::new();
    };

    let detail: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if detail.chars().count() <= LIMIT {
        return detail;
    }

    let cut: String = detail.chars().take(LIMIT).collect();
    format!("{cut}…")
}

fn result_event(value: &Value) -> Event {
    let body = value
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();

    if value.get("is_error").and_then(Value::as_bool) == Some(true) {
        // The CLI puts its own diagnosis in `result`; pass it through rather
        // than replacing it with something vaguer.
        return Event::Failed {
            message: if body.trim().is_empty() {
                "Claude Code reported an error".into()
            } else {
                body
            },
        };
    }

    Event::Finished {
        body,
        cost_usd: value.get("total_cost_usd").and_then(Value::as_f64),
        duration_ms: value.get("duration_ms").and_then(Value::as_u64),
    }
}

/// The `meta` a finished turn leaves on its message row.
pub fn finished_meta(cost_usd: Option<f64>, duration_ms: Option<u64>) -> Map<String, Value> {
    let mut meta = Map::new();
    if let Some(cost) = cost_usd {
        meta.insert("cost_usd".into(), serde_json::json!(cost));
    }
    if let Some(duration) = duration_ms {
        meta.insert("duration_ms".into(), serde_json::json!(duration));
    }
    meta
}

/// A started CLI process whose output can be read line by line.
pub struct Stream {
    child: Child,
    lines: std::io::Lines<BufReader<ChildStdout>>,
}

/// A way to stop a run that does not go through the reader.
///
/// Stopping and reading are separate on purpose, and this was learned the hard
/// way: with the kill living on [`Stream`], cancelling had to take the same
/// lock the reading loop holds while it waits for the next line — which is
/// exactly the whole time a run is going. Cancellation blocked until the CLI
/// finished on its own, which is the opposite of cancelling.
#[derive(Debug, Clone, Copy)]
pub struct Stopper {
    pid: u32,
}

impl Stopper {
    /// Stop the run now. Whatever it had already said stays.
    ///
    /// Killing the spawned process alone is not enough on Windows: the
    /// executable is `claude.cmd`, so what is spawned is a shell wrapper and
    /// the process doing the work is its child. Measured, not assumed — a run
    /// cancelled after two seconds went on to finish a 63-second essay. The
    /// whole tree goes instead; `taskkill /T` walks it.
    ///
    /// Elsewhere the CLI is the process itself, and killing it is the whole
    /// story.
    pub fn stop(self) {
        #[cfg(windows)]
        {
            let mut sweep = std::process::Command::new("taskkill");
            sweep.args(["/T", "/F", "/PID", &self.pid.to_string()]);

            // Without this the sweep flashes a console window of its own.
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            sweep.creation_flags(CREATE_NO_WINDOW);

            let _ = sweep.stdout(Stdio::null()).stderr(Stdio::null()).status();
        }

        #[cfg(not(windows))]
        {
            // `kill(1)` rather than a signal through libc: it saves a
            // dependency carried for one line, and the CLI there is the
            // process itself rather than a wrapper around one.
            let _ = std::process::Command::new("kill")
                .args(["-9", &self.pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

impl Stream {
    /// Start a turn and return before it finishes.
    ///
    /// `workdir` is where the CLI starts — the empty directory of ADR 0008.
    ///
    /// The prompt goes in over stdin for the reason spelled out in
    /// [`super::cli::ask`]: on Windows the executable is a `.cmd`, and Rust
    /// refuses to pass an argument containing a newline to a batch file.
    pub fn start(
        prompt: &str,
        session_id: Option<&str>,
        workdir: Option<&std::path::Path>,
    ) -> Result<Self> {
        let mut command = super::cli::command();
        command.args([
            "--print",
            "--output-format",
            "stream-json",
            // stream-json refuses to run without it.
            "--verbose",
        ]);

        if let Some(session_id) = session_id {
            command.args(["--resume", session_id]);
        }
        if let Some(dir) = workdir {
            command.current_dir(dir);
        }

        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => Error::Assistant(super::cli::MISSING.into()),
                _ => Error::Assistant(format!("could not run Claude Code: {error}")),
            })?;

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

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Assistant("could not read from Claude Code".into()))?;

        Ok(Self {
            child,
            lines: BufReader::new(stdout).lines(),
        })
    }

    /// The next event, or `None` when the CLI has stopped printing.
    pub fn next_event(&mut self) -> Option<Event> {
        loop {
            let line = self.lines.next()?.ok()?;
            if let Some(event) = parse_line(&line) {
                return Some(event);
            }
        }
    }

    /// A handle that can stop this run without waiting for the reader.
    pub fn stopper(&self) -> Stopper {
        Stopper {
            pid: self.child.id(),
        }
    }

    /// Reap the process so a finished run leaves nothing behind.
    pub fn wait(&mut self) {
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shapes below are trimmed from real `stream-json` output (CLI 2.1.241).

    #[test]
    fn the_init_line_names_the_session() {
        let line = r#"{"type":"system","subtype":"init","session_id":"2f653af5","model":"claude-opus-5","tools":["Read"]}"#;

        assert_eq!(
            parse_line(line),
            Some(Event::Started {
                session_id: "2f653af5".into()
            })
        );
    }

    #[test]
    fn a_text_block_becomes_a_text_event() {
        let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Here is a verse."}]},"session_id":"x"}"#;

        assert_eq!(
            parse_line(line),
            Some(Event::Text {
                body: "Here is a verse.".into()
            })
        );
    }

    #[test]
    fn a_tool_call_is_named_with_its_most_telling_argument() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/tmp/notes.md"}}]}}"#;

        assert_eq!(
            parse_line(line),
            Some(Event::Tool {
                name: "Read".into(),
                detail: "/tmp/notes.md".into()
            })
        );
    }

    #[test]
    fn a_tool_call_without_a_usable_argument_still_reports_its_name() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Ponder","input":{"depth":3}}]}}"#;

        assert_eq!(
            parse_line(line),
            Some(Event::Tool {
                name: "Ponder".into(),
                detail: String::new()
            })
        );
    }

    #[test]
    fn a_long_detail_is_cut_rather_than_wrapped() {
        let long = "x".repeat(200);
        let block = serde_json::json!({
            "type": "assistant",
            "message": {"content": [{"type": "tool_use", "name": "Bash", "input": {"command": long}}]}
        });

        let Some(Event::Tool { detail, .. }) = parse_line(&block.to_string()) else {
            panic!("expected a tool event");
        };

        // 80 kept plus the ellipsis.
        assert_eq!(detail.chars().count(), 81);
        assert!(detail.ends_with('…'));
    }

    #[test]
    fn a_multiline_command_is_flattened_into_one_caption() {
        let block = serde_json::json!({
            "type": "assistant",
            "message": {"content": [{"type": "tool_use", "name": "Bash", "input": {"command": "git status\ngit diff"}}]}
        });

        assert_eq!(
            parse_line(&block.to_string()),
            Some(Event::Tool {
                name: "Bash".into(),
                detail: "git status git diff".into()
            })
        );
    }

    #[test]
    fn the_result_line_finishes_the_run_with_its_cost() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,"result":"ping","total_cost_usd":0.39,"duration_ms":2065,"session_id":"x"}"#;

        assert_eq!(
            parse_line(line),
            Some(Event::Finished {
                body: "ping".into(),
                cost_usd: Some(0.39),
                duration_ms: Some(2065),
            })
        );
    }

    #[test]
    fn an_error_result_keeps_the_clis_own_wording() {
        let line =
            r#"{"type":"result","is_error":true,"result":"Not logged in · Please run /login"}"#;

        assert_eq!(
            parse_line(line),
            Some(Event::Failed {
                message: "Not logged in · Please run /login".into()
            })
        );
    }

    #[test]
    fn an_error_result_with_nothing_to_say_still_says_something() {
        let line = r#"{"type":"result","is_error":true,"result":""}"#;

        let Some(Event::Failed { message }) = parse_line(line) else {
            panic!("expected a failure");
        };
        assert!(!message.trim().is_empty());
    }

    #[test]
    fn lines_this_build_does_not_understand_are_passed_over() {
        // The CLI prints more kinds than kilna shows, and gains new ones.
        for line in [
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"}}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"ok"}]}}"#,
            r#"{"type":"brand_new_shape","payload":{"nested":true}}"#,
            "not json at all",
            "",
            "   ",
        ] {
            assert_eq!(parse_line(line), None, "`{line}` should say nothing");
        }
    }

    #[test]
    fn an_empty_text_block_is_not_shown() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"  "}]}}"#;

        assert_eq!(parse_line(line), None);
    }

    #[test]
    fn a_finished_turn_without_costs_leaves_no_empty_fields() {
        let meta = finished_meta(None, None);

        assert!(meta.is_empty(), "absent numbers must not be stored as null");
    }

    #[test]
    fn a_finished_turn_records_what_the_cli_reported() {
        let meta = finished_meta(Some(0.18), Some(2756));

        assert_eq!(meta["cost_usd"], serde_json::json!(0.18));
        assert_eq!(meta["duration_ms"], serde_json::json!(2756));
    }
}
