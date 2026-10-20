//! A work's conception, as the assistant and a writer read it (v0.99, ADR
//! 0065).
//!
//! What a person prepares before a work is written - the brief in their
//! words, the idea it was refined into, the boards of candidates, the notes
//! and phrases attached - is read here into the sheets a template names:
//! `{conception}`, `{starred}`, `{notes}`, and `{order}`, the whole order a
//! writer works from. The order is one text whichever way it travels - into
//! the window's chat, to an agent over MCP (`brief`), copied out as markdown
//! - so the three cannot drift.

use rusqlite::Connection;

use crate::board::{self, BoardCard, Verdict, answer};
use crate::error::{Error, Result};
use crate::profile::config::Lens;
use crate::profile::config::{Conception, ProfileConfig};
use crate::work::Work;
use crate::work::version::{self, Version};

/// The brief as it stands: its newest revision. A brief is never a work's
/// current version, so the newest is the one being written.
pub fn brief(conn: &Connection, work: &Work, conception: &Conception) -> Result<Option<Version>> {
    Ok(version::latest(conn, &work.id, &conception.brief)?
        .filter(|brief| !brief.body.trim().is_empty()))
}

/// The idea the brief was refined into, when the kind names a field for it
/// and the work has words there.
pub fn idea(work: &Work, conception: &Conception) -> Option<String> {
    let field = conception.idea.as_deref()?;
    work.meta
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// The word the profile has for the idea's field, or "The idea".
fn idea_word(config: &ProfileConfig, conception: &Conception) -> String {
    conception
        .idea
        .as_deref()
        .and_then(|key| {
            config
                .work_meta_fields
                .iter()
                .find(|field| field.key == key)
        })
        .map_or_else(|| "The idea".to_owned(), |field| field.label.to_string())
}

/// The word the kind has for its brief's role, or "The brief".
fn brief_word(config: &ProfileConfig, work: &Work, conception: &Conception) -> String {
    config
        .vocabulary(&work.kind)
        .version_roles
        .iter()
        .find(|role| role.key == conception.brief)
        .map_or_else(|| "The brief".to_owned(), |role| role.label.to_string())
}

/// `{conception}`: the idea and the brief, each when written.
///
/// Refused when the work's kind is prepared on no conception, and when
/// neither says anything: an action that works from a conception has
/// nothing to work from, and an answer written from nothing would read as
/// one written from something.
pub fn sheet(conn: &Connection, work: &Work) -> Result<String> {
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let conception = config.require_conception(&work.kind)?;
    let idea = idea(work, conception);
    let brief = brief(conn, work, conception)?;
    if idea.is_none() && brief.is_none() {
        return Err(Error::refused("conception.empty").param("title", work.title.clone()));
    }
    let mut out = String::new();
    if let Some(idea) = idea {
        out.push_str(&format!("{}:\n{idea}\n\n", idea_word(&config, conception)));
    }
    if let Some(brief) = brief {
        out.push_str(&format!(
            "{}, in the author's words:\n{}\n",
            brief_word(&config, work, conception),
            brief.body.trim()
        ));
    }
    Ok(out.trim_end().to_owned())
}

/// `{starred}` and `{starred:<board>}`: what is starred on the work's boards,
/// or on one of them, board by board under its name.
pub fn starred(conn: &Connection, work: &Work, only: Option<&str>) -> Result<String> {
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let mut out = String::new();
    for board in config.boards_of(&work.kind) {
        if only.is_some_and(|key| key != board.key) {
            continue;
        }
        let cards: Vec<BoardCard> = board::for_board(conn, &work.id, &board.key)?
            .into_iter()
            .filter(|card| card.verdict == Some(Verdict::Star))
            .collect();
        if cards.is_empty() {
            continue;
        }
        out.push_str(&format!("{}, starred:\n", board.label));
        for card in &cards {
            out.push_str(&format!("- {}\n", answer::card_line(Some(board), card)));
        }
        out.push('\n');
    }
    if out.is_empty() {
        return Ok("(nothing is starred yet)".to_owned());
    }
    Ok(out.trim_end().to_owned())
}

/// `{notes}`: the notes attached to the work, each whole - not the phrases,
/// which `{lines}` reads with how they are sung, nor the cards of the canon,
/// which `{canon}` reads by its lens.
pub fn notes(conn: &Connection, work: &Work) -> Result<String> {
    let notes = crate::note::list(
        conn,
        &work.profile_id,
        &crate::note::NoteFilter {
            work_id: Some(work.id.clone()),
            canon: Some(false),
            line: Some(false),
            ..crate::note::NoteFilter::default()
        },
    )?;
    let written: Vec<String> = notes
        .iter()
        .filter(|note| !note.body.trim().is_empty())
        .map(|note| {
            match note
                .title
                .as_deref()
                .map(str::trim)
                .filter(|t| !t.is_empty())
            {
                Some(title) => format!("### {title}\n{}", note.body.trim()),
                None => note.body.trim().to_owned(),
            }
        })
        .collect();
    if written.is_empty() {
        return Ok("(no notes are attached to this work)".to_owned());
    }
    Ok(format!(
        "Notes the author attached to this work:\n\n{}",
        written.join("\n\n")
    ))
}

/// The role a work of this kind is written in - its lyrics, its text: the
/// first that is a draft of it and comments on nothing.
fn text_role(config: &ProfileConfig, work: &Work) -> Option<String> {
    config
        .vocabulary(&work.kind)
        .version_roles
        .iter()
        .find(|role| role.comments_on.is_none() && role.counts_as_a_version())
        .map(|role| role.label.to_string())
}

/// `{order}`: the whole order a writer works from, in markdown.
///
/// The title and what is to be written; the idea and the brief; the texts
/// the writer keeps to (a song's style prompt); the work's fields; each
/// board as its rule reads it; the notes and phrases attached; the channel's
/// voice and the canon the work is about; the register of what is worn out.
/// A part with nothing in it is left out. Refused, as `{conception}` is,
/// when the work is prepared on no conception or nothing of it is written -
/// an order of a title alone is not one.
pub fn order(conn: &Connection, work: &Work) -> Result<String> {
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let conception = config.require_conception(&work.kind)?;
    let idea = idea(work, conception);
    let brief = brief(conn, work, conception)?;
    if idea.is_none() && brief.is_none() {
        return Err(Error::refused("conception.empty").param("title", work.title.clone()));
    }
    let kind = config
        .kind(&work.kind)
        .map_or_else(|| work.kind.clone(), |known| known.label.to_string());

    let mut parts = vec![format!("# “{}”", work.title)];
    // No article: "a experiment" reads wrong, and the kind is a word of the
    // profile's, in whatever language it was written.
    parts.push(match text_role(&config, work) {
        Some(role) => format!(
            "To write: the {} of this {}.",
            role.to_lowercase(),
            kind.to_lowercase()
        ),
        None => format!("To write: this {}.", kind.to_lowercase()),
    });
    if let Some(idea) = idea {
        parts.push(format!("## {}\n\n{idea}", idea_word(&config, conception)));
    }
    if let Some(brief) = &brief {
        parts.push(format!(
            "## {}\n\n{}",
            brief_word(&config, work, conception),
            nested(brief.body.trim())
        ));
    }
    let roles = &config.vocabulary(&work.kind).version_roles;
    for key in &conception.with {
        let Some(text) =
            version::latest(conn, &work.id, key)?.filter(|v| !v.body.trim().is_empty())
        else {
            continue;
        };
        let word = roles
            .iter()
            .find(|role| role.key == *key)
            .map_or_else(|| key.clone(), |role| role.label.to_string());
        parts.push(format!("## {word}\n\n{}", nested(text.body.trim())));
    }
    let fields = fields(&config, work, conception);
    if !fields.is_empty() {
        parts.push(format!("## Fields\n\n{fields}"));
    }
    for board in config.boards_of(&work.kind) {
        let cards = board::for_board(conn, &work.id, &board.key)?;
        let (rule, given) = answer::for_writer(board, &cards);
        if given.is_empty() {
            continue;
        }
        let mut part = format!("## {}\n\n{rule}\n", board.label);
        for card in given {
            let star = if card.verdict == Some(Verdict::Star) {
                "★ "
            } else {
                ""
            };
            part.push_str(&format!(
                "\n- {star}{}",
                answer::card_line(Some(board), card)
            ));
        }
        parts.push(part);
    }
    let notes = notes(conn, work)?;
    if !notes.starts_with('(') {
        parts.push(format!("## Attached notes\n\n{notes}"));
    }
    let lines = crate::lines::sheet(conn, &work.profile_id, &work.id, None)?;
    if !lines.starts_with('(') {
        parts.push(format!("## Attached phrases\n\n{lines}"));
    }
    let canon = canon(conn, work, brief.as_ref())?;
    if !canon.is_empty() {
        parts.push(format!(
            "## The channel and the canon\n\n{}",
            nested(&canon)
        ));
    }
    let register = crate::register::sheet::register(conn, &work.profile_id, None)?;
    if !register.trim().is_empty() && !register.starts_with('(') {
        parts.push(format!("## The register\n\n{register}"));
    }
    Ok(parts.join("\n\n") + "\n")
}

/// The work's filled fields, the idea's own left out: it has a part of its
/// own above them.
fn fields(config: &ProfileConfig, work: &Work, conception: &Conception) -> String {
    let mut without = work.clone();
    if let Some(key) = &conception.idea {
        without.meta.remove(key);
    }
    let sheet = crate::assistant::prompt::fields_sheet(config, &without);
    if sheet.starts_with('(') {
        String::new()
    } else {
        sheet
    }
}

/// What a writer is told of the canon: the channel's card as the public
/// reads it - its voice, its bans - and the cards the work is about as a
/// work's text reads them: the ones that live at it, and the ones its brief
/// names, there being no text to find them in yet.
///
/// Not `{canon}`'s sheet: that one lists every card of the canon by name and
/// id, so an answer can name the card it means - a hundred and more lines a
/// writer has no use for (found by the live run of v0.99, where it was most
/// of a hundred-kilobyte order).
fn canon(conn: &Connection, work: &Work, brief: Option<&Version>) -> Result<String> {
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let text = brief
        .map(|version| version.body.as_str())
        .unwrap_or_default();
    let mut root = Vec::new();
    let mut about = Vec::new();
    for card in crate::canon::view::cards(
        conn,
        &work.profile_id,
        &crate::canon::view::CardFilter::default(),
    )? {
        if config.card_kind(&card.kind).is_some_and(|kind| kind.root) {
            root.push(crate::canon::view::render(conn, &card.id, Lens::Public)?);
            continue;
        }
        let named = card
            .title
            .iter()
            .chain(card.aliases.iter())
            .any(|name| crate::canon::appearances::names_word(text, name));
        if card.work_id.as_deref() == Some(work.id.as_str()) || named {
            about.push(crate::canon::view::render(conn, &card.id, Lens::Work)?);
        }
    }
    Ok(root
        .into_iter()
        .chain(about)
        .filter(|seen| !seen.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n"))
}

/// A markdown text set one heading level deeper, so its own headings stand
/// under the order's part rather than beside it: a brief's `## What it is
/// about` under `## Brief`. A fenced block is left as it is.
fn nested(text: &str) -> String {
    let mut fenced = false;
    text.lines()
        .map(|line| {
            if line.trim_start().starts_with("```") {
                fenced = !fenced;
            }
            if !fenced && line.starts_with('#') {
                format!("#{line}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{NewCard, Source};
    use crate::fixtures;

    fn scenario(conn: &Connection, work_id: &str, line: &str) -> BoardCard {
        crate::actions::board::create(
            conn,
            NewCard {
                work_id: work_id.to_owned(),
                board: "scenarios".into(),
                kind: "scenario".into(),
                topic: String::new(),
                position: None,
                source: Source::Ai,
                verdict: None,
                fields: [("line".to_owned(), line.to_owned())].into_iter().collect(),
            },
        )
        .unwrap()
    }

    /// The order is one text of everything prepared: the idea, the brief,
    /// the style the text keeps to, the scenarios by the star rule, the notes
    /// and the channel's voice - and a turned-down scenario never in it.
    #[test]
    fn the_order_holds_what_was_prepared_and_not_what_was_turned_down() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        crate::actions::work::update(
            &conn,
            &song.id,
            crate::work::WorkPatch {
                meta: Some(
                    [(
                        "premise".to_owned(),
                        serde_json::json!("Telling a vigil from a habit."),
                    )]
                    .into_iter()
                    .collect(),
                ),
                ..crate::work::WorkPatch::default()
            },
        )
        .unwrap();
        fixtures::version(
            &conn,
            &song.id,
            "brief",
            "## What it is about\nthe harbour at night",
        );
        fixtures::version(&conn, &song.id, "style", "slow post-rock, cello");
        let keeper = scenario(&conn, &song.id, "A keeper counts the ships");
        let boat = scenario(&conn, &song.id, "A boat repainted every spring");
        crate::actions::board::judge(&conn, &boat.id, Some(Verdict::Rejected)).unwrap();
        crate::actions::note::create(
            &conn,
            crate::note::NewNote {
                body: "the lamp was my grandfather's".into(),
                work_id: Some(song.id.clone()),
                ..crate::note::NewNote::default()
            },
        )
        .unwrap();
        let channel = fixtures::card(&conn, &profile_id, "channel", "The channel");
        fixtures::fact(&conn, &channel.id, "voice", "Quiet, never preachy");

        let work = crate::work::get(&conn, &song.id).unwrap().unwrap();
        let order = order(&conn, &work).unwrap();
        for said in [
            "# “Harbour lights”",
            "Telling a vigil from a habit.",
            "the harbour at night",
            "slow post-rock, cello",
            "A keeper counts the ships",
            "Nothing is starred",
            "the lamp was my grandfather's",
            "Quiet, never preachy",
        ] {
            assert!(order.contains(said), "the order lacks {said:?}:\n{order}");
        }
        assert!(
            !order.contains("A boat repainted"),
            "a turned-down card went:\n{order}"
        );
        assert!(
            order.contains("### What it is about"),
            "the brief's headings stand under its part:\n{order}"
        );
        assert!(
            !order.contains("Every card, by name"),
            "the canon's index of names went:\n{order}"
        );

        crate::actions::board::judge(&conn, &keeper.id, Some(Verdict::Star)).unwrap();
        let order = super::order(&conn, &work).unwrap();
        assert!(order.contains("Write by this one."), "{order}");
        assert!(order.contains("★ "), "{order}");
    }

    /// A conception with nothing written is refused rather than sent as an
    /// order of a title alone, and a kind with no conception has none.
    #[test]
    fn an_order_of_nothing_is_refused() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let refused = order(&conn, &song).unwrap_err();
        assert_eq!(refused.refusal().unwrap().code, "conception.empty");
        assert_eq!(
            sheet(&conn, &song).unwrap_err().refusal().unwrap().code,
            "conception.empty"
        );
        let audio = fixtures::work(&conn, &profile_id, "audio", "Harbour lights (audio)");
        assert_eq!(
            order(&conn, &audio).unwrap_err().refusal().unwrap().code,
            "conception.noConception"
        );
    }

    /// A brief is written beside the text, never as it: making one current
    /// is refused, and a version of it made "current" on creation is not.
    #[test]
    fn a_brief_is_never_the_works_current_text() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let lyrics = fixtures::version(&conn, &song.id, "lyrics", "[Verse 1]\nthe lamp");
        let brief = fixtures::version(&conn, &song.id, "brief", "## What it is about");
        let style = fixtures::version(&conn, &song.id, "style", "slow post-rock");
        let work = crate::work::get(&conn, &song.id).unwrap().unwrap();
        assert_eq!(work.current_version_id.as_deref(), Some(lyrics.id.as_str()));
        for not_a_draft in [&brief, &style] {
            let refused =
                crate::actions::work::set_current_version(&conn, &song.id, &not_a_draft.id)
                    .unwrap_err();
            assert_eq!(refused.refusal().unwrap().code, "version.notADraft");
        }
    }

    /// A workspace whose current version is no draft - a style prompt, as an
    /// older build let one be - points at its newest draft once opened, or at
    /// nothing when it has none.
    #[test]
    fn a_current_version_that_is_no_draft_is_settled() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        let lyrics = fixtures::version(&conn, &song.id, "lyrics", "[Verse 1]\nthe lamp");
        let style = fixtures::version(&conn, &song.id, "style", "slow post-rock");
        let lab = fixtures::work(&conn, &profile_id, "experiment", "Breaks");
        let brief = fixtures::version(&conn, &lab.id, "brief", "## Direction");
        for (work, version) in [(&song.id, &style.id), (&lab.id, &brief.id)] {
            conn.execute(
                "UPDATE work SET current_version_id = ?2 WHERE id = ?1",
                rusqlite::params![work, version],
            )
            .unwrap();
        }
        assert_eq!(version::settle_current(&conn).unwrap(), 2);
        let current = |id: &str| {
            crate::work::get(&conn, id)
                .unwrap()
                .unwrap()
                .current_version_id
        };
        assert_eq!(current(&song.id).as_deref(), Some(lyrics.id.as_str()));
        assert_eq!(current(&lab.id), None);
        assert_eq!(
            version::settle_current(&conn).unwrap(),
            0,
            "settled once, for good"
        );
    }
}
