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

use rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::assistant::apply;
use crate::assistant::proposal::{self, PackagedNote, PackagedVersion, Proposal};
use crate::error::{Error, Result};
use crate::journal::{self, Record};
use crate::link;
use crate::note::{self, NoteFilter};
use crate::work::version;
use crate::work::{self, WorkFilter};
use crate::{assistant, profile, release, score, search};

const PROTOCOL_VERSION: &str = "2024-11-05";
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

/// How the chat a proposal lands in is named when the client did not say
/// who it is.
const UNNAMED_CLIENT: &str = "An agent";

/// Serves the workspace on stdin/stdout until the client closes the stream.
///
/// The database is opened once: a session is a long conversation, and
/// re-opening the file for every call would only add failure modes. The
/// client's name, from `initialize`, is remembered so its proposals land in
/// a chat that says who made them.
pub fn serve(conn: &Connection) -> Result<()> {
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
                eprintln!("kilna --mcp: ignoring a line that is not JSON: {err}");
                continue;
            }
        };
        let Some(response) = handle(conn, &mut session, &request) else {
            continue;
        };
        writeln!(stdout, "{response}")?;
        stdout.flush()?;
    }
}

/// What one connection knows about the client on the other end.
#[derive(Debug, Default)]
pub struct Session {
    /// The client's name from `initialize` — "Claude Code", say. Names the
    /// chat its proposals go into.
    pub client: Option<String>,
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
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "kilna", "version": env!("CARGO_PKG_VERSION") },
        "instructions": "The workspace of a content maker: works, their versions, scores along the \
    craft's axes, releases in a calendar, notes. Start with `workspace` to learn the craft's \
    vocabulary (roles, axes, fields, statuses), then `catalogue` and `work` to find and read. You \
    cannot write: you PROPOSE, and the person applies it in kilna with one click, or does not. \
    `propose_work` proposes a whole new work — title, kind, overview fields, versions by role, a \
    score, notes — or, given `work`, a package of those for an existing one; prefer it whenever \
    you have more than one thing to say about a work, so the person applies it all at once. \
    `propose_version`, `propose_score` and `propose_note` propose one thing each. Name works by \
    id when you have one; an exact title works too.",
    })
}

/// One entry of the tool list, with the shape of its arguments.
fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": { "type": "object", "properties": properties, "required": required },
    })
}

fn work_arg() -> Value {
    json!({ "type": "string", "description": "A work: its id, or its exact title" })
}

fn text_arg(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

/// The tools, in the order a session uses them: learn the vocabulary, read,
/// then propose.
fn tools() -> Vec<Value> {
    vec![
        tool(
            "workspace",
            "The active profile: the craft's vocabulary, per kind of work. Each kind lists its              version roles and how each reads, its scoring axes with weights and scales, its              tiers, statuses and kinds of release; plus how many works there are. Read this              first: a work is judged and versioned in its own kind's keys, and the other tools              speak in those keys.",
            json!({}),
            &[],
        ),
        tool(
            "catalogue",
            "Every work in the profile with its verdict: id, title, kind, status, score total and \
             tier, whether the score is stale (the text changed since), releases out and scheduled, \
             when it was last touched. Filter by a substring of the title, a kind or a status.",
            json!({
                "query": text_arg("A case-insensitive substring of the title"),
                "kind": text_arg("A work kind key, as `workspace` lists them"),
                "status": text_arg("A status key, as `workspace` lists them"),
                "limit": { "type": "integer", "description": "At most this many rows; 200 by default" },
            }),
            &[],
        ),
        tool(
            "work",
            "One work as its card shows it: the fields and meta, tags, every version by role \
             (id, revision, label, length, which is current), the latest score with its axes, \
             the releases, how many notes, what it was made from (`sources`, with whether the \
             source has moved on since) and what was made from it (`derived`). Bodies are not \
             included — read one with `text`.",
            json!({ "work": work_arg() }),
            &["work"],
        ),
        tool(
            "text",
            "The body of a version: the current version of a role by default, or the version \
             named by id. Plain roles come back exactly as typed; markdown roles are markdown.",
            json!({
                "work": work_arg(),
                "role": text_arg("A version role key; the profile's first role when omitted"),
                "version": text_arg("A version id, to read a particular revision instead of the current one"),
            }),
            &["work"],
        ),
        tool(
            "scores",
            "The score history of a work, newest first: total, tier, the marks per axis, who \
             judged (empty is the author), the note, which version it judged.",
            json!({ "work": work_arg() }),
            &["work"],
        ),
        tool(
            "calendar",
            "Every release with a date, in calendar order: what, of which work, which kind, \
             scheduled for when, released when, the link. Past releases included; filter with \
             `from` to start at a day.",
            json!({ "from": text_arg("Only releases on or after this day, YYYY-MM-DD") }),
            &[],
        ),
        tool(
            "notes",
            "Notes in the profile, newest first — all of them, or those of one work.",
            json!({ "work": work_arg(), "query": text_arg("A substring of the title or body") }),
            &[],
        ),
        tool(
            "search",
            "Find anything by text: works by title, versions by body, notes, assistant replies. \
             Each hit names the work it belongs to.",
            json!({ "query": text_arg("What to look for") }),
            &["query"],
        ),
        tool(
            "propose_work",
            "Propose a whole work, or a package of changes to one, applied together with one \
             click. Without `work`: a NEW work — `title` and `kind` are required, plus any of \
             the rest. With `work`: a package for that work — `title` and `kind` are ignored, \
             the rest is added to it. `fields` are the overview fields of the profile (see \
             `workspace`: `fields`), by key — a premise, a mood, a tagline go here, not into a \
             note. `versions` are texts by role; on a new work they become its current versions, \
             on an existing one they wait beside the current. `score` is marks along the kind's \
             axes. `notes` are notes on the work. Unknown fields and axes are named and left \
             out; an unknown role or kind refuses the whole package. Nothing is written until \
             the person applies it.",
            json!({
                "work": text_arg("An existing work to package changes for: its id, or its exact title. Omit to propose a new work"),
                "title": text_arg("The title of the new work; required without `work`"),
                "kind": text_arg("The kind of the new work, as `workspace` lists them; required without `work`"),
                "fields": { "type": "object", "description": "Overview field key to value, as `workspace` lists the fields", "additionalProperties": true },
                "versions": {
                    "type": "array",
                    "description": "Texts by role, each stored exactly as given",
                    "items": { "type": "object", "properties": {
                        "role": text_arg("The version role key, as `workspace` lists them"),
                        "body": text_arg("The whole text of the version"),
                        "label": text_arg("A short name for the version, optional"),
                    }, "required": ["role", "body"] },
                },
                "score": {
                    "type": "object",
                    "description": "Marks along the kind's axes, with an optional note",
                    "properties": {
                        "axes": { "type": "object", "additionalProperties": { "type": "number" } },
                        "note": text_arg("One sentence on why, optional"),
                    },
                    "required": ["axes"],
                },
                "notes": {
                    "type": "array",
                    "description": "Notes on the work",
                    "items": { "type": "object", "properties": {
                        "title": text_arg("A title for the note, optional"),
                        "body": text_arg("The note itself"),
                    }, "required": ["body"] },
                },
            }),
            &[],
        ),
        tool(
            "propose_version",
            "Propose a new version of a work in a role — a rewritten lyric, a style prompt. The \
             text lands in a chat on the work with an *insert as version* button; the person \
             decides. Nothing is written to the work until they do. Say in `note` what you \
             changed and why, briefly: it is shown beside the text.",
            json!({
                "work": work_arg(),
                "role": text_arg("The version role key, as `workspace` lists them"),
                "body": text_arg("The whole text of the proposed version, exactly as it should be stored"),
                "label": text_arg("A short name for the version, optional"),
                "note": text_arg("One or two sentences on what changed and why, optional"),
            }),
            &["work", "role", "body"],
        ),
        tool(
            "propose_score",
            "Propose a score along the profile's axes. Use the axis keys and scales from \
             `workspace`; every axis, no others. The proposal is shown with the numbers and an \
             apply button; the person decides. Nothing is written until they do.",
            json!({
                "work": work_arg(),
                "axes": { "type": "object", "description": "Axis key to mark, within the axis scale", "additionalProperties": { "type": "number" } },
                "note": text_arg("One sentence on why, optional"),
            }),
            &["work", "axes"],
        ),
        tool(
            "propose_note",
            "Propose a note — on a work, or on nothing in particular. Lands in the chat with an \
             *add as note* button; the person decides.",
            json!({
                "work": work_arg(),
                "title": text_arg("A title for the note, optional"),
                "body": text_arg("The note itself"),
            }),
            &["body"],
        ),
    ]
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

/// The active profile, or the sentence an agent can act on.
fn active(conn: &Connection) -> Result<profile::Profile> {
    profile::active(conn)?
        .ok_or_else(|| Error::Other("no profile is active in this workspace".into()))
}

fn arg<'a>(args: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn required<'a>(args: &'a Map<String, Value>, key: &str) -> Result<&'a str> {
    arg(args, key).ok_or_else(|| Error::Other(format!("`{key}` is required and cannot be empty")))
}

/// The work an argument names: by id first, then by exact title within the
/// active profile. A title shared by two works is refused rather than
/// guessed, with the ids to choose from.
fn find_work(conn: &Connection, profile_id: &str, named: &str) -> Result<work::Work> {
    if let Some(found) = work::get(conn, named)? {
        return Ok(found);
    }
    let same_title: Vec<work::Work> = work::list(conn, profile_id, &WorkFilter::default())?
        .into_iter()
        .filter(|w| w.title == named)
        .collect();
    match same_title.len() {
        0 => Err(Error::Other(format!(
            "no work with id or title `{named}`; `catalogue` lists them"
        ))),
        1 => Ok(same_title.into_iter().next().expect("one")),
        _ => Err(Error::Other(format!(
            "{} works are titled `{named}`; name one by id: {}",
            same_title.len(),
            same_title
                .iter()
                .map(|w| w.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn pretty(value: &impl serde::Serialize) -> Result<String> {
    Ok(serde_json::to_string_pretty(value)?)
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
                .map(|f| json!({ "key": f.key, "label": f.label, "type": f.field_type }))
                .collect();
            let kinds: Vec<Value> = config
                .work_kinds
                .iter()
                .map(|kind| {
                    json!({
                        "key": kind.key,
                        "label": kind.label,
                        "version_roles": kind.version_roles,
                        "axes": kind.axes.iter().map(|a| json!({
                            "key": a.key, "label": a.label, "weight": a.weight, "scale": a.scale,
                            "description": a.description,
                        })).collect::<Vec<_>>(),
                        "tiers": kind.tiers,
                        "statuses": kind.statuses.iter().map(|s| json!({ "key": s.key, "label": s.label })).collect::<Vec<_>>(),
                        "release_kinds": kind.release_kinds.iter().map(|k| json!({ "key": k.key, "label": k.label })).collect::<Vec<_>>(),
                    })
                })
                .collect();
            pretty(&json!({
                "profile": { "key": profile.key, "name": profile.name, "description": profile.description },
                "works": works,
                "fields": fields,
                "work_kinds": kinds,
            }))
        }

        "catalogue" => {
            let query = arg(args, "query").map(|q| q.to_lowercase());
            let kind = arg(args, "kind");
            let status = arg(args, "status");
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(200) as usize;
            let rows: Vec<Value> = score::catalogue(conn, &profile.id)?
                .into_iter()
                .filter(|row| query.as_ref().is_none_or(|q| row.title.to_lowercase().contains(q)))
                .filter(|row| kind.is_none_or(|k| row.kind == k))
                .filter(|row| status.is_none_or(|s| row.status == s))
                .take(limit)
                .map(|row| json!({
                    "id": row.work_id, "title": row.title, "kind": row.kind, "status": row.status,
                    "total": row.total, "tier": row.tier, "tier_pinned": row.tier_pinned,
                    "stale": row.stale, "released": row.released, "scheduled": row.scheduled,
                    "scored_at": row.scored_at, "updated_at": row.updated_at,
                }))
                .collect();
            pretty(&rows)
        }

        "work" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            let versions = version::list(conn, &found.id)?;
            let mut by_role: BTreeMap<String, Vec<Value>> = BTreeMap::new();
            for v in versions {
                by_role.entry(v.role.clone()).or_default().push(json!({
                    "id": v.id, "revision": v.revision, "label": v.label, "length": v.length,
                    "is_current": v.is_current, "created_at": v.created_at,
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
            pretty(&json!({
                "id": found.id, "title": found.title, "kind": found.kind, "status": found.status,
                "meta": found.meta, "tags": found.tags, "marks": found.marks,
                "tier_pinned": found.tier_pinned, "tier_pin_reason": found.tier_pin_reason,
                "created_at": found.created_at, "updated_at": found.updated_at,
                "versions": by_role,
                "latest_score": latest,
                "releases": releases,
                "notes": notes,
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
                        return Err(Error::Other(format!(
                            "version `{id}` belongs to another work"
                        )));
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
                                Error::Other(format!("`{}` names no version roles", found.kind))
                            })?,
                    };
                    if !vocabulary.version_roles.iter().any(|r| r.key == role) {
                        return Err(Error::Other(format!(
                            "no version role `{role}` for `{}`; `workspace` lists them",
                            found.kind
                        )));
                    }
                    // The current version when it is of this role; otherwise
                    // the newest of the role — the same answer the panel gives.
                    let current = found
                        .current_version_id
                        .as_deref()
                        .and_then(|id| version::get(conn, id).ok().flatten())
                        .filter(|v| v.role == role);
                    match current {
                        Some(v) => v,
                        None => version::latest(conn, &found.id, &role)?.ok_or_else(|| {
                            Error::Other(format!("“{}” has no `{role}` version yet", found.title))
                        })?,
                    }
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
            let rows: Vec<release::ScheduledRelease> = release::calendar(conn, &profile.id)?
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
            pretty(&rows)
        }

        "notes" => {
            let work_id = match arg(args, "work") {
                Some(named) => Some(find_work(conn, &profile.id, named)?.id),
                None => None,
            };
            let filter = NoteFilter {
                work_id,
                search: arg(args, "query").map(str::to_owned),
                ..NoteFilter::default()
            };
            pretty(&note::list(conn, &profile.id, &filter)?)
        }

        "search" => pretty(&search::find(conn, &profile.id, required(args, "query")?)?),

        "propose_version" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            let role = required(args, "role")?;
            if !config
                .vocabulary(&found.kind)
                .version_roles
                .iter()
                .any(|r| r.key == role)
            {
                return Err(Error::Other(format!(
                    "no version role `{role}` for `{}`; `workspace` lists them",
                    found.kind
                )));
            }
            let body = required(args, "body")?;
            let proposal = Proposal::Version {
                role: role.to_owned(),
                label: arg(args, "label").map(str::to_owned),
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
                "Proposed a `{role}` version for “{}”. It is in the chat on the work, waiting to be inserted — or not.",
                found.title
            ))
        }

        "propose_score" => {
            let found = find_work(conn, &profile.id, required(args, "work")?)?;
            let axes = args
                .get("axes")
                .and_then(Value::as_object)
                .cloned()
                .ok_or_else(|| {
                    Error::Other("`axes` must be an object of axis key to mark".into())
                })?;
            let note = arg(args, "note").map(str::to_owned);
            let vocabulary = config.vocabulary(&found.kind);
            let proposal =
                proposal::score_from(axes, note.clone(), vocabulary).ok_or_else(|| {
                    Error::Other(format!(
                        "none of those axes belongs to `{}`; its axes are {}",
                        found.kind,
                        vocabulary
                            .axes
                            .iter()
                            .map(|a| a.key.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
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
                    let title = required(args, "title").map_err(|_| {
                        Error::Other("a new work needs a `title`; name `work` to package changes for an existing one".into())
                    })?;
                    let kind = required(args, "kind").map_err(|_| {
                        Error::Other("a new work needs a `kind`; `workspace` lists them".into())
                    })?;
                    (Some(title.to_owned()), kind.to_owned())
                }
            };
            if config.kind(&kind).is_none() {
                return Err(Error::Other(format!(
                    "no kind of work `{kind}`; `workspace` lists them"
                )));
            }
            let vocabulary = config.vocabulary(&kind);

            let (fields, unknown_fields) = proposal::fields_from(
                args.get("fields")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default(),
                &config,
            );

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
                    return Err(Error::Other(format!(
                        "no version role `{role}` for `{kind}`; `workspace` lists them"
                    )));
                }
                versions.push(PackagedVersion {
                    role: role.to_owned(),
                    body: required(&item, "body")?.to_owned(),
                    label: arg(&item, "label").map(str::to_owned),
                });
            }

            let score = match args.get("score").and_then(Value::as_object) {
                Some(raw) => {
                    let axes = raw
                        .get("axes")
                        .and_then(Value::as_object)
                        .cloned()
                        .ok_or_else(|| {
                            Error::Other(
                                "`score.axes` must be an object of axis key to mark".into(),
                            )
                        })?;
                    let note = arg(raw, "note").map(str::to_owned);
                    Some(proposal::marks_from(axes, note, vocabulary).ok_or_else(|| {
                        Error::Other(format!(
                            "none of the score's axes belongs to `{kind}`; its axes are {}",
                            vocabulary
                                .axes
                                .iter()
                                .map(|a| a.key.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
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
                });
            }

            if found.is_some()
                && fields.is_empty()
                && versions.is_empty()
                && score.is_none()
                && notes.is_empty()
            {
                return Err(Error::Other(
                    "the package is empty: give `fields`, `versions`, `score` or `notes`".into(),
                ));
            }

            let proposal = Proposal::Work {
                title,
                work_kind: Some(kind.clone()),
                fields,
                unknown_fields,
                versions,
                score,
                notes,
            };
            let body = apply::render_package(&proposal, &config, &kind);
            let summary = package_summary(&proposal);
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

        "propose_note" => {
            let found = match arg(args, "work") {
                Some(named) => Some(find_work(conn, &profile.id, named)?),
                None => None,
            };
            let body = required(args, "body")?;
            let proposal = Proposal::Note {
                title: arg(args, "title").map(str::to_owned),
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

        other => Err(Error::Other(format!(
            "no tool named `{other}`; the tools are workspace, catalogue, work, text, scores, \
             calendar, notes, search, propose_work, propose_version, propose_score, propose_note"
        ))),
    }
}

/// Put a proposal where a person will see it: an assistant message in the
/// chat named after the client, on the work when there is one.
///
/// The chat is the client's, one per work, so a week of proposals from one
/// agent reads as one conversation rather than twenty chats named
/// "Proposal". The message body is the proposed text itself for a version —
/// that is what *insert as version* keeps, verbatim — and the reasoning goes
/// beside it in `meta.note`. A line in the journal says it arrived, so the
/// bell in the corner counts it.
fn deliver(
    conn: &Connection,
    profile_id: &str,
    session: &Session,
    work: Option<&work::Work>,
    proposal: Proposal,
    body: &str,
    note: Option<&str>,
) -> Result<assistant::Message> {
    let client = session
        .client
        .clone()
        .unwrap_or_else(|| UNNAMED_CLIENT.to_owned());
    let work_id = work.map(|w| w.id.as_str());

    let existing = assistant::summaries(conn, profile_id, work_id)?
        .into_iter()
        .find(|c| c.work_id.as_deref() == work_id && c.title.as_deref() == Some(client.as_str()));
    let chat_id = match existing {
        Some(chat) => chat.id,
        None => {
            assistant::create(
                conn,
                profile_id,
                assistant::NewChat {
                    work_id: work_id.map(str::to_owned),
                    title: Some(client.clone()),
                },
            )?
            .id
        }
    };

    let meta = apply::proposal_meta(&client, &proposal, note)?;
    let message = assistant::append(conn, &chat_id, assistant::ASSISTANT, body, meta)?;

    // Literal keys, one per sentence in the locale: the journal gate reads
    // them out of the source. A note on nothing has no title to name, and a
    // sentence with an empty quotation in it reads as a bug, so it gets a
    // sentence of its own.
    let record = match (&proposal, work) {
        (Proposal::Version { .. }, _) => Record::new("proposal.version"),
        (Proposal::Score { .. }, _) => Record::new("proposal.score"),
        (Proposal::Note { .. }, Some(_)) => Record::new("proposal.note"),
        (Proposal::Note { .. }, None) => Record::new("proposal.freeNote"),
        (Proposal::Work { .. }, Some(_)) => Record::new("proposal.package"),
        // A work that does not exist yet has no row to file the line under;
        // the title it would have is the one thing the sentence can name.
        (Proposal::Work { title, .. }, None) => {
            Record::new("proposal.work").param("title", title.clone().unwrap_or_default())
        }
    };
    let mut record = record.param("client", client);
    if let Some(w) = work {
        record = record
            .param("title", w.title.clone())
            .about("work", w.id.clone());
    }
    journal::record(conn, profile_id, record);

    Ok(message)
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
    if !unknown_fields.is_empty() {
        parts.push(format!(
            "fields ignored (not in the profile): {}",
            unknown_fields.join(", ")
        ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::work::NewWork;

    fn workspace() -> (Connection, String) {
        let mut conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        let work_id = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id;
        version::create(
            &mut conn,
            &work_id,
            serde_json::from_value(json!({ "role": "lyrics", "body": "one line\ntwo lines" }))
                .unwrap(),
        )
        .unwrap();
        (conn, work_id)
    }

    fn claude() -> Session {
        Session {
            client: Some("Claude Code".into()),
        }
    }

    fn args(value: Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap()
    }

    #[test]
    fn initialize_names_the_protocol_the_build_and_remembers_the_client() {
        let (conn, _) = workspace();
        let mut session = Session::default();
        let response = handle(
            &conn,
            &mut session,
            &json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
                     "params": { "clientInfo": { "name": "Claude Code", "version": "1" } } }),
        )
        .unwrap();
        assert_eq!(response["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(
            response["result"]["serverInfo"]["version"],
            env!("CARGO_PKG_VERSION")
        );
        assert!(response["result"]["capabilities"]["tools"].is_object());
        assert_eq!(session.client.as_deref(), Some("Claude Code"));
    }

    #[test]
    fn a_notification_is_not_answered() {
        let (conn, _) = workspace();
        let mut session = Session::default();
        let note = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(handle(&conn, &mut session, &note).is_none());
    }

    #[test]
    fn an_unknown_method_is_a_protocol_error_not_a_crash() {
        let (conn, _) = workspace();
        let mut session = Session::default();
        let response = handle(
            &conn,
            &mut session,
            &json!({ "jsonrpc": "2.0", "id": 1, "method": "resources/list" }),
        )
        .unwrap();
        assert_eq!(response["error"]["code"], METHOD_NOT_FOUND);
    }

    #[test]
    fn every_tool_describes_what_it_requires() {
        for tool in tools() {
            let schema = &tool["inputSchema"];
            for name in schema["required"].as_array().unwrap() {
                let name = name.as_str().unwrap();
                assert!(
                    schema["properties"].get(name).is_some(),
                    "{} requires `{name}` without describing it",
                    tool["name"]
                );
            }
            assert!(
                tool["description"].as_str().is_some_and(|d| d.len() > 60),
                "{} needs a description a model can act on",
                tool["name"]
            );
        }
    }

    #[test]
    fn a_work_is_found_by_id_or_by_exact_title_and_read_with_its_text() {
        let (conn, work_id) = workspace();
        let by_title = run_tool(
            &conn,
            &claude(),
            "work",
            &args(json!({ "work": "Harbour lights" })),
        )
        .unwrap();
        let by_id = run_tool(&conn, &claude(), "work", &args(json!({ "work": work_id }))).unwrap();
        assert_eq!(by_title, by_id);
        let card: Value = serde_json::from_str(&by_id).unwrap();
        assert_eq!(card["versions"]["lyrics"][0]["revision"], 1);
        assert!(card.get("body").is_none(), "the card must not carry bodies");

        let text = run_tool(&conn, &claude(), "text", &args(json!({ "work": work_id }))).unwrap();
        let text: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(text["body"], "one line\ntwo lines");
        assert_eq!(text["reads_as"], "plain");
    }

    #[test]
    fn a_missing_work_answers_the_agent_rather_than_the_client() {
        let (conn, _) = workspace();
        let call = json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/call",
                           "params": { "name": "text", "arguments": { "work": "nowhere" } } });
        let response = handle(&conn, &mut claude(), &call).unwrap();
        assert!(response["error"].is_null(), "{response}");
        assert_eq!(response["result"]["isError"], true);
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(
            text.contains("no work with id or title `nowhere`"),
            "{text}"
        );
    }

    #[test]
    fn a_proposed_version_is_a_message_in_the_clients_chat_not_a_version() {
        let (conn, work_id) = workspace();
        let before = version::list(&conn, &work_id).unwrap().len();

        run_tool(
            &conn,
            &claude(),
            "propose_version",
            &args(json!({ "work": work_id, "role": "lyrics", "body": "one line\ntwo lines\nthree", "note": "added a third" })),
        )
        .unwrap();

        assert_eq!(
            version::list(&conn, &work_id).unwrap().len(),
            before,
            "a proposal is not a version"
        );
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        let chats = assistant::summaries(&conn, &profile_id, Some(&work_id)).unwrap();
        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].title.as_deref(), Some("Claude Code"));
        let transcript = assistant::transcript(&conn, &chats[0].id).unwrap().unwrap();
        let message = &transcript.messages[0];
        assert_eq!(message.role, "assistant");
        assert_eq!(
            message.body, "one line\ntwo lines\nthree",
            "the body is the text, verbatim"
        );
        assert_eq!(message.meta["proposal"]["kind"], "version");
        assert_eq!(message.meta["proposal"]["role"], "lyrics");
        assert_eq!(message.meta["note"], "added a third");
        assert_eq!(message.meta["source"], "mcp");
    }

    #[test]
    fn a_second_proposal_joins_the_same_chat() {
        let (conn, work_id) = workspace();
        for body in ["first", "second"] {
            run_tool(
                &conn,
                &claude(),
                "propose_version",
                &args(json!({ "work": work_id, "role": "lyrics", "body": body })),
            )
            .unwrap();
        }
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        let chats = assistant::summaries(&conn, &profile_id, Some(&work_id)).unwrap();
        assert_eq!(chats.len(), 1, "one chat per client per work");
        let transcript = assistant::transcript(&conn, &chats[0].id).unwrap().unwrap();
        assert_eq!(transcript.messages.len(), 2);
    }

    #[test]
    fn a_proposed_score_is_checked_against_the_profile_and_never_written() {
        let (conn, work_id) = workspace();
        let answer = run_tool(
            &conn,
            &claude(),
            "propose_score",
            &args(json!({ "work": work_id, "axes": { "hook": 7, "lyrics": 11, "sparkle": 3 }, "note": "strong chorus" })),
        )
        .unwrap();
        assert!(
            answer.contains("sparkle"),
            "unknown axes are named: {answer}"
        );
        assert!(
            score::history(&conn, &work_id).unwrap().is_empty(),
            "a proposal is not a score"
        );

        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        let chat = assistant::summaries(&conn, &profile_id, Some(&work_id))
            .unwrap()
            .remove(0);
        let message = assistant::transcript(&conn, &chat.id)
            .unwrap()
            .unwrap()
            .messages
            .remove(0);
        assert_eq!(message.meta["proposal"]["kind"], "score");
        assert_eq!(message.meta["proposal"]["axes"]["hook"], 7.0);
        assert_eq!(
            message.meta["proposal"]["axes"]["lyrics"], 10.0,
            "clamped to the scale"
        );
        assert_eq!(message.meta["proposal"]["unknown"][0], "sparkle");
    }

    #[test]
    fn a_score_naming_no_known_axis_is_refused() {
        let (conn, work_id) = workspace();
        let err = run_tool(
            &conn,
            &claude(),
            "propose_score",
            &args(json!({ "work": work_id, "axes": { "sparkle": 3 } })),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("none of those axes"), "{err}");
    }

    #[test]
    fn a_proposal_leaves_a_line_in_the_journal() {
        let (conn, work_id) = workspace();
        run_tool(
            &conn,
            &claude(),
            "propose_note",
            &args(json!({ "work": work_id, "title": "Bridge idea", "body": "try a key change" })),
        )
        .unwrap();
        let lines: i64 = conn
            .query_row(
                "SELECT count(*) FROM journal WHERE action = 'proposal.note' AND entity_id = ?1",
                [&work_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(lines, 1);
    }

    #[test]
    fn a_proposal_from_an_unnamed_client_still_lands_somewhere() {
        let (conn, work_id) = workspace();
        run_tool(
            &conn,
            &Session::default(),
            "propose_note",
            &args(json!({ "work": work_id, "body": "anonymous" })),
        )
        .unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        let chat = assistant::summaries(&conn, &profile_id, Some(&work_id))
            .unwrap()
            .remove(0);
        assert_eq!(chat.title.as_deref(), Some(UNNAMED_CLIENT));
    }

    #[test]
    fn a_proposed_work_waits_in_the_clients_chat_on_nothing_with_everything_readable() {
        let (conn, _) = workspace();
        let answer = run_tool(
            &conn,
            &claude(),
            "propose_work",
            &args(json!({
                "title": "Winter road", "kind": "song",
                "fields": { "premise": "a road at night", "colour": "blue" },
                "versions": [{ "role": "lyrics", "body": "snow on the road", "label": "first" }],
                "score": { "axes": { "hook": 7, "sparkle": 2 }, "note": "quiet" },
                "notes": [{ "title": "Concept", "body": "the road as witness" }]
            })),
        )
        .unwrap();
        assert!(answer.contains("Proposed a new work"), "{answer}");
        assert!(
            answer.contains("colour"),
            "ignored fields are named: {answer}"
        );
        assert!(
            answer.contains("sparkle"),
            "ignored axes are named: {answer}"
        );

        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        assert_eq!(
            work::list(&conn, &profile_id, &WorkFilter::default())
                .unwrap()
                .len(),
            1,
            "a proposal is not a work"
        );
        let chats = assistant::summaries(&conn, &profile_id, None).unwrap();
        let chat = chats
            .iter()
            .find(|c| c.work_id.is_none())
            .expect("a chat on nothing");
        assert_eq!(chat.title.as_deref(), Some("Claude Code"));
        let message = assistant::transcript(&conn, &chat.id)
            .unwrap()
            .unwrap()
            .messages
            .remove(0);
        assert_eq!(message.meta["proposal"]["kind"], "work");
        assert_eq!(message.meta["proposal"]["title"], "Winter road");
        assert_eq!(
            message.meta["proposal"]["fields"]["premise"],
            "a road at night"
        );
        assert!(message.meta["proposal"]["fields"].get("colour").is_none());
        assert_eq!(
            message.meta["proposal"]["versions"][0]["body"],
            "snow on the road"
        );
        assert_eq!(message.meta["proposal"]["score"]["axes"]["hook"], 7.0);
        assert!(
            message.body.contains("snow on the road"),
            "readable before applied: {}",
            message.body
        );
        assert!(message.body.contains("Winter road"), "{}", message.body);
        assert!(apply::is_pending(&message));

        let lines: i64 = conn
            .query_row(
                "SELECT count(*) FROM journal WHERE action = 'proposal.work'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(lines, 1);
    }

    #[test]
    fn a_package_for_a_work_joins_its_chat() {
        let (conn, work_id) = workspace();
        let answer = run_tool(
            &conn,
            &claude(),
            "propose_work",
            &args(
                json!({ "work": work_id, "title": "ignored", "kind": "video",
                           "versions": [{ "role": "style", "body": "brushed drums" }] }),
            ),
        )
        .unwrap();
        assert!(
            answer.contains("Proposed a package for “Harbour lights”"),
            "{answer}"
        );

        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        let chats = assistant::summaries(&conn, &profile_id, Some(&work_id)).unwrap();
        assert_eq!(chats.len(), 1);
        let message = assistant::transcript(&conn, &chats[0].id)
            .unwrap()
            .unwrap()
            .messages
            .remove(0);
        assert_eq!(message.meta["proposal"]["kind"], "work");
        assert!(
            message.meta["proposal"].get("title").is_none(),
            "an existing work keeps its title"
        );
        assert_eq!(message.meta["proposal"]["kind"], "work");
        assert_eq!(message.meta["proposal"]["versions"][0]["role"], "style");
    }

    #[test]
    fn a_package_that_names_a_role_the_kind_lacks_is_refused_whole() {
        let (conn, work_id) = workspace();
        let err = run_tool(
            &conn,
            &claude(),
            "propose_work",
            &args(json!({ "work": work_id, "versions": [{ "role": "storyboard", "body": "x" }] })),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("storyboard"), "{err}");
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        assert!(
            assistant::summaries(&conn, &profile_id, Some(&work_id))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_new_work_needs_a_title_and_a_kind_the_profile_has() {
        let (conn, _) = workspace();
        let err = run_tool(
            &conn,
            &claude(),
            "propose_work",
            &args(json!({ "kind": "song" })),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("`title`"), "{err}");
        let err = run_tool(
            &conn,
            &claude(),
            "propose_work",
            &args(json!({ "title": "x", "kind": "sculpture" })),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("sculpture"), "{err}");
    }

    #[test]
    fn an_empty_package_for_a_work_is_refused() {
        let (conn, work_id) = workspace();
        let err = run_tool(
            &conn,
            &claude(),
            "propose_work",
            &args(json!({ "work": work_id })),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("empty"), "{err}");
    }

    #[test]
    fn the_workspace_lists_the_overview_fields() {
        let (conn, _) = workspace();
        let answer = run_tool(&conn, &claude(), "workspace", &args(json!({}))).unwrap();
        let answer: Value = serde_json::from_str(&answer).unwrap();
        assert!(answer["fields"].as_array().is_some_and(|f| !f.is_empty()));
        assert!(answer["fields"][0]["key"].is_string());
    }

    #[test]
    fn the_registration_command_quotes_the_executable() {
        let command = registration_command().unwrap();
        assert!(
            command.starts_with("claude mcp add -s user kilna -- \""),
            "{command}"
        );
        assert!(command.ends_with("\" --mcp"), "{command}");
    }
}
