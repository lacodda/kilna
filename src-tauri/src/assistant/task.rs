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
#[derive(Debug, Clone, PartialEq, serde::Serialize, ts_rs::TS)]
pub struct Composed {
    /// The message, exactly as the run receives it.
    pub prompt: String,
    /// The action's method, exactly as the run is briefed with it.
    #[ts(optional)]
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
    /// The lines a person selected in the text, for an action about a
    /// selection (`{selection}`).
    pub selection: Option<&'a str>,
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
    let profile = profile::active(conn)?.ok_or_else(|| Error::refused("profile.noneActive"))?;

    let template = profile
        .config
        .prompts
        .iter()
        .find(|prompt| prompt.key == action)
        .ok_or_else(|| Error::not_found("prompt", action))?;

    let work = work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    if !template.applies_to(&work.kind) {
        return Err(Error::refused("task.wrongKindForAction")
            .param("action", template.label.as_str())
            .param("kind", work.kind.clone()));
    }
    // Checked here rather than left to the render: a version of another work
    // must be refused before a chat is opened for it.
    if let Some(id) = about.version_id {
        let found = version::get(conn, id)?.ok_or_else(|| Error::not_found("version", id))?;
        if found.work_id != work.id {
            return Err(Error::refused("link.versionNotOfSource")
                .param("version", id)
                .param("title", work.title.clone()));
        }
    }
    // An action about a selection is nothing without one: its prompt would
    // read an empty `{selection}`.
    if template.scope() == Scope::Selection
        && about.selection.map(str::trim).is_none_or(str::is_empty)
    {
        return Err(Error::refused("task.needsSelection").param("action", template.label.as_str()));
    }
    let scene = match (template.scope(), about.scene_id) {
        (Scope::Scene, None) => {
            return Err(Error::refused("task.needsScene").param("action", template.label.as_str()));
        }
        (Scope::Scene, Some(id)) => {
            let found = scene::get(conn, id)?.ok_or_else(|| Error::not_found("scene", id))?;
            if found.work_id != work.id {
                return Err(Error::refused("prompt.sceneNotOfWork")
                    .param("scene", id)
                    .param("title", work.title.clone()));
            }
            Some(found)
        }
        // A work action started with a scene in hand is about the work: the
        // scene is not read, and the key does not name it. So is an action
        // about selected lines: the lines are the work's.
        (Scope::Work | Scope::Selection, _) => None,
        // Composed by `compose_for_card`, against a card of the canon.
        (Scope::Canon, _) => {
            return Err(
                Error::refused("task.isCanonAction").param("action", template.label.as_str())
            );
        }
        // A style action is not about a work at all — it is composed by
        // `compose_for_style` against a brick of the dictionary. Reaching
        // here means a caller aimed one at a card, which is a mistake worth
        // naming rather than a prompt worth sending.
        (Scope::Style, _) => {
            return Err(
                Error::refused("task.isStyleAction").param("action", template.label.as_str())
            );
        }
        // Composed by `compose_for_comment` or `compose_for_screenshot`.
        (Scope::Comment, _) => {
            return Err(
                Error::refused("task.isCommentAction").param("action", template.label.as_str())
            );
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
            return Err(
                Error::refused("task.needsSceneForBlock").param("action", template.label.as_str())
            );
        }
        (Some(_), Some(key)) => {
            if !vocabulary_for_block
                .scene_blocks
                .iter()
                .any(|block| block.key == key)
            {
                return Err(Error::refused("task.unknownActionBlock")
                    .param("block", key)
                    .param(
                        "kind",
                        serde_json::to_value(&vocabulary_for_block.label).unwrap_or_default(),
                    )
                    .param("known", named_blocks(&vocabulary_for_block.scene_blocks)));
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
            selection: about.selection,
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
                return Err(Error::refused("task.actionNoStoryboard")
                    .param("kind", work.kind.clone())
                    .param("action", template.label.as_str()));
            }
            prompt.push_str(&super::proposal::scenes_instruction(
                vocabulary,
                change,
                scene.as_ref().map(|s| s.position),
                block,
            ));
        }
        Produces::Prose => {}
        // Only an action about a comment produces these, and it was refused
        // above; a profile that pairs them with a work does not validate.
        Produces::Comment | Produces::Reply => {
            return Err(Error::refused("task.actionAnswersComment")
                .param("action", template.label.as_str()));
        }
        // The same for an action about a style brick.
        Produces::Description => {
            return Err(
                Error::refused("task.actionAnswersStyle").param("action", template.label.as_str())
            );
        }
        Produces::Canon => prompt.push_str(&crate::canon::proposal::instruction(&profile.config)),
        // Only an action about a card describes one.
        Produces::CardPrompt => {
            return Err(
                Error::refused("task.actionAnswersCard").param("action", template.label.as_str())
            );
        }
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
            return Err(Error::refused("task.fileMissing").param("path", trimmed));
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
    let profile = profile::active(conn)?.ok_or_else(|| Error::refused("profile.noneActive"))?;

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

/// The brick a style task's key names - the other half of [`style_key`].
pub fn style_of_key(key: &str) -> Option<&str> {
    let mut parts = key.splitn(3, ':');
    parts.next()?;
    (parts.next()? == "style").then(|| parts.next()).flatten()
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
    let profile = profile::active(conn)?.ok_or_else(|| Error::refused("profile.noneActive"))?;
    let template = profile
        .config
        .prompts
        .iter()
        .find(|prompt| prompt.key == action)
        .ok_or_else(|| Error::not_found("prompt", action))?;

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

/// What a task about a card of the canon is, as a key: this action, on this
/// card.
pub fn card_key(action: &str, note_id: &str) -> String {
    format!("{action}:card:{note_id}")
}

/// The card a task's key names - the other half of [`card_key`].
pub fn card_of_key(key: &str) -> Option<&str> {
    let mut parts = key.splitn(3, ':');
    parts.next()?;
    (parts.next()? == "card").then(|| parts.next()).flatten()
}

/// Compose `action` against a card of the canon rather than a work (ADR 0043).
///
/// Two things are asked of a card: its facts, gathered out of its own free
/// note, and its description for a picture generator, written from the facts
/// of the sections its kind says it is described from. Both read the card as
/// the work's lens reads it - every fact, so a gathered one is not proposed
/// twice - and the description reads only the settled public facts it will
/// answer to, with the card's pictures as references.
pub fn compose_for_card(
    conn: &Connection,
    note_id: &str,
    action: &str,
) -> Result<(Composed, String)> {
    let profile = profile::active(conn)?.ok_or_else(|| Error::refused("profile.noneActive"))?;
    let template = profile
        .config
        .prompts
        .iter()
        .find(|prompt| prompt.key == action)
        .ok_or_else(|| Error::not_found("prompt", action))?;
    if template.scope() != Scope::Canon {
        return Err(Error::refused("task.notCanonAction").param("action", template.label.as_str()));
    }
    let (card, kind) = crate::canon::fact::card_of(conn, &profile.config, note_id)?;
    let title = card.title.clone().unwrap_or_default();

    let mut prompt = format!(
        "The card is “{title}”, a {} of the canon (card `{}`).\n\n",
        kind.label.as_str().to_lowercase(),
        card.id
    );
    let mut attachments: Vec<PathBuf> = Vec::new();
    match template.produces() {
        Produces::CardPrompt => {
            let mut facts = String::new();
            for key in &kind.describe_from {
                for one in crate::canon::fact::in_section(conn, &card.id, key)? {
                    if one.status == crate::canon::FactStatus::Canon
                        && one.layer == crate::canon::Layer::Public
                    {
                        facts.push_str(&format!("- {}\n", one.body));
                    }
                }
            }
            if facts.is_empty() {
                prompt.push_str(
                    "There are no settled public facts about how it looks yet. Write from its name and say in one line that you had nothing to go on.\n\n",
                );
            } else {
                prompt.push_str(&format!("What is settled about how it looks:\n{facts}\n"));
            }
            if let Some(existing) = card.prompt.as_deref() {
                prompt.push_str(&format!(
                    "What its description says today, which you are rewriting:\n{existing}\n\n"
                ));
            }
            attachments = crate::asset::for_card(conn, &card.id)?
                .into_iter()
                .filter(|picture| matches!(picture.kind.as_str(), "portrait" | "reference"))
                .map(|picture| PathBuf::from(picture.path))
                .filter(|path| path.is_file())
                .collect();
            prompt.push_str(&template.template);
            if !attachments.is_empty() {
                prompt.push_str("\n\nReference pictures of it:\n");
                for path in &attachments {
                    prompt.push_str(&format!("{}\n", path.display()));
                }
            }
        }
        Produces::Canon => {
            let seen =
                crate::canon::view::render(conn, &card.id, crate::profile::config::Lens::Work)?;
            prompt.push_str(&format!("What the card holds now:\n\n{seen}\n\n"));
            let note = card.body.trim();
            if note.is_empty() {
                prompt.push_str("Its free note is empty.\n\n");
            } else {
                prompt.push_str(&format!("Its free note:\n\n{note}\n\n"));
            }
            prompt.push_str(&template.template);
            prompt.push_str(&crate::canon::proposal::instruction(&profile.config));
        }
        _ => {
            return Err(
                Error::refused("task.canonActionShape").param("action", template.label.as_str())
            );
        }
    }

    let prompt = super::waiting::instruct(&prompt);
    Ok((
        Composed {
            prompt,
            method: template.method().map(str::to_owned),
            key: card_key(action, &card.id),
            title: format!("{} · {title}", template.label.as_str()),
            attachments,
        },
        profile.id,
    ))
}

/// [`compose_for_card`] with the chat it will be answered in: on no work, for
/// the reason a style's is - a card belongs to the workspace's world.
pub fn prepare_for_card(conn: &Connection, note_id: &str, action: &str) -> Result<Prepared> {
    let (composed, profile_id) = compose_for_card(conn, note_id, action)?;
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

/// The key of a task about one comment: this action, on this comment. A
/// second click on the same comment is the same task; another comment's is
/// not.
pub fn comment_key(action: &str, comment_id: &str) -> String {
    format!("{action}:comment:{comment_id}")
}

/// The comment a task key names, when it names one.
pub fn comment_of_key(key: &str) -> Option<&str> {
    let mut parts = key.splitn(3, ':');
    parts.next()?;
    (parts.next()? == "comment").then(|| parts.next()).flatten()
}

/// The key of reading one screenshot: this action, on this channel, this
/// picture. The channel travels in the key because the answer is read back
/// against it — the picture cannot say which channel it is from — the way a
/// scene action's key carries its scene. A `:` or `%` in the channel's name
/// is escaped so the key still splits where it should.
pub fn screenshot_key(action: &str, channel: &str, shot: &str) -> String {
    let escaped = channel.replace('%', "%25").replace(':', "%3A");
    format!("{action}:channel:{escaped}:{shot}")
}

/// The channel a screenshot task's key names, when it names one.
pub fn channel_of_key(key: &str) -> Option<String> {
    let parts: Vec<&str> = key.splitn(4, ':').collect();
    let [_, "channel", escaped, _] = parts.as_slice() else {
        return None;
    };
    Some(escaped.replace("%3A", ":").replace("%25", "%"))
}

/// How many replies already posted on a channel are shown as its voice.
/// Enough to hear a manner in; few enough that the comment itself is not
/// buried under them.
const VOICE_SAMPLES: usize = 8;

/// How much of a work's text travels with a reply draft. A reply is about
/// the work, not a critique of it: the opening is enough to know what the
/// commenter heard, and a whole chapter would drown the comment.
const WORK_EXCERPT: usize = 3000;

/// The active profile's action by key, held to be about a comment and to
/// produce what the caller is about to compose.
fn comment_action<'a>(
    profile: &'a crate::profile::Profile,
    action: &str,
    produces: Produces,
) -> Result<&'a PromptTemplate> {
    let template = profile
        .config
        .prompts
        .iter()
        .find(|prompt| prompt.key == action)
        .ok_or_else(|| Error::not_found("prompt", action))?;
    if template.scope() != Scope::Comment {
        return Err(
            Error::refused("task.notCommentAction").param("action", template.label.as_str())
        );
    }
    if template.produces() != produces {
        return Err(match produces {
            Produces::Reply => Error::refused("task.actionReadsScreenshot")
                .param("action", template.label.as_str()),
            _ => Error::refused("task.actionDraftsReply").param("action", template.label.as_str()),
        });
    }
    Ok(template)
}

/// Compose `action` against a comment: a reply, in the voice of its channel.
///
/// The voice is the replies already posted on the same channel, given as
/// examples — the channel is only a word, so there is no setting to keep a
/// voice in, and what was said there describes how the channel speaks better
/// than a setting would. The action's method says the rest (ADR 0021).
pub fn compose_for_comment(
    conn: &Connection,
    comment_id: &str,
    action: &str,
) -> Result<(Composed, String)> {
    let profile = profile::active(conn)?.ok_or_else(|| Error::refused("profile.noneActive"))?;
    let template = comment_action(&profile, action, Produces::Reply)?;
    let comment = crate::comment::get(conn, comment_id)?
        .ok_or_else(|| Error::not_found("comment", comment_id))?;

    let mut prompt = format!("A comment on the channel “{}”", comment.channel);
    if let Some(author) = &comment.author {
        prompt.push_str(&format!(", from {author}"));
    }
    if let Some(day) = &comment.commented_on {
        prompt.push_str(&format!(", written on {day}"));
    }
    prompt.push_str(":\n\n");
    for line in comment.body.lines() {
        prompt.push_str(&format!("> {line}\n"));
    }

    let under = match comment.work_id.as_deref() {
        Some(id) => work::get(conn, id)?,
        None => None,
    };
    if let Some(work) = under {
        let kind = profile
            .config
            .kind(&work.kind)
            .map_or(work.kind.clone(), |kind| kind.label.as_str().to_lowercase());
        prompt.push_str(&format!(
            "\nIt was written under “{}”, a {kind}.",
            work.title
        ));
        let current = match work.current_version_id.as_deref() {
            Some(id) => version::get(conn, id)?,
            None => None,
        };
        if let Some(current) = current.as_ref() {
            let body = current.body.trim();
            if !body.is_empty() {
                let excerpt: String = body.chars().take(WORK_EXCERPT).collect();
                let cut = if excerpt.len() < body.len() {
                    "\n[…]"
                } else {
                    ""
                };
                prompt.push_str(&format!("\n\nIts text:\n\n{excerpt}{cut}\n"));
            }
        }
        // A reply is said in public: it is given the canon a public text may
        // see and nothing more - the internal layer never reaches it to be
        // repeated (ADR 0043).
        if !crate::canon::view::cards(
            conn,
            &profile.id,
            &crate::canon::view::CardFilter::default(),
        )?
        .is_empty()
        {
            let seen = super::prompt::canon_through(
                conn,
                &work,
                current.as_ref(),
                crate::profile::config::Lens::Public,
            )?;
            prompt.push_str(&format!(
                "\n\nWhat may be said in public about the world of the channel:\n\n{seen}\n"
            ));
        }
    }

    let voice = crate::comment::posted_on(
        conn,
        &profile.id,
        &comment.channel,
        &comment.id,
        VOICE_SAMPLES,
    )?;
    if voice.is_empty() {
        prompt.push_str(
            "\nNothing has been posted on this channel yet, so there is no voice to match: \
             write plainly and warmly, as the author would.\n",
        );
    } else {
        prompt.push_str(
            "\nReplies already posted on this channel, newest first — this is how the channel speaks:\n\n",
        );
        for (index, (said, answered)) in voice.iter().enumerate() {
            prompt.push_str(&format!(
                "{}. Comment: {}\n   Reply: {}\n",
                index + 1,
                one_line(said),
                one_line(answered)
            ));
        }
    }
    if let Some(draft) = comment.reply.as_deref().filter(|d| !d.trim().is_empty()) {
        prompt.push_str(&format!(
            "\nThe reply drafted so far, which you are rewriting:\n\n{draft}\n"
        ));
    }

    prompt.push('\n');
    prompt.push_str(&template.template);
    prompt.push_str(super::proposal::reply_instruction());
    let prompt = super::waiting::instruct(&prompt);

    let who = comment
        .author
        .clone()
        .unwrap_or_else(|| comment.body.chars().take(32).collect());
    Ok((
        Composed {
            prompt,
            method: template.method().map(str::to_owned),
            key: comment_key(action, comment_id),
            title: format!("{} · {who}", template.label.as_str()),
            attachments: Vec::new(),
        },
        profile.id,
    ))
}

/// [`compose_for_comment`] with the chat it will be answered in: on the
/// comment's work, when it has one, so the answer is where the work is.
pub fn prepare_for_comment(conn: &Connection, comment_id: &str, action: &str) -> Result<Prepared> {
    let (composed, profile_id) = compose_for_comment(conn, comment_id, action)?;
    let work_id = crate::comment::get(conn, comment_id)?.and_then(|comment| comment.work_id);
    prepared(conn, &profile_id, composed, work_id, action)
}

/// What reading a screenshot is about: the picture, and where it was pasted.
pub struct Screenshot<'a> {
    pub path: &'a Path,
    pub channel: &'a str,
    pub work_id: Option<&'a str>,
    /// Today in the person's own calendar, for turning "3 weeks ago" into a
    /// day. The backend knows only UTC, which after sunset is tomorrow.
    pub today: &'a str,
}

/// Compose `action` against a screenshot of a comment.
///
/// The picture travels as an attachment, the way a style's references do.
/// The channel is said in the prompt for context, and carried in the key for
/// the answer to be read back against — never taken from the answer.
pub fn compose_for_screenshot(
    conn: &Connection,
    action: &str,
    shot: &Screenshot<'_>,
) -> Result<(Composed, String)> {
    let profile = profile::active(conn)?.ok_or_else(|| Error::refused("profile.noneActive"))?;
    let template = comment_action(&profile, action, Produces::Comment)?;
    let channel = shot.channel.trim();
    if channel.is_empty() {
        return Err(Error::refused("task.needsChannel"));
    }
    if !shot.path.is_file() {
        return Err(
            Error::refused("task.screenshotMissing").param("path", shot.path.display().to_string())
        );
    }
    let under = match shot.work_id {
        Some(id) => Some(work::get(conn, id)?.ok_or_else(|| Error::not_found("work", id))?),
        None => None,
    };

    let mut prompt = format!("A screenshot of a comment from the channel “{channel}”");
    if let Some(work) = &under {
        prompt.push_str(&format!(", written under “{}”", work.title));
    }
    prompt.push_str(".\n\n");
    prompt.push_str(&template.template);
    prompt.push_str(&super::proposal::comment_instruction(shot.today));
    prompt.push_str(&format!("\n\nThe screenshot:\n{}\n", shot.path.display()));
    let prompt = super::waiting::instruct(&prompt);

    let stem = shot.path.file_stem().map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    Ok((
        Composed {
            prompt,
            method: template.method().map(str::to_owned),
            key: screenshot_key(action, channel, &stem),
            title: format!("{} · {channel}", template.label.as_str()),
            attachments: vec![shot.path.to_path_buf()],
        },
        profile.id,
    ))
}

/// [`compose_for_screenshot`] with the chat it will be answered in.
pub fn prepare_for_screenshot(
    conn: &Connection,
    action: &str,
    shot: &Screenshot<'_>,
) -> Result<Prepared> {
    let (composed, profile_id) = compose_for_screenshot(conn, action, shot)?;
    prepared(
        conn,
        &profile_id,
        composed,
        shot.work_id.map(str::to_owned),
        action,
    )
}

/// A composed task with a new chat of its own.
fn prepared(
    conn: &Connection,
    profile_id: &str,
    composed: Composed,
    work_id: Option<String>,
    action: &str,
) -> Result<Prepared> {
    let chat = super::create(
        conn,
        profile_id,
        super::NewChat {
            work_id,
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

/// A text on one line, for a list of examples.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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
    use crate::fixtures;
    use crate::link::{self, NewLink};
    use crate::scene::NewScene;
    use crate::work::NewWork;
    use crate::work::version::{self, NewVersion};

    #[test]
    fn a_channel_survives_the_key_with_its_colons_and_percents() {
        let key = screenshot_key("read-comment", "live: 100% raw", "shot");
        assert_eq!(channel_of_key(&key).as_deref(), Some("live: 100% raw"));
        assert_eq!(action_of_key_str(&key), "read-comment");
        assert_eq!(channel_of_key("critique:work-1"), None);
    }

    #[test]
    fn a_comment_key_names_its_comment_and_nothing_else_does() {
        assert_eq!(
            comment_of_key(&comment_key("reply-to-comment", "c1")),
            Some("c1")
        );
        assert_eq!(comment_of_key("critique:work-1"), None);
        assert_eq!(comment_of_key("critique:work-1:scene-2"), None);
    }

    fn action_of_key_str(key: &str) -> &str {
        key.split(':').next().unwrap_or_default()
    }

    fn post(conn: &Connection, profile_id: &str, channel: &str, body: &str, reply: &str) {
        let kept = crate::comment::create_minted(
            conn,
            profile_id,
            crate::comment::NewComment {
                channel: channel.into(),
                body: body.into(),
                ..Default::default()
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap();
        crate::comment::update_at(
            conn,
            &kept.id,
            crate::comment::CommentPatch {
                reply: Some(Some(reply.into())),
                state: Some(crate::comment::POSTED.into()),
                ..Default::default()
            },
            "2026-09-20T00:00:00Z",
        )
        .unwrap();
    }

    /// The reply is drafted against the comment, the work it is under, and
    /// the replies already posted on that channel — and no other channel's.
    #[test]
    fn a_reply_is_composed_in_the_voice_of_its_own_channel() {
        let (conn, profile_id) = fixtures::workspace();
        let work_id = crate::work::create(
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
            &conn,
            &work_id,
            NewVersion {
                role: "lyrics".into(),
                body: "the lights go down over the water".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
        post(
            &conn,
            &profile_id,
            "main",
            "great song",
            "thank you, friend!",
        );
        post(
            &conn,
            &profile_id,
            "second",
            "nice",
            "OFFICIAL REPLY FROM THE TEAM",
        );
        let comment = crate::comment::create_minted(
            &conn,
            &profile_id,
            crate::comment::NewComment {
                channel: "main".into(),
                body: "what is the bridge about?".into(),
                author: Some("anna".into()),
                work_id: Some(work_id),
                ..Default::default()
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap();

        let (composed, _) = compose_for_comment(&conn, &comment.id, "reply-to-comment").unwrap();

        assert!(composed.prompt.contains("> what is the bridge about?"));
        assert!(composed.prompt.contains("from anna"));
        assert!(composed.prompt.contains("“Harbour lights”"));
        assert!(
            composed
                .prompt
                .contains("the lights go down over the water")
        );
        assert!(
            composed.prompt.contains("thank you, friend!"),
            "the channel's own replies are its voice"
        );
        assert!(
            !composed.prompt.contains("OFFICIAL REPLY"),
            "another channel's replies are another voice"
        );
        assert_eq!(composed.key, comment_key("reply-to-comment", &comment.id));
        assert!(
            composed.method.is_some(),
            "the action's method travels with it"
        );
    }

    #[test]
    fn a_style_key_names_its_brick_and_nothing_else_does() {
        let key = style_key("describe-style", "b-7");
        assert_eq!(style_of_key(&key), Some("b-7"));
        assert_eq!(style_of_key("describe-style:comment:c-1"), None);
        assert_eq!(style_of_key("polish:w-1"), None);
    }

    #[test]
    fn a_comment_action_is_started_only_from_what_it_is_for() {
        let (conn, profile_id) = fixtures::workspace();
        let comment = crate::comment::create_minted(
            &conn,
            &profile_id,
            crate::comment::NewComment {
                channel: "main".into(),
                body: "hello".into(),
                ..Default::default()
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap();
        let work_id = crate::work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Any".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id;

        assert!(
            compose_for_comment(&conn, &comment.id, "read-comment").is_err(),
            "the reader reads screenshots, it does not draft replies"
        );
        assert!(compose_for_comment(&conn, &comment.id, "critique").is_err());
        assert!(
            compose(&conn, &work_id, "reply-to-comment", About::default()).is_err(),
            "an action about a comment is not a button on a work"
        );
    }

    #[test]
    fn a_screenshot_is_composed_as_an_attachment_on_its_channel() {
        let (conn, _) = fixtures::workspace();
        let dir = tempfile::tempdir().unwrap();
        let shot = dir.path().join("shot-7.png");
        std::fs::write(&shot, b"not really a png").unwrap();

        let (composed, _) = compose_for_screenshot(
            &conn,
            "read-comment",
            &Screenshot {
                path: &shot,
                channel: " main ",
                work_id: None,
                today: "2026-09-22",
            },
        )
        .unwrap();

        assert_eq!(composed.attachments, vec![shot.clone()]);
        assert!(composed.prompt.contains(&shot.display().to_string()));
        assert!(composed.prompt.contains("Today is 2026-09-22"));
        assert_eq!(channel_of_key(&composed.key).as_deref(), Some("main"));

        let nowhere = compose_for_screenshot(
            &conn,
            "read-comment",
            &Screenshot {
                path: &shot,
                channel: "  ",
                work_id: None,
                today: "2026-09-22",
            },
        );
        assert!(
            nowhere.is_err(),
            "a comment is filed under a channel, so one must be said"
        );
    }

    /// The type's own instruction reaches the model, the author's steer is
    /// kept apart and labelled, and neither is left to be guessed at.
    #[test]
    fn describing_a_brick_carries_the_types_question_and_the_authors_steer() {
        let (conn, profile_id) = fixtures::workspace();
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
        let (conn, profile_id) = fixtures::workspace();
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
        let (mut conn, profile_id) = fixtures::workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");

        let refused = compose(&conn, &work_id, "describe-style", About::default()).unwrap_err();

        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("task.isStyleAction"),
            "{refused}"
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
        let (mut conn, profile_id) = fixtures::workspace();
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
        let (mut conn, profile_id) = fixtures::workspace();
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
        let (mut conn, profile_id) = fixtures::workspace();
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
        let (mut conn, profile_id) = fixtures::workspace();
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
        let (mut conn, profile_id) = fixtures::workspace();
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
        let (mut conn, profile_id) = fixtures::workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");

        let refused = prepare(&conn, &work_id, "no-such-action", About::default());

        assert!(refused.is_err());
    }

    #[test]
    fn a_failed_task_leaves_no_chat_behind() {
        let (mut conn, profile_id) = fixtures::workspace();
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
        let (conn, _) = fixtures::workspace();
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
        fixtures::video(conn, profile_id, "The clip").id
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
        let (mut conn, profile_id) = fixtures::workspace();
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
        let (conn, profile_id) = fixtures::workspace();
        let video_id = video(&conn, &profile_id);

        let refused = compose(&conn, &video_id, "critique", About::default()).unwrap_err();

        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("task.wrongKindForAction"),
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
        let (mut conn, profile_id) = fixtures::workspace();
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
        let refused = compose(&conn, &video_id, "prompts", about(Some("grade"))).unwrap_err();
        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("task.unknownActionBlock"),
            "{refused}"
        );
        assert_eq!(
            refused.refusal().and_then(|r| r.params.get("block")),
            Some(&serde_json::Value::String("grade".into())),
            "names what was asked: {refused}"
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
        .unwrap_err();
        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("task.needsSceneForBlock"),
            "{refused}"
        );
    }

    #[test]
    fn a_scene_action_needs_its_scene_and_names_it_in_the_key() {
        let (mut conn, profile_id) = fixtures::workspace();
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
        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("task.needsScene"),
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
        let (conn, profile_id) = fixtures::workspace();
        let video_id = video(&conn, &profile_id);

        let refused = compose(&conn, &video_id, "storyboard", About::default()).unwrap_err();

        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("prompt.roleEmpty"),
            "{refused}"
        );
    }

    #[test]
    fn the_plot_action_reads_the_donor_and_refuses_without_one() {
        let (mut conn, profile_id) = fixtures::workspace();
        let song_id = work_with_body(
            &mut conn,
            &profile_id,
            "Harbour lights",
            "the cranes go still",
        );
        let video_id = video(&conn, &profile_id);

        let refused = compose(&conn, &video_id, "plot", About::default()).unwrap_err();
        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("prompt.noDonor"),
            "{refused}"
        );

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
        let (mut conn, profile_id) = fixtures::workspace();
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
        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("task.fileMissing"),
            "{refused}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod version_tests {
    use super::*;
    use crate::fixtures;
    use crate::work::NewWork;
    use crate::work::version::{self, NewVersion};

    /// A task started on a version reads that version, remembers it on the
    /// chat with the action, and a task started on the work as a whole
    /// remembers the action alone.
    #[test]
    fn a_task_on_a_version_reads_it_and_the_chat_remembers_it() {
        let (conn, profile_id) = fixtures::workspace();
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
        let make = |body: &str| {
            version::create(
                &conn,
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
        let (conn, profile_id) = fixtures::workspace();
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
            &conn,
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

    #[test]
    fn a_card_is_described_from_its_settled_public_looks_and_its_pictures() {
        let (conn, profile_id) = fixtures::workspace();
        let card = fixtures::card(&conn, &profile_id, "character", "Wren");
        fixtures::fact(&conn, &card.id, "looks", "Freckles across the nose.");
        fixtures::fact(&conn, &card.id, "bio", "Born in Petersburg.");

        let (composed, _) = compose_for_card(&conn, &card.id, "describe-card").unwrap();
        assert!(composed.prompt.contains("Freckles across the nose."));
        assert!(
            !composed.prompt.contains("Petersburg"),
            "the biography went to the generator"
        );
        assert_eq!(composed.key, card_key("describe-card", &card.id));
        assert_eq!(card_of_key(&composed.key), Some(card.id.as_str()));
        assert!(composed.method.is_some());

        let (gathering, _) = compose_for_card(&conn, &card.id, "gather-card").unwrap();
        assert!(
            gathering.prompt.contains("```json"),
            "the block to answer in is not asked for"
        );

        assert!(
            compose(
                &conn,
                &fixtures::song(&conn, &profile_id, "x").id,
                "gather-card",
                About::default()
            )
            .is_err(),
            "an action about a card was aimed at a work"
        );
    }

    #[test]
    fn an_action_about_a_selection_reads_it_and_refuses_without_one() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Kiln");
        fixtures::version(&conn, &song.id, "lyrics", "у Льва в подвале печь");
        let otto = fixtures::card(&conn, &profile_id, "character", "Лев");
        fixtures::fact(&conn, &otto.id, "identity", "A ceramicist.");

        assert!(compose(&conn, &song.id, "to-canon", About::default()).is_err());

        let composed = compose(
            &conn,
            &song.id,
            "to-canon",
            About {
                selection: Some("у Льва в подвале печь"),
                ..About::default()
            },
        )
        .unwrap();
        assert!(composed.prompt.contains("у Льва в подвале печь"));
        assert!(composed.prompt.contains("Лев"), "the canon was not given");
        assert!(
            composed.prompt.contains("\"facts\""),
            "the block to answer in is not asked for"
        );
    }

    #[test]
    fn the_canon_of_a_work_is_its_cards_by_name_and_in_full_those_it_is_about() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Kiln");
        let lyric = fixtures::version(&conn, &song.id, "lyrics", "Otto lights the kiln");
        let otto = fixtures::card(&conn, &profile_id, "character", "Otto");
        fixtures::fact(&conn, &otto.id, "identity", "A ceramicist.");
        let other = fixtures::card(&conn, &profile_id, "character", "Pashka");
        fixtures::fact(&conn, &other.id, "identity", "A sound engineer.");

        let work = crate::work::get(&conn, &song.id).unwrap().unwrap();
        let canon = super::super::prompt::canon_of_work(&conn, &work, Some(&lyric)).unwrap();
        assert!(canon.contains("Pashka") && canon.contains("Otto"));
        assert!(
            canon.contains("A ceramicist."),
            "the card the text names was not given whole"
        );
        assert!(
            !canon.contains("A sound engineer."),
            "a card the work is not about was given whole"
        );
    }

    /// A card of the canon with one public fact and one internal one.
    fn otto_with_a_secret(conn: &Connection, profile_id: &str) -> crate::note::Note {
        let otto = fixtures::card(conn, profile_id, "character", "Otto");
        fixtures::fact(conn, &otto.id, "identity", "A ceramicist.");
        crate::canon::fact::create_minted(
            conn,
            crate::canon::NewFact {
                note_id: otto.id.clone(),
                section: "bio".into(),
                body: "Lives above the bakery on Mill Lane.".into(),
                layer: Some(crate::canon::Layer::Internal),
                ..crate::canon::NewFact::default()
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap();
        otto
    }

    #[test]
    fn a_public_text_reads_the_canon_without_its_internal_layer() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Kiln");
        let lyric = fixtures::version(&conn, &song.id, "lyrics", "Otto lights the kiln");
        crate::note::create(
            &conn,
            &profile_id,
            crate::note::NewNote {
                kind: Some("character".into()),
                title: Some("Nobody Knows".into()),
                layer: Some(crate::canon::Layer::Internal),
                ..crate::note::NewNote::default()
            },
        )
        .unwrap();
        otto_with_a_secret(&conn, &profile_id);
        let work = crate::work::get(&conn, &song.id).unwrap().unwrap();

        let rendered = super::super::prompt::for_work(
            &conn,
            &work.id,
            "{canon:public}",
            super::super::prompt::Context::default(),
        )
        .unwrap();
        assert!(rendered.contains("A ceramicist."), "{rendered}");
        assert!(!rendered.contains("Mill Lane"), "{rendered}");
        assert!(
            !rendered.contains("Nobody Knows"),
            "a card that publicly does not exist was named: {rendered}"
        );

        let own = super::super::prompt::canon_of_work(&conn, &work, Some(&lyric)).unwrap();
        assert!(
            own.contains("Mill Lane"),
            "the work reads every layer: {own}"
        );
    }

    #[test]
    fn a_reply_is_given_the_canon_a_public_text_may_see() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Kiln");
        fixtures::version(&conn, &song.id, "lyrics", "Otto lights the kiln");
        otto_with_a_secret(&conn, &profile_id);
        let comment = crate::comment::create_minted(
            &conn,
            &profile_id,
            crate::comment::NewComment {
                channel: "main".into(),
                body: "who is Otto?".into(),
                work_id: Some(song.id.clone()),
                ..Default::default()
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap();

        let (composed, _) = compose_for_comment(&conn, &comment.id, "reply-to-comment").unwrap();

        assert!(
            composed.prompt.contains("A ceramicist."),
            "{}",
            composed.prompt
        );
        assert!(
            !composed.prompt.contains("Mill Lane"),
            "the internal layer reached a public reply: {}",
            composed.prompt
        );
    }
}
