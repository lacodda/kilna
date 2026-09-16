pub mod config;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::db::migrations;
use crate::error::{Error, Result};
use crate::time::now;
use config::{Derive, ProfileConfig};

/// A profile as it ships with the application, before it reaches the database.
#[derive(Debug, Clone, Deserialize)]
pub struct BuiltinProfile {
    pub key: String,
    pub name: String,
    pub description: String,
    pub config: ProfileConfig,
}

/// A profile as the frontend sees it.
#[derive(Debug, Clone, Serialize)]
pub struct Profile {
    pub id: String,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub config: ProfileConfig,
    pub is_active: bool,
    pub is_builtin: bool,
}

/// Profiles compiled into the binary. Music ships first; the others arrive with
/// the profile editor.
pub fn builtin() -> Result<Vec<BuiltinProfile>> {
    // Four crafts against one schema. If a craft needed a schema change to fit,
    // the premise of ADR 0001 would be wrong — so far none has.
    const SOURCES: &[&str] = &[
        include_str!("../../profiles/music.json"),
        include_str!("../../profiles/novel.json"),
        include_str!("../../profiles/podcast.json"),
        include_str!("../../profiles/blog.json"),
    ];
    SOURCES
        .iter()
        .map(|source| serde_json::from_str(source).map_err(Error::from))
        .collect()
}

/// Install any built-in profile the workspace does not have yet, and make sure
/// exactly one profile is active.
///
/// Existing built-in rows are left alone: a user who edited the Music profile
/// keeps their edits across upgrades.
pub fn seed(conn: &Connection) -> Result<()> {
    let timestamp = now();

    for profile in builtin()? {
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM profile WHERE key = ?1",
                params![profile.key],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if exists {
            carry_forward(conn, &profile)?;
            continue;
        }

        conn.execute(
            "INSERT INTO profile (id, key, name, description, config, is_active, is_builtin, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 1, ?6, ?6)",
            params![
                uuid::Uuid::new_v4().to_string(),
                profile.key,
                profile.name,
                profile.description,
                serde_json::to_string(&profile.config)?,
                timestamp,
            ],
        )?;
    }

    // A profile the owner made themselves is carried forward by nobody, so
    // its document is brought to the current format here: read, which
    // migrates, and written back so the next read finds it done.
    rewrite_older_formats(conn)?;

    ensure_one_active(conn)?;
    Ok(())
}

/// Bring every stored profile document to [`config::FORMAT`].
///
/// Reading is the migration (see `RawProfileConfig`); this makes it stick.
/// A document already in the current format is left untouched, byte for
/// byte, so an edit the owner made by hand is not reserialised behind them.
fn rewrite_older_formats(conn: &Connection) -> Result<()> {
    let rows: Vec<(String, String)> = conn
        .prepare("SELECT id, config FROM profile")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (id, raw) in rows {
        if raw_format(&raw) == config::FORMAT {
            continue;
        }
        let config: ProfileConfig = serde_json::from_str(&raw)?;
        conn.execute(
            "UPDATE profile SET config = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, serde_json::to_string(&config)?, now()],
        )?;
    }
    Ok(())
}

/// Carry newly shipped parts of a built-in profile into the copy a workspace
/// already holds.
///
/// A profile is copied into the database on first run and then belongs to the
/// user, so shipping a new field changes nothing for anybody who already ran
/// the app — the older copy simply lacks it. What is added here is what has no
/// user-authored value to overwrite:
///
/// * **Prompt templates** whose key is missing. A user who reworded an action
///   keeps their wording; one they deliberately deleted comes back at the next
///   upgrade — accepted, because an action nobody ever sees is worse.
/// * **The `derive` role of a status**, where the stored copy still says
///   `Manual` and the shipped one names a meaning. Without this the status
///   automation is silently inert in every workspace that predates it, which is
///   exactly how it was found: on a real database, deriving nothing at all.
/// * **The `requires` list of a release kind**, where the stored copy states
///   nothing and the shipped one names roles. Same failure mode as `derive`:
///   readiness marks would sit blank in every workspace that predates them.
/// * **The rhythm**, where the stored copy has none and the shipped one names
///   a pace. Without it the auto-layout is a button that only ever refuses in
///   every workspace that predates the field.
/// * **Meta fields** whose key is missing. Same reasoning as prompts: a field
///   the user renamed or retyped keeps their version, because the match is by
///   key. New keys are appended rather than merged in place, so an order the
///   user arranged is not rewritten.
/// * **Marks** and **version roles**, on exactly those terms.
///
/// A role the user deliberately set to `Manual` is indistinguishable from one
/// that was never written, and is restored along with the rest. The same is
/// true of a deleted prompt or meta field: absence is absence either way. That
/// is the price of not asking; both are one click away in the profile editor.
fn carry_forward(conn: &Connection, shipped: &BuiltinProfile) -> Result<()> {
    let Some((id, raw)): Option<(String, String)> = conn
        .query_row(
            "SELECT id, config FROM profile WHERE key = ?1",
            params![shipped.key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
    else {
        return Ok(());
    };

    let mut config: ProfileConfig = serde_json::from_str(&raw)?;
    let mut changed = false;

    let missing: Vec<_> = shipped
        .config
        .prompts
        .iter()
        .filter(|shipped| {
            !config
                .prompts
                .iter()
                .any(|existing| existing.key == shipped.key)
        })
        .cloned()
        .collect();

    if !missing.is_empty() {
        config.prompts.extend(missing);
        changed = true;
    }

    // An action's method (ADR 0021) arrives on the terms a role's body did:
    // where the stored action names none and the shipped one does. What the
    // action produces follows only where the stored copy says nothing. And a
    // `score` template still reading exactly as it shipped before v0.61 —
    // naming the axes by hand — is moved to the shipped wording, because the
    // instruction now states the axes from the profile and a template that
    // names other ones contradicts it; a template the owner reworded stays.
    for prompt in &mut config.prompts {
        let Some(shipped) = shipped
            .config
            .prompts
            .iter()
            .find(|shipped| shipped.key == prompt.key)
        else {
            continue;
        };
        if prompt.method().is_none() && shipped.method().is_some() {
            prompt.method = shipped.method.clone();
            changed = true;
        }
        if prompt.produces.is_none() && shipped.produces.is_some() {
            prompt.produces = shipped.produces.clone();
            changed = true;
        }
        // The kinds and the scope (v0.64) arrive the same way: an action
        // stored before they existed is for every kind, which is what
        // `critique` on a video was — a button sending a prompt with a
        // hole in it — until the shipped copy said `song`.
        if prompt.kinds.is_empty() && !shipped.kinds.is_empty() {
            prompt.kinds = shipped.kinds.clone();
            changed = true;
        }
        if prompt.scope.is_none() && shipped.scope.is_some() {
            prompt.scope = shipped.scope.clone();
            changed = true;
        }
        if SCORE_TEMPLATES_BEFORE_METHODS.contains(&prompt.template.as_str())
            && prompt.template != shipped.template
        {
            prompt.template = shipped.template.clone();
            changed = true;
        }
    }

    // The vocabulary lives on each kind (format 2), so everything below is
    // carried kind by kind: the stored kind and the shipped kind meet by key,
    // and a kind the owner invented meets nothing and keeps what it has.
    for kind in &mut config.work_kinds {
        let Some(shipped_kind) = shipped
            .config
            .work_kinds
            .iter()
            .find(|shipped| shipped.key == kind.key)
        else {
            continue;
        };

        for status in &mut kind.statuses {
            if status.derive != Derive::Manual {
                continue;
            }
            let Some(shipped) = shipped_kind
                .statuses
                .iter()
                .find(|shipped| shipped.key == status.key)
            else {
                continue;
            };
            if shipped.derive != Derive::Manual {
                status.derive = shipped.derive;
                changed = true;
            }
        }

        for release_kind in &mut kind.release_kinds {
            let Some(shipped) = shipped_kind
                .release_kinds
                .iter()
                .find(|shipped| shipped.key == release_kind.key)
            else {
                continue;
            };
            if release_kind.requires.is_empty() && !shipped.requires.is_empty() {
                release_kind.requires = shipped.requires.clone();
                changed = true;
            }
            // The glyph arrives the same way the requirements did: a workspace
            // made before the field existed gains what the shipped profile
            // states for a kind it still recognises by key. A kind the owner
            // added themselves is not in the shipped list and keeps its blank
            // -- guessing a glyph for "Vinyl pressing" is not something this
            // code can do.
            if release_kind.icon.is_none() && shipped.icon.is_some() {
                release_kind.icon = shipped.icon.clone();
                changed = true;
            }
            // What a release of this kind says about itself arrives whole or
            // not at all: a workspace that already lists fields for this kind
            // has an owner who decided what a release says, and appending the
            // shipped ones to that would put two titles in one box. A
            // workspace with none gains the shipped list, which is how a live
            // workspace made before v0.71 comes to have release metadata at
            // all.
            if release_kind.fields.is_empty() && !shipped.fields.is_empty() {
                release_kind.fields = shipped.fields.clone();
                changed = true;
            }
        }

        // A role newly shipped for a kind the workspace already has is
        // appended after the owner's; a role they renamed or retyped stays
        // theirs, because the match is by key.
        changed |= add_new_keys(
            &mut kind.version_roles,
            &shipped_kind.version_roles,
            |role| &role.key,
        );

        // How a role's body reads arrives the way a kind's glyph did: a role
        // the workspace still shares by key gains what the shipped profile
        // states, when the stored copy states nothing. A choice the owner
        // made stays theirs.
        for role in &mut kind.version_roles {
            let Some(shipped) = shipped_kind
                .version_roles
                .iter()
                .find(|shipped| shipped.key == role.key)
            else {
                continue;
            };
            if role.body.is_none() && shipped.body.is_some() {
                role.body = shipped.body.clone();
                changed = true;
            }
        }

        // The storyboard's vocabulary arrives on the same terms as a role's
        // body: a kind the workspace has by key, whose stored copy names no
        // kinds of shot and no prompt blocks, gains what the shipped profile
        // states. The video kind reached the owner's workspace in v0.57
        // before scenes existed, and without this it would have no Scenes
        // tab. A list the owner narrowed or renamed is left alone.
        if kind.shot_types.is_empty() && !shipped_kind.shot_types.is_empty() {
            kind.shot_types = shipped_kind.shot_types.clone();
            changed = true;
        }
        if kind.scene_blocks.is_empty() && !shipped_kind.scene_blocks.is_empty() {
            kind.scene_blocks = shipped_kind.scene_blocks.clone();
            changed = true;
        }

        // A status badge's colour, on the same terms: by key, only where the
        // stored status names none.
        for status in &mut kind.statuses {
            let Some(shipped_status) = shipped_kind
                .statuses
                .iter()
                .find(|shipped| shipped.key == status.key)
            else {
                continue;
            };
            if status.colour.is_none() && shipped_status.colour.is_some() {
                status.colour = shipped_status.colour;
                changed = true;
            }
        }
    }

    // A kind the shipped profile gained since this workspace was made -- a
    // video beside the songs -- arrives with its statuses, roles and kinds of
    // release, and WITHOUT its axes and tiers. The judgement is the craft's
    // own: this workspace's axes are the owner's words, and a stranger's
    // axes appearing silently beside them would be the one thing the profile
    // must never do. The owner writes the kind's axes when they are ready to
    // judge it; until then works of that kind are scored empty. A fresh
    // workspace, seeded rather than carried forward, gets the whole kind.
    let arriving: Vec<config::WorkKind> = shipped
        .config
        .work_kinds
        .iter()
        .filter(|candidate| {
            !config
                .work_kinds
                .iter()
                .any(|kind| kind.key == candidate.key)
        })
        .map(config::WorkKind::without_judgement)
        .collect();
    if !arriving.is_empty() {
        config.work_kinds.extend(arriving);
        changed = true;
    }

    if config.rhythm.is_none() && shipped.config.rhythm.is_some() {
        config.rhythm = shipped.config.rhythm.clone();
        changed = true;
    }

    // Everything the user keys by name: a vocabulary entry they renamed or
    // retyped stays theirs, and anything newly shipped is appended after it.
    changed |= add_new_keys(
        &mut config.work_meta_fields,
        &shipped.config.work_meta_fields,
        |field| &field.key,
    );
    changed |= add_new_keys(&mut config.marks, &shipped.config.marks, |mark| &mark.key);
    // The stops of the stage dial, new in 0.72. A workspace that predates them
    // gains the craft's own words; without this the dial would fall back to the
    // line's generic stops in every workspace that already existed, which is
    // every real one. A stop the owner renamed or added stays theirs.
    changed |= add_new_keys(&mut config.stages, &shipped.config.stages, |stage| {
        &stage.key
    });

    // The kinds a note can take, new in 0.65: a workspace that predates them
    // gains the craft's words, and a kind the owner added or renamed stays
    // theirs — matched by key like every other vocabulary.
    changed |= add_new_keys(&mut config.note_kinds, &shipped.config.note_kinds, |kind| {
        &kind.key
    });

    // A mark's glyph arrives the way a role's body does: a mark the
    // workspace still shares by key gains the shipped icon when its stored
    // copy names none. The three built-in marks reached every workspace
    // before marks had glyphs; a glyph the owner chose stays theirs.
    for mark in &mut config.marks {
        let Some(shipped_mark) = shipped.config.marks.iter().find(|m| m.key == mark.key) else {
            continue;
        };
        if mark.icon.is_none() && shipped_mark.icon.is_some() {
            mark.icon = shipped_mark.icon.clone();
            changed = true;
        }
    }

    // A stored copy still in format 1 was read into format 2 by the parser;
    // writing it back is what makes the migration permanent rather than
    // something every read repeats.
    if raw_format(&raw) != config::FORMAT {
        changed = true;
    }

    // The shipped profile's name follows it only where the stored copy still
    // carries the name it shipped with before: "Music" became "Studio" in
    // v0.57, because the craft makes videos too. A name the owner chose
    // themselves is theirs, so the match is against the old shipped name,
    // never against "anything different".
    for (before, after) in RENAMED {
        if shipped.name == after {
            let renamed = conn.execute(
                "UPDATE profile SET name = ?2, updated_at = ?3 WHERE id = ?1 AND name = ?4",
                params![id, after, now(), before],
            )?;
            if renamed > 0 {
                changed = true;
            }
        }
    }
    // The one-sentence description follows on the same terms: only where the
    // stored copy still says exactly what the old shipped file said.
    for (before, after) in REDESCRIBED {
        if shipped.description == after {
            conn.execute(
                "UPDATE profile SET description = ?2, updated_at = ?3 WHERE id = ?1 AND description = ?4",
                params![id, after, now(), before],
            )?;
        }
    }

    if !changed {
        return Ok(());
    }

    conn.execute(
        "UPDATE profile SET config = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, serde_json::to_string(&config)?, now()],
    )?;

    Ok(())
}

/// Shipped profile names that changed, old to new. A stored copy still
/// carrying the old one is renamed at the next start; see `carry_forward`.
const RENAMED: [(&str, &str); 1] = [("Music", "Studio")];

/// The `score` templates as the four profiles shipped them before v0.61,
/// when the axes were named in the template's own words. A stored copy
/// still reading exactly like one of these follows the shipped wording;
/// see `carry_forward`.
const SCORE_TEMPLATES_BEFORE_METHODS: [&str; 5] = [
    // Studio's v0.61 wording read `{role:lyrics}`, which a video does not
    // have; from v0.64 the action is for every kind and reads `{body}`.
    "Here are the lyrics of a song called \"{title}\".

{role:lyrics}

Judge it as a finished song, along the axes below. Be honest rather than kind: a score that flatters is worth nothing.",
    "Here are the lyrics of a song called \"{title}\".\n\n{role:lyrics}\n\nJudge it along these axes: Hook (Does the chorus stay with you after one listen), Lyrics (Imagery, rhyme and whether a line earns its place), Emotion (Does it move a listener who knows nothing about it), Production (Arrangement, mix and how finished it sounds), Originality (Distance from the obvious version of this idea), Visual potential (Is there a clip in it, or only a cover).\n\nBe honest rather than kind: a score that flatters is worth nothing.",
    "Here is a chapter called \"{title}\".\n\n{body}\n\nJudge it along these axes: Pull (Does the reader turn the page, or put the book down here), Prose (Sentence by sentence: rhythm, precision, nothing limp), Character (Do people behave like people rather than like plot requirements), Structure (Does the chapter earn its place and end where it should), Tension (Is something at stake on every page).\n\nBe honest rather than kind: a score that flatters is worth nothing.",
    "Here are the notes for an episode called \"{title}\".\n\n{body}\n\nJudge it along these axes: Hook (Does the cold open earn the next thirty seconds), Clarity (Could a listener explain the point to someone else afterward), Pacing (Where does it drag, and would a listener skip ahead), Insight (Is there a claim here nobody else is making, or just a summary), Delivery (Energy, pauses, whether it sounds read or spoken), Shareability (Is there a moment worth clipping and sending to a friend).\n\nBe honest rather than kind: a score that flatters is worth nothing.",
    "Here is a post called \"{title}\".\n\n{body}\n\nJudge it along these axes: Hook (Does the first paragraph survive contact with a stranger's attention span), Usefulness (Could a reader act on this, or is it just an opinion floating by), Clarity (Sentence by sentence: does every paragraph earn the next one), Angle (Is there a take here, or a restatement of what everyone already thinks), Shareability (Is there a line worth quoting out of context).\n\nBe honest rather than kind: a score that flatters is worth nothing.",
];

/// Shipped descriptions that changed, old to new, on the same terms as the
/// names: a copy the owner never touched follows, a rewritten one stays.
const REDESCRIBED: [(&str, &str); 1] = [(
    "Songs with independent lyrics and style drafts, judged on hook and craft, shipped as clips, shorts and audio releases.",
    "Songs and the videos made for them: lyrics and style drafts judged on hook and craft, clips and shorts judged on the cut, shipped as clips, shorts and audio releases.",
)];

/// The `format` a stored document claims — 1 when it says nothing, the way
/// every document written before the field did.
fn raw_format(raw: &str) -> u32 {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| value.get("format")?.as_u64())
        .map_or(1, |format| format as u32)
}

/// Append entries whose key the stored list does not have yet.
///
/// Returns whether anything was added, so the caller can tell a config that
/// needs writing from one that does not.
fn add_new_keys<T: Clone>(stored: &mut Vec<T>, shipped: &[T], key: impl Fn(&T) -> &String) -> bool {
    let new: Vec<T> = shipped
        .iter()
        .filter(|candidate| {
            stored
                .iter()
                .all(|existing| key(existing) != key(candidate))
        })
        .cloned()
        .collect();

    if new.is_empty() {
        return false;
    }
    stored.extend(new);
    true
}

/// Make sure a profile is active — a workspace is never without one.
///
/// The default is the first profile in `builtin()`, not the first by key or by
/// timestamp: the seeds all land in the same second, so ordering by either
/// would hand a new user whichever craft happened to sort first.
fn ensure_one_active(conn: &Connection) -> Result<()> {
    let active: i64 = conn.query_row(
        "SELECT count(*) FROM profile WHERE is_active = 1",
        [],
        |row| row.get(0),
    )?;

    if active > 0 {
        return Ok(());
    }

    let preferred = builtin()?.first().map(|profile| profile.key.clone());

    if let Some(key) = preferred {
        let changed = conn.execute(
            "UPDATE profile SET is_active = 1 WHERE key = ?1",
            params![key],
        )?;
        if changed > 0 {
            return Ok(());
        }
    }

    // A workspace with only user-made profiles falls back to the oldest.
    conn.execute(
        "UPDATE profile SET is_active = 1
         WHERE id = (SELECT id FROM profile ORDER BY created_at, key LIMIT 1)",
        [],
    )?;
    Ok(())
}

const SELECT_PROFILE: &str =
    "SELECT id, key, name, description, config, is_active, is_builtin FROM profile";

/// The profile the workspace is currently working in.
pub fn active(conn: &Connection) -> Result<Option<Profile>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_PROFILE} WHERE is_active = 1"),
            [],
            read_row,
        )
        .optional()?;

    raw.map(RawProfile::into_profile).transpose()
}

/// The profile a stable key names, whatever id this workspace gave it.
///
/// A profile's id is minted by [`seed`] on the machine that first opened the
/// workspace, so the same builtin profile has a different id in every copy. The
/// key does not move, which is why the operations log names profiles by key and
/// resolves them through here when a replay rebuilds elsewhere. See ADR 0014.
pub fn id_for_key(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM profile WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()?)
}

/// The stable key of the profile an id names.
pub fn key_for_id(conn: &Connection, profile_id: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT key FROM profile WHERE id = ?1",
            params![profile_id],
            |row| row.get(0),
        )
        .optional()?)
}

/// One profile's configuration, without the rest of the row.
///
/// Used by everything that has a profile id in hand and needs the vocabulary
/// behind it — scoring, status derivation — so the query lives here rather
/// than being written out again in each of them.
pub fn config_for(conn: &Connection, profile_id: &str) -> Result<config::ProfileConfig> {
    let raw: String = conn.query_row(
        "SELECT config FROM profile WHERE id = ?1",
        params![profile_id],
        |row| row.get(0),
    )?;
    Ok(serde_json::from_str(&raw)?)
}

/// Every profile in the workspace.
pub fn list(conn: &Connection) -> Result<Vec<Profile>> {
    let mut statement = conn.prepare(&format!("{SELECT_PROFILE} ORDER BY created_at, key"))?;
    let raw = statement
        .query_map([], read_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    raw.into_iter().map(RawProfile::into_profile).collect()
}

/// Make `id` the active profile, deactivating the previous one.
pub fn activate(conn: &mut Connection, id: &str) -> Result<()> {
    let tx = conn.transaction()?;
    // The partial unique index forbids two active rows, so clear first.
    tx.execute("UPDATE profile SET is_active = 0 WHERE is_active = 1", [])?;
    let changed = tx.execute(
        "UPDATE profile SET is_active = 1, updated_at = ?2 WHERE id = ?1",
        params![id, now()],
    )?;

    if changed == 0 {
        return Err(Error::not_found("profile", id));
    }

    tx.commit()?;
    Ok(())
}

/// Replace a profile's configuration.
///
/// Nothing else is touched: axis keys are strings, so past score snapshots stay
/// readable, and works keep whatever status and kind they already had even if
/// the vocabulary that named them was edited away.
pub fn update_config(conn: &Connection, id: &str, config: &ProfileConfig) -> Result<Profile> {
    // Refused whole, with every problem named: a document edited by hand is
    // fixed by reading the list, not by guessing which line the app minded.
    let problems = config.validate();
    if !problems.is_empty() {
        return Err(Error::Other(format!(
            "the profile cannot be saved as written:\n- {}",
            problems.join("\n- ")
        )));
    }

    let changed = conn.execute(
        "UPDATE profile SET config = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, serde_json::to_string(config)?, now()],
    )?;

    if changed == 0 {
        return Err(Error::not_found("profile", id));
    }

    let raw = conn
        .query_row(
            &format!("{SELECT_PROFILE} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?
        .ok_or_else(|| Error::not_found("profile", id))?;

    raw.into_profile()
}

/// A profile row before its config is parsed. Reading and parsing are separate
/// steps because rusqlite's row closure cannot fail with our error type.
struct RawProfile {
    id: String,
    key: String,
    name: String,
    description: Option<String>,
    config: String,
    is_active: bool,
    is_builtin: bool,
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawProfile> {
    Ok(RawProfile {
        id: row.get(0)?,
        key: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        config: row.get(4)?,
        is_active: row.get::<_, i64>(5)? == 1,
        is_builtin: row.get::<_, i64>(6)? == 1,
    })
}

impl RawProfile {
    fn into_profile(self) -> Result<Profile> {
        Ok(Profile {
            config: serde_json::from_str(&self.config)?,
            id: self.id,
            key: self.key,
            name: self.name,
            description: self.description,
            is_active: self.is_active,
            is_builtin: self.is_builtin,
        })
    }
}

/// A snapshot of the workspace for the status screen.
#[derive(Debug, Serialize)]
pub struct Workspace {
    pub schema_version: i64,
    pub profile: Option<Profile>,
    pub works: i64,
    pub releases: i64,
}

pub fn workspace(conn: &Connection) -> Result<Workspace> {
    Ok(Workspace {
        schema_version: migrations::current_version(conn)?,
        profile: active(conn)?,
        works: conn.query_row("SELECT count(*) FROM work", [], |row| row.get(0))?,
        releases: conn.query_row("SELECT count(*) FROM release", [], |row| row.get(0))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile::config::Derive;

    #[test]
    fn every_builtin_profile_parses() {
        let profiles = builtin().unwrap();
        assert!(!profiles.is_empty());
        for profile in profiles {
            assert!(
                !profile.config.work_kinds[0].axes.is_empty(),
                "{} has no axes",
                profile.key
            );
            assert!(
                !profile.config.work_kinds[0].tiers.is_empty(),
                "{} has no tiers",
                profile.key
            );
        }
    }

    #[test]
    fn builtin_keys_are_unique() {
        let profiles = builtin().unwrap();
        let mut keys: Vec<_> = profiles.iter().map(|p| p.key.as_str()).collect();
        keys.sort_unstable();
        let count = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), count, "two built-in profiles share a key");
    }

    #[test]
    fn seed_installs_the_builtins_and_activates_one() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let profiles = list(&conn).unwrap();
        assert_eq!(profiles.len(), builtin().unwrap().len());
        let active = active(&conn).unwrap().expect("a profile must be active");
        assert!(active.is_builtin);
    }

    #[test]
    fn a_new_workspace_starts_on_the_first_declared_profile() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        // Not the first by key — the seeds share a timestamp, so ordering by
        // key or date would pick a craft at random.
        assert_eq!(
            active(&conn).unwrap().unwrap().key,
            builtin().unwrap()[0].key
        );
    }

    #[test]
    fn seed_does_not_duplicate_on_a_second_run() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        seed(&conn).unwrap();

        assert_eq!(list(&conn).unwrap().len(), builtin().unwrap().len());
    }

    /// A workspace created before a field existed keeps the profile it copied
    /// on first run, and shipping the field changes nothing there on its own.
    /// This is not hypothetical: the status automation was found inert on a
    /// real database for exactly this reason, deriving `Manual` for every
    /// status while the shipped profile named meanings for four of them.
    #[test]
    fn an_older_workspace_gains_the_meanings_its_statuses_were_missing() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        // Wind the stored copy back to what a pre-`derive` workspace holds: the
        // same words, with no meaning attached to any of them.
        let (id, raw): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
        for status in &mut config.work_kinds[0].statuses {
            status.derive = Derive::Manual;
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let config = config_for(&conn, &id).unwrap();
        let derive_of = |key: &str| {
            config.work_kinds[0]
                .statuses
                .iter()
                .find(|status| status.key == key)
                .unwrap()
                .derive
        };
        assert_eq!(derive_of("draft"), Derive::Draft);
        assert_eq!(derive_of("scored"), Derive::Scored);
        assert_eq!(derive_of("scheduled"), Derive::Scheduled);
        assert_eq!(derive_of("released"), Derive::Released);
        // Nothing is invented for a status the shipped profile leaves manual.
        assert_eq!(derive_of("shelved"), Derive::Manual);
    }

    /// Same delivery path as `derive`: a workspace copied before release kinds
    /// stated their requirements gains them on the next start, or readiness
    /// marks stay blank there forever.
    #[test]
    fn an_older_workspace_gains_the_roles_its_release_kinds_require() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        // Wind the stored copy back: the same kinds, requiring nothing — and
        // one the user narrowed by hand, which must stay narrowed.
        let (id, raw): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
        for kind in &mut config.work_kinds[0].release_kinds {
            kind.requires = if kind.key == "audio" {
                vec!["style".into()]
            } else {
                Vec::new()
            };
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let config = config_for(&conn, &id).unwrap();
        let requires_of = |key: &str| {
            config.work_kinds[0]
                .release_kinds
                .iter()
                .find(|kind| kind.key == key)
                .unwrap()
                .requires
                .clone()
        };
        assert_eq!(requires_of("clip"), vec!["lyrics", "style"]);
        // A list the user set themselves is not overwritten by the upgrade.
        assert_eq!(requires_of("audio"), vec!["style"]);
    }

    /// Same delivery path again, for the glyph the calendar draws a kind with.
    /// A workspace made before the field existed shows every kind under the
    /// same fallback mark until this backfill runs.
    #[test]
    fn an_older_workspace_gains_the_icons_its_release_kinds_are_drawn_with() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        // Wind the stored copy back to before the field: no icons at all,
        // except one the user chose by hand, which must survive the upgrade.
        let (id, raw): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
        for kind in &mut config.work_kinds[0].release_kinds {
            kind.icon = if kind.key == "audio" {
                Some("radio".into())
            } else {
                None
            };
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let config = config_for(&conn, &id).unwrap();
        let icon_of = |key: &str| {
            config.work_kinds[0]
                .release_kinds
                .iter()
                .find(|kind| kind.key == key)
                .unwrap()
                .icon
                .clone()
        };
        assert_eq!(icon_of("clip").as_deref(), Some("film"));
        assert_eq!(icon_of("short").as_deref(), Some("smartphone"));
        // A glyph the user picked themselves is not taken back by the upgrade.
        assert_eq!(icon_of("audio").as_deref(), Some("radio"));
    }

    #[test]
    fn an_older_workspace_gains_the_way_its_roles_read() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        // Before the field: no role says how it reads, except one the owner
        // set by hand, which the upgrade must leave alone.
        let (id, raw): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
        for role in &mut config.work_kinds[0].version_roles {
            role.body = if role.key == "critique" {
                Some("plain".into())
            } else {
                None
            };
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let config = config_for(&conn, &id).unwrap();
        let body_of = |key: &str| {
            config.work_kinds[0]
                .version_roles
                .iter()
                .find(|role| role.key == key)
                .unwrap()
                .body
                .clone()
        };
        assert_eq!(body_of("lyrics").as_deref(), Some("plain"));
        assert_eq!(body_of("review").as_deref(), Some("markdown"));
        // The owner's own choice is not taken back by the upgrade.
        assert_eq!(body_of("critique").as_deref(), Some("plain"));
    }

    /// A kind the owner invented is theirs alone: the shipped profile has
    /// nothing to say about it, and the backfill must not reach for a glyph
    /// belonging to some other kind that happens to sit at the same index.
    #[test]
    fn a_kind_of_the_owners_own_gains_no_icon() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let (id, raw): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
        config.work_kinds[0].release_kinds.insert(
            0,
            crate::profile::config::ReleaseKind::new("vinyl", "Vinyl pressing", &[]),
        );
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let config = config_for(&conn, &id).unwrap();
        let vinyl = config.work_kinds[0]
            .release_kinds
            .iter()
            .find(|kind| kind.key == "vinyl")
            .unwrap();
        assert_eq!(vinyl.icon, None);
        // And the kinds that do ship still got theirs, so the assertion above
        // is not passing because the backfill did nothing at all.
        assert!(
            config.work_kinds[0]
                .release_kinds
                .iter()
                .any(|kind| kind.key == "clip" && kind.icon.as_deref() == Some("film"))
        );
    }

    /// The labels are the user's, and an upgrade must not take them back.
    #[test]
    fn carrying_meanings_forward_leaves_renamed_statuses_renamed() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let (id, raw): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
        for status in &mut config.work_kinds[0].statuses {
            status.derive = Derive::Manual;
            if status.key == "released" {
                status.label = "Out in the world".into();
            }
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let config = config_for(&conn, &id).unwrap();
        let released = config.work_kinds[0]
            .statuses
            .iter()
            .find(|status| status.key == "released")
            .unwrap();
        assert_eq!(released.label, "Out in the world");
        assert_eq!(released.derive, Derive::Released);
    }

    /// Every profile must be self-consistent, because a broken one is only
    /// discovered when a user switches to it.
    #[test]
    fn every_builtin_profile_is_internally_consistent() {
        for profile in builtin().unwrap() {
            let config = &profile.config;
            let key = &profile.key;

            assert!(!config.work_kinds.is_empty(), "{key} has no work kinds");
            // Every kind carries its own vocabulary (format 2), and each has
            // to be sound on its own: a video is judged and shipped without
            // ever consulting the song's lists.
            for vocab in &config.work_kinds {
                assert!(
                    !vocab.statuses.is_empty(),
                    "{key}/{}: no statuses",
                    vocab.key
                );

                // A profile whose statuses carry no meaning derives nothing, and
                // the automation would sit there doing nothing with no error to
                // show for it. Every derivable meaning needs a word, and no two
                // words may claim the same one — the automation would pick between
                // them by list order, which is not a decision anybody made.
                for meaning in [
                    Derive::Draft,
                    Derive::Scored,
                    Derive::Scheduled,
                    Derive::Released,
                ] {
                    let named: Vec<&str> = vocab
                        .statuses
                        .iter()
                        .filter(|status| status.derive == meaning)
                        .map(|status| status.key.as_str())
                        .collect();
                    assert_eq!(
                        named.len(),
                        1,
                        "{key} names {} status(es) for {meaning:?}: {named:?}",
                        named.len()
                    );
                }
                assert!(
                    !vocab.version_roles.is_empty(),
                    "{key}/{}: no version roles",
                    vocab.key
                );

                // A requirement naming a role the profile does not have can never
                // be satisfied, so every release of that kind reads as unready
                // forever — with nothing on any screen to explain why.
                for kind in &vocab.release_kinds {
                    for role in &kind.requires {
                        assert!(
                            vocab.version_roles.iter().any(|known| &known.key == role),
                            "{key}: release kind `{}` requires role `{role}`, which the profile does not define",
                            kind.key
                        );
                    }
                }

                // A tier reachable by nothing is a tier that never appears.
                assert!(
                    vocab.tiers.iter().any(|tier| tier.min == 0.0),
                    "{key} has no tier a zero score falls into"
                );
                assert!(
                    vocab.tiers.iter().all(|tier| tier.min <= 100.0),
                    "{key} has a tier no score can reach"
                );

                // Full marks on every axis must land in the top tier, or the
                // scoring scale and the tiers disagree with each other.
                let full: serde_json::Map<String, serde_json::Value> = vocab
                    .axes
                    .iter()
                    .map(|axis| (axis.key.clone(), serde_json::json!(axis.scale)))
                    .collect();
                let total = vocab.total(&full);
                assert!(
                    (total - 100.0).abs() < 1e-9,
                    "{key}: full marks scored {total}, not 100"
                );
            }

            // A rhythm every built-in states, and states sanely: zero days
            // between releases is not a pace, and the usual time is HH:MM or
            // absent. User-made profiles are free to omit the rhythm — the
            // layout button then explains itself — but a built-in shipping
            // without one would make the feature invisible by default.
            let rhythm = config
                .rhythm
                .as_ref()
                .unwrap_or_else(|| panic!("{key} ships without a rhythm"));
            assert!(rhythm.every_days >= 1, "{key} has a rhythm of zero days");
            if let Some(time) = &rhythm.default_time {
                let split = time.split_once(':');
                let sane = split.is_some_and(|(hours, minutes)| {
                    hours.parse::<u8>().is_ok_and(|h| h < 24)
                        && minutes.parse::<u8>().is_ok_and(|m| m < 60)
                        && hours.len() == 2
                        && minutes.len() == 2
                });
                assert!(sane, "{key} has a default time of `{time}`, not HH:MM");
            }

            // Prompts may only ask for roles every kind they are for has;
            // `validate` refuses otherwise, and a shipped file must pass it.
            for prompt in &config.prompts {
                let kinds: Vec<_> = config
                    .work_kinds
                    .iter()
                    .filter(|kind| prompt.applies_to(&kind.key))
                    .collect();
                assert!(
                    !kinds.is_empty(),
                    "{key}: prompt `{}` is for no kind",
                    prompt.key
                );
                for fragment in prompt.template.split("{role:").skip(1) {
                    let role = fragment.split('}').next().unwrap_or_default();
                    for kind in &kinds {
                        assert!(
                            kind.version_roles.iter().any(|r| r.key == role),
                            "{key}: prompt `{}` asks `{}` for unknown role `{role}`",
                            prompt.key,
                            kind.key
                        );
                    }
                }
            }
        }
    }

    /// Same delivery path as `derive` and `requires`: a workspace copied
    /// before profiles knew their pace gains the shipped one on the next
    /// start — and one the user set themselves is left alone.
    #[test]
    fn an_older_workspace_gains_the_rhythm_its_profile_was_missing() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let (id, raw): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
        config.rhythm = None;
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let shipped = builtin().unwrap()[0].config.rhythm.clone().unwrap();
        let gained = config_for(&conn, &id).unwrap().rhythm.unwrap();
        assert_eq!(gained.every_days, shipped.every_days);

        // A pace the user chose is not overwritten by the upgrade.
        let mut config = config_for(&conn, &id).unwrap();
        config.rhythm = Some(config::Rhythm {
            every_days: 30,
            default_time: None,
        });
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        assert_eq!(
            config_for(&conn, &id).unwrap().rhythm.unwrap().every_days,
            30
        );
    }

    #[test]
    fn seed_carries_newly_shipped_version_roles_into_an_existing_profile() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let (id, _): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        // A workspace from before the review roles shipped, with the user's own
        // rename on the one role it did have.
        let mut config = config_for(&conn, &id).unwrap();
        config.work_kinds[0]
            .version_roles
            .retain(|role| role.key == "lyrics");
        config.work_kinds[0].version_roles[0].label = "Words".into();
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let roles = config_for(&conn, &id).unwrap().work_kinds[0]
            .version_roles
            .clone();
        let keys: Vec<&str> = roles.iter().map(|role| role.key.as_str()).collect();
        assert!(
            keys.contains(&"review"),
            "a newly shipped role arrives: {keys:?}"
        );
        assert_eq!(keys.first(), Some(&"lyrics"), "the kept role stays first");
        assert_eq!(
            roles[0].label, "Words",
            "and keeps the user's own name for it"
        );

        // The pairing arrives with the role: without it the review would be one
        // more list to page through rather than something read beside the text.
        assert_eq!(
            roles
                .iter()
                .find(|role| role.key == "review")
                .and_then(|role| role.comments_on.as_deref()),
            Some("lyrics"),
        );

        let mut sorted = keys.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), keys.len(), "no role appears twice: {keys:?}");
    }

    #[test]
    fn seed_carries_newly_shipped_meta_fields_into_an_existing_profile() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        // A workspace created before the descriptive fields shipped: it holds
        // the reference numbers and nothing else.
        let (id, _): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config = config_for(&conn, &id).unwrap();
        config.work_meta_fields.retain(|field| field.key == "bpm");
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let gained = config_for(&conn, &id).unwrap().work_meta_fields;
        let keys: Vec<&str> = gained.iter().map(|field| field.key.as_str()).collect();
        assert!(
            keys.contains(&"premise"),
            "a newly shipped field arrives: {keys:?}"
        );
        assert_eq!(
            keys.first(),
            Some(&"bpm"),
            "the kept field stays where it was"
        );
        assert_eq!(
            gained
                .iter()
                .find(|field| field.key == "premise")
                .map(|field| field.field_type),
            Some(config::MetaFieldType::Multiline),
            "and arrives with its own type, not as plain text"
        );
    }

    // Caught on the owner's own workspace during the live run of v0.72: every
    // real workspace predates the stage dial, so without this the craft's stops
    // never arrive and the dial silently falls back to the line's generic ones.
    #[test]
    fn seed_carries_newly_shipped_stages_into_an_existing_profile() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        // A workspace created before stages existed: the list is empty, which
        // is exactly what `#[serde(default)]` leaves behind.
        let id: String = conn
            .query_row("SELECT id FROM profile WHERE key = 'music'", [], |row| {
                row.get(0)
            })
            .unwrap();
        let mut config = config_for(&conn, &id).unwrap();
        assert!(
            !config.stages.is_empty(),
            "the shipped profile has stages to lose"
        );
        config.stages.clear();
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let gained = config_for(&conn, &id).unwrap().stages;
        assert!(
            !gained.is_empty(),
            "the craft's own stops arrive rather than the line's fallback"
        );
        assert_eq!(
            gained.last().map(|stage| stage.percent),
            Some(100),
            "and the scale still ends where a finished work stands"
        );
    }

    #[test]
    fn carrying_stages_forward_leaves_the_users_own_stops_alone() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let id: String = conn
            .query_row("SELECT id FROM profile WHERE key = 'music'", [], |row| {
                row.get(0)
            })
            .unwrap();
        let mut config = config_for(&conn, &id).unwrap();
        for stage in &mut config.stages {
            stage.label = format!("{} (mine)", stage.label);
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let kept = config_for(&conn, &id).unwrap().stages;
        assert!(
            kept.iter().all(|stage| stage.label.ends_with("(mine)")),
            "renamed stops stay renamed: {:?}",
            kept.iter().map(|s| &s.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn carrying_meta_fields_forward_leaves_the_users_own_wording_alone() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let (id, _): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        // The user renames a shipped field and retypes another.
        let mut config = config_for(&conn, &id).unwrap();
        for field in &mut config.work_meta_fields {
            if field.key == "premise" {
                field.label = "What it is about".into();
                field.field_type = config::MetaFieldType::Text;
            }
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let fields = config_for(&conn, &id).unwrap().work_meta_fields;

        // Every key at most once. `find` would happily return the user's own
        // field while a shipped duplicate sat behind it, which is exactly what
        // a carry-forward without the key filter produces.
        let premises: Vec<_> = fields
            .iter()
            .filter(|field| field.key == "premise")
            .collect();
        assert_eq!(
            premises.len(),
            1,
            "the field is carried once, not duplicated"
        );
        assert_eq!(premises[0].label, "What it is about");
        assert_eq!(premises[0].field_type, config::MetaFieldType::Text);

        let mut keys: Vec<&str> = fields.iter().map(|field| field.key.as_str()).collect();
        let total = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), total, "no key appears twice: {keys:?}");
    }

    #[test]
    fn seed_carries_newly_shipped_prompts_into_an_existing_profile() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        // A workspace created before the prompts shipped.
        let mut config = active(&conn).unwrap().unwrap().config;
        config.prompts.clear();
        conn.execute(
            "UPDATE profile SET config = ?1 WHERE key = 'music'",
            params![serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let shipped = builtin().unwrap()[0].config.prompts.len();
        assert_eq!(
            active(&conn).unwrap().unwrap().config.prompts.len(),
            shipped
        );
    }

    #[test]
    fn seed_does_not_overwrite_a_reworded_prompt() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        let mut config = active(&conn).unwrap().unwrap().config;
        let key = config.prompts[0].key.clone();
        config.prompts[0].label = "My own wording".into();
        conn.execute(
            "UPDATE profile SET config = ?1 WHERE key = 'music'",
            params![serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let reloaded = active(&conn).unwrap().unwrap().config;
        let kept = reloaded.prompts.iter().find(|p| p.key == key).unwrap();
        assert_eq!(kept.label, "My own wording");
        assert_eq!(
            reloaded.prompts.len(),
            builtin().unwrap()[0].config.prompts.len(),
            "no duplicate was added alongside it"
        );
    }

    #[test]
    fn seed_keeps_user_edits_to_a_builtin_profile() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        conn.execute(
            "UPDATE profile SET name = 'My music' WHERE key = 'music'",
            [],
        )
        .unwrap();

        seed(&conn).unwrap();

        let profile = list(&conn)
            .unwrap()
            .into_iter()
            .find(|p| p.key == "music")
            .unwrap();
        assert_eq!(profile.name, "My music");
    }

    #[test]
    fn activate_moves_the_active_flag() {
        let mut conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        conn.execute(
            "INSERT INTO profile (id, key, name, config, is_active, is_builtin, created_at, updated_at)
             VALUES ('p-mine', 'mine', 'Mine', ?1, 0, 0, '2026-02-01T00:00:00Z', '2026-02-01T00:00:00Z')",
            params![serde_json::to_string(&builtin().unwrap()[0].config).unwrap()],
        )
        .unwrap();

        activate(&mut conn, "p-mine").unwrap();

        assert_eq!(active(&conn).unwrap().unwrap().id, "p-mine");
        let active_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM profile WHERE is_active = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(active_count, 1);
    }

    #[test]
    fn activating_an_unknown_profile_fails_without_deactivating_the_current_one() {
        let mut conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        assert!(activate(&mut conn, "nope").is_err());
        assert!(active(&conn).unwrap().is_some());
    }

    #[test]
    fn workspace_reports_the_schema_and_counts() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let workspace = workspace(&conn).unwrap();

        assert_eq!(workspace.schema_version, migrations::latest_version());
        assert_eq!(workspace.works, 0);
        assert!(workspace.profile.is_some());
    }

    #[test]
    fn every_builtin_profile_is_sound() {
        for profile in builtin().unwrap() {
            let problems = profile.config.validate();
            assert!(problems.is_empty(), "{}: {problems:?}", profile.key);
        }
    }

    #[test]
    fn a_profile_that_would_misbehave_is_refused_with_every_problem_named() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        let profile = active(&conn).unwrap().unwrap();
        let mut broken = profile.config.clone();
        broken.work_kinds[0].axes[0].scale = 0.0;
        let first_tier = broken.work_kinds[0].tiers[0].key.clone();
        broken.work_kinds[0].tiers.push(config::Tier {
            key: first_tier,
            label: "Twice".into(),
            min: 0.0,
        });

        let refused = update_config(&conn, &profile.id, &broken).unwrap_err();
        let message = refused.to_string();
        assert!(message.contains("cannot be saved"), "{message}");
        assert!(message.contains("has the scale 0"), "{message}");
        assert!(message.contains("repeats the key"), "{message}");

        let stored = config_for(&conn, &profile.id).unwrap();
        assert!(
            stored.work_kinds[0].axes[0].scale > 0.0,
            "the stored copy must be untouched"
        );
    }

    /// A workspace written in format 1 comes up in format 2 — and the
    /// document in the row is rewritten, so the migration happens once rather
    /// than on every read.
    #[test]
    fn an_older_workspace_is_rewritten_into_format_2_once() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        // Wind the owner's own profile back to format 1: flat vocabulary, no
        // `format`, a kind of their own beside the shipped ones.
        let flat = serde_json::json!({
            "work_kinds": [{ "key": "song", "label": "Song" }, { "key": "poem", "label": "Poem" }],
            "release_kinds": [{ "key": "clip", "label": "Clip" }],
            "collection_kinds": [],
            "version_roles": [{ "key": "lyrics", "label": "Lyrics" }],
            "statuses": [{ "key": "draft", "label": "Draft", "derive": "draft" }],
            "axes": [{ "key": "imagery", "label": "Imagery", "weight": 1.0, "scale": 10.0 }],
            "tiers": [{ "key": "hold", "label": "Hold", "min": 0.0 }],
            "work_meta_fields": []
        });
        conn.execute(
            "INSERT INTO profile (id, key, name, description, config, is_active, is_builtin, created_at, updated_at)
             VALUES ('mine', 'mine', 'Mine', NULL, ?1, 0, 0, 'x', 'x')",
            params![flat.to_string()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let raw: String = conn
            .query_row("SELECT config FROM profile WHERE id = 'mine'", [], |row| {
                row.get(0)
            })
            .unwrap();
        let written: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            written["format"],
            config::FORMAT,
            "the row was not rewritten"
        );
        assert!(written.get("axes").is_none());
        let config = config_for(&conn, "mine").unwrap();
        assert_eq!(config.kind("poem").unwrap().axes[0].key, "imagery");
        assert_eq!(config.kind("song").unwrap().tiers.len(), 1);

        // A second start leaves the row alone: same bytes, no churn.
        let before: String = conn
            .query_row(
                "SELECT config || updated_at FROM profile WHERE id = 'mine'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        seed(&conn).unwrap();
        let after: String = conn
            .query_row(
                "SELECT config || updated_at FROM profile WHERE id = 'mine'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(before, after);
    }

    /// The owner's music workspace, made before videos existed, gains the
    /// video and short kinds — with their statuses, roles and kinds of
    /// release, and without their axes and tiers. The judgement is theirs to
    /// write; the owner's own axes stay exactly as they were.
    #[test]
    fn an_older_music_workspace_gains_the_video_kinds_without_their_judgement() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let (id, raw): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
        // Before videos: only the song kinds, and the owner's own axes.
        config
            .work_kinds
            .retain(|kind| kind.key == "song" || kind.key == "instrumental");
        for kind in &mut config.work_kinds {
            kind.axes = vec![config::Axis {
                key: "imagery".into(),
                label: "Imagery".into(),
                weight: 1.0,
                scale: 10.0,
                description: None,
                kind: config::AxisKind::Scale,
                options: Vec::new(),
                rubric: Vec::new(),
            }];
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let config = config_for(&conn, &id).unwrap();
        let video = config.kind("video").expect("the video kind arrived");
        assert!(
            video.axes.is_empty(),
            "a stranger's axes must not appear beside the owner's"
        );
        assert!(video.tiers.is_empty());
        assert!(!video.statuses.is_empty(), "but a video can hold a work");
        assert!(video.version_roles.iter().any(|role| role.key == "plot"));
        assert!(!video.release_kinds.is_empty());
        assert!(config.kind("short").is_some());
        // The owner's judgement of a song is untouched.
        let song = config.kind("song").unwrap();
        assert_eq!(song.axes.len(), 1);
        assert_eq!(song.axes[0].key, "imagery");
    }

    /// Marks had no glyph and statuses no colour before v0.63: a stored copy
    /// naming none gains what the shipped profile states, by key, and a
    /// glyph or a colour the owner chose stays theirs.
    #[test]
    fn a_workspace_from_before_glyphs_gains_mark_icons_and_status_colours() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let (id, raw): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
        for mark in &mut config.marks {
            mark.icon = if mark.key == "unsure" {
                Some("flame".into())
            } else {
                None
            };
        }
        for kind in &mut config.work_kinds {
            for status in &mut kind.statuses {
                status.colour = if status.key == "released" {
                    Some(config::MarkColour::Bad)
                } else {
                    None
                };
            }
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let config = config_for(&conn, &id).unwrap();
        let icon = |key: &str| {
            config
                .marks
                .iter()
                .find(|m| m.key == key)
                .and_then(|m| m.icon.clone())
        };
        assert_eq!(icon("working").as_deref(), Some("wrench"), "arrived");
        assert_eq!(
            icon("unsure").as_deref(),
            Some("flame"),
            "the owner's stays"
        );
        let song = config.kind("song").unwrap();
        let colour = |key: &str| {
            song.statuses
                .iter()
                .find(|s| s.key == key)
                .and_then(|s| s.colour)
        };
        assert_eq!(colour("scored"), Some(config::MarkColour::Accent));
        assert_eq!(
            colour("released"),
            Some(config::MarkColour::Bad),
            "the owner's stays"
        );
        assert_eq!(
            colour("draft"),
            None,
            "a status shipped without a colour gains none"
        );
    }

    /// The owner's workspace gained the video kinds in v0.57, before scenes
    /// existed: its stored video kind names no kinds of shot, no blocks and
    /// no `context` role. At the next start it gains all three, on the same
    /// terms a role's body arrived — where the stored copy states nothing.
    /// The kinds a note can take arrive in a workspace made before them, and
    /// a kind the owner wrote themselves is left alone: the same rule every
    /// vocabulary follows — matched by key, theirs stays theirs.
    #[test]
    fn a_workspace_from_before_note_kinds_gains_them_and_keeps_its_own() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let (id, raw): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
        // As a workspace written before 0.65 looks: no kinds of note at all,
        // plus one the owner added for themselves.
        config.note_kinds = vec![config::Kind::new("prop", "Prop")];
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let gained = config_for(&conn, &id).unwrap().note_kinds;
        let keys: Vec<&str> = gained.iter().map(|kind| kind.key.as_str()).collect();
        assert!(
            keys.contains(&"character"),
            "the craft's words arrive: {keys:?}"
        );
        assert!(keys.contains(&"location"));
        assert!(
            keys.contains(&"prop"),
            "and the owner's own stays: {keys:?}"
        );
        assert_eq!(keys[0], "prop", "theirs first, the shipped ones after");
    }

    /// A list the owner narrowed is left alone.
    #[test]
    fn a_workspace_with_video_kinds_from_before_scenes_gains_the_storyboard_words() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();

        let (id, raw): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
        for kind in &mut config.work_kinds {
            if kind.key == "video" {
                kind.shot_types.clear();
                kind.scene_blocks.clear();
                kind.version_roles.retain(|role| role.key != "context");
            }
            if kind.key == "short" {
                // The owner kept one kind of shot on purpose.
                kind.shot_types.retain(|shot| shot.key == "close");
            }
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let config = config_for(&conn, &id).unwrap();
        let video = config.kind("video").unwrap();
        assert!(!video.shot_types.is_empty(), "the kinds of shot arrived");
        assert!(!video.scene_blocks.is_empty(), "the blocks arrived");
        assert!(
            video.version_roles.iter().any(|role| role.key == "context"),
            "the context role arrived"
        );
        let short = config.kind("short").unwrap();
        assert_eq!(
            short.shot_types.len(),
            1,
            "a list the owner narrowed is theirs"
        );
    }

    /// A workspace whose actions predate methods: the shipped method
    /// arrives where the stored action has none, the shipped `produces`
    /// where the stored one says nothing, and a `score` template still
    /// reading exactly as it shipped follows the new wording. A template
    /// the owner reworded stays, and so does a method they wrote.
    #[test]
    fn methods_arrive_and_an_unchanged_score_template_follows() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        let (id, raw): (String, String) = conn
            .query_row(
                "SELECT id, config FROM profile WHERE key = 'music'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let mut config: ProfileConfig = serde_json::from_str(&raw).unwrap();
        for prompt in &mut config.prompts {
            prompt.method = None;
            if prompt.key == "score" {
                prompt.template = SCORE_TEMPLATES_BEFORE_METHODS[0].to_owned();
            }
            if prompt.key == "critique" {
                prompt.produces = None;
            }
            if prompt.key == "polish" {
                prompt.template = "My own polish".into();
                prompt.method = Some("My own way".into());
            }
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            params![id, serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let config = config_for(&conn, &id).unwrap();
        let by_key = |key: &str| config.prompts.iter().find(|p| p.key == key).unwrap();
        assert!(by_key("score").method().is_some(), "the method arrived");
        assert!(
            !by_key("score").template.contains("Hook ("),
            "the template no longer names the axes by hand"
        );
        assert_eq!(
            by_key("critique").produces.as_deref(),
            Some("version:critique")
        );
        assert_eq!(by_key("polish").template, "My own polish", "reworded stays");
        assert_eq!(by_key("polish").method.as_deref(), Some("My own way"));
    }

    /// A workspace made when the profile was still called Music wakes up
    /// with it called Studio; one whose owner named it themselves keeps that.
    #[test]
    fn the_old_shipped_name_follows_the_new_one_and_an_owners_name_stays() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        conn.execute("UPDATE profile SET name = 'Music' WHERE key = 'music'", [])
            .unwrap();
        conn.execute(
            "UPDATE profile SET name = 'Chapters' WHERE key = 'novel'",
            [],
        )
        .unwrap();

        seed(&conn).unwrap();

        let name = |key: &str| -> String {
            conn.query_row("SELECT name FROM profile WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .unwrap()
        };
        assert_eq!(name("music"), "Studio");
        assert_eq!(
            name("novel"),
            "Chapters",
            "a name the owner chose is not taken back"
        );
        let description: String = conn
            .query_row(
                "SELECT description FROM profile WHERE key = 'music'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            description.starts_with("Songs and the videos"),
            "{description}"
        );
    }

    /// A fresh workspace gets the whole shipped kind, judgement included: there
    /// is nobody's vocabulary to keep out of the way.
    #[test]
    fn a_fresh_workspace_gets_the_video_kinds_whole() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        let profile = active(&conn).unwrap().unwrap();
        let video = profile.config.kind("video").unwrap();
        assert!(!video.axes.is_empty());
        assert!(!video.tiers.is_empty());
        assert_eq!(profile.name, "Studio");
    }

    #[test]
    fn seed_carries_kinds_and_scope_onto_actions_stored_without_them() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn).unwrap();
        let mut config = active(&conn).unwrap().unwrap().config;
        for prompt in &mut config.prompts {
            prompt.kinds.clear();
            prompt.scope = None;
        }
        conn.execute(
            "UPDATE profile SET config = ?1 WHERE key = 'music'",
            params![serde_json::to_string(&config).unwrap()],
        )
        .unwrap();

        seed(&conn).unwrap();

        let reloaded = active(&conn).unwrap().unwrap().config;
        let by_key = |key: &str| reloaded.prompts.iter().find(|p| p.key == key).unwrap();
        assert_eq!(by_key("critique").kinds, vec!["song"]);
        assert_eq!(by_key("prompts").scope.as_deref(), Some("scene"));
        assert!(
            by_key("score").kinds.is_empty(),
            "an action for every kind stays so"
        );
    }
}
