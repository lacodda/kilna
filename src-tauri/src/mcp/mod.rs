//! The workspace, served to an agent over MCP.
//!
//! `kilna --mcp` runs the same binary without a window: JSON-RPC 2.0 on
//! stdin/stdout, one message per line, the way the line's other tools serve
//! theirs. Nothing else may be written to stdout — a stray `println!` is a
//! protocol error to the client.
//!
//! Reading is open: the catalogue, a card, a body, the scores, the calendar,
//! the notes, a search. Writing is not. An agent **proposes** — a version, a
//! score, a note, or a whole work as one package — and the proposal lands as
//! a message in a chat on the work (in the client's own chat, for a work that
//! does not exist yet), where the buttons that already apply the assistant's
//! own proposals apply this one too. Nothing here writes a version, a score
//! or a note directly: the rule since v0.28, "the assistant proposes, a
//! person applies", holds for an assistant outside the window exactly as for
//! the one inside it. See ADR 0016; the package and the mark a person leaves
//! on an applied proposal are ADR 0018.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::Path;

use rusqlite::Connection;
use serde_json::{Value, json};

use crate::error::{Error, Result};

mod call;
mod deliver;
mod schema;
mod skills;
#[cfg(test)]
mod tests;
mod tools;

use skills::{get_prompt, list_prompts};
pub use tools::run_tool;

const PROTOCOL_VERSION: &str = "2024-11-05";
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

/// Serves the spaces of the application at `app_dir` on stdin/stdout until
/// the client closes the stream.
///
/// A call is served from the space kilna last opened - looked up at every
/// call, so the server follows the window - or from the one its `space`
/// argument names (ADR 0077). Each space's database is opened once, the first
/// time a call needs it: a session is a long conversation, and re-opening the
/// file for every call would only add failure modes. It is opened as a
/// companion of the window: no hold is taken, no run is swept, and a space
/// another machine holds is served for reading only. The client's name, from
/// `initialize`, is remembered so its proposals land in a chat that says who
/// made them.
pub fn serve(app_dir: &Path) -> Result<()> {
    let app = crate::db::open_app(&app_dir.join(crate::db::APP_DATABASE_FILE))?;
    crate::space::legacy::adopt(app_dir, &app)?;
    if crate::space::registry::newest(&app, app_dir)?.is_none() {
        crate::state::first_space(&app, app_dir)?;
    }
    let mut book = Book {
        app,
        app_dir: app_dir.to_path_buf(),
        open: BTreeMap::new(),
        blank: Connection::open_in_memory()?,
    };

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut line = String::new();
    let mut session = Session::default();

    loop {
        line.clear();
        if stdin.lock().read_line(&mut line)? == 0 {
            return Ok(());
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(err) => {
                // A line that is not JSON has no id to answer against, so
                // there is nobody to answer; saying so on stderr is all a
                // server can do without corrupting the stream.
                crate::log::warn("mcp", &format!("ignoring a line that is not JSON: {err}"));
                continue;
            }
        };
        // A tool and a prompt read a space: the one named, or the one kilna
        // last opened. The rest of the protocol reads none.
        let method = request.get("method").and_then(Value::as_str);
        let calls_a_tool = method == Some("tools/call");
        let reads_a_space = reads_a_space(method);
        let response = if reads_a_space {
            let named = request
                .pointer("/params/arguments/space")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned);
            match book.resolve(named.as_deref()) {
                Ok((conn, space)) => {
                    session.space = Some(space);
                    handle(conn, &mut session, &request)
                }
                Err(err) if calls_a_tool => failed_call(&request, &err),
                Err(err) => failed_request(&request, &err),
            }
        } else {
            handle(&book.blank, &mut session, &request)
        };
        let Some(response) = response else {
            continue;
        };
        writeln!(stdout, "{response}")?;
        stdout.flush()?;
    }
}

/// The spaces a session has opened, by id.
struct Book {
    app: Connection,
    app_dir: std::path::PathBuf,
    open: BTreeMap<String, Connection>,
    /// What a message that reads no space - `initialize`, `ping`, the list of
    /// tools - is answered with.
    blank: Connection,
}

impl Book {
    /// The space a call is served from, opened when it is not yet.
    fn resolve(&mut self, named: Option<&str>) -> Result<(&Connection, SpaceInfo)> {
        use crate::space::registry;
        let space = match named {
            Some(name) => registry::by_name(&self.app, &self.app_dir, name)?,
            None => registry::newest(&self.app, &self.app_dir)?
                .ok_or_else(|| Error::refused("space.none"))?,
        };
        if !space.reachable {
            return Err(Error::refused("space.unreachable").param("path", space.path.clone()));
        }
        let listed: Vec<crate::space::Space> = registry::list(&self.app, &self.app_dir, None)?
            .into_iter()
            .filter(|other| other.id != space.id && other.trashed_at.is_none())
            .collect();
        // Where each neighbour keeps its database, for `calendar`'s
        // `cross_spaces` (v0.151): read only, never opened for writing.
        let mut neighbours = Vec::new();
        for other in listed.iter().filter(|other| other.reachable) {
            if let Ok(folder) = registry::folder(&self.app, &self.app_dir, &other.id) {
                neighbours.push((other.name.clone(), folder.database()));
            }
        }
        let others: Vec<String> = listed.into_iter().map(|other| other.name).collect();
        if !self.open.contains_key(&space.id) {
            let folder = registry::folder(&self.app, &self.app_dir, &space.id)?;
            let (conn, held_by) = crate::state::open_space(&folder, crate::state::Role::Companion)?;
            if let Some(holder) = held_by {
                crate::log::info(
                    "mcp",
                    &format!(
                        "{} is held by {}: served for reading",
                        space.name, holder.machine
                    ),
                );
            }
            self.open.insert(space.id.clone(), conn);
        }
        let folder = registry::folder(&self.app, &self.app_dir, &space.id)?;
        let held_by = match crate::space::lock::check(&folder) {
            crate::space::lock::Hold::Elsewhere(holder) => Some(holder.machine),
            crate::space::lock::Hold::Ours => None,
        };
        let info = SpaceInfo {
            name: space.name,
            kind: space.kind.unwrap_or_default(),
            read_only_held_by: held_by,
            others,
            neighbours,
        };
        let conn = self
            .open
            .get(&space.id)
            .ok_or_else(|| Error::Internal("a space opened is not in the book".into()))?;
        Ok((conn, info))
    }
}

/// A tool called in a space that cannot be served answers the agent with the
/// reason, as a tool that fails does.
/// Whether a request is answered from a space - opened by the server loop -
/// rather than from nothing. Found live in v0.127: the prompts were answered
/// from nothing and every one failed, while every test, handed a space,
/// passed.
fn reads_a_space(method: Option<&str>) -> bool {
    matches!(method, Some("tools/call" | "prompts/list" | "prompts/get"))
}

/// A prompt asked of a space that cannot be served: a protocol error, since
/// a prompt has no result to carry the reason in.
fn failed_request(request: &Value, err: &Error) -> Option<Value> {
    let id = request.get("id")?.clone();
    Some(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": INVALID_PARAMS, "message": err.to_string() },
    }))
}

fn failed_call(request: &Value, err: &Error) -> Option<Value> {
    let id = request.get("id")?.clone();
    Some(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": { "content": [{ "type": "text", "text": err.to_string() }], "isError": true },
    }))
}
/// What one connection knows about the client on the other end.
#[derive(Debug, Default)]
pub struct Session {
    /// The client's name from `initialize` — "Claude Code", say. Names the
    /// chat its proposals go into.
    pub client: Option<String>,
    /// The space the call at hand is served from, as `workspace` names it.
    pub space: Option<SpaceInfo>,
}

/// The space a call is served from, and the others an agent may name.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SpaceInfo {
    pub name: String,
    /// The kind it was made from: `music` (Studio), `blog`, `novel`, `podcast`.
    pub kind: String,
    /// The machine that holds it, when another does: proposals are refused.
    pub read_only_held_by: Option<String>,
    /// Every other space, by name: what `space` may name.
    pub others: Vec<String>,
    /// The other spaces that can be read, each with its database: what
    /// `calendar`'s `cross_spaces` reads, and nothing else.
    #[serde(skip)]
    pub neighbours: Vec<(String, std::path::PathBuf)>,
}

/// Answers one message, or `None` when it is a notification.
pub fn handle(conn: &Connection, session: &mut Session, request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

    // Notifications carry no id and expect nothing back.
    id.as_ref()?;
    let id = id.unwrap_or(Value::Null);

    let result = match method {
        "initialize" => {
            session.client = params
                .pointer("/clientInfo/name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned);
            Ok(initialize())
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": schema::tools() })),
        "tools/call" => call_tool(conn, session, &params),
        // The profile's skills as the client's own prompts - slash commands
        // in the clients that offer them (FR-17).
        "prompts/list" => list_prompts(conn),
        "prompts/get" => get_prompt(conn, &params),
        other => Err(Failure {
            code: METHOD_NOT_FOUND,
            message: format!("kilna does not serve `{other}`"),
        }),
    };

    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(failure) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": failure.code, "message": failure.message },
        }),
    })
}

/// A protocol-level failure: the request itself was wrong. A tool that runs
/// and fails is not one of these — it answers with its own text, so that the
/// agent reads what went wrong instead of the client swallowing it.
struct Failure {
    code: i64,
    message: String,
}

fn initialize() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {}, "prompts": { "listChanged": false } },
        "serverInfo": { "name": "kilna", "version": env!("CARGO_PKG_VERSION") },
        "instructions": "The workspace of a content maker, kept in spaces - a channel, a blog, a novel, \
    each its own folder and its own works: every tool answers from the space kilna last opened, or \
    from the one its `space` argument names, and `workspace` names the space and lists the others. \
    In it: works, their versions, scores along the \
    craft's axes, releases in a calendar, notes. Start with `workspace` to learn the craft's \
    vocabulary (roles, axes, fields, statuses), then `catalogue` and `work` to find and read. You \
    cannot write: you PROPOSE, and the person applies it in kilna with one click, or does not. \
    `propose_work` proposes a whole new work — title, kind, overview fields, versions by role, a \
    score, notes, a storyboard, and the releases it ships as with the text each goes out under — \
    or, given `work`, a package of those for an existing one; prefer it whenever you have more \
    than one thing to say about a work, so the person applies it all at once. \
    `propose_version`, `propose_score` and `propose_note` propose one thing each; \
    `propose_scenes` proposes a storyboard for a video or a short, added to the board or \
    replacing it. A song never goes out itself: its clip, its audio release and its shorts do, \
    each a work made from it with releases of its own, and `propose_release` proposes what one \
    of those releases goes out under - its title, description, tags, pinned comment. `cover` \
    reads the board of ideas for a publication's cover and everything an idea is built from, \
    and `propose_cover` proposes ideas onto it. An experiment is a work whose kind is a lab: \
    `trials` reads its board - trials by series, each with what it moves, its text, what came \
    out and the verdict - and `propose_trials` proposes trials onto it; a kept trial becomes a \
    version of a song's style in the window. A work may be prepared before it is written - a \
    brief, the idea it was refined into, boards of scenarios: `brief` reads the whole order a \
    writer works from, and `propose_cards` proposes cards onto one of its boards. `canon` \
    reads the world the works share - cards of people, places and the channel, with facts in \
    layers - and `propose_canon` proposes cards, facts and relations for it. The material for \
    works to come is two shelves of notes: phrases - a word, a line, a quote, each one line, \
    sorted onto the owner's shelves - and notes, a thought from a seed to a page; `notes` reads \
    them, a phrase with its shelves, how many works say it and how it is sung (the stress as a \
    capital vowel), and `propose_note` proposes one. `propose_words` proposes words for the \
    record - a phrase to keep, ways of singing, terms for the register and the works a meaning \
    is in. `styles` reads the dictionary a prompt is built from, and `propose_style` proposes \
    styles for it - new ones and rewrites, with when each can go into a picture. Name \
    works by id when you have one; an exact title works too. \
    The methods of the craft, what a work is judged by and the rules of its world live in \
    kilna, never in your own files: `workspace` lists the skills of this profile under \
    `guides`, `guide` hands you a skill's method with the live context of the space, and \
    `canon` and `notes` hold the world and the words. Read them each time rather than \
    remembering them.",
    })
}

fn call_tool(
    conn: &Connection,
    session: &Session,
    params: &Value,
) -> std::result::Result<Value, Failure> {
    let name = params.get("name").and_then(Value::as_str).ok_or(Failure {
        code: INVALID_PARAMS,
        message: "tools/call needs a `name`".into(),
    })?;
    let args = params
        .get("arguments")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    // A tool that fails answers the agent, not the client: `isError` keeps
    // the reason in the conversation, where the agent can act on it.
    Ok(match run_tool(conn, session, name, &args) {
        Ok(text) => json!({ "content": [{ "type": "text", "text": text }] }),
        Err(err) => {
            json!({ "content": [{ "type": "text", "text": err.to_string() }], "isError": true })
        }
    })
}

/// The command that registers this build with Claude Code.
///
/// Shown in the settings so nobody has to find the executable by hand; the
/// path is this process's own, quoted, because application directories have
/// spaces in them. User scope, because the promise is a session anywhere on
/// the machine and Claude Code's default scope is the current project only.
pub fn registration_command() -> Result<String> {
    let exe = std::env::current_exe()?;
    Ok(format!(
        "claude mcp add -s user kilna -- \"{}\" --mcp",
        exe.display()
    ))
}
