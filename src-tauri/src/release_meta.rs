//! What a release says about itself: the title it goes out under, the text
//! under it, the words it is found by, the comment pinned beneath it.
//!
//! The fields are the craft's, named by the release kind in the profile
//! ([`ReleaseField`]); the values live in `release.meta` under the field keys.
//! There is no table here and no migration, deliberately: `release.meta` is
//! already a map on the row, already patched through `release::update`, which
//! the operation log and undo already know how to replay and reverse. A
//! second table would have been a second truth about one release, and a
//! second thing to keep in step with it.
//!
//! Generating is rendering a template against the work, with the same
//! renderer an AI action uses -- see [`crate::assistant::prompt`]. A release
//! title that reads `{title}` and an action that reads `{title}` are reading
//! the same word, because there is one vocabulary and one renderer, not two
//! that look alike.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::assistant::prompt;
use crate::assistant::proposal::Proposal;
use crate::error::{Error, Result};
use crate::profile::config::{Label, ReleaseField, ReleaseFieldType};
use crate::release;
use crate::work;

/// One field of a release, as a screen needs it: what it is, what is written
/// in it, and whether the profile could write it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Field {
    pub key: String,
    pub label: Label,
    #[serde(rename = "type")]
    pub field_type: ReleaseFieldType,
    /// What is stored on the release now. Empty is empty -- a field never
    /// written and a field written blank are the same thing to a reader, and
    /// telling them apart would only let one of them hide.
    pub value: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<Label>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Whether the profile can fill this field on its own.
    pub has_template: bool,
    /// The tail the value always ends with: " (audio)" on an audio track's
    /// title on a video platform. Kept by the release whoever writes the
    /// field, so a screen only shows it.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

/// Every field of a release, in the order the profile lists them.
///
/// A release whose kind the profile no longer has gets an empty list rather
/// than an error: the release is still a real row with a real date, and the
/// tab around it must still draw.
pub fn fields(conn: &Connection, release_id: &str) -> Result<Vec<Field>> {
    let (release, defined) = definition(conn, release_id)?;
    Ok(defined
        .iter()
        .map(|field| Field {
            key: field.key.clone(),
            label: field.label.clone(),
            field_type: field.field_type,
            value: stored(&release.meta, &field.key),
            hint: field.hint.clone(),
            limit: field.limit,
            has_template: field.template().is_some(),
            suffix: field.suffix().map(str::to_owned),
        })
        .collect())
}

/// Where a release stands, as far as the tails of its fields are concerned:
/// whether it went out, and the day it goes out on.
#[derive(Debug, Clone, Copy)]
pub struct Standing<'a> {
    pub released: bool,
    /// The day it went out, or is booked for; None when it holds no day.
    pub day: Option<&'a str>,
    /// The moment of the write: its year is the year of a premiere that
    /// holds no day yet.
    pub at: &'a str,
}

impl<'a> Standing<'a> {
    /// A release as it is stored, at the moment `at`.
    pub fn of(release: &'a release::Release, at: &'a str) -> Self {
        Self {
            released: release.status == release::RELEASED,
            day: release
                .released_at
                .as_deref()
                .or(release.scheduled_at.as_deref()),
            at,
        }
    }
}

/// `meta` as a release through `door` keeps it: every field of the door that
/// keeps a tail ends with the right one (v0.90) - the premiere's, with its
/// year, when the release opens its song, and the plain one otherwise
/// (v0.100.1, ADR 0067). When the release moves to `door` from another place
/// before it went out, the tails of the old door that the new one does not
/// keep come off: a tail belongs to the place - a track's title on the
/// streaming services is not "(audio)" there. A release that went out keeps
/// the tail it went out under: that title is on the platform already.
///
/// Called by the release itself on every write of its fields, its door or
/// its day (`release::create_minted`, `release::update_at`, and [`settle`]
/// after the rest), so a title typed by hand, generated, proposed by an agent
/// or replayed from the log ends the same way.
pub fn keep_suffixes(
    conn: &Connection,
    work_id: &str,
    from: Option<&str>,
    door: &str,
    standing: Standing<'_>,
    meta: &mut Map<String, Value>,
) -> Result<()> {
    // A tail ends written words: with none, there is nothing to read the
    // profile - a large document - for.
    if !says_anything(meta) {
        return Ok(());
    }
    let Some(work) = work::get(conn, work_id)? else {
        return Ok(());
    };
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    keep_tails(conn, &config, &work, from, door, standing, meta)
}

/// Whether a release's meta holds any words a tail could end.
fn says_anything(meta: &Map<String, Value>) -> bool {
    meta.values()
        .any(|value| value.as_str().is_some_and(|text| !text.trim().is_empty()))
}

/// [`keep_suffixes`] with the work and its profile already read: the profile
/// is a large document, and [`settle`] walks a whole family with one.
fn keep_tails(
    conn: &Connection,
    config: &crate::profile::config::ProfileConfig,
    work: &work::Work,
    from: Option<&str>,
    door: &str,
    standing: Standing<'_>,
    meta: &mut Map<String, Value>,
) -> Result<()> {
    let doors = &config.vocabulary(&work.kind).release_kinds;
    let fields_of = |key: &str| {
        doors
            .iter()
            .find(|candidate| candidate.key == key)
            .map(|found| found.fields.as_slice())
            .unwrap_or_default()
    };
    let now = fields_of(door);
    if let Some(from) = from.filter(|from| *from != door) {
        for field in fields_of(from).iter().filter(|field| field.keeps_a_tail()) {
            let kept_here = now.iter().any(|other| {
                other.key == field.key
                    && other.suffix() == field.suffix()
                    && other.premiere == field.premiere
            });
            if kept_here {
                continue;
            }
            if let Some(Value::String(text)) = meta.get_mut(&field.key) {
                *text = field.without_tail(text).to_owned();
            }
        }
    }
    let tailed: Vec<&ReleaseField> = now.iter().filter(|field| field.keeps_a_tail()).collect();
    if tailed.is_empty() {
        return Ok(());
    }
    let opens = if tailed.iter().any(|field| field.premiere.is_some()) {
        opens_its_song(conn, config, work, standing)?
    } else {
        false
    };
    for field in tailed {
        let Some(Value::String(text)) = meta.get_mut(&field.key) else {
            continue;
        };
        let written = text.trim_end();
        let bare = field.without_tail(written).to_owned();
        if standing.released && bare.len() != written.len() {
            continue;
        }
        let tail = opens
            .then(|| field.premiere_tail(year_of(standing), &bare))
            .flatten()
            .or_else(|| field.suffix().map(str::to_owned));
        *text = match tail {
            Some(tail) => field.with_tail(&bare, &tail),
            None => bare,
        };
    }
    Ok(())
}

/// The year a release goes out in: its day's, or the moment's when it holds
/// none yet.
fn year_of(standing: Standing<'_>) -> i32 {
    standing
        .day
        .unwrap_or(standing.at)
        .get(..4)
        .and_then(|year| year.parse().ok())
        .unwrap_or_default()
}

/// Whether the release of `work` opens its song: nothing else made from what
/// it is all made from has gone out before it, or is booked for an earlier
/// day. An excerpt never opens one and never stands in the way - a teaser cut
/// from the track is not its premiere. What went out while this still waits
/// came first, whatever day this one was booked for; otherwise what holds no
/// day is not earlier than anything, and one day is a tie that both open. A
/// work made from nothing opens itself. A work has one release (ADR 0051),
/// so the work names it.
fn opens_its_song(
    conn: &Connection,
    config: &crate::profile::config::ProfileConfig,
    work: &work::Work,
    standing: Standing<'_>,
) -> Result<bool> {
    if config.vocabulary(&work.kind).excerpt {
        return Ok(false);
    }
    let Some(origin) = crate::publication::origin(conn, config, &work.id)? else {
        return Ok(true);
    };
    for (id, _) in crate::link::descendants(conn, &origin.id)? {
        if id == work.id {
            continue;
        }
        let Some(other) = work::get(conn, &id)? else {
            continue;
        };
        if config.vocabulary(&other.kind).excerpt {
            continue;
        }
        let Some(theirs) = release::of_work(conn, &id)? else {
            continue;
        };
        let them = Standing::of(&theirs, standing.at);
        let out_while_this_waits = them.released && !standing.released;
        let booked_earlier = match (them.day, standing.day) {
            (Some(their_day), Some(own_day)) => day_of(their_day) < day_of(own_day),
            _ => false,
        };
        if out_while_this_waits || booked_earlier {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Settle the tails of every release of the song `work_id` belongs to - its
/// own and its siblings', all made from the same origin - after a write that
/// may have changed which of them opens it: a day booked or moved, a release
/// made, marked out or taken back. `except` is the release the write itself
/// already settled. What went out keeps its tails; what changes is written
/// straight to the row, as the release's own derived word, and every writer
/// that can change it calls this, so a replay or an undo of the write
/// settles the same.
pub fn settle(conn: &Connection, work_id: &str, except: Option<&str>, at: &str) -> Result<()> {
    // First the releases that could change at all - not out yet, with words
    // a tail could end - found through the links alone, everything made from
    // the furthest work this one is made from. With none, there is nothing
    // to read the profile, a large document, for.
    let top = crate::link::ancestors(conn, work_id)?
        .pop()
        .unwrap_or_else(|| work_id.to_owned());
    let mut related: Vec<String> = crate::link::descendants(conn, &top)?
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    related.push(top);
    let mut waiting = Vec::new();
    for id in &related {
        if let Some(found) = release::of_work(conn, id)?
            && Some(found.id.as_str()) != except
            && found.status != release::RELEASED
            && says_anything(&found.meta)
        {
            waiting.push(found);
        }
    }
    if waiting.is_empty() {
        return Ok(());
    }
    let Some(work) = work::get(conn, work_id)? else {
        return Ok(());
    };
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    // Only a premiere's tail depends on where a release stands; a profile
    // that names none has nothing here to settle.
    let premieres = |kind: &str, door: &str| {
        config
            .vocabulary(kind)
            .release_kinds
            .iter()
            .filter(|candidate| candidate.key == door)
            .flat_map(|candidate| &candidate.fields)
            .any(|field| field.premiere.is_some())
    };
    let anywhere = config.work_kinds.iter().any(|kind| {
        kind.release_kinds
            .iter()
            .any(|door| premieres(&kind.key, &door.key))
    });
    if !anywhere {
        return Ok(());
    }
    let origin = crate::publication::origin(conn, &config, &work.id)?;
    let family: Vec<String> = match &origin {
        Some(origin) => crate::link::descendants(conn, &origin.id)?
            .into_iter()
            .map(|(id, _)| id)
            .collect(),
        None => vec![work.id.clone()],
    };
    for found in waiting {
        if !family.contains(&found.work_id) {
            continue;
        }
        let Some(made) = work::get(conn, &found.work_id)? else {
            continue;
        };
        if !premieres(&made.kind, &found.kind) {
            continue;
        }
        let mut meta = found.meta.clone();
        keep_tails(
            conn,
            &config,
            &made,
            None,
            &found.kind,
            Standing::of(&found, at),
            &mut meta,
        )?;
        if meta != found.meta {
            conn.execute(
                "UPDATE release SET meta = ?2, updated_at = ?3 WHERE id = ?1",
                rusqlite::params![found.id, Value::Object(meta).to_string(), at],
            )?;
        }
    }
    Ok(())
}

/// The release, and the fields its kind declares.
fn definition(
    conn: &Connection,
    release_id: &str,
) -> Result<(release::Release, Vec<ReleaseField>)> {
    let release =
        release::get(conn, release_id)?.ok_or_else(|| Error::not_found("release", release_id))?;
    let work = work::get(conn, &release.work_id)?
        .ok_or_else(|| Error::not_found("work", &release.work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let defined = config
        .vocabulary(&work.kind)
        .release_kinds
        .iter()
        .find(|kind| kind.key == release.kind)
        .map(|kind| kind.fields.clone())
        .unwrap_or_default();
    Ok((release, defined))
}

/// What the release holds under a key, as text.
///
/// The map is JSON, so a value may have been written as a number or a list by
/// something other than the screen -- by an agent's package, by a hand-edited
/// export. Each reads back as the text it obviously is rather than as
/// `["a","b"]`: the field is text on every platform it is going to, and a
/// reader who sees JSON in a description box learns nothing useful from it.
fn stored(meta: &Map<String, Value>, key: &str) -> String {
    match meta.get(key) {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join(", "),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

/// What a generation produced, field by field.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Generated {
    /// The values, by field key, ready to be written.
    pub values: Map<String, Value>,
    /// Fields that could not be rendered, and why: a role the work has no
    /// version in yet, most often. Reported rather than silently skipped --
    /// "the description is empty" and "the description could not be written
    /// because there are no lyrics" are different problems, and only one of
    /// them is fixed by typing in the box.
    pub refused: Vec<Refusal>,
}

/// A field the profile could not fill, and the reason in the renderer's own
/// words.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Refusal {
    pub key: String,
    pub label: Label,
    pub reason: String,
}

/// Render every templated field of a release against its work.
///
/// Fields without a template are left out entirely rather than rendered
/// empty: they are the ones only ever typed by hand, and overwriting what
/// someone typed with nothing is the one thing a generate button must never
/// do.
///
/// One field failing does not take the rest with it. A video whose lyrics are
/// not written yet should still get its title and its tags, and be told what
/// the description is waiting for.
pub fn generate(conn: &Connection, release_id: &str) -> Result<Generated> {
    let (release, defined) = definition(conn, release_id)?;
    let mut values = Map::new();
    let mut refused = Vec::new();
    let forms = sung_forms_of(conn, &release.work_id)?;

    for field in &defined {
        let Some(template) = field.template() else {
            continue;
        };
        match prompt::for_work(conn, &release.work_id, template, prompt::Context::default()) {
            Ok(rendered) => {
                // A public text: the lyric a description reads is the one
                // the audience reads, without the singer's marks (ADR 0053).
                let public = crate::words::sung::clean(&rendered, &forms);
                values.insert(field.key.clone(), Value::String(tidy(&public, field)));
            }
            Err(cause) => refused.push(Refusal {
                key: field.key.clone(),
                label: field.label.clone(),
                reason: cause.to_string(),
            }),
        }
    }

    Ok(Generated { values, refused })
}

/// The owner's ways of singing, for the workspace a work is in: what a public
/// text made from a sung one is cleaned by.
fn sung_forms_of(conn: &Connection, work_id: &str) -> Result<Vec<crate::register::Sung>> {
    let profile_id: String = conn.query_row(
        "SELECT profile_id FROM work WHERE id = ?1",
        rusqlite::params![work_id],
        |row| row.get(0),
    )?;
    crate::register::sung_forms(conn, &profile_id)
}

/// A proposal of what a release goes out under, as the public reads it: an
/// answer or an agent that quoted the lyric quoted it with the singer's
/// marks, and those come off before a person sees what would be written
/// (ADR 0053). Any other proposal is returned as it is.
pub fn public(conn: &Connection, proposal: Proposal) -> Result<Proposal> {
    let Proposal::Release {
        release_id,
        fields,
        unknown,
    } = proposal
    else {
        return Ok(proposal);
    };
    let release =
        release::get(conn, &release_id)?.ok_or_else(|| Error::not_found("release", &release_id))?;
    let forms = sung_forms_of(conn, &release.work_id)?;
    let fields = fields
        .into_iter()
        .map(|(key, value)| match value {
            Value::String(text) => (key, Value::String(crate::words::sung::clean(&text, &forms))),
            other => (key, other),
        })
        .collect();
    Ok(Proposal::Release {
        release_id,
        fields,
        unknown,
    })
}

/// The rendered text as the field's shape wants it.
///
/// A line is a line: a template that reads `{title}` and a work whose title
/// someone pasted a newline into would otherwise put a second line in a
/// one-line box, where it is invisible until the platform rejects it. Tags
/// lose their blanks and their repeats, keeping the first spelling of each,
/// because a list holding `lighthouse, Lighthouse, lighthouse` is a list
/// someone will have to clean by hand.
pub fn tidy(rendered: &str, field: &ReleaseField) -> String {
    match field.field_type {
        ReleaseFieldType::Line => rendered.split_whitespace().collect::<Vec<_>>().join(" "),
        ReleaseFieldType::Text => rendered.trim().to_owned(),
        ReleaseFieldType::Tags => {
            let mut kept: Vec<String> = Vec::new();
            for tag in rendered.split(',') {
                let tag = tag.trim();
                if tag.is_empty() {
                    continue;
                }
                if kept.iter().any(|seen| seen.eq_ignore_ascii_case(tag)) {
                    continue;
                }
                kept.push(tag.to_owned());
            }
            kept.join(", ")
        }
    }
}

/// The values a release would be patched with, its own meta kept underneath.
///
/// Generation replaces the fields it produced and touches nothing else: a
/// release may carry meta no field of the profile describes -- written by an
/// older profile, by an agent, by hand -- and a generate button is not a
/// reason to lose it.
pub fn merged(existing: &Map<String, Value>, values: &Map<String, Value>) -> Map<String, Value> {
    let mut merged = existing.clone();
    for (key, value) in values {
        merged.insert(key.clone(), value.clone());
    }
    merged
}

/// The release an action is about, for `{release}`: where it goes out, when,
/// and every field its kind asks for with what is written there now.
///
/// What is written is said to be an earlier take (v0.100.3). The action is
/// run on a release that has words to get other words: shown as the fields'
/// contents and nothing more, they were read as a draft to polish, and a run
/// a week later came back with the same description, three words changed.
pub fn sheet(conn: &Connection, release_id: &str) -> Result<String> {
    let (release, defined) = definition(conn, release_id)?;
    let work = work::get(conn, &release.work_id)?
        .ok_or_else(|| Error::not_found("work", &release.work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let door = config
        .vocabulary(&work.kind)
        .release_kinds
        .iter()
        .find(|kind| kind.key == release.kind)
        .map_or(release.kind.clone(), |kind| kind.label.to_string());
    let when = match (&release.released_at, &release.scheduled_at) {
        (Some(out), _) => format!("went out {}", day_of(out)),
        (None, Some(day)) => format!("planned for {day}"),
        (None, None) => "not planned for a day yet".to_owned(),
    };
    let mut out = format!("The release: {door}, {when}.\n");
    let written = defined
        .iter()
        .any(|field| !stored(&release.meta, &field.key).trim().is_empty());
    if written {
        out.push_str(
            "\nWhat the fields hold now is an earlier take, not a draft to polish: \
             the author asks for another one. It is shown so that nothing of it comes \
             back.\n",
        );
    }
    for field in &defined {
        let value = stored(&release.meta, &field.key);
        if value.trim().is_empty() {
            out.push_str(&format!("\n{} (`{}`): (empty)", field.label, field.key));
        } else {
            out.push_str(&format!(
                "\n{} (`{}`):\n{}",
                field.label,
                field.key,
                value.trim()
            ));
        }
        out.push('\n');
    }
    Ok(out.trim_end().to_owned())
}

/// A proposal for what a release goes out under that still waits for the
/// person: from the release's own action, the fields that were already
/// started; from an agent outside the window, all of them.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct ReleaseProposal {
    pub message_id: String,
    pub chat_id: String,
    /// Who proposed it, when it was an agent outside the window.
    pub client: Option<String>,
    pub created_at: String,
    /// The fields it would change, each beside what is written now.
    pub fields: Vec<ProposedField>,
}

/// One field of a waiting proposal.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct ProposedField {
    pub key: String,
    pub label: Label,
    pub proposed: String,
    pub current: String,
}

/// Every proposal for a release that still waits, oldest first, with only the
/// fields it would actually change: one the application already wrote on
/// arrival is left out, and so is one whose text is what the field already
/// says. A proposal with nothing left to change is not listed.
///
/// A proposed text is shown, and compared, as the field would keep it: with
/// the tail the place adds - "(audio)", a premiere and its year - which a
/// model is told never to write. Read raw, a title the model wrote right
/// stood as a change beside the same title with its tail (v0.100.3).
pub fn pending(conn: &Connection, release_id: &str) -> Result<Vec<ReleaseProposal>> {
    let ids: Vec<String> = conn
        .prepare(
            "SELECT id FROM chat_message
              WHERE json_extract(meta, '$.proposal.kind') = 'release'
                AND json_extract(meta, '$.proposal.release_id') = ?1
              ORDER BY created_at, rowid",
        )?
        .query_map(rusqlite::params![release_id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let current = fields(conn, release_id)?;
    let (release, _) = definition(conn, release_id)?;
    let at = crate::time::now();
    let mut found = Vec::new();
    for id in ids {
        let Some(message) = crate::assistant::message(conn, &id)? else {
            continue;
        };
        if !crate::assistant::apply::is_pending(&message) {
            continue;
        }
        let Ok(crate::assistant::proposal::Proposal::Release {
            fields: proposed, ..
        }) = crate::actions::proposal::stored_proposal(&message.meta)
        else {
            continue;
        };
        let filled = crate::actions::proposal::filled_of(&message.meta);
        let mut kept: Map<String, Value> = proposed
            .iter()
            .filter(|(_, value)| value.is_string())
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        keep_suffixes(
            conn,
            &release.work_id,
            None,
            &release.kind,
            Standing::of(&release, &at),
            &mut kept,
        )?;
        let changes: Vec<ProposedField> = current
            .iter()
            .filter(|field| !filled.contains(&field.key))
            .filter_map(|field| {
                let text = kept.get(&field.key)?.as_str()?;
                (text.trim() != field.value.trim()).then(|| ProposedField {
                    key: field.key.clone(),
                    label: field.label.clone(),
                    proposed: text.to_owned(),
                    current: field.value.clone(),
                })
            })
            .collect();
        if changes.is_empty() {
            continue;
        }
        found.push(ReleaseProposal {
            message_id: message.id.clone(),
            chat_id: message.chat_id.clone(),
            client: message
                .meta
                .get("client")
                .and_then(Value::as_str)
                .map(str::to_owned),
            created_at: message.created_at.clone(),
            fields: changes,
        });
    }
    Ok(found)
}

/// The fields a release's kind declares, whatever is written in them.
pub fn declared(conn: &Connection, release_id: &str) -> Result<Vec<ReleaseField>> {
    Ok(definition(conn, release_id)?.1)
}

/// How many earlier releases `{releases}` reads: enough to see the habits a
/// channel repeats - the sign-off, the question at the end - and few enough
/// not to bury the work the text is about.
pub const HISTORY: usize = 8;

/// What went out before, for `{releases}`: the latest releases of the
/// workspace through doors with fields, newest first, each with what it went
/// out under. Read so a new description is not built the way the last few
/// were. Ordered by the day each goes out on, whether it went out or is
/// booked: a text written for next week is as recent as one that went out
/// yesterday, and one not yet marked as out is no less written (v0.100.1).
/// The release the action is about is left out.
pub fn history(conn: &Connection, release_id: &str, limit: usize) -> Result<String> {
    let (release, _) = definition(conn, release_id)?;
    let work = work::get(conn, &release.work_id)?
        .ok_or_else(|| Error::not_found("work", &release.work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let mut statement = conn.prepare(
        "SELECT r.id FROM release r JOIN work w ON w.id = r.work_id
          WHERE w.profile_id = ?1 AND r.id <> ?2 AND r.meta <> '{}'
            AND (r.released_at IS NOT NULL OR r.scheduled_at IS NOT NULL)
          ORDER BY substr(coalesce(r.released_at, r.scheduled_at), 1, 10) DESC, r.rowid DESC",
    )?;
    let ids: Vec<String> = statement
        .query_map(rusqlite::params![work.profile_id, release.id], |row| {
            row.get(0)
        })?
        .collect::<rusqlite::Result<_>>()?;

    let mut entries = Vec::new();
    for id in ids {
        if entries.len() >= limit {
            break;
        }
        let Some(earlier) = release::get(conn, &id)? else {
            continue;
        };
        let Some(earlier_work) = work::get(conn, &earlier.work_id)? else {
            continue;
        };
        let Some(door) = config
            .vocabulary(&earlier_work.kind)
            .release_kinds
            .iter()
            .find(|kind| kind.key == earlier.kind)
        else {
            continue;
        };
        let written: Vec<String> = door
            .fields
            .iter()
            .filter_map(|field| {
                let value = stored(&earlier.meta, &field.key);
                (!value.trim().is_empty()).then(|| format!("{}:\n{}", field.label, value.trim()))
            })
            .collect();
        if written.is_empty() {
            continue;
        }
        let when = match (&earlier.released_at, &earlier.scheduled_at) {
            (Some(out), _) => format!("went out {}", day_of(out)),
            (None, Some(day)) => format!("booked for {day}"),
            (None, None) => String::new(),
        };
        entries.push(format!(
            "“{}” ({}, {when})\n{}",
            earlier_work.title,
            door.label,
            written.join("\n\n")
        ));
    }
    if entries.is_empty() {
        return Ok("(nothing has gone out yet)".to_owned());
    }
    Ok(entries.join("\n\n---\n\n"))
}

/// The day of a moment, as a person dates a release.
fn day_of(moment: &str) -> String {
    moment.get(..10).unwrap_or(moment).to_owned()
}

/// How full a release's metadata is, for a screen that has to say whether
/// this one is ready to go out.
///
/// Counted over the fields the kind declares, so a kind that declares none is
/// complete rather than empty: nothing was asked of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Fullness {
    pub written: usize,
    pub total: usize,
}

impl Fullness {
    pub fn complete(self) -> bool {
        self.written == self.total
    }
}

/// What of a release's metadata is written.
pub fn fullness(fields: &[Field]) -> Fullness {
    Fullness {
        written: fields
            .iter()
            .filter(|field| !field.value.trim().is_empty())
            .count(),
        total: fields.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::release::NewRelease;
    use crate::work::{self, NewWork, version};
    use serde_json::json;

    /// The lyric an audio's description reads is the public's: the singer's
    /// marks come off and the owner's respellings go back (ADR 0053). The
    /// title reads the song's name, not the publication's (v0.90).
    #[test]
    fn a_generated_field_reads_the_song_without_the_singers_marks() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        fixtures::version(&conn, &song.id, "lyrics", "В МарсЭль, где пульсАр");
        crate::register::create(
            &conn,
            &profile_id,
            crate::register::NewTerm {
                sung: vec![crate::register::Sung {
                    written: "Марсель".into(),
                    sung: "МарсЭль".into(),
                }],
                ..crate::register::NewTerm::unspent("Марсель")
            },
        )
        .unwrap();
        let made =
            crate::actions::work::derive(&conn, &song.id, "audio", None, None, None).unwrap();

        let generated = generate(&conn, made.release_id.as_deref().unwrap()).unwrap();

        assert_eq!(generated.values["description"], "В Марсель, где пульсар");
        assert_eq!(
            generated.values["title"], "Tide",
            "the song's name, not \"Tide (audio)\""
        );
    }

    /// An audio track made from `song`, through its first door.
    fn audio_of(conn: &Connection, song_id: &str) -> String {
        made_of(conn, song_id, "audio")
    }

    fn made_of(conn: &Connection, source_id: &str, kind: &str) -> String {
        crate::actions::work::derive(conn, source_id, kind, None, None, None)
            .unwrap()
            .release_id
            .unwrap()
    }

    fn title_of(conn: &Connection, release_id: &str) -> String {
        stored(
            &release::get(conn, release_id).unwrap().unwrap().meta,
            "title",
        )
    }

    fn write_title(conn: &Connection, release_id: &str, title: &str) {
        crate::actions::release::set_fields(
            conn,
            release_id,
            &std::collections::BTreeMap::from([("title".to_owned(), title.to_owned())]),
        )
        .unwrap();
    }

    /// A song premieres with the first of its publications to go out: the
    /// audio that opens it carries the premiere and its year, in the language
    /// of its title, whatever tail the writer typed (ADR 0067).
    #[test]
    fn a_song_premieres_with_the_first_publication_to_go_out() {
        let (conn, profile_id) = fixtures::workspace();
        let tide = fixtures::song(&conn, &profile_id, "Tide");
        let audio = audio_of(&conn, &tide.id);
        release::schedule(&conn, &audio, "2026-11-04").unwrap();

        write_title(&conn, &audio, "Tide");
        assert_eq!(title_of(&conn, &audio), "Tide (Song premiere 2026)");

        let tide_ru = fixtures::song(&conn, &profile_id, "Прилив");
        let audio = audio_of(&conn, &tide_ru.id);
        release::schedule(&conn, &audio, "2027-01-02").unwrap();
        write_title(&conn, &audio, "Прилив (audio)");
        assert_eq!(
            title_of(&conn, &audio),
            "Прилив (Премьера песни 2027)",
            "the plain tail a writer typed is taken off, and the premiere speaks the title's language"
        );
    }

    /// An answer waiting in a chat on the release's work.
    fn answered(conn: &Connection, profile_id: &str, release_id: &str, fields: Value) {
        let release = release::get(conn, release_id).unwrap().unwrap();
        let chat = crate::assistant::create(
            conn,
            profile_id,
            crate::assistant::NewChat {
                work_id: Some(release.work_id),
                ..crate::assistant::NewChat::default()
            },
        )
        .unwrap();
        let proposal = Proposal::Release {
            release_id: release_id.to_owned(),
            fields: fields.as_object().cloned().unwrap(),
            unknown: Vec::new(),
        };
        let mut meta = Map::new();
        meta.insert("proposal".into(), serde_json::to_value(&proposal).unwrap());
        crate::assistant::append(
            conn,
            &chat.id,
            crate::assistant::ASSISTANT,
            "the answer",
            meta,
        )
        .unwrap();
    }

    /// A model is told never to write the tail; the field always ends with
    /// it. Compared raw, the title it wrote right stood as a change beside
    /// the same title with its premiere (v0.100.3).
    #[test]
    fn a_proposal_is_shown_and_compared_with_the_tail_the_place_adds() {
        let (conn, profile_id) = fixtures::workspace();
        let tide = fixtures::song(&conn, &profile_id, "Tide");
        let audio = audio_of(&conn, &tide.id);
        release::schedule(&conn, &audio, "2026-11-04").unwrap();
        crate::actions::release::set_fields(
            &conn,
            &audio,
            &std::collections::BTreeMap::from([
                ("title".to_owned(), "Tide".to_owned()),
                ("alt_title_1".to_owned(), "Tide - the old hook".to_owned()),
            ]),
        )
        .unwrap();

        answered(
            &conn,
            &profile_id,
            &audio,
            json!({ "title": "Tide", "alt_title_1": "Tide - a new hook" }),
        );

        let waiting = pending(&conn, &audio).unwrap();
        assert_eq!(waiting.len(), 1);
        assert_eq!(
            waiting[0]
                .fields
                .iter()
                .map(|field| (field.key.as_str(), field.proposed.as_str()))
                .collect::<Vec<_>>(),
            vec![("alt_title_1", "Tide - a new hook (audio)")],
            "the title is the same title; the one for later is shown as it will be kept"
        );
    }

    /// What the fields hold reaches the model as an earlier take to stay away
    /// from, not as a draft to polish - and an empty release is not told it
    /// has one.
    #[test]
    fn the_release_sheet_calls_what_is_written_an_earlier_take() {
        let (conn, profile_id) = fixtures::workspace();
        let tide = fixtures::song(&conn, &profile_id, "Tide");
        let audio = audio_of(&conn, &tide.id);
        assert!(!sheet(&conn, &audio).unwrap().contains("earlier take"));

        crate::actions::release::set_fields(
            &conn,
            &audio,
            &std::collections::BTreeMap::from([(
                "description".to_owned(),
                "A ficus. Half a pack of buckwheat.".to_owned(),
            )]),
        )
        .unwrap();

        let told = sheet(&conn, &audio).unwrap();
        assert!(told.contains("earlier take"), "{told}");
        assert!(told.contains("A ficus. Half a pack of buckwheat."));
    }

    /// The audio that comes after its song's clip is not the premiere: it
    /// keeps the plain tail of its door.
    #[test]
    fn an_audio_after_its_clip_keeps_the_plain_tail() {
        let (conn, profile_id) = fixtures::workspace();
        let tide = fixtures::song(&conn, &profile_id, "Tide");
        let clip = made_of(&conn, &tide.id, "video");
        release::schedule(&conn, &clip, "2026-11-01").unwrap();
        release::mark_released(&conn, &clip, None, Some("2026-11-01".into())).unwrap();

        let audio = audio_of(&conn, &tide.id);
        release::schedule(&conn, &audio, "2026-11-08").unwrap();
        write_title(&conn, &audio, "Tide (Song premiere 2026)");

        assert_eq!(title_of(&conn, &audio), "Tide (audio)");
    }

    /// The tail follows the release through the calendar, and follows its
    /// siblings: a year changed changes the year, a clip booked earlier takes
    /// the premiere and gives it back when it leaves the day, and a release
    /// that went out keeps the title it went out under.
    #[test]
    fn the_tail_follows_the_day_and_the_siblings() {
        let (conn, profile_id) = fixtures::workspace();
        let tide = fixtures::song(&conn, &profile_id, "Tide");
        let audio = audio_of(&conn, &tide.id);
        release::schedule(&conn, &audio, "2026-12-30").unwrap();
        write_title(&conn, &audio, "Tide");
        assert_eq!(title_of(&conn, &audio), "Tide (Song premiere 2026)");

        release::schedule(&conn, &audio, "2027-01-06").unwrap();
        assert_eq!(title_of(&conn, &audio), "Tide (Song premiere 2027)");

        let clip = made_of(&conn, &tide.id, "video");
        release::schedule(&conn, &clip, "2027-01-02").unwrap();
        assert_eq!(
            title_of(&conn, &audio),
            "Tide (audio)",
            "the clip booked earlier opens the song now"
        );

        release::unschedule(&conn, &clip).unwrap();
        assert_eq!(title_of(&conn, &audio), "Tide (Song premiere 2027)");

        release::mark_released(&conn, &audio, None, Some("2027-01-06".into())).unwrap();
        release::schedule(&conn, &clip, "2027-01-01").unwrap();
        assert_eq!(
            title_of(&conn, &audio),
            "Tide (Song premiere 2027)",
            "what went out keeps the title it went out under"
        );
        write_title(&conn, &audio, "Tide (Song premiere 2027)");
        assert_eq!(title_of(&conn, &audio), "Tide (Song premiere 2027)");
    }

    /// A teaser cut from the track, out before it, is an excerpt: it neither
    /// opens the song nor takes the premiere from the track.
    #[test]
    fn a_teaser_does_not_take_the_premiere() {
        let (conn, profile_id) = fixtures::workspace();
        let tide = fixtures::song(&conn, &profile_id, "Tide");
        let teaser = made_of(&conn, &tide.id, "short");
        release::schedule(&conn, &teaser, "2026-11-01").unwrap();
        release::mark_released(&conn, &teaser, None, Some("2026-11-01".into())).unwrap();

        let audio = audio_of(&conn, &tide.id);
        release::schedule(&conn, &audio, "2026-11-04").unwrap();
        write_title(&conn, &audio, "Tide");

        assert_eq!(title_of(&conn, &audio), "Tide (Song premiere 2026)");
    }

    /// What the writer reads as recent is ordered by the day each goes out
    /// on, booked or out: a text written for next week, or one not marked as
    /// out yet, is as recent as its day, and says which it is.
    #[test]
    fn the_history_reads_what_is_booked_beside_what_went_out() {
        let (mut conn, profile_id) = fixtures::workspace();
        let old = fixtures::video(&conn, &profile_id, "Old harbour");
        let old = fixtures::release(&conn, &old.id, "youtube", Some("2026-09-01"));
        write_title(&conn, &old.id, "Old harbour");
        release::mark_released(&conn, &old.id, None, Some("2026-09-01".into())).unwrap();
        let new = fixtures::video(&conn, &profile_id, "New harbour");
        let new = fixtures::release(&conn, &new.id, "youtube", Some("2026-10-10"));
        write_title(&conn, &new.id, "New harbour");
        let id = video_with_release(&mut conn, &profile_id, None);

        let read = history(&conn, &id, HISTORY).unwrap();

        let booked = read.find("“New harbour” (YouTube, booked for 2026-10-10)");
        let out = read.find("“Old harbour” (YouTube, went out 2026-09-01)");
        assert!(booked.is_some() && out.is_some(), "{read}");
        assert!(booked < out, "the newer day first, booked or not: {read}");
    }

    /// A video with a YouTube release planned for it, and the plot its
    /// description template reads.
    fn video_with_release(conn: &mut Connection, profile_id: &str, plot: Option<&str>) -> String {
        let work = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "video".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap();

        if let Some(plot) = plot {
            version::create(
                conn,
                &work.id,
                version::NewVersion {
                    role: "plot".into(),
                    body: plot.into(),
                    label: None,
                    meta: None,
                    make_current: true,
                    parent_version_id: None,
                    trial_id: None,
                },
            )
            .unwrap();
        }

        release::create(
            conn,
            NewRelease {
                work_id: work.id,
                kind: "youtube".into(),
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn the_fields_are_the_release_kinds_own() {
        let (mut conn, profile_id) = fixtures::workspace();
        let id = video_with_release(&mut conn, &profile_id, None);

        let fields = fields(&conn, &id).unwrap();

        let keys: Vec<&str> = fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(
            keys,
            vec!["title", "description", "tags", "pinned"],
            "a video's release is asked for what it goes out as, in the profile's order"
        );
    }

    #[test]
    fn a_release_of_a_kind_the_profile_lost_still_reads() {
        let (mut conn, profile_id) = fixtures::workspace();
        let id = video_with_release(&mut conn, &profile_id, None);
        release::update(
            &conn,
            &id,
            crate::release::ReleasePatch {
                kind: Some("a kind nobody ships".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let fields = fields(&conn, &id).unwrap();

        assert!(
            fields.is_empty(),
            "the release is still a real row with a real date, and the tab around it must draw"
        );
    }

    #[test]
    fn generating_fills_the_templated_fields_and_leaves_the_rest_alone() {
        let (mut conn, profile_id) = fixtures::workspace();
        let id = video_with_release(&mut conn, &profile_id, Some("the lamps come on at four"));

        let generated = generate(&conn, &id).unwrap();

        assert_eq!(
            generated.values.get("title").and_then(|v| v.as_str()),
            Some("Harbour lights")
        );
        assert_eq!(
            generated.values.get("description").and_then(|v| v.as_str()),
            Some("the lamps come on at four")
        );
        assert!(
            !generated.values.contains_key("tags"),
            "a field with no template is never written by generating: \
             overwriting what someone typed with nothing is the one thing it must not do"
        );
        assert!(generated.refused.is_empty());
    }

    #[test]
    fn a_field_waiting_on_a_missing_role_is_named_rather_than_written_blank() {
        let (mut conn, profile_id) = fixtures::workspace();
        // No plot: the description template reads `{role:plot}`.
        let id = video_with_release(&mut conn, &profile_id, None);

        let generated = generate(&conn, &id).unwrap();

        assert_eq!(
            generated.values.get("title").and_then(|v| v.as_str()),
            Some("Harbour lights"),
            "one field failing must not take the rest with it"
        );
        assert!(!generated.values.contains_key("description"));
        let refused: Vec<&str> = generated
            .refused
            .iter()
            .map(|refusal| refusal.key.as_str())
            .collect();
        assert_eq!(refused, vec!["description"]);
        assert!(
            generated.refused[0].reason.contains("Harbour lights"),
            "the reason says which work is waiting on what, not just that something failed: {}",
            generated.refused[0].reason
        );
    }

    #[test]
    fn generating_keeps_meta_no_field_describes() {
        let (mut conn, profile_id) = fixtures::workspace();
        let id = video_with_release(&mut conn, &profile_id, Some("a body"));
        // What a plugin left behind: `release.meta` is open, and a generate
        // button is not a reason to lose it.
        release::update(
            &conn,
            &id,
            crate::release::ReleasePatch {
                meta: Some(
                    json!({ "uploaded_by": "a plugin", "title": "typed by hand" })
                        .as_object()
                        .cloned()
                        .unwrap(),
                ),
                ..Default::default()
            },
        )
        .unwrap();

        let before = release::get(&conn, &id).unwrap().unwrap();
        let generated = generate(&conn, &id).unwrap();
        let merged = merged(&before.meta, &generated.values);

        assert_eq!(
            merged.get("uploaded_by").and_then(|v| v.as_str()),
            Some("a plugin"),
            "meta the profile does not describe survives a generation"
        );
        assert_eq!(
            merged.get("title").and_then(|v| v.as_str()),
            Some("Harbour lights"),
            "a field the profile does describe is replaced by what it makes"
        );
    }

    #[test]
    fn a_line_stays_one_line_however_the_title_was_typed() {
        let field = ReleaseField::new("title", "Title", ReleaseFieldType::Line);

        assert_eq!(
            tidy("Harbour\n  lights  ", &field),
            "Harbour lights",
            "a newline pasted into a title is invisible in a one-line box until the platform rejects it"
        );
    }

    #[test]
    fn tags_lose_their_blanks_and_their_repeats() {
        let field = ReleaseField::new("tags", "Tags", ReleaseFieldType::Tags);

        assert_eq!(
            tidy("lighthouse, , Lighthouse,  sea , lighthouse", &field),
            "lighthouse, sea",
            "the first spelling of each is kept; a list to clean by hand is not a list"
        );
    }

    #[test]
    fn a_stored_list_reads_back_as_the_text_it_obviously_is() {
        let meta = json!({ "tags": ["sea", "night"], "count": 3 })
            .as_object()
            .cloned()
            .unwrap();

        assert_eq!(stored(&meta, "tags"), "sea, night");
        assert_eq!(stored(&meta, "count"), "3");
        assert_eq!(stored(&meta, "nothing"), "");
    }

    #[test]
    fn fullness_counts_what_is_written() {
        let (mut conn, profile_id) = fixtures::workspace();
        let id = video_with_release(&mut conn, &profile_id, Some("a body"));

        let empty = fullness(&fields(&conn, &id).unwrap());
        assert_eq!(
            empty,
            Fullness {
                written: 0,
                total: 4
            }
        );
        assert!(!empty.complete());

        let generated = generate(&conn, &id).unwrap();
        let before = release::get(&conn, &id).unwrap().unwrap();
        release::update(
            &conn,
            &id,
            crate::release::ReleasePatch {
                meta: Some(merged(&before.meta, &generated.values)),
                ..Default::default()
            },
        )
        .unwrap();

        let after = fullness(&fields(&conn, &id).unwrap());
        assert_eq!(
            after,
            Fullness {
                written: 2,
                total: 4
            },
            "the two templated fields are written; the ones typed by hand are not"
        );
    }

    #[test]
    fn a_blank_field_counts_as_unwritten() {
        let (mut conn, profile_id) = fixtures::workspace();
        let id = video_with_release(&mut conn, &profile_id, None);
        release::update(
            &conn,
            &id,
            crate::release::ReleasePatch {
                meta: Some(json!({ "title": "   " }).as_object().cloned().unwrap()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(fullness(&fields(&conn, &id).unwrap()).written, 0);
    }
}
