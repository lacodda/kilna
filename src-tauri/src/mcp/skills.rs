//! The skills of a profile, served two ways: read by the `guide` tool, and
//! offered as the client's own prompts (FR-17).

use rusqlite::Connection;
use serde_json::{Value, json};

use super::{Failure, INVALID_PARAMS, active, find_work};

/// The skills of a profile as an agent picks among them: what each is, when
/// to reach for it, what it is for and about, and what it answers with -
/// never its method, which `guide` opens one at a time (FR-17).
pub(super) fn guides(
    config: &crate::profile::config::ProfileConfig,
    kind: Option<&str>,
    scope: Option<&str>,
) -> Vec<Value> {
    config
        .prompts
        .iter()
        .filter(|p| kind.is_none_or(|kind| p.applies_to(kind)))
        .filter(|p| scope.is_none_or(|scope| scope_word(p) == scope))
        .map(|p| {
            json!({
                "key": p.key, "label": p.label, "description": p.description,
                "when": p.when, "category": p.category, "kinds": p.kinds,
                "scope": scope_word(p), "produces": p.produces,
                "show_in_ui": p.show_in_ui, "has_method": p.method().is_some(),
            })
        })
        .collect()
}

/// What a skill is about, as one word: `work` when it says nothing.
fn scope_word(prompt: &crate::assistant::prompt::PromptTemplate) -> &str {
    prompt
        .scope
        .as_deref()
        .map(str::trim)
        .filter(|scope| !scope.is_empty())
        .unwrap_or("work")
}

/// How an agent hands back what a skill produces: through the tool that
/// proposes that thing, or as prose for the person to read.
fn answer_with(prompt: &crate::assistant::prompt::PromptTemplate) -> String {
    use crate::assistant::prompt::Produces;
    match prompt.produces() {
        Produces::Prose => "prose: answer the person in the conversation".into(),
        Produces::Score => "`propose_score`, along the kind's axes".into(),
        Produces::Version(role) => format!("`propose_version` in the role `{role}`"),
        Produces::Scenes(_) => "`propose_scenes`".into(),
        Produces::Canon => "`propose_canon`".into(),
        Produces::Words => "`propose_words`".into(),
        Produces::Release => "`propose_release`".into(),
        Produces::CoverIdeas => "`propose_cover`".into(),
        Produces::Trials => "`propose_trials`".into(),
        Produces::Cards(board) => format!("`propose_cards` onto the board `{board}`"),
        Produces::Conception => {
            "`propose_work` with the idea field and the brief as a version".into()
        }
        Produces::Comment
        | Produces::Reply
        | Produces::Description
        | Produces::CardPrompt
        | Produces::CardNote
        | Produces::Bricks => "prose: kilna applies this answer only from the window".into(),
    }
}

/// One skill whole, with what its method leans on in this space: the axes it
/// judges by and their rubrics, the roles it writes into, the statuses, the
/// doors, the boards and the style types - read live, never copied into an
/// agent's own files.
pub(super) fn skill_document(
    config: &crate::profile::config::ProfileConfig,
    prompt: &crate::assistant::prompt::PromptTemplate,
    kind: Option<&str>,
) -> Value {
    let kinds: Vec<Value> = config
        .work_kinds
        .iter()
        .filter(|k| kind.map_or_else(|| prompt.applies_to(&k.key), |kind| k.key == kind))
        .map(|k| {
            json!({
                "key": k.key, "label": k.label,
                "axes": k.axes, "tiers": k.tiers,
                "version_roles": k.version_roles.iter().map(|r| json!({ "key": r.key, "label": r.label })).collect::<Vec<_>>(),
                "statuses": k.statuses.iter().map(|s| json!({ "key": s.key, "label": s.label })).collect::<Vec<_>>(),
                "release_kinds": k.release_kinds.iter().map(|r| json!({ "key": r.key, "label": r.label })).collect::<Vec<_>>(),
                "conception": k.conception,
            })
        })
        .collect();
    let mut entry = guides(config, None, None)
        .into_iter()
        .find(|g| g["key"] == prompt.key.as_str())
        .unwrap_or_default();
    if let Some(entry) = entry.as_object_mut() {
        entry.remove("has_method");
    }
    json!({
        "skill": entry,
        "method": prompt.method(),
        "message": prompt.sends().then_some(prompt.template.as_str()),
        "answer_with": answer_with(prompt),
        "workspace_context": {
            "kinds": kinds,
            "boards": config.boards.iter().map(|b| json!({ "key": b.key, "label": b.label })).collect::<Vec<_>>(),
            "style_types": config.style_types.iter().filter(|t| t.retired.is_none()).map(|t| json!({ "key": t.key, "label": t.label })).collect::<Vec<_>>(),
            "reading": "`catalogue`, `work`, `text`, `scores`, `canon`, `notes`, `register`, `brief`, `search`",
            "proposing": "the `propose_*` tools: you propose, the person applies it in kilna",
        },
    })
}

/// The profile's skills as MCP prompts: each by its key, with the work it is
/// about as its one argument.
pub(super) fn list_prompts(conn: &Connection) -> std::result::Result<Value, Failure> {
    let profile = active(conn).map_err(|err| Failure {
        code: INVALID_PARAMS,
        message: err.to_string(),
    })?;
    let prompts: Vec<Value> = profile
        .config
        .prompts
        .iter()
        .map(|p| {
            let said = p
                .when
                .as_ref()
                .or(p.description.as_ref())
                .map(|label| label.as_str().to_owned());
            json!({
                "name": p.key,
                "title": p.label.as_str(),
                "description": said,
                "arguments": [{
                    "name": "work",
                    "description": "The work it is about, by id or exact title; the method alone when left out",
                    "required": false,
                }],
            })
        })
        .collect();
    Ok(json!({ "prompts": prompts }))
}

/// One skill as a prompt: its method, then its message filled from the work
/// when one is named - or the method alone, to be applied as the person asks.
pub(super) fn get_prompt(conn: &Connection, params: &Value) -> std::result::Result<Value, Failure> {
    let failed = |message: String| Failure {
        code: INVALID_PARAMS,
        message,
    };
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| failed("prompts/get needs a `name`".into()))?;
    let profile = active(conn).map_err(|err| failed(err.to_string()))?;
    let prompt = profile
        .config
        .prompts
        .iter()
        .find(|p| p.key == name)
        .ok_or_else(|| failed(format!("kilna has no skill keyed `{name}`")))?;
    let work = params
        .pointer("/arguments/work")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|w| !w.is_empty());

    let mut text = String::new();
    if let Some(method) = prompt.method() {
        text.push_str(method);
        text.push_str("\n\n---\n\n");
    }
    match (work, prompt.sends()) {
        (Some(named), true) => {
            let found = find_work(conn, &profile.id, named).map_err(|err| failed(err.to_string()))?;
            let rendered = crate::assistant::prompt::for_work(
                conn,
                &found.id,
                &prompt.template,
                crate::assistant::prompt::Context::default(),
            )
            .map_err(|err| failed(err.to_string()))?;
            text.push_str(&rendered);
        }
        (Some(named), false) => {
            let found = find_work(conn, &profile.id, named).map_err(|err| failed(err.to_string()))?;
            text.push_str(&format!(
                "Apply this to the work \"{}\" (id `{}`): read it with `work` and `text`, and propose \
                 through {}.",
                found.title,
                found.id,
                answer_with(prompt)
            ));
        }
        (None, _) => text.push_str(&format!(
            "Apply this to the work the person names: find it with `catalogue`, read it with `work` \
             and `text`, and propose through {}.",
            answer_with(prompt)
        )),
    }
    Ok(json!({
        "description": prompt.label.as_str(),
        "messages": [{ "role": "user", "content": { "type": "text", "text": text } }],
    }))
}
