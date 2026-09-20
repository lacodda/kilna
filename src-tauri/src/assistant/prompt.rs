use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::proposal::BoardChange;
use crate::error::{Error, Result};
use crate::link;
use crate::profile::config::{Label, ProfileConfig, WorkKind};
use crate::scene::{self, Scene};
use crate::work::{self, version};

/// A named action the panel offers, defined by the profile.
///
/// A template is a string with `{placeholders}` filled from the work in
/// context, so "Critique the lyrics" means the right thing in a music profile
/// and something else entirely in a novel one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub key: String,
    pub label: Label,
    pub template: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Label>,
    /// Name of the glyph the button is drawn with, from the fixed set the
    /// window knows. Carried, never read here: which picture goes with an
    /// action is a question for the screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// What the action asks the assistant to produce, beyond prose.
    ///
    /// Absent means an ordinary action: the answer is text and the reader
    /// decides what to do with it. `"score"` asks for values along the
    /// profile's axes, `"version:<role>"` for a text kept in that role,
    /// `"scenes"` for a storyboard — which kilna can offer to apply, with a
    /// confirmation, because the assistant never writes to the workspace
    /// itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produces: Option<String>,
    /// How the action is done: the role the assistant takes, what it checks
    /// and in what order, the shape of the answer, what it must never say.
    /// A method is the craft's own and lives in the profile so it ships with
    /// it and is edited with it; it reaches the model as a system
    /// instruction, not as part of the message. See ADR 0021.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// The kinds of work the action is for; empty means every kind. An
    /// action that reads `{role:lyrics}` is for the kinds that have lyrics,
    /// and a button for it on a video would send a prompt with a hole in it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<String>,
    /// What the action is about: `work` (absent) or `scene`. A scene action
    /// is started from a row of the storyboard, reads it as `{scene}`, and
    /// proposes a revision of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// What an action asks the answer to be, read off `produces`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Produces {
    /// Text, for a person to read.
    Prose,
    /// Marks along the kind's axes, in a block the application reads.
    Score,
    /// The whole answer, kept as a version in this role.
    Version(String),
    /// Scenes for the storyboard, in a block the application reads, doing
    /// this to the board that is there.
    Scenes(BoardChange),
}

/// What an action is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The work as it stands, or one of its versions.
    Work,
    /// One scene of the work's storyboard.
    Scene,
}

/// The value of `scope` that names a scene action.
pub const SCENE_SCOPE: &str = "scene";

impl PromptTemplate {
    /// `produces` as the application understands it. An unknown value reads
    /// as prose rather than failing: a profile written for a later kilna
    /// still loads, and its answer is still an answer. (Saving a profile is
    /// stricter — see `ProfileConfig::validate`.)
    pub fn produces(&self) -> Produces {
        match self.produces.as_deref().map(str::trim) {
            Some("score") => Produces::Score,
            Some("scenes") => Produces::Scenes(BoardChange::Replace),
            Some(value) => {
                if let Some(role) = value.strip_prefix("version:") {
                    return if role.trim().is_empty() {
                        Produces::Prose
                    } else {
                        Produces::Version(role.trim().to_owned())
                    };
                }
                match value.strip_prefix("scenes:").and_then(BoardChange::parse) {
                    Some(change) => Produces::Scenes(change),
                    None => Produces::Prose,
                }
            }
            None => Produces::Prose,
        }
    }

    /// Whether `produces` names something this build knows.
    pub fn produces_is_known(&self) -> bool {
        match self.produces.as_deref().map(str::trim) {
            None | Some("") => true,
            Some(_) => self.produces() != Produces::Prose,
        }
    }

    /// The method, when the action states one worth sending: blank is none.
    pub fn method(&self) -> Option<&str> {
        self.method
            .as_deref()
            .map(str::trim)
            .filter(|method| !method.is_empty())
    }

    /// What the action is about. Anything but `scene` is the work.
    pub fn scope(&self) -> Scope {
        match self.scope.as_deref().map(str::trim) {
            Some(SCENE_SCOPE) => Scope::Scene,
            _ => Scope::Work,
        }
    }

    /// Whether the action is offered on a work of this kind.
    pub fn applies_to(&self, kind: &str) -> bool {
        self.kinds.is_empty() || self.kinds.iter().any(|k| k == kind)
    }

    /// The placeholders the template reads, as written between the braces.
    pub fn placeholders(&self) -> Vec<String> {
        placeholders(&self.template)
    }
}

/// The placeholders a template reads: `{title}`, `{role:lyrics}`, …
///
/// Only what looks like a placeholder counts — a short name, optionally
/// with a `:` argument — so a brace in prose is prose. Each is reported
/// once.
pub fn placeholders(template: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('}') else { break };
        let inner = &rest[..close];
        let looks_like_one = !inner.is_empty()
            && inner
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ':')
            && inner.chars().next().is_some_and(|c| c.is_ascii_lowercase());
        if looks_like_one && !found.iter().any(|f| f == inner) {
            found.push(inner.to_owned());
        }
        rest = &rest[close + 1..];
    }
    found
}

/// Whether a placeholder is one the renderer fills. `{role:x}` and
/// `{donor:x}` are checked against a vocabulary elsewhere; here only the
/// name is judged.
pub fn is_known_placeholder(name: &str) -> bool {
    matches!(
        name,
        "title" | "kind" | "status" | "body" | "scenes" | "scene" | "donor" | "styles"
    ) || name.strip_prefix("role:").is_some_and(|r| !r.is_empty())
        || name.strip_prefix("donor:").is_some_and(|r| !r.is_empty())
}

/// Placeholders a template may use.
///
/// Unknown placeholders are left as written rather than blanked: a visible
/// `{typo}` in the prompt is easier to diagnose than a silent gap.
pub fn render(template: &str, values: &[(&str, String)]) -> String {
    let mut rendered = template.to_owned();
    for (key, value) in values {
        rendered = rendered.replace(&format!("{{{key}}}"), value);
    }
    rendered
}

/// What a prompt is rendered against, beyond the work: the version open on
/// the versions tab, and the scene the action was started on.
#[derive(Debug, Clone, Copy, Default)]
pub struct Context<'a> {
    /// The version the action is about — the one open on the versions tab.
    /// It stands in for `{body}` and for `{role:<its role>}`; without it the
    /// work's current version is `{body}` and every role reads its latest
    /// revision, which is what an action started from the overview means.
    pub version_id: Option<&'a str>,
    /// The scene the action is about, for `{scene}`.
    pub scene_id: Option<&'a str>,
    /// The style bricks the person picked, in the order they picked them, for
    /// `{styles}`. Chosen at the moment the action is started rather than
    /// stored on the work: which parts a picture is built from is the
    /// question being asked, and it is a different answer every time.
    pub style_brick_ids: &'a [String],
}

/// Build the prompt for `template` in the context of `work_id`.
///
/// Fails when the template reads a donor the work does not have: an action
/// "from the source" on a work made from nothing would otherwise send a
/// prompt with a silent hole where the source should be.
pub fn for_work(
    conn: &Connection,
    work_id: &str,
    template: &str,
    context: Context<'_>,
) -> Result<String> {
    let work = work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let kind = config.vocabulary(&work.kind);
    let wanted = placeholders(template);
    let wants = |name: &str| wanted.iter().any(|w| w == name);

    // The named version, or the current one: what the user is looking at.
    // Roles beyond it are fetched by name so a template can ask for the style
    // as well as the text.
    let named = match context.version_id {
        Some(id) => {
            let found = version::get(conn, id)?.ok_or_else(|| Error::not_found("version", id))?;
            if found.work_id != work.id {
                return Err(Error::Other(format!(
                    "version `{id}` is not a version of “{}”",
                    work.title
                )));
            }
            Some(found)
        }
        None => None,
    };
    let current = match (&named, &work.current_version_id) {
        (Some(named), _) => Some(named.clone()),
        (None, Some(id)) => version::get(conn, id)?,
        (None, None) => None,
    };

    let mut values: Vec<(&str, String)> = vec![
        ("title", work.title.clone()),
        ("kind", work.kind.clone()),
        ("status", work.status.clone()),
        (
            "body",
            current.as_ref().map(|v| v.body.clone()).unwrap_or_default(),
        ),
    ];

    // The board and the scene: read only when asked for, since a board is
    // pages of text and most actions never look at it.
    if wants("scenes") {
        let scenes = scene::for_work(conn, work_id)?;
        values.push(("scenes", board_table(&scenes, kind)));
    }
    if wants("scene") {
        let rendered = match context.scene_id {
            Some(id) => {
                let found = scene::get(conn, id)?.ok_or_else(|| Error::not_found("scene", id))?;
                if found.work_id != work.id {
                    return Err(Error::Other(format!(
                        "scene `{id}` is not a scene of “{}”",
                        work.title
                    )));
                }
                scene_sheet(&found, kind)
            }
            None => String::new(),
        };
        values.push(("scene", rendered));
    }

    // The bricks the person picked, each with the word of the craft that says
    // what it contributes. Read in the order they were picked: a person who
    // names the character first and the place second has said something about
    // which matters more, and re-sorting it would throw that away.
    if wants("styles") {
        let mut bricks = Vec::with_capacity(context.style_brick_ids.len());
        for id in context.style_brick_ids {
            let brick =
                crate::style_brick::get(conn, id)?.ok_or_else(|| Error::not_found("style", id))?;
            if brick.profile_id != work.profile_id {
                return Err(Error::Other(format!(
                    "style “{}” is not of this workspace",
                    brick.name
                )));
            }
            bricks.push(brick);
        }
        values.push(("styles", brick_sheet(&bricks, &config)));
    }

    let mut rendered = render(template, &values);

    // Every role the work has, latest revision, as {role:lyrics} and friends.
    let roles = version::list(conn, work_id)?;
    let mut seen: Vec<String> = Vec::new();
    for summary in roles {
        if seen.contains(&summary.role) {
            continue;
        }
        seen.push(summary.role.clone());
    }
    for role in &seen {
        // The named version is its role's text, whatever revision is latest:
        // a critique of revision 2 must read revision 2.
        if let Some(named) = named.as_ref().filter(|named| named.role == *role) {
            rendered = rendered.replace(&format!("{{role:{role}}}"), &named.body);
            continue;
        }
        if let Some(latest) = version::latest(conn, work_id, role)? {
            rendered = rendered.replace(&format!("{{role:{role}}}"), &latest.body);
        }
    }
    // A role of the kind the work has no version in yet is refused, not
    // rendered empty: a critique of nothing is not a critique, and the
    // person is told what to write first. A role the kind does not name
    // stays visible in the prompt, as any unknown placeholder does.
    for name in wanted.iter().filter_map(|w| w.strip_prefix("role:")) {
        if seen.iter().any(|role| role == name) {
            continue;
        }
        if let Some(role) = kind.version_roles.iter().find(|r| r.key == name) {
            return Err(Error::Other(format!(
                "“{}” has no {} yet: write it first",
                work.title, role.label
            )));
        }
    }

    // The donor: the first work this one was made from. Refused when the
    // template reads one and there is none, rather than rendered empty.
    if wanted
        .iter()
        .any(|w| w == "donor" || w.starts_with("donor:"))
    {
        let donor = link::sources(conn, work_id)?
            .into_iter()
            .find(|source| source.role == link::DONOR)
            .ok_or_else(|| {
                Error::Other(format!(
                    "“{}” is not made from anything yet: link its source on the Links tab first",
                    work.title
                ))
            })?;
        rendered = rendered.replace(
            "{donor}",
            &format!("“{}” ({})", donor.source_title, donor.source_kind),
        );
        for name in wanted.iter().filter_map(|w| w.strip_prefix("donor:")) {
            let body = version::latest(conn, &donor.source_id, name)?
                .map(|v| v.body)
                .unwrap_or_default();
            rendered = rendered.replace(&format!("{{donor:{name}}}"), &body);
        }
    }

    Ok(rendered)
}

/// The board as a table — number, section, seconds, kind of shot,
/// description — for a prompt that reads `{scenes}`. The blocks stay out:
/// a board of fifty scenes with three prompts each is not what an action
/// about the board's shape needs to read. An empty board says so.
pub fn board_table(scenes: &[Scene], kind: &WorkKind) -> String {
    if scenes.is_empty() {
        return "(the board is empty)".to_owned();
    }
    let has_shots = !kind.shot_types.is_empty();
    let mut out = String::from(if has_shots {
        "| # | Section | Time | Shot | Description |\n| --- | --- | --- | --- | --- |\n"
    } else {
        "| # | Section | Time | Description |\n| --- | --- | --- | --- |\n"
    });
    for scene in scenes {
        let cell = |text: &str| text.replace('|', "\\|").replace('\n', " ");
        let mut row = vec![
            scene.position.to_string(),
            cell(scene.section.as_deref().unwrap_or("")),
            span(scene.starts_at, scene.ends_at),
        ];
        if has_shots {
            row.push(cell(
                &scene
                    .shot_type
                    .as_deref()
                    .map_or(String::new(), |key| shot_label(kind, key)),
            ));
        }
        row.push(cell(&scene.description));
        out.push_str(&format!("| {} |\n", row.join(" | ")));
    }
    out.trim_end().to_owned()
}

/// One scene whole — its fields and every block it holds — for a prompt
/// that reads `{scene}`.
pub fn scene_sheet(scene: &Scene, kind: &WorkKind) -> String {
    let mut out = format!("Scene {}", scene.position);
    if let Some(section) = scene.section.as_deref().filter(|s| !s.is_empty()) {
        out.push_str(&format!(" · {section}"));
    }
    let time = span(scene.starts_at, scene.ends_at);
    if !time.is_empty() {
        out.push_str(&format!(" · {time}"));
    }
    if let Some(shot) = scene.shot_type.as_deref() {
        out.push_str(&format!(" · {}", shot_label(kind, shot)));
    }
    out.push('\n');
    if !scene.description.trim().is_empty() {
        out.push_str(&format!("\n{}\n", scene.description.trim()));
    }
    for block in &kind.scene_blocks {
        let Some(text) = scene
            .blocks
            .get(&block.key)
            .and_then(serde_json::Value::as_str)
            .filter(|t| !t.trim().is_empty())
        else {
            continue;
        };
        out.push_str(&format!(
            "\n{} (`{}`):\n{}\n",
            block.label,
            block.key,
            text.trim()
        ));
    }
    out.trim_end().to_owned()
}

/// The picked bricks, each under the craft's word for what it contributes.
///
/// The type's label leads the line because that is the whole of what makes a
/// dictionary of parts different from a heap of paragraphs: an assistant told
/// «Environment: a flooded car park at dusk» knows the sentence is the place
/// and not the person, and can therefore write one prompt rather than glue
/// three together. A brick with no description is still listed, by name: a
/// silent gap would read as "there is no character", and the person picked it
/// on purpose.
///
/// The type's `hint` is deliberately absent. It says what to write *about* a
/// brick, which is a question already answered by the time one is being used;
/// carrying it here would ask the generator to take notes.
pub fn brick_sheet(bricks: &[crate::style_brick::StyleBrick], config: &ProfileConfig) -> String {
    let mut out = String::new();
    for brick in bricks {
        let label = config
            .style_type(&brick.type_key)
            .map_or(brick.type_key.clone(), |kind| {
                kind.label.as_str().to_owned()
            });
        out.push_str(&format!("{} — {}", label, brick.name));
        match brick.description.as_deref().map(str::trim) {
            Some(text) if !text.is_empty() => out.push_str(&format!("\n{text}\n\n")),
            _ => out.push_str("\n(not described yet)\n\n"),
        }
    }
    out.trim_end().to_owned()
}

fn shot_label(kind: &WorkKind, key: &str) -> String {
    kind.shot_types
        .iter()
        .find(|s| s.key == key)
        .map_or(key.to_owned(), |s| s.label.as_str().to_owned())
}

fn span(starts_at: Option<f64>, ends_at: Option<f64>) -> String {
    match (starts_at, ends_at) {
        (Some(from), Some(to)) => format!("{}–{}", scene::timecode(from), scene::timecode(to)),
        (Some(from), None) => format!("{}–", scene::timecode(from)),
        (None, Some(to)) => format!("–{}", scene::timecode(to)),
        (None, None) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::link::{self, NewLink};
    use crate::profile;
    use crate::scene::{self, NewScene};
    use crate::work::NewWork;
    use crate::work::version::NewVersion;

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    #[test]
    fn placeholders_are_replaced() {
        let rendered = render(
            "Rewrite {title} in {kind}",
            &[("title", "Harbour lights".into()), ("kind", "song".into())],
        );

        assert_eq!(rendered, "Rewrite Harbour lights in song");
    }

    #[test]
    fn an_unknown_placeholder_is_left_visible() {
        let rendered = render("Check {typo}", &[("title", "x".into())]);

        assert_eq!(
            rendered, "Check {typo}",
            "a visible placeholder is easier to diagnose than a silent gap"
        );
    }

    /// The point of a typed dictionary: the assistant is told what each part
    /// contributes, so it can write one prompt instead of gluing three.
    #[test]
    fn picked_styles_reach_the_prompt_under_the_word_for_what_they_are() {
        let (conn, profile_id) = workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "video".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap();

        let place = crate::style_brick::create(
            &conn,
            &profile_id,
            crate::style_brick::NewStyleBrick {
                type_key: "environment".into(),
                name: "Flooded car park".into(),
                description: Some("Standing water to the ankles, sodium light.".into()),
                hint: Some("only the ground floor".into()),
            },
        )
        .unwrap();
        let who = crate::style_brick::create(
            &conn,
            &profile_id,
            crate::style_brick::NewStyleBrick {
                type_key: "character".into(),
                name: "The keeper".into(),
                description: Some("Sixty, weathered, a long grey coat.".into()),
                hint: None,
            },
        )
        .unwrap();

        let picked = [place.id.clone(), who.id.clone()];
        let prompt = for_work(
            &conn,
            &work.id,
            "Parts:

{styles}",
            Context {
                style_brick_ids: &picked,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(
            prompt.contains("Environment — Flooded car park"),
            "the type's label leads the part: {prompt}"
        );
        assert!(
            prompt.contains("Standing water to the ankles, sodium light."),
            "the description goes in verbatim: {prompt}"
        );
        assert!(
            prompt.contains("Character — The keeper"),
            "every picked part is there: {prompt}"
        );
        assert!(
            !prompt.contains("only the ground floor"),
            "the author's steer is about writing the description, not about the picture —              carrying it would ask the generator to take notes: {prompt}"
        );
        assert!(
            prompt.find("Flooded car park").unwrap() < prompt.find("The keeper").unwrap(),
            "the order picked is the order of importance, and is kept: {prompt}"
        );
    }

    /// A part with nothing written is still named. A silent gap would read as
    /// "there is no character", and the person picked it on purpose.
    #[test]
    fn a_style_with_no_description_is_listed_as_undescribed_rather_than_dropped() {
        let (conn, profile_id) = workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "video".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        let bare = crate::style_brick::create(
            &conn,
            &profile_id,
            crate::style_brick::NewStyleBrick {
                type_key: "character".into(),
                name: "The keeper".into(),
                description: None,
                hint: None,
            },
        )
        .unwrap();

        let picked = [bare.id.clone()];
        let prompt = for_work(
            &conn,
            &work.id,
            "{styles}",
            Context {
                style_brick_ids: &picked,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(prompt.contains("Character — The keeper"), "{prompt}");
        assert!(prompt.contains("not described yet"), "{prompt}");
    }

    /// Reading the dictionary costs a query per brick, so a template that
    /// never asks for it must not pay — the same rule the board follows.
    #[test]
    fn a_template_that_never_asks_for_styles_does_not_refuse_a_missing_one() {
        let (conn, profile_id) = workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "video".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap();

        let nonsense = ["no such brick".to_owned()];
        let prompt = for_work(
            &conn,
            &work.id,
            "About {title}.",
            Context {
                style_brick_ids: &nonsense,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(prompt, "About Harbour lights.");
    }

    /// A picked brick that is not there is a refusal, not a hole: the person
    /// chose it, and a prompt quietly missing a part is worse than none.
    #[test]
    fn a_style_that_is_not_there_is_refused() {
        let (conn, profile_id) = workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "video".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap();

        let nonsense = ["no such brick".to_owned()];
        let error = for_work(
            &conn,
            &work.id,
            "{styles}",
            Context {
                style_brick_ids: &nonsense,
                ..Default::default()
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("no such brick"), "{error}");
    }

    #[test]
    fn a_work_prompt_carries_the_title_and_the_current_body() {
        let (mut conn, profile_id) = workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        version::create(
            &mut conn,
            &work.id,
            NewVersion {
                role: "lyrics".into(),
                body: "the cranes go still".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();

        let prompt = for_work(
            &conn,
            &work.id,
            "About {title}:\n{body}",
            Context::default(),
        )
        .unwrap();

        assert_eq!(prompt, "About Harbour lights:\nthe cranes go still");
    }

    #[test]
    fn a_prompt_can_ask_for_a_role_the_work_is_not_currently_on() {
        let (mut conn, profile_id) = workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Subject".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        version::create(
            &mut conn,
            &work.id,
            NewVersion {
                role: "lyrics".into(),
                body: "the words".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
        version::create(
            &mut conn,
            &work.id,
            NewVersion {
                role: "style".into(),
                body: "slow indie folk".into(),
                label: None,
                meta: None,
                make_current: false,
                parent_version_id: None,
            },
        )
        .unwrap();

        let prompt = for_work(
            &conn,
            &work.id,
            "Text: {role:lyrics}\nStyle: {role:style}",
            Context::default(),
        )
        .unwrap();

        assert_eq!(prompt, "Text: the words\nStyle: slow indie folk");
    }

    #[test]
    fn a_work_without_versions_renders_an_empty_body_rather_than_failing() {
        let (conn, profile_id) = workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Empty".into(),
                ..NewWork::default()
            },
        )
        .unwrap();

        let prompt = for_work(&conn, &work.id, "{title}:{body}", Context::default()).unwrap();

        assert_eq!(prompt, "Empty:");
    }

    #[test]
    fn an_unknown_work_fails() {
        let (conn, _) = workspace();

        assert!(for_work(&conn, "nope", "{title}", Context::default()).is_err());
    }

    #[test]
    fn placeholders_are_found_once_and_prose_braces_are_not() {
        assert_eq!(
            placeholders("{title} and {role:lyrics}, {title} again; a set {1, 2} and {Not one}"),
            vec!["title".to_owned(), "role:lyrics".to_owned()]
        );
        assert!(is_known_placeholder("donor:lyrics"));
        assert!(is_known_placeholder("scenes"));
        assert!(!is_known_placeholder("typo"));
        assert!(!is_known_placeholder("role:"));
    }

    #[test]
    fn produces_reads_scenes_and_what_they_do_to_the_board() {
        let action = |produces: &str| PromptTemplate {
            key: "k".into(),
            label: "K".into(),
            template: String::new(),
            description: None,
            icon: None,
            produces: Some(produces.into()),
            method: None,
            kinds: Vec::new(),
            scope: None,
        };
        assert_eq!(
            action("scenes").produces(),
            Produces::Scenes(BoardChange::Replace)
        );
        assert_eq!(
            action("scenes:add").produces(),
            Produces::Scenes(BoardChange::Add)
        );
        assert_eq!(
            action("scenes:revise").produces(),
            Produces::Scenes(BoardChange::Revise)
        );
        assert_eq!(action("scenes:later").produces(), Produces::Prose);
        assert!(!action("scenes:later").produces_is_known());
        assert!(action("scenes:revise").produces_is_known());
    }

    fn video(conn: &Connection, profile_id: &str, title: &str) -> String {
        work::create(
            conn,
            profile_id,
            NewWork {
                kind: "video".into(),
                title: title.into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn the_board_and_a_scene_render_for_a_video() {
        let (conn, profile_id) = workspace();
        let video_id = video(&conn, &profile_id, "The clip");
        let mut blocks = serde_json::Map::new();
        blocks.insert("still".into(), serde_json::json!("cranes, fog, 35mm"));
        let first = scene::create(
            &conn,
            &profile_id,
            NewScene {
                work_id: video_id.clone(),
                section: Some("intro".into()),
                starts_at: Some(0.0),
                ends_at: Some(4.5),
                shot_type: Some("wide".into()),
                description: Some("the harbour at dawn".into()),
                blocks: Some(blocks),
                ..NewScene::default()
            },
        )
        .unwrap();
        scene::create(
            &conn,
            &profile_id,
            NewScene {
                work_id: video_id.clone(),
                description: Some("a face | turned".into()),
                ..NewScene::default()
            },
        )
        .unwrap();

        let board = for_work(&conn, &video_id, "{scenes}", Context::default()).unwrap();
        assert!(
            board.contains("| 1 | intro | 0:00–0:04.5 | Wide | the harbour at dawn |"),
            "{board}"
        );
        assert!(
            board.contains("| 2 |  |  |  | a face \\| turned |"),
            "{board}"
        );
        assert!(
            !board.contains("35mm"),
            "the table leaves the blocks out: {board}"
        );

        let sheet = for_work(
            &conn,
            &video_id,
            "{scene}",
            Context {
                version_id: None,
                scene_id: Some(&first.id),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            sheet.starts_with("Scene 1 · intro · 0:00–0:04.5 · Wide\n"),
            "{sheet}"
        );
        assert!(sheet.contains("the harbour at dawn"), "{sheet}");
        assert!(
            sheet.contains("Still frame (`still`):\ncranes, fog, 35mm"),
            "{sheet}"
        );

        let bare = video(&conn, &profile_id, "Bare");
        assert_eq!(
            for_work(&conn, &bare, "{scenes}", Context::default()).unwrap(),
            "(the board is empty)"
        );
        let err = for_work(
            &conn,
            &bare,
            "{scene}",
            Context {
                version_id: None,
                scene_id: Some(&first.id),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("not a scene of"), "{err}");
    }

    #[test]
    fn a_donor_is_read_and_its_absence_is_refused() {
        let (mut conn, profile_id) = workspace();
        let song = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        version::create(
            &mut conn,
            &song.id,
            NewVersion {
                role: "lyrics".into(),
                body: "the cranes go still".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
        let video_id = video(&conn, &profile_id, "The clip");

        let refused = for_work(&conn, &video_id, "from {donor}", Context::default()).unwrap_err();
        assert!(
            refused.to_string().contains("not made from anything yet"),
            "{refused}"
        );

        link::create(
            &conn,
            &profile_id,
            NewLink {
                work_id: video_id.clone(),
                source_id: song.id.clone(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();
        let rendered = for_work(
            &conn,
            &video_id,
            "from {donor}:\n{donor:lyrics}|{donor:style}",
            Context::default(),
        )
        .unwrap();
        assert_eq!(
            rendered,
            "from “Harbour lights” (song):\nthe cranes go still|"
        );
    }

    #[test]
    fn a_role_the_kind_names_but_the_work_lacks_is_refused_not_blanked() {
        let (conn, profile_id) = workspace();
        let video_id = video(&conn, &profile_id, "The clip");

        let refused = for_work(&conn, &video_id, "{role:plot}", Context::default()).unwrap_err();
        assert!(refused.to_string().contains("has no Plot yet"), "{refused}");

        // A role the kind does not name stays visible, as any unknown
        // placeholder does: the save-time check is what catches it.
        assert_eq!(
            for_work(&conn, &video_id, "{role:lyrics}", Context::default()).unwrap(),
            "{role:lyrics}"
        );
    }
}

#[cfg(test)]
mod version_tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::work::NewWork;
    use crate::work::version::NewVersion;

    /// An action started on revision 1 reads revision 1 — as `{body}` and as
    /// `{role:lyrics}` — while the work has moved on to revision 2. Another
    /// role still reads its latest, and a version of another work is refused.
    #[test]
    fn a_named_version_is_what_the_template_reads() {
        let mut conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        let make = |conn: &mut Connection, role: &str, body: &str| {
            version::create(
                conn,
                &work.id,
                NewVersion {
                    role: role.into(),
                    body: body.into(),
                    label: None,
                    meta: None,
                    make_current: true,
                    parent_version_id: None,
                },
            )
            .unwrap()
            .id
        };
        let first = make(&mut conn, "lyrics", "first draft");
        make(&mut conn, "lyrics", "second draft");
        make(&mut conn, "style", "warm tape");

        let rendered = for_work(
            &conn,
            &work.id,
            "B:{body}|L:{role:lyrics}|S:{role:style}",
            Context {
                version_id: Some(&first),
                scene_id: None,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(rendered, "B:first draft|L:first draft|S:warm tape");

        let latest = for_work(&conn, &work.id, "L:{role:lyrics}", Context::default()).unwrap();
        assert_eq!(latest, "L:second draft", "without a version, the latest");

        let other = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Other".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        let err = for_work(
            &conn,
            &other.id,
            "{body}",
            Context {
                version_id: Some(&first),
                scene_id: None,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("not a version of"), "{err}");
    }

    #[test]
    fn produces_is_read_as_the_application_understands_it() {
        let action = |produces: Option<&str>| PromptTemplate {
            key: "k".into(),
            label: "K".into(),
            template: String::new(),
            description: None,
            icon: None,
            produces: produces.map(str::to_owned),
            method: None,
            kinds: Vec::new(),
            scope: None,
        };
        assert_eq!(action(None).produces(), Produces::Prose);
        assert_eq!(action(Some("score")).produces(), Produces::Score);
        assert_eq!(
            action(Some("version:critique")).produces(),
            Produces::Version("critique".into())
        );
        assert_eq!(action(Some("version:")).produces(), Produces::Prose);
        assert_eq!(
            action(Some("something-later")).produces(),
            Produces::Prose,
            "an unknown value is prose, not a refusal"
        );
    }
}
