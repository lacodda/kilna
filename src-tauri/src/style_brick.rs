//! The parts a prompt is built from: an image style, a character, a place.
//!
//! A brick belongs to the workspace, not to a work. The same character stands
//! in thirty videos, and written as a version it would be thirty texts with one
//! of them edited. So it is a row here, picked by whichever prompt wants it and
//! edited in one place.
//!
//! Its type is a word of the craft — kept in the profile document beside the
//! shot types and the scene blocks — and the type carries a `hint` saying what
//! to describe when a brick is of that type. That hint is the whole point of
//! one dictionary rather than several: the same photograph yields a render
//! recipe under `image-style` and a person under `character`, because the type
//! says which question is being answered. See ADR 0031.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::profile::config::Label;
use crate::time::now;

/// A brick of the workspace's style dictionary.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct StyleBrick {
    pub id: String,
    pub profile_id: String,
    /// A key of the profile's `style_types`.
    pub type_key: String,
    pub name: String,
    /// The vetted text that goes into a prompt verbatim. Absent while the
    /// brick is references and a name with nothing written yet.
    pub description: Option<String>,
    /// The owner's steer for whoever describes it — never part of a prompt.
    pub hint: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    /// How many reference pictures it carries. Read with the row because a
    /// list of bricks is drawn with their covers, and one query per row is the
    /// shape that made the predecessor's screens slow.
    pub reference_count: i64,
    /// The name per language while it is the one the set shipped; absent once
    /// the owner names the brick themselves.
    pub label: Option<Label>,
    /// A key of its type's `families`.
    pub family: Option<String>,
    /// When to reach for it - read by whoever picks bricks for a picture.
    pub when_to_use: Option<String>,
    /// What it is and what it gives, for the person reading it - per language
    /// while it is the set's, one string once a person writes it. Never part
    /// of a prompt: for a phrase of a sound the description is the generator's
    /// own English, and this is what it means.
    pub explanation: Option<Label>,
    /// `#RRGGBB` colours: a background's or an accent's one, or the stops of
    /// its gradient in order; an image style's palette; a lettering sample's
    /// ground.
    pub colours: Vec<String>,
    /// CSS declarations for the live sample of a lettering brick.
    pub sample: Option<String>,
    /// The entry of the starter set it came from; absent for the owner's own.
    pub set_key: Option<String>,
    /// Where it came from and whether it was changed since - see [`Origin`].
    pub origin: Origin,
    /// The fingerprint of the set entry last written into it. Bookkeeping of
    /// the seeding, never shown.
    #[serde(skip)]
    #[ts(skip)]
    pub set_digest: Option<String>,
    /// The trial of an experiment it was cut from, while the trial is there
    /// (ADR 0061).
    pub trial_id: Option<String>,
}

/// Where a brick came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename = "StyleOrigin")]
pub enum Origin {
    /// From the starter set, untouched: a newer set may rewrite it.
    Set,
    /// From the starter set, and changed by the owner since: theirs to keep,
    /// with the set's version one click away.
    Changed,
    /// The owner's own.
    Own,
}

/// What a brick says - every field a person writes except its status and the
/// steer, which are about the brick rather than part of it. The fingerprint
/// of a set brick is taken over exactly this, so the set entry and the row
/// are compared on the same terms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Content<'a> {
    pub type_key: &'a str,
    pub name: &'a str,
    pub label: Option<&'a Label>,
    pub family: Option<&'a str>,
    pub description: Option<&'a str>,
    pub when_to_use: Option<&'a str>,
    pub colours: &'a [String],
    pub sample: Option<&'a str>,
    /// Left out of the fingerprint while there is none, so every brick
    /// fingerprinted before explanations existed still reads untouched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<&'a Label>,
}

impl Content<'_> {
    /// A stable fingerprint: FNV-1a over the content's JSON. Not a guard
    /// against anyone - only "is this still what was written".
    pub fn digest(&self) -> String {
        let text = serde_json::to_string(self).unwrap_or_default();
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in text.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        format!("{hash:016x}")
    }
}

impl StyleBrick {
    pub fn content(&self) -> Content<'_> {
        Content {
            type_key: &self.type_key,
            name: &self.name,
            label: self.label.as_ref(),
            family: self.family.as_deref(),
            description: self.description.as_deref(),
            when_to_use: self.when_to_use.as_deref(),
            colours: &self.colours,
            sample: self.sample.as_deref(),
            explanation: self.explanation.as_ref(),
        }
    }
}

/// A brick is a draft until it carries a description, ready once it does, and
/// dropped when the owner retires it without losing what it was.
pub const DRAFT: &str = "draft";
pub const READY: &str = "ready";
pub const DROPPED: &str = "dropped";

const STATUSES: [&str; 3] = [DRAFT, READY, DROPPED];

const SELECT: &str = "SELECT b.id, b.profile_id, b.type_key, b.name, b.description, b.hint, \
     b.status, b.created_at, b.updated_at, \
     (SELECT count(*) FROM asset a WHERE a.style_brick_id = b.id), \
     b.label, b.family, b.when_to_use, b.colours, b.sample, b.set_key, b.set_digest, \
     b.explanation, b.trial_id \
     FROM style_brick b";

/// What to make a brick out of.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct NewStyleBrick {
    pub type_key: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub when_to_use: Option<String>,
    #[serde(default)]
    pub colours: Option<Vec<String>>,
    #[serde(default)]
    pub sample: Option<String>,
    #[serde(default)]
    pub explanation: Option<Label>,
    /// The trial of an experiment the phrase was cut from: a kept one.
    #[serde(default)]
    pub trial_id: Option<String>,
}

/// What may be changed about one.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
pub struct StyleBrickPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub hint: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub label: Option<Option<Label>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub family: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub when_to_use: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colours: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub sample: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub explanation: Option<Option<Label>>,
    /// The fingerprint of the set entry the brick now matches - written by
    /// "restore as in the set", so a restored brick reads untouched again.
    /// Travels in the log like any field; the window never sends it.
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[ts(skip)]
    pub set_digest: Option<Option<String>>,
}

impl StyleBrickPatch {
    /// The patch a person's edit really is, given the brick it lands on.
    ///
    /// Renaming a brick makes the name the owner's: the shipped word per
    /// language goes with it, or the window would go on showing the old one.
    /// Moving a brick to another type leaves a family that type may not have:
    /// it goes too. Said in the patch rather than done behind it, so the log
    /// carries it and undo puts it back.
    pub fn completed(mut self, before: &StyleBrick) -> Self {
        if self
            .name
            .as_ref()
            .is_some_and(|name| name.trim() != before.name)
            && self.label.is_none()
            && before.label.is_some()
        {
            self.label = Some(None);
        }
        if self
            .type_key
            .as_ref()
            .is_some_and(|key| *key != before.type_key)
            && self.family.is_none()
            && before.family.is_some()
        {
            self.family = Some(None);
        }
        self
    }
}

/// Which bricks to list.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct StyleBrickFilter {
    /// One type only. Absent means every type.
    #[serde(default)]
    pub type_key: Option<String>,
    /// Only bricks that can go into a prompt. The constructor sets it: a draft
    /// is unfinished by definition, and offering one silently is how a picker
    /// fills up with things nobody has touched.
    // Not an `Option`, so `optional_fields` cannot reach it - said by hand.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub ready_only: bool,
    /// Text to match against the name and the description.
    #[serde(default)]
    pub query: Option<String>,
}

pub fn create(conn: &Connection, profile_id: &str, new: NewStyleBrick) -> Result<StyleBrick> {
    create_minted(conn, profile_id, new, Minted::fresh())
}

/// Make a brick with the id and moment already decided — the seam a replay
/// comes back through, see ADR 0014.
pub fn create_minted(
    conn: &Connection,
    profile_id: &str,
    new: NewStyleBrick,
    minted: Minted,
) -> Result<StyleBrick> {
    let name = new.name.trim();
    if name.is_empty() {
        return Err(Error::refused("style.needsName"));
    }
    let config = crate::profile::config_for(conn, profile_id)?;
    check_new_type(&config, &new.type_key)?;
    check_family(&config, &new.type_key, new.family.as_deref())?;
    let colours = checked_colours(new.colours.as_deref().unwrap_or_default())?;
    if let Some(trial) = new.trial_id.as_deref() {
        crate::lab::harvest::check_phrase(conn, profile_id, &config, trial, &new.type_key)?;
    }

    // Born ready when it arrives with its description already written — an
    // imported brick, or one the assistant described in the same breath. Born
    // a draft when it is only references and a name.
    let status = if new
        .description
        .as_deref()
        .is_some_and(|text| !text.trim().is_empty())
    {
        READY
    } else {
        DRAFT
    };

    let explanation = new
        .explanation
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    conn.execute(
        "INSERT INTO style_brick (id, profile_id, type_key, name, description, hint, status, created_at, updated_at,
                                  family, when_to_use, colours, sample, explanation, trial_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            minted.id(),
            profile_id,
            new.type_key,
            name,
            new.description,
            new.hint,
            status,
            minted.at(),
            new.family,
            new.when_to_use,
            colours,
            new.sample,
            explanation,
            new.trial_id
        ],
    )
    .map_err(|error| taken(error, &new.type_key, name))?;

    get(conn, minted.id())?.ok_or_else(|| unknown(minted.id()))
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<StyleBrick>> {
    let mut statement = conn.prepare(&format!("{SELECT} WHERE b.id = ?1"))?;
    Ok(statement.query_row(params![id], read).optional()?)
}

/// The dictionary, by type and then by name — the order the picker reads in,
/// which is the order the profile names the types in rather than the alphabet.
pub fn list(
    conn: &Connection,
    profile_id: &str,
    filter: &StyleBrickFilter,
) -> Result<Vec<StyleBrick>> {
    let mut sql = format!("{SELECT} WHERE b.profile_id = ?1");
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(profile_id.to_owned())];

    if let Some(type_key) = &filter.type_key {
        values.push(Box::new(type_key.clone()));
        sql.push_str(&format!(" AND b.type_key = ?{}", values.len()));
    }
    if filter.ready_only {
        values.push(Box::new(READY.to_owned()));
        sql.push_str(&format!(" AND b.status = ?{}", values.len()));
    }
    if let Some(query) = filter
        .query
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
    {
        values.push(Box::new(format!("%{query}%")));
        let at = values.len();
        sql.push_str(&format!(
            " AND (b.name LIKE ?{at} ESCAPE '\\' OR b.description LIKE ?{at} ESCAPE '\\')"
        ));
    }
    sql.push_str(" ORDER BY b.name COLLATE NOCASE, b.rowid");

    let mut statement = conn.prepare(&sql)?;
    let rows = statement
        .query_map(
            rusqlite::params_from_iter(values.iter().map(AsRef::as_ref)),
            read,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    // Sorted into the profile's own order of types here rather than in SQL:
    // the order is a fact of the document, and the database does not read it.
    let config = crate::profile::config_for(conn, profile_id)?;
    let place = |key: &str| {
        config
            .style_types
            .iter()
            .position(|kind| kind.key == key)
            .unwrap_or(usize::MAX)
    };
    let mut rows = rows;
    rows.sort_by_key(|brick| place(&brick.type_key));
    Ok(rows)
}

/// How many bricks stand under each type, for the counts beside the filter.
pub fn counts(conn: &Connection, profile_id: &str) -> Result<Vec<(String, i64)>> {
    let mut statement = conn.prepare(
        "SELECT type_key, count(*) FROM style_brick
         WHERE profile_id = ?1 AND status <> 'dropped'
         GROUP BY type_key",
    )?;
    let rows = statement
        .query_map(params![profile_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn update(conn: &Connection, id: &str, patch: StyleBrickPatch) -> Result<StyleBrick> {
    update_at(conn, id, patch, &now())
}

/// Update a brick with the change's timestamp already decided — the replay
/// seam again, ADR 0014.
pub fn update_at(
    conn: &Connection,
    id: &str,
    patch: StyleBrickPatch,
    at: &str,
) -> Result<StyleBrick> {
    let brick = get(conn, id)?.ok_or_else(|| unknown(id))?;

    let mut assignments: Vec<String> = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    fn set(
        assignments: &mut Vec<String>,
        values: &mut Vec<Box<dyn rusqlite::ToSql>>,
        column: &str,
        value: Box<dyn rusqlite::ToSql>,
    ) {
        values.push(value);
        assignments.push(format!("{column} = ?{}", values.len()));
    }

    let mut name = brick.name.clone();
    let mut type_key = brick.type_key.clone();

    let config = crate::profile::config_for(conn, &brick.profile_id)?;
    if let Some(next) = patch.type_key {
        // Staying in a retired type is reading what was written; moving into
        // one is making a new brick of it.
        if next != brick.type_key {
            check_new_type(&config, &next)?;
        }
        type_key = next.clone();
        set(&mut assignments, &mut values, "type_key", Box::new(next));
    }
    let family = match &patch.family {
        Some(next) => next.clone(),
        None => brick.family.clone(),
    };
    if patch.family.is_some() || type_key != brick.type_key {
        check_family(&config, &type_key, family.as_deref())?;
    }
    if let Some(next) = patch.family {
        set(&mut assignments, &mut values, "family", Box::new(next));
    }
    if let Some(next) = patch.label {
        let text = next
            .map(|label| serde_json::to_string(&label))
            .transpose()?;
        set(&mut assignments, &mut values, "label", Box::new(text));
    }
    if let Some(next) = patch.when_to_use {
        set(&mut assignments, &mut values, "when_to_use", Box::new(next));
    }
    if let Some(next) = patch.explanation {
        let text = next
            .map(|label| serde_json::to_string(&label))
            .transpose()?;
        set(&mut assignments, &mut values, "explanation", Box::new(text));
    }
    if let Some(next) = patch.colours {
        let text = checked_colours(&next)?;
        set(&mut assignments, &mut values, "colours", Box::new(text));
    }
    if let Some(next) = patch.sample {
        set(&mut assignments, &mut values, "sample", Box::new(next));
    }
    if let Some(next) = patch.set_digest {
        set(&mut assignments, &mut values, "set_digest", Box::new(next));
    }
    if let Some(next) = patch.name {
        let trimmed = next.trim().to_owned();
        if trimmed.is_empty() {
            return Err(Error::refused("style.needsName"));
        }
        name = trimmed.clone();
        set(&mut assignments, &mut values, "name", Box::new(trimmed));
    }
    if let Some(next) = patch.description {
        set(&mut assignments, &mut values, "description", Box::new(next));
    }
    if let Some(next) = patch.hint {
        set(&mut assignments, &mut values, "hint", Box::new(next));
    }
    if let Some(next) = patch.status {
        if !STATUSES.contains(&next.as_str()) {
            return Err(Error::refused("style.badStatus").param("value", next.clone()));
        }
        set(&mut assignments, &mut values, "status", Box::new(next));
    }

    if assignments.is_empty() {
        return Ok(brick);
    }

    set(
        &mut assignments,
        &mut values,
        "updated_at",
        Box::new(at.to_owned()),
    );
    values.push(Box::new(id.to_owned()));

    let sql = format!(
        "UPDATE style_brick SET {} WHERE id = ?{}",
        assignments.join(", "),
        values.len()
    );
    conn.execute(
        &sql,
        rusqlite::params_from_iter(values.iter().map(AsRef::as_ref)),
    )
    .map_err(|error| taken(error, &type_key, &name))?;

    get(conn, id)?.ok_or_else(|| unknown(id))
}

/// Write a description onto a brick and let it out of draft in one move.
///
/// The two belong together: a description is exactly what a draft is missing,
/// and a brick left `draft` with its text written would be invisible to the
/// constructor for no reason a person could see. A brick the owner has dropped
/// stays dropped — describing it again is not un-retiring it.
pub fn describe(conn: &Connection, id: &str, description: &str) -> Result<StyleBrick> {
    let brick = get(conn, id)?.ok_or_else(|| unknown(id))?;
    let status = if brick.status == DROPPED {
        None
    } else {
        Some(READY.to_owned())
    };
    update(
        conn,
        id,
        StyleBrickPatch {
            description: Some(Some(description.to_owned())),
            status,
            ..Default::default()
        },
    )
}

/// A new brick is of a type the profile names, and not one it retired.
///
/// Checked here rather than in the schema, for the reason `scene_note` checks
/// its kinds there: the vocabulary is the profile's and changes while the
/// workspace lives. A profile naming no types has not decided yet, and anything
/// goes. A retired type still reads - its bricks keep their word - but makes no
/// new ones: what did its work is somewhere else now.
fn check_new_type(config: &crate::profile::config::ProfileConfig, type_key: &str) -> Result<()> {
    if let Some(style) = config.style_type(type_key)
        && style.retired.is_some()
    {
        return Err(Error::refused("style.retiredType").param("type", type_key));
    }
    if config.style_types.is_empty() || config.style_type(type_key).is_some() {
        return Ok(());
    }
    let known = config
        .style_types
        .iter()
        .map(|one| one.key.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(Error::refused("style.unknownType")
        .param("type", type_key)
        .param("known", known))
}

/// A family is one its type files bricks under. A type the profile no longer
/// names, or one that files nothing, takes none.
fn check_family(
    config: &crate::profile::config::ProfileConfig,
    type_key: &str,
    family: Option<&str>,
) -> Result<()> {
    let Some(family) = family else {
        return Ok(());
    };
    let known = config
        .style_type(type_key)
        .is_some_and(|style| style.families.iter().any(|f| f.key == family));
    if known {
        return Ok(());
    }
    Err(Error::refused("style.unknownFamily")
        .param("family", family)
        .param("type", type_key))
}

/// Colours as the column keeps them: `#RRGGBB`, upper case, as JSON.
fn checked_colours(colours: &[String]) -> Result<String> {
    let mut out = Vec::with_capacity(colours.len());
    for colour in colours {
        let colour = colour.trim();
        let hex = colour.strip_prefix('#').unwrap_or("");
        if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::refused("style.badColour").param("value", colour));
        }
        out.push(format!("#{}", hex.to_ascii_uppercase()));
    }
    Ok(serde_json::to_string(&out)?)
}

/// The unique index speaking in the craft's words rather than SQLite's.
fn taken(error: rusqlite::Error, type_key: &str, name: &str) -> Error {
    if let rusqlite::Error::SqliteFailure(failure, _) = &error
        && failure.code == rusqlite::ErrorCode::ConstraintViolation
    {
        return Error::refused("style.alreadyExists")
            .param("type", type_key)
            .param("name", name);
    }
    Error::from(error)
}

fn unknown(id: &str) -> Error {
    Error::not_found("style", id)
}

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<StyleBrick> {
    let json = |index: usize| -> rusqlite::Result<Option<serde_json::Value>> {
        let text: Option<String> = row.get(index)?;
        Ok(text.and_then(|text| serde_json::from_str(&text).ok()))
    };
    let label = json(10)?.and_then(|value| serde_json::from_value(value).ok());
    let colours = json(13)?
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
    let explanation = json(17)?.and_then(|value| serde_json::from_value(value).ok());
    let mut brick = StyleBrick {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        type_key: row.get(2)?,
        name: row.get(3)?,
        description: row.get(4)?,
        hint: row.get(5)?,
        status: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        reference_count: row.get(9)?,
        label,
        family: row.get(11)?,
        when_to_use: row.get(12)?,
        colours,
        sample: row.get(14)?,
        set_key: row.get(15)?,
        set_digest: row.get(16)?,
        explanation,
        origin: Origin::Own,
        trial_id: row.get(18)?,
    };
    brick.origin = match (&brick.set_key, &brick.set_digest) {
        (Some(_), Some(digest)) if *digest == brick.content().digest() => Origin::Set,
        (Some(_), _) => Origin::Changed,
        (None, _) => Origin::Own,
    };
    Ok(brick)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::profile;
    use crate::profile::config::StyleType;

    fn brick(type_key: &str, name: &str) -> NewStyleBrick {
        NewStyleBrick {
            type_key: type_key.to_owned(),
            name: name.to_owned(),
            ..NewStyleBrick::default()
        }
    }

    #[test]
    fn a_brick_without_a_description_is_a_draft_and_one_with_it_is_ready() {
        let (conn, profile_id) = fixtures::workspace();

        let draft = create(&conn, &profile_id, brick("character", "Ranger")).unwrap();
        assert_eq!(draft.status, DRAFT);

        let written = create(
            &conn,
            &profile_id,
            NewStyleBrick {
                description: Some("A tall figure in a long coat.".into()),
                ..brick("character", "Warden")
            },
        )
        .unwrap();
        assert_eq!(written.status, READY);
    }

    #[test]
    fn describing_a_draft_makes_it_ready_but_does_not_revive_a_dropped_one() {
        let (conn, profile_id) = fixtures::workspace();

        let draft = create(&conn, &profile_id, brick("character", "Ranger")).unwrap();
        let described = describe(&conn, &draft.id, "A tall figure in a long coat.").unwrap();
        assert_eq!(described.status, READY);
        assert_eq!(
            described.description.as_deref(),
            Some("A tall figure in a long coat.")
        );

        let retired = update(
            &conn,
            &draft.id,
            StyleBrickPatch {
                status: Some(DROPPED.into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(retired.status, DROPPED);

        let again = describe(&conn, &draft.id, "A different figure.").unwrap();
        assert_eq!(
            again.status, DROPPED,
            "describing a retired style is not un-retiring it"
        );
        assert_eq!(again.description.as_deref(), Some("A different figure."));
    }

    #[test]
    fn the_constructor_is_offered_only_ready_bricks() {
        let (conn, profile_id) = fixtures::workspace();

        create(&conn, &profile_id, brick("character", "Draft one")).unwrap();
        let ready = create(
            &conn,
            &profile_id,
            NewStyleBrick {
                description: Some("Written.".into()),
                ..brick("character", "Ready one")
            },
        )
        .unwrap();
        let dropped = create(
            &conn,
            &profile_id,
            NewStyleBrick {
                description: Some("Written too.".into()),
                ..brick("character", "Dropped one")
            },
        )
        .unwrap();
        update(
            &conn,
            &dropped.id,
            StyleBrickPatch {
                status: Some(DROPPED.into()),
                ..Default::default()
            },
        )
        .unwrap();

        let offered = list(
            &conn,
            &profile_id,
            &StyleBrickFilter {
                ready_only: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            offered.iter().map(|b| b.id.clone()).collect::<Vec<_>>(),
            vec![ready.id],
            "a draft is unfinished and a dropped one is retired"
        );

        let all = list(&conn, &profile_id, &StyleBrickFilter::default()).unwrap();
        assert_eq!(all.len(), 3, "the dictionary screen still shows every one");
    }

    #[test]
    fn two_bricks_of_one_type_may_not_share_a_name_but_two_types_may() {
        let (conn, profile_id) = fixtures::workspace();

        create(&conn, &profile_id, brick("character", "Ranger")).unwrap();
        let clash = create(&conn, &profile_id, brick("character", "Ranger"));
        let clash = clash.unwrap_err();
        assert_eq!(
            clash.refusal().map(|r| r.code),
            Some("style.alreadyExists"),
            "two `character` styles called Ranger are a picker nobody can choose from"
        );
        assert_eq!(
            clash.refusal().and_then(|r| r.params.get("name")),
            Some(&serde_json::Value::String("Ranger".into())),
            "the refusal names the style"
        );

        create(&conn, &profile_id, brick("look", "Ranger"))
            .expect("a character and a look may both be called Ranger");
    }

    #[test]
    fn a_type_the_profile_does_not_name_is_refused() {
        let (conn, profile_id) = fixtures::workspace();

        let refused = create(&conn, &profile_id, brick("nonsense", "Whatever")).unwrap_err();
        assert_eq!(refused.refusal().map(|r| r.code), Some("style.unknownType"));
        assert_eq!(
            refused.refusal().and_then(|r| r.params.get("type")),
            Some(&serde_json::Value::String("nonsense".into())),
            "names what was asked"
        );
        assert!(
            refused
                .refusal()
                .and_then(|r| r.params.get("known"))
                .and_then(|v| v.as_str())
                .is_some_and(|known| known.contains("character")),
            "names what the craft has: {refused}"
        );
    }

    #[test]
    fn a_profile_naming_no_types_has_not_decided_yet() {
        let (conn, profile_id) = fixtures::workspace();
        let mut config = profile::config_for(&conn, &profile_id).unwrap();
        config.style_types.clear();
        // So do the compositions picked from its phrases.
        config.compose.clear();
        // The actions that read `{styles}` go with the dictionary: a craft
        // with no types has no prompt to build out of them, and the profile
        // refuses the pair — which is the check this test leans on elsewhere.
        config
            .prompts
            .retain(|action| !action.placeholders().iter().any(|name| name == "styles"));
        profile::update_config(&conn, &profile_id, &config).unwrap();

        create(&conn, &profile_id, brick("anything", "Whatever"))
            .expect("a craft with no dictionary refuses nothing");
    }

    #[test]
    fn the_list_reads_in_the_profiles_order_of_types_not_the_alphabet() {
        let (conn, profile_id) = fixtures::workspace();
        let mut config = profile::config_for(&conn, &profile_id).unwrap();
        config.style_types = vec![
            StyleType::new("zebra", "Zebra"),
            StyleType::new("alpha", "Alpha"),
        ];
        config.compose.clear();
        profile::update_config(&conn, &profile_id, &config).unwrap();

        create(&conn, &profile_id, brick("alpha", "An alpha one")).unwrap();
        create(&conn, &profile_id, brick("zebra", "A zebra one")).unwrap();

        let listed = list(&conn, &profile_id, &StyleBrickFilter::default()).unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|b| b.type_key.clone())
                .collect::<Vec<_>>(),
            vec!["zebra", "alpha"],
            "the document's order is the picker's order"
        );
    }

    #[test]
    fn a_search_matches_the_name_and_the_description() {
        let (conn, profile_id) = fixtures::workspace();

        create(
            &conn,
            &profile_id,
            NewStyleBrick {
                description: Some("Grainy monochrome film.".into()),
                ..brick("image-style", "Cold north")
            },
        )
        .unwrap();
        create(&conn, &profile_id, brick("character", "Grainy name")).unwrap();
        create(&conn, &profile_id, brick("character", "Nothing alike")).unwrap();

        let found = list(
            &conn,
            &profile_id,
            &StyleBrickFilter {
                query: Some("grainy".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(found.len(), 2, "the text and the name are both searched");
    }

    #[test]
    fn a_dropped_brick_is_left_out_of_the_counts() {
        let (conn, profile_id) = fixtures::workspace();

        let one = create(&conn, &profile_id, brick("character", "Kept")).unwrap();
        create(&conn, &profile_id, brick("character", "Retired")).unwrap();
        update(
            &conn,
            &one.id,
            StyleBrickPatch {
                status: Some(DROPPED.into()),
                ..Default::default()
            },
        )
        .unwrap();

        let counts = counts(&conn, &profile_id).unwrap();
        assert_eq!(counts, vec![("character".to_owned(), 1)]);
    }

    #[test]
    fn a_status_the_craft_does_not_have_is_refused() {
        let (conn, profile_id) = fixtures::workspace();
        let one = create(&conn, &profile_id, brick("character", "Ranger")).unwrap();

        let refused = update(
            &conn,
            &one.id,
            StyleBrickPatch {
                status: Some("nearly".into()),
                ..Default::default()
            },
        );
        assert!(refused.is_err());
        assert_eq!(
            get(&conn, &one.id).unwrap().unwrap().status,
            DRAFT,
            "a refused change changes nothing"
        );
    }

    #[test]
    fn deleting_a_brick_leaves_a_tombstone_under_its_name() {
        let (conn, profile_id) = fixtures::workspace();
        let one = create(&conn, &profile_id, brick("character", "Ranger")).unwrap();

        // The road every deletion takes since v0.76.1: into the trash.
        crate::trash::discard(&conn, crate::trash::Entity::Style, &one.id).unwrap();

        let label: String = conn
            .query_row(
                "SELECT label FROM tombstone WHERE entity = 'style_brick' AND entity_id = ?1",
                params![one.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(label, "Ranger");
    }
}
