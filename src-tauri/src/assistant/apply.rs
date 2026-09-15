//! Applying a proposal: the one place a proposal becomes rows.
//!
//! A proposal — the panel's fenced score, an agent's version, note or whole
//! package — is a message with `meta.proposal`. Until v0.57.1 each kind was
//! applied by the screen that showed it, through the commands a hand uses,
//! and nothing marked the message: "added" was a variable in the component,
//! gone on the next fetch, and a package of five proposals meant five clicks
//! and a walk through the tabs to see which were already in. Now applying is
//! one function over any kind. It writes the same operations a hand would —
//! a replay cannot tell them apart, which is the point — and it stamps
//! `meta.applied` on the message with what it made, so the mark survives a
//! refetch, a package is one click, and *apply all* is a loop over a chat.
//!
//! Nothing here is reached by an agent. `kilna --mcp` writes the proposal;
//! the window calls this when a person clicks. See ADR 0018.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::assistant::proposal::{BoardChange, Marks, PackagedNote, PackagedScene, Proposal};
use crate::assistant::{self, ASSISTANT, Chat, Message};
use crate::commands::{recording, restate, was};
use crate::error::{Error, Result};
use crate::journal::{self, Record};
use crate::minted::Minted;
use crate::note::{self, NewNote};
use crate::operation;
use crate::profile;
use crate::scene::{self, NewScene, ScenePatch};
use crate::score::{self, NewScore};
use crate::time;
use crate::trash;
use crate::work::version::{self, NewVersion};
use crate::work::{self, NewWork, WorkPatch};

/// What a person may change about a version on the way in: the dialog lets
/// them pick the role, name the version and make it current.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Overrides {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub make_current: Option<bool>,
}

/// What applying made. Returned to the caller and stamped on the message as
/// `meta.applied`, so the chat shows the mark and the work it points at.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Outcome {
    pub message_id: String,
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_id: Option<String>,
    /// True when the package created the work rather than adding to one.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub created_work: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub versions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    /// The overview fields written, by key.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    /// Scenes written onto the board: created, or rewritten in place when
    /// a board was replaced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenes: Vec<String>,
    /// Scenes a replaced board sent to the trash, by trash entry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_scenes: Vec<String>,
}

/// Whether a message carries a proposal nobody has applied yet.
pub fn is_pending(message: &Message) -> bool {
    message.role == ASSISTANT
        && message.meta.get("proposal").is_some_and(Value::is_object)
        && !message.meta.contains_key("applied")
}

/// Apply the proposal a message carries, and mark the message.
///
/// One proposal at a time, each write its own operation: a package that
/// fails halfway leaves what it wrote — a work with two of its three
/// versions — rather than nothing, and the message stays unmarked, so the
/// person sees what landed and applies the rest by hand or not at all. The
/// checks that can fail run first, before a row is written, so halfway is
/// the rare case: a role or a kind the profile does not have is refused
/// with nothing done.
pub fn apply(
    conn: &mut Connection,
    profile_id: &str,
    message_id: &str,
    overrides: Overrides,
) -> Result<Outcome> {
    let message = assistant::message(conn, message_id)?
        .ok_or_else(|| Error::not_found("message", message_id))?;
    if message.meta.contains_key("applied") {
        return Err(Error::Other("this proposal is already applied".into()));
    }
    // A storyboard stored by v0.62 carried `replace: true` where v0.64
    // writes `change: "replace"`; read as it was meant, not as `add`.
    let stored = message.meta.get("proposal").cloned().map(|mut value| {
        if value.get("kind").and_then(Value::as_str) == Some("scenes")
            && value.get("replace").and_then(Value::as_bool) == Some(true)
            && value.get("change").is_none()
        {
            if let Some(object) = value.as_object_mut() {
                object.insert("change".into(), json!("replace"));
            }
        }
        value
    });
    let proposal: Proposal = match stored.as_ref() {
        Some(raw) => serde_json::from_value(raw.clone())?,
        None => return Err(Error::Other("this message proposes nothing".into())),
    };
    let chat = assistant::get(conn, &message.chat_id)?
        .ok_or_else(|| Error::not_found("chat", &message.chat_id))?;
    if chat.profile_id != profile_id {
        return Err(Error::Other(
            "the proposal belongs to another profile".into(),
        ));
    }
    // Who proposed, when it was an agent outside the window: a score it
    // proposed is judged by it, not by the author, and the history says so.
    let client = message
        .meta
        .get("client")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let config = profile::config_for(conn, profile_id)?;

    let mut outcome = Outcome {
        message_id: message_id.to_owned(),
        at: time::now(),
        work_id: chat.work_id.clone(),
        ..Outcome::default()
    };

    match proposal {
        Proposal::Version { role, label } => {
            let work_id = on_work(&chat, "a version")?;
            let found =
                work::get(conn, &work_id)?.ok_or_else(|| Error::not_found("work", &work_id))?;
            let role = overrides.role.unwrap_or(role);
            check_role(&config, &found.kind, &role)?;
            let label = overrides
                .label
                .or(label)
                .filter(|label| !label.trim().is_empty());
            // Not current unless the person said so: an answer worth keeping
            // is not yet an answer worth standing behind — the dialog's rule.
            let make_current = overrides.make_current.unwrap_or(false);
            // A commentary is about the version its chat was started on: the
            // critique of revision 2 says so, and the versions tab shows it
            // beside revision 2 rather than beside whichever revision shares
            // its number.
            let meta = about_version(conn, &chat, &work_id).map(|about| {
                let mut meta = serde_json::Map::new();
                meta.insert("about".into(), serde_json::Value::String(about));
                meta
            });
            let version = write_version(
                conn,
                profile_id,
                &work_id,
                NewVersion {
                    role,
                    body: message.body.clone(),
                    label,
                    meta,
                    make_current,
                    parent_version_id: None,
                },
            )?;
            outcome.versions.push(version);
        }

        Proposal::Score { axes, note, .. } => {
            let work_id = on_work(&chat, "a score")?;
            // Tied to the version the chat is about, when it is about one:
            // a score judged revision 2 is a snapshot of revision 2.
            let version_id = about_version(conn, &chat, &work_id);
            outcome.score = Some(write_score(
                conn,
                profile_id,
                &work_id,
                version_id,
                Marks {
                    axes,
                    note,
                    unknown: Vec::new(),
                    missing: Vec::new(),
                },
                client,
            )?);
        }

        Proposal::Note { title } => {
            outcome.notes.push(write_note(
                conn,
                profile_id,
                chat.work_id.as_deref(),
                PackagedNote {
                    title,
                    body: message.body.clone(),
                },
            )?);
        }

        Proposal::Work {
            title,
            work_kind,
            fields,
            versions,
            score,
            notes,
            scenes,
            ..
        } => {
            // A package on a work adds to it; one in a chat on nothing is a
            // new work and has to say what it is.
            let existing = match &chat.work_id {
                Some(id) => Some(work::get(conn, id)?.ok_or_else(|| Error::not_found("work", id))?),
                None => None,
            };
            let kind = match &existing {
                Some(found) => found.kind.clone(),
                None => work_kind
                    .filter(|kind| !kind.trim().is_empty())
                    .ok_or_else(|| Error::Other("a new work needs a `kind`".into()))?,
            };
            if config.kind(&kind).is_none() {
                return Err(Error::Other(format!(
                    "the profile has no kind of work `{kind}`"
                )));
            }
            for packaged in &versions {
                check_role(&config, &kind, &packaged.role)?;
            }

            let (work_id, fresh) = match existing {
                Some(found) => (found.id, false),
                None => {
                    let title = title
                        .map(|title| title.trim().to_owned())
                        .filter(|title| !title.is_empty())
                        .ok_or_else(|| Error::Other("a new work needs a `title`".into()))?;
                    let id = write_work(
                        conn,
                        profile_id,
                        NewWork {
                            kind: kind.clone(),
                            title,
                            meta: Some(fields.clone()),
                            ..NewWork::default()
                        },
                    )?;
                    outcome.fields = fields.keys().cloned().collect();
                    (id, true)
                }
            };

            if !fresh && !fields.is_empty() {
                outcome.fields = write_fields(conn, profile_id, &work_id, fields)?;
            }
            // A work has one current version, and the score judges it. On a
            // new work it is the package's version in the vocabulary's first
            // role — the lyrics, not the style prompt — or its first version
            // when no role matches; the rest are read as the newest of their
            // role, as the card reads them. On a work that has versions, a
            // package's wait beside the current one like any proposal's.
            let leading = fresh
                .then(|| {
                    config
                        .vocabulary(&kind)
                        .version_roles
                        .iter()
                        .find_map(|role| versions.iter().position(|v| v.role == role.key))
                        .or(if versions.is_empty() { None } else { Some(0) })
                })
                .flatten();
            for (index, packaged) in versions.into_iter().enumerate() {
                outcome.versions.push(write_version(
                    conn,
                    profile_id,
                    &work_id,
                    NewVersion {
                        role: packaged.role,
                        body: packaged.body,
                        label: packaged.label,
                        meta: None,
                        make_current: leading == Some(index),
                        parent_version_id: None,
                    },
                )?);
            }
            if let Some(marks) = score {
                outcome.score = Some(write_score(
                    conn, profile_id, &work_id, None, marks, client,
                )?);
            }
            for packaged in notes {
                outcome
                    .notes
                    .push(write_note(conn, profile_id, Some(&work_id), packaged)?);
            }
            // A package adds its scenes after the last; replacing a board
            // is the scenes proposal's own decision, not a package's.
            if !scenes.is_empty() {
                check_storyboard(&config, &kind)?;
                for packaged in scenes {
                    let position = packaged.position;
                    outcome
                        .scenes
                        .push(write_scene(conn, profile_id, &work_id, packaged, position)?);
                }
            }
            outcome.created_work = fresh;
            outcome.work_id = Some(work_id);
        }

        Proposal::Scenes { scenes, change } => {
            let work_id = on_work(&chat, "a storyboard")?;
            let found =
                work::get(conn, &work_id)?.ok_or_else(|| Error::not_found("work", &work_id))?;
            check_storyboard(&config, &found.kind)?;
            match change {
                BoardChange::Replace => {
                    // By number: the scene that already holds a number is
                    // rewritten in place and keeps its id, what the new
                    // board does not number goes to the trash, and a number
                    // nobody holds is a new row. A board rebuilt from
                    // scratch would be a board whose every row is a stranger
                    // to what pointed at it.
                    let mut standing = scene::for_work(conn, &work_id)?;
                    for (index, packaged) in scenes.into_iter().enumerate() {
                        let position = packaged.position.unwrap_or(index as i64 + 1);
                        match standing.iter().position(|s| s.position == position) {
                            Some(at) => {
                                let existing = standing.remove(at);
                                outcome.scenes.push(rewrite_scene(
                                    conn,
                                    profile_id,
                                    &existing.id,
                                    packaged,
                                    position,
                                )?);
                            }
                            None => outcome.scenes.push(write_scene(
                                conn,
                                profile_id,
                                &work_id,
                                packaged,
                                Some(position),
                            )?),
                        }
                    }
                    for leftover in standing {
                        outcome
                            .removed_scenes
                            .push(discard_scene(conn, profile_id, &leftover.id)?);
                    }
                }
                BoardChange::Revise => {
                    // Only the numbers named, and only in what is said about
                    // them: the rest of the board is not looked at. A number
                    // nobody holds is a new row, as on a replaced board — a
                    // revision that adds scene 9 to a board of 8 is a
                    // revision, not a mistake.
                    let standing = scene::for_work(conn, &work_id)?;
                    for packaged in scenes {
                        let position = packaged.position.ok_or_else(|| {
                            Error::Other("a revised scene names its number on the board".into())
                        })?;
                        match standing.iter().find(|s| s.position == position) {
                            Some(existing) => outcome.scenes.push(revise_scene(
                                conn,
                                profile_id,
                                &existing.id,
                                packaged,
                            )?),
                            None => outcome.scenes.push(write_scene(
                                conn,
                                profile_id,
                                &work_id,
                                packaged,
                                Some(position),
                            )?),
                        }
                    }
                }
                BoardChange::Add => {
                    for packaged in scenes {
                        let position = packaged.position;
                        outcome
                            .scenes
                            .push(write_scene(conn, profile_id, &work_id, packaged, position)?);
                    }
                }
            }
        }
    }

    // The status follows the facts, as after any hand-made version or score.
    if let Some(work_id) = &outcome.work_id {
        restate(conn, profile_id, work_id);
    }

    let mut meta = message.meta;
    meta.insert("applied".into(), serde_json::to_value(&outcome)?);
    assistant::set_meta(conn, message_id, &meta)?;

    Ok(outcome)
}

/// Apply every proposal in a chat nobody has applied yet, oldest first.
///
/// Stops at the first failure and says which message it was, with the ones
/// before it applied and marked: the person sees where it stopped rather
/// than a chat where nothing happened.
pub fn apply_pending(
    conn: &mut Connection,
    profile_id: &str,
    chat_id: &str,
) -> Result<Vec<Outcome>> {
    let transcript =
        assistant::transcript(conn, chat_id)?.ok_or_else(|| Error::not_found("chat", chat_id))?;
    let mut outcomes = Vec::new();
    for message in transcript.messages.iter().filter(|m| is_pending(m)) {
        match apply(conn, profile_id, &message.id, Overrides::default()) {
            Ok(outcome) => outcomes.push(outcome),
            Err(err) => {
                return Err(Error::Other(format!(
                    "{} applied, then one could not be: {err}",
                    outcomes.len()
                )));
            }
        }
    }
    Ok(outcomes)
}

fn on_work(chat: &Chat, what: &str) -> Result<String> {
    chat.work_id.clone().ok_or_else(|| {
        Error::Other(format!(
            "{what} needs a work, and this chat is about nothing"
        ))
    })
}

fn check_role(config: &profile::config::ProfileConfig, kind: &str, role: &str) -> Result<()> {
    if config
        .vocabulary(kind)
        .version_roles
        .iter()
        .any(|r| r.key == role)
    {
        Ok(())
    } else {
        Err(Error::Other(format!(
            "no version role `{role}` for `{kind}`"
        )))
    }
}

/// The stable key operations name a profile by — see ADR 0014.
fn profile_key(conn: &Connection, profile_id: &str) -> Result<String> {
    profile::key_for_id(conn, profile_id)?.ok_or_else(|| Error::not_found("profile", profile_id))
}

// Each writer below is the body of the command a hand reaches the same row
// through — the same intent, the same params, the same journal line — so
// the operations log cannot tell a proposal applied from a thing typed. If
// the command changes, this has to change with it; `operation_coverage.rs`
// and `replay_rebuilds.rs` are the gates that notice.

fn write_work(conn: &mut Connection, profile_id: &str, new: NewWork) -> Result<String> {
    let minted = Minted::fresh();
    let logged = operation::Intent::new("work.create")
        .in_profile(profile_id)
        .param("profile", profile_key(conn, profile_id)?)
        .param("work", serde_json::to_value(&new)?)
        .minted(&minted);
    let created = recording(conn, logged, |tx| {
        work::create_minted(tx, profile_id, new, minted)
    })?;
    journal::record(
        conn,
        profile_id,
        Record::new("work.created")
            .param("title", created.title.clone())
            .about("work", created.id.clone()),
    );
    Ok(created.id)
}

/// Merge fields into a work's overview, keeping what the package did not name.
fn write_fields(
    conn: &mut Connection,
    profile_id: &str,
    work_id: &str,
    fields: Map<String, Value>,
) -> Result<Vec<String>> {
    let before = work::get(conn, work_id)?;
    let mut meta = before.as_ref().map(|w| w.meta.clone()).unwrap_or_default();
    let written: Vec<String> = fields.keys().cloned().collect();
    meta.extend(fields);
    let patch = WorkPatch {
        meta: Some(meta),
        ..WorkPatch::default()
    };
    let at = time::now();
    let logged = operation::Intent::new("work.update")
        .in_profile(profile_id)
        .param("profile", profile_key(conn, profile_id)?)
        .param("id", work_id.to_owned())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(before.as_ref(), &patch)?)
        .param("at", at.clone());
    recording(conn, logged, |tx| work::update_at(tx, work_id, patch, &at))?;
    Ok(written)
}

fn write_version(
    conn: &mut Connection,
    profile_id: &str,
    work_id: &str,
    new: NewVersion,
) -> Result<String> {
    let minted = Minted::fresh();
    let logged = operation::Intent::new("version.create")
        .in_profile(profile_id)
        .param("profile", profile_key(conn, profile_id)?)
        .param("workId", work_id.to_owned())
        .param("version", serde_json::to_value(&new)?)
        .minted(&minted);
    let created = version::create_minted(conn, work_id, new, minted, Some(logged))?;
    journal::record(
        conn,
        profile_id,
        Record::new("version.created")
            .param("role", created.role.clone())
            .param("revision", created.revision)
            .param(
                "title",
                journal::work_title(conn, work_id).unwrap_or_default(),
            )
            .about("work", work_id.to_owned()),
    );
    Ok(created.id)
}

/// The version a chat is about, when it still is: the one the action was
/// started on, if it belongs to this work and has not been deleted since.
/// Anything else binds to nothing, which the score reads as "the current
/// version" — the same answer a chat started from the overview gives.
fn about_version(conn: &Connection, chat: &assistant::Chat, work_id: &str) -> Option<String> {
    let id = chat.version_id.as_deref()?;
    let found = version::get(conn, id).ok()??;
    (found.work_id == work_id).then_some(found.id)
}

fn write_score(
    conn: &mut Connection,
    profile_id: &str,
    work_id: &str,
    version_id: Option<String>,
    marks: Marks,
    rater: Option<String>,
) -> Result<String> {
    let new = NewScore {
        axes: marks.axes,
        version_id,
        note: marks.note,
        rater,
    };
    let minted = Minted::fresh();
    let logged = operation::Intent::new("score.create")
        .in_profile(profile_id)
        .param("profile", profile_key(conn, profile_id)?)
        .param("workId", work_id.to_owned())
        .param("score", serde_json::to_value(&new)?)
        .minted(&minted);
    let created = recording(conn, logged, |tx| {
        score::create_minted(tx, work_id, new, minted)
    })?;
    journal::record(
        conn,
        profile_id,
        Record::new("score.added")
            .param(
                "title",
                journal::work_title(conn, work_id).unwrap_or_default(),
            )
            .param("total", (created.total * 10.0).round() / 10.0)
            .param("tier", created.tier.clone().unwrap_or_default())
            .about("work", work_id.to_owned()),
    );
    Ok(created.id)
}

fn write_note(
    conn: &mut Connection,
    profile_id: &str,
    work_id: Option<&str>,
    packaged: PackagedNote,
) -> Result<String> {
    let new = NewNote {
        body: packaged.body,
        kind: None,
        title: packaged.title.filter(|title| !title.trim().is_empty()),
        work_id: work_id.map(str::to_owned),
        tags: Vec::new(),
    };
    let minted = Minted::fresh();
    let logged = operation::Intent::new("note.create")
        .in_profile(profile_id)
        .param("profile", profile_key(conn, profile_id)?)
        .param("note", serde_json::to_value(&new)?)
        .minted(&minted);
    let created = recording(conn, logged, |tx| {
        note::create_minted(tx, profile_id, new, minted)
    })?;
    Ok(created.id)
}

/// A kind that has a storyboard, or the reason it takes no scenes.
fn check_storyboard(config: &profile::config::ProfileConfig, kind: &str) -> Result<()> {
    if scene::kind_has_scenes(config, kind) {
        Ok(())
    } else {
        Err(Error::Other(format!(
            "`{kind}` has no storyboard: the kind names no kinds of shot and no prompt blocks"
        )))
    }
}

/// The row a packaged scene becomes, numbered as told or after the last.
fn write_scene(
    conn: &mut Connection,
    profile_id: &str,
    work_id: &str,
    packaged: PackagedScene,
    position: Option<i64>,
) -> Result<String> {
    let new = NewScene {
        work_id: work_id.to_owned(),
        position,
        section: packaged.section,
        starts_at: packaged.starts_at,
        ends_at: packaged.ends_at,
        shot_type: packaged.shot_type,
        description: Some(packaged.description),
        blocks: Some(packaged.blocks),
    };
    let minted = Minted::fresh();
    let logged = operation::Intent::new("scene.create")
        .in_profile(profile_id)
        .param("profile", profile_key(conn, profile_id)?)
        .param("scene", serde_json::to_value(&new)?)
        .minted(&minted);
    let created = recording(conn, logged, |tx| {
        scene::create_minted(tx, profile_id, new, minted)
    })?;
    journal::record(
        conn,
        profile_id,
        Record::new("scene.created")
            .param(
                "title",
                journal::work_title(conn, work_id).unwrap_or_default(),
            )
            .param("number", created.position)
            .about("work", work_id.to_owned()),
    );
    Ok(created.id)
}

/// A scene rewritten whole by a replaced board: every field set, the blocks
/// as a set, so the log's `before` holds the row as it was and an undo puts
/// the whole row back.
fn rewrite_scene(
    conn: &mut Connection,
    profile_id: &str,
    id: &str,
    packaged: PackagedScene,
    position: i64,
) -> Result<String> {
    let before = scene::get(conn, id)?;
    let patch = ScenePatch {
        position: Some(position),
        section: Some(packaged.section),
        starts_at: Some(packaged.starts_at),
        ends_at: Some(packaged.ends_at),
        shot_type: Some(packaged.shot_type),
        description: Some(packaged.description),
        blocks: Some(packaged.blocks),
    };
    let at = time::now();
    let logged = operation::Intent::new("scene.update")
        .in_profile(profile_id)
        .param("profile", profile_key(conn, profile_id)?)
        .param("id", id.to_owned())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(before.as_ref(), &patch)?)
        .param("at", at.clone());
    recording(conn, logged, |tx| scene::update_at(tx, id, patch, &at))?;
    Ok(id.to_owned())
}

/// A scene revised in place: only what the proposal says about it changes.
/// A field left out is kept — a revision that brings the prompt blocks says
/// nothing about the seconds — and a block left out is kept too: the blocks
/// named are laid over the ones the scene holds, so an action aimed at one
/// block cannot delete the others (0.66). An emptied block is cleared.
fn revise_scene(
    conn: &mut Connection,
    profile_id: &str,
    id: &str,
    packaged: PackagedScene,
) -> Result<String> {
    let before = scene::get(conn, id)?;
    // The blocks the revision names are laid over the ones the scene holds,
    // not put in their place. A task aimed at one block — the animation,
    // rewritten without touching the still — comes back carrying only that
    // block, and setting the map whole would delete the two it said nothing
    // about. A block whose text comes back empty is cleared, which is what
    // an emptied box means on the board.
    let blocks = (!packaged.blocks.is_empty()).then(|| {
        let mut merged = before
            .as_ref()
            .map(|scene| scene.blocks.clone())
            .unwrap_or_default();
        for (key, value) in packaged.blocks {
            let emptied = value.as_str().is_some_and(|text| text.trim().is_empty());
            if emptied {
                merged.remove(&key);
            } else {
                merged.insert(key, value);
            }
        }
        merged
    });
    let patch = ScenePatch {
        position: None,
        section: packaged.section.map(Some),
        starts_at: packaged.starts_at.map(Some),
        ends_at: packaged.ends_at.map(Some),
        shot_type: packaged.shot_type.map(Some),
        description: (!packaged.description.trim().is_empty()).then_some(packaged.description),
        blocks,
    };
    let at = time::now();
    let logged = operation::Intent::new("scene.update")
        .in_profile(profile_id)
        .param("profile", profile_key(conn, profile_id)?)
        .param("id", id.to_owned())
        .param("patch", serde_json::to_value(&patch)?)
        .param("before", was(before.as_ref(), &patch)?)
        .param("at", at.clone());
    recording(conn, logged, |tx| scene::update_at(tx, id, patch, &at))?;
    Ok(id.to_owned())
}

/// A scene the new board has no number for, sent to the trash the way the
/// board's own delete button sends one: the same operation, the same line
/// in the history, so it comes back through the trash like any deletion.
fn discard_scene(conn: &mut Connection, profile_id: &str, id: &str) -> Result<String> {
    let minted = Minted::fresh();
    let logged = operation::Intent::new("entity.discard")
        .in_profile(profile_id)
        .param("profile", profile_key(conn, profile_id)?)
        .param("entity", trash::Entity::Scene.as_str())
        .param("entityId", id)
        .minted(&minted);
    let entry_id = trash::discard_minted(conn, trash::Entity::Scene, id, minted, Some(logged))?;
    let described = trash::list(conn, profile_id)?
        .into_iter()
        .find(|entry| entry.id == entry_id);
    let mut record = Record::new("scene.deleted").param(
        "label",
        described
            .as_ref()
            .map_or_else(|| id.to_owned(), |entry| entry.label.clone()),
    );
    if let Some(origin) = described.as_ref().and_then(|entry| entry.origin.clone()) {
        record = record.param("origin", origin);
    }
    if let Some(work_id) = trash::snapshot_work_id(conn, trash::Entity::Scene, id) {
        record = record.about("work", work_id);
    }
    journal::record(conn, profile_id, record);
    Ok(entry_id)
}

/// The message body a package is shown with: everything it would write,
/// readable before it is written.
///
/// Plain roles go in a fence so their lines stay lines — the same monospace
/// column the versions panel gives them; markdown roles are markdown. The
/// fence is chosen longer than any run of backticks in the text, so a lyric
/// with a code fence in it does not end the block early.
pub fn render_package(
    proposal: &Proposal,
    config: &profile::config::ProfileConfig,
    kind: &str,
) -> String {
    let Proposal::Work {
        title,
        fields,
        unknown_fields,
        versions,
        score,
        notes,
        scenes,
        ..
    } = proposal
    else {
        return String::new();
    };
    let vocabulary = config.vocabulary(kind);
    let mut out = String::new();

    if let Some(title) = title {
        let kind_label = config.kind(kind).map_or(kind, |k| k.label.as_str());
        out.push_str(&format!("## {title} · {kind_label}\n\n"));
    }

    for packaged in versions {
        let role = vocabulary
            .version_roles
            .iter()
            .find(|r| r.key == packaged.role);
        let heading = role.map_or(packaged.role.as_str(), |r| r.label.as_str());
        match &packaged.label {
            Some(label) => out.push_str(&format!("### {heading} — {label}\n\n")),
            None => out.push_str(&format!("### {heading}\n\n")),
        }
        if role.is_some_and(|r| r.reads_as_markdown()) {
            out.push_str(packaged.body.trim_end());
            out.push_str("\n\n");
        } else {
            let fence = "`".repeat(longest_backtick_run(&packaged.body).max(2) + 1);
            out.push_str(&format!(
                "{fence}\n{}\n{fence}\n\n",
                packaged.body.trim_end()
            ));
        }
    }

    if !fields.is_empty() {
        out.push_str("### Fields\n\n");
        for (key, value) in fields {
            let label = config
                .work_meta_fields
                .iter()
                .find(|f| f.key == *key)
                .map_or(key.as_str(), |f| f.label.as_str());
            let shown = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            if shown.contains('\n') {
                out.push_str(&format!("**{label}**\n\n{}\n\n", shown.trim_end()));
            } else {
                out.push_str(&format!("- **{label}:** {shown}\n"));
            }
        }
        out.push('\n');
    }
    if !unknown_fields.is_empty() {
        out.push_str(&format!(
            "_Not fields of this profile, left out: {}._\n\n",
            unknown_fields.join(", ")
        ));
    }

    if let Some(marks) = score {
        out.push_str("### Score\n\n");
        for (key, value) in &marks.axes {
            let axis = vocabulary.axes.iter().find(|a| a.key == *key);
            let label = axis.map_or(key.as_str(), |a| a.label.as_str());
            match axis {
                Some(axis) => out.push_str(&format!("- {label}: {value}/{}\n", axis.scale)),
                None => out.push_str(&format!("- {label}: {value}\n")),
            }
        }
        if let Some(note) = &marks.note {
            out.push_str(&format!("\n{note}\n"));
        }
        out.push('\n');
    }

    if !notes.is_empty() {
        out.push_str("### Notes\n\n");
        for packaged in notes {
            match &packaged.title {
                Some(t) => out.push_str(&format!("**{t}**\n\n{}\n\n", packaged.body.trim_end())),
                None => out.push_str(&format!("{}\n\n", packaged.body.trim_end())),
            }
        }
    }

    if !scenes.is_empty() {
        out.push_str("### Scenes\n\n");
        out.push_str(&render_board(scenes, vocabulary, BoardChange::Add));
        out.push_str("\n\n");
    }

    out.trim_end().to_owned()
}

/// The message body a proposed storyboard is shown with: the board as a
/// table — number, section, seconds, kind of shot, description — and under
/// it each scene's prompt blocks in fences, readable before a row is
/// written. Numbers are the scenes' own when given, else their order, the
/// same rule the application follows on a replaced board; an added scene
/// without a number is shown as `+`, since where it lands is after the last
/// at the moment of applying.
pub fn render_board(
    scenes: &[PackagedScene],
    vocabulary: &profile::config::WorkKind,
    change: BoardChange,
) -> String {
    let mut out = String::new();
    let has_shots = !vocabulary.shot_types.is_empty();
    let shot_label = |key: &str| {
        vocabulary
            .shot_types
            .iter()
            .find(|s| s.key == key)
            .map_or(key.to_owned(), |s| s.label.clone())
    };
    let number = |index: usize, scene: &PackagedScene| match scene.position {
        Some(position) => position.to_string(),
        None if change == BoardChange::Replace => (index + 1).to_string(),
        None => "+".to_owned(),
    };

    out.push_str(if has_shots {
        "| # | Section | Time | Shot | Description |\n| --- | --- | --- | --- | --- |\n"
    } else {
        "| # | Section | Time | Description |\n| --- | --- | --- | --- |\n"
    });
    for (index, scene) in scenes.iter().enumerate() {
        let time = match (scene.starts_at, scene.ends_at) {
            (Some(from), Some(to)) => format!("{}–{}", timecode(from), timecode(to)),
            (Some(from), None) => format!("{}–", timecode(from)),
            (None, Some(to)) => format!("–{}", timecode(to)),
            (None, None) => String::new(),
        };
        let cell = |text: &str| text.replace('|', "\\|").replace('\n', " ");
        let mut row = vec![
            number(index, scene),
            cell(scene.section.as_deref().unwrap_or("")),
            time,
        ];
        if has_shots {
            row.push(cell(
                &scene.shot_type.as_deref().map_or(String::new(), shot_label),
            ));
        }
        row.push(cell(&scene.description));
        out.push_str(&format!("| {} |\n", row.join(" | ")));
    }

    for (index, scene) in scenes.iter().enumerate() {
        let blocks: Vec<_> = vocabulary
            .scene_blocks
            .iter()
            .filter_map(|block| {
                scene
                    .blocks
                    .get(&block.key)
                    .and_then(Value::as_str)
                    .map(|text| (block.label.as_str(), text))
            })
            .collect();
        if blocks.is_empty() {
            continue;
        }
        out.push_str(&format!("\n**Scene {}**\n", number(index, scene)));
        for (label, text) in blocks {
            let fence = "`".repeat(longest_backtick_run(text).max(2) + 1);
            out.push_str(&format!(
                "\n_{label}_\n\n{fence}\n{}\n{fence}\n",
                text.trim_end()
            ));
        }
    }

    out.trim_end().to_owned()
}

use crate::scene::timecode;

fn longest_backtick_run(text: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    for ch in text.chars() {
        if ch == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    longest
}

/// The message meta a proposal from outside the window is stored with.
pub fn proposal_meta(
    client: &str,
    proposal: &Proposal,
    note: Option<&str>,
) -> Result<Map<String, Value>> {
    let mut meta = Map::new();
    meta.insert("source".into(), json!("mcp"));
    meta.insert("client".into(), json!(client));
    meta.insert("proposal".into(), serde_json::to_value(proposal)?);
    if let Some(note) = note {
        meta.insert("note".into(), json!(note));
    }
    Ok(meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::proposal::PackagedVersion;
    use crate::db;

    fn workspace() -> (Connection, String, String) {
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
            NewVersion {
                role: "lyrics".into(),
                body: "one line".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
        (conn, profile_id, work_id)
    }

    fn chat_on(conn: &Connection, profile_id: &str, work_id: Option<&str>) -> String {
        assistant::create(
            conn,
            profile_id,
            assistant::NewChat {
                work_id: work_id.map(str::to_owned),
                title: Some("Claude Code".into()),
                ..assistant::NewChat::default()
            },
        )
        .unwrap()
        .id
    }

    fn propose(conn: &Connection, chat_id: &str, body: &str, proposal: Proposal) -> String {
        let meta = proposal_meta("Claude Code", &proposal, None).unwrap();
        assistant::append(conn, chat_id, ASSISTANT, body, meta)
            .unwrap()
            .id
    }

    fn is_current(conn: &Connection, work_id: &str, version_id: &str) -> bool {
        version::list(conn, work_id)
            .unwrap()
            .iter()
            .any(|v| v.id == version_id && v.is_current)
    }

    fn operation_kinds(conn: &Connection) -> Vec<String> {
        operation::all(conn)
            .unwrap()
            .into_iter()
            .map(|op| op.kind)
            .collect()
    }

    fn package() -> Proposal {
        let mut fields = Map::new();
        fields.insert(
            "premise".into(),
            json!("a lighthouse keeper who never leaves"),
        );
        let mut axes = Map::new();
        for axis in ["imagery", "hook"] {
            axes.insert(axis.into(), json!(7.0));
        }
        Proposal::Work {
            title: Some("Winter road".into()),
            work_kind: Some("song".into()),
            fields,
            unknown_fields: Vec::new(),
            versions: vec![
                PackagedVersion {
                    role: "lyrics".into(),
                    body: "snow on the road\nnobody home".into(),
                    label: Some("first pass".into()),
                },
                PackagedVersion {
                    role: "style".into(),
                    body: "slow, brushed drums".into(),
                    label: None,
                },
            ],
            score: Some(Marks {
                axes,
                note: Some("a quiet one".into()),
                unknown: Vec::new(),
                missing: Vec::new(),
            }),
            notes: vec![PackagedNote {
                title: Some("Concept".into()),
                body: "the road as the only witness".into(),
            }],
            scenes: Vec::new(),
        }
    }

    /// A video in the profile — a kind with a storyboard — with `boarded`
    /// scenes already on it, numbered from 1.
    fn video(conn: &Connection, profile_id: &str, boarded: usize) -> (String, Vec<String>) {
        let work_id = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "video".into(),
                title: "The clip".into(),
                ..NewWork::default()
            },
        )
        .unwrap()
        .id;
        let ids = (0..boarded)
            .map(|index| {
                scene::create(
                    conn,
                    profile_id,
                    NewScene {
                        work_id: work_id.clone(),
                        description: Some(format!("old scene {}", index + 1)),
                        ..NewScene::default()
                    },
                )
                .unwrap()
                .id
            })
            .collect();
        (work_id, ids)
    }

    fn packaged(description: &str) -> PackagedScene {
        let mut blocks = Map::new();
        blocks.insert("still".into(), json!(format!("{description}, 35mm")));
        PackagedScene {
            position: None,
            section: Some("chorus".into()),
            starts_at: Some(12.0),
            ends_at: Some(16.5),
            shot_type: Some("wide".into()),
            description: description.into(),
            blocks,
        }
    }

    #[test]
    fn added_scenes_land_after_the_last_through_the_log_and_mark_the_message() {
        let (mut conn, profile_id, _) = workspace();
        let (video_id, _) = video(&conn, &profile_id, 1);
        let chat = chat_on(&conn, &profile_id, Some(&video_id));
        let message = propose(
            &conn,
            &chat,
            "the board",
            Proposal::Scenes {
                scenes: vec![packaged("new two"), packaged("new three")],
                change: BoardChange::Add,
            },
        );

        let outcome = apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap();

        assert_eq!(outcome.scenes.len(), 2);
        assert!(outcome.removed_scenes.is_empty());
        let board = scene::for_work(&conn, &video_id).unwrap();
        assert_eq!(
            board
                .iter()
                .map(|s| (s.position, s.description.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "old scene 1"), (2, "new two"), (3, "new three")]
        );
        assert_eq!(board[1].shot_type.as_deref(), Some("wide"));
        assert_eq!(board[1].blocks["still"], "new two, 35mm");
        assert_eq!(board[1].ends_at, Some(16.5));
        assert_eq!(
            operation_kinds(&conn),
            vec!["scene.create", "scene.create"],
            "the same operations a hand writes; the seeded scene went in without the log"
        );
        let marked = assistant::message(&conn, &message).unwrap().unwrap();
        assert_eq!(
            marked.meta["applied"]["scenes"].as_array().unwrap().len(),
            2
        );
        assert!(
            apply(&mut conn, &profile_id, &message, Overrides::default()).is_err(),
            "not twice"
        );
    }

    #[test]
    fn a_replaced_board_rewrites_by_number_and_sends_the_rest_to_the_trash() {
        let (mut conn, profile_id, _) = workspace();
        let (video_id, old) = video(&conn, &profile_id, 3);
        let chat = chat_on(&conn, &profile_id, Some(&video_id));
        let message = propose(
            &conn,
            &chat,
            "the board",
            Proposal::Scenes {
                scenes: vec![packaged("rewritten one"), packaged("rewritten two")],
                change: BoardChange::Replace,
            },
        );

        let outcome = apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap();

        assert_eq!(
            outcome.scenes,
            vec![old[0].clone(), old[1].clone()],
            "a scene with the same number keeps its id"
        );
        assert_eq!(outcome.removed_scenes.len(), 1);
        let board = scene::for_work(&conn, &video_id).unwrap();
        assert_eq!(
            board
                .iter()
                .map(|s| (s.position, s.description.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "rewritten one"), (2, "rewritten two")]
        );
        assert_eq!(board[0].section.as_deref(), Some("chorus"));
        assert_eq!(board[0].blocks["still"], "rewritten one, 35mm");
        assert_eq!(
            operation_kinds(&conn),
            vec!["scene.update", "scene.update", "entity.discard"]
        );
        let trashed = trash::list(&conn, &profile_id).unwrap();
        assert_eq!(trashed.len(), 1);
        assert_eq!(trashed[0].id, outcome.removed_scenes[0]);
        assert!(
            trashed[0].label.contains('3'),
            "the third scene is what went: {}",
            trashed[0].label
        );
    }

    #[test]
    fn a_replaced_board_grows_where_the_old_one_was_shorter() {
        let (mut conn, profile_id, _) = workspace();
        let (video_id, old) = video(&conn, &profile_id, 1);
        let chat = chat_on(&conn, &profile_id, Some(&video_id));
        let message = propose(
            &conn,
            &chat,
            "the board",
            Proposal::Scenes {
                scenes: vec![packaged("one"), packaged("two")],
                change: BoardChange::Replace,
            },
        );

        let outcome = apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap();

        assert_eq!(outcome.scenes[0], old[0]);
        assert_eq!(outcome.scenes.len(), 2);
        assert_eq!(operation_kinds(&conn), vec!["scene.update", "scene.create"]);
        assert_eq!(scene::count(&conn, &video_id).unwrap(), 2);
    }

    #[test]
    fn a_storyboard_for_a_kind_without_one_is_refused_before_anything_is_written() {
        let (mut conn, profile_id, song_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&song_id));
        let message = propose(
            &conn,
            &chat,
            "the board",
            Proposal::Scenes {
                scenes: vec![packaged("one")],
                change: BoardChange::Replace,
            },
        );

        let err = apply(&mut conn, &profile_id, &message, Overrides::default())
            .unwrap_err()
            .to_string();

        assert!(err.contains("no storyboard"), "{err}");
        assert!(operation_kinds(&conn).is_empty());
        assert!(is_pending(
            &assistant::message(&conn, &message).unwrap().unwrap()
        ));
    }

    #[test]
    fn a_package_on_a_new_video_creates_its_board_with_it() {
        let (mut conn, profile_id, _) = workspace();
        let chat = chat_on(&conn, &profile_id, None);
        let message = propose(
            &conn,
            &chat,
            "rendered",
            Proposal::Work {
                title: Some("Winter road — the clip".into()),
                work_kind: Some("video".into()),
                fields: Map::new(),
                unknown_fields: Vec::new(),
                versions: Vec::new(),
                score: None,
                notes: Vec::new(),
                scenes: vec![packaged("the road"), packaged("the car")],
            },
        );

        let outcome = apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap();

        assert!(outcome.created_work);
        assert_eq!(outcome.scenes.len(), 2);
        let board = scene::for_work(&conn, outcome.work_id.as_deref().unwrap()).unwrap();
        assert_eq!(
            board.iter().map(|s| s.position).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(
            operation_kinds(&conn),
            vec!["work.create", "scene.create", "scene.create"]
        );
    }

    #[test]
    fn a_rendered_board_is_a_table_with_the_blocks_under_it() {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let config = profile::active(&conn).unwrap().unwrap().config;
        let vocabulary = config.vocabulary("video");
        let mut second = packaged("hands on a rope");
        second.position = Some(7);
        second.starts_at = None;
        second.ends_at = None;
        second.blocks = Map::new();

        let added = render_board(
            &[packaged("the harbour"), second.clone()],
            vocabulary,
            BoardChange::Add,
        );
        assert!(
            added.contains("| + | chorus | 0:12–0:16.5 | Wide | the harbour |"),
            "{added}"
        );
        assert!(
            added.contains("| 7 | chorus |  | Wide | hands on a rope |"),
            "{added}"
        );
        assert!(
            added.contains("**Scene +**\n\n_Still frame_\n\n```\nthe harbour, 35mm\n```"),
            "{added}"
        );
        assert!(
            !added.contains("**Scene 7**"),
            "no blocks, no section: {added}"
        );

        let replaced = render_board(&[packaged("the harbour")], vocabulary, BoardChange::Replace);
        assert!(
            replaced.contains("| 1 | chorus |"),
            "a replaced board numbers in order: {replaced}"
        );
    }

    #[test]
    fn a_version_proposal_lands_as_a_version_that_is_not_current_and_marks_the_message() {
        let (mut conn, profile_id, work_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let message = propose(
            &conn,
            &chat,
            "one line\ntwo lines",
            Proposal::Version {
                role: "lyrics".into(),
                label: Some("longer".into()),
            },
        );

        let outcome = apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap();

        assert_eq!(outcome.versions.len(), 1);
        let created = version::get(&conn, &outcome.versions[0]).unwrap().unwrap();
        assert_eq!(created.body, "one line\ntwo lines");
        assert_eq!(created.label.as_deref(), Some("longer"));
        assert!(
            !is_current(&conn, &work_id, &created.id),
            "a proposal applied is not yet stood behind"
        );

        let stored = assistant::message(&conn, &message).unwrap().unwrap();
        assert_eq!(stored.meta["applied"]["versions"][0], created.id);
        assert!(!is_pending(&stored));

        let again = apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap_err();
        assert!(again.to_string().contains("already applied"), "{again}");
        assert_eq!(
            version::list(&conn, &work_id).unwrap().len(),
            2,
            "applied once"
        );
    }

    #[test]
    fn the_dialogs_choices_override_the_proposal() {
        let (mut conn, profile_id, work_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let message = propose(
            &conn,
            &chat,
            "brushed drums",
            Proposal::Version {
                role: "lyrics".into(),
                label: None,
            },
        );

        let outcome = apply(
            &mut conn,
            &profile_id,
            &message,
            Overrides {
                role: Some("style".into()),
                label: Some("from the chat".into()),
                make_current: Some(true),
            },
        )
        .unwrap();

        let created = version::get(&conn, &outcome.versions[0]).unwrap().unwrap();
        assert_eq!(created.role, "style");
        assert_eq!(created.label.as_deref(), Some("from the chat"));
        assert!(is_current(&conn, &work_id, &created.id));
    }

    #[test]
    fn a_role_the_kind_does_not_have_is_refused_before_anything_is_written() {
        let (mut conn, profile_id, work_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let message = propose(
            &conn,
            &chat,
            "text",
            Proposal::Version {
                role: "storyboard".into(),
                label: None,
            },
        );
        let before = operation_kinds(&conn).len();

        let err = apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap_err();

        assert!(err.to_string().contains("storyboard"), "{err}");
        assert_eq!(operation_kinds(&conn).len(), before, "nothing was written");
        assert!(is_pending(
            &assistant::message(&conn, &message).unwrap().unwrap()
        ));
    }

    #[test]
    fn a_score_from_an_agent_is_judged_by_the_agent() {
        let (mut conn, profile_id, work_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let mut axes = Map::new();
        axes.insert("hook".into(), json!(8.0));
        let message = propose(
            &conn,
            &chat,
            "strong chorus",
            Proposal::Score {
                axes,
                note: Some("strong chorus".into()),
                unknown: Vec::new(),
                missing: Vec::new(),
            },
        );

        let outcome = apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap();

        let history = score::history(&conn, &work_id).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, outcome.score.unwrap());
        assert_eq!(history[0].rater.as_deref(), Some("Claude Code"));
        assert_eq!(history[0].note.as_deref(), Some("strong chorus"));
    }

    #[test]
    fn a_package_in_a_chat_on_nothing_creates_the_whole_work_through_the_log() {
        let (mut conn, profile_id, _) = workspace();
        let chat = chat_on(&conn, &profile_id, None);
        let message = propose(&conn, &chat, "rendered", package());
        let before = operation_kinds(&conn);

        let outcome = apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap();

        assert!(outcome.created_work);
        let work_id = outcome.work_id.clone().unwrap();
        let created = work::get(&conn, &work_id).unwrap().unwrap();
        assert_eq!(created.title, "Winter road");
        assert_eq!(created.kind, "song");
        assert_eq!(
            created.meta["premise"], "a lighthouse keeper who never leaves",
            "fields land in the overview"
        );
        assert_eq!(outcome.fields, vec!["premise".to_owned()]);

        let versions = version::list(&conn, &work_id).unwrap();
        assert_eq!(versions.len(), 2);
        let lyrics = versions.iter().find(|v| v.role == "lyrics").unwrap();
        assert_eq!(lyrics.label.as_deref(), Some("first pass"));
        assert!(
            lyrics.is_current,
            "the version in the vocabulary's first role is the current one: {versions:?}"
        );
        assert!(
            !versions
                .iter()
                .find(|v| v.role == "style")
                .unwrap()
                .is_current,
            "one current version per work"
        );

        let history = score::history(&conn, &work_id).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].rater.as_deref(), Some("Claude Code"));
        assert_eq!(
            history[0].version_id.as_deref(),
            Some(lyrics.id.as_str()),
            "the score judges the version the package brought"
        );

        let notes = note::list(
            &conn,
            &profile_id,
            &note::NoteFilter {
                work_id: Some(work_id.clone()),
                ..note::NoteFilter::default()
            },
        )
        .unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title.as_deref(), Some("Concept"));

        let written: Vec<String> = operation_kinds(&conn)[before.len()..].to_vec();
        assert_eq!(
            written,
            vec![
                "work.create",
                "version.create",
                "version.create",
                "score.create",
                "note.create"
            ],
            "every write is an operation a replay rebuilds"
        );

        let stored = assistant::message(&conn, &message).unwrap().unwrap();
        assert_eq!(stored.meta["applied"]["work_id"], work_id);
        assert_eq!(stored.meta["applied"]["created_work"], true);
    }

    #[test]
    fn a_package_in_a_chat_on_nothing_needs_a_title_and_a_kind() {
        let (mut conn, profile_id, _) = workspace();
        let chat = chat_on(&conn, &profile_id, None);
        let Proposal::Work { versions, .. } = package() else {
            unreachable!()
        };
        let message = propose(
            &conn,
            &chat,
            "rendered",
            Proposal::Work {
                title: None,
                work_kind: Some("song".into()),
                fields: Map::new(),
                unknown_fields: Vec::new(),
                versions,
                score: None,
                notes: Vec::new(),
                scenes: Vec::new(),
            },
        );

        let err = apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap_err();

        assert!(err.to_string().contains("`title`"), "{err}");
    }

    #[test]
    fn a_package_on_a_work_adds_to_it_and_keeps_the_fields_it_did_not_name() {
        let (mut conn, profile_id, work_id) = workspace();
        let mut meta = Map::new();
        meta.insert("bpm".into(), json!(92));
        meta.insert("premise".into(), json!("old premise"));
        work::update(
            &conn,
            &work_id,
            WorkPatch {
                meta: Some(meta),
                ..WorkPatch::default()
            },
        )
        .unwrap();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let Proposal::Work {
            fields,
            versions,
            score,
            notes,
            ..
        } = package()
        else {
            unreachable!()
        };
        let message = propose(
            &conn,
            &chat,
            "rendered",
            Proposal::Work {
                title: None,
                work_kind: None,
                fields,
                unknown_fields: Vec::new(),
                versions,
                score,
                notes,
                scenes: Vec::new(),
            },
        );

        let outcome = apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap();

        assert!(!outcome.created_work);
        assert_eq!(outcome.work_id.as_deref(), Some(work_id.as_str()));
        let updated = work::get(&conn, &work_id).unwrap().unwrap();
        assert_eq!(
            updated.meta["bpm"], 92,
            "a field the package did not name stays"
        );
        assert_eq!(
            updated.meta["premise"],
            "a lighthouse keeper who never leaves"
        );

        let versions = version::list(&conn, &work_id).unwrap();
        assert_eq!(versions.len(), 3);
        let current: Vec<_> = versions.iter().filter(|v| v.is_current).collect();
        assert_eq!(
            current.len(),
            1,
            "the package's versions wait beside the current one"
        );
        let body = version::get(&conn, &current[0].id).unwrap().unwrap().body;
        assert_eq!(body, "one line");
    }

    #[test]
    fn apply_pending_takes_every_unapplied_proposal_in_order_and_skips_the_applied() {
        let (mut conn, profile_id, work_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let first = propose(
            &conn,
            &chat,
            "first",
            Proposal::Version {
                role: "lyrics".into(),
                label: None,
            },
        );
        apply(&mut conn, &profile_id, &first, Overrides::default()).unwrap();
        propose(
            &conn,
            &chat,
            "second",
            Proposal::Version {
                role: "lyrics".into(),
                label: None,
            },
        );
        propose(&conn, &chat, "a thought", Proposal::Note { title: None });
        // A plain answer with no proposal is not something to apply.
        assistant::append(&conn, &chat, ASSISTANT, "just prose", Map::new()).unwrap();

        let outcomes = apply_pending(&mut conn, &profile_id, &chat).unwrap();

        assert_eq!(outcomes.len(), 2);
        assert_eq!(version::list(&conn, &work_id).unwrap().len(), 3);
        let transcript = assistant::transcript(&conn, &chat).unwrap().unwrap();
        assert!(transcript.messages.iter().all(|m| !is_pending(m)));

        assert!(
            apply_pending(&mut conn, &profile_id, &chat)
                .unwrap()
                .is_empty(),
            "nothing left to apply"
        );
    }

    #[test]
    fn apply_pending_stops_at_the_first_failure_and_says_so() {
        let (mut conn, profile_id, work_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        propose(
            &conn,
            &chat,
            "fine",
            Proposal::Version {
                role: "lyrics".into(),
                label: None,
            },
        );
        propose(
            &conn,
            &chat,
            "broken",
            Proposal::Version {
                role: "storyboard".into(),
                label: None,
            },
        );

        let err = apply_pending(&mut conn, &profile_id, &chat).unwrap_err();

        assert!(err.to_string().starts_with("1 applied"), "{err}");
        assert_eq!(version::list(&conn, &work_id).unwrap().len(), 2);
    }

    #[test]
    fn a_rendered_package_shows_every_part_and_fences_plain_text() {
        let (conn, profile_id, _) = workspace();
        let config = profile::config_for(&conn, &profile_id).unwrap();

        let body = render_package(&package(), &config, "song");

        assert!(body.starts_with("## Winter road · Song"), "{body}");
        assert!(
            body.contains("### Lyrics — first pass\n\n```\nsnow on the road\nnobody home\n```"),
            "{body}"
        );
        assert!(
            body.contains("- **Premise:** a lighthouse keeper"),
            "fields are named by label: {body}"
        );
        assert!(
            body.contains("- Hook: 7/10") || body.contains("- Hook: 7.0/10"),
            "{body}"
        );
        assert!(
            body.contains("**Concept**\n\nthe road as the only witness"),
            "{body}"
        );
    }

    #[test]
    fn a_fence_outlasts_the_backticks_in_the_text() {
        let (conn, profile_id, _) = workspace();
        let config = profile::config_for(&conn, &profile_id).unwrap();
        let proposal = Proposal::Work {
            title: None,
            work_kind: None,
            fields: Map::new(),
            unknown_fields: Vec::new(),
            versions: vec![PackagedVersion {
                role: "lyrics".into(),
                body: "a line with ``` in it".into(),
                label: None,
            }],
            score: None,
            notes: Vec::new(),
            scenes: Vec::new(),
        };

        let body = render_package(&proposal, &config, "song");

        assert!(body.contains("````\na line with ``` in it\n````"), "{body}");
    }

    #[test]
    fn a_revised_scene_changes_only_what_it_names_and_the_rest_of_the_board_stands() {
        let (mut conn, profile_id, _) = workspace();
        let (video_id, old) = video(&conn, &profile_id, 3);
        let chat = chat_on(&conn, &profile_id, Some(&video_id));
        let mut blocks = Map::new();
        blocks.insert("still".into(), json!("a new still"));
        let revised = PackagedScene {
            position: Some(2),
            section: None,
            starts_at: None,
            ends_at: None,
            shot_type: Some("close".into()),
            description: String::new(),
            blocks,
        };
        let mut added = packaged("a fourth");
        added.position = Some(4);
        let message = propose(
            &conn,
            &chat,
            "the blocks",
            Proposal::Scenes {
                scenes: vec![revised, added],
                change: BoardChange::Revise,
            },
        );

        let outcome = apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap();

        assert_eq!(outcome.scenes[0], old[1], "the revised scene keeps its id");
        assert!(
            outcome.removed_scenes.is_empty(),
            "a revision sends nothing to the trash"
        );
        let board = scene::for_work(&conn, &video_id).unwrap();
        assert_eq!(
            board
                .iter()
                .map(|s| (s.position, s.description.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (1, "old scene 1"),
                (2, "old scene 2"),
                (3, "old scene 3"),
                (4, "a fourth")
            ],
            "a blank description is kept as it was; a number nobody held is new"
        );
        assert_eq!(board[1].blocks["still"], "a new still");
        assert_eq!(board[1].shot_type.as_deref(), Some("close"));
        assert_eq!(operation_kinds(&conn), vec!["scene.update", "scene.create"]);

        let unnumbered = propose(
            &conn,
            &chat,
            "no number",
            Proposal::Scenes {
                scenes: vec![packaged("x")],
                change: BoardChange::Revise,
            },
        );
        let refused = apply(&mut conn, &profile_id, &unnumbered, Overrides::default()).unwrap_err();
        assert!(
            refused.to_string().contains("names its number"),
            "{refused}"
        );
    }

    /// A revision that carries one block leaves the others where they are.
    ///
    /// This is what makes a task about a single block possible at all: the
    /// answer comes back holding only the block it was asked for, and the
    /// scene's other prompts are not its to delete. An emptied block is
    /// cleared, which is what clearing the box on the board means.
    #[test]
    fn a_revised_block_is_laid_over_the_others_not_put_in_their_place() {
        let (mut conn, profile_id, _) = workspace();
        let (video_id, old) = video(&conn, &profile_id, 2);

        let mut standing = Map::new();
        standing.insert("still".into(), json!("the still as it was"));
        standing.insert("motion".into(), json!("the motion as it was"));
        standing.insert("negative".into(), json!("no hands"));
        scene::update(
            &conn,
            &old[1],
            scene::ScenePatch {
                blocks: Some(standing),
                ..scene::ScenePatch::default()
            },
        )
        .unwrap();

        let chat = chat_on(&conn, &profile_id, Some(&video_id));
        let mut only_motion = Map::new();
        only_motion.insert("motion".into(), json!("a slower push in"));
        let mut revised = packaged("");
        revised.position = Some(2);
        revised.blocks = only_motion;
        let message = propose(
            &conn,
            &chat,
            "just the animation",
            Proposal::Scenes {
                scenes: vec![revised],
                change: BoardChange::Revise,
            },
        );

        apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap();

        let board = scene::for_work(&conn, &video_id).unwrap();
        assert_eq!(board[1].blocks["motion"], "a slower push in");
        assert_eq!(
            board[1].blocks["still"], "the still as it was",
            "a block nobody asked about was deleted"
        );
        assert_eq!(board[1].blocks["negative"], "no hands");

        // And an emptied block is cleared rather than kept.
        let mut emptied = Map::new();
        emptied.insert("negative".into(), json!("   "));
        let mut clearing = packaged("");
        clearing.position = Some(2);
        clearing.blocks = emptied;
        let message = propose(
            &conn,
            &chat,
            "drop the negative",
            Proposal::Scenes {
                scenes: vec![clearing],
                change: BoardChange::Revise,
            },
        );
        apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap();

        let board = scene::for_work(&conn, &video_id).unwrap();
        assert!(
            !board[1].blocks.contains_key("negative"),
            "an emptied block is cleared"
        );
        assert_eq!(
            board[1].blocks["motion"], "a slower push in",
            "and the rest stands"
        );
    }

    /// A timed board survives a revision of what its scenes say.
    ///
    /// The board is timed once and dragged by hand from there (0.65); a
    /// revision is about the words, and the seconds are the person's. The
    /// instruction tells the model to leave out what it does not change, but
    /// the shape it is given names `starts_at` and `ends_at`, so nothing but
    /// this holds the line: a revision that mentions no seconds must not
    /// move a single one.
    #[test]
    fn revising_a_scene_leaves_the_seconds_the_person_set() {
        let (mut conn, profile_id, _) = workspace();
        let (video_id, old) = video(&conn, &profile_id, 3);

        // Timed the way the board times itself.
        for (index, id) in old.iter().enumerate() {
            let from = index as f64 * 30.0;
            scene::update(
                &conn,
                id,
                scene::ScenePatch {
                    starts_at: Some(Some(from)),
                    ends_at: Some(Some(from + 30.0)),
                    ..scene::ScenePatch::default()
                },
            )
            .unwrap();
        }

        let chat = chat_on(&conn, &profile_id, Some(&video_id));
        let mut revised = packaged("a better second scene");
        revised.position = Some(2);
        revised.starts_at = None;
        revised.ends_at = None;
        let message = propose(
            &conn,
            &chat,
            "the words, not the clock",
            Proposal::Scenes {
                scenes: vec![revised],
                change: BoardChange::Revise,
            },
        );

        apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap();

        let board = scene::for_work(&conn, &video_id).unwrap();
        assert_eq!(
            board
                .iter()
                .map(|s| (s.position, s.starts_at, s.ends_at))
                .collect::<Vec<_>>(),
            vec![
                (1, Some(0.0), Some(30.0)),
                (2, Some(30.0), Some(60.0)),
                (3, Some(60.0), Some(90.0)),
            ],
            "a revision of the words moved the clock"
        );
        assert_eq!(board[1].description, "a better second scene");
        assert_eq!(board[1].position, 2, "and the number is the person's too");
    }

    #[test]
    fn a_storyboard_stored_by_v0_62_with_the_replace_flag_still_replaces() {
        let (mut conn, profile_id, _) = workspace();
        let (video_id, old) = video(&conn, &profile_id, 2);
        let chat = chat_on(&conn, &profile_id, Some(&video_id));
        let mut meta = Map::new();
        meta.insert(
            "proposal".into(),
            json!({
                "kind": "scenes",
                "replace": true,
                "scenes": [{ "description": "the only scene" }]
            }),
        );
        let message = assistant::append(&conn, &chat, ASSISTANT, "the board", meta)
            .unwrap()
            .id;

        let outcome = apply(&mut conn, &profile_id, &message, Overrides::default()).unwrap();

        assert_eq!(outcome.scenes, vec![old[0].clone()], "rewritten in place");
        assert_eq!(
            outcome.removed_scenes.len(),
            1,
            "the rest went to the trash"
        );
    }
}

#[cfg(test)]
mod bound_to_a_version_tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::work::version::{self, NewVersion};
    use crate::work::{self, NewWork};

    fn two_revisions() -> (Connection, String, String, String) {
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
        let mut make = |body: &str| {
            version::create(
                &mut conn,
                &work_id,
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
        let first = make("one");
        make("two");
        (conn, profile_id, work_id, first)
    }

    fn chat_about(conn: &Connection, profile_id: &str, work_id: &str, version_id: &str) -> String {
        assistant::create(
            conn,
            profile_id,
            assistant::NewChat {
                work_id: Some(work_id.to_owned()),
                title: Some("Critique".into()),
                action: Some("critique".into()),
                version_id: Some(version_id.to_owned()),
            },
        )
        .unwrap()
        .id
    }

    /// A score applied from a chat about revision 1 is a snapshot of
    /// revision 1, although revision 2 is current.
    #[test]
    fn a_score_binds_to_the_version_the_chat_is_about() {
        let (mut conn, profile_id, work_id, first) = two_revisions();
        let chat_id = chat_about(&conn, &profile_id, &work_id, &first);
        let config = profile::active(&conn).unwrap().unwrap().config;
        let axis = config.vocabulary("song").axes[0].key.clone();
        let mut axes = Map::new();
        axes.insert(axis, serde_json::json!(7));
        let meta = proposal_meta(
            "Claude Code",
            &Proposal::Score {
                axes,
                note: None,
                unknown: Vec::new(),
                missing: Vec::new(),
            },
            None,
        )
        .unwrap();
        let message_id = assistant::append(&conn, &chat_id, ASSISTANT, "judged", meta)
            .unwrap()
            .id;
        let outcome = apply(&mut conn, &profile_id, &message_id, Overrides::default()).unwrap();

        let score = crate::score::get(&conn, &outcome.score.unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(score.version_id.as_deref(), Some(first.as_str()));
    }

    /// A version applied from a chat about revision 1 says it is about
    /// revision 1, and the summary reads it back.
    #[test]
    fn a_commentary_says_which_version_it_is_about() {
        let (mut conn, profile_id, work_id, first) = two_revisions();
        let chat_id = chat_about(&conn, &profile_id, &work_id, &first);
        let meta = proposal_meta(
            "Claude Code",
            &Proposal::Version {
                role: "critique".into(),
                label: None,
            },
            None,
        )
        .unwrap();
        let message_id = assistant::append(&conn, &chat_id, ASSISTANT, "weak second line", meta)
            .unwrap()
            .id;
        let outcome = apply(&mut conn, &profile_id, &message_id, Overrides::default()).unwrap();

        let id = &outcome.versions[0];
        let summary = version::list(&conn, &work_id)
            .unwrap()
            .into_iter()
            .find(|v| &v.id == id)
            .unwrap();
        assert_eq!(summary.about_version_id.as_deref(), Some(first.as_str()));
        assert_eq!(summary.role, "critique");
    }

    /// The version the chat was about is gone: the proposal still applies,
    /// bound to nothing rather than refused.
    #[test]
    fn a_deleted_version_binds_to_nothing() {
        let (mut conn, profile_id, work_id, first) = two_revisions();
        let chat_id = chat_about(&conn, &profile_id, &work_id, &first);
        conn.execute("DELETE FROM work_version WHERE id = ?1", [&first])
            .unwrap();
        let config = profile::active(&conn).unwrap().unwrap().config;
        let axis = config.vocabulary("song").axes[0].key.clone();
        let mut axes = Map::new();
        axes.insert(axis, serde_json::json!(7));
        let meta = proposal_meta(
            "Claude Code",
            &Proposal::Score {
                axes,
                note: None,
                unknown: Vec::new(),
                missing: Vec::new(),
            },
            None,
        )
        .unwrap();
        let message_id = assistant::append(&conn, &chat_id, ASSISTANT, "judged", meta)
            .unwrap()
            .id;
        let outcome = apply(&mut conn, &profile_id, &message_id, Overrides::default()).unwrap();
        let score = crate::score::get(&conn, &outcome.score.unwrap())
            .unwrap()
            .unwrap();
        assert_ne!(score.version_id.as_deref(), Some(first.as_str()));
    }
}
