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

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Write};
use std::path::Path;

use rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::assistant::apply;
use crate::assistant::proposal::{self, BoardChange, PackagedNote, PackagedVersion, Proposal};
use crate::error::{Error, Result};
use crate::link;
use crate::note::{self, NoteFilter};
use crate::profile::config::SectionShape;
use crate::scene;
use crate::speech::Speech;
use crate::work;
use crate::work::version;
use crate::{profile, release, score, search};

mod call;
mod deliver;
mod schema;
mod skills;
#[cfg(test)]
mod tests;

use call::{MOST, Page, active, arg, find_work, length_of, pretty, required};
use deliver::deliver;
use schema::{section_schema, tools};
use skills::{get_prompt, guides, list_prompts, skill_document};

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
        "tools/list" => Ok(json!({ "tools": tools() })),
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

/// The bricks an agent named for a composed text, by id or exact name - a
/// phrase is also its name - each of the composition's types. A name two
/// bricks share is refused with their ids, as a title is.
fn brick_ids(
    conn: &Connection,
    profile_id: &str,
    composition: &crate::profile::config::Composition,
    named: &[String],
) -> Result<Vec<String>> {
    let bricks: Vec<crate::style_brick::StyleBrick> =
        crate::style_brick::list(conn, profile_id, &Default::default())?
            .into_iter()
            .filter(|brick| composition.types().any(|key| key == brick.type_key))
            .collect();
    let mut ids = Vec::with_capacity(named.len());
    for name in named {
        if let Some(brick) = bricks.iter().find(|b| b.id == *name) {
            ids.push(brick.id.clone());
            continue;
        }
        let key = crate::phrase::key_of(name);
        let matching: Vec<&crate::style_brick::StyleBrick> = bricks
            .iter()
            .filter(|b| {
                crate::phrase::key_of(&b.name) == key
                    || b.description
                        .as_deref()
                        .is_some_and(|d| crate::phrase::key_of(d) == key)
            })
            .collect();
        match matching.as_slice() {
            [one] => ids.push(one.id.clone()),
            [] => return Err(Error::refused("mcp.unknownBrick").param("named", name.clone())),
            many => {
                return Err(Error::refused("mcp.ambiguousBrick")
                    .param("named", name.clone())
                    .param(
                        "ids",
                        many.iter()
                            .map(|b| b.id.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    ));
            }
        }
    }
    Ok(ids)
}

/// The dictionary as `styles` reads it.
fn read_styles(
    conn: &Connection,
    profile_id: &str,
    config: &crate::profile::config::ProfileConfig,
    args: &Map<String, Value>,
) -> Result<String> {
    let all = crate::style_brick::list(conn, profile_id, &Default::default())?;
    let type_key = arg(args, "type");
    let composition = match arg(args, "composition") {
        Some(key) => Some(
            config
                .composition(key)
                .ok_or_else(|| Error::not_found("composition", key))?,
        ),
        None => None,
    };
    let query = arg(args, "query").map(str::to_lowercase);
    let branch = arg(args, "branch");
    if type_key.is_none() && composition.is_none() && query.is_none() && branch.is_none() {
        let types: Vec<Value> = config
            .style_types
            .iter()
            .map(|style| {
                json!({
                    "type": style.key, "label": style.label, "form": style.form,
                    "branch": config.branch_of(&style.key),
                    "retired": style.retired.is_some(),
                    "bricks": all.iter().filter(|b| b.type_key == style.key).count(),
                })
            })
            .collect();
        return pretty(&json!({ "types": types, "compose": config.compose }));
    }
    let house = crate::phrase::house(conn, profile_id)?;
    let mut rows: Vec<&crate::style_brick::StyleBrick> = all
        .iter()
        .filter(|b| type_key.is_none_or(|key| b.type_key == key))
        .filter(|b| branch.is_none_or(|branch| config.branch_of(&b.type_key) == branch))
        .filter(|b| composition.is_none_or(|c| c.types().any(|key| key == b.type_key)))
        .filter(|b| {
            query.as_ref().is_none_or(|q| {
                b.name.to_lowercase().contains(q)
                    || b.description
                        .as_deref()
                        .is_some_and(|d| d.to_lowercase().contains(q))
                    || b.explanation
                        .as_ref()
                        .is_some_and(|l| l.words().iter().any(|w| w.to_lowercase().contains(q)))
            })
        })
        .collect();
    if let Some(composition) = composition {
        let place = |key: &str| {
            composition
                .types()
                .position(|t| t == key)
                .unwrap_or(usize::MAX)
        };
        rows.sort_by_key(|b| place(&b.type_key));
    }
    let out: Vec<Value> = rows
        .into_iter()
        .map(|b| {
            json!({
                "id": b.id, "type": b.type_key, "name": b.name, "family": b.family,
                "phrase": b.description, "explanation": b.explanation, "when": b.when_to_use,
                "house": house.contains(&b.id), "origin": b.origin, "status": b.status,
                "favorite": b.favorite, "branch": config.branch_of(&b.type_key),
            })
        })
        .collect();
    pretty(&out)
}

pub fn run_tool(
    conn: &Connection,
    session: &Session,
    name: &str,
    args: &Map<String, Value>,
) -> Result<String> {
    let profile = active(conn)?;
    let config = profile::config_for(conn, &profile.id)?;

    match name {
        "workspace" => {
            let works: i64 = conn.query_row(
                "SELECT count(*) FROM work WHERE profile_id = ?1",
                [&profile.id],
                |row| row.get(0),
            )?;
            // One entry per kind, each with its own vocabulary: the keys an
            // agent scores or proposes in belong to the work's kind.
            let fields: Vec<Value> = config
                .work_meta_fields
                .iter()
                .map(|f| {
                    let mut field = json!({
                        "key": f.key, "label": f.label, "type": f.field_type,
                        "kinds": f.kinds, "options": f.options,
                    });
                    // A numbered field is given by kilna when a work is made:
                    // an agent that proposes a work leaves it out.
                    if let Some(first) = &f.numbered_from {
                        field["numbered_from"] = json!(first);
                    }
                    field
                })
                .collect();
            let kinds: Vec<Value> = config
                .work_kinds
                .iter()
                .map(|kind| {
                    json!({
                        "key": kind.key,
                        "label": kind.label,
                        "version_roles": kind.version_roles,
                        // Whole: the rubric's marks are the anchors a judge
                        // places a work against (FR-17).
                        "axes": kind.axes,
                        "tiers": kind.tiers,
                        "statuses": kind.statuses.iter().map(|s| json!({ "key": s.key, "label": s.label })).collect::<Vec<_>>(),
                        "release_kinds": kind.release_kinds.iter().map(|k| json!({
                            "key": k.key, "label": k.label, "cover_format": k.cover_format,
                            "fields": k.fields.iter().map(|f| json!({
                                "key": f.key, "label": f.label, "type": f.field_type,
                                "hint": f.hint, "limit": f.limit,
                            })).collect::<Vec<_>>(),
                        })).collect::<Vec<_>>(),
                        "goes_out": kind.has_doors(),
                        "shot_types": kind.shot_types,
                        "layers": kind.layers,
                        "scene_blocks": kind.scene_blocks,
                        "cover": kind.cover,
                        "frame": kind.frame,
                        "lab": kind.lab,
                        "conception": kind.conception,
                    })
                })
                .collect();
            let style_types: Vec<Value> = config
                .style_types
                .iter()
                .filter(|style| style.retired.is_none())
                .map(|style| {
                    json!({
                        "key": style.key, "label": style.label, "form": style.form,
                        "hint": style.hint,
                        "families": style.families.iter().map(|f| json!({ "key": f.key, "label": f.label })).collect::<Vec<_>>(),
                    })
                })
                .collect();
            // The kinds of note a work's material is kept as - a phrase, a
            // note (ADR 0064) - and the cards of the canon apart: those are
            // read through `canon`.
            let note_kinds: Vec<Value> = config
                .note_kinds
                .iter()
                .filter(|kind| !kind.is_card())
                .map(|kind| {
                    let mut said = json!({
                        "key": kind.key, "label": kind.label,
                        "line": kind.line, "material": kind.material,
                    });
                    if kind.library {
                        said["library"] = json!(true);
                        said["media"] = json!(kind.media);
                    }
                    said
                })
                .collect();
            pretty(&json!({
                "space": session.space,
                "profile": { "key": profile.key, "name": profile.name, "description": profile.description },
                "works": works,
                "fields": fields,
                "work_kinds": kinds,
                "note_kinds": note_kinds,
                "style_types": style_types,
                "compose": config.compose,
                "boards": config.boards,
                // The three lines a work's feel is said along (FR-02).
                "vibe": config.vibe(),
                // The skills of the profile, without their methods: what
                // `guide` opens one by one (FR-17).
                "guides": guides(&config, None, None),
            }))
        }

        "guide" => {
            let kind = arg(args, "kind");
            let scope = arg(args, "scope");
            match arg(args, "skill") {
                None => pretty(&json!({ "guides": guides(&config, kind, scope) })),
                Some(key) => {
                    let skill = config
                        .prompts
                        .iter()
                        .find(|p| p.key == key)
                        .ok_or_else(|| {
                            Error::refused("mcp.unknownSkill")
                                .param("skill", key)
                                .param(
                                    "known",
                                    config
                                        .prompts
                                        .iter()
                                        .map(|p| p.key.as_str())
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                )
                        })?;
                    pretty(&skill_document(&config, skill, kind))
                }
            }
        }

        "catalogue" => {
            let query = arg(args, "query").map(|q| q.to_lowercase());
            let kind = arg(args, "kind");
            let status = arg(args, "status");
            let felt = [
                ("mood", arg(args, "mood")),
                ("character", arg(args, "character")),
                ("complexity", arg(args, "complexity")),
            ];
            let page = Page::of(args, 100);
            // The guard's mark, by work (ADR 0054): what a song that has not
            // gone out shares with songs that have.
            let today = crate::time::now();
            let marks: std::collections::HashMap<String, crate::register::guard::RepeatMark> =
                crate::register::guard::marks(conn, &profile.id, &today[..10])?
                    .into_iter()
                    .map(|mark| (mark.work_id.clone(), mark))
                    .collect();
            let rows: Vec<Value> = score::catalogue(conn, &profile.id)?
                .into_iter()
                .filter(|row| query.as_ref().is_none_or(|q| row.title.to_lowercase().contains(q)))
                .filter(|row| kind.is_none_or(|k| row.kind == k))
                .filter(|row| status.is_none_or(|s| row.status == s))
                .filter(|row| {
                    felt.iter().all(|(line, wanted)| {
                        wanted.is_none_or(|wanted| {
                            let shown = row.vibe.as_ref().map(|shown| &shown.vibe);
                            let said = shown.and_then(|vibe| match *line {
                                "mood" => vibe.mood.as_deref(),
                                "character" => vibe.character.as_deref(),
                                _ => vibe.complexity.as_deref(),
                            });
                            said == Some(wanted)
                        })
                    })
                })
                .map(|row| if page.summary { json!({
                    "id": row.work_id, "title": row.title, "kind": row.kind, "status": row.status,
                    "updated_at": row.updated_at,
                }) } else { json!({
                    "id": row.work_id, "title": row.title, "kind": row.kind, "status": row.status,
                    "total": row.total, "tier": row.tier, "tier_pinned": row.tier_pinned,
                    "stale": row.stale, "released": row.released, "scheduled": row.scheduled,
                    "scored_at": row.scored_at, "updated_at": row.updated_at,
                    "stage": row.stage,
                    // What it feels like, its own or taken from what it was
                    // made from (`from`), when anybody has said (FR-02).
                    "vibe": row.vibe,
                    "repeats": marks.get(&row.work_id).map(|mark| json!({
                        "level": mark.level, "count": mark.count, "word": mark.top.word,
                        "with": mark.top.neighbour_title, "day": mark.top.day,
                    })),
                }) })
                .collect();
            pretty(&page.cut("works", rows))
        }

        "work" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            let versions = version::list(conn, &found.id)?;
            let mut by_role: BTreeMap<String, Vec<Value>> = BTreeMap::new();
            for v in versions {
                by_role.entry(v.role.clone()).or_default().push(json!({
                    "id": v.id, "revision": v.revision, "label": v.label, "length": v.length,
                    "is_current": v.is_current, "is_main": v.is_main,
                    "created_at": v.created_at, "parent": v.parent_version_id,
                }));
            }
            let latest = score::latest(conn, &found.id)?;
            let releases = release::for_work(conn, &profile.id, &found.id)?;
            let links = link::for_work(conn, &found.id)?;
            let notes: i64 = conn.query_row(
                "SELECT count(*) FROM note WHERE work_id = ?1",
                [&found.id],
                |row| row.get(0),
            )?;
            let scenes = scene::count(conn, &found.id)?;
            let vocabulary = config.vocabulary(&found.kind);
            let frame = if vocabulary.frame {
                let view = crate::cover::read::frame_view(conn, &found.id)?;
                Some(json!({
                    "still": view.prompts.still, "loop": view.prompts.loop_,
                    "negative": view.prompts.negative, "built_from_cover": view.from_cover,
                    "motion": found.frame.motion, "seconds": found.frame.seconds(),
                    "still_camera": found.frame.still_camera, "seamless": found.frame.seamless,
                }))
            } else {
                None
            };
            // The cover as a generator gets it - the prompt written from the
            // concept - beside the concept itself.
            let cover = if vocabulary.cover || found.cover.holds_anything() {
                let view = crate::cover::read::view(conn, &found.id, None)?;
                Some(json!({
                    "prompt": view.prompts, "format": view.format, "title": view.title,
                    "concept": found.cover,
                }))
            } else {
                None
            };
            let lab = config.lab(&found.kind).is_some();
            let publications = if vocabulary.has_doors() || lab {
                None
            } else {
                Some(crate::publication::of(conn, &config, &found.id)?)
            };
            let trials = if lab {
                Some(crate::lab::trial::counts(conn, &found.id)?)
            } else {
                None
            };
            pretty(&json!({
                "id": found.id, "title": found.title, "kind": found.kind, "status": found.status,
                "meta": found.meta, "tags": found.tags, "marks": found.marks,
                // How finished the author says it is, 0..=100. Worth telling an
                // agent: a full lyric at stage 20 is a sketch to build on, not
                // a work to polish, and nothing else in this shape says so.
                "stage": found.stage,
                // What it feels like (FR-02): its own, as said for it, and the
                // one it shows - its own, or that of what it was made from.
                "vibe": found.vibe,
                "shown_vibe": crate::work::shown_vibes(conn, &profile.id)?.remove(&found.id),
                "tier_pinned": found.tier_pinned, "tier_pin_reason": found.tier_pin_reason,
                // The day it is planned for before anything of it can go out
                // (v0.138), when it is.
                "work_plan": crate::work_plan::of_work(conn, &found.id)?
                    .filter(|plan| !crate::work_plan::closed(conn, plan, config.plan_window()).unwrap_or(true))
                    .map(|plan| json!({ "id": plan.id, "scheduled_at": plan.scheduled_at, "note": plan.note })),
                "created_at": found.created_at, "updated_at": found.updated_at,
                "versions": by_role,
                "latest_score": latest,
                "releases": releases,
                "notes": notes,
                "scenes": scenes,
                "cover": cover,
                "frame": frame,
                "publications": publications,
                "trials": trials,
                "sources": links.sources.iter().map(|l| json!({
                    "work": l.source_id, "title": l.source_title, "kind": l.source_kind, "role": l.role,
                    "taken_at_version": l.source_version_id, "drifted": l.drifted,
                })).collect::<Vec<_>>(),
                "derived": links.derived.iter().map(|d| json!({
                    "work": d.work_id, "title": d.title, "kind": d.kind, "status": d.status, "role": d.role,
                })).collect::<Vec<_>>(),
            }))
        }

        "text" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            let body = match arg(args, "version") {
                Some(id) => {
                    let v =
                        version::get(conn, id)?.ok_or_else(|| Error::not_found("version", id))?;
                    if v.work_id != found.id {
                        return Err(Error::refused("mcp.versionOtherWork").param("version", id));
                    }
                    v
                }
                None => {
                    let vocabulary = config.vocabulary(&found.kind);
                    let role = match arg(args, "role") {
                        Some(role) => role.to_owned(),
                        None => vocabulary
                            .version_roles
                            .first()
                            .map(|r| r.key.clone())
                            .ok_or_else(|| {
                                Error::refused("mcp.kindNoRoles").param("kind", found.kind.clone())
                            })?,
                    };
                    if !vocabulary.version_roles.iter().any(|r| r.key == role) {
                        return Err(Error::refused("mcp.unknownVersionRole")
                            .param("role", role.clone())
                            .param("kind", found.kind.clone()));
                    }
                    // The version that stands for the role: the current one
                    // when it is of this role, the one marked main, otherwise
                    // the newest - the same answer the panel gives.
                    version::for_role(conn, &found.id, &role)?.ok_or_else(|| {
                        Error::refused("mcp.noVersionYet")
                            .param("title", found.title.clone())
                            .param("role", role.clone())
                    })?
                }
            };
            let reads = config
                .vocabulary(&found.kind)
                .version_roles
                .iter()
                .find(|r| r.key == body.role)
                .map_or("plain", |r| {
                    if r.reads_as_markdown() {
                        "markdown"
                    } else {
                        "plain"
                    }
                });
            pretty(&json!({
                "version": body.id, "role": body.role, "revision": body.revision, "label": body.label,
                "reads_as": reads, "created_at": body.created_at, "body": body.body,
            }))
        }

        "scores" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            pretty(&score::history(conn, &found.id)?)
        }

        "calendar" => {
            let from = arg(args, "from").map(str::to_owned);
            let releases: Vec<release::ScheduledRelease> = release::calendar(conn, &profile.id)?
                .into_iter()
                .filter(|r| {
                    from.as_deref().is_none_or(|day| {
                        r.release
                            .scheduled_at
                            .as_deref()
                            .is_some_and(|at| at >= day)
                    })
                })
                .collect();
            // A span still running on `from` is part of what lies ahead.
            let closed: Vec<crate::time_off::TimeOff> = crate::time_off::list(conn, &profile.id)?
                .into_iter()
                .filter(|span| {
                    from.as_deref()
                        .is_none_or(|day| span.ends_on.as_str() >= day)
                })
                .collect();
            // The planned slots (v0.138): a work meant for a day before
            // anything of it can go out.
            let window = config.plan_window();
            let mut plans = Vec::new();
            for plan in crate::work_plan::open(conn, &profile.id, window)? {
                if from
                    .as_deref()
                    .is_some_and(|day| plan.scheduled_at.as_str() < day)
                {
                    continue;
                }
                let Some(work) = work::get(conn, &plan.work_id)? else {
                    continue;
                };
                let made = crate::link::derived(conn, &work.id)?.len();
                plans.push(json!({
                    "id": plan.id, "work_id": work.id, "title": work.title, "kind": work.kind,
                    "stage": work.stage, "scheduled_at": plan.scheduled_at, "note": plan.note,
                    "derivative_count": made,
                }));
            }
            // The neighbours on the same days (v0.151, FR-20 20.2).
            let mut days: BTreeSet<String> = releases
                .iter()
                .filter_map(|r| r.release.scheduled_at.as_deref())
                .map(|at| at.get(..10).unwrap_or(at).to_owned())
                .collect();
            days.extend(
                plans
                    .iter()
                    .filter_map(|plan| plan["scheduled_at"].as_str().map(str::to_owned)),
            );
            let wanted: Option<Vec<String>> = match args.get("cross_spaces") {
                Some(Value::Bool(true)) => Some(Vec::new()),
                Some(Value::Array(names)) => Some(
                    names
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|name| name.trim().to_owned())
                        .collect(),
                ),
                _ => None,
            };
            let mut answer = json!({ "releases": releases, "closed": closed, "work_plans": plans });
            if let Some(wanted) = wanted {
                let neighbours = session
                    .space
                    .as_ref()
                    .map(|space| space.neighbours.clone())
                    .unwrap_or_default();
                let mut found = Vec::new();
                for (name, database) in neighbours
                    .iter()
                    .filter(|(name, _)| wanted.is_empty() || wanted.contains(name))
                {
                    found.extend(neighbour_days(name, database, &days)?);
                }
                answer["cross_spaces"] = Value::Array(found);
            }
            pretty(&answer)
        }

        "propose_batch_prompts" => {
            let collection_id = match arg(args, "collection") {
                Some(named) => {
                    let named = named.trim();
                    let found = crate::collection::list(conn, &profile.id)?
                        .into_iter()
                        .find(|one| one.id == named || one.title.trim() == named)
                        .ok_or_else(|| Error::not_found("collection", named))?;
                    Some(found.id)
                }
                None => None,
            };
            let month = arg(args, "month").map(|month| month.trim().to_owned());
            if let Some(month) = &month {
                crate::period::month_of(month)?;
            }
            // The bank's items by name, case aside, across the space's banks.
            let mut items_by_name: Vec<(String, String)> = Vec::new();
            for kind in config.note_kinds.iter().filter(|kind| kind.is_card()) {
                for section in kind
                    .sections
                    .iter()
                    .filter(|section| section.shape == crate::profile::config::SectionShape::Bank)
                {
                    for card in crate::note::list(
                        conn,
                        &profile.id,
                        &NoteFilter {
                            kind: Some(kind.key.clone()),
                            ..NoteFilter::default()
                        },
                    )? {
                        for fact in crate::canon::fact::in_section(conn, &card.id, &section.key)? {
                            items_by_name.push((fact.body.trim().to_lowercase(), fact.id));
                        }
                    }
                }
            }
            let mut items = Vec::new();
            let mut left_out: Vec<String> = Vec::new();
            let mut body = String::new();
            for raw in args
                .get("works")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let raw = raw.as_object().cloned().unwrap_or_default();
                let found = find_work(conn, &profile.id, required(&raw, "work")?)?;
                let (fields, unknown) = proposal::fields_from(
                    raw.get("fields")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default(),
                    &config,
                );
                left_out.extend(
                    unknown
                        .into_iter()
                        .map(|key| format!("{} ({key})", found.title)),
                );
                let mut versions = Vec::new();
                for version in raw
                    .get("versions")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let version = version.as_object().cloned().unwrap_or_default();
                    let role = required(&version, "role")?;
                    config.require_role(&found.kind, role)?;
                    versions.push(PackagedVersion {
                        role: role.to_owned(),
                        body: required(&version, "body")?.to_owned(),
                        label: None,
                        from: None,
                    });
                }
                let mut bank = Vec::new();
                for name in raw
                    .get("bank")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let name = name.as_str().unwrap_or_default().trim();
                    match items_by_name
                        .iter()
                        .find(|(known, _)| *known == name.to_lowercase())
                    {
                        Some((_, id)) => bank.push(id.clone()),
                        None => left_out.push(format!("{} (bank: {name})", found.title)),
                    }
                }
                let title = arg(&raw, "title").map(str::to_owned);
                body.push_str(&format!(
                    "### {}\n",
                    title.as_deref().unwrap_or(&found.title)
                ));
                for (key, value) in &fields {
                    let said = value
                        .as_str()
                        .map_or_else(|| value.to_string(), str::to_owned);
                    body.push_str(&format!("- {key}: {said}\n"));
                }
                for version in &versions {
                    body.push_str(&format!(
                        "\n{}:\n\n```\n{}\n```\n",
                        version.role, version.body
                    ));
                }
                body.push('\n');
                items.push(proposal::MonthItem {
                    work_id: found.id.clone(),
                    work_title: found.title.clone(),
                    title,
                    fields,
                    versions,
                    bank,
                });
            }
            if items.is_empty() {
                return Err(Error::refused("proposal.monthEmpty"));
            }
            let mut terms = Vec::new();
            for raw in args
                .get("register")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let raw = raw.as_object().cloned().unwrap_or_default();
                let word = required(&raw, "word")?.trim().to_owned();
                let scope = match arg(&raw, "scope") {
                    None => crate::register::TermScope::Lyrics,
                    Some(said) => serde_json::from_value(Value::String(said.to_owned()))
                        .map_err(|_| Error::refused("mcp.badTermScope").param("scope", said))?,
                };
                terms.push(proposal::MonthTerm {
                    word,
                    scope,
                    note: arg(&raw, "note").map(str::to_owned),
                });
            }
            let count = items.len();
            deliver(
                conn,
                &profile.id,
                session,
                None,
                Proposal::Month {
                    collection_id,
                    month,
                    items,
                    terms,
                },
                body.trim_end(),
                None,
            )?;
            let mut said = format!(
                "Proposed {count} works of the month in one proposal. It waits in the chat named after you, with the checklist the quotas give; one click applies all of it."
            );
            if !left_out.is_empty() {
                said.push_str(&format!(" Left out: {}.", left_out.join("; ")));
            }
            Ok(said)
        }

        "propose_media" => {
            let raw = args
                .get("items")
                .and_then(Value::as_array)
                .ok_or_else(|| Error::refused("mcp.missingArg").param("key", "items"))?;
            let mut items = Vec::new();
            for one in raw {
                let one = one
                    .as_object()
                    .ok_or_else(|| Error::refused("mcp.missingArg").param("key", "items"))?;
                let work = find_work(conn, &profile.id, required(one, "work")?)?;
                let path = required(one, "path")?.trim().to_owned();
                let said = required(one, "role")?;
                let role: crate::actions::media::MediaRole =
                    serde_json::from_value(Value::String(said.to_owned()))
                        .map_err(|_| Error::refused("mcp.badMediaRole").param("role", said))?;
                let file = std::path::Path::new(&path);
                let name = file
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if !file.is_absolute() || !file.is_file() {
                    return Err(Error::refused("media.notAFile").param("path", path));
                }
                if !role.fits(&name) {
                    return Err(Error::refused("media.wrongKind")
                        .param("name", name)
                        .param("role", said));
                }
                items.push(proposal::MediaItem {
                    work_id: work.id,
                    work_title: work.title,
                    path,
                    role,
                    rename: one
                        .get("rename_to_work")
                        .and_then(Value::as_bool)
                        .unwrap_or(true),
                    reason: arg(one, "reason").map(str::to_owned),
                    confidence: one
                        .get("confidence")
                        .and_then(Value::as_f64)
                        .map(|sure| sure.clamp(0.0, 1.0)),
                });
            }
            if items.is_empty() {
                return Err(Error::refused("proposal.nothingTaken"));
            }
            let mut body = String::new();
            for item in &items {
                body.push_str(&format!(
                    "- {} -> {} ({})\n",
                    item.path,
                    item.work_title,
                    serde_json::to_value(item.role)?
                        .as_str()
                        .unwrap_or_default()
                ));
            }
            let count = items.len();
            deliver(
                conn,
                &profile.id,
                session,
                None,
                Proposal::Media { items },
                body.trim_end(),
                None,
            )?;
            Ok(format!(
                "Proposed {count} files to take into their works. They wait in the chat named after you; nothing moves until the person applies it."
            ))
        }

        "quotas" => {
            let work_id = match arg(args, "work") {
                Some(named) => Some(find_work(conn, &profile.id, named)?.id),
                None => None,
            };
            let collection_id = match arg(args, "collection") {
                Some(named) => {
                    let named = named.trim();
                    let found = crate::collection::list(conn, &profile.id)?
                        .into_iter()
                        .find(|one| one.id == named || one.title.trim() == named)
                        .ok_or_else(|| Error::not_found("collection", named))?;
                    Some(found.id)
                }
                None => None,
            };
            let subject = crate::canon::quota::QuotaSubject {
                work_id,
                collection_id,
                month: arg(args, "month").map(str::to_owned),
            };
            pretty(&crate::canon::quota::check(conn, &subject)?)
        }

        "notes" => {
            let work_id = match arg(args, "work") {
                Some(named) => Some(find_work(conn, &profile.id, named)?.id),
                None => None,
            };
            let state =
                match arg(args, "state") {
                    Some(word) => Some(note::NoteState::from_word(word).ok_or_else(|| {
                        Error::refused("mcp.unknownNoteState").param("state", word)
                    })?),
                    None => None,
                };
            let filter = NoteFilter {
                work_id,
                search: arg(args, "query").map(str::to_owned),
                kind: arg(args, "kind").map(str::to_owned),
                state,
                ..NoteFilter::default()
            };
            // One note whole, by its id or its exact title: what a summary
            // of the list is read down to.
            if let Some(named) = arg(args, "note") {
                let found = find_note(conn, &profile.id, named)?;
                let whole = read_notes(
                    conn,
                    &profile.id,
                    &config,
                    &NoteFilter {
                        kind: Some(found.kind.clone()),
                        ..NoteFilter::default()
                    },
                    None,
                )?
                .into_iter()
                .find(|shown| shown["id"] == json!(found.id))
                .map_or_else(|| serde_json::to_value(&found), Ok)?;
                return pretty(&whole);
            }
            let page = Page::of(args, 50);
            let mut notes = read_notes(conn, &profile.id, &config, &filter, arg(args, "shelf"))?;
            if page.summary {
                notes = notes.iter().map(note_summary).collect();
            }
            let mut answer = page.cut("notes", notes);
            // With no question asked, the shelves too: what there is to ask
            // for. Narrowing by a page is no question.
            let asked = args
                .keys()
                .any(|key| !["limit", "offset", "summary", "space"].contains(&key.as_str()));
            if !asked {
                let shelves: Vec<Value> = crate::shelf::views(conn, &profile.id)?
                    .iter()
                    .map(|view| {
                        json!({
                            "id": view.shelf.id, "name": view.shelf.name,
                            "phrases": view.note_ids.len(),
                        })
                    })
                    .collect();
                answer.insert("shelves".into(), Value::Array(shelves));
            }
            pretty(&answer)
        }

        // The register of repeats (ADR 0044): the whole of it, or a text
        // checked against it. The marks are the window's; an agent is given
        // what was found.
        "register" => {
            let text = match (arg(args, "text"), arg(args, "work")) {
                (Some(text), _) => Some(text.to_owned()),
                (None, Some(named)) => {
                    let found = find_work(conn, &profile.id, named)?;
                    let current = match found.current_version_id.as_deref() {
                        Some(id) => version::get(conn, id)?,
                        None => None,
                    };
                    Some(current.map(|v| v.body).unwrap_or_default())
                }
                (None, None) => None,
            };
            match text {
                Some(text) => {
                    let checked = crate::register::check::text(conn, &profile.id, &text, false)?;
                    // For a work, what the guard says too: the songs out or
                    // booked it shares a spent term or a rare word with.
                    let guard = match arg(args, "work") {
                        Some(named) => {
                            let found = find_work(conn, &profile.id, named)?;
                            let today = crate::time::now();
                            crate::register::guard::of_work(conn, &found.id, &today[..10])?
                        }
                        None => None,
                    };
                    pretty(&json!({
                        "terms": checked.terms,
                        "repeats": checked.repeats,
                        "guard": guard,
                    }))
                }
                None => {
                    let entries = crate::register::entries(conn, &profile.id)?;
                    // One word whole, with the works that carry it.
                    if let Some(named) = arg(args, "term") {
                        let entry = entries
                            .iter()
                            .find(|entry| entry.term.id == named)
                            .or_else(|| {
                                entries.iter().find(|entry| {
                                    entry.term.word.to_lowercase() == named.trim().to_lowercase()
                                })
                            })
                            .ok_or_else(|| {
                                Error::refused("mcp.unknownTerm").param("term", named)
                            })?;
                        let mut whole = serde_json::to_value(entry)?;
                        if let Some(object) = whole.as_object_mut() {
                            object.insert(
                                "works".into(),
                                serde_json::to_value(crate::register::uses(conn, &entry.term.id)?)?,
                            );
                        }
                        return pretty(&whole);
                    }
                    let page = Page::of(args, 100);
                    let terms: Vec<Value> = entries
                        .iter()
                        .map(|entry| {
                            if page.summary {
                                Ok(json!({
                                    "id": entry.term.id, "word": entry.term.word,
                                    "kind": entry.term.kind,
                                    "strictness": entry.term.strictness, "uses": entry.uses,
                                }))
                            } else {
                                serde_json::to_value(entry)
                            }
                        })
                        .collect::<std::result::Result<_, _>>()?;
                    pretty(&page.cut("terms", terms))
                }
            }
        }

        "scenes" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            pretty(&scene::for_work(conn, &found.id)?)
        }

        "styles" => read_styles(conn, &profile.id, &config, args),

        "propose_style" => {
            let found = match arg(args, "work") {
                Some(named) => Some(find_work(conn, &profile.id, named)?),
                None => None,
            };
            let raw = args
                .get("styles")
                .cloned()
                .ok_or_else(|| Error::refused("mcp.missingArg").param("key", "styles"))?;
            let package = crate::style_proposal::read(conn, &profile.id, &config, &raw)?;
            if package.is_empty() {
                return Err(Error::refused("styles.noStyles"));
            }
            let body = crate::style_proposal::render(&package);
            let count = package.styles.len();
            let dropped = package.dropped.len();
            deliver(
                conn,
                &profile.id,
                session,
                found.as_ref(),
                Proposal::Styles { package },
                &body,
                arg(args, "note"),
            )?;
            let mut answer = format!(
                "Proposed {count} style{} for the dictionary. It waits in the chat {}; the person keeps it whole or style by style.",
                if count == 1 { "" } else { "s" },
                if found.is_some() {
                    "on the work"
                } else {
                    "named after you"
                }
            );
            if dropped > 0 {
                answer.push_str(&format!(" {dropped} left out - the chat says why."));
            }
            Ok(answer)
        }
        "propose_words" => {
            let found = match arg(args, "work") {
                Some(named) => Some(find_work(conn, &profile.id, named)?),
                None => None,
            };
            let raw = args
                .get("words")
                .cloned()
                .ok_or_else(|| Error::refused("mcp.missingArg").param("key", "words"))?;
            let package = crate::register::proposal::read(conn, &profile.id, &raw)?;
            if package.is_empty() {
                return Err(Error::refused("words.noWords"));
            }
            let body = crate::register::proposal::render(&package);
            let count = package.words.len();
            let dropped = package.dropped.len();
            let changes = package.words.iter().filter(|word| word.known).count();
            let warned = package.warnings.len();
            deliver(
                conn,
                &profile.id,
                session,
                found.as_ref(),
                Proposal::Words { package },
                &body,
                arg(args, "note"),
            )?;
            let mut answer = format!(
                "Proposed {count} word{} for the record. It waits in the chat {}; the person keeps it whole or word by word.",
                if count == 1 { "" } else { "s" },
                if found.is_some() {
                    "on the work"
                } else {
                    "named after you"
                }
            );
            if changes > 0 {
                answer.push_str(&format!(
                    " {changes} of them the record already keeps: the chat says what changes."
                ));
            }
            if warned > 0 {
                answer.push_str(&format!(
                    " {warned} word(s) read as several, written with a comma or a slash: one \
                     `word` is one thing - put synonyms in `forms`, or split them."
                ));
            }
            if dropped > 0 {
                answer.push_str(&format!(" {dropped} item(s) were left out: read the chat."));
            }
            Ok(answer)
        }

        "search" => pretty(&search::find(conn, &profile.id, required(args, "query")?)?),

        "canon" => read_canon(conn, &profile.id, &config, args),

        "propose_canon" => {
            let found = match arg(args, "work") {
                Some(named) => Some(find_work(conn, &profile.id, named)?),
                None => None,
            };
            let defaults = crate::canon::proposal::Defaults {
                card_id: None,
                work_id: found.as_ref().map(|w| w.id.clone()),
                version_id: found.as_ref().and_then(|w| w.current_version_id.clone()),
            };
            let package = crate::canon::proposal::read(
                conn,
                &profile.id,
                &Value::Object(args.clone()),
                &defaults,
            )?;
            let body = apply::render_canon(conn, &package, Speech::ENGLISH);
            let summary = format!(
                "{} card(s), {} card update(s), {} fact(s), {} relation(s), {} picture(s), {} description(s)",
                package.cards.len(),
                package.card_updates.len(),
                package.facts.len(),
                package.links.len(),
                package.pictures.len(),
                package.descriptions.len()
            );
            // A new fact the card nearly holds is said back to the agent
            // (v0.144), beside what it proposed.
            let similar: Vec<String> = package
                .facts
                .iter()
                .enumerate()
                .filter_map(|(index, fact)| {
                    fact.similar_to.as_ref().map(|held| {
                        format!(
                            "fact {} is similar_to {} ({}): {}",
                            index + 1,
                            held.fact_id,
                            held.section,
                            held.text
                        )
                    })
                })
                .collect();
            let dropped: Vec<String> = package
                .dropped
                .iter()
                .map(|reason| {
                    let code = reason.key.strip_prefix("refusal.").unwrap_or(&reason.key);
                    crate::error::english(code, &reason.params)
                })
                .collect();
            deliver(
                conn,
                &profile.id,
                session,
                found.as_ref(),
                Proposal::Canon { package },
                &body,
                None,
            )?;
            let mut answer = format!(
                "Proposed {summary} for the canon. It waits in the chat {}; the person applies it whole or item by item.",
                if found.is_some() {
                    "on the work"
                } else {
                    "named after you"
                }
            );
            if !dropped.is_empty() {
                answer.push_str(&format!(" Left out: {}.", dropped.join("; ")));
            }
            if !similar.is_empty() {
                answer.push_str(&format!(" Marked: {}.", similar.join("; ")));
            }
            Ok(answer)
        }

        "propose_version" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            let role = required(args, "role")?;
            if !config
                .vocabulary(&found.kind)
                .version_roles
                .iter()
                .any(|r| r.key == role)
            {
                return Err(Error::refused("mcp.unknownVersionRole")
                    .param("role", role)
                    .param("kind", found.kind.clone()));
            }
            // A text written out of the dictionary is written by kilna, from
            // the bricks named - the same function the constructor uses - so
            // an agent and a person picking the same phrases get one text.
            let named: Vec<String> = args
                .get("bricks")
                .and_then(Value::as_array)
                .map(|list| {
                    list.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            let written;
            let mut said = String::new();
            let body = if named.is_empty() {
                required(args, "body")?
            } else {
                if arg(args, "body").is_some() {
                    return Err(Error::refused("mcp.bodyAndBricks"));
                }
                let composition = config.composition_for(&found.kind, role).ok_or_else(|| {
                    Error::refused("mcp.roleNotComposed")
                        .param("role", role)
                        .param("kind", found.kind.clone())
                })?;
                let bricks = brick_ids(conn, &profile.id, composition, &named)?;
                let composed = crate::phrase::compose::write(
                    conn,
                    &profile.id,
                    &crate::phrase::compose::ComposeRequest {
                        composition: composition.key.clone(),
                        bricks,
                        work_id: Some(found.id.clone()),
                    },
                )?;
                for problem in &composed.problems {
                    said.push_str(&format!(
                        " `{}`: {} picked, the composition takes {}{}.",
                        problem.type_key,
                        problem.picked,
                        problem.min,
                        problem
                            .max
                            .map(|max| format!("-{max}"))
                            .unwrap_or_else(|| "+".into())
                    ));
                }
                if let Some(limit) = composed.limit
                    && composed.length > limit as usize
                {
                    said.push_str(&format!(
                        " The text is {} characters, past the limit of {limit}.",
                        composed.length
                    ));
                }
                written = composed.text;
                written.as_str()
            };
            // The version rewritten, said now rather than lost at apply time:
            // lineage stays inside a work and a role (ADR 0055).
            let from = arg(args, "from").map(str::to_owned);
            if let Some(from) = &from {
                version::check_line(conn, &found.id, role, from)?;
            }
            let proposal = Proposal::Version {
                role: role.to_owned(),
                label: arg(args, "label").map(str::to_owned),
                from,
            };
            deliver(
                conn,
                &profile.id,
                session,
                Some(&found),
                proposal,
                body,
                arg(args, "note"),
            )?;
            Ok(format!(
                "Proposed a `{role}` version for “{}”. It is in the chat on the work, waiting to be inserted — or not.{said}",
                found.title
            ))
        }

        "propose_score" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            let axes = args
                .get("axes")
                .and_then(Value::as_object)
                .cloned()
                .ok_or_else(|| Error::refused("mcp.axesNotObject"))?;
            let note = arg(args, "note").map(str::to_owned);
            let vocabulary = config.vocabulary(&found.kind);
            let proposal =
                proposal::score_from(axes, note.clone(), vocabulary).ok_or_else(|| {
                    Error::refused("mcp.noKnownAxes")
                        .param("kind", found.kind.clone())
                        .param(
                            "known",
                            vocabulary
                                .axes
                                .iter()
                                .map(|a| a.key.as_str())
                                .collect::<Vec<_>>()
                                .join(", "),
                        )
                })?;
            let summary = match &proposal {
                Proposal::Score {
                    axes,
                    unknown,
                    missing,
                    ..
                } => {
                    let mut s = format!(
                        "Proposed a score for “{}”: {}.",
                        found.title,
                        axes.iter()
                            .map(|(k, v)| format!("{k}={v}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    if !unknown.is_empty() {
                        s.push_str(&format!(
                            " Ignored axes the profile does not have: {}.",
                            unknown.join(", ")
                        ));
                    }
                    if !missing.is_empty() {
                        s.push_str(&format!(" Left unjudged: {}.", missing.join(", ")));
                    }
                    s
                }
                _ => unreachable!("score_from makes scores"),
            };
            let body = note
                .clone()
                .unwrap_or_else(|| "A score, proposed from outside kilna.".into());
            deliver(
                conn,
                &profile.id,
                session,
                Some(&found),
                proposal,
                &body,
                None,
            )?;
            Ok(summary)
        }

        "propose_work" => {
            let found = match arg(args, "work") {
                Some(named) => Some(find_work(conn, &profile.id, named)?),
                None => None,
            };
            let (title, kind) = match &found {
                Some(w) => (None, w.kind.clone()),
                None => {
                    let title = required(args, "title")
                        .map_err(|_| Error::refused("proposal.newWorkNeedsTitle"))?;
                    let kind = required(args, "kind")
                        .map_err(|_| Error::refused("proposal.newWorkNeedsKind"))?;
                    (Some(title.to_owned()), kind.to_owned())
                }
            };
            config.require_kind(&kind)?;
            let vocabulary = config.vocabulary(&kind);

            let (fields, unknown_fields) = proposal::fields_from(
                args.get("fields")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default(),
                &config,
            );

            // What the work feels like (FR-02), in the profile's words: a
            // word it does not have is refused with the ones it has.
            let vibe = match args.get("vibe") {
                None | Some(Value::Null) => None,
                Some(raw) => {
                    let said: crate::work::Vibe = serde_json::from_value(raw.clone())
                        .map_err(|_| Error::refused("mcp.badVibe"))?;
                    let said = said.kept();
                    if let Some(said) = &said {
                        config.vibe().check(said)?;
                    }
                    said
                }
            };

            let mut versions = Vec::new();
            for item in args
                .get("versions")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let item = item.as_object().cloned().unwrap_or_default();
                let role = required(&item, "role")?;
                if !vocabulary.version_roles.iter().any(|r| r.key == role) {
                    return Err(Error::refused("mcp.unknownVersionRole")
                        .param("role", role)
                        .param("kind", kind.clone()));
                }
                // Only a work that exists has versions to have been written
                // from; a new work's text is written from nothing.
                let from = arg(&item, "from").map(str::to_owned);
                if let Some(from) = &from {
                    match &found {
                        Some(work) => version::check_line(conn, &work.id, role, from)?,
                        None => {
                            return Err(Error::refused("version.parentNotSameRole")
                                .param("parent", from.clone())
                                .param("role", role));
                        }
                    }
                }
                versions.push(PackagedVersion {
                    role: role.to_owned(),
                    body: required(&item, "body")?.to_owned(),
                    label: arg(&item, "label").map(str::to_owned),
                    from,
                });
            }

            let score = match args.get("score").and_then(Value::as_object) {
                Some(raw) => {
                    let axes = raw
                        .get("axes")
                        .and_then(Value::as_object)
                        .cloned()
                        .ok_or_else(|| Error::refused("mcp.scoreAxesNotObject"))?;
                    let note = arg(raw, "note").map(str::to_owned);
                    Some(proposal::marks_from(axes, note, vocabulary).ok_or_else(|| {
                        Error::refused("mcp.noKnownScoreAxes")
                            .param("kind", kind.clone())
                            .param(
                                "known",
                                vocabulary
                                    .axes
                                    .iter()
                                    .map(|a| a.key.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            )
                    })?)
                }
                None => None,
            };

            let mut notes = Vec::new();
            for item in args
                .get("notes")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let item = item.as_object().cloned().unwrap_or_default();
                notes.push(PackagedNote {
                    title: arg(&item, "title").map(str::to_owned),
                    body: required(&item, "body")?.to_owned(),
                    kind: arg(&item, "kind").map(str::to_owned),
                    attribution: arg(&item, "attribution").map(str::to_owned),
                });
            }

            let scenes = match args.get("scenes").and_then(Value::as_array) {
                Some(raw) if !raw.is_empty() => proposal::scenes_from(raw, vocabulary)?,
                _ => Vec::new(),
            };

            let mut releases = Vec::new();
            for item in args
                .get("releases")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let item = item.as_object().cloned().unwrap_or_default();
                let release_kind = required(&item, "kind")?;
                config.require_release_kind(&kind, release_kind)?;
                let defined = vocabulary
                    .release_kinds
                    .iter()
                    .find(|k| k.key == release_kind)
                    .expect("just checked by require_release_kind");
                let scheduled_at = arg(&item, "scheduled_at").map(str::to_owned);
                if let Some(date) = &scheduled_at
                    && !crate::time::is_date(date)
                {
                    return Err(Error::refused("mcp.badReleaseDate").param("value", date.clone()));
                }
                let (fields, unknown_fields) = proposal::release_fields_from(
                    item.get("fields")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default(),
                    defined,
                );
                releases.push(proposal::PackagedRelease {
                    kind: release_kind.to_owned(),
                    scheduled_at,
                    fields,
                    unknown_fields,
                });
            }

            let mut trials_left_out = Vec::new();
            let trials = match args.get("trials") {
                Some(raw) if raw.as_array().is_some_and(|list| !list.is_empty()) => {
                    if config.lab(&kind).is_none() {
                        return Err(Error::refused("trial.noLabKind").param("kind", kind.clone()));
                    }
                    let read = crate::lab::answer::read(
                        conn,
                        &config,
                        crate::lab::answer::For {
                            profile_id: &profile.id,
                            kind: &kind,
                            work_id: found.as_ref().map(|w| w.id.as_str()),
                        },
                        raw,
                        None,
                    )?;
                    trials_left_out = read
                        .dropped
                        .iter()
                        .map(|one| format!("trial {}: {} “{}”", one.trial, one.part, one.value))
                        .collect();
                    read.trials
                }
                _ => Vec::new(),
            };

            // Cards for the boards of the work's conception, read as
            // `propose_cards` reads them: a song arrives with its research.
            let mut cards_left_out = Vec::new();
            let mut boards = Vec::new();
            for item in args
                .get("boards")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let item = item.as_object().cloned().unwrap_or_default();
                let laid = config.require_board(&kind, required(&item, "board")?)?;
                let raw =
                    json!({ "cards": item.get("cards").cloned().unwrap_or_else(|| json!([])) });
                let read = crate::board::answer::read(laid, config.library(), &raw, None)?;
                cards_left_out.extend(read.dropped.iter().map(|one| {
                    format!(
                        "{} card {}: {} “{}”",
                        laid.label, one.card, one.part, one.value
                    )
                }));
                boards.push(crate::assistant::proposal::PackagedBoard {
                    board: laid.key.clone(),
                    cards: read.cards,
                });
            }

            if found.is_some()
                && fields.is_empty()
                && vibe.is_none()
                && versions.is_empty()
                && score.is_none()
                && notes.is_empty()
                && scenes.is_empty()
                && releases.is_empty()
                && trials.is_empty()
                && boards.is_empty()
            {
                return Err(Error::refused("mcp.emptyPackage"));
            }

            let proposal = Proposal::Work {
                vibe: vibe.map(Box::new),
                title,
                work_kind: Some(kind.clone()),
                fields,
                unknown_fields,
                versions,
                score,
                notes,
                scenes,
                releases,
                trials,
                boards,
            };
            let body = apply::render_package(&proposal, &config, &kind, Speech::ENGLISH);
            let mut summary = package_summary(&proposal);
            if !trials_left_out.is_empty() {
                summary.push_str(&format!(
                    ", left out of trials (not in the workspace): {}",
                    trials_left_out.join("; ")
                ));
            }
            if !cards_left_out.is_empty() {
                summary.push_str(&format!(
                    ", left out of cards: {}",
                    cards_left_out.join("; ")
                ));
            }
            deliver(
                conn,
                &profile.id,
                session,
                found.as_ref(),
                proposal,
                &body,
                None,
            )?;
            Ok(match found {
                Some(w) => format!(
                    "Proposed a package for “{}” — {summary}. It waits in the chat on the work; one click applies all of it.",
                    w.title
                ),
                None => format!(
                    "Proposed a new work — {summary}. It waits in the chat named after you; one click creates it with everything in it."
                ),
            })
        }

        "cover" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            if !config.vocabulary(&found.kind).cover {
                return Err(Error::refused("idea.noCover").param("title", found.title.clone()));
            }
            let board = crate::cover::idea::board(conn, &found.id)?;
            let ideas: Vec<Value> = board
                .ideas
                .iter()
                .map(|card| {
                    json!({
                        "id": card.idea.id,
                        "source": card.idea.source,
                        "verdict": card.idea.verdict,
                        "angle": card.idea.angle,
                        "headline": card.idea.headline,
                        "from": card.from_title,
                        "concept": card.idea.concept,
                    })
                })
                .collect();
            let siblings: Vec<Value> = board
                .siblings
                .iter()
                .map(|sibling| {
                    json!({
                        "work": sibling.work_id,
                        "title": sibling.title,
                        "kind": sibling.kind,
                        "concept": sibling.concept,
                    })
                })
                .collect();
            pretty(&json!({
                "work": found.id,
                "title": found.title,
                "format": board.format,
                "cover": found.cover,
                "ideas": ideas,
                "siblings": siblings,
                "choices": crate::cover::idea::choices_sheet(conn, &found)?,
            }))
        }

        "propose_cover" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            if !config.vocabulary(&found.kind).cover {
                return Err(Error::refused("idea.noCover").param("title", found.title.clone()));
            }
            let raw = args
                .get("ideas")
                .filter(|raw| raw.as_array().is_some_and(|list| !list.is_empty()))
                .ok_or_else(|| Error::refused("mcp.ideasNotArray"))?;
            let read = crate::cover::idea::read(conn, &profile.id, &config, raw, None)?;
            let count = read.ideas.len();
            let left_out: Vec<String> = read
                .dropped
                .iter()
                .map(|one| format!("idea {}: {} “{}”", one.idea, one.part, one.value))
                .collect();
            let body = apply::render_ideas(conn, &read.ideas, &read.dropped, Speech::ENGLISH);
            let proposal = Proposal::CoverIdeas {
                work_id: found.id.clone(),
                ideas: read.ideas,
                dropped: read.dropped,
            };
            deliver(
                conn,
                &profile.id,
                session,
                Some(&found),
                proposal,
                &body,
                arg(args, "note"),
            )?;
            let plural = if count == 1 { "" } else { "s" };
            let mut said = format!(
                "Proposed {count} idea{plural} for the cover of “{}”. They wait in the chat on the work and above its board; one click puts them on the board.",
                found.title
            );
            if !left_out.is_empty() {
                said.push_str(&format!(
                    " Left out, not in the workspace: {}.",
                    left_out.join("; ")
                ));
            }
            Ok(said)
        }

        "brief" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            let conception = config.require_conception(&found.kind)?;
            // Nothing written yet is a conception still to be written, not a
            // mistake: the boards are there to read all the same.
            let order = match crate::conception::order(conn, &found) {
                Ok(order) => Value::from(order),
                Err(error)
                    if error
                        .refusal()
                        .is_some_and(|r| r.code == "conception.empty") =>
                {
                    Value::Null
                }
                Err(error) => return Err(error),
            };
            let boards: Vec<Value> = crate::board::boards(conn, &found)?
                .into_iter()
                .map(|view| {
                    let laid = config.board(&view.board);
                    json!({
                        "key": view.board,
                        "label": laid.map(|board| &board.label),
                        "rule": laid.map(|board| board.rule),
                        "cards": view.cards.iter().map(|card| json!({
                            "id": card.id,
                            "kind": card.kind,
                            "topic": card.topic,
                            "source": card.source,
                            "verdict": card.verdict,
                            "entry": card.entry,
                            "fields": card.fields,
                        })).collect::<Vec<_>>(),
                        "directions": view.directions,
                        "elsewhere": view.elsewhere,
                    })
                })
                .collect();
            // The author's key, entry by entry, with the rule it is kept by
            // (v0.146, FR-10 10.1).
            let anchors: Vec<String> = config
                .fields_of(&found.kind)
                .into_iter()
                .filter(|field| field.protected)
                .filter_map(|field| found.meta.get(&field.key))
                .flat_map(profile::config::MetaFieldType::entries)
                .collect();
            pretty(&json!({
                "work": found.id,
                "title": found.title,
                "kind": found.kind,
                "brief_role": conception.brief,
                "idea_field": conception.idea,
                "order": order,
                "anchors": anchors,
                "anchors_rule": "Keep every anchor as it is: do not change, move or drop one unless the author asks.",
                "boards": boards,
            }))
        }

        "propose_cards" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            let board = config.require_board(&found.kind, required(args, "board")?)?;
            let cards = args
                .get("cards")
                .filter(|raw| raw.as_array().is_some_and(|list| !list.is_empty()))
                .ok_or_else(|| Error::refused("mcp.cardsNotArray"))?;
            let mut raw = json!({ "cards": cards });
            if let Some(directions) = args.get("directions") {
                raw["directions"] = directions.clone();
            }
            let read = crate::board::answer::read(board, config.library(), &raw, None)?;
            let count = read.cards.len();
            let left_out: Vec<String> = read
                .dropped
                .iter()
                .map(|one| format!("card {}: {} “{}”", one.card, one.part, one.value))
                .collect();
            let body = apply::render_cards(
                board,
                &read.cards,
                &read.directions,
                &read.dropped,
                Speech::ENGLISH,
            );
            let proposal = Proposal::Cards {
                work_id: found.id.clone(),
                board: board.key.clone(),
                cards: read.cards,
                directions: read.directions,
                dropped: read.dropped,
            };
            deliver(
                conn,
                &profile.id,
                session,
                Some(&found),
                proposal,
                &body,
                arg(args, "note"),
            )?;
            let plural = if count == 1 { "" } else { "s" };
            let mut said = format!(
                "Proposed {count} card{plural} for the board “{}” of “{}”. They wait in the chat on the work and above its board; one click puts them on the board.",
                board.label, found.title
            );
            if !left_out.is_empty() {
                said.push_str(&format!(
                    " Left out, not on the board: {}.",
                    left_out.join("; ")
                ));
            }
            Ok(said)
        }

        "trials" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            let board = crate::lab::trial::board(conn, &found.id)?;
            let page = Page::of(args, 50);
            let one = arg(args, "trial");
            let summary = page.summary && one.is_none();
            let trials: Vec<Value> = board
                .trials
                .iter()
                .filter(|card| one.is_none_or(|id| card.trial.id == id))
                .map(|card| {
                    if summary {
                        return json!({
                            "id": card.trial.id,
                            "series": card.trial.series,
                            "parent": card.trial.parent_id,
                            "angle": card.trial.angle,
                            "verdict": card.trial.verdict,
                            "length": length_of(&card.trial.body),
                            "takes": card.takes.len(),
                        });
                    }
                    json!({
                        "id": card.trial.id,
                        "series": card.trial.series,
                        "parent": card.trial.parent_id,
                        "angle": card.trial.angle,
                        "body": card.trial.body,
                        "bricks": card.trial.bricks,
                        "reference": card.trial.reference,
                        "outcome": card.trial.outcome,
                        "verdict": card.trial.verdict,
                        "run_first": card.trial.run_first,
                        "source": card.source.as_ref().map(|source| json!({
                            "work": source.work_id, "title": source.title,
                            "version": source.version_id, "revision": source.revision,
                        })),
                        "lost_anchors": card.lost_anchors,
                        "takes": card.takes.len(),
                        "harvest": card.harvest,
                    })
                })
                .collect();
            // One trial, by its id: whole, and nothing of the board around it.
            if let Some(id) = one {
                let whole = trials
                    .into_iter()
                    .next()
                    .ok_or_else(|| Error::refused("mcp.unknownTrial").param("trial", id))?;
                return pretty(&whole);
            }
            let mut answer = page.cut("trials", trials);
            answer.insert("work".into(), json!(found.id));
            answer.insert("title".into(), json!(found.title));
            answer.insert("anchors".into(), json!(board.anchors));
            answer.insert("harvest_role".into(), json!(board.harvest_role));
            answer.insert("harvest_kinds".into(), json!(board.harvest_kinds));
            answer.insert("series".into(), json!(board.series));
            // The phrases a trial is written from are the long half of the
            // board: left out of a summary, like the texts.
            if !page.summary {
                answer.insert(
                    "choices".into(),
                    serde_json::to_value(crate::lab::answer::choices_sheet(conn, &found)?)?,
                );
            }
            pretty(&answer)
        }

        "propose_trials" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            if config.lab(&found.kind).is_none() {
                return Err(Error::refused("trial.noLab").param("title", found.title.clone()));
            }
            let raw = args
                .get("trials")
                .filter(|raw| raw.as_array().is_some_and(|list| !list.is_empty()))
                .ok_or_else(|| Error::refused("mcp.trialsNotArray"))?;
            let read = crate::lab::answer::read(
                conn,
                &config,
                crate::lab::answer::For::work(&found),
                raw,
                None,
            )?;
            let count = read.trials.len();
            let left_out: Vec<String> = read
                .dropped
                .iter()
                .map(|one| format!("trial {}: {} “{}”", one.trial, one.part, one.value))
                .collect();
            let body = apply::render_trials(&read.trials, &read.dropped, Speech::ENGLISH);
            let proposal = Proposal::Trials {
                work_id: found.id.clone(),
                trials: read.trials,
                dropped: read.dropped,
            };
            deliver(
                conn,
                &profile.id,
                session,
                Some(&found),
                proposal,
                &body,
                arg(args, "note"),
            )?;
            let plural = if count == 1 { "" } else { "s" };
            let mut said = format!(
                "Proposed {count} trial{plural} for the board of “{}”. They wait in the chat on the work and above its board; one click puts them on the board.",
                found.title
            );
            if !left_out.is_empty() {
                said.push_str(&format!(
                    " Left out, not in the workspace: {}.",
                    left_out.join("; ")
                ));
            }
            Ok(said)
        }

        "propose_scenes" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            let vocabulary = config.vocabulary(&found.kind);
            let raw = args
                .get("scenes")
                .and_then(Value::as_array)
                .filter(|raw| !raw.is_empty())
                .ok_or_else(|| Error::refused("mcp.scenesNotArray"))?;
            let scenes = proposal::scenes_from(raw, vocabulary)?;
            let change = match args.get("change").and_then(Value::as_str) {
                None => BoardChange::Add,
                Some(value) => BoardChange::parse(value)
                    .ok_or_else(|| Error::refused("mcp.badBoardChange").param("value", value))?,
            };
            if change == BoardChange::Revise
                && let Some(missing) = scenes.iter().position(|scene| scene.position.is_none())
            {
                return Err(Error::refused("mcp.reviseNeedsPosition").param("index", missing + 1));
            }
            let count = scenes.len();
            let standing = scene::count(conn, &found.id)?;
            let body = apply::render_board(&scenes, vocabulary, change, Speech::ENGLISH);
            let proposal = Proposal::Scenes { scenes, change };
            deliver(
                conn,
                &profile.id,
                session,
                Some(&found),
                proposal,
                &body,
                None,
            )?;
            let plural = if count == 1 { "" } else { "s" };
            Ok(match change {
                BoardChange::Replace => format!(
                    "Proposed a storyboard of {count} scene{plural} to replace the {standing} on the board of “{}”. It waits in the chat on the work; one click replaces the board.",
                    found.title
                ),
                BoardChange::Revise => format!(
                    "Proposed a revision of {count} scene{plural} on the board of “{}”. It waits in the chat on the work; one click revises them in place.",
                    found.title
                ),
                BoardChange::Add => format!(
                    "Proposed {count} scene{plural} to add after the {standing} on the board of “{}”. It waits in the chat on the work; one click adds them.",
                    found.title
                ),
            })
        }

        "propose_release" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            // A publication goes out once (ADR 0051): the work names its
            // release.
            let chosen = release::of_work(conn, &found.id)?.ok_or_else(|| {
                Error::refused("mcp.workHasNoRelease").param("title", found.title.clone())
            })?;
            let declared = crate::release_meta::declared(conn, &chosen.id)?;
            let raw = args
                .get("fields")
                .cloned()
                .ok_or_else(|| Error::refused("mcp.missingArg").param("key", "fields"))?;
            let proposal =
                proposal::release_from_value(&json!({ "fields": raw }), &chosen.id, &declared)
                    .map_err(|why| Error::refused("mcp.releaseFieldsUnread").param("why", why))?;
            let proposal = crate::release_meta::public(conn, proposal)?;
            let Proposal::Release {
                fields, unknown, ..
            } = &proposal
            else {
                return Err(Error::Internal(
                    "a release proposal read as something else".into(),
                ));
            };
            let mut body = String::new();
            for field in &declared {
                if let Some(text) = fields.get(&field.key).and_then(Value::as_str) {
                    body.push_str(&format!("**{}**\n\n{text}\n\n", field.label));
                }
            }
            let taken = fields.len();
            let left: Vec<String> = unknown.clone();
            deliver(
                conn,
                &profile.id,
                session,
                Some(&found),
                proposal,
                body.trim_end(),
                arg(args, "note"),
            )?;
            let mut said = format!(
                "Proposed {taken} field{} for the {} release of “{}”; it waits in the chat on the work, each field beside what is written now.",
                if taken == 1 { "" } else { "s" },
                chosen.kind,
                found.title
            );
            if !left.is_empty() {
                said.push_str(&format!(
                    " Left out, as the kind of release has no such field: {}.",
                    left.join(", ")
                ));
            }
            Ok(said)
        }

        "propose_note" => {
            let found = match arg(args, "work") {
                Some(named) => Some(find_work(conn, &profile.id, named)?),
                None => None,
            };
            // Several at once, or one that says more than its words - tags,
            // a layer, a note it changes: read as notes, kept whole or one by
            // one, each change shown against what it changes (v0.116).
            let several = args.get("notes").is_some();
            if several
                || args.get("tags").is_some()
                || arg(args, "layer").is_some()
                || arg(args, "note").is_some()
                || arg(args, "origin_work").is_some()
            {
                let raw: Vec<Value> = match args.get("notes") {
                    Some(Value::Array(list)) if !list.is_empty() => list.clone(),
                    Some(_) => return Err(Error::refused("mcp.notesNotArray")),
                    None => vec![Value::Object(args.clone())],
                };
                let mut notes = Vec::with_capacity(raw.len());
                for one in raw {
                    notes.push(read_proposed_note(conn, &profile.id, &config, &one)?);
                }
                let count = notes.len();
                let changes = notes.iter().filter(|one| one.note.is_some()).count();
                let body = render_notes(&notes);
                deliver(
                    conn,
                    &profile.id,
                    session,
                    found.as_ref(),
                    Proposal::Notes { notes },
                    &body,
                    None,
                )?;
                return Ok(format!(
                    "Proposed {count} note{} ({changes} of them a change to a note that is there). \
                     It waits in the chat {}; the person keeps it whole or note by note.",
                    if count == 1 { "" } else { "s" },
                    if found.is_some() {
                        "on the work"
                    } else {
                        "named after you"
                    }
                ));
            }
            let body = required(args, "body")?;
            let kind = arg(args, "kind").map(str::to_owned);
            if let Some(kind) = kind.as_deref()
                && !config.note_kind(kind).is_some_and(|found| !found.is_card())
            {
                return Err(Error::refused("proposal.unknownNoteKind").param("kind", kind));
            }
            let proposal = Proposal::Note {
                title: arg(args, "title").map(str::to_owned),
                note_kind: kind,
                attribution: arg(args, "attribution").map(str::to_owned),
                shelf: arg(args, "shelf").map(str::to_owned),
            };
            deliver(
                conn,
                &profile.id,
                session,
                found.as_ref(),
                proposal,
                body,
                None,
            )?;
            Ok(match found {
                Some(w) => format!(
                    "Proposed a note on “{}”; it waits in the chat on the work.",
                    w.title
                ),
                None => "Proposed a note; it waits in the chat named after you.".into(),
            })
        }

        other => Err(Error::refused("mcp.unknownTool").param("name", other)),
    }
}

/// The `notes` tool: the notes a filter leaves - a phrase with the shelves it
/// stands on, how many works say it and how it is sung (ADR 0064) - or the
/// phrases of one shelf; asked for nothing in particular, the shelves too.
fn read_notes(
    conn: &Connection,
    profile_id: &str,
    config: &profile::config::ProfileConfig,
    filter: &NoteFilter,
    shelf: Option<&str>,
) -> Result<Vec<Value>> {
    let shelves = crate::shelf::views(conn, profile_id)?;
    let only = match shelf {
        Some(named) => Some(
            crate::shelf::find(conn, profile_id, named)?
                .ok_or_else(|| Error::refused("shelf.unknown").param("shelf", named))?,
        ),
        None => None,
    };
    let names = |ids: &[String]| -> Vec<String> {
        ids.iter()
            .filter_map(|id| shelves.iter().find(|view| view.shelf.id == *id))
            .map(|view| view.shelf.name.clone())
            .collect()
    };
    let lines = crate::lines::rows(conn, profile_id, filter)?;
    let mut out: Vec<Value> = Vec::new();
    for found in note::list(conn, profile_id, filter)? {
        let line = config.note_kind(&found.kind).is_some_and(|kind| kind.line);
        if !line {
            if only.is_none() {
                out.push(serde_json::to_value(&found)?);
            }
            continue;
        }
        let Some(row) = lines.iter().find(|row| row.note.id == found.id) else {
            continue;
        };
        if only
            .as_ref()
            .is_some_and(|shelf| !row.shelf_ids.contains(&shelf.id))
        {
            continue;
        }
        let mut shown = serde_json::to_value(&found)?;
        if let Some(object) = shown.as_object_mut() {
            object.insert("shelves".into(), json!(names(&row.shelf_ids)));
            object.insert("sung_in".into(), json!(row.uses));
            if !row.sung.is_empty() {
                object.insert("sung".into(), json!(row.sung));
            }
        }
        out.push(shown);
    }
    Ok(out)
}

/// One note of a proposal, read from what an agent wrote: its kind checked
/// against the craft, the note it changes found, and what that note holds
/// now kept beside it for the person to read against.
fn read_proposed_note(
    conn: &Connection,
    profile_id: &str,
    config: &profile::config::ProfileConfig,
    raw: &Value,
) -> Result<crate::assistant::proposal::ProposedNote> {
    use crate::assistant::proposal::{NoteBefore, ProposedNote};

    let object = raw
        .as_object()
        .ok_or_else(|| Error::refused("mcp.notesNotArray"))?;
    let text = |key: &str| arg(object, key).map(str::to_owned);
    let kind = text("kind");
    if let Some(kind) = kind.as_deref()
        && !config.note_kind(kind).is_some_and(|found| !found.is_card())
    {
        return Err(Error::refused("proposal.unknownNoteKind").param("kind", kind));
    }
    let layer = match arg(object, "layer") {
        Some(word) => Some(
            serde_json::from_value::<crate::canon::Layer>(json!(word))
                .map_err(|_| Error::refused("mcp.unknownLayer").param("layer", word))?,
        ),
        None => None,
    };
    let tags: Vec<String> = match object.get("tags") {
        Some(Value::Array(list)) => list
            .iter()
            .filter_map(Value::as_str)
            .map(|tag| tag.trim().trim_start_matches('#').to_owned())
            .filter(|tag| !tag.is_empty())
            .collect(),
        _ => Vec::new(),
    };
    let body = text("body").unwrap_or_default();
    // The work a new note arose at (v0.145): a phrase that came while a song
    // was written remembers the song, and stays fresh for the next one.
    let origin_work_id = match arg(object, "origin_work") {
        Some(named) => Some(find_work(conn, profile_id, named)?.id),
        None => None,
    };
    let target = match arg(object, "note") {
        Some(named) => Some(find_note(conn, profile_id, named)?),
        None => None,
    };
    // A new note is its words; a change may leave them as they are.
    if target.is_none() && body.trim().is_empty() {
        return Err(Error::refused("mcp.missingArg").param("key", "body"));
    }
    Ok(ProposedNote {
        title: text("title"),
        body,
        kind,
        attribution: text("attribution"),
        shelf: text("shelf"),
        tags,
        layer,
        before: target.as_ref().map(|found| NoteBefore {
            title: found.title.clone(),
            body: found.body.clone(),
            tags: found.tags.clone(),
        }),
        note: target.map(|found| found.id),
        origin_work_id,
    })
}

/// Proposed notes as a person reads them before keeping them: each new one
/// with its words, each change with what it changes.
fn render_notes(notes: &[crate::assistant::proposal::ProposedNote]) -> String {
    let mut out = String::new();
    for one in notes {
        let name = one
            .title
            .clone()
            .or_else(|| one.before.as_ref().and_then(|before| before.title.clone()))
            .unwrap_or_else(|| {
                let source = if one.body.trim().is_empty() {
                    one.before
                        .as_ref()
                        .map_or("", |before| before.body.as_str())
                } else {
                    one.body.as_str()
                };
                source
                    .lines()
                    .find(|line| !line.trim().is_empty())
                    .unwrap_or("")
                    .trim()
                    .chars()
                    .take(80)
                    .collect()
            });
        match &one.before {
            None => {
                out.push_str(&format!("- **{name}**"));
                if !one.tags.is_empty() {
                    out.push_str(&format!(" _({})_", one.tags.join(", ")));
                }
                out.push('\n');
                if one.title.is_some() {
                    out.push_str(&format!("  {}\n", one.body.trim().replace('\n', "\n  ")));
                }
            }
            Some(before) => {
                out.push_str(&format!("- **{name}** — a change\n"));
                if let Some(title) = one.title.as_deref().filter(|title| {
                    before.title.as_deref() != Some(*title) && !title.trim().is_empty()
                }) {
                    out.push_str(&format!(
                        "  title: {} → {title}\n",
                        before.title.as_deref().unwrap_or("-")
                    ));
                }
                if !one.tags.is_empty() && one.tags != before.tags {
                    out.push_str(&format!(
                        "  tags: {} → {}\n",
                        if before.tags.is_empty() {
                            "-".to_owned()
                        } else {
                            before.tags.join(", ")
                        },
                        one.tags.join(", ")
                    ));
                }
                if !one.body.trim().is_empty() && one.body.trim() != before.body.trim() {
                    out.push_str(&format!(
                        "  the text is rewritten ({} → {} characters)\n",
                        length_of(&before.body),
                        length_of(&one.body)
                    ));
                }
            }
        }
    }
    out.trim_end().to_owned()
}

/// A note as a summary gives it: what it is and how much there is of it,
/// without its body. A note with no title is called by its first words.
fn note_summary(note: &Value) -> Value {
    let body = note["body"].as_str().unwrap_or_default();
    let title = match note["title"]
        .as_str()
        .filter(|title| !title.trim().is_empty())
    {
        Some(title) => title.to_owned(),
        None => {
            let first = body
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("");
            first.trim().chars().take(80).collect()
        }
    };
    json!({
        "id": note["id"], "title": title, "kind": note["kind"], "state": note["state"],
        "tags": note["tags"], "work_id": note["work_id"],
        "length": length_of(body), "updated_at": note["updated_at"],
    })
}

/// A note by its id, or by its exact title.
fn find_note(conn: &Connection, profile_id: &str, named: &str) -> Result<note::Note> {
    if let Some(found) = note::get(conn, named)?.filter(|found| found.profile_id == profile_id) {
        return Ok(found);
    }
    let mut titled = note::list(conn, profile_id, &NoteFilter::default())?
        .into_iter()
        .filter(|found| found.title.as_deref().map(str::trim) == Some(named.trim()));
    match (titled.next(), titled.next()) {
        (Some(found), None) => Ok(found),
        (Some(_), Some(_)) => Err(Error::refused("mcp.ambiguousNote").param("note", named)),
        (None, _) => Err(Error::refused("mcp.unknownNote").param("note", named)),
    }
}

/// The `canon` tool: the list of cards, one card through a lens, a search of
/// the facts, or the timeline.
fn read_canon(
    conn: &Connection,
    profile_id: &str,
    config: &profile::config::ProfileConfig,
    args: &Map<String, Value>,
) -> Result<String> {
    use crate::canon::{proposal::CardIndex, view};

    let lens = match arg(args, "lens") {
        Some(word) => Some(
            profile::config::Lens::parse(word)
                .ok_or_else(|| Error::refused("mcp.unknownLens").param("lens", word))?,
        ),
        None => None,
    };
    let index = CardIndex::of(conn, profile_id)?;
    let card = match arg(args, "card") {
        Some(named) => Some(
            index
                .find_or_root(named, config)
                .cloned()
                .ok_or_else(|| Error::refused("canon.unknownCard").param("card", named))?,
        ),
        None => None,
    };

    if args.get("timeline").and_then(Value::as_bool) == Some(true) {
        let dated = view::timeline(conn, profile_id, card.as_ref().map(|c| c.id.as_str()))?;
        return pretty(
            &dated
                .into_iter()
                .map(|one| {
                    json!({
                        "card": one.fact.note_id, "card_title": one.card_title, "fact": one.fact.id,
                        "text": one.fact.body, "when": one.fact.when, "layer": one.fact.layer,
                        "status": one.fact.status, "read_by": one.lenses,
                    })
                })
                .collect::<Vec<_>>(),
        );
    }

    if let Some(query) = arg(args, "query") {
        let needle = search::fold(query);
        let mut hits = Vec::new();
        for one in crate::canon::fact::for_profile(conn, profile_id)? {
            if !search::matches(&one.body, &needle) {
                continue;
            }
            let title = crate::note::get(conn, &one.note_id)?.and_then(|c| c.title);
            hits.push(json!({
                "fact": one.id, "card": one.note_id, "card_title": title, "section": one.section,
                "text": one.body, "layer": one.layer, "status": one.status, "when": one.when,
                "source": one.source,
            }));
        }
        return pretty(&hits);
    }

    if let Some(card) = card {
        let seen = view::card(conn, &card.id)?;
        let kind = config.card_kind(&seen.card.kind);
        // A long card is read in parts (v0.144, FR-18 18.5): its head and
        // counts alone, one section, one state, a page of facts.
        if args.get("summary").and_then(Value::as_bool) == Some(true) {
            let counts: Vec<Value> = kind
                .map(|k| {
                    k.sections
                        .iter()
                        .map(|s| {
                            let held = seen.facts.iter().filter(|r| r.fact.section == s.key);
                            json!({
                                "key": s.key, "label": s.label, "shape": s.shape,
                                "facts": held.clone().count(),
                                "drafts": held.filter(|r| r.fact.status == crate::canon::FactStatus::Draft).count(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            return pretty(&json!({
                "id": seen.card.id, "kind": seen.card.kind, "title": seen.card.title,
                "aliases": seen.card.aliases, "layer": seen.card.layer,
                "lives_at_work": seen.card.work_id, "note": seen.card.body,
                "total_facts": seen.facts.len(), "relations": seen.relations.len(),
                "sections": counts,
            }));
        }
        let wanted_section = arg(args, "section");
        let wanted_status = match arg(args, "status") {
            Some(word) => Some(
                crate::canon::FactStatus::from_word(word)
                    .ok_or_else(|| Error::refused("canon.unknownStatus").param("status", word))?,
            ),
            None => None,
        };
        let narrowed: Vec<&view::ReadFact> = seen
            .facts
            .iter()
            .filter(|read| lens.is_none_or(|lens| read.lenses.contains(&lens)))
            .filter(|read| wanted_section.is_none_or(|key| read.fact.section == key))
            .filter(|read| wanted_status.is_none_or(|status| read.fact.status == status))
            .collect();
        let total_facts = narrowed.len();
        let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .map(|n| n as usize);
        let page: Vec<&&view::ReadFact> = narrowed
            .iter()
            .skip(offset)
            .take(limit.unwrap_or(usize::MAX))
            .collect();
        let has_more = offset + page.len() < total_facts;
        let facts: Vec<Value> = page
            .into_iter()
            .map(|read| {
                let one = &read.fact;
                json!({
                    "id": one.id, "section": one.section, "text": one.body, "layer": one.layer,
                    "status": one.status, "retired_because": one.retired_reason,
                    "source": one.source, "when": one.when, "only_for_work": one.scope_work_id,
                    "data": one.data, "read_by": read.lenses,
                })
            })
            .collect();
        let relations: Vec<Value> = seen
            .relations
            .iter()
            .filter(|relation| lens.is_none_or(|lens| relation.lenses.contains(&lens)))
            .map(|relation| {
                json!({
                    "card": relation.other_id, "title": relation.other_title,
                    "kind": relation.link.kind, "is": relation.label, "layer": relation.link.layer,
                    "section": relation.section,
                })
            })
            .collect();
        if lens.is_some_and(|lens| !seen.lenses.contains(&lens)) {
            return Ok(
                "This card is not seen through that lens: publicly it does not exist.".into(),
            );
        }
        return pretty(&json!({
            "id": seen.card.id, "kind": seen.card.kind, "title": seen.card.title,
            "aliases": seen.card.aliases, "layer": seen.card.layer,
            "lives_at_work": seen.card.work_id, "note": seen.card.body,
            "description_for_pictures": seen.card.prompt, "description_stale": seen.prompt_stale,
            "sections": kind.map(|k| k.sections.iter().map(|s| json!({
                "key": s.key, "label": s.label, "shape": s.shape, "read_by": s.lenses,
            })).collect::<Vec<_>>()),
            "facts": facts,
            "total_facts": total_facts,
            "has_more": has_more,
            "relations": relations,
            "pictures": seen.pictures.iter().map(|p| json!({
                "role": p.kind, "path": p.path, "of_fact": p.canon_fact_id,
            })).collect::<Vec<_>>(),
            "appears_in": seen.appearances,
        }));
    }

    let cards = view::cards(
        conn,
        profile_id,
        &view::CardFilter {
            kind: arg(args, "kind").map(str::to_owned),
            ..view::CardFilter::default()
        },
    )?;
    // Whether the space has its passport, and what each shape of section
    // carries beside an entry's words (v0.144).
    let root = index.root(config);
    let schemas: Map<String, Value> = [
        SectionShape::Facts,
        SectionShape::Slots,
        SectionShape::Details,
        SectionShape::Palette,
        SectionShape::Marks,
        SectionShape::Styles,
        SectionShape::Bans,
        SectionShape::Branding,
        SectionShape::Quotas,
        SectionShape::Bank,
    ]
    .into_iter()
    .map(|shape| {
        let said = serde_json::to_value(shape).unwrap_or_default();
        let key = said.as_str().unwrap_or_default().to_owned();
        (key, section_schema(shape))
    })
    .collect();
    pretty(&json!({
        "has_root_card": root.is_some(),
        "root_card_id": root.map(|card| card.id.clone()),
        "section_schemas": schemas,
        "kinds": config.note_kinds.iter().filter(|k| k.is_card()).map(|k| json!({
            "key": k.key, "label": k.label, "root": k.root,
            "sections": k.sections.iter().map(|s| json!({
                "key": s.key, "label": s.label, "shape": s.shape, "read_by": s.lenses,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "relation_kinds": config.relation_kinds,
        "cards": cards.iter().map(|c| json!({
            "id": c.id, "kind": c.kind, "title": c.title, "aliases": c.aliases, "layer": c.layer,
            "lives_at_work": c.work_title, "facts": c.facts, "drafts": c.drafts, "open": c.open,
        })).collect::<Vec<_>>(),
    }))
}

/// What a neighbouring space puts out or plans on `days`, in the compact
/// fields `calendar`'s `cross_spaces` gives (v0.151, FR-20 20.2): the space,
/// the day, the title, the cover's idea, the genre and the mood - nothing of
/// its texts, its canon or its people. Read from its database opened for
/// reading only; a space that cannot be read says nothing.
fn neighbour_days(
    name: &str,
    database: &std::path::Path,
    days: &BTreeSet<String>,
) -> Result<Vec<Value>> {
    if days.is_empty() {
        return Ok(Vec::new());
    }
    let Ok(conn) = crate::db::open_read_only(database) else {
        return Ok(Vec::new());
    };
    let Some(profile) = profile::active(&conn)? else {
        return Ok(Vec::new());
    };
    let mut dated: Vec<(String, String)> = Vec::new();
    for entry in release::calendar(&conn, &profile.id)? {
        if let Some(at) = entry.release.scheduled_at.as_deref() {
            dated.push((
                at.get(..10).unwrap_or(at).to_owned(),
                entry.release.work_id.clone(),
            ));
        }
    }
    let config = profile::config_for(&conn, &profile.id)?;
    for plan in crate::work_plan::open(&conn, &profile.id, config.plan_window())? {
        dated.push((plan.scheduled_at.clone(), plan.work_id.clone()));
    }
    let mut out = Vec::new();
    for (day, work_id) in dated {
        if !days.contains(&day) {
            continue;
        }
        let Some(work) = work::get(&conn, &work_id)? else {
            continue;
        };
        let text = |key: &str| {
            work.meta
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
        };
        let cover_idea = text("cover_idea")
            .or_else(|| (!work.cover.idea.trim().is_empty()).then(|| work.cover.idea.clone()));
        out.push(json!({
            "space_name": name,
            "date": day,
            "title": work.title,
            "cover_idea": cover_idea,
            "genre": text("genre"),
            "mood": text("mood"),
        }));
    }
    Ok(out)
}

/// What a package holds, in one clause for the agent's answer.
fn package_summary(proposal: &Proposal) -> String {
    let Proposal::Work {
        title,
        fields,
        unknown_fields,
        versions,
        score,
        notes,
        scenes,
        releases,
        trials,
        boards,
        ..
    } = proposal
    else {
        return String::new();
    };
    let mut parts = Vec::new();
    if let Some(title) = title {
        parts.push(format!("“{title}”"));
    }
    if !versions.is_empty() {
        parts.push(format!(
            "{} version{} ({})",
            versions.len(),
            if versions.len() == 1 { "" } else { "s" },
            versions
                .iter()
                .map(|v| v.role.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !fields.is_empty() {
        parts.push(format!(
            "fields {}",
            fields.keys().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    if let Some(marks) = score {
        parts.push(format!("a score on {} axes", marks.axes.len()));
        if !marks.unknown.is_empty() {
            parts.push(format!("axes ignored: {}", marks.unknown.join(", ")));
        }
    }
    if !notes.is_empty() {
        parts.push(format!(
            "{} note{}",
            notes.len(),
            if notes.len() == 1 { "" } else { "s" }
        ));
    }
    if !scenes.is_empty() {
        parts.push(format!(
            "{} scene{}",
            scenes.len(),
            if scenes.len() == 1 { "" } else { "s" }
        ));
    }
    if !trials.is_empty() {
        parts.push(format!(
            "{} trial{}",
            trials.len(),
            if trials.len() == 1 { "" } else { "s" }
        ));
    }
    for packaged in boards {
        parts.push(format!(
            "{} card{} for `{}`",
            packaged.cards.len(),
            if packaged.cards.len() == 1 { "" } else { "s" },
            packaged.board
        ));
    }
    if !releases.is_empty() {
        parts.push(format!(
            "{} release{} ({})",
            releases.len(),
            if releases.len() == 1 { "" } else { "s" },
            releases
                .iter()
                .map(|r| r.kind.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !unknown_fields.is_empty() {
        parts.push(format!(
            "fields ignored (not in the profile): {}",
            unknown_fields.join(", ")
        ));
    }
    // Named per release rather than pooled: two releases may each ignore a
    // different key, and one merged list would not say which.
    for release in releases {
        if !release.unknown_fields.is_empty() {
            parts.push(format!(
                "`{}` fields ignored (not of that kind of release): {}",
                release.kind,
                release.unknown_fields.join(", ")
            ));
        }
    }
    parts.join(", ")
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
