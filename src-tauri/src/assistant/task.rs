//! Profile actions started as tasks rather than as conversation.
//!
//! The panel's own action buttons fill the composer and wait: what is about to
//! be sent — and paid for — is read first. That is right when the panel is
//! already open and the answer is the point. It is wrong for the other way an
//! action is used: from the card, hands on the work, wanting the thing done
//! and not wanting to move.
//!
//! A task is that second way. It renders the profile's template, opens a chat
//! of its own for it, and starts a run — all before returning, so the caller
//! has something to show at once. The chat is always new: dropping a task into
//! whatever conversation happened to be open would bury it in someone else's
//! thread and, worse, hand it that thread's session as context.
//!
//! What a task sends is composed once, by [`compose`], and the preview a card
//! offers is the same call without the chat: the preview and the run cannot
//! part, because there is only one text.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::{Error, Result};
use crate::profile;
use crate::scene;
use crate::work;
use crate::work::version;

use super::prompt::{Context, Produces, PromptTemplate, Scope};

/// A task the caller asked for, before anything was started.
pub struct Prepared {
    /// The chat opened to hold it.
    pub chat_id: String,
    /// The rendered prompt.
    pub prompt: String,
    /// What this task is, for the duplicate check.
    pub key: String,
    /// The chat's name, so the list does not show a task as an untitled chat.
    pub title: String,
    /// Reference files the run may read, by path.
    pub attachments: Vec<PathBuf>,
}

/// What a task would send: the text, the method behind it, its name — read
/// before it is started, or started as it is.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Composed {
    /// The message, exactly as the run receives it.
    pub prompt: String,
    /// The action's method, exactly as the run is briefed with it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// What this task is, for the duplicate check.
    pub key: String,
    /// The chat's name.
    pub title: String,
    #[serde(skip)]
    pub attachments: Vec<PathBuf>,
}

/// The `produces` value naming a scoring action.
pub const SCORE: &str = "score";

/// What a task is, as a key: this action, on this work.
///
/// Two clicks on the same button produce the same key; the same action on
/// another work does not. That is the whole rule — the key says what is being
/// done, never which run is doing it.
pub fn key(action: &str, work_id: &str) -> String {
    format!("{action}:{work_id}")
}

/// The key of a scene action: this action, on this work, on this scene.
/// The prompts of scene 2 can be written while scene 1's are still going.
pub fn scene_key(action: &str, work_id: &str, scene_id: &str) -> String {
    format!("{action}:{work_id}:{scene_id}")
}

/// The key of a task about one prompt block of one scene.
///
/// A fourth segment, so two blocks of the same scene run side by side and the
/// same block twice does not: the registry refuses by string equality, and
/// what a person means by "already running" is this block, not this scene.
pub fn block_key(action: &str, work_id: &str, scene_id: &str, block: &str) -> String {
    format!("{action}:{work_id}:{scene_id}:{block}")
}

/// The prompt blocks a kind names, for a refusal that says what was offered.
fn named_blocks(blocks: &[crate::profile::config::SceneBlock]) -> String {
    if blocks.is_empty() {
        return "none".to_owned();
    }
    blocks
        .iter()
        .map(|block| format!("`{}`", block.key))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The block a task key names, if it names one.
pub fn block_of_key(key: &str) -> Option<&str> {
    key.splitn(4, ':').nth(3)
}

/// The scene a task key names, when it names one.
///
/// Reads the third segment only: a key that also names a block has four, and
/// the scene is still the third. Splitting into three would hand back
/// `scene:block` as the scene id and find no scene at all.
pub fn scene_of_key(key: &str) -> Option<&str> {
    key.split(':').nth(2)
}

/// What a task is about, beyond the work.
#[derive(Debug, Clone, Copy, Default)]
pub struct About<'a> {
    /// The version the action is about — the one open on the versions tab.
    /// It is what the template reads and what the answer's proposal will
    /// bind to; without it the action is about the work as it stands.
    pub version_id: Option<&'a str>,
    /// The scene a scene action is about.
    pub scene_id: Option<&'a str>,
    /// One prompt block of that scene, when the action is about a single
    /// block rather than the whole scene: regenerating the animation without
    /// touching the still. A key of the kind's `scene_blocks`.
    pub block: Option<&'a str>,
    /// Reference files the run may read: images, documents, anything the
    /// method should look at. Listed in the prompt by path, and the run is
    /// given leave to read their folders.
    pub attachments: &'a [String],
    /// The style bricks the person picked for this run, in order, read by a
    /// template's `{styles}`. Picked at the moment the action is started, not
    /// kept on the work: which parts a picture is built from is the question
    /// being asked (ADR 0031).
    pub style_brick_ids: &'a [String],
}

/// Compose `action` of the active profile against `work_id`: the prompt as
/// it will be sent, the method as it will be briefed.
///
/// Fails when the profile has no such action, when the action is not for
/// the work's kind, when a scene action has no scene, when a file does not
/// exist, or when the template reads a donor the work does not have: a
/// card offering a button the profile dropped is a card that has to be told,
/// not one that should quietly send an empty prompt.
pub fn compose(
    conn: &Connection,
    work_id: &str,
    action: &str,
    about: About<'_>,
) -> Result<Composed> {
    let profile =
        profile::active(conn)?.ok_or_else(|| Error::Other("no profile is active".into()))?;

    let template = profile
        .config
        .prompts
        .iter()
        .find(|prompt| prompt.key == action)
        .ok_or_else(|| Error::not_found("prompt", action))?;

    let work = work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    if !template.applies_to(&work.kind) {
        return Err(Error::Other(format!(
            "“{}” is not an action for a {}",
            template.label, work.kind
        )));
    }
    // Checked here rather than left to the render: a version of another work
    // must be refused before a chat is opened for it.
    if let Some(id) = about.version_id {
        let found = version::get(conn, id)?.ok_or_else(|| Error::not_found("version", id))?;
        if found.work_id != work.id {
            return Err(Error::Other(format!(
                "version `{id}` is not a version of “{}”",
                work.title
            )));
        }
    }
    let scene = match (template.scope(), about.scene_id) {
        (Scope::Scene, None) => {
            return Err(Error::Other(format!(
                "“{}” is about a scene: start it from one",
                template.label
            )));
        }
        (Scope::Scene, Some(id)) => {
            let found = scene::get(conn, id)?.ok_or_else(|| Error::not_found("scene", id))?;
            if found.work_id != work.id {
                return Err(Error::Other(format!(
                    "scene `{id}` is not a scene of “{}”",
                    work.title
                )));
            }
            Some(found)
        }
        // A work action started with a scene in hand is about the work: the
        // scene is not read, and the key does not name it.
        (Scope::Work, _) => None,
        // A style action is not about a work at all — it is composed by
        // `compose_for_style` against a brick of the dictionary. Reaching
        // here means a caller aimed one at a card, which is a mistake worth
        // naming rather than a prompt worth sending.
        (Scope::Style, _) => {
            return Err(Error::Other(format!(
                "“{}” is about a style, not a work: start it from the dictionary",
                template.label
            )));
        }
    };

    // One block of that scene, when the action is aimed at one. Refused
    // before a chat is opened: a block the kind does not name would be a
    // task whose answer nothing could apply, and a block without a scene is
    // a block of nothing.
    let vocabulary_for_block = profile.config.vocabulary(&work.kind);
    let block = match (&scene, about.block) {
        (_, None) => None,
        (None, Some(_)) => {
            return Err(Error::Other(format!(
                "“{}” is not about a scene, so it is not about a block of one",
                template.label
            )));
        }
        (Some(_), Some(key)) => {
            if !vocabulary_for_block
                .scene_blocks
                .iter()
                .any(|block| block.key == key)
            {
                return Err(Error::Other(format!(
                    "`{key}` is not a prompt block of a {}; the profile names {}",
                    vocabulary_for_block.label.as_str().to_lowercase(),
                    named_blocks(&vocabulary_for_block.scene_blocks)
                )));
            }
            Some(key)
        }
    };

    let mut prompt = super::prompt::for_work(
        conn,
        work_id,
        &template.template,
        Context {
            version_id: about.version_id,
            scene_id: scene.as_ref().map(|s| s.id.as_str()),
            style_brick_ids: about.style_brick_ids,
        },
    )?;

    // An action that asks for something the application can act on says the
    // shape it needs. Ordinary actions say nothing and get prose.
    let vocabulary = profile.config.vocabulary(&work.kind);
    match template.produces() {
        Produces::Score => prompt.push_str(&super::proposal::scoring_instruction(
            &profile.config,
            &work.kind,
        )),
        Produces::Version(role) => {
            let label = vocabulary
                .version_roles
                .iter()
                .find(|r| r.key == role)
                .map(|r| r.label.as_str().to_owned())
                .unwrap_or(role);
            prompt.push_str(&super::proposal::version_instruction(&label));
        }
        Produces::Scenes(change) => {
            if !scene::kind_has_scenes(&profile.config, &work.kind) {
                return Err(Error::Other(format!(
                    "a {} has no storyboard for “{}” to propose scenes on",
                    work.kind, template.label
                )));
            }
            prompt.push_str(&super::proposal::scenes_instruction(
                vocabulary,
                change,
                scene.as_ref().map(|s| s.position),
                block,
            ));
        }
        Produces::Prose => {}
    }

    // Reference files: named in the prompt so the run knows they are there
    // and the chat shows what went. A path that is not a file is refused
    // now, not discovered by a run that reports "no such file" ten minutes
    // later.
    let attachments = attachments_of(about.attachments)?;
    if !attachments.is_empty() {
        prompt.push_str("\n\nReference files — read each of them before answering:\n");
        for path in &attachments {
            prompt.push_str(&format!("- {}\n", path.display()));
        }
        let trimmed = prompt.trim_end().len();
        prompt.truncate(trimmed);
    }

    // The instruction that lets the assistant mark its own question. Only
    // tasks carry it: a prompt typed in the panel is read before it is sent,
    // and appending words the person did not write would break that.
    let prompt = super::waiting::instruct(&prompt);

    // Named on creation rather than left to borrow its first question: a
    // rendered template can open with pages of the work's own text, and a
    // chat list full of lyrics tells nobody which task produced what.
    let title = match &scene {
        Some(scene) => format!("{} · {} · #{}", template.label, work.title, scene.position),
        None => format!("{} · {}", template.label, work.title),
    };
    let key = match (&scene, block) {
        (Some(scene), Some(block)) => block_key(action, work_id, &scene.id, block),
        (Some(scene), None) => scene_key(action, work_id, &scene.id),
        (None, _) => key(action, work_id),
    };

    Ok(Composed {
        prompt,
        method: template.method().map(str::to_owned),
        key,
        title,
        attachments,
    })
}

/// The files as paths, each checked to exist. Repeated paths are listed
/// once.
fn attachments_of(given: &[String]) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for raw in given {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let path = Path::new(trimmed);
        if !path.is_file() {
            return Err(Error::Other(format!("no file at {trimmed}")));
        }
        if !paths.iter().any(|p| p == path) {
            paths.push(path.to_path_buf());
        }
    }
    Ok(paths)
}

/// Render `action` of the active profile against `work_id` and open a chat for
/// it — [`compose`] with the chat.
pub fn prepare(
    conn: &Connection,
    work_id: &str,
    action: &str,
    about: About<'_>,
) -> Result<Prepared> {
    let composed = compose(conn, work_id, action, about)?;
    let profile =
        profile::active(conn)?.ok_or_else(|| Error::Other("no profile is active".into()))?;

    let chat = super::create(
        conn,
        &profile.id,
        super::NewChat {
            work_id: Some(work_id.to_owned()),
            title: Some(composed.title.clone()),
            action: Some(action.to_owned()),
            version_id: about.version_id.map(str::to_owned),
        },
    )?;

    Ok(Prepared {
        chat_id: chat.id,
        prompt: composed.prompt,
        key: composed.key,
        title: composed.title,
        attachments: composed.attachments,
    })
}

/// What a task about a style brick is, as a key: this action, on this brick.
pub fn style_key(action: &str, brick_id: &str) -> String {
    format!("{action}:style:{brick_id}")
}

/// Compose `action` against a style brick rather than a work.
///
/// A brick is not a work and never will be — it belongs to the workspace, and
/// there is no title, no body and no board to read. So this is its own
/// composer rather than `compose` with the work made optional: the two have
/// almost nothing in common but the action, and threading an absent work
/// through a hundred lines of `for_work` would make both harder to read for
/// the sake of sharing the four that overlap.
///
/// The prompt carries the type's own `hint` — what to describe for a brick of
/// this type — and the author's steer, kept separate and labelled as such.
/// Those two are the whole reason the answer is worth having: without them the
/// model is looking at pictures and guessing which question it is answering.
pub fn compose_for_style(
    conn: &Connection,
    brick_id: &str,
    action: &str,
) -> Result<(Composed, String)> {
    let profile =
        profile::active(conn)?.ok_or_else(|| Error::Other("no profile is active".into()))?;
    let template = profile
        .config
        .prompts
        .iter()
        .find(|prompt| prompt.key == action)
        .ok_or_else(|| Error::Other(format!("the profile has no action `{action}`")))?;

    let brick = crate::style_brick::get(conn, brick_id)?
        .ok_or_else(|| Error::not_found("style", brick_id))?;
    let kind = profile.config.style_type(&brick.type_key);

    let mut prompt = String::new();
    prompt.push_str(&format!(
        "The style brick is “{}”, of the type {}.\n\n",
        brick.name,
        kind.map_or(brick.type_key.clone(), |k| k.label.as_str().to_owned())
    ));
    if let Some(hint) = kind.and_then(|k| k.hint.as_ref()) {
        prompt.push_str(&format!(
            "What to describe for this type:\n{}\n\n",
            hint.as_str()
        ));
    }
    if let Some(steer) = brick
        .hint
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        prompt.push_str(&format!("The author's steer:\n{steer}\n\n"));
    }
    if let Some(existing) = brick
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        prompt.push_str(&format!(
            "What it says today, which you are rewriting:\n{existing}\n\n"
        ));
    }
    prompt.push_str(&template.template);

    // The references travel as the attachments an ordinary task's files do:
    // one way for a run to be given something to look at.
    let references = crate::asset::for_style_brick(conn, brick_id)?;
    let attachments: Vec<PathBuf> = references
        .iter()
        .map(|asset| PathBuf::from(&asset.path))
        .filter(|path| path.is_file())
        .collect();
    // Said by the composer rather than by the template, because only the
    // composer knows whether there are any. A template that claims pictures
    // are attached when none are sends the model looking for them, and what
    // comes back is a description of nothing.
    if attachments.is_empty() {
        prompt.push_str(
            "\n\nThere are no reference pictures. Write from the name and what is said above, and say in one line that you had nothing to look at.",
        );
    } else {
        prompt.push_str("\n\nThe reference pictures:\n");
        for path in &attachments {
            prompt.push_str(&format!("{}\n", path.display()));
        }
    }

    let prompt = super::waiting::instruct(&prompt);

    Ok((
        Composed {
            prompt,
            method: template.method().map(str::to_owned),
            key: style_key(action, brick_id),
            title: format!("{} · {}", template.label.as_str(), brick.name),
            attachments,
        },
        profile.id,
    ))
}

/// [`compose_for_style`] with the chat it will be answered in.
pub fn prepare_for_style(conn: &Connection, brick_id: &str, action: &str) -> Result<Prepared> {
    let (composed, profile_id) = compose_for_style(conn, brick_id, action)?;

    // A chat on no work: a brick belongs to the workspace, so the answer does
    // not hang on anybody's card.
    let chat = super::create(
        conn,
        &profile_id,
        super::NewChat {
            work_id: None,
            title: Some(composed.title.clone()),
            action: Some(action.to_owned()),
            version_id: None,
        },
    )?;

    Ok(Prepared {
        chat_id: chat.id,
        prompt: composed.prompt,
        key: composed.key,
        title: composed.title,
        attachments: composed.attachments,
    })
}

/// The action of the active profile a task key names, when the profile
/// still has it.
pub fn action_of_key(conn: &Connection, task_key: &str) -> Option<PromptTemplate> {
    let action = task_key.split(':').next()?;
    let profile = profile::active(conn).ok()??;
    profile
        .config
        .prompts
        .iter()
        .find(|prompt| prompt.key == action)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::link::{self, NewLink};
    use crate::scene::NewScene;
    use crate::work::NewWork;
    use crate::work::version::{self, NewVersion};

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    /// The type's own instruction reaches the model, the author's steer is
    /// kept apart and labelled, and neither is left to be guessed at.
    #[test]
    fn describing_a_brick_carries_the_types_question_and_the_authors_steer() {
        let (conn, profile_id) = workspace();
        let brick = crate::style_brick::create(
            &conn,
            &profile_id,
            crate::style_brick::NewStyleBrick {
                type_key: "image-style".into(),
                name: "Cold north".into(),
                description: None,
                hint: Some("only the ground floor".into()),
            },
        )
        .unwrap();

        let (composed, _) = compose_for_style(&conn, &brick.id, "describe-style").unwrap();

        assert!(
            composed.prompt.contains("Cold north"),
            "{}",
            composed.prompt
        );
        assert!(
            composed.prompt.contains("Image style"),
            "the type is named: {}",
            composed.prompt
        );
        assert!(
            composed.prompt.contains("Not what is in the picture"),
            "the type's own hint is what makes the answer worth having: {}",
            composed.prompt
        );
        assert!(
            composed.prompt.contains("only the ground floor"),
            "the author's steer reaches the model when describing: {}",
            composed.prompt
        );
        assert!(
            composed.method.is_some(),
            "the action ships a method and it is briefed"
        );
    }

    /// A prompt that claims pictures are attached when none are sends the
    /// model looking for them, and what comes back is a description of
    /// nothing. The composer knows, so the composer says.
    #[test]
    fn describing_a_brick_with_no_references_says_so_rather_than_claiming_some() {
        let (conn, profile_id) = workspace();
        let brick = crate::style_brick::create(
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

        let (composed, _) = compose_for_style(&conn, &brick.id, "describe-style").unwrap();

        assert!(
            composed.prompt.contains("no reference pictures"),
            "{}",
            composed.prompt
        );
        assert!(
            !composed.prompt.contains("The reference pictures:"),
            "nothing is listed that is not there: {}",
            composed.prompt
        );
        assert!(composed.attachments.is_empty());
    }

    /// A style action is about a brick of the dictionary. Aimed at a card it
    /// is refused by name rather than sending a prompt about the wrong thing
    /// — and `actionsFor` in the window keeps it off the bar for the same
    /// reason, so this is the second lock on one door.
    #[test]
    fn a_style_action_aimed_at_a_work_is_refused() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");

        let refused = compose(&conn, &work_id, "describe-style", About::default())
            .unwrap_err()
            .to_string();

        assert!(
            refused.contains("about a style"),
            "the refusal says what it is about: {refused}"
        );
        assert!(
            refused.contains("dictionary"),
            "and where to start it: {refused}"
        );
    }

    fn work_with_body(conn: &mut Connection, profile_id: &str, title: &str, body: &str) -> String {
        let work = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: title.into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        version::create(
            conn,
            &work.id,
            NewVersion {
                role: "lyrics".into(),
                body: body.into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
        work.id
    }

    /// The first action the seeded profile offers, whatever it is called.
    fn some_action(conn: &Connection) -> crate::assistant::prompt::PromptTemplate {
        profile::active(conn)
            .unwrap()
            .unwrap()
            .config
            .prompts
            .first()
            .cloned()
            .expect("the seeded profile has actions")
    }

    #[test]
    fn a_task_gets_a_chat_of_its_own_tied_to_the_work() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");
        let action = some_action(&conn);

        let prepared = prepare(&conn, &work_id, &action.key, About::default()).unwrap();

        let chat = super::super::get(&conn, &prepared.chat_id)
            .unwrap()
            .unwrap();
        assert_eq!(chat.work_id.as_deref(), Some(work_id.as_str()));
        assert!(chat.session_id.is_none(), "a fresh chat carries no session");
    }

    #[test]
    fn every_task_opens_a_new_chat_rather_than_reusing_one() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");
        let action = some_action(&conn);

        let first = prepare(&conn, &work_id, &action.key, About::default()).unwrap();
        let second = prepare(&conn, &work_id, &action.key, About::default()).unwrap();

        assert_ne!(
            first.chat_id, second.chat_id,
            "a task must never land in a conversation already going"
        );
    }

    #[test]
    fn the_chat_is_named_after_the_action_and_the_work() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");
        let action = some_action(&conn);

        let prepared = prepare(&conn, &work_id, &action.key, About::default()).unwrap();

        let chat = super::super::get(&conn, &prepared.chat_id)
            .unwrap()
            .unwrap();
        let title = chat.title.expect("a task chat is named on creation");
        assert!(title.contains(action.label.as_str()), "{title}");
        assert!(title.contains("Harbour lights"), "{title}");
    }

    #[test]
    fn the_prompt_is_rendered_against_the_work() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(
            &mut conn,
            &profile_id,
            "Harbour lights",
            "the cranes go still",
        );
        let action = some_action(&conn);

        let prepared = prepare(&conn, &work_id, &action.key, About::default()).unwrap();

        assert!(
            !prepared.prompt.contains('{'),
            "an unrendered placeholder means the template never saw the work: {}",
            prepared.prompt
        );
        assert!(
            prepared.prompt.contains("the cranes go still"),
            "{}",
            prepared.prompt
        );
    }

    #[test]
    fn a_task_prompt_carries_the_marker_instruction() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");
        let action = some_action(&conn);

        let prepared = prepare(&conn, &work_id, &action.key, About::default()).unwrap();

        assert!(
            prepared.prompt.contains(crate::assistant::waiting::MARKER),
            "a task must be able to say it stopped to ask: {}",
            prepared.prompt
        );
        assert!(
            prepared.prompt.starts_with("Here are the lyrics"),
            "the instruction goes after the action, never in front of it"
        );
    }

    #[test]
    fn an_action_the_profile_does_not_have_fails() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");

        let refused = prepare(&conn, &work_id, "no-such-action", About::default());

        assert!(refused.is_err());
    }

    #[test]
    fn a_failed_task_leaves_no_chat_behind() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");

        let _ = prepare(&conn, &work_id, "no-such-action", About::default());

        let chats: i64 = conn
            .query_row("SELECT count(*) FROM chat", [], |row| row.get(0))
            .unwrap();
        assert_eq!(chats, 0, "the chat is opened only once there is a prompt");
        let _ = work_id;
    }

    #[test]
    fn an_unknown_work_fails() {
        let (conn, _) = workspace();
        let action = some_action(&conn);

        assert!(prepare(&conn, "nope", &action.key, About::default()).is_err());
    }

    #[test]
    fn the_key_names_the_action_and_the_work_not_the_run() {
        assert_eq!(key("critique", "w1"), key("critique", "w1"));
        assert_ne!(key("critique", "w1"), key("critique", "w2"));
        assert_ne!(key("critique", "w1"), key("score", "w1"));
    }

    fn video(conn: &Connection, profile_id: &str) -> String {
        work::create(
            conn,
            profile_id,
            NewWork {
                kind: "video".into(),
                title: "The clip".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id
    }

    fn context(conn: &mut Connection, video_id: &str) {
        version::create(
            conn,
            video_id,
            NewVersion {
                role: "context".into(),
                body: "hero: a woman in a red coat".into(),
                label: None,
                meta: None,
                make_current: false,
                parent_version_id: None,
            },
        )
        .unwrap();
    }

    #[test]
    fn the_preview_is_what_the_task_sends() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");

        let composed = compose(&conn, &work_id, "critique", About::default()).unwrap();
        let prepared = prepare(&conn, &work_id, "critique", About::default()).unwrap();

        assert_eq!(prepared.prompt, composed.prompt);
        assert_eq!(prepared.key, composed.key);
        assert_eq!(prepared.title, composed.title);
        assert!(
            composed.method.is_some(),
            "the critique ships with a method"
        );
        assert!(composed.prompt.contains("the cranes"));
    }

    #[test]
    fn an_action_not_for_the_kind_is_refused() {
        let (conn, profile_id) = workspace();
        let video_id = video(&conn, &profile_id);

        let refused = compose(&conn, &video_id, "critique", About::default()).unwrap_err();

        assert!(
            refused.to_string().contains("not an action for a video"),
            "{refused}"
        );
    }

    /// A task about one prompt block carries the block in its key, so two
    /// blocks of the same scene go side by side and the same block twice does
    /// not — the registry refuses by string equality, and what a person means
    /// by "already running" is this block.
    ///
    /// A block the kind does not name is refused before a chat is opened, and
    /// so is a block on an action that is not about a scene at all.
    #[test]
    fn a_task_about_one_block_names_it_in_the_key_and_refuses_a_stranger() {
        let (mut conn, profile_id) = workspace();
        let video_id = video(&conn, &profile_id);
        context(&mut conn, &video_id);
        let scene = crate::scene::create(
            &conn,
            &profile_id,
            NewScene {
                work_id: video_id.clone(),
                description: Some("she turns".into()),
                ..NewScene::default()
            },
        )
        .unwrap();

        let about = |block: Option<&'static str>| About {
            scene_id: Some(&scene.id),
            block,
            ..About::default()
        };

        let still = compose(&conn, &video_id, "prompts", about(Some("still"))).unwrap();
        let motion = compose(&conn, &video_id, "prompts", about(Some("motion"))).unwrap();
        assert_eq!(
            still.key,
            format!("prompts:{video_id}:{}:still", scene.id),
            "the block is the fourth segment"
        );
        assert_ne!(
            still.key, motion.key,
            "two blocks of one scene are two tasks"
        );
        assert_eq!(
            block_of_key(&still.key),
            Some("still"),
            "and the key reads back"
        );
        assert_eq!(block_of_key(&motion.key), Some("motion"));

        // The scene is still found in a key that also names a block.
        assert_eq!(scene_of_key(&still.key), Some(scene.id.as_str()));
        assert_eq!(
            block_of_key(&format!("prompts:{video_id}:{}", scene.id)),
            None
        );

        // The prompt asks for that block and nothing else.
        assert!(
            still.prompt.contains("`still` block only"),
            "{}",
            still.prompt
        );

        // A block the kind does not name, refused by name.
        let refused = compose(&conn, &video_id, "prompts", about(Some("grade")))
            .unwrap_err()
            .to_string();
        assert!(refused.contains("`grade`"), "{refused}");
        assert!(
            refused.contains("`still`"),
            "names what it does have: {refused}"
        );

        // A block on an action that is not about a scene.
        let refused = compose(
            &conn,
            &video_id,
            "plot",
            About {
                block: Some("still"),
                ..About::default()
            },
        )
        .unwrap_err()
        .to_string();
        assert!(refused.contains("not about a scene"), "{refused}");
    }

    #[test]
    fn a_scene_action_needs_its_scene_and_names_it_in_the_key() {
        let (mut conn, profile_id) = workspace();
        let video_id = video(&conn, &profile_id);
        context(&mut conn, &video_id);
        let scene = crate::scene::create(
            &conn,
            &profile_id,
            NewScene {
                work_id: video_id.clone(),
                description: Some("she turns".into()),
                ..NewScene::default()
            },
        )
        .unwrap();

        let refused = compose(&conn, &video_id, "prompts", About::default()).unwrap_err();
        assert!(
            refused.to_string().contains("start it from one"),
            "{refused}"
        );

        let composed = compose(
            &conn,
            &video_id,
            "prompts",
            About {
                scene_id: Some(&scene.id),
                ..About::default()
            },
        )
        .unwrap();
        assert_eq!(composed.key, format!("prompts:{video_id}:{}", scene.id));
        assert_eq!(scene_of_key(&composed.key), Some(scene.id.as_str()));
        assert_eq!(scene_of_key("critique:w1"), None);
        assert!(composed.title.ends_with("· #1"), "{}", composed.title);
        assert!(
            composed.prompt.contains("Scene 1\n\nshe turns"),
            "{}",
            composed.prompt
        );
        assert!(
            composed.prompt.contains("The scene is number 1."),
            "{}",
            composed.prompt
        );
        assert!(
            composed.prompt.contains("`still`"),
            "the instruction names the kind's blocks: {}",
            composed.prompt
        );

        // Started as a task, the chat is tied to the work and the key
        // refuses a second click on this scene, not on the board.
        let prepared = prepare(
            &conn,
            &video_id,
            "prompts",
            About {
                scene_id: Some(&scene.id),
                ..About::default()
            },
        )
        .unwrap();
        assert_eq!(prepared.key, composed.key);
    }

    #[test]
    fn a_storyboard_action_needs_the_plot_first() {
        let (conn, profile_id) = workspace();
        let video_id = video(&conn, &profile_id);

        let refused = compose(&conn, &video_id, "storyboard", About::default()).unwrap_err();

        assert!(refused.to_string().contains("has no Plot yet"), "{refused}");
    }

    #[test]
    fn the_plot_action_reads_the_donor_and_refuses_without_one() {
        let (mut conn, profile_id) = workspace();
        let song_id = work_with_body(
            &mut conn,
            &profile_id,
            "Harbour lights",
            "the cranes go still",
        );
        let video_id = video(&conn, &profile_id);

        let refused = compose(&conn, &video_id, "plot", About::default()).unwrap_err();
        assert!(refused.to_string().contains("Links tab"), "{refused}");

        link::create(
            &conn,
            &profile_id,
            NewLink {
                work_id: video_id.clone(),
                source_id: song_id,
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();
        let composed = compose(&conn, &video_id, "plot", About::default()).unwrap();
        assert!(
            composed
                .prompt
                .contains("made from “Harbour lights” (song)"),
            "{}",
            composed.prompt
        );
        assert!(composed.prompt.contains("the cranes go still"));
        assert!(
            composed.prompt.contains("kept as the Plot"),
            "a version instruction names the role: {}",
            composed.prompt
        );
    }

    #[test]
    fn reference_files_are_listed_and_a_missing_one_is_refused() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");
        let dir = std::env::temp_dir().join(format!("kilna-refs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("hero.png");
        std::fs::write(&file, b"png").unwrap();
        let given = vec![
            file.display().to_string(),
            file.display().to_string(),
            "  ".into(),
        ];

        let composed = compose(
            &conn,
            &work_id,
            "critique",
            About {
                attachments: &given,
                ..About::default()
            },
        )
        .unwrap();
        assert_eq!(
            composed.attachments,
            vec![file.clone()],
            "once, and blanks dropped"
        );
        assert!(
            composed.prompt.contains(&format!(
                "Reference files — read each of them before answering:\n- {}",
                file.display()
            )),
            "{}",
            composed.prompt
        );
        assert_eq!(
            super::super::stream::folders_of(&composed.attachments),
            vec![dir.clone()]
        );

        let missing = vec![dir.join("nope.png").display().to_string()];
        let refused = compose(
            &conn,
            &work_id,
            "critique",
            About {
                attachments: &missing,
                ..About::default()
            },
        )
        .unwrap_err();
        assert!(refused.to_string().contains("no file at"), "{refused}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod version_tests {
    use super::*;
    use crate::db;
    use crate::work::NewWork;
    use crate::work::version::{self, NewVersion};

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    /// A task started on a version reads that version, remembers it on the
    /// chat with the action, and a task started on the work as a whole
    /// remembers the action alone.
    #[test]
    fn a_task_on_a_version_reads_it_and_the_chat_remembers_it() {
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
        let mut make = |body: &str| {
            version::create(
                &mut conn,
                &work.id,
                NewVersion {
                    role: "lyrics".into(),
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
        let first = make("the cranes go still");
        make("the cranes go on");

        let prepared = prepare(
            &conn,
            &work.id,
            "critique",
            About {
                version_id: Some(&first),
                ..About::default()
            },
        )
        .unwrap();
        assert!(
            prepared.prompt.contains("the cranes go still")
                && !prepared.prompt.contains("the cranes go on"),
            "{}",
            prepared.prompt
        );
        let chat = super::super::get(&conn, &prepared.chat_id)
            .unwrap()
            .unwrap();
        assert_eq!(chat.action.as_deref(), Some("critique"));
        assert_eq!(chat.version_id.as_deref(), Some(first.as_str()));

        let whole = prepare(&conn, &work.id, "critique", About::default()).unwrap();
        let chat = super::super::get(&conn, &whole.chat_id).unwrap().unwrap();
        assert_eq!(chat.action.as_deref(), Some("critique"));
        assert!(chat.version_id.is_none());
        assert!(whole.prompt.contains("the cranes go on"), "the latest");
    }

    /// The shipped critique produces a version in the `critique` role, and
    /// the prompt says so: the whole answer is what is kept.
    #[test]
    fn an_action_that_produces_a_version_says_so_in_the_prompt() {
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
                body: "x".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
        let prepared = prepare(&conn, &work.id, "critique", About::default()).unwrap();
        assert!(
            prepared.prompt.contains("kept as the Critique"),
            "{}",
            prepared.prompt
        );
        let scored = prepare(&conn, &work.id, "score", About::default()).unwrap();
        assert!(
            scored
                .prompt
                .contains("exactly as the profile defines them"),
            "{}",
            scored.prompt
        );
    }
}
