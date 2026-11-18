//! The thin trigger an agent outside the window is given (v0.127, FR-17):
//! a short file in each client's own place that says one thing - when the
//! person talks about their works, go to kilna's MCP server and read the
//! method there. Not a line of a method, an axis or a rule of the world is
//! in it: those live in kilna and are read each time, so the file never goes
//! stale and never carries the person's craft out of their space.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// The agents kilna writes a trigger for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "kebab-case")]
pub enum Client {
    ClaudeCode,
    Codex,
    Gemini,
    Cursor,
}

impl Client {
    pub const ALL: [Client; 4] = [
        Client::ClaudeCode,
        Client::Codex,
        Client::Gemini,
        Client::Cursor,
    ];

    /// Where the client reads such a file, under the person's home - none for
    /// one that reads it from a project, which the person picks.
    pub fn place(self, home: &Path) -> Option<PathBuf> {
        match self {
            Client::ClaudeCode => Some(
                home.join(".claude")
                    .join("skills")
                    .join("kilna")
                    .join("SKILL.md"),
            ),
            Client::Codex => Some(home.join(".codex").join("prompts").join("kilna.md")),
            Client::Gemini => Some(home.join(".gemini").join("commands").join("kilna.toml")),
            Client::Cursor => None,
        }
    }

    /// The file's name where the person picks the folder.
    pub fn file_name(self) -> &'static str {
        match self {
            Client::ClaudeCode => "SKILL.md",
            Client::Codex => "kilna.md",
            Client::Gemini => "kilna.toml",
            Client::Cursor => "kilna.mdc",
        }
    }
}

/// When the trigger fires: what an agent matches the person's words against.
const WHEN: &str = "Work on the person's songs, videos, posts, episodes or chapters kept in \
kilna - critique, score, write, plan releases, work the canon or the words. Use it whenever \
they talk about their works, their space, a release or a method of their craft.";

/// What the agent does once it fires. Routing only.
const STEPS: &str = "kilna keeps the person's works and the methods of their craft. None of \
the craft is written here: read it from kilna every time.

1. Call the `kilna` MCP server's `workspace`: the space, its kinds, the axes with their \
rubrics, the statuses - and `guides`, the skills of this profile.
2. Pick the skill whose `when` fits the request and call `guide` with its key: you get its \
method and the live context of the space.
3. Read what the method needs with `catalogue`, `work`, `text`, `canon`, `notes`, `register`.
4. Follow the method. You cannot write to kilna: propose through the tool `guide` names, and \
the person applies it with one click.
5. If kilna's server is not connected, say so and stop - never make up a method.";

/// The trigger for a client, in the shape that client reads.
pub fn text(client: Client) -> String {
    match client {
        Client::ClaudeCode => {
            format!("---\nname: kilna\ndescription: {WHEN}\n---\n\n# kilna\n\n{STEPS}\n")
        }
        Client::Codex => format!("---\ndescription: {WHEN}\n---\n\n# kilna\n\n{STEPS}\n"),
        Client::Gemini => format!(
            "description = \"{}\"\nprompt = \"\"\"\n{STEPS}\n\nThe request: {{{{args}}}}\n\"\"\"\n",
            WHEN.replace('"', "'")
        ),
        Client::Cursor => {
            format!("---\ndescription: {WHEN}\nalwaysApply: false\n---\n\n# kilna\n\n{STEPS}\n")
        }
    }
}

/// Where the person's home is, by the variable each system names it with.
pub fn home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
}

/// Write a client's trigger at `path`. A file already there is replaced only
/// when the person said so: it may be one they wrote.
pub fn install(client: Client, path: &Path, replace: bool) -> Result<PathBuf> {
    let shown = path.display().to_string();
    if path.exists() && !replace {
        return Err(Error::refused("trigger.exists").param("path", shown.as_str()));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|cause| {
            Error::refused("file.notWritten")
                .param("path", shown.as_str())
                .param("cause", cause.to_string())
        })?;
    }
    std::fs::write(path, text(client)).map_err(|cause| {
        Error::refused("file.notWritten")
            .param("path", shown.as_str())
            .param("cause", cause.to_string())
    })?;
    Ok(path.to_path_buf())
}

/// A client's trigger as Settings shows it: the text, and where it goes.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct TriggerFile {
    pub client: Client,
    pub text: String,
    /// Where it is installed; none for a client that reads it from a project.
    pub path: Option<String>,
    /// Whether a file is already there.
    pub exists: bool,
    pub file_name: String,
}

pub fn file(client: Client, home: Option<&Path>) -> TriggerFile {
    let path = home.and_then(|home| client.place(home));
    TriggerFile {
        client,
        text: text(client),
        exists: path.as_ref().is_some_and(|path| path.exists()),
        path: path.map(|path| path.display().to_string()),
        file_name: client.file_name().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trigger_only_routes_to_kilna_and_is_short() {
        let shipped = crate::profile::builtin().unwrap();
        for client in Client::ALL {
            let text = text(client);
            let lines = text.lines().filter(|line| !line.trim().is_empty()).count();
            assert!(lines <= 20, "{client:?}: {lines} lines\n{text}");
            assert!(
                text.contains("`workspace`") && text.contains("`guide`"),
                "{text}"
            );
            // Not a line of any shipped method.
            for profile in &shipped {
                for prompt in &profile.config.prompts {
                    for line in prompt.method().unwrap_or_default().lines() {
                        let line = line.trim();
                        if line.len() > 30 {
                            assert!(!text.contains(line), "{client:?} carries a method: {line}");
                        }
                    }
                }
            }
        }
        assert!(text(Client::ClaudeCode).starts_with("---\nname: kilna\n"));
        assert!(text(Client::Gemini).contains("prompt = \"\"\""));
    }

    #[test]
    fn a_trigger_is_installed_in_its_place_and_never_over_a_file_unasked() {
        let home = tempfile::tempdir().unwrap();
        let path = Client::ClaudeCode.place(home.path()).unwrap();
        assert!(!file(Client::ClaudeCode, Some(home.path())).exists);
        install(Client::ClaudeCode, &path, false).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            text(Client::ClaudeCode)
        );
        assert!(file(Client::ClaudeCode, Some(home.path())).exists);

        std::fs::write(&path, "mine").unwrap();
        let refused = install(Client::ClaudeCode, &path, false).unwrap_err();
        assert_eq!(refused.refusal().map(|r| r.code), Some("trigger.exists"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "mine");
        install(Client::ClaudeCode, &path, true).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            text(Client::ClaudeCode)
        );

        assert!(Client::Cursor.place(home.path()).is_none());
        assert_eq!(
            file(Client::Cursor, Some(home.path())).file_name,
            "kilna.mdc"
        );
    }
}
