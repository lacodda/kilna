//! Keeping a proposal: the one place a proposal becomes rows.
//!
//! A proposal — the panel's fenced score, an agent's version, note or whole
//! package — is a message with `meta.proposal`. Keeping it writes the rows a
//! hand would write, through the same actions a hand goes through, so a replay
//! cannot tell a proposal applied from a thing typed; and it stamps
//! `meta.applied` on the message with what it made, so the mark survives a
//! refetch and *apply all* is a loop over a chat.
//!
//! A proposal is kept whole or not at all (ADR 0040). Until v0.83 each row
//! was its own transaction, and a package that failed at its board left the
//! work and its versions behind, unmarked, so the next click made a second
//! work. Now everything - the rows, their log entries, their journal lines,
//! the mark - is one unit. And everything that can be known wrong before a
//! row is written is checked first, all of it at once: a package with a role
//! the profile lacks and a release kind the work does not ship is refused
//! with both, not with the first and then, on the next try, the second.
//!
//! Nothing here is reached by an agent. `kilna --mcp` writes the proposal;
//! the window calls this when a person clicks. See ADR 0018.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::active_profile_id;
use crate::assistant::proposal::{
    BoardChange, Marks, PackagedNote, PackagedRelease, PackagedScene, Proposal,
};
use crate::assistant::{self, Chat, apply::DISMISSED, apply::is_pending};
use crate::db::unit::atomically;
use crate::error::{Error, Reason, Result};
use crate::note::NewNote;
use crate::profile::config::ProfileConfig;
use crate::scene::{self, NewScene, ScenePatch};
use crate::score::NewScore;
use crate::trash::Entity;
use crate::work::version::{self, NewVersion};
use crate::work::{self, NewWork, WorkPatch};

/// What a person may change about a proposal on the way in: the dialog lets
/// them pick a version's role, name it and make it current, and correct a
/// comment or a reply before keeping it.
#[derive(Debug, Clone, Default, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct Overrides {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub make_current: Option<bool>,
    /// A comment read off a screenshot, as the person corrected it before
    /// keeping: the words fixed, the work chosen, the channel moved. Kept
    /// instead of what was read.
    #[serde(default)]
    pub comment: Option<crate::comment::NewComment>,
    /// A drafted reply, as the person edited it before keeping.
    #[serde(default)]
    pub reply: Option<String>,
    /// The items of a proposal the person kept: for the canon `card:0`,
    /// `fact:2`, `relation:1`; for a release the keys of the fields taken.
    /// Every item when absent. A proposal of twelve facts is read one by
    /// one, and one wrong fact must not cost the eleven right ones - nor a
    /// description the person had already written cost them the title.
    #[serde(default)]
    pub items: Option<Vec<String>>,
}

/// What applying made. Returned to the caller and stamped on the message as
/// `meta.applied`, so the chat shows the mark and the work it points at.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ts_rs::TS)]
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
    /// Releases planned by the package.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub releases: Vec<String>,
    /// The style brick an answer was written onto, when it was one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_brick: Option<String>,
    /// The comment kept from a screenshot, or whose reply was written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Cards of the canon made, or described.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cards: Vec<String>,
    /// Facts written, refined or retired.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<String>,
    /// Relations drawn or redrawn.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<String>,
    /// Pictures attached to cards, by asset id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pictures: Vec<String>,
    /// The fields of a release written, by key.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub release_fields: Vec<String>,
    /// Ideas put on a cover's board.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ideas: Vec<String>,
    /// Trials put on an experiment's board.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trials: Vec<String>,
    /// Words of the record written: made, or given a facet (ADR 0052).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terms: Vec<String>,
    /// Bricks of the dictionary made (v0.94).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub style_bricks: Vec<String>,
}

/// Keep the proposal a message carries, and mark the message - as one unit.
pub fn apply(conn: &Connection, message_id: &str, overrides: Overrides) -> Result<Outcome> {
    atomically(conn, |conn| {
        let profile_id = active_profile_id(conn)?;
        let message = assistant::message(conn, message_id)?
            .ok_or_else(|| Error::not_found("message", message_id))?;
        if message.meta.contains_key("applied") {
            return Err(Error::refused("proposal.alreadyApplied"));
        }
        if message.meta.contains_key(DISMISSED) {
            return Err(Error::refused("proposal.dismissed"));
        }
        let proposal = stored_proposal(&message.meta)?;
        // What the application already wrote of a release's proposal - the
        // fields that were empty when the answer came - is not offered twice.
        let mut overrides = overrides;
        if let Proposal::Release { fields, .. } = &proposal
            && overrides.items.is_none()
        {
            let filled = filled_of(&message.meta);
            overrides.items = Some(
                fields
                    .keys()
                    .filter(|key| !filled.contains(key))
                    .cloned()
                    .collect(),
            );
        }
        let chat = assistant::get(conn, &message.chat_id)?
            .ok_or_else(|| Error::not_found("chat", &message.chat_id))?;
        if chat.profile_id != profile_id {
            return Err(Error::refused("proposal.otherProfile"));
        }
        let config = crate::profile::config_for(conn, &profile_id)?;

        let problems = check(conn, &proposal, &chat, &config, &overrides)?;
        if !problems.is_empty() {
            let problems: Vec<Value> = problems.into_iter().map(Value::from).collect();
            return Err(Error::refused("proposal.invalid").param("problems", problems));
        }

        // Who proposed, when it was an agent outside the window: a score it
        // proposed is judged by it, not by the author, and the history says so.
        let client = message
            .meta
            .get("client")
            .and_then(Value::as_str)
            .map(str::to_owned);

        let mut outcome = Outcome {
            message_id: message_id.to_owned(),
            at: crate::time::now(),
            work_id: chat.work_id.clone(),
            ..Outcome::default()
        };
        keep(
            conn,
            &config,
            &chat,
            &message.body,
            proposal,
            overrides,
            client,
            &mut outcome,
        )?;

        // The status follows the facts, as after any hand-made version or score.
        if let Some(work_id) = &outcome.work_id {
            super::restate(conn, &profile_id, work_id);
        }

        let mut meta = message.meta;
        meta.insert("applied".into(), serde_json::to_value(&outcome)?);
        assistant::set_meta(conn, message_id, &meta)?;
        Ok(outcome)
    })
}

/// Keep every proposal in a chat nobody has answered yet, oldest first.
///
/// Each proposal is its own unit - they are separate answers, and one that
/// cannot be kept says nothing about the others before it. Stops at the first
/// that fails and says which it was, with the ones before it kept and marked:
/// the person sees where it stopped rather than a chat where nothing happened.
pub fn apply_pending(conn: &Connection, chat_id: &str) -> Result<Vec<Outcome>> {
    let transcript =
        assistant::transcript(conn, chat_id)?.ok_or_else(|| Error::not_found("chat", chat_id))?;
    let mut outcomes = Vec::new();
    for message in transcript.messages.iter().filter(|m| is_pending(m)) {
        match apply(conn, &message.id, Overrides::default()) {
            Ok(outcome) => outcomes.push(outcome),
            Err(cause) => {
                return Err(Error::refused("proposal.stoppedAfter")
                    .param("applied", outcomes.len())
                    .param("reason", cause.reason()));
            }
        }
    }
    Ok(outcomes)
}

/// The fields of a release's proposal the application already wrote when the
/// answer came, by key.
pub fn filled_of(meta: &Map<String, Value>) -> Vec<String> {
    meta.get(FILLED)
        .and_then(Value::as_array)
        .map(|keys| {
            keys.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Where a message records the fields of a release's proposal written on
/// arrival: the ones that were empty (v0.86).
pub const FILLED: &str = "filled";

/// The proposal as stored, read as it was meant.
pub fn stored_proposal(meta: &Map<String, Value>) -> Result<Proposal> {
    let Some(mut value) = meta.get("proposal").cloned() else {
        return Err(Error::refused("proposal.nothing"));
    };
    // A storyboard stored by v0.62 carried `replace: true` where v0.64
    // writes `change: "replace"`; read as it was meant, not as `add`.
    if value.get("kind").and_then(Value::as_str) == Some("scenes")
        && value.get("replace").and_then(Value::as_bool) == Some(true)
        && value.get("change").is_none()
        && let Some(object) = value.as_object_mut()
    {
        object.insert("change".into(), json!("replace"));
    }
    Ok(serde_json::from_value(value)?)
}

/// Everything that can be known wrong about a proposal before a row is
/// written, all of it. Empty when it can be kept.
///
/// What is left to find out while writing - a comment the channel rules
/// refuse, a row that vanished a moment ago - still refuses the whole
/// proposal, because the writes are one unit; this list is so that the
/// ordinary mistakes are named together rather than one per click.
pub fn check(
    conn: &Connection,
    proposal: &Proposal,
    chat: &Chat,
    config: &ProfileConfig,
    overrides: &Overrides,
) -> Result<Vec<Reason>> {
    let mut problems = Vec::new();
    let mut note = |outcome: Result<()>| {
        if let Err(cause) = outcome {
            problems.push(cause.reason());
        }
    };

    // The work the chat is about, when a proposal needs one.
    let chat_work = || -> Result<work::Work> {
        let id = chat
            .work_id
            .as_deref()
            .ok_or_else(|| Error::refused("proposal.needsWork"))?;
        work::get(conn, id)?.ok_or_else(|| Error::not_found("work", id))
    };

    match proposal {
        Proposal::Version { role, .. } => match chat_work() {
            Ok(found) => {
                let role = overrides.role.as_deref().unwrap_or(role);
                note(config.require_role(&found.kind, role));
            }
            Err(cause) => note(Err(cause)),
        },
        Proposal::Score { .. } => note(chat_work().map(|_| ())),
        Proposal::Note { .. } => {}
        Proposal::Work {
            title,
            work_kind,
            versions,
            scenes,
            releases,
            trials,
            ..
        } => {
            // A package on a work adds to it; one in a chat on nothing is a
            // new work and has to say what it is.
            let kind = match &chat.work_id {
                Some(_) => match chat_work() {
                    Ok(found) => Some(found.kind),
                    Err(cause) => {
                        note(Err(cause));
                        None
                    }
                },
                None => {
                    if title.as_deref().is_none_or(|title| title.trim().is_empty()) {
                        note(Err(Error::refused("proposal.newWorkNeedsTitle")));
                    }
                    match work_kind.as_deref().filter(|kind| !kind.trim().is_empty()) {
                        Some(kind) => Some(kind.to_owned()),
                        None => {
                            note(Err(Error::refused("proposal.newWorkNeedsKind")));
                            None
                        }
                    }
                }
            };
            if let Some(kind) = kind {
                match config.require_kind(&kind) {
                    Ok(_) => {
                        for packaged in versions {
                            note(config.require_role(&kind, &packaged.role));
                        }
                        if !scenes.is_empty() {
                            note(config.require_storyboard(&kind));
                        }
                        for packaged in releases {
                            note(config.require_release_kind(&kind, &packaged.kind));
                        }
                        if !trials.is_empty() && config.lab(&kind).is_none() {
                            note(Err(Error::refused("trial.noLabKind").param("kind", kind)));
                        }
                    }
                    Err(cause) => note(Err(cause)),
                }
            }
        }
        Proposal::Scenes { scenes, change } => match chat_work() {
            Ok(found) => {
                note(config.require_storyboard(&found.kind));
                if *change == BoardChange::Revise {
                    for (index, packaged) in scenes.iter().enumerate() {
                        if packaged.position.is_none() {
                            note(Err(Error::refused("proposal.revisedSceneNeedsNumber")
                                .param("scene", index + 1)));
                        }
                    }
                }
            }
            Err(cause) => note(Err(cause)),
        },
        Proposal::Comment { .. } => {}
        Proposal::Reply { comment_id } => {
            if crate::comment::get(conn, comment_id)?.is_none() {
                note(Err(Error::not_found("comment", comment_id)));
            }
        }
        Proposal::Description { style_id } => {
            if crate::style_brick::get(conn, style_id)?.is_none() {
                note(Err(Error::not_found("style", style_id)));
            }
        }
        Proposal::Words { package } => {
            problems.extend(crate::register::proposal::check(
                conn,
                package,
                overrides.items.as_deref(),
            )?);
        }
        Proposal::Bricks { package } => {
            problems.extend(crate::phrase::proposal::check(
                conn,
                package,
                overrides.items.as_deref(),
            )?);
        }
        Proposal::Canon { package } => {
            for problem in
                super::canon::check_package(conn, config, package, overrides.items.as_deref())?
            {
                problems.push(problem);
            }
        }
        Proposal::CardPrompt { note_id, .. } => {
            note(crate::canon::fact::card_of(conn, config, note_id).map(|_| ()));
        }
        Proposal::Release {
            release_id, fields, ..
        } => match crate::release::get(conn, release_id)? {
            None => note(Err(Error::not_found("release", release_id))),
            Some(_) => {
                let known = crate::release_meta::fields(conn, release_id)?;
                for key in fields.keys() {
                    if !known.iter().any(|field| field.key == *key) {
                        note(Err(
                            Error::refused("release.unknownField").param("field", key.as_str())
                        ));
                    }
                }
                if let Some(items) = &overrides.items {
                    if items.is_empty() {
                        note(Err(Error::refused("proposal.nothingTaken")));
                    }
                    for key in items {
                        if !fields.contains_key(key) {
                            note(Err(
                                Error::refused("release.unknownField").param("field", key.as_str())
                            ));
                        }
                    }
                }
            }
        },
        Proposal::CoverIdeas { work_id, ideas, .. } => match work::get(conn, work_id)? {
            None => note(Err(Error::not_found("work", work_id))),
            Some(found) => {
                if !config.vocabulary(&found.kind).cover {
                    note(Err(
                        Error::refused("idea.noCover").param("title", found.title.clone())
                    ));
                }
                if overrides.items.as_ref().is_some_and(Vec::is_empty) || ideas.is_empty() {
                    note(Err(Error::refused("proposal.nothingTaken")));
                }
                // What a brick deleted since the answer came would make of an
                // idea is said before any idea lands.
                for (index, idea) in ideas.iter().enumerate() {
                    if taken_idea(overrides, index) {
                        note(crate::cover::check(conn, &chat.profile_id, &idea.concept));
                    }
                }
            }
        },
        Proposal::Trials {
            work_id, trials, ..
        } => match work::get(conn, work_id)? {
            None => note(Err(Error::not_found("work", work_id))),
            Some(found) => {
                if config.lab(&found.kind).is_none() {
                    note(Err(
                        Error::refused("trial.noLab").param("title", found.title.clone())
                    ));
                }
                if overrides.items.as_ref().is_some_and(Vec::is_empty) || trials.is_empty() {
                    note(Err(Error::refused("proposal.nothingTaken")));
                }
            }
        },
    }
    Ok(problems)
}

/// Whether the person kept the idea at `index` of a proposal: every one when
/// they named none, else the ones named `idea:<index>`.
fn taken_idea(overrides: &Overrides, index: usize) -> bool {
    overrides
        .items
        .as_ref()
        .is_none_or(|items| items.iter().any(|item| *item == format!("idea:{index}")))
}

/// Write what a checked proposal says, through the actions a hand uses.
#[allow(clippy::too_many_arguments)]
fn keep(
    conn: &Connection,
    config: &ProfileConfig,
    chat: &Chat,
    body: &str,
    proposal: Proposal,
    overrides: Overrides,
    client: Option<String>,
    outcome: &mut Outcome,
) -> Result<()> {
    match proposal {
        Proposal::Version { role, label, from } => {
            let work_id = on_work(chat)?;
            let role = overrides.role.unwrap_or(role);
            let read = about_version(conn, chat, &work_id);
            // Written from the version the agent named, or from the one the
            // chat was started on when the text lands in its role: a rewrite
            // of revision 2 is revision 2's child (ADR 0055). A text the
            // person moved to another role has no parent there - lineage
            // does not cross roles.
            let mut parent = None;
            for candidate in [from.as_deref(), read.as_deref()].into_iter().flatten() {
                if version::in_line(conn, &work_id, &role, candidate)? {
                    parent = Some(candidate.to_owned());
                    break;
                }
            }
            // A commentary is about the version its chat was started on: the
            // critique of revision 2 says so, and the versions tab shows it
            // beside revision 2 rather than beside whichever revision shares
            // its number. A rewrite in the same role is its child instead.
            let meta = read.filter(|_| parent.is_none()).map(|about| {
                let mut meta = Map::new();
                meta.insert("about".into(), Value::String(about));
                meta
            });
            let version = super::version::create(
                conn,
                &work_id,
                NewVersion {
                    role,
                    body: body.to_owned(),
                    label: overrides
                        .label
                        .or(label)
                        .filter(|label| !label.trim().is_empty()),
                    meta,
                    // Not current unless the person said so: an answer worth
                    // keeping is not yet an answer worth standing behind.
                    make_current: overrides.make_current.unwrap_or(false),
                    parent_version_id: parent,
                    trial_id: None,
                },
            )?;
            outcome.versions.push(version.id);
        }

        Proposal::Score { axes, note, .. } => {
            let work_id = on_work(chat)?;
            // Tied to the version the chat is about, when it is about one:
            // a score judged revision 2 is a snapshot of revision 2.
            let version_id = about_version(conn, chat, &work_id);
            outcome.score = Some(score(
                conn,
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
            outcome.notes.push(note(
                conn,
                chat.work_id.as_deref(),
                PackagedNote {
                    title,
                    body: body.to_owned(),
                },
            )?);
        }

        Proposal::Work {
            title,
            work_kind,
            fields,
            versions,
            score: marks,
            notes,
            scenes,
            releases,
            trials,
            ..
        } => {
            let (work_id, kind, fresh) = match &chat.work_id {
                Some(id) => {
                    let found = work::get(conn, id)?.ok_or_else(|| Error::not_found("work", id))?;
                    (found.id, found.kind, false)
                }
                None => {
                    let kind = work_kind.unwrap_or_default();
                    let created = super::work::create(
                        conn,
                        NewWork {
                            kind: kind.clone(),
                            title: title.unwrap_or_default().trim().to_owned(),
                            meta: Some(fields.clone()),
                            ..NewWork::default()
                        },
                    )?;
                    outcome.fields = fields.keys().cloned().collect();
                    (created.id, kind, true)
                }
            };

            if !fresh && !fields.is_empty() {
                outcome.fields = fields.keys().cloned().collect();
                // Only the fields the package names: the patch merges them
                // into the work's by key, and its undo takes back these and
                // no others.
                super::work::update(
                    conn,
                    &work_id,
                    WorkPatch {
                        meta: Some(fields),
                        ..WorkPatch::default()
                    },
                )?;
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
                // The version it was written from, while that is still a
                // version of this work in this role (ADR 0055); a new work
                // has none to name.
                let parent = match packaged.from {
                    Some(from) if version::in_line(conn, &work_id, &packaged.role, &from)? => {
                        Some(from)
                    }
                    _ => None,
                };
                let created = super::version::create(
                    conn,
                    &work_id,
                    NewVersion {
                        role: packaged.role,
                        body: packaged.body,
                        label: packaged.label,
                        meta: None,
                        make_current: leading == Some(index),
                        parent_version_id: parent,
                        trial_id: None,
                    },
                )?;
                outcome.versions.push(created.id);
            }
            if let Some(marks) = marks {
                outcome.score = Some(score(conn, &work_id, None, marks, client)?);
            }
            for packaged in notes {
                outcome.notes.push(note(conn, Some(&work_id), packaged)?);
            }
            // A package adds its scenes after the last; replacing a board
            // is the scenes proposal's own decision, not a package's.
            for packaged in scenes {
                let position = packaged.position;
                outcome
                    .scenes
                    .push(add_scene(conn, &work_id, packaged, position)?);
            }
            // Last, because a release is about the finished thing: the
            // versions it requires and the board it describes have to be on
            // the work before it makes sense to plan shipping it.
            for packaged in releases {
                outcome.releases.push(release(conn, &work_id, packaged)?);
            }
            if !trials.is_empty() {
                outcome
                    .trials
                    .extend(super::trial::put(conn, &work_id, trials, |_| true)?);
            }
            outcome.created_work = fresh;
            outcome.work_id = Some(work_id);
        }

        Proposal::Scenes { scenes, change } => {
            let work_id = on_work(chat)?;
            board(conn, &work_id, scenes, change, outcome)?;
        }

        Proposal::Comment {
            channel,
            work_id,
            author,
            body,
            commented_on,
            ..
        } => {
            let new = overrides.comment.unwrap_or(crate::comment::NewComment {
                channel,
                body,
                work_id,
                author,
                reply: None,
                commented_on,
            });
            outcome.work_id = new.work_id.clone();
            outcome.comment = Some(super::comment::create(conn, new)?.id);
        }

        Proposal::Reply { comment_id } => {
            let text = overrides.reply.unwrap_or_else(|| body.to_owned());
            super::comment::reply(conn, &comment_id, text.trim())?;
            outcome.comment = Some(comment_id);
        }

        Proposal::Description { style_id } => {
            super::style::describe(conn, &style_id, body.trim())?;
            outcome.style_brick = Some(style_id);
        }

        Proposal::Canon { package } => {
            super::canon::keep_package(conn, package, overrides.items.as_deref(), outcome)?;
        }

        Proposal::Words { package } => {
            outcome.terms =
                crate::register::proposal::keep(conn, package, overrides.items.as_deref())?;
        }

        Proposal::Bricks { package } => {
            outcome.style_bricks =
                crate::phrase::proposal::keep(conn, package, overrides.items.as_deref())?;
        }

        Proposal::CardPrompt { note_id, basis } => {
            super::canon::describe(conn, &note_id, Some(body.to_owned()), basis)?;
            outcome.cards.push(note_id);
        }

        Proposal::Release {
            release_id, fields, ..
        } => {
            let taken: std::collections::BTreeMap<String, String> = fields
                .into_iter()
                .filter(|(key, _)| {
                    overrides
                        .items
                        .as_ref()
                        .is_none_or(|items| items.contains(key))
                })
                .filter_map(|(key, value)| Some((key, value.as_str()?.to_owned())))
                .collect();
            if !taken.is_empty() {
                let written = super::release::set_fields(conn, &release_id, &taken)?;
                outcome.work_id = Some(written.work_id);
            }
            outcome.release_fields = taken.into_keys().collect();
            outcome.releases.push(release_id);
        }

        Proposal::CoverIdeas { work_id, ideas, .. } => {
            for (index, idea) in ideas.into_iter().enumerate() {
                if !taken_idea(&overrides, index) {
                    continue;
                }
                let made = super::idea::create(
                    conn,
                    crate::cover::idea::NewIdea {
                        work_id: work_id.clone(),
                        source: idea.source,
                        from_work_id: None,
                        angle: idea.angle,
                        headline: idea.headline,
                        concept: idea.concept,
                        verdict: None,
                    },
                )?;
                outcome.ideas.push(made.id);
            }
            outcome.work_id = Some(work_id);
        }

        Proposal::Trials {
            work_id, trials, ..
        } => {
            outcome.trials = super::trial::put(conn, &work_id, trials, |index| {
                overrides
                    .items
                    .as_ref()
                    .is_none_or(|items| items.iter().any(|item| *item == format!("trial:{index}")))
            })?;
            outcome.work_id = Some(work_id);
        }
    }
    Ok(())
}

/// A proposed storyboard, written onto the board the way it says.
fn board(
    conn: &Connection,
    work_id: &str,
    scenes: Vec<PackagedScene>,
    change: BoardChange,
    outcome: &mut Outcome,
) -> Result<()> {
    match change {
        BoardChange::Replace => {
            // By number: the scene that already holds a number is rewritten in
            // place and keeps its id, what the new board does not number goes
            // to the trash, and a number nobody holds is a new row. A board
            // rebuilt from scratch would be a board whose every row is a
            // stranger to what pointed at it.
            let mut standing = scene::for_work(conn, work_id)?;
            for (index, packaged) in scenes.into_iter().enumerate() {
                let position = packaged.position.unwrap_or(index as i64 + 1);
                match standing.iter().position(|s| s.position == position) {
                    Some(at) => {
                        let existing = standing.remove(at);
                        outcome
                            .scenes
                            .push(rewrite_scene(conn, &existing.id, packaged, position)?);
                    }
                    None => {
                        outcome
                            .scenes
                            .push(add_scene(conn, work_id, packaged, Some(position))?)
                    }
                }
            }
            for leftover in standing {
                outcome.removed_scenes.push(super::trash::discard(
                    conn,
                    Entity::Scene,
                    &leftover.id,
                )?);
            }
        }
        BoardChange::Revise => {
            // Only the numbers named, and only in what is said about them: the
            // rest of the board is not looked at. A number nobody holds is a
            // new row, as on a replaced board — a revision that adds scene 9
            // to a board of 8 is a revision, not a mistake.
            let standing = scene::for_work(conn, work_id)?;
            for packaged in scenes {
                let position = packaged
                    .position
                    .ok_or_else(|| Error::refused("proposal.revisedSceneNeedsNumber"))?;
                match standing.iter().find(|s| s.position == position) {
                    Some(existing) => {
                        outcome
                            .scenes
                            .push(revise_scene(conn, &existing.id, packaged)?)
                    }
                    None => {
                        outcome
                            .scenes
                            .push(add_scene(conn, work_id, packaged, Some(position))?)
                    }
                }
            }
        }
        BoardChange::Add => {
            for packaged in scenes {
                let position = packaged.position;
                outcome
                    .scenes
                    .push(add_scene(conn, work_id, packaged, position)?);
            }
        }
    }
    Ok(())
}

fn on_work(chat: &Chat) -> Result<String> {
    chat.work_id
        .clone()
        .ok_or_else(|| Error::refused("proposal.needsWork"))
}

/// The version a chat is about, when it still is: the one the action was
/// started on, if it belongs to this work and has not been deleted since.
/// Anything else binds to nothing, which the score reads as "the current
/// version" — the same answer a chat started from the overview gives.
fn about_version(conn: &Connection, chat: &Chat, work_id: &str) -> Option<String> {
    let id = chat.version_id.as_deref()?;
    let found = version::get(conn, id).ok()??;
    (found.work_id == work_id).then_some(found.id)
}

fn score(
    conn: &Connection,
    work_id: &str,
    version_id: Option<String>,
    marks: Marks,
    rater: Option<String>,
) -> Result<String> {
    let created = super::score::create(
        conn,
        work_id,
        NewScore {
            axes: marks.axes,
            version_id,
            note: marks.note,
            rater,
        },
    )?;
    Ok(created.id)
}

fn note(conn: &Connection, work_id: Option<&str>, packaged: PackagedNote) -> Result<String> {
    let created = super::note::create(
        conn,
        NewNote {
            body: packaged.body,
            kind: None,
            title: packaged.title.filter(|title| !title.trim().is_empty()),
            work_id: work_id.map(str::to_owned),
            tags: Vec::new(),
            ..Default::default()
        },
    )?;
    Ok(created.id)
}

/// Plan a release, with what it goes out as.
fn release(conn: &Connection, work_id: &str, packaged: PackagedRelease) -> Result<String> {
    let created = super::release::create(
        conn,
        crate::release::NewRelease {
            work_id: work_id.to_owned(),
            kind: packaged.kind,
            scheduled_at: packaged.scheduled_at,
            meta: Some(packaged.fields),
            scheduled_time: None,
            time_zone: None,
        },
    )?;
    Ok(created.id)
}

/// The row a packaged scene becomes, numbered as told or after the last.
fn add_scene(
    conn: &Connection,
    work_id: &str,
    packaged: PackagedScene,
    position: Option<i64>,
) -> Result<String> {
    let created = super::scene::create(
        conn,
        NewScene {
            work_id: work_id.to_owned(),
            position,
            section: packaged.section,
            starts_at: packaged.starts_at,
            ends_at: packaged.ends_at,
            shot_type: packaged.shot_type,
            description: Some(packaged.description),
            blocks: Some(packaged.blocks),
        },
    )?;
    Ok(created.id)
}

/// A scene rewritten whole by a replaced board: every field set, the blocks
/// as a set, so the log's `before` holds the row as it was and an undo puts
/// the whole row back.
fn rewrite_scene(
    conn: &Connection,
    id: &str,
    packaged: PackagedScene,
    position: i64,
) -> Result<String> {
    super::scene::update(
        conn,
        id,
        ScenePatch {
            position: Some(position),
            section: Some(packaged.section),
            starts_at: Some(packaged.starts_at),
            ends_at: Some(packaged.ends_at),
            shot_type: Some(packaged.shot_type),
            description: Some(packaged.description),
            blocks: Some(packaged.blocks),
            // A storyboard proposed whole says nothing about a scene's built
            // frame: the one the person set stays.
            framing: None,
        },
    )?;
    Ok(id.to_owned())
}

/// A scene revised in place: only what the proposal says about it changes.
/// A field left out is kept — a revision that brings the prompt blocks says
/// nothing about the seconds — and a block left out is kept too: the blocks
/// named are laid over the ones the scene holds, so an action aimed at one
/// block cannot delete the others (0.66). An emptied block is cleared.
fn revise_scene(conn: &Connection, id: &str, packaged: PackagedScene) -> Result<String> {
    let before = scene::get(conn, id)?;
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
    super::scene::update(
        conn,
        id,
        ScenePatch {
            position: None,
            section: packaged.section.map(Some),
            starts_at: packaged.starts_at.map(Some),
            ends_at: packaged.ends_at.map(Some),
            shot_type: packaged.shot_type.map(Some),
            description: (!packaged.description.trim().is_empty()).then_some(packaged.description),
            blocks,
            framing: None,
        },
    )?;
    Ok(id.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::ASSISTANT;
    use crate::assistant::apply::{
        dismiss, pending, pending_comments, proposal_meta, render_board, render_package,
    };
    use crate::assistant::proposal::PackagedVersion;
    use crate::fixtures;
    use crate::minted::Minted;
    use crate::{note, operation, profile, score, trash};

    fn workspace() -> (Connection, String, String) {
        let (conn, profile_id) = fixtures::workspace();
        let work_id = fixtures::song(&conn, &profile_id, "Harbour lights").id;
        fixtures::version(&conn, &work_id, "lyrics", "one line");
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

    /// The problems a refused proposal was refused for, by locale key.
    fn problems(err: &Error) -> Vec<String> {
        let refusal = err.refusal().expect("the proposal was refused");
        assert_eq!(refusal.code, "proposal.invalid", "{err}");
        refusal.params["problems"]
            .as_array()
            .expect("a list of problems")
            .iter()
            .map(|problem| problem["key"].as_str().unwrap_or_default().to_owned())
            .collect()
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
                    from: None,
                },
                PackagedVersion {
                    role: "style".into(),
                    body: "slow, brushed drums".into(),
                    label: None,
                    from: None,
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
            releases: Vec::new(),
            trials: Vec::new(),
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
        let (conn, profile_id, _) = workspace();
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

        let outcome = apply(&conn, &message, Overrides::default()).unwrap();

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
            apply(&conn, &message, Overrides::default()).is_err(),
            "not twice"
        );
    }

    #[test]
    fn a_replaced_board_rewrites_by_number_and_sends_the_rest_to_the_trash() {
        let (conn, profile_id, _) = workspace();
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

        let outcome = apply(&conn, &message, Overrides::default()).unwrap();

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
        let (conn, profile_id, _) = workspace();
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

        let outcome = apply(&conn, &message, Overrides::default()).unwrap();

        assert_eq!(outcome.scenes[0], old[0]);
        assert_eq!(outcome.scenes.len(), 2);
        assert_eq!(operation_kinds(&conn), vec!["scene.update", "scene.create"]);
        assert_eq!(scene::count(&conn, &video_id).unwrap(), 2);
    }

    #[test]
    fn a_storyboard_for_a_kind_without_one_is_refused_before_anything_is_written() {
        let (conn, profile_id, song_id) = workspace();
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

        let err = apply(&conn, &message, Overrides::default()).unwrap_err();

        assert_eq!(problems(&err), ["refusal.scene.noStoryboard"]);
        assert!(operation_kinds(&conn).is_empty());
        assert!(is_pending(
            &assistant::message(&conn, &message).unwrap().unwrap()
        ));
    }

    #[test]
    fn a_package_on_a_new_video_creates_its_board_with_it() {
        let (conn, profile_id, _) = workspace();
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
                releases: Vec::new(),
                trials: Vec::new(),
            },
        );

        let outcome = apply(&conn, &message, Overrides::default()).unwrap();

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
        let (conn, profile_id) = fixtures::workspace();
        let config = profile::config_for(&conn, &profile_id).unwrap();
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
        let (conn, profile_id, work_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let message = propose(
            &conn,
            &chat,
            "one line\ntwo lines",
            Proposal::Version {
                role: "lyrics".into(),
                label: Some("longer".into()),
                from: None,
            },
        );

        let outcome = apply(&conn, &message, Overrides::default()).unwrap();

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

        let again = apply(&conn, &message, Overrides::default()).unwrap_err();
        assert_eq!(
            again.refusal().map(|refusal| refusal.code),
            Some("proposal.alreadyApplied")
        );
        assert_eq!(
            version::list(&conn, &work_id).unwrap().len(),
            2,
            "applied once"
        );
    }

    #[test]
    fn a_comment_read_off_a_screenshot_is_kept_through_the_log() {
        let (conn, profile_id, _) = workspace();
        // A comment is under a publication, never under the song (v0.86).
        let work_id = fixtures::video(&conn, &profile_id, "Harbour lights - clip").id;
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let message = propose(
            &conn,
            &chat,
            "One comment.",
            Proposal::Comment {
                channel: "main".into(),
                work_id: Some(work_id.clone()),
                author: Some("anna".into()),
                body: "loved the bridge".into(),
                commented_on: Some("2026-09-01".into()),
                about: None,
            },
        );

        let waiting = pending_comments(&conn, &profile_id).unwrap();
        assert_eq!(waiting.len(), 1, "the screen offers it before it is kept");

        let outcome = apply(&conn, &message, Overrides::default()).unwrap();

        let kept = crate::comment::get(&conn, outcome.comment.as_deref().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(kept.body, "loved the bridge");
        assert_eq!(kept.channel, "main");
        assert_eq!(kept.work_id.as_deref(), Some(work_id.as_str()));
        assert_eq!(kept.commented_on.as_deref(), Some("2026-09-01"));
        assert!(operation_kinds(&conn).contains(&"comment.create".to_owned()));
        assert!(
            pending_comments(&conn, &profile_id).unwrap().is_empty(),
            "a kept comment stops waiting"
        );
    }

    #[test]
    fn a_corrected_comment_is_kept_as_corrected() {
        let (conn, profile_id, _) = workspace();
        let chat = chat_on(&conn, &profile_id, None);
        let message = propose(
            &conn,
            &chat,
            "One comment.",
            Proposal::Comment {
                channel: "main".into(),
                work_id: None,
                author: None,
                body: "lovd the brige".into(),
                commented_on: None,
                about: None,
            },
        );

        let outcome = apply(
            &conn,
            &message,
            Overrides {
                comment: Some(crate::comment::NewComment {
                    channel: "second".into(),
                    body: "loved the bridge".into(),
                    ..Default::default()
                }),
                ..Overrides::default()
            },
        )
        .unwrap();

        let kept = crate::comment::get(&conn, outcome.comment.as_deref().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(kept.body, "loved the bridge");
        assert_eq!(kept.channel, "second");
    }

    #[test]
    fn a_drafted_reply_is_written_onto_its_comment_and_can_be_edited_first() {
        let (conn, profile_id, _) = workspace();
        let comment = crate::comment::create_minted(
            &conn,
            &profile_id,
            crate::comment::NewComment {
                channel: "main".into(),
                body: "what is the bridge about?".into(),
                ..Default::default()
            },
            Minted::fresh(),
        )
        .unwrap();
        let chat = chat_on(&conn, &profile_id, None);
        let first = propose(
            &conn,
            &chat,
            "It is about the last light.",
            Proposal::Reply {
                comment_id: comment.id.clone(),
            },
        );
        let second = propose(
            &conn,
            &chat,
            "Another take.",
            Proposal::Reply {
                comment_id: comment.id.clone(),
            },
        );

        apply(&conn, &first, Overrides::default()).unwrap();
        let written = crate::comment::get(&conn, &comment.id).unwrap().unwrap();
        assert_eq!(
            written.reply.as_deref(),
            Some("It is about the last light.")
        );
        assert_eq!(
            written.state,
            crate::comment::OPEN,
            "posting is the person's, by hand"
        );

        apply(
            &conn,
            &second,
            Overrides {
                reply: Some("  It is about the last light, and a boat.  ".into()),
                ..Overrides::default()
            },
        )
        .unwrap();
        let edited = crate::comment::get(&conn, &comment.id).unwrap().unwrap();
        assert_eq!(
            edited.reply.as_deref(),
            Some("It is about the last light, and a boat.")
        );

        let log = operation::all(&conn).unwrap();
        let last = log.last().unwrap();
        assert_eq!(last.kind, "comment.update");
        assert_eq!(
            last.params["before"]["reply"], "It is about the last light.",
            "the reply that stood before is recorded, so it can be taken back"
        );
    }

    #[test]
    fn a_description_is_written_onto_its_brick_as_an_edit_undo_takes_back() {
        let (conn, profile_id, _) = workspace();
        let brick = crate::style_brick::create(
            &conn,
            &profile_id,
            crate::style_brick::NewStyleBrick {
                type_key: "look".into(),
                name: "Dusk over water".into(),
                description: None,
                hint: None,
                ..crate::style_brick::NewStyleBrick::default()
            },
        )
        .unwrap();
        assert_eq!(brick.status, "draft");
        let chat = chat_on(&conn, &profile_id, None);
        let message = propose(
            &conn,
            &chat,
            "  Low sun, long reflections, teal against amber.  ",
            Proposal::Description {
                style_id: brick.id.clone(),
            },
        );

        let outcome = apply(&conn, &message, Overrides::default()).unwrap();
        assert_eq!(outcome.style_brick.as_deref(), Some(brick.id.as_str()));

        let written = crate::style_brick::get(&conn, &brick.id).unwrap().unwrap();
        assert_eq!(
            written.description.as_deref(),
            Some("Low sun, long reflections, teal against amber.")
        );
        assert_eq!(
            written.status, "ready",
            "a kept description lets it out of draft"
        );

        // Recorded as the edit a person makes in the dictionary, so a rebuild
        // plays it back and undo takes it back.
        let last = operation::all(&conn).unwrap().pop().unwrap();
        assert_eq!(last.kind, "style.update");
        let offer = crate::undo::last(&conn).unwrap().expect("it can be undone");
        crate::undo::undo(&conn, &offer.operation_id).unwrap();
        let back = crate::style_brick::get(&conn, &brick.id).unwrap().unwrap();
        assert_eq!(back.description, None);
        assert_eq!(back.status, "draft");

        // And the proposal is spent: the button becomes a mark.
        assert!(apply(&conn, &message, Overrides::default()).is_err());
    }

    #[test]
    fn a_description_kept_for_a_dropped_brick_leaves_it_dropped() {
        let (conn, profile_id, _) = workspace();
        let brick = crate::style_brick::create(
            &conn,
            &profile_id,
            crate::style_brick::NewStyleBrick {
                type_key: "look".into(),
                name: "Neon rain".into(),
                description: None,
                hint: None,
                ..crate::style_brick::NewStyleBrick::default()
            },
        )
        .unwrap();
        crate::style_brick::update(
            &conn,
            &brick.id,
            crate::style_brick::StyleBrickPatch {
                status: Some("dropped".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let chat = chat_on(&conn, &profile_id, None);
        let message = propose(
            &conn,
            &chat,
            "Wet asphalt under pink light.",
            Proposal::Description {
                style_id: brick.id.clone(),
            },
        );

        apply(&conn, &message, Overrides::default()).unwrap();
        let written = crate::style_brick::get(&conn, &brick.id).unwrap().unwrap();
        assert_eq!(
            written.description.as_deref(),
            Some("Wet asphalt under pink light.")
        );
        assert_eq!(written.status, "dropped");
    }

    #[test]
    fn the_dialogs_choices_override_the_proposal() {
        let (conn, profile_id, work_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let message = propose(
            &conn,
            &chat,
            "brushed drums",
            Proposal::Version {
                role: "lyrics".into(),
                label: None,
                from: None,
            },
        );

        let outcome = apply(
            &conn,
            &message,
            Overrides {
                role: Some("style".into()),
                label: Some("from the chat".into()),
                make_current: Some(true),
                ..Overrides::default()
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
        let (conn, profile_id, work_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let message = propose(
            &conn,
            &chat,
            "text",
            Proposal::Version {
                role: "storyboard".into(),
                label: None,
                from: None,
            },
        );
        let before = operation_kinds(&conn).len();

        let err = apply(&conn, &message, Overrides::default()).unwrap_err();

        assert!(err.to_string().contains("storyboard"), "{err}");
        assert_eq!(operation_kinds(&conn).len(), before, "nothing was written");
        assert!(is_pending(
            &assistant::message(&conn, &message).unwrap().unwrap()
        ));
    }

    #[test]
    fn a_score_from_an_agent_is_judged_by_the_agent() {
        let (conn, profile_id, work_id) = workspace();
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

        let outcome = apply(&conn, &message, Overrides::default()).unwrap();

        let history = score::history(&conn, &work_id).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, outcome.score.unwrap());
        assert_eq!(history[0].rater.as_deref(), Some("Claude Code"));
        assert_eq!(history[0].note.as_deref(), Some("strong chorus"));
    }

    #[test]
    fn a_package_in_a_chat_on_nothing_creates_the_whole_work_through_the_log() {
        let (conn, profile_id, _) = workspace();
        let chat = chat_on(&conn, &profile_id, None);
        let message = propose(&conn, &chat, "rendered", package());
        let before = operation_kinds(&conn);

        let outcome = apply(&conn, &message, Overrides::default()).unwrap();

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
        let (conn, profile_id, _) = workspace();
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
                releases: Vec::new(),
                trials: Vec::new(),
            },
        );

        let err = apply(&conn, &message, Overrides::default()).unwrap_err();

        assert_eq!(problems(&err), ["refusal.proposal.newWorkNeedsTitle"]);
    }

    /// Every mistake a package carries is named at once: a role the kind
    /// lacks, a release it does not ship, a board on a kind without one -
    /// not the first, and then on the next click the second.
    #[test]
    fn a_package_names_every_problem_at_once() {
        let (conn, profile_id, _) = workspace();
        let chat = chat_on(&conn, &profile_id, None);
        let Proposal::Work {
            fields, versions, ..
        } = package()
        else {
            unreachable!()
        };
        let mut versions = versions;
        versions.push(PackagedVersion {
            role: "storyboard".into(),
            body: "no such role for a song".into(),
            label: None,
            from: None,
        });
        let message = propose(
            &conn,
            &chat,
            "rendered",
            Proposal::Work {
                title: Some("Winter road".into()),
                work_kind: Some("song".into()),
                fields,
                unknown_fields: Vec::new(),
                versions,
                score: None,
                notes: Vec::new(),
                scenes: vec![packaged("a board on a song")],
                releases: vec![PackagedRelease {
                    kind: "podcast-episode".into(),
                    scheduled_at: None,
                    fields: Map::new(),
                    unknown_fields: Vec::new(),
                }],
                trials: Vec::new(),
            },
        );

        let err = apply(&conn, &message, Overrides::default()).unwrap_err();

        assert_eq!(
            problems(&err),
            [
                "refusal.version.unknownRole",
                "refusal.scene.noStoryboard",
                "refusal.release.noDoors",
            ]
        );
        assert!(operation_kinds(&conn).is_empty(), "nothing was written");
    }

    /// A package that passes the check and still fails while it is being
    /// written - here a scene that ends before it starts, which only the
    /// scene itself refuses - leaves nothing behind: no work, no versions,
    /// no line in the log or the history, and the message still waiting.
    /// Before v0.83 the work and its versions stayed, and the next click made
    /// a second work.
    #[test]
    fn a_package_that_fails_while_it_is_written_leaves_nothing_behind() {
        let (conn, profile_id, _) = workspace();
        let chat = chat_on(&conn, &profile_id, None);
        let Proposal::Work { fields, .. } = package() else {
            unreachable!()
        };
        let mut backwards = packaged("ends before it starts");
        backwards.starts_at = Some(20.0);
        backwards.ends_at = Some(5.0);
        let message = propose(
            &conn,
            &chat,
            "rendered",
            Proposal::Work {
                title: Some("The clip".into()),
                work_kind: Some("video".into()),
                fields,
                unknown_fields: Vec::new(),
                versions: vec![PackagedVersion {
                    role: "plot".into(),
                    body: "a keeper, a lamp, a storm".into(),
                    label: None,
                    from: None,
                }],
                score: None,
                notes: vec![PackagedNote {
                    title: None,
                    body: "a note that would have landed".into(),
                }],
                scenes: vec![packaged("fine"), backwards],
                releases: Vec::new(),
                trials: Vec::new(),
            },
        );
        let works_before = work::list(&conn, &profile_id, &Default::default())
            .unwrap()
            .len();
        let journal_before = crate::journal::list(&conn, &profile_id).unwrap().len();

        let err = apply(&conn, &message, Overrides::default()).unwrap_err();
        assert!(
            err.refusal()
                .is_none_or(|refusal| refusal.code != "proposal.invalid"),
            "the package passed the check and failed while it was written: {err}"
        );

        assert_eq!(
            work::list(&conn, &profile_id, &Default::default())
                .unwrap()
                .len(),
            works_before,
            "no work was left behind"
        );
        assert!(
            operation_kinds(&conn).is_empty(),
            "no operation was left behind"
        );
        assert_eq!(
            crate::journal::list(&conn, &profile_id).unwrap().len(),
            journal_before,
            "no line of history for a work that does not exist"
        );
        assert!(
            is_pending(&assistant::message(&conn, &message).unwrap().unwrap()),
            "the proposal still waits"
        );
    }

    #[test]
    fn a_package_on_a_work_adds_to_it_and_keeps_the_fields_it_did_not_name() {
        let (conn, profile_id, work_id) = workspace();
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
                releases: Vec::new(),
                trials: Vec::new(),
            },
        );

        let outcome = apply(&conn, &message, Overrides::default()).unwrap();

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
    fn what_is_pending_is_listed_across_every_chat_oldest_first() {
        // The bell's question: what has the assistant done that nobody has
        // answered? A proposal used to wait in a chat there was no reason to
        // open, which is the whole defect.
        let (conn, profile_id, work_id) = workspace();
        let first = chat_on(&conn, &profile_id, Some(&work_id));
        let second = chat_on(&conn, &profile_id, None);

        let older = propose(
            &conn,
            &first,
            "a version",
            Proposal::Version {
                role: "lyrics".into(),
                label: None,
                from: None,
            },
        );
        let newer = propose(&conn, &second, "a note", Proposal::Note { title: None });

        let listed = pending(&conn, &profile_id).unwrap();
        let ids: Vec<&str> = listed.iter().map(|p| p.message_id.as_str()).collect();
        assert_eq!(ids, vec![older.as_str(), newer.as_str()], "oldest first");
        assert_eq!(listed[0].work_id.as_deref(), Some(work_id.as_str()));
        assert_eq!(listed[0].kind, "version");
        assert_eq!(listed[1].work_id, None, "a chat about nothing has no work");
        assert_eq!(listed[1].kind, "note");

        apply(&conn, &older, Overrides::default()).unwrap();

        let after = pending(&conn, &profile_id).unwrap();
        assert_eq!(
            after
                .iter()
                .map(|p| p.message_id.as_str())
                .collect::<Vec<_>>(),
            vec![newer.as_str()],
            "an applied proposal has been answered and stops waiting"
        );
    }

    #[test]
    fn a_dismissed_proposal_stops_waiting_without_writing_anything() {
        let (conn, profile_id, work_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let message = propose(
            &conn,
            &chat,
            "a version",
            Proposal::Version {
                role: "lyrics".into(),
                label: None,
                from: None,
            },
        );

        let before = version::list(&conn, &work_id).unwrap().len();
        dismiss(&conn, &message).unwrap();

        assert!(
            pending(&conn, &profile_id).unwrap().is_empty(),
            "refusing is answering"
        );
        assert_eq!(
            version::list(&conn, &work_id).unwrap().len(),
            before,
            "and nothing was written"
        );

        // The answer itself is kept: a refused revision often has one good
        // line in it, and a transcript with holes is not a transcript.
        let stored = assistant::message(&conn, &message).unwrap().unwrap();
        assert_eq!(stored.body, "a version");
        assert!(stored.meta.contains_key(DISMISSED));
        assert!(!stored.meta.contains_key("applied"));
    }

    #[test]
    fn apply_pending_leaves_a_dismissed_proposal_alone() {
        // The two have to agree, or "apply everything pending" would bring
        // back exactly what someone had just refused.
        let (conn, profile_id, work_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let refused = propose(
            &conn,
            &chat,
            "no thanks",
            Proposal::Version {
                role: "lyrics".into(),
                label: None,
                from: None,
            },
        );
        let wanted = propose(
            &conn,
            &chat,
            "this one",
            Proposal::Version {
                role: "lyrics".into(),
                label: None,
                from: None,
            },
        );

        dismiss(&conn, &refused).unwrap();
        let outcomes = apply_pending(&conn, &chat).unwrap();

        assert_eq!(outcomes.len(), 1, "only the one still waiting");
        assert_eq!(outcomes[0].message_id, wanted);
    }

    #[test]
    fn an_applied_proposal_cannot_then_be_dismissed() {
        // Marking it refused after it was written would have the message say
        // the opposite of what the rows say.
        let (conn, profile_id, work_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let message = propose(
            &conn,
            &chat,
            "a version",
            Proposal::Version {
                role: "lyrics".into(),
                label: None,
                from: None,
            },
        );
        apply(&conn, &message, Overrides::default()).unwrap();

        assert!(dismiss(&conn, &message).is_err());
    }

    #[test]
    fn apply_pending_takes_every_unapplied_proposal_in_order_and_skips_the_applied() {
        let (conn, profile_id, work_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let first = propose(
            &conn,
            &chat,
            "first",
            Proposal::Version {
                role: "lyrics".into(),
                label: None,
                from: None,
            },
        );
        apply(&conn, &first, Overrides::default()).unwrap();
        propose(
            &conn,
            &chat,
            "second",
            Proposal::Version {
                role: "lyrics".into(),
                label: None,
                from: None,
            },
        );
        propose(&conn, &chat, "a thought", Proposal::Note { title: None });
        // A plain answer with no proposal is not something to apply.
        assistant::append(&conn, &chat, ASSISTANT, "just prose", Map::new()).unwrap();

        let outcomes = apply_pending(&conn, &chat).unwrap();

        assert_eq!(outcomes.len(), 2);
        assert_eq!(version::list(&conn, &work_id).unwrap().len(), 3);
        let transcript = assistant::transcript(&conn, &chat).unwrap().unwrap();
        assert!(transcript.messages.iter().all(|m| !is_pending(m)));

        assert!(
            apply_pending(&conn, &chat).unwrap().is_empty(),
            "nothing left to apply"
        );
    }

    #[test]
    fn apply_pending_stops_at_the_first_failure_and_says_so() {
        let (conn, profile_id, work_id) = workspace();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        propose(
            &conn,
            &chat,
            "fine",
            Proposal::Version {
                role: "lyrics".into(),
                label: None,
                from: None,
            },
        );
        propose(
            &conn,
            &chat,
            "broken",
            Proposal::Version {
                role: "storyboard".into(),
                label: None,
                from: None,
            },
        );

        let err = apply_pending(&conn, &chat).unwrap_err();

        let refusal = err.refusal().expect("a refusal");
        assert_eq!(refusal.code, "proposal.stoppedAfter");
        assert_eq!(refusal.params["applied"], 1);
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
                from: None,
            }],
            score: None,
            notes: Vec::new(),
            scenes: Vec::new(),
            releases: Vec::new(),
            trials: Vec::new(),
        };

        let body = render_package(&proposal, &config, "song");

        assert!(body.contains("````\na line with ``` in it\n````"), "{body}");
    }

    #[test]
    fn a_revised_scene_changes_only_what_it_names_and_the_rest_of_the_board_stands() {
        let (conn, profile_id, _) = workspace();
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

        let outcome = apply(&conn, &message, Overrides::default()).unwrap();

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
        let refused = apply(&conn, &unnumbered, Overrides::default()).unwrap_err();
        assert_eq!(
            problems(&refused),
            ["refusal.proposal.revisedSceneNeedsNumber"]
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
        let (conn, profile_id, _) = workspace();
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

        apply(&conn, &message, Overrides::default()).unwrap();

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
        apply(&conn, &message, Overrides::default()).unwrap();

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
        let (conn, profile_id, _) = workspace();
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

        apply(&conn, &message, Overrides::default()).unwrap();

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
        let (conn, profile_id, _) = workspace();
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

        let outcome = apply(&conn, &message, Overrides::default()).unwrap();

        assert_eq!(outcome.scenes, vec![old[0].clone()], "rewritten in place");
        assert_eq!(
            outcome.removed_scenes.len(),
            1,
            "the rest went to the trash"
        );
    }

    /// A proposal for the canon of a new card with its facts and a relation.
    fn canon_package(conn: &Connection, profile_id: &str, wren: &str, work_id: &str) -> Proposal {
        let raw = json!({
            "cards": [{ "handle": "new-1", "kind": "character", "title": "Alex", "on_work": true }],
            "facts": [
                { "card": "new-1", "section": "identity", "text": "A courier.", "line": "the courier knocks" },
                { "card": wren, "section": "tastes", "text": "Counts the pauses." },
            ],
            "relations": [{ "from": "new-1", "to": wren, "label": "courier", "back_label": "client" }]
        });
        let package = crate::canon::proposal::read(
            conn,
            profile_id,
            &raw,
            &crate::canon::proposal::Defaults {
                work_id: Some(work_id.to_owned()),
                ..crate::canon::proposal::Defaults::default()
            },
        )
        .unwrap();
        Proposal::Canon { package }
    }

    #[test]
    fn a_proposal_for_the_canon_is_kept_whole_through_the_gestures_a_hand_uses() {
        let (conn, profile_id, work_id) = workspace();
        let wren = fixtures::card(&conn, &profile_id, "character", "Wren").id;
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let message = propose(
            &conn,
            &chat,
            "",
            canon_package(&conn, &profile_id, &wren, &work_id),
        );

        let outcome = apply(&conn, &message, Overrides::default()).unwrap();

        assert_eq!(outcome.cards.len(), 1);
        assert_eq!(outcome.facts.len(), 2);
        assert_eq!(outcome.relations.len(), 1);
        let alex = note::get(&conn, &outcome.cards[0]).unwrap().unwrap();
        assert_eq!(
            alex.work_id.as_deref(),
            Some(work_id.as_str()),
            "the hero lives at the work"
        );
        let courier = crate::canon::fact::for_card(&conn, &alex.id).unwrap();
        assert_eq!(
            courier[0].source.as_ref().and_then(|s| s.line.as_deref()),
            Some("the courier knocks")
        );
        let kinds = operation_kinds(&conn);
        for kind in ["note.create", "fact.create", "canonLink.create"] {
            assert!(
                kinds.iter().any(|k| k == kind),
                "no `{kind}` in the log: {kinds:?}"
            );
        }
        assert!(
            apply(&conn, &message, Overrides::default()).is_err(),
            "kept twice"
        );
    }

    #[test]
    fn what_the_assistant_proposes_is_kept_as_a_draft_and_a_live_zone_stays_one() {
        let (conn, profile_id, work_id) = workspace();
        let wren = fixtures::card(&conn, &profile_id, "character", "Wren");
        let settled = fixtures::fact(&conn, &wren.id, "looks", "Freckles.");
        let raw = json!({
            "facts": [
                { "card": wren.id, "section": "tastes", "text": "Counts the pauses.", "status": "canon" },
                { "card": wren.id, "section": "open", "text": "Who taught her the song?", "status": "open" },
                { "change": "refine", "fact": settled.id, "text": "Freckles across the nose.", "status": "canon" },
            ]
        });
        let package = crate::canon::proposal::read(
            &conn,
            &profile_id,
            &raw,
            &crate::canon::proposal::Defaults::default(),
        )
        .unwrap();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let message = propose(&conn, &chat, "", Proposal::Canon { package });

        apply(&conn, &message, Overrides::default()).unwrap();

        let status_of = |body: &str| {
            crate::canon::fact::for_card(&conn, &wren.id)
                .unwrap()
                .into_iter()
                .find(|fact| fact.body == body)
                .map(|fact| fact.status)
        };
        assert_eq!(
            status_of("Counts the pauses."),
            Some(crate::canon::FactStatus::Draft),
            "a proposal settled a fact itself: it would reach a cover unread"
        );
        assert_eq!(
            status_of("Who taught her the song?"),
            Some(crate::canon::FactStatus::Open)
        );
        assert_eq!(
            status_of("Freckles across the nose."),
            Some(crate::canon::FactStatus::Canon),
            "a sharper wording keeps the fact as settled as it was"
        );
    }

    /// An agent brings a card its pictures and its description (v0.89.2):
    /// what is read off the proposal is what the person can keep - a file
    /// that is not a picture, a role that is not one, a card the canon does
    /// not have are left out and said.
    #[test]
    fn pictures_and_descriptions_are_read_against_the_canon_and_the_disk() {
        let (conn, profile_id, _) = workspace();
        let wren = fixtures::card(&conn, &profile_id, "character", "Wren");
        let coat = fixtures::fact(&conn, &wren.id, "outfits", "A long green coat.");
        let dir = tempfile::tempdir().unwrap();
        let face = fixtures::file(dir.path(), "wren-face.PNG");
        let notes = fixtures::file(dir.path(), "notes.txt");
        let raw = json!({
            "pictures": [
                { "card": "wren", "path": face.display().to_string(), "role": "portrait" },
                { "fact": coat.id, "path": face.display().to_string(), "role": "outfit" },
                { "card": "Wren", "path": notes.display().to_string() },
                { "card": "Wren", "path": dir.path().join("gone.jpg").display().to_string() },
                { "card": "Wren", "path": "wren-face.PNG" },
                { "card": "Wren", "path": face.display().to_string(), "role": "selfie" },
                { "card": "Nobody", "path": face.display().to_string() },
            ],
            "descriptions": [
                { "card": "Wren", "text": "A young woman with freckles." },
                { "card": "Nobody", "text": "Someone." },
                { "card": "Wren", "text": "  " },
            ]
        });
        let package = crate::canon::proposal::read(
            &conn,
            &profile_id,
            &raw,
            &crate::canon::proposal::Defaults::default(),
        )
        .unwrap();

        assert_eq!(package.pictures.len(), 2);
        assert_eq!(package.pictures[0].card, wren.id, "named, found by name");
        assert_eq!(package.pictures[0].card_title, "Wren");
        assert_eq!(
            package.pictures[1].card, wren.id,
            "a picture of a fact is a picture of its card"
        );
        assert_eq!(package.descriptions.len(), 1);
        assert_eq!(package.descriptions[0].card, wren.id);
        let said: Vec<&str> = package.dropped.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(
            said,
            [
                "refusal.canon.pictureNotAPicture",
                "refusal.canon.pictureNotAFile",
                "refusal.canon.pictureNotAFile",
                "refusal.canon.pictureRole",
                "refusal.canon.pictureNamesNoCard",
                "refusal.canon.descriptionNamesNoCard",
                "refusal.canon.descriptionEmpty",
            ]
        );
    }

    /// Kept, a picture is the arrival a chosen file takes - copied beside the
    /// workspace and attached with its role - and a description is written
    /// with the facts it answers to; an item left out is not touched.
    #[test]
    fn kept_pictures_are_copied_in_and_a_description_is_written() {
        let (conn, profile_id, dir) = fixtures::workspace_on_disk();
        let work_id = fixtures::song(&conn, &profile_id, "Harbour lights").id;
        let wren = fixtures::card(&conn, &profile_id, "character", "Wren");
        fixtures::fact(&conn, &wren.id, "looks", "Freckles.");
        let coat = fixtures::fact(&conn, &wren.id, "outfits", "A long green coat.");
        let source = tempfile::tempdir().unwrap();
        let face = fixtures::file(source.path(), "wren-face.png");
        let coat_file = fixtures::file(source.path(), "wren-coat.jpg");
        let raw = json!({
            "pictures": [
                { "card": wren.id, "path": face.display().to_string(), "role": "portrait" },
                { "fact": coat.id, "path": coat_file.display().to_string(), "role": "outfit" },
                { "card": wren.id, "path": coat_file.display().to_string() },
            ],
            "descriptions": [{ "card": wren.id, "text": "A young woman with freckles." }]
        });
        let package = crate::canon::proposal::read(
            &conn,
            &profile_id,
            &raw,
            &crate::canon::proposal::Defaults::default(),
        )
        .unwrap();
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let message = propose(&conn, &chat, "", Proposal::Canon { package });

        let outcome = apply(
            &conn,
            &message,
            Overrides {
                items: Some(vec![
                    "picture:0".into(),
                    "picture:1".into(),
                    "description:0".into(),
                ]),
                ..Overrides::default()
            },
        )
        .unwrap();

        assert_eq!(outcome.pictures.len(), 2, "the one left out stays out");
        let pictures = crate::asset::for_card(&conn, &wren.id).unwrap();
        assert_eq!(pictures.len(), 2);
        let portrait = pictures.iter().find(|p| p.kind == "portrait").unwrap();
        assert_eq!(portrait.original_name.as_deref(), Some("wren-face.png"));
        // By the real paths: on macOS the temporary directory is reached
        // through a link (/var is /private/var), and the database names it
        // by where it really is.
        let media = std::fs::canonicalize(dir.path().join("media")).unwrap();
        assert!(
            std::fs::canonicalize(&portrait.path)
                .unwrap()
                .starts_with(&media),
            "copied beside the workspace: {}",
            portrait.path
        );
        assert!(std::path::Path::new(&portrait.path).is_file());
        let outfit = pictures.iter().find(|p| p.kind == "outfit").unwrap();
        assert_eq!(outfit.canon_fact_id.as_deref(), Some(coat.id.as_str()));

        let described = note::get(&conn, &wren.id).unwrap().unwrap();
        assert_eq!(
            described.prompt.as_deref(),
            Some("A young woman with freckles.")
        );
        assert!(
            described.prompt_basis.is_some(),
            "written with the facts it answers to"
        );
        assert!(
            operation_kinds(&conn).iter().any(|k| k == "asset.attach"),
            "a picture arrives as a chosen file does"
        );
    }

    /// A package of pictures and descriptions alone waits on the cards it
    /// is about: the side panel of an open card finds it there (found
    /// moving the owner's characters, v0.89.3 - the panel listed only the
    /// cards of facts and relations).
    #[test]
    fn a_package_of_pictures_and_descriptions_waits_on_the_cards_it_is_about() {
        let (conn, profile_id, _) = workspace();
        let wren = fixtures::card(&conn, &profile_id, "character", "Wren");
        let harbour = fixtures::card(&conn, &profile_id, "location", "Harbour");
        let dir = tempfile::tempdir().unwrap();
        let face = fixtures::file(dir.path(), "wren.png");
        let raw = json!({
            "pictures": [{ "card": wren.id, "path": face.display().to_string() }],
            "descriptions": [{ "card": harbour.id, "text": "A stone pier at dusk." }]
        });
        let package = crate::canon::proposal::read(
            &conn,
            &profile_id,
            &raw,
            &crate::canon::proposal::Defaults::default(),
        )
        .unwrap();
        let chat = chat_on(&conn, &profile_id, None);
        propose(&conn, &chat, "", Proposal::Canon { package });

        let waiting = crate::assistant::apply::pending_canon(&conn, &profile_id).unwrap();
        assert_eq!(waiting.len(), 1);
        assert!(
            waiting[0].cards.contains(&wren.id),
            "{:?}",
            waiting[0].cards
        );
        assert!(
            waiting[0].cards.contains(&harbour.id),
            "{:?}",
            waiting[0].cards
        );
    }

    #[test]
    fn a_proposal_for_the_canon_is_kept_item_by_item_and_a_fact_on_a_card_left_out_is_refused() {
        let (conn, profile_id, work_id) = workspace();
        let wren = fixtures::card(&conn, &profile_id, "character", "Wren").id;
        let chat = chat_on(&conn, &profile_id, Some(&work_id));
        let message = propose(
            &conn,
            &chat,
            "",
            canon_package(&conn, &profile_id, &wren, &work_id),
        );

        let refused = apply(
            &conn,
            &message,
            Overrides {
                items: Some(vec!["fact:0".into()]),
                ..Overrides::default()
            },
        )
        .unwrap_err();
        assert_eq!(problems(&refused), ["refusal.canon.onACardLeftOut"]);
        assert!(
            crate::canon::fact::for_card(&conn, &wren)
                .unwrap()
                .is_empty(),
            "a refused proposal wrote a fact"
        );

        let outcome = apply(
            &conn,
            &message,
            Overrides {
                items: Some(vec!["fact:1".into()]),
                ..Overrides::default()
            },
        )
        .unwrap();
        assert!(outcome.cards.is_empty());
        assert_eq!(outcome.facts.len(), 1);
        assert_eq!(
            crate::canon::fact::for_card(&conn, &wren).unwrap()[0].body,
            "Counts the pauses."
        );
    }

    #[test]
    fn a_description_is_written_with_the_facts_it_answers_to() {
        let (conn, profile_id, _) = workspace();
        let wren = fixtures::card(&conn, &profile_id, "character", "Wren").id;
        fixtures::fact(&conn, &wren, "looks", "Freckles.");
        let basis = crate::canon::view::basis_for(&conn, &wren).unwrap();
        let chat = chat_on(&conn, &profile_id, None);
        let message = propose(
            &conn,
            &chat,
            "  a young woman with freckles  ",
            Proposal::CardPrompt {
                note_id: wren.clone(),
                basis,
            },
        );

        apply(&conn, &message, Overrides::default()).unwrap();

        let card = note::get(&conn, &wren).unwrap().unwrap();
        assert_eq!(card.prompt.as_deref(), Some("a young woman with freckles"));
        assert!(!crate::canon::view::card(&conn, &wren).unwrap().prompt_stale);
    }
}

#[cfg(test)]
mod bound_to_a_version_tests {
    use super::*;
    use crate::assistant::ASSISTANT;
    use crate::assistant::apply::proposal_meta;
    use crate::db;
    use crate::profile;
    use crate::work::version::{self, NewVersion};
    use crate::work::{self, NewWork};

    fn two_revisions() -> (Connection, String, String, String) {
        let conn = db::open_in_memory().unwrap();
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
        let make = |body: &str| {
            version::create(
                &conn,
                &work_id,
                NewVersion {
                    role: "lyrics".into(),
                    body: body.into(),
                    label: None,
                    meta: None,
                    make_current: true,
                    parent_version_id: None,
                    trial_id: None,
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
        let (conn, profile_id, work_id, first) = two_revisions();
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
        let outcome = apply(&conn, &message_id, Overrides::default()).unwrap();

        let score = crate::score::get(&conn, &outcome.score.unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(score.version_id.as_deref(), Some(first.as_str()));
    }

    /// A version applied from a chat about revision 1 says it is about
    /// revision 1, and the summary reads it back.
    #[test]
    fn a_commentary_says_which_version_it_is_about() {
        let (conn, profile_id, work_id, first) = two_revisions();
        let chat_id = chat_about(&conn, &profile_id, &work_id, &first);
        let meta = proposal_meta(
            "Claude Code",
            &Proposal::Version {
                role: "critique".into(),
                label: None,
                from: None,
            },
            None,
        )
        .unwrap();
        let message_id = assistant::append(&conn, &chat_id, ASSISTANT, "weak second line", meta)
            .unwrap()
            .id;
        let outcome = apply(&conn, &message_id, Overrides::default()).unwrap();

        let id = &outcome.versions[0];
        let summary = version::list(&conn, &work_id)
            .unwrap()
            .into_iter()
            .find(|v| &v.id == id)
            .unwrap();
        assert_eq!(summary.about_version_id.as_deref(), Some(first.as_str()));
        assert_eq!(summary.role, "critique");
    }

    /// A rewrite in the role of the version the chat is about is that
    /// version's child (ADR 0055): revision 1 rewritten is written from
    /// revision 1, although revision 2 is the newest - and it is a draft, not
    /// a commentary about revision 1.
    #[test]
    fn a_rewrite_is_the_child_of_the_version_the_chat_is_about() {
        let (conn, profile_id, work_id, first) = two_revisions();
        let chat_id = chat_about(&conn, &profile_id, &work_id, &first);
        let meta = proposal_meta(
            "Claude Code",
            &Proposal::Version {
                role: "lyrics".into(),
                label: None,
                from: None,
            },
            None,
        )
        .unwrap();
        let message_id = assistant::append(&conn, &chat_id, ASSISTANT, "one, sharper", meta)
            .unwrap()
            .id;
        let outcome = apply(&conn, &message_id, Overrides::default()).unwrap();

        let made = version::get(&conn, &outcome.versions[0]).unwrap().unwrap();
        assert_eq!(made.parent_version_id.as_deref(), Some(first.as_str()));
        assert!(
            !made.meta.contains_key("about"),
            "a draft written from a revision is not a commentary on it"
        );
    }

    /// The version an agent says it rewrote is the parent, over the one the
    /// chat is about; moved by the person into another role, the text has no
    /// parent there - lineage does not cross roles.
    #[test]
    fn the_version_an_agent_rewrote_is_the_parent_while_the_role_holds() {
        let (conn, profile_id, work_id, first) = two_revisions();
        let newest = version::latest(&conn, &work_id, "lyrics")
            .unwrap()
            .unwrap()
            .id;
        let chat_id = chat_about(&conn, &profile_id, &work_id, &first);
        let proposed = |body: &str| {
            let meta = proposal_meta(
                "Claude Code",
                &Proposal::Version {
                    role: "lyrics".into(),
                    label: None,
                    from: Some(newest.clone()),
                },
                None,
            )
            .unwrap();
            assistant::append(&conn, &chat_id, ASSISTANT, body, meta)
                .unwrap()
                .id
        };

        let kept = apply(&conn, &proposed("two, sharper"), Overrides::default()).unwrap();
        let made = version::get(&conn, &kept.versions[0]).unwrap().unwrap();
        assert_eq!(made.parent_version_id.as_deref(), Some(newest.as_str()));

        let moved = apply(
            &conn,
            &proposed("now a style"),
            Overrides {
                role: Some("style".into()),
                ..Overrides::default()
            },
        )
        .unwrap();
        let made = version::get(&conn, &moved.versions[0]).unwrap().unwrap();
        assert_eq!(made.role, "style");
        assert_eq!(made.parent_version_id, None);
    }

    /// The version the chat was about is gone: the proposal still applies,
    /// bound to nothing rather than refused.
    #[test]
    fn a_deleted_version_binds_to_nothing() {
        let (conn, profile_id, work_id, first) = two_revisions();
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
        let outcome = apply(&conn, &message_id, Overrides::default()).unwrap();
        let score = crate::score::get(&conn, &outcome.score.unwrap())
            .unwrap()
            .unwrap();
        assert_ne!(score.version_id.as_deref(), Some(first.as_str()));
    }
}
