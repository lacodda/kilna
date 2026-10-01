//! Proposals waiting for an answer, and how they read before they are kept.
//!
//! A proposal — the panel's fenced score, an agent's version, note or whole
//! package — is a message with `meta.proposal`. This module finds the ones
//! nobody has answered, turns one down, and renders a package or a board as
//! the text its message shows. Keeping one is a gesture, and lives with the
//! others: see [`crate::actions::proposal`].

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::assistant::proposal::{BoardChange, PackagedScene, Proposal};
use crate::assistant::{ASSISTANT, Message};
use crate::error::{Error, Result};
use crate::profile;
use crate::time;

/// Whether a message carries a proposal nobody has applied yet.
pub fn is_pending(message: &Message) -> bool {
    message.role == ASSISTANT
        && message.meta.get("proposal").is_some_and(Value::is_object)
        && !message.meta.contains_key("applied")
        // A proposal turned down is answered, and "apply everything pending"
        // must not bring back what someone just refused.
        && !message.meta.contains_key(DISMISSED)
}

/// A proposal nobody has answered yet, with enough about it to be listed
/// away from the chat it arrived in.
///
/// The chat is where a proposal is read and applied; this is how it is
/// *noticed*. A proposal that came in over MCP while the window was on
/// another screen used to wait in a chat nobody had a reason to open — the
/// assistant had done the work and said so to an empty room.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Pending {
    pub message_id: String,
    pub chat_id: String,
    /// What the chat is called, for a line that has to say where to go.
    pub chat_title: Option<String>,
    /// The work the chat is about, when it is about one.
    pub work_id: Option<String>,
    /// `version`, `score`, `note`, `scenes`, `work`, `package` — what kind of
    /// thing is being proposed, so the line can say so without reading it.
    pub kind: String,
    pub created_at: String,
}

/// Every proposal of a profile that has been neither applied nor dismissed,
/// oldest first.
///
/// Oldest first for the reason the waiting questions are: the one that has
/// been sitting longest is the one holding something up.
pub fn pending(conn: &Connection, profile_id: &str) -> Result<Vec<Pending>> {
    let mut statement = conn.prepare(
        "SELECT m.id, m.chat_id, c.title, c.work_id, m.meta, m.created_at
           FROM chat_message m
           JOIN chat c ON c.id = m.chat_id
          WHERE c.profile_id = ?1 AND m.role = ?2
          ORDER BY m.created_at, m.rowid",
    )?;

    let rows = statement.query_map(params![profile_id, ASSISTANT], |row| {
        let raw: String = row.get(4)?;
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            raw,
            row.get::<_, String>(5)?,
        ))
    })?;

    let mut out = Vec::new();
    for row in rows {
        let (message_id, chat_id, chat_title, work_id, raw, created_at) = row?;
        let meta: Map<String, Value> = serde_json::from_str(&raw).unwrap_or_default();

        // The same three conditions as `is_pending`, plus the dismissal: read
        // here from the stored row rather than from a `Message`, because the
        // whole point is not to load every transcript of every chat.
        let Some(proposal) = meta.get("proposal").and_then(Value::as_object) else {
            continue;
        };
        if meta.contains_key("applied") || meta.contains_key(DISMISSED) {
            continue;
        }

        out.push(Pending {
            message_id,
            chat_id,
            chat_title,
            work_id,
            kind: proposal
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("proposal")
                .to_owned(),
            created_at,
        });
    }
    Ok(out)
}

/// A comment or a reply waiting to be kept, with what it says: the comments
/// screen shows each where it belongs — a reply under its comment, a
/// comment read off a screenshot at the top of the inbox.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct CommentProposal {
    pub message_id: String,
    pub chat_id: String,
    /// The answer: the reply itself, or the text the comment was read from.
    pub body: String,
    pub proposal: Proposal,
    pub created_at: String,
}

/// Every comment and reply proposal of a profile nobody has answered, oldest
/// first — [`pending`] narrowed to the two kinds, with their contents.
pub fn pending_comments(conn: &Connection, profile_id: &str) -> Result<Vec<CommentProposal>> {
    let mut statement = conn.prepare(
        "SELECT m.id, m.chat_id, m.body, m.meta, m.created_at
           FROM chat_message m
           JOIN chat c ON c.id = m.chat_id
          WHERE c.profile_id = ?1 AND m.role = ?2
            AND json_extract(m.meta, '$.proposal.kind') IN ('comment', 'reply')
          ORDER BY m.created_at, m.rowid",
    )?;
    let rows = statement
        .query_map(params![profile_id, ASSISTANT], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut out = Vec::new();
    for (message_id, chat_id, body, raw, created_at) in rows {
        let meta: Map<String, Value> = serde_json::from_str(&raw).unwrap_or_default();
        if meta.contains_key("applied") || meta.contains_key(DISMISSED) {
            continue;
        }
        let Some(proposal) = meta
            .get("proposal")
            .cloned()
            .and_then(|value| serde_json::from_value::<Proposal>(value).ok())
        else {
            continue;
        };
        out.push(CommentProposal {
            message_id,
            chat_id,
            body,
            proposal,
            created_at,
        });
    }
    Ok(out)
}

/// The mark that says a proposal was turned down.
///
/// Stored beside `applied` rather than as a second table, and for the same
/// reason `applied` is: what happened to a proposal is a fact about that
/// message, and a row elsewhere pointing at it would be a second place to
/// look and a second place to forget.
pub const DISMISSED: &str = "dismissed";

/// Turn a proposal down: it stops waiting, and nothing is written.
///
/// Kept rather than deleted. The answer is still worth reading after it has
/// been refused — a rejected revision often has one line in it — and a
/// transcript with holes in it is not a transcript.
pub fn dismiss(conn: &Connection, message_id: &str) -> Result<()> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT meta FROM chat_message WHERE id = ?1",
            params![message_id],
            |row| row.get(0),
        )
        .optional()?;
    let raw = raw.ok_or_else(|| Error::not_found("message", message_id))?;

    let mut meta: Map<String, Value> = serde_json::from_str(&raw).unwrap_or_default();
    if meta.contains_key("applied") {
        return Err(Error::refused("proposal.alreadyApplied"));
    }
    // Twice is not a failure: two windows can show the same bell.
    meta.insert(DISMISSED.to_owned(), Value::String(time::now()));

    conn.execute(
        "UPDATE chat_message SET meta = ?2 WHERE id = ?1",
        params![message_id, serde_json::to_string(&meta)?],
    )?;
    Ok(())
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
        releases,
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

    for packaged in releases {
        let kind = vocabulary
            .release_kinds
            .iter()
            .find(|k| k.key == packaged.kind);
        let heading = kind.map_or(packaged.kind.as_str(), |k| k.label.as_str());
        match &packaged.scheduled_at {
            Some(date) => out.push_str(&format!("### {heading} — {date}\n\n")),
            // No date is not a missing one: a release with none is queued,
            // which is what "plan this, I will find it a day" means.
            None => out.push_str(&format!("### {heading}\n\n")),
        }
        for (key, value) in &packaged.fields {
            let label = kind
                .and_then(|k| k.fields.iter().find(|f| f.key == *key))
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
        if !packaged.unknown_fields.is_empty() {
            out.push_str(&format!(
                "_Not fields of this kind of release, left out: {}._\n\n",
                packaged.unknown_fields.join(", ")
            ));
        }
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
            .map_or(key.to_owned(), |s| s.label.as_str().to_owned())
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

/// Ideas for a cover as a person reads them before putting them on the
/// board: each with its headline and angle, what it says, and what it is
/// built from; then what was left out, and why.
pub fn render_ideas(
    conn: &rusqlite::Connection,
    ideas: &[crate::cover::idea::Packaged],
    dropped: &[crate::cover::idea::Dropped],
) -> String {
    let mut out = String::new();
    for (index, idea) in ideas.iter().enumerate() {
        let headline = if idea.headline.is_empty() {
            format!("Idea {}", index + 1)
        } else {
            idea.headline.clone()
        };
        out.push_str(&format!("**{headline}**"));
        if !idea.angle.is_empty() {
            out.push_str(&format!(" · _{}_", idea.angle));
        }
        out.push('\n');
        if !idea.concept.idea.trim().is_empty() {
            out.push_str(&format!("\n{}\n", idea.concept.idea.trim()));
        }
        if !idea.concept.scene.trim().is_empty() {
            out.push_str(&format!("\n> {}\n", idea.concept.scene.trim()));
        }
        if !idea.concept.avoid.trim().is_empty() {
            out.push_str(&format!("\n> Avoid: {}\n", idea.concept.avoid.trim()));
        }
        let mut parts = Vec::new();
        if let Some(framing) = idea.concept.framing {
            parts.push(
                serde_json::to_value(framing.layout)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_default(),
            );
        }
        for id in [
            &idea.concept.bricks.style,
            &idea.concept.bricks.typography,
            &idea.concept.bricks.dressing,
            &idea.concept.bricks.background,
        ]
        .into_iter()
        .flatten()
        {
            if let Ok(Some(brick)) = crate::style_brick::get(conn, id) {
                parts.push(brick.name);
            }
        }
        if let Some(accent) = &idea.concept.accent {
            parts.push(accent.said());
        }
        if !parts.is_empty() {
            out.push_str(&format!("\n{}\n", parts.join(" · ")));
        }
        out.push('\n');
    }
    if !dropped.is_empty() {
        out.push_str("Left out, not in the workspace:\n");
        for one in dropped {
            out.push_str(&format!(
                "- idea {}: {} “{}”\n",
                one.idea, one.part, one.value
            ));
        }
    }
    out.trim_end().to_owned()
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

/// A proposal for the canon waiting to be kept, with what it says: the Canon
/// screen shows each beside the cards it touches (ADR 0043).
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct CanonProposal {
    pub message_id: String,
    pub chat_id: String,
    pub chat_title: Option<String>,
    /// The work the facts were read from, when they were.
    pub work_id: Option<String>,
    /// A description's text; the answer's prose for a package.
    pub body: String,
    pub proposal: Proposal,
    /// Every card of the canon the proposal touches, by id: the cards its
    /// facts land on or change, the ends of its relations, the card it
    /// describes. What the screen finds the proposals of an open card by.
    pub cards: Vec<String>,
    pub created_at: String,
}

/// Every proposal for the canon nobody has answered, oldest first.
pub fn pending_canon(conn: &Connection, profile_id: &str) -> Result<Vec<CanonProposal>> {
    let mut statement = conn.prepare(
        "SELECT m.id, m.chat_id, c.title, c.work_id, m.body, m.meta, m.created_at
           FROM chat_message m
           JOIN chat c ON c.id = m.chat_id
          WHERE c.profile_id = ?1 AND m.role = ?2
            AND json_extract(m.meta, '$.proposal.kind') IN ('canon', 'cardPrompt')
          ORDER BY m.created_at, m.rowid",
    )?;
    let rows = statement
        .query_map(params![profile_id, ASSISTANT], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut out = Vec::new();
    for (message_id, chat_id, chat_title, work_id, body, raw, created_at) in rows {
        let meta: Map<String, Value> = serde_json::from_str(&raw).unwrap_or_default();
        if meta.contains_key("applied") || meta.contains_key(DISMISSED) {
            continue;
        }
        let Some(proposal) = meta
            .get("proposal")
            .cloned()
            .and_then(|value| serde_json::from_value::<Proposal>(value).ok())
        else {
            continue;
        };
        let mut cards: Vec<String> = Vec::new();
        let mut touch = |id: &str| {
            if !id.is_empty() && !cards.iter().any(|known| known == id) {
                cards.push(id.to_owned());
            }
        };
        match &proposal {
            Proposal::Canon { package } => {
                for fact in &package.facts {
                    if let Some(card) = fact.card.as_deref() {
                        touch(card);
                    }
                }
                for link in &package.links {
                    touch(&link.from);
                    touch(&link.to);
                }
            }
            Proposal::CardPrompt { note_id, .. } => touch(note_id),
            _ => {}
        }
        out.push(CanonProposal {
            message_id,
            chat_id,
            chat_title,
            work_id,
            body,
            proposal,
            cards,
            created_at,
        });
    }
    Ok(out)
}

/// A proposal for the canon as a person reads it in a chat: the new cards,
/// the facts by card, the relations. The body of a message an agent's
/// proposal lands as, where there is no answer of the assistant's own to show.
pub fn render_canon(conn: &Connection, package: &crate::canon::proposal::Package) -> String {
    use crate::canon::proposal::FactChange;

    let name = |id: &str| -> String {
        if let Some(card) = package.cards.iter().find(|c| c.handle == id) {
            return card.title.clone();
        }
        crate::note::get(conn, id)
            .ok()
            .flatten()
            .and_then(|card| card.title)
            .unwrap_or_else(|| id.to_owned())
    };
    let mut out = String::new();
    if !package.cards.is_empty() {
        out.push_str("New cards:\n");
        for card in &package.cards {
            out.push_str(&format!("- {} ({})\n", card.title, card.kind));
        }
        out.push('\n');
    }
    if !package.facts.is_empty() {
        out.push_str("Facts:\n");
        for fact in &package.facts {
            let card = name(fact.card.as_deref().unwrap_or_default());
            match fact.change {
                FactChange::Add => out.push_str(&format!(
                    "- {card} · {}: {}\n",
                    fact.section.as_deref().unwrap_or_default(),
                    fact.body.as_deref().unwrap_or_default()
                )),
                FactChange::Refine => out.push_str(&format!(
                    "- {card}, refined: {}\n",
                    fact.body.as_deref().unwrap_or_default()
                )),
                FactChange::Retire => out.push_str(&format!(
                    "- {card}, retired: {}\n",
                    fact.reason.as_deref().unwrap_or_default()
                )),
            }
        }
        out.push('\n');
    }
    if !package.links.is_empty() {
        out.push_str("Relations:\n");
        for link in &package.links {
            out.push_str(&format!("- {} — {}", name(&link.from), name(&link.to)));
            if let Some(label) = &link.label {
                out.push_str(&format!(": {label}"));
            }
            out.push('\n');
        }
        out.push('\n');
    }
    if !package.pictures.is_empty() {
        out.push_str("Pictures:\n");
        for picture in &package.pictures {
            let file = std::path::Path::new(&picture.path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| picture.path.clone());
            out.push_str(&format!(
                "- {} · {}: {file}\n",
                picture.card_title,
                picture.role.as_deref().unwrap_or("reference")
            ));
        }
        out.push('\n');
    }
    if !package.descriptions.is_empty() {
        out.push_str("Descriptions for pictures:\n");
        for description in &package.descriptions {
            out.push_str(&format!(
                "- {}: {}\n",
                description.card_title, description.text
            ));
        }
    }
    out.trim_end().to_owned()
}
