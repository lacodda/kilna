//! The trash: deletion moves rows aside instead of destroying them.
//!
//! A deleted row is captured verbatim — column names and values as they were —
//! and put in `deletion`. Restoring inserts it back under the same id, so
//! anything still pointing at it keeps pointing at it.
//!
//! The live tables are left untouched by this: no query anywhere has to filter
//! deleted rows out, because deleted rows are not there. That is the whole
//! reason for a snapshot table rather than a `deleted_at` column.

use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::db::unit::atomically;
use crate::error::{Error, Result};
use crate::minted::Minted;

/// What can be thrown away, and what each kind takes with it.
///
/// The strings are stored in the database and travel to the frontend, so they
/// are part of the format rather than an implementation detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Entity {
    Work,
    Version,
    Score,
    Release,
    Note,
    Collection,
    Scene,
    Cut,
    Comment,
    /// A brick of the style dictionary, with the pictures it was described from.
    Style,
}

impl Entity {
    /// Every kind of thing the trash holds. What a gate iterates rather than
    /// a list of its own that someone has to remember to extend.
    pub const ALL: [Entity; 10] = [
        Entity::Work,
        Entity::Version,
        Entity::Score,
        Entity::Release,
        Entity::Note,
        Entity::Collection,
        Entity::Scene,
        Entity::Cut,
        Entity::Comment,
        Entity::Style,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::Version => "version",
            Self::Score => "score",
            Self::Release => "release",
            Self::Note => "note",
            Self::Collection => "collection",
            Self::Scene => "scene",
            Self::Cut => "cut",
            Self::Comment => "comment",
            Self::Style => "style",
        }
    }

    /// The entity a stored name means. Public because the operations log stores
    /// the same names and the replay has to read them back.
    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "work" => Ok(Self::Work),
            "version" => Ok(Self::Version),
            "score" => Ok(Self::Score),
            "release" => Ok(Self::Release),
            "note" => Ok(Self::Note),
            "collection" => Ok(Self::Collection),
            "scene" => Ok(Self::Scene),
            "cut" => Ok(Self::Cut),
            "comment" => Ok(Self::Comment),
            "style" => Ok(Self::Style),
            other => Err(Error::Other(format!("unknown trash entity `{other}`"))),
        }
    }

    /// The table the entity itself lives in.
    fn table(self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::Version => "work_version",
            Self::Score => "work_score",
            Self::Release => "release",
            Self::Note => "note",
            Self::Collection => "collection",
            Self::Scene => "scene",
            Self::Cut => "cut",
            Self::Comment => "comment",
            Self::Style => "style_brick",
        }
    }
}

/// One entry in the trash, as the screen shows it.
#[derive(Debug, Clone, Serialize)]
pub struct Deletion {
    pub id: String,
    pub entity: Entity,
    pub entity_id: String,
    /// What it was: a work's title, a note's opening words.
    pub label: String,
    /// Where it came from — the parent work's title, when it had one.
    pub origin: Option<String>,
    /// The work it is, or the work it belonged to: what the screen draws the
    /// entry's cover from, so a version sits beside its song in the song's
    /// colour. `None` for what hangs off no work - a style, a collection, a
    /// note about nothing in particular.
    pub work_id: Option<String>,
    /// `manual`, or the entity that took it down with it.
    pub reason: String,
    pub deleted_at: String,
    /// Whether restoring would work right now. A version whose work is itself in
    /// the trash cannot come back alone.
    pub restorable: bool,
}

/// Tables captured for an entity, parents before children.
///
/// Order matters on the way back: a version cannot be inserted before its work.
/// The same order reversed is not needed on the way out, because the snapshot is
/// taken before anything is removed.
fn cascade(entity: Entity) -> &'static [Capture] {
    match entity {
        // Everything that hangs off a work goes with it, and comes back with it.
        Entity::Work => &[
            Capture {
                table: "work",
                key: "id",
            },
            Capture {
                table: "work_version",
                key: "work_id",
            },
            Capture {
                table: "work_score",
                key: "work_id",
            },
            Capture {
                table: "release",
                key: "work_id",
            },
            Capture {
                table: "note",
                key: "work_id",
            },
            Capture {
                table: "asset",
                key: "work_id",
            },
            // The storyboard belongs to the work (ADR 0020) and goes with it.
            Capture {
                table: "scene",
                key: "work_id",
            },
            // And what those scenes are about goes with the scenes. Keyed
            // through the board rather than by a column of its own: the row
            // names a scene, and the scenes are the work's.
            Capture {
                table: "scene_note",
                key: "scene_id IN (SELECT id FROM scene WHERE work_id = ?)",
            },
            // A link is about two works and cascades from either; the
            // snapshot takes it from both sides, so a video restored gets
            // its song back and a song restored gets its videos back.
            Capture {
                table: "work_link",
                key: "work_id",
            },
            Capture {
                table: "work_link",
                key: "source_id",
            },
            // The stretches of a splice, from both sides for the reason the
            // link gives: a short restored gets its cuts back, and a video
            // restored gets back the marks the shorts made in it.
            Capture {
                table: "cut",
                key: "work_id",
            },
            Capture {
                table: "cut",
                key: "source_id",
            },
            // What the audience said about it. The schema's cascade takes the
            // comments down with the work; captured so they come back with it.
            Capture {
                table: "comment",
                key: "work_id",
            },
        ],
        Entity::Version => &[Capture {
            table: "work_version",
            key: "id",
        }],
        Entity::Score => &[Capture {
            table: "work_score",
            key: "id",
        }],
        Entity::Release => &[
            Capture {
                table: "release",
                key: "id",
            },
            Capture {
                table: "asset",
                key: "release_id",
            },
        ],
        Entity::Note => &[
            Capture {
                table: "note",
                key: "id",
            },
            // What a scene said this note was in. Captured so a character
            // restored is back in the scenes that named them, rather than
            // coming back as a note nothing points at.
            Capture {
                table: "scene_note",
                key: "note_id",
            },
        ],
        Entity::Scene => &[
            Capture {
                table: "scene",
                key: "id",
            },
            Capture {
                table: "scene_note",
                key: "scene_id",
            },
        ],
        // A stretch of a splice is one row and hangs nothing off itself.
        Entity::Cut => &[Capture {
            table: "cut",
            key: "id",
        }],
        // A comment is one row and hangs nothing off itself.
        Entity::Comment => &[Capture {
            table: "comment",
            key: "id",
        }],
        // A brick and the pictures it was described from. The pictures are
        // what the brick is recognised by, so they come back with it; their
        // files stay on disk while the entry is in the trash and go when it
        // is purged (`forget_files`).
        Entity::Style => &[
            Capture {
                table: "style_brick",
                key: "id",
            },
            Capture {
                table: "asset",
                key: "style_brick_id",
            },
        ],
        // Works are not deleted with a collection — they are only let go of. The
        // membership they lose is captured separately, under `members`.
        Entity::Collection => &[Capture {
            table: "collection",
            key: "id",
        }],
    }
}

/// Snapshot key holding the ids of works that belonged to a deleted collection.
///
/// Membership is a column on `work`, not a row of its own, so the generic
/// capture cannot see it: deleting a collection sets those columns to NULL and
/// the link is gone. Restoring puts each id back, skipping works that have since
/// been moved elsewhere or deleted.
const MEMBERS: &str = "members";

/// One table to snapshot, and the column that ties it to the entity being
/// deleted.
struct Capture {
    table: &'static str,
    /// The column the row is found by — or, where the tie is indirect, a
    /// whole condition naming the value once as `?`. A work's scenes are
    /// found by `work_id`; what those scenes are *about* is found through
    /// the board, because the row names a scene and not the work.
    key: &'static str,
}

impl Capture {
    /// The `WHERE` clause this capture reads and deletes by.
    fn condition(&self) -> String {
        if self.key.contains('?') {
            self.key.to_owned()
        } else {
            format!("{} = ?", self.key)
        }
    }
}

/// Delete an entity, keeping a snapshot of it and everything beneath it.
///
/// Returns the id of the trash entry, which is what an undo needs.
pub fn discard(conn: &Connection, entity: Entity, id: &str) -> Result<String> {
    discard_minted(conn, entity, id, Minted::fresh())
}

/// Move an entity to the trash with the entry's id and moment already decided.
///
/// The seam a replay comes back through: a restore names the trash entry by id,
/// so a rebuilt workspace has to bury the row under the same one. See ADR 0014.
///
/// One unit: the snapshot and the deletion land together, alone or inside the
/// gesture that discards - see [`crate::db::unit`].
pub fn discard_minted(
    conn: &Connection,
    entity: Entity,
    id: &str,
    minted: Minted,
) -> Result<String> {
    atomically(conn, |tx| discard_in(tx, entity, id, &minted))
}

/// The body of a discard: the snapshot, the entry and the deletion.
fn discard_in(tx: &Connection, entity: Entity, id: &str, minted: &Minted) -> Result<String> {
    let (label, origin, profile_id) = describe(tx, entity, id)?;

    // Snapshot first, delete second: the rows have to be read while they exist.
    let mut snapshot = Map::new();
    for capture in cascade(entity) {
        let rows = read_rows(tx, capture.table, &capture.condition(), id)?;
        if rows.is_empty() {
            continue;
        }
        // A table captured from two sides lands under one key: the snapshot
        // is by table, and a restore reads it by table.
        match snapshot.get_mut(capture.table) {
            Some(Value::Array(existing)) => existing.extend(rows),
            _ => {
                snapshot.insert(capture.table.to_owned(), Value::Array(rows));
            }
        }
    }

    // The row itself must have been there; the cascade tables may legitimately
    // be empty.
    if !snapshot.contains_key(entity.table()) {
        return Err(Error::not_found(entity_label(entity), id));
    }

    if entity == Entity::Collection {
        snapshot.insert(MEMBERS.to_owned(), Value::Array(members(tx, id)?));
    }

    let deletion_id = minted.id().to_owned();
    tx.execute(
        "INSERT INTO deletion (id, profile_id, entity, entity_id, label, origin, reason, snapshot, deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'manual', ?7, ?8)",
        params![
            deletion_id,
            profile_id,
            entity.as_str(),
            id,
            label,
            origin,
            Value::Object(snapshot).to_string(),
            minted.at(),
        ],
    )?;

    // One delete; the schema's ON DELETE CASCADE takes the rest, exactly as it
    // did before the trash existed.
    tx.execute(
        &format!("DELETE FROM {} WHERE id = ?1", entity.table()),
        params![id],
    )?;

    Ok(deletion_id)
}

/// Move several rows to the trash at once: a gesture that made several rows
/// is taken back as one gesture - the works of a batch, the frame a board was
/// built with - so the undo has one operation to write and the trash one
/// moment to group by.
///
/// Each still gets its own trash entry - an entry snapshots one entity, so
/// each stays separately restorable. Returns `(id, entry)` for every row
/// actually discarded, in the order given, and what could not be discarded
/// with the reason: one row failing (already gone, most likely) does not take
/// the others with it, and its half-written snapshot is rolled back alone.
pub fn discard_batch(
    conn: &Connection,
    entity: Entity,
    ids: &[String],
    minted_ids: &[Minted],
) -> Result<Discarded> {
    atomically(conn, |tx| {
        let mut discarded = Discarded::default();
        for (id, minted) in ids.iter().zip(minted_ids) {
            match discard_minted(tx, entity, id, minted.clone()) {
                Ok(entry_id) => discarded.done.push((id.clone(), entry_id)),
                Err(cause) => discarded.failed.push((id.clone(), cause)),
            }
        }
        Ok(discarded)
    })
}

/// What a batch discard did.
#[derive(Debug, Default)]
pub struct Discarded {
    /// `(id, trash entry)` for every row that went.
    pub done: Vec<(String, String)>,
    /// Every row that stayed, and why.
    pub failed: Vec<(String, Error)>,
}

/// Put a trashed entity back and forget the entry, as one unit - a restore
/// that failed halfway would leave rows back that the entry still claims.
pub fn restore(conn: &Connection, deletion_id: &str) -> Result<()> {
    atomically(conn, |tx| restore_in(tx, deletion_id))
}

/// The body of [`restore`].
fn restore_in(tx: &Connection, deletion_id: &str) -> Result<()> {
    let (entity, entity_id, snapshot): (String, String, String) = tx
        .query_row(
            "SELECT entity, entity_id, snapshot FROM deletion WHERE id = ?1",
            params![deletion_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| Error::not_found("deletion", deletion_id))?;

    let entity = Entity::parse(&entity)?;
    let snapshot: Map<String, Value> = serde_json::from_str(&snapshot)?;

    // Refuse rather than resurrect an orphan: if the work a version belongs to
    // is itself in the trash, the version has nowhere to go, and inserting it
    // would fail on a foreign key half-way through anyway.
    if let Some(missing) = missing_parent(tx, entity, &snapshot)? {
        return Err(Error::NotRestorable(missing));
    }

    let mut restored: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for capture in cascade(entity) {
        // A table captured from two sides was snapshotted once; it is put
        // back once.
        if !restored.insert(capture.table) {
            continue;
        }
        let Some(Value::Array(rows)) = snapshot.get(capture.table) else {
            continue;
        };
        for row in rows {
            let Value::Object(row) = row else {
                return Err(Error::Other("a trashed row is not an object".into()));
            };
            // A link is about two works. One restored while the other is
            // still gone has nothing to point at, and inserting it would fail
            // on the foreign key; it stays out, as the cascade left it, and
            // comes back with the other work if that one is restored.
            if capture.table == "work_link" && !link_has_both_sides(tx, row)? {
                continue;
            }
            insert_row(tx, capture.table, row)?;
        }
    }

    if let Some(Value::Array(members)) = snapshot.get(MEMBERS) {
        for member in members {
            let Value::String(work_id) = member else {
                continue;
            };
            // A work that was reassigned or deleted meanwhile keeps whatever it
            // has now: a restore returns the collection, it does not overrule
            // decisions taken since.
            tx.execute(
                "UPDATE work SET collection_id = ?1 WHERE id = ?2 AND collection_id IS NULL",
                params![entity_id, work_id],
            )?;
        }
    }

    tx.execute("DELETE FROM deletion WHERE id = ?1", params![deletion_id])?;

    Ok(())
}

/// Whether both works a trashed link joins are alive.
fn link_has_both_sides(conn: &Connection, row: &Map<String, Value>) -> Result<bool> {
    for side in ["work_id", "source_id"] {
        let Some(id) = row.get(side).and_then(Value::as_str) else {
            return Ok(false);
        };
        let alive: bool = conn
            .query_row("SELECT 1 FROM work WHERE id = ?1", params![id], |_| {
                Ok(true)
            })
            .optional()?
            .unwrap_or(false);
        if !alive {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Ids of the works currently in a collection.
fn members(conn: &Connection, collection_id: &str) -> Result<Vec<Value>> {
    let mut statement = conn.prepare("SELECT id FROM work WHERE collection_id = ?1")?;
    let rows = statement.query_map(params![collection_id], |row| {
        Ok(Value::String(row.get::<_, String>(0)?))
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Drop one entry for good.
///
/// Purging a work also drops the entries that only made sense underneath it: a
/// version whose work will never come back can never be restored either, and
/// leaving it in the trash is dead weight that only ever grows.
///
/// The files the entries held are removed once the purge is committed - see
/// [`forget_files`].
pub fn purge(conn: &Connection, deletion_id: &str) -> Result<()> {
    atomically(conn, |tx| {
        let (entity, entity_id): (String, String) = tx
            .query_row(
                "SELECT entity, entity_id FROM deletion WHERE id = ?1",
                params![deletion_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| Error::not_found("deletion", deletion_id))?;

        // The files of everything this purge forgets, read while the entries
        // that name them are still there.
        let mut files = snapshot_files(tx, "deletion.id = ?1", deletion_id)?;
        tx.execute("DELETE FROM deletion WHERE id = ?1", params![deletion_id])?;

        if Entity::parse(&entity)? == Entity::Work {
            files.extend(snapshot_files(tx, CHILDREN_OF_WORK, &entity_id)?);
            tx.execute(
                &format!("DELETE FROM deletion WHERE {CHILDREN_OF_WORK}"),
                params![entity_id],
            )?;
        }

        forget_files(tx, files);
        Ok(())
    })
}

/// The entries that hang off a purged work: they could only ever be restored
/// into it, so they go with it. Qualified by table, because it is also read
/// beside `json_each`, whose own columns include an `id`.
const CHILDREN_OF_WORK: &str =
    "deletion.entity IN ('version', 'score', 'release', 'note', 'comment')
     AND json_extract(deletion.snapshot, '$.' || (
         CASE deletion.entity
             WHEN 'version' THEN 'work_version'
             WHEN 'score' THEN 'work_score'
             WHEN 'release' THEN 'release'
             WHEN 'comment' THEN 'comment'
             ELSE 'note'
         END
     ) || '[0].work_id') = ?1";

/// Empty the trash for a profile. Returns how many entries went.
pub fn empty(conn: &Connection, profile_id: &str) -> Result<usize> {
    atomically(conn, |tx| {
        let files = snapshot_files(tx, "deletion.profile_id = ?1", profile_id)?;
        let gone = tx.execute(
            "DELETE FROM deletion WHERE profile_id = ?1",
            params![profile_id],
        )?;
        forget_files(tx, files);
        Ok(gone)
    })
}

/// The files the asset rows in some entries' snapshots point at.
///
/// A trashed work, release or style keeps its asset rows in the snapshot and
/// its files on disk, so that a restore puts back a picture rather than a
/// broken link. Those files are what purging has to account for.
fn snapshot_files(conn: &Connection, condition: &str, value: &str) -> Result<Vec<String>> {
    // `condition` comes from this file, never from input.
    let mut statement = conn.prepare(&format!(
        "SELECT json_extract(a.value, '$.path')
         FROM deletion, json_each(deletion.snapshot, '$.asset') AS a
         WHERE {condition}"
    ))?;
    let paths = statement
        .query_map(params![value], |row| row.get::<_, Option<String>>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(paths.into_iter().flatten().collect())
}

/// Remove the files a purge let go of, unless something still names them -
/// once the purge is committed.
///
/// Before this, purging only forgot the snapshot: the pictures of a purged
/// work, release or style stayed in `media/` with nothing pointing at them,
/// for good. A path is kept while a live asset row or another entry of the
/// trash still names it - a cloned video shares its files with the original,
/// and two entries can hold the same picture.
///
/// After the commit, because a file removed for a purge that then rolled back
/// could not be put back. `empty` used to remove them inside its transaction.
fn forget_files(conn: &Connection, paths: Vec<String>) {
    crate::db::unit::after_commit(conn, move |conn| {
        let mut seen = std::collections::BTreeSet::new();
        for path in &paths {
            if !seen.insert(path.as_str()) {
                continue;
            }
            let named: rusqlite::Result<i64> = conn.query_row(
                "SELECT (SELECT COUNT(*) FROM asset WHERE path = ?1)
                      + (SELECT COUNT(*) FROM deletion, json_each(deletion.snapshot, '$.asset') AS a
                         WHERE json_extract(a.value, '$.path') = ?1)",
                params![path],
                |row| row.get(0),
            );
            match named {
                Ok(0) => {}
                Ok(_) => continue,
                Err(cause) => {
                    crate::log::error(
                        "trash",
                        &format!("could not tell whether {path} is still named: {cause}"),
                    );
                    continue;
                }
            }
            match std::fs::remove_file(path) {
                Ok(()) => {}
                // Already gone is what was wanted.
                Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => {}
                // Said, not raised: the entry is gone either way, and a file
                // left behind is not a reason to tell a person the purge failed.
                Err(cause) => {
                    crate::log::warn("trash", &format!("could not remove {path}: {cause}"));
                }
            }
        }
    });
}

/// Everything in a profile's trash, newest first.
pub fn list(conn: &Connection, profile_id: &str) -> Result<Vec<Deletion>> {
    let mut statement = conn.prepare(
        "SELECT id, entity, entity_id, label, origin, reason, snapshot, deleted_at
         FROM deletion WHERE profile_id = ?1 ORDER BY deleted_at DESC, rowid DESC",
    )?;

    let rows = statement
        .query_map(params![profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    rows.into_iter()
        .map(
            |(id, entity, entity_id, label, origin, reason, snapshot, deleted_at)| {
                let entity = Entity::parse(&entity)?;
                let snapshot: Map<String, Value> = serde_json::from_str(&snapshot)?;
                let work_id = match entity {
                    Entity::Work => Some(entity_id.clone()),
                    _ => work_in(entity, &snapshot),
                };
                Ok(Deletion {
                    restorable: missing_parent(conn, entity, &snapshot)?.is_none(),
                    id,
                    entity,
                    entity_id,
                    label,
                    origin,
                    work_id,
                    reason,
                    deleted_at,
                })
            },
        )
        .collect()
}

/// The work a trashed child belonged to, read out of its snapshot.
///
/// Once the row is deleted the snapshot is the only place that still knows, and
/// the journal needs it to file the entry under the right work. `None` when the
/// entity has no work — a standalone note — or when the entry is not there.
pub fn snapshot_work_id(conn: &Connection, entity: Entity, entity_id: &str) -> Option<String> {
    let snapshot: String = conn
        .query_row(
            "SELECT snapshot FROM deletion
              WHERE entity = ?1 AND entity_id = ?2
              ORDER BY deleted_at DESC, rowid DESC LIMIT 1",
            params![entity.as_str(), entity_id],
            |row| row.get(0),
        )
        .optional()
        .ok()
        .flatten()?;

    let snapshot: Map<String, Value> = serde_json::from_str(&snapshot).ok()?;
    work_in(entity, &snapshot)
}

/// The `work_id` of the row a snapshot captured, when it has one.
fn work_in(entity: Entity, snapshot: &Map<String, Value>) -> Option<String> {
    match snapshot.get(entity.table())?.as_array()?.first()? {
        Value::Object(row) => match row.get("work_id")? {
            Value::String(work_id) => Some(work_id.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// The parent an entry needs, if that parent is not there.
///
/// Returns the sentence to refuse with, or `None` when the restore can proceed.
fn missing_parent(
    conn: &Connection,
    entity: Entity,
    snapshot: &Map<String, Value>,
) -> Result<Option<String>> {
    let parents: &[&str] = match entity {
        // These stand on their own; the profile they need is checked by the
        // insert itself.
        Entity::Work | Entity::Collection | Entity::Style => return Ok(None),
        Entity::Version
        | Entity::Score
        | Entity::Release
        | Entity::Note
        | Entity::Scene
        | Entity::Comment => &["work_id"],
        // A stretch of a splice names two works and needs both: without the
        // short it belongs to nothing, and without the video it is seconds of
        // nowhere. Either being gone is the same refusal.
        Entity::Cut => &["work_id", "source_id"],
    };

    let rows = match snapshot.get(entity.table()) {
        Some(Value::Array(rows)) => rows,
        _ => return Ok(None),
    };
    let Some(Value::Object(row)) = rows.first() else {
        return Ok(None);
    };

    for parent in parents {
        // A note or a comment need not belong to a work at all.
        let Some(Value::String(work_id)) = row.get(*parent) else {
            continue;
        };
        let exists: bool = conn
            .query_row("SELECT 1 FROM work WHERE id = ?1", params![work_id], |_| {
                Ok(true)
            })
            .optional()?
            .unwrap_or(false);
        if !exists {
            return Ok(Some(
                "the work this belonged to is gone — restore it first, or this has nowhere to go"
                    .to_owned(),
            ));
        }
    }

    Ok(None)
}

/// A one-line name, where it came from, and the profile it belongs to.
fn describe(
    conn: &Connection,
    entity: Entity,
    id: &str,
) -> Result<(String, Option<String>, Option<String>)> {
    // Each entity is named by whatever it has that reads like a name, and placed
    // by its work's title when it hangs off one.
    let found = match entity {
        Entity::Work => conn
            .query_row(
                "SELECT title, NULL, profile_id FROM work WHERE id = ?1",
                params![id],
                describe_row,
            )
            .optional()?,
        Entity::Version => conn
            .query_row(
                "SELECT coalesce(v.label, v.role || ' ' || v.revision), w.title, w.profile_id
                 FROM work_version v JOIN work w ON w.id = v.work_id WHERE v.id = ?1",
                params![id],
                describe_row,
            )
            .optional()?,
        // Named by what it said, not by when it was said: `scored_at` is an RFC
        // 3339 instant, and a row reading "2026-08-17T20:06:45.0175882Z" tells
        // nobody which score they are about to put back.
        Entity::Score => conn
            .query_row(
                "SELECT coalesce(s.tier || ' · ', '') || cast(round(s.total, 1) AS TEXT),
                        w.title, w.profile_id
                 FROM work_score s JOIN work w ON w.id = s.work_id WHERE s.id = ?1",
                params![id],
                describe_row,
            )
            .optional()?,
        Entity::Release => conn
            .query_row(
                "SELECT coalesce(r.title, r.kind), w.title, w.profile_id
                 FROM release r JOIN work w ON w.id = r.work_id WHERE r.id = ?1",
                params![id],
                describe_row,
            )
            .optional()?,
        Entity::Note => conn
            .query_row(
                "SELECT coalesce(n.title, substr(n.body, 1, 80)), w.title, n.profile_id
                 FROM note n LEFT JOIN work w ON w.id = n.work_id WHERE n.id = ?1",
                params![id],
                describe_row,
            )
            .optional()?,
        // Named by what was said and who said it: "Loved the bridge — @anna".
        Entity::Comment => conn
            .query_row(
                "SELECT substr(c.body, 1, 80) || coalesce(' — ' || nullif(c.author, ''), ''),
                        w.title, c.profile_id
                 FROM comment c LEFT JOIN work w ON w.id = c.work_id WHERE c.id = ?1",
                params![id],
                describe_row,
            )
            .optional()?,
        Entity::Collection => conn
            .query_row(
                "SELECT title, NULL, profile_id FROM collection WHERE id = ?1",
                params![id],
                describe_row,
            )
            .optional()?,
        // Named as the dictionary shows it: by its name, which is unique
        // within its type.
        Entity::Style => conn
            .query_row(
                "SELECT name, NULL, profile_id FROM style_brick WHERE id = ?1",
                params![id],
                describe_row,
            )
            .optional()?,
        // Named by its number and the part it plays against: "Scene 4 · chorus".
        Entity::Scene => conn
            .query_row(
                "SELECT 'Scene ' || s.position || coalesce(' · ' || nullif(s.section, ''), ''),
                        w.title, s.profile_id
                 FROM scene s JOIN work w ON w.id = s.work_id WHERE s.id = ?1",
                params![id],
                describe_row,
            )
            .optional()?,
        // Named by where it was taken from: "Harbour lights 0:48-1:01". The
        // person recognises a stretch by the video and the moment, which is
        // why the source's title is in the name and not only in the origin.
        Entity::Cut => conn
            .query_row(
                "SELECT src.title || ' ' ||
                        cast(cast(c.starts_at / 60 AS INTEGER) AS TEXT) || ':' ||
                        substr('0' || cast(cast(c.starts_at % 60 AS INTEGER) AS TEXT), -2) ||
                        '-' ||
                        cast(cast(c.ends_at / 60 AS INTEGER) AS TEXT) || ':' ||
                        substr('0' || cast(cast(c.ends_at % 60 AS INTEGER) AS TEXT), -2),
                        w.title, c.profile_id
                 FROM cut c JOIN work w ON w.id = c.work_id
                            JOIN work src ON src.id = c.source_id
                 WHERE c.id = ?1",
                params![id],
                describe_row,
            )
            .optional()?,
    };

    found.ok_or_else(|| Error::not_found(entity_label(entity), id))
}

fn describe_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(String, Option<String>, Option<String>)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
}

/// The word used when saying something was not found.
fn entity_label(entity: Entity) -> &'static str {
    match entity {
        Entity::Work => "work",
        Entity::Version => "version",
        Entity::Score => "score",
        Entity::Release => "release",
        Entity::Note => "note",
        Entity::Collection => "collection",
        Entity::Scene => "scene",
        Entity::Cut => "cut",
        Entity::Comment => "comment",
        Entity::Style => "style",
    }
}

/// Read whole rows as JSON objects, keyed by their real column names.
///
/// Reading by name rather than by position means a column added in a later
/// migration lands in the snapshot without this function being told about it.
fn read_rows(conn: &Connection, table: &str, condition: &str, value: &str) -> Result<Vec<Value>> {
    // `table` and `condition` come from the fixed cascade table above, never
    // from input, which is why they can be formatted into the SQL. The
    // condition names the value once, as the single `?` it carries.
    let mut statement = conn.prepare(&format!("SELECT * FROM {table} WHERE {condition}"))?;
    let columns: Vec<String> = statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect();

    let rows = statement.query_map(params![value], |row| {
        let mut object = Map::new();
        for (index, column) in columns.iter().enumerate() {
            object.insert(column.clone(), sql_to_json(row.get_ref(index)?));
        }
        Ok(Value::Object(object))
    })?;

    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Put one snapshotted row back.
fn insert_row(tx: &Connection, table: &str, row: &Map<String, Value>) -> Result<()> {
    let columns: Vec<&String> = row.keys().collect();
    let placeholders: Vec<String> = (1..=columns.len()).map(|n| format!("?{n}")).collect();
    let values: Vec<SqlValue> = columns
        .iter()
        .map(|column| json_to_sql(&row[*column]))
        .collect::<Result<Vec<_>>>()?;

    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({})",
        columns
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        placeholders.join(", ")
    );

    tx.execute(&sql, rusqlite::params_from_iter(values.iter()))?;
    Ok(())
}

/// A stored value as JSON. The schema is STRICT, so the types are the five
/// SQLite has and nothing else.
fn sql_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(number) => Value::from(number),
        ValueRef::Real(number) => Value::from(number),
        ValueRef::Text(bytes) => Value::String(String::from_utf8_lossy(bytes).into_owned()),
        ValueRef::Blob(bytes) => Value::String(String::from_utf8_lossy(bytes).into_owned()),
    }
}

fn json_to_sql(value: &Value) -> Result<SqlValue> {
    Ok(match value {
        Value::Null => SqlValue::Null,
        Value::Bool(flag) => SqlValue::Integer(i64::from(*flag)),
        Value::String(text) => SqlValue::Text(text.clone()),
        Value::Number(number) => match (number.as_i64(), number.as_f64()) {
            (Some(int), _) => SqlValue::Integer(int),
            (None, Some(float)) => SqlValue::Real(float),
            _ => return Err(Error::Other("a trashed number cannot be stored".into())),
        },
        other => {
            return Err(Error::Other(format!(
                "a trashed value has an unexpected shape: {other}"
            )));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection;
    use crate::db;
    use crate::note::{self, NewNote};
    use crate::profile;
    use crate::work::version::{self, NewVersion};
    use crate::work::{self, NewWork};

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    fn song(title: &str) -> NewWork {
        NewWork {
            kind: "song".into(),
            title: title.into(),
            ..NewWork::default()
        }
    }

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
    }

    #[test]
    fn discarding_a_work_takes_its_children_and_gives_them_all_back() {
        let (conn, profile_id) = workspace();
        let work = work::create(&conn, &profile_id, song("Winter road")).unwrap();
        version::create(
            &conn,
            &work.id,
            NewVersion {
                role: "lyrics".into(),
                body: "a first draft".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
        note::create(
            &conn,
            &profile_id,
            NewNote {
                body: "a thought".into(),
                kind: None,
                title: None,
                work_id: Some(work.id.clone()),
                tags: vec![],
            },
        )
        .unwrap();

        // Read back after the version was added: that is the state the trash
        // has to reproduce, not the one `create` returned.
        let before = work::get(&conn, &work.id).unwrap().unwrap();
        assert!(before.current_version_id.is_some());

        let entry = discard(&conn, Entity::Work, &work.id).unwrap();

        assert_eq!(count(&conn, "work"), 0);
        assert_eq!(count(&conn, "work_version"), 0, "the cascade took it");
        assert_eq!(count(&conn, "note"), 0);

        restore(&conn, &entry).unwrap();

        let back = work::get(&conn, &work.id)
            .unwrap()
            .expect("the work is back");
        assert_eq!(back.title, "Winter road");
        assert_eq!(
            back.current_version_id, before.current_version_id,
            "the pointer to the current version survived"
        );
        assert_eq!(count(&conn, "work_version"), 1);
        assert_eq!(count(&conn, "note"), 1);
        assert_eq!(count(&conn, "deletion"), 0, "the entry is spent");
    }

    #[test]
    fn a_restored_row_keeps_its_id_and_its_timestamps() {
        let (conn, profile_id) = workspace();
        let work = work::create(&conn, &profile_id, song("Unchanged")).unwrap();

        let entry = discard(&conn, Entity::Work, &work.id).unwrap();
        restore(&conn, &entry).unwrap();

        let back = work::get(&conn, &work.id).unwrap().unwrap();
        assert_eq!(back.id, work.id);
        assert_eq!(back.created_at, work.created_at);
        assert_eq!(
            back.updated_at, work.updated_at,
            "a round trip through the trash is not an edit"
        );
        assert_eq!(back.position, work.position);
    }

    #[test]
    fn a_version_cannot_come_back_while_its_work_is_still_in_the_trash() {
        let (conn, profile_id) = workspace();
        let work = work::create(&conn, &profile_id, song("Doomed")).unwrap();
        let draft = version::create(
            &conn,
            &work.id,
            NewVersion {
                role: "lyrics".into(),
                body: "words".into(),
                label: None,
                meta: None,
                make_current: false,
                parent_version_id: None,
            },
        )
        .unwrap();

        let version_entry = discard(&conn, Entity::Version, &draft.id).unwrap();
        let work_entry = discard(&conn, Entity::Work, &work.id).unwrap();

        // The error has to be the one that explains itself, not a foreign-key
        // failure from an insert that was attempted anyway: the difference is
        // invisible to `is_err`, and it is the whole point of the check.
        let refused = restore(&conn, &version_entry).unwrap_err();
        assert!(
            matches!(refused, Error::NotRestorable(_)),
            "an orphan must be refused with a sentence, got: {refused}"
        );

        let listed = list(&conn, &profile_id).unwrap();
        let version_row = listed
            .iter()
            .find(|entry| entry.id == version_entry)
            .unwrap();
        assert!(!version_row.restorable, "and the screen says so beforehand");

        // With the work back, the version can follow.
        restore(&conn, &work_entry).unwrap();
        restore(&conn, &version_entry).unwrap();
        assert_eq!(count(&conn, "work_version"), 1);
    }

    #[test]
    fn the_trash_lists_what_each_entry_was_and_where_it_came_from() {
        let (conn, profile_id) = workspace();
        let work = work::create(&conn, &profile_id, song("Winter road")).unwrap();
        let draft = version::create(
            &conn,
            &work.id,
            NewVersion {
                role: "lyrics".into(),
                body: "words".into(),
                label: Some("second attempt".into()),
                meta: None,
                make_current: false,
                parent_version_id: None,
            },
        )
        .unwrap();

        discard(&conn, Entity::Version, &draft.id).unwrap();
        let listed = list(&conn, &profile_id).unwrap();

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].label, "second attempt");
        assert_eq!(listed[0].origin.as_deref(), Some("Winter road"));
        assert_eq!(listed[0].entity, Entity::Version);
        assert!(listed[0].restorable);
    }

    #[test]
    fn an_entry_names_the_work_its_cover_is_drawn_from() {
        let (conn, profile_id) = workspace();
        let work = work::create(&conn, &profile_id, song("Winter road")).unwrap();
        let draft = version::create(
            &conn,
            &work.id,
            NewVersion {
                role: "lyrics".into(),
                body: "words".into(),
                label: None,
                meta: None,
                make_current: false,
                parent_version_id: None,
            },
        )
        .unwrap();

        // The version first and the work after it, so the version outlives
        // its work in the trash: the snapshot is then the only place that
        // still knows which work it was.
        discard(&conn, Entity::Version, &draft.id).unwrap();
        discard(&conn, Entity::Work, &work.id).unwrap();
        let listed = list(&conn, &profile_id).unwrap();

        let works: Vec<Option<&str>> = listed
            .iter()
            .map(|entry| entry.work_id.as_deref())
            .collect();
        assert_eq!(works, [Some(work.id.as_str()), Some(work.id.as_str())]);
    }

    #[test]
    fn a_trashed_score_is_named_by_what_it_said_not_when() {
        let (conn, profile_id) = workspace();
        let work = work::create(&conn, &profile_id, song("Winter road")).unwrap();
        let scored = crate::score::create(
            &conn,
            &work.id,
            crate::score::NewScore {
                axes: serde_json::from_value(serde_json::json!({ "hook": 8, "text": 8 }))
                    .expect("axes are an object"),
                version_id: None,
                note: None,
                rater: None,
            },
        )
        .unwrap();

        discard(&conn, Entity::Score, &scored.id).unwrap();
        let label = list(&conn, &profile_id).unwrap()[0].label.clone();

        // The old label was `scored_at`, which reads as
        // `2026-08-17T20:06:45.0175882Z` on the trash screen and in the journal.
        assert!(
            !label.contains('T') || !label.contains('Z'),
            "a score must not be named by its timestamp, got `{label}`"
        );
        assert!(
            label.contains(&format!("{}", (scored.total * 10.0).round() / 10.0)),
            "a score should be named by its total, got `{label}`"
        );
    }

    #[test]
    fn a_version_without_a_label_is_named_by_its_role_and_revision() {
        let (conn, profile_id) = workspace();
        let work = work::create(&conn, &profile_id, song("Nameless")).unwrap();
        let draft = version::create(
            &conn,
            &work.id,
            NewVersion {
                role: "lyrics".into(),
                body: "words".into(),
                label: None,
                meta: None,
                make_current: false,
                parent_version_id: None,
            },
        )
        .unwrap();

        discard(&conn, Entity::Version, &draft.id).unwrap();

        assert_eq!(list(&conn, &profile_id).unwrap()[0].label, "lyrics 1");
    }

    #[test]
    fn the_trash_is_scoped_to_its_profile() {
        let (conn, profile_id) = workspace();
        let mine = work::create(&conn, &profile_id, song("Mine")).unwrap();
        conn.execute(
            "INSERT INTO profile (id, key, name, config, is_active, is_builtin, created_at, updated_at)
             SELECT 'other', 'other', 'Other', config, 0, 0, created_at, updated_at FROM profile LIMIT 1",
            [],
        )
        .unwrap();
        let theirs = work::create(&conn, "other", song("Theirs")).unwrap();

        discard(&conn, Entity::Work, &mine.id).unwrap();
        discard(&conn, Entity::Work, &theirs.id).unwrap();

        let listed = list(&conn, &profile_id).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].label, "Mine");
    }

    #[test]
    fn purge_drops_one_entry_and_empty_drops_the_profiles() {
        let (conn, profile_id) = workspace();
        let first = work::create(&conn, &profile_id, song("One")).unwrap();
        let second = work::create(&conn, &profile_id, song("Two")).unwrap();
        let first_entry = discard(&conn, Entity::Work, &first.id).unwrap();
        discard(&conn, Entity::Work, &second.id).unwrap();

        purge(&conn, &first_entry).unwrap();
        assert_eq!(count(&conn, "deletion"), 1);
        assert!(
            restore(&conn, &first_entry).is_err(),
            "a purged entry is gone for good"
        );

        assert_eq!(empty(&conn, &profile_id).unwrap(), 1);
        assert_eq!(count(&conn, "deletion"), 0);
    }

    #[test]
    fn purging_a_work_takes_the_entries_that_needed_it() {
        let (conn, profile_id) = workspace();
        let work = work::create(&conn, &profile_id, song("Doomed")).unwrap();
        let draft = version::create(
            &conn,
            &work.id,
            NewVersion {
                role: "lyrics".into(),
                body: "words".into(),
                label: None,
                meta: None,
                make_current: false,
                parent_version_id: None,
            },
        )
        .unwrap();
        let survivor = work::create(&conn, &profile_id, song("Untouched")).unwrap();
        let survivor_note = note::create(
            &conn,
            &profile_id,
            NewNote {
                body: "belongs to the other work".into(),
                kind: None,
                title: None,
                work_id: Some(survivor.id.clone()),
                tags: vec![],
            },
        )
        .unwrap();

        discard(&conn, Entity::Version, &draft.id).unwrap();
        discard(&conn, Entity::Note, &survivor_note.id).unwrap();
        let work_entry = discard(&conn, Entity::Work, &work.id).unwrap();
        assert_eq!(count(&conn, "deletion"), 3);

        purge(&conn, &work_entry).unwrap();

        // The version could never have come back once its work went for good,
        // so it goes too — but the note belonging to a different work stays.
        let left = list(&conn, &profile_id).unwrap();
        assert_eq!(left.len(), 1, "only the unrelated entry survives: {left:?}");
        assert_eq!(left[0].entity, Entity::Note);
        assert!(left[0].restorable);
    }

    #[test]
    fn a_restored_collection_gets_its_works_back() {
        let (conn, profile_id) = workspace();
        let album = collection::create(
            &conn,
            &profile_id,
            collection::NewCollection {
                kind: "album".into(),
                title: "First album".into(),
                description: None,
                meta: None,
                target_size: None,
                due_on: None,
            },
        )
        .unwrap();
        let mut grouped = song("Track one");
        grouped.collection_id = Some(album.id.clone());
        let track = work::create(&conn, &profile_id, grouped).unwrap();

        let entry = discard(&conn, Entity::Collection, &album.id).unwrap();

        assert_eq!(count(&conn, "collection"), 0);
        assert!(
            work::get(&conn, &track.id)
                .unwrap()
                .unwrap()
                .collection_id
                .is_none(),
            "the work outlives the collection, unattached"
        );

        restore(&conn, &entry).unwrap();

        assert_eq!(
            work::get(&conn, &track.id).unwrap().unwrap().collection_id,
            Some(album.id),
            "and is put back in it"
        );
    }

    #[test]
    fn restoring_a_collection_does_not_overrule_a_later_move() {
        let (conn, profile_id) = workspace();
        let new_collection = |title: &str| collection::NewCollection {
            kind: "album".into(),
            title: title.into(),
            description: None,
            meta: None,
            target_size: None,
            due_on: None,
        };
        let first = collection::create(&conn, &profile_id, new_collection("First")).unwrap();
        let second = collection::create(&conn, &profile_id, new_collection("Second")).unwrap();
        let mut grouped = song("Track one");
        grouped.collection_id = Some(first.id.clone());
        let track = work::create(&conn, &profile_id, grouped).unwrap();

        let entry = discard(&conn, Entity::Collection, &first.id).unwrap();
        // Meanwhile the work found a new home.
        work::update(
            &conn,
            &track.id,
            work::WorkPatch {
                collection_id: Some(Some(second.id.clone())),
                ..Default::default()
            },
        )
        .unwrap();

        restore(&conn, &entry).unwrap();

        assert_eq!(
            work::get(&conn, &track.id).unwrap().unwrap().collection_id,
            Some(second.id),
            "the decision taken since stands"
        );
    }

    #[test]
    fn discarding_something_that_is_not_there_fails() {
        let (conn, _) = workspace();

        assert!(discard(&conn, Entity::Work, "nope").is_err());
        assert!(restore(&conn, "nope").is_err());
        assert!(purge(&conn, "nope").is_err());
    }

    #[test]
    fn a_note_without_a_work_survives_the_round_trip() {
        let (conn, profile_id) = workspace();
        let note = note::create(
            &conn,
            &profile_id,
            NewNote {
                body: "an idea with no home".into(),
                kind: None,
                title: None,
                work_id: None,
                tags: vec!["idea".into()],
            },
        )
        .unwrap();

        let entry = discard(&conn, Entity::Note, &note.id).unwrap();
        let listed = list(&conn, &profile_id).unwrap();
        assert_eq!(listed[0].label, "an idea with no home");
        assert!(listed[0].origin.is_none());
        assert!(
            listed[0].work_id.is_none(),
            "a note on nothing has no cover to borrow"
        );
        assert!(listed[0].restorable);

        restore(&conn, &entry).unwrap();
        let back = note::get(&conn, &note.id).unwrap().unwrap();
        assert_eq!(back.tags, vec!["idea".to_owned()], "the tags came back too");
    }

    /// A style brick with one reference picture, the picture copied into a
    /// media directory the test owns.
    fn a_style_with_a_reference(
        conn: &Connection,
        profile_id: &str,
        media: &std::path::Path,
    ) -> (crate::style_brick::StyleBrick, crate::asset::Asset) {
        let brick = crate::style_brick::create(
            conn,
            profile_id,
            crate::style_brick::NewStyleBrick {
                type_key: "image-style".into(),
                name: "Cold north".into(),
                description: Some("Grainy monochrome film.".into()),
                hint: None,
            },
        )
        .unwrap();
        let outside = tempfile::tempdir().unwrap();
        let source = outside.path().join("north-01.png");
        std::fs::write(&source, b"not really a picture").unwrap();
        let reference = crate::asset::attach(
            conn,
            profile_id,
            media,
            &source,
            crate::asset::NewAsset {
                style_brick_id: Some(brick.id.clone()),
                ..crate::asset::NewAsset::default()
            },
        )
        .unwrap();
        (brick, reference)
    }

    /// Deleting a style used to be a plain DELETE: no entry, no undo, and the
    /// pictures gone with it. It is a trash entity now, and comes back whole.
    #[test]
    fn a_style_goes_to_the_trash_with_its_references_and_comes_back_whole() {
        let (conn, profile_id) = workspace();
        let media = tempfile::tempdir().unwrap();
        let (brick, reference) = a_style_with_a_reference(&conn, &profile_id, media.path());

        let entry = discard(&conn, Entity::Style, &brick.id).unwrap();
        assert!(crate::style_brick::get(&conn, &brick.id).unwrap().is_none());
        assert!(crate::asset::get(&conn, &reference.id).unwrap().is_none());
        assert!(
            std::path::Path::new(&reference.path).exists(),
            "the picture stays on disk while its entry can still be restored"
        );

        let listed = list(&conn, &profile_id).unwrap();
        assert_eq!(listed[0].entity, Entity::Style);
        assert_eq!(listed[0].label, "Cold north");
        assert!(listed[0].restorable);

        restore(&conn, &entry).unwrap();
        let back = crate::style_brick::get(&conn, &brick.id).unwrap().unwrap();
        assert_eq!(back.description.as_deref(), Some("Grainy monochrome film."));
        assert_eq!(back.reference_count, 1, "the picture came back with it");

        // The tombstone the delete wrote is marked restored (migration 0028):
        // without it a restored brick still reads as deleted to anything that
        // trusts the tombstones.
        let restored: Option<String> = conn
            .query_row(
                "SELECT restored_at FROM tombstone WHERE entity = 'style_brick' AND entity_id = ?1",
                params![brick.id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(restored.is_some(), "the tombstone knows the brick is back");
    }

    #[test]
    fn purging_a_style_removes_the_pictures_nothing_else_names() {
        let (conn, profile_id) = workspace();
        let media = tempfile::tempdir().unwrap();
        let (brick, reference) = a_style_with_a_reference(&conn, &profile_id, media.path());

        let entry = discard(&conn, Entity::Style, &brick.id).unwrap();
        purge(&conn, &entry).unwrap();

        assert!(
            !std::path::Path::new(&reference.path).exists(),
            "a purged style's picture used to stay in media/ for good, named by nothing"
        );
    }

    #[test]
    fn emptying_the_trash_removes_the_pictures_it_held() {
        let (conn, profile_id) = workspace();
        let media = tempfile::tempdir().unwrap();
        let (brick, reference) = a_style_with_a_reference(&conn, &profile_id, media.path());

        discard(&conn, Entity::Style, &brick.id).unwrap();
        empty(&conn, &profile_id).unwrap();

        assert!(!std::path::Path::new(&reference.path).exists());
    }

    /// A cloned video hangs its own rows on the same files, so a path can be
    /// named by more than one row. Purging one of them must not take the file
    /// from under the other.
    #[test]
    fn a_purged_picture_that_a_live_row_still_names_stays() {
        let (conn, profile_id) = workspace();
        let media = tempfile::tempdir().unwrap();
        let (brick, reference) = a_style_with_a_reference(&conn, &profile_id, media.path());

        let work = work::create(&conn, &profile_id, song("Harbour lights")).unwrap();
        conn.execute(
            "INSERT INTO asset (id, profile_id, work_id, kind, path, created_at)
             VALUES ('shared', ?1, ?2, 'attachment', ?3, '2026-09-24T00:00:00Z')",
            params![profile_id, work.id, reference.path],
        )
        .unwrap();

        let entry = discard(&conn, Entity::Style, &brick.id).unwrap();
        purge(&conn, &entry).unwrap();

        assert!(
            std::path::Path::new(&reference.path).exists(),
            "the work still shows this picture"
        );
    }
}
