use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::canon::Layer;
use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::time::now;

/// Ideas, phrases, lore, reference — one type distinguished by `kind` and tags
/// rather than by separate subsystems. The predecessor built those subsystems
/// and they went unused; see the vision notes.
///
/// A note of a kind the profile marks `material` - an idea, a phrase - is
/// spent by works, and says so in its `state` (ADR 0045).
///
/// A note of a kind the profile names with sections is a card of the canon
/// (ADR 0043): the last four fields are its own, and on a plain note they
/// stand at their defaults.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Note {
    pub id: String,
    pub profile_id: String,
    pub work_id: Option<String>,
    pub kind: String,
    pub title: Option<String>,
    pub body: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Who may know the card exists: a task that reads the public layer
    /// does not see a card of the internal one at all.
    pub layer: Layer,
    /// The names the card goes by in the works' texts - the forms a lyric
    /// uses, since there is no stemmer to find them.
    pub aliases: Vec<String>,
    /// The English description a picture generator is given instead of the
    /// name.
    pub prompt: Option<String>,
    /// A fingerprint of the facts the description was written from. When it
    /// no longer matches the card's facts, the description is stale.
    pub prompt_basis: Option<String>,
    /// Where a note of a material kind stands: fresh, used by a work, set
    /// aside, given up on. A plain note is fresh for ever.
    pub state: NoteState,
}

/// Where a material note stands (ADR 0045). The code's words, not the
/// profile's: sending a note to a work sets `used`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum NoteState {
    /// Still there to use.
    #[default]
    Fresh,
    /// Spent: gone into a work, which `work_id` names when it is in kilna.
    Used,
    /// Set aside for later.
    Parked,
    /// Given up on, kept so it is not written again.
    Dropped,
}

impl NoteState {
    pub const ALL: [NoteState; 4] = [
        NoteState::Fresh,
        NoteState::Used,
        NoteState::Parked,
        NoteState::Dropped,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            NoteState::Fresh => "fresh",
            NoteState::Used => "used",
            NoteState::Parked => "parked",
            NoteState::Dropped => "dropped",
        }
    }

    /// A stored state. An unknown word is a row the schema should not have let
    /// in.
    pub fn parse(raw: &str) -> Result<Self> {
        Self::from_word(raw)
            .ok_or_else(|| Error::Internal(format!("a stored note state reads `{raw}`")))
    }

    /// A state as a caller names it; none for any other word.
    pub fn from_word(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|one| one.as_str() == raw.trim())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct NewNote {
    pub body: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub work_id: Option<String>,
    // Not an `Option`, so `optional_fields` cannot reach it - said by hand.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub tags: Vec<String>,
    /// The public layer when absent.
    #[serde(default)]
    pub layer: Option<Layer>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub aliases: Vec<String>,
    /// Fresh when absent.
    #[serde(default)]
    pub state: Option<NoteState>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
pub struct NotePatch {
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub title: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub work_id: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<Layer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub prompt: Option<Option<String>>,
    /// Set together with `prompt`, by the action that knows which facts the
    /// description was written from - never by the window.
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub prompt_basis: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<NoteState>,
}

/// Narrowing applied to a listing. Every field may be left out - serde
/// reads a missing `Option` as none - and the generated type says so.
#[derive(Debug, Clone, Default, Deserialize, ts_rs::TS)]
#[ts(optional_fields = nullable)]
pub struct NoteFilter {
    pub work_id: Option<String>,
    pub kind: Option<String>,
    /// Notes carrying this tag.
    pub tag: Option<String>,
    /// Case-insensitive substring of the title or body.
    pub search: Option<String>,
    /// Cards of the canon only (`true`), or plain notes only (`false`): the
    /// Notes screen and the Canon screen list different halves of one table.
    pub canon: Option<bool>,
    /// Notes of the kinds kept as one line (`true`), or of the others
    /// (`false`): a thousand phrases are read as rows of their own, not
    /// under "All" beside the pages (ADR 0045).
    pub line: Option<bool>,
    /// Notes in this state.
    pub state: Option<NoteState>,
}

const SELECT_NOTE: &str = "SELECT id, profile_id, work_id, kind, title, body, tags, created_at, \
     updated_at, layer, aliases, prompt, prompt_basis, state FROM note";

pub fn create(conn: &Connection, profile_id: &str, new: NewNote) -> Result<Note> {
    create_minted(conn, profile_id, new, Minted::fresh())
}

/// Create a note with the id and timestamp already decided.
///
/// The seam a replay comes back through: live, `create` mints them; replaying,
/// the log supplies what the first run generated, so the note lands under the
/// id everything else already names. See ADR 0014.
pub fn create_minted(
    conn: &Connection,
    profile_id: &str,
    new: NewNote,
    minted: Minted,
) -> Result<Note> {
    let id = minted.id().to_owned();
    let timestamp = minted.at().to_owned();
    let kind = new.kind.unwrap_or_else(|| "note".into());
    one_root(conn, profile_id, &kind, None)?;

    conn.execute(
        "INSERT INTO note (id, profile_id, work_id, kind, title, body, tags, created_at, updated_at, layer, aliases, state)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9, ?10, ?11)",
        params![
            id,
            profile_id,
            new.work_id,
            kind,
            new.title,
            new.body,
            serde_json::to_string(&new.tags)?,
            timestamp,
            new.layer.unwrap_or_default().as_str(),
            serde_json::to_string(&clean_aliases(new.aliases))?,
            new.state.unwrap_or_default().as_str(),
        ],
    )?;

    get(conn, &id)?.ok_or_else(|| Error::Internal("the note vanished after insert".into()))
}

/// The names a card goes by, as they are kept: trimmed, the empty ones
/// dropped, each once - the first spelling wins, whatever its case.
pub fn clean_aliases(aliases: Vec<String>) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for alias in aliases {
        let alias = alias.trim().to_owned();
        if alias.is_empty()
            || kept
                .iter()
                .any(|k| k.to_lowercase() == alias.to_lowercase())
        {
            continue;
        }
        kept.push(alias);
    }
    kept
}

/// The canon has one root: a second card of the root kind is refused, whether
/// it is made or an existing note is turned into one. `except` is the note
/// being changed, which may of course be the root already.
fn one_root(conn: &Connection, profile_id: &str, kind: &str, except: Option<&str>) -> Result<()> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let Some(root) = config.note_kind(kind).filter(|k| k.root) else {
        return Ok(());
    };
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM note WHERE profile_id = ?1 AND kind = ?2 AND id IS NOT ?3 LIMIT 1",
            params![profile_id, kind, except],
            |row| row.get(0),
        )
        .optional()?;
    match existing {
        Some(_) => Err(Error::refused("canon.secondRoot").param(
            "kind",
            serde_json::to_value(&root.label).unwrap_or_default(),
        )),
        None => Ok(()),
    }
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Note>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_NOTE} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?;

    raw.map(RawNote::into_note).transpose()
}

/// Notes in a profile, most recently touched first.
pub fn list(conn: &Connection, profile_id: &str, filter: &NoteFilter) -> Result<Vec<Note>> {
    let mut sql = format!("{SELECT_NOTE} WHERE profile_id = ?1");
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(profile_id.to_owned())];

    if let Some(work_id) = &filter.work_id {
        values.push(Box::new(work_id.clone()));
        sql.push_str(&format!(" AND work_id = ?{}", values.len()));
    }
    if let Some(kind) = &filter.kind {
        values.push(Box::new(kind.clone()));
        sql.push_str(&format!(" AND kind = ?{}", values.len()));
    }
    if let Some(tag) = &filter.tag {
        // Tags are a JSON array; matching an element beats a LIKE over the
        // serialised text, which would also match a tag that merely contains it.
        values.push(Box::new(tag.clone()));
        sql.push_str(&format!(
            " AND EXISTS (SELECT 1 FROM json_each(note.tags) WHERE json_each.value = ?{})",
            values.len()
        ));
    }
    if let Some(search) = &filter.search {
        values.push(Box::new(format!("%{search}%")));
        let n = values.len();
        sql.push_str(&format!(
            " AND (coalesce(title, '') LIKE ?{n} OR body LIKE ?{n})"
        ));
    }
    if let Some(state) = filter.state {
        values.push(Box::new(state.as_str().to_owned()));
        sql.push_str(&format!(" AND state = ?{}", values.len()));
    }
    if let Some(line) = filter.line {
        // Which kinds are lines is the profile's word, read at the moment of
        // asking, as the canon's split is.
        let config = crate::profile::config_for(conn, profile_id)?;
        values.push(Box::new(serde_json::to_string(&config.line_kinds())?));
        let n = values.len();
        let not = if line { "" } else { "NOT " };
        sql.push_str(&format!(
            " AND kind {not}IN (SELECT value FROM json_each(?{n}))"
        ));
    }
    if let Some(canon) = filter.canon {
        // Which kinds are cards is the profile's word, so the split is read
        // from it at the moment of asking rather than kept on the row.
        let config = crate::profile::config_for(conn, profile_id)?;
        values.push(Box::new(serde_json::to_string(&config.card_kinds())?));
        let n = values.len();
        let not = if canon { "" } else { "NOT " };
        sql.push_str(&format!(
            " AND kind {not}IN (SELECT value FROM json_each(?{n}))"
        ));
    }

    sql.push_str(" ORDER BY updated_at DESC, rowid DESC");

    let mut statement = conn.prepare(&sql)?;
    let params = rusqlite::params_from_iter(values.iter().map(AsRef::as_ref));
    let raw = statement
        .query_map(params, read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    raw.into_iter().map(RawNote::into_note).collect()
}

pub fn update(conn: &Connection, id: &str, patch: NotePatch) -> Result<Note> {
    update_at(conn, id, patch, &now())
}

/// Update a note with the change's timestamp already decided.
///
/// The seam a replay comes back through: live, `update` stamps `now()`;
/// replaying, the log supplies the moment the first run recorded, so the
/// edit lands with the time it actually happened. See ADR 0014.
pub fn update_at(conn: &Connection, id: &str, patch: NotePatch, at: &str) -> Result<Note> {
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

    if let Some(title) = patch.title {
        set(&mut assignments, &mut values, "title", Box::new(title));
    }
    if let Some(body) = patch.body {
        set(&mut assignments, &mut values, "body", Box::new(body));
    }
    if let Some(kind) = patch.kind {
        let found = get(conn, id)?.ok_or_else(|| unknown_note(id))?;
        one_root(conn, &found.profile_id, &kind, Some(id))?;
        set(&mut assignments, &mut values, "kind", Box::new(kind));
    }
    if let Some(tags) = patch.tags {
        set(
            &mut assignments,
            &mut values,
            "tags",
            Box::new(serde_json::to_string(&tags)?),
        );
    }
    if let Some(work_id) = patch.work_id {
        set(&mut assignments, &mut values, "work_id", Box::new(work_id));
    }
    if let Some(layer) = patch.layer {
        set(
            &mut assignments,
            &mut values,
            "layer",
            Box::new(layer.as_str().to_owned()),
        );
    }
    if let Some(aliases) = patch.aliases {
        set(
            &mut assignments,
            &mut values,
            "aliases",
            Box::new(serde_json::to_string(&clean_aliases(aliases))?),
        );
    }
    if let Some(prompt) = patch.prompt {
        let prompt = prompt
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty());
        set(&mut assignments, &mut values, "prompt", Box::new(prompt));
    }
    if let Some(state) = patch.state {
        set(
            &mut assignments,
            &mut values,
            "state",
            Box::new(state.as_str().to_owned()),
        );
    }
    if let Some(basis) = patch.prompt_basis {
        set(
            &mut assignments,
            &mut values,
            "prompt_basis",
            Box::new(basis),
        );
    }

    if assignments.is_empty() {
        return get(conn, id)?.ok_or_else(|| unknown_note(id));
    }

    set(
        &mut assignments,
        &mut values,
        "updated_at",
        Box::new(at.to_owned()),
    );
    values.push(Box::new(id.to_owned()));

    let sql = format!(
        "UPDATE note SET {} WHERE id = ?{}",
        assignments.join(", "),
        values.len()
    );
    let params = rusqlite::params_from_iter(values.iter().map(AsRef::as_ref));

    if conn.execute(&sql, params)? == 0 {
        return Err(unknown_note(id));
    }

    get(conn, id)?.ok_or_else(|| unknown_note(id))
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    if conn.execute("DELETE FROM note WHERE id = ?1", params![id])? == 0 {
        return Err(unknown_note(id));
    }
    Ok(())
}

/// Every tag in use in a profile, with how often it appears.
pub fn tags(conn: &Connection, profile_id: &str) -> Result<Vec<(String, i64)>> {
    let mut statement = conn.prepare(
        "SELECT json_each.value AS tag, count(*) AS uses
         FROM note, json_each(note.tags)
         WHERE note.profile_id = ?1
         GROUP BY tag
         ORDER BY uses DESC, tag",
    )?;

    let rows = statement.query_map(params![profile_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// What promoting a note asks for: the kind of work it becomes, and the title
/// it goes by.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Promotion {
    pub kind: String,
    pub title: String,
}

/// What a promotion made: the work, its first version, and the trash entry
/// the note went to - none for a material note, which stays, used by the new
/// work (ADR 0045).
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Promoted {
    pub work_id: String,
    pub version_id: String,
    pub deletion_id: Option<String>,
}

/// The ids one promotion mints, sharing the moment of the gesture.
#[derive(Debug, Clone)]
pub struct PromotionIds {
    pub work: Minted,
    pub version: Minted,
    pub deletion: Minted,
}

impl PromotionIds {
    pub fn fresh() -> Self {
        let at = now();
        let mint = || Minted::of(uuid::Uuid::new_v4().to_string(), at.clone());
        Self {
            work: mint(),
            version: mint(),
            deletion: mint(),
        }
    }
}

/// Turn a note into a work whose first version is the note's body.
///
/// An idea that grew up is not an idea beside the work any more: keeping the
/// note would leave the same text in two places, one of them edited and the
/// other not (ADR 0001). So the body moves - into the first version of the
/// kind's first role that is the work itself - and the note goes to the
/// trash, where it can be restored like any other deletion.
///
/// Except material (ADR 0045): an idea or a phrase of a kind the profile marks
/// material is spent, not moved. It stays in the bank, used and tied to the
/// new work, so the bank knows it went - the one thing a bank is for.
///
/// Its tags stay behind with it. A note's vocabulary and a work's are
/// different vocabularies (see `work_tags`), and carrying "reference" onto a
/// song would be the app guessing at a connection nobody made.
///
/// One unit for all three rows: a work made while the note failed to move
/// would be the duplicate this exists to prevent.
pub fn promote(
    conn: &Connection,
    profile_id: &str,
    note_id: &str,
    promotion: Promotion,
    ids: &PromotionIds,
) -> Result<Promoted> {
    crate::db::unit::atomically(conn, |tx| {
        promote_in(tx, profile_id, note_id, promotion, ids)
    })
}

fn promote_in(
    tx: &Connection,
    profile_id: &str,
    note_id: &str,
    promotion: Promotion,
    ids: &PromotionIds,
) -> Result<Promoted> {
    let found = get(tx, note_id)?.ok_or_else(|| unknown_note(note_id))?;
    if found.profile_id != profile_id {
        return Err(unknown_note(note_id));
    }
    let title = promotion.title.trim();
    if title.is_empty() {
        return Err(Error::refused("note.promoteNeedsTitle"));
    }

    let config = crate::profile::config_for(tx, profile_id)?;
    let kind = config.require_kind(&promotion.kind)?;
    let role = kind
        .version_roles
        .iter()
        .find(|role| role.counts_as_a_version())
        .map(|role| role.key.clone())
        .ok_or_else(|| {
            Error::refused("note.promoteNoRole").param(
                "kind",
                serde_json::to_value(&kind.label).unwrap_or_default(),
            )
        })?;

    let work = crate::work::create_minted(
        tx,
        profile_id,
        crate::work::NewWork {
            kind: promotion.kind,
            title: title.to_owned(),
            ..crate::work::NewWork::default()
        },
        ids.work.clone(),
    )?;
    let version_id = crate::work::version::create_minted(
        tx,
        &work.id,
        crate::work::version::NewVersion {
            role,
            body: found.body,
            label: None,
            meta: None,
            make_current: true,
            parent_version_id: None,
            trial_id: None,
        },
        ids.version.clone(),
    )?
    .id;
    let material = config
        .note_kind(&found.kind)
        .is_some_and(|kind| kind.material);
    let deletion_id = if material {
        update_at(
            tx,
            note_id,
            NotePatch {
                state: Some(NoteState::Used),
                work_id: Some(Some(work.id.clone())),
                ..NotePatch::default()
            },
            ids.work.at(),
        )?;
        None
    } else {
        Some(crate::trash::discard_minted(
            tx,
            crate::trash::Entity::Note,
            note_id,
            ids.deletion.clone(),
        )?)
    };

    Ok(Promoted {
        work_id: work.id,
        version_id,
        deletion_id,
    })
}

fn unknown_note(id: &str) -> Error {
    Error::not_found("note", id)
}

struct RawNote {
    id: String,
    profile_id: String,
    work_id: Option<String>,
    kind: String,
    title: Option<String>,
    body: String,
    tags: String,
    created_at: String,
    updated_at: String,
    layer: String,
    aliases: String,
    prompt: Option<String>,
    prompt_basis: Option<String>,
    state: String,
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawNote> {
    Ok(RawNote {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        work_id: row.get(2)?,
        kind: row.get(3)?,
        title: row.get(4)?,
        body: row.get(5)?,
        tags: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        layer: row.get(9)?,
        aliases: row.get(10)?,
        prompt: row.get(11)?,
        prompt_basis: row.get(12)?,
        state: row.get(13)?,
    })
}

impl RawNote {
    fn into_note(self) -> Result<Note> {
        Ok(Note {
            tags: serde_json::from_str(&self.tags)?,
            layer: Layer::parse(&self.layer)?,
            state: NoteState::parse(&self.state)?,
            aliases: serde_json::from_str(&self.aliases)?,
            id: self.id,
            profile_id: self.profile_id,
            work_id: self.work_id,
            kind: self.kind,
            title: self.title,
            body: self.body,
            created_at: self.created_at,
            updated_at: self.updated_at,
            prompt: self.prompt,
            prompt_basis: self.prompt_basis,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::work::{self, NewWork};

    fn note(body: &str, tags: &[&str]) -> NewNote {
        NewNote {
            body: body.into(),
            kind: None,
            title: None,
            work_id: None,
            tags: tags.iter().map(|t| (*t).to_owned()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn a_note_defaults_to_the_plain_kind() {
        let (conn, profile_id) = fixtures::workspace();

        let note = create(&conn, &profile_id, note("a thought", &[])).unwrap();

        assert_eq!(note.kind, "note");
        assert!(note.tags.is_empty());
    }

    #[test]
    fn tags_round_trip_through_the_database() {
        let (conn, profile_id) = fixtures::workspace();

        let created = create(&conn, &profile_id, note("tagged", &["idea", "winter"])).unwrap();
        let reloaded = get(&conn, &created.id).unwrap().unwrap();

        assert_eq!(reloaded.tags, vec!["idea", "winter"]);
    }

    #[test]
    fn filtering_by_tag_matches_whole_tags_only() {
        let (conn, profile_id) = fixtures::workspace();
        create(&conn, &profile_id, note("exact", &["win"])).unwrap();
        create(&conn, &profile_id, note("longer", &["winter"])).unwrap();

        let found = list(
            &conn,
            &profile_id,
            &NoteFilter {
                tag: Some("win".into()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(found.len(), 1, "`winter` must not match the tag `win`");
        assert_eq!(found[0].body, "exact");
    }

    #[test]
    fn search_covers_the_title_and_the_body() {
        let (conn, profile_id) = fixtures::workspace();
        let mut titled = note("unrelated body", &[]);
        titled.title = Some("Winter sketch".into());
        create(&conn, &profile_id, titled).unwrap();
        create(&conn, &profile_id, note("something about winter", &[])).unwrap();
        create(&conn, &profile_id, note("nothing relevant", &[])).unwrap();

        let found = list(
            &conn,
            &profile_id,
            &NoteFilter {
                search: Some("winter".into()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(found.len(), 2);
    }

    #[test]
    fn a_note_can_be_attached_to_a_work_and_filtered_by_it() {
        let (conn, profile_id) = fixtures::workspace();
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
        let mut attached = note("about this song", &[]);
        attached.work_id = Some(work.id.clone());
        create(&conn, &profile_id, attached).unwrap();
        create(&conn, &profile_id, note("loose", &[])).unwrap();

        let found = list(
            &conn,
            &profile_id,
            &NoteFilter {
                work_id: Some(work.id.clone()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].work_id.as_deref(), Some(work.id.as_str()));
    }

    #[test]
    fn update_replaces_the_whole_tag_set() {
        let (conn, profile_id) = fixtures::workspace();
        let created = create(&conn, &profile_id, note("body", &["old"])).unwrap();

        let updated = update(
            &conn,
            &created.id,
            NotePatch {
                tags: Some(vec!["new".into(), "fresh".into()]),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(updated.tags, vec!["new", "fresh"]);
    }

    #[test]
    fn update_can_clear_the_title() {
        let (conn, profile_id) = fixtures::workspace();
        let mut titled = note("body", &[]);
        titled.title = Some("Working title".into());
        let created = create(&conn, &profile_id, titled).unwrap();

        let updated = update(
            &conn,
            &created.id,
            NotePatch {
                title: Some(None),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(updated.title.is_none());
    }

    #[test]
    fn tags_are_counted_across_the_profile() {
        let (conn, profile_id) = fixtures::workspace();
        create(&conn, &profile_id, note("one", &["idea", "winter"])).unwrap();
        create(&conn, &profile_id, note("two", &["idea"])).unwrap();

        let tags = tags(&conn, &profile_id).unwrap();

        assert_eq!(tags[0], ("idea".to_owned(), 2));
        assert_eq!(tags[1], ("winter".to_owned(), 1));
    }

    fn promote(
        conn: &mut Connection,
        profile_id: &str,
        id: &str,
        kind: &str,
        title: &str,
    ) -> Result<Promoted> {
        let tx = conn.transaction().unwrap();
        let promoted = promote_in(
            &tx,
            profile_id,
            id,
            Promotion {
                kind: kind.into(),
                title: title.into(),
            },
            &PromotionIds::fresh(),
        )?;
        tx.commit().unwrap();
        Ok(promoted)
    }

    #[test]
    fn a_promoted_note_becomes_the_first_version_of_a_work() {
        let (mut conn, profile_id) = fixtures::workspace();
        let idea = create(
            &conn,
            &profile_id,
            note("a comma in the rock", &["geology"]),
        )
        .unwrap();

        let promoted = promote(&mut conn, &profile_id, &idea.id, "song", "  Graphite  ").unwrap();

        let made = work::get(&conn, &promoted.work_id).unwrap().unwrap();
        assert_eq!(made.title, "Graphite", "the title is trimmed");
        assert_eq!(made.kind, "song");
        assert_eq!(
            made.current_version_id.as_deref(),
            Some(promoted.version_id.as_str())
        );
        let body = crate::work::version::get(&conn, &promoted.version_id)
            .unwrap()
            .unwrap();
        assert_eq!(body.body, "a comma in the rock");
        assert!(
            made.tags.is_empty(),
            "a note's tags are a different vocabulary from a work's"
        );
    }

    #[test]
    fn a_promoted_note_leaves_for_the_trash_and_can_come_back() {
        let (mut conn, profile_id) = fixtures::workspace();
        let idea = create(&conn, &profile_id, note("one text, one place", &[])).unwrap();

        let promoted = promote(&mut conn, &profile_id, &idea.id, "song", "Place").unwrap();

        assert!(
            get(&conn, &idea.id).unwrap().is_none(),
            "the body lives in the version now, not twice"
        );
        crate::trash::restore(&conn, promoted.deletion_id.as_deref().unwrap()).unwrap();
        assert_eq!(
            get(&conn, &idea.id).unwrap().unwrap().body,
            "one text, one place"
        );
    }

    #[test]
    fn a_material_note_is_spent_not_moved() {
        let (mut conn, profile_id) = fixtures::workspace();
        let phrase = create(
            &conn,
            &profile_id,
            NewNote {
                body: "a lighthouse keeps the hours".into(),
                kind: Some("phrase".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let promoted = promote(&mut conn, &profile_id, &phrase.id, "song", "Lighthouse").unwrap();

        assert!(
            promoted.deletion_id.is_none(),
            "a phrase does not go to the trash"
        );
        let kept = get(&conn, &phrase.id)
            .unwrap()
            .expect("the phrase stays in the bank");
        assert_eq!(kept.state, NoteState::Used);
        assert_eq!(kept.work_id.as_deref(), Some(promoted.work_id.as_str()));
        let version = crate::work::version::get(&conn, &promoted.version_id)
            .unwrap()
            .unwrap();
        assert_eq!(version.body, "a lighthouse keeps the hours");
    }

    #[test]
    fn the_lines_are_listed_apart_from_the_pages() {
        let (conn, profile_id) = fixtures::workspace();
        let mut phrase = note("a line", &[]);
        phrase.kind = Some("phrase".into());
        create(&conn, &profile_id, phrase).unwrap();
        create(&conn, &profile_id, note("a page", &[])).unwrap();

        let pages = list(
            &conn,
            &profile_id,
            &NoteFilter {
                line: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
        let lines = list(
            &conn,
            &profile_id,
            &NoteFilter {
                line: Some(true),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(
            pages.iter().map(|n| n.body.as_str()).collect::<Vec<_>>(),
            ["a page"]
        );
        assert_eq!(
            lines.iter().map(|n| n.body.as_str()).collect::<Vec<_>>(),
            ["a line"]
        );
    }

    #[test]
    fn a_promotion_to_nothing_changes_nothing() {
        let (mut conn, profile_id) = fixtures::workspace();
        let idea = create(&conn, &profile_id, note("still here", &[])).unwrap();

        assert!(promote(&mut conn, &profile_id, &idea.id, "limerick", "Title").is_err());
        assert!(promote(&mut conn, &profile_id, &idea.id, "song", "   ").is_err());

        assert!(get(&conn, &idea.id).unwrap().is_some());
        let works: i64 = conn
            .query_row("SELECT count(*) FROM work", [], |row| row.get(0))
            .unwrap();
        assert_eq!(works, 0, "a refused promotion made a work anyway");
    }

    #[test]
    fn deleting_an_unknown_note_fails() {
        let (conn, _) = fixtures::workspace();

        assert!(delete(&conn, "nope").is_err());
    }
}
