//! Packs of styles from a registry (v0.131, FR-06): a static index of packs and
//! a manifest per pack, read from a folder or a URL - the snapshot shipped
//! beside the application, a folder of the person's, or a site such as GitHub
//! Pages - and installed into the open space's style dictionary whole or one
//! brick at a time.
//!
//! A pack's brick is a shipped entry like any other (ADR 0087): its id is
//! decided by the profile, the pack and the brick, so installing twice makes
//! no second brick; one nobody touched takes the pack's newer wording; one its
//! owner changed keeps their words; one they deleted stays deleted unless they
//! install that very brick again. Every brick is written by the gestures a
//! person's own are - `style.create`, `style.update` - so the history says it
//! and an undo takes it back.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::error::{Error, Result};
use crate::profile::config::Label;
use crate::style_brick::{self, Content, NewStyleBrick, StyleBrickPatch};

/// The registry's own folder in the snapshot shipped with the application.
pub const BUILTIN_DIR: &str = "style-registry";

/// The format of an index this build reads.
pub const FORMAT: u32 = 1;

/// The root of a registry.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct PackIndex {
    pub format_version: u32,
    pub packs: Vec<PackEntry>,
}

/// One pack as the index lists it.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct PackEntry {
    pub id: String,
    pub version: String,
    /// The SHA-256 of the pack's manifest, as hex: what it is checked against
    /// when it is read.
    pub digest: String,
    pub title: Label,
    pub description: Label,
    /// Which half of the dictionary it fills: `picture`, `motion`, `sound`.
    pub half: String,
    pub category: String,
    pub brick_count: u32,
    pub updated_at: String,
    /// Where its manifest is, relative to the index: `packs/<id>.json`.
    pub manifest_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub preview_webp_url: Option<String>,
}

/// A pack's manifest: its bricks.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct PackManifest {
    pub id: String,
    pub version: String,
    pub bricks: Vec<PackBrick>,
}

/// One brick of a pack, in the schema of the starter set (`SetBrick`), with
/// its preview and what it needs, gives and cannot sit beside (FR-07).
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct PackBrick {
    pub key: String,
    /// A key of the profile's `style_types`.
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub type_key: String,
    pub label: Label,
    /// The family it is filed under: the pack's own, registered on install.
    pub family: String,
    /// The words that go into a prompt, in English, naming no artist.
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub explanation: Option<Label>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<String>>")]
    pub colours: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub sample: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub preview_webp: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<String>>")]
    pub needs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<String>>")]
    pub provides: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<String>>")]
    pub conflicts: Vec<String>,
}

impl PackBrick {
    /// What the brick says, as the dictionary fingerprints a brick's words.
    fn content(&self) -> Content<'_> {
        Content {
            type_key: &self.type_key,
            name: self.label.as_str(),
            label: Some(&self.label),
            family: Some(&self.family),
            description: Some(&self.description),
            when_to_use: self.when.as_deref(),
            colours: &self.colours,
            sample: self.sample.as_deref(),
            explanation: self.explanation.as_ref(),
            needs: &self.needs,
            provides: &self.provides,
            conflicts: &self.conflicts,
        }
    }

    /// The contract the brick carries into the dictionary.
    pub fn contract(&self) -> crate::style_brick::Contract {
        crate::style_brick::Contract {
            needs: self.needs.clone(),
            provides: self.provides.clone(),
            conflicts: self.conflicts.clone(),
        }
    }

    pub fn digest(&self) -> String {
        self.content().digest()
    }
}

/// Where a registry is read from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "kind", content = "location", rename_all = "snake_case")]
#[ts(rename = "PackSource")]
pub enum Source {
    /// The snapshot shipped beside the application.
    Builtin,
    /// A folder on this machine holding an `index.json`.
    Folder(String),
    /// A site serving an `index.json` - its URL, the index's own or its folder's.
    Url(String),
}

impl Source {
    /// A stable name for the source, as the window and the history say it.
    pub fn id(&self) -> String {
        match self {
            Source::Builtin => "builtin".into(),
            Source::Folder(path) => format!("folder:{path}"),
            Source::Url(url) => format!("url:{url}"),
        }
    }

    fn is_remote(&self) -> bool {
        matches!(self, Source::Url(_))
    }
}

/// Read a file of a registry by its path relative to the index.
fn read(source: &Source, builtin: &Path, relative: &str) -> Result<Vec<u8>> {
    let unreadable = |cause: String| {
        Error::refused("packs.unreadable")
            .param("source", source.id())
            .param("cause", cause)
    };
    match source {
        Source::Builtin => {
            std::fs::read(builtin.join(relative)).map_err(|e| unreadable(e.to_string()))
        }
        Source::Folder(folder) => {
            std::fs::read(Path::new(folder).join(relative)).map_err(|e| unreadable(e.to_string()))
        }
        Source::Url(url) => {
            let base = url
                .strip_suffix("index.json")
                .unwrap_or(url)
                .trim_end_matches('/');
            let target = format!("{base}/{relative}");
            let response = ureq::get(&target)
                .config()
                .timeout_global(Some(std::time::Duration::from_secs(20)))
                .build()
                .call()
                .map_err(|e| unreadable(e.to_string()))?;
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut response.into_body().into_reader(), &mut bytes)
                .map_err(|e| unreadable(e.to_string()))?;
            Ok(bytes)
        }
    }
}

/// The index of a registry.
pub fn index(source: &Source, builtin: &Path) -> Result<PackIndex> {
    let bytes = read(source, builtin, "index.json")?;
    let index: PackIndex = serde_json::from_slice(&bytes).map_err(|e| {
        Error::refused("packs.notAnIndex")
            .param("source", source.id())
            .param("cause", e.to_string())
    })?;
    if index.format_version > FORMAT {
        return Err(Error::refused("packs.newerFormat")
            .param("source", source.id())
            .param("format", index.format_version));
    }
    Ok(index)
}

/// A pack's manifest, checked against the digest its index gives.
pub fn manifest(source: &Source, builtin: &Path, entry: &PackEntry) -> Result<PackManifest> {
    let bytes = read(source, builtin, &entry.manifest_url)?;
    let found = hex(&sha2::Sha256::digest(&bytes));
    if !entry.digest.eq_ignore_ascii_case(&found) {
        return Err(Error::refused("packs.digestMismatch").param("pack", entry.id.clone()));
    }
    serde_json::from_slice(&bytes).map_err(|e| {
        Error::refused("packs.notAManifest")
            .param("pack", entry.id.clone())
            .param("cause", e.to_string())
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The id a pack's brick has in a space: the same profile, pack and brick are
/// the same brick, so an install never doubles and a deletion is found again.
pub fn brick_id(profile_key: &str, pack: &str, brick: &str) -> String {
    const NAMESPACE: uuid::Uuid = uuid::uuid!("2c6e4f1a-8b3d-4e7a-9c15-7d0b3e5a6f21");
    uuid::Uuid::new_v5(
        &NAMESPACE,
        format!("style-pack:{profile_key}:{pack}:{brick}").as_bytes(),
    )
    .to_string()
}

/// Where a pack's brick stands in a space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum BrickStanding {
    /// Not in the space.
    Absent,
    /// In the space, reading as the pack has it.
    Installed,
    /// In the space untouched, and the pack has newer words.
    Update,
    /// In the space with its owner's words.
    Changed,
    /// Installed and deleted since: stays out unless installed again by hand.
    Deleted,
    /// Of a type this space's profile does not have.
    Unfit,
}

/// Where a whole pack stands in a space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum PackStanding {
    Absent,
    Installed,
    Update,
    Unfit,
}

fn deleted(conn: &Connection, id: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM tombstone WHERE entity = 'style_brick' AND entity_id = ?1 AND restored_at IS NULL",
            params![id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false))
}

/// Where one brick stands in the open space.
pub fn standing(conn: &Connection, pack: &str, brick: &PackBrick) -> Result<BrickStanding> {
    let profile =
        crate::profile::active(conn)?.ok_or_else(|| Error::refused("profile.noneActive"))?;
    if profile.config.style_type(&brick.type_key).is_none() {
        return Ok(BrickStanding::Unfit);
    }
    let id = brick_id(&profile.key, pack, &brick.key);
    let Some(stored) = style_brick::get(conn, &id)? else {
        return Ok(if deleted(conn, &id)? {
            BrickStanding::Deleted
        } else {
            BrickStanding::Absent
        });
    };
    let shipped = brick.digest();
    let given = stored
        .set_digest
        .clone()
        .map(|digest| crate::origin::Given { digest, kept: None });
    Ok(
        match crate::origin::decide(Some(&stored.content().digest()), given.as_ref(), &shipped) {
            crate::origin::Verdict::Current | crate::origin::Verdict::Adopted => {
                BrickStanding::Installed
            }
            crate::origin::Verdict::Renewed => BrickStanding::Update,
            _ => BrickStanding::Changed,
        },
    )
}

/// Where a pack stands, by its bricks: absent when none is in, an update when
/// one untouched has newer words, installed otherwise.
pub fn pack_standing(standings: &[BrickStanding]) -> PackStanding {
    if !standings.is_empty() && standings.iter().all(|s| *s == BrickStanding::Unfit) {
        return PackStanding::Unfit;
    }
    if standings.contains(&BrickStanding::Update) {
        return PackStanding::Update;
    }
    let present = standings
        .iter()
        .any(|s| matches!(s, BrickStanding::Installed | BrickStanding::Changed));
    if present {
        PackStanding::Installed
    } else {
        PackStanding::Absent
    }
}

/// What an install did.
#[derive(Debug, Clone, Default, Serialize, ts_rs::TS)]
pub struct Installed {
    pub added: u32,
    pub renewed: u32,
    /// Left as they are: the owner's words, a deleted one, one already as the
    /// pack has it.
    pub kept: u32,
    /// Of a type this space does not have.
    pub unfit: u32,
    /// Previews kept as the first picture of a brick just added (v0.132).
    #[serde(default)]
    #[ts(optional, as = "Option<u32>")]
    pub previews: u32,
}

/// Install bricks of a pack into the open space: all of them, or only the
/// keys named. A pack installed whole leaves out what its owner deleted; a
/// brick named by hand comes back even so - the person asked for it.
pub fn install(
    conn: &Connection,
    manifest: &PackManifest,
    entry: &PackEntry,
    only: Option<&[String]>,
    preview_of: &dyn Fn(&str) -> Option<Vec<u8>>,
) -> Result<Installed> {
    let profile =
        crate::profile::active(conn)?.ok_or_else(|| Error::refused("profile.noneActive"))?;
    let chosen: Vec<&PackBrick> = manifest
        .bricks
        .iter()
        .filter(|brick| only.is_none_or(|keys| keys.contains(&brick.key)))
        .collect();
    if chosen.is_empty() {
        return Err(Error::refused("packs.nothingChosen").param("pack", entry.id.clone()));
    }
    let mut done = Installed::default();

    // The families the bricks are filed under, registered on their types
    // first: the dictionary files a brick only under a family its type has.
    register_families(conn, &profile, &chosen, &entry.title)?;
    let source = format!("pack:{}", entry.id);

    for brick in chosen {
        match standing(conn, &entry.id, brick)? {
            BrickStanding::Unfit => done.unfit += 1,
            BrickStanding::Installed | BrickStanding::Changed => done.kept += 1,
            BrickStanding::Deleted if only.is_none() => done.kept += 1,
            BrickStanding::Absent | BrickStanding::Deleted => {
                let id = brick_id(&profile.key, &entry.id, &brick.key);
                crate::actions::style::create_given(
                    conn,
                    &id,
                    NewStyleBrick {
                        type_key: brick.type_key.clone(),
                        name: brick.label.as_str().to_owned(),
                        label: Some(brick.label.clone()),
                        description: Some(brick.description.clone()),
                        family: Some(brick.family.clone()),
                        when_to_use: brick.when.clone(),
                        colours: Some(brick.colours.clone()),
                        sample: brick.sample.clone(),
                        explanation: brick.explanation.clone(),
                        set_key: Some(brick.key.clone()),
                        set_digest: Some(brick.digest()),
                        source: Some(source.clone()),
                        contract: Some(brick.contract()),
                        ..NewStyleBrick::default()
                    },
                )?;
                done.added += 1;
                // The pack's preview is the brick's first picture (FR-05
                // 5.2): kept in the space's own folder like any reference,
                // so it works offline and goes to the trash with the brick.
                // A preview that cannot be read leaves the brick without one.
                if let Some(relative) = &brick.preview_webp
                    && let Ok(media) = crate::state::media_dir_of(conn)
                    && let Some(bytes) = preview_of(relative)
                {
                    let name = format!("{}.webp", brick.key);
                    crate::actions::style::paste_reference(conn, &media, &id, &bytes, &name)?;
                    done.previews += 1;
                }
            }
            BrickStanding::Update => {
                let id = brick_id(&profile.key, &entry.id, &brick.key);
                crate::actions::style::update(
                    conn,
                    &id,
                    StyleBrickPatch {
                        name: Some(brick.label.as_str().to_owned()),
                        label: Some(Some(brick.label.clone())),
                        description: Some(Some(brick.description.clone())),
                        family: Some(Some(brick.family.clone())),
                        when_to_use: Some(brick.when.clone()),
                        colours: Some(brick.colours.clone()),
                        sample: Some(brick.sample.clone()),
                        explanation: Some(brick.explanation.clone()),
                        contract: Some(brick.contract()),
                        set_digest: Some(Some(brick.digest())),
                        ..StyleBrickPatch::default()
                    },
                )?;
                done.renewed += 1;
            }
        }
    }
    Ok(done)
}

/// Give each type the families its bricks are filed under, named after the
/// pack: one change to the profile, logged as any other.
fn register_families(
    conn: &Connection,
    profile: &crate::profile::Profile,
    bricks: &[&PackBrick],
    title: &Label,
) -> Result<()> {
    let mut config = profile.config.clone();
    let mut changed = false;
    for brick in bricks {
        let Some(style) = config
            .style_types
            .iter_mut()
            .find(|t| t.key == brick.type_key)
        else {
            continue;
        };
        if style.families.iter().any(|f| f.key == brick.family) {
            continue;
        }
        style.families.push(crate::profile::config::StyleFamily {
            off: false,
            key: brick.family.clone(),
            label: title.clone(),
        });
        changed = true;
    }
    if changed {
        crate::actions::profile::update_config(conn, &profile.id, &config)?;
    }
    Ok(())
}

/// The registries the application reads, in order: the snapshot first, then
/// the person's folders and sites. Offline, a site is not asked.
pub fn sources(app: &Connection) -> Result<(Vec<Source>, bool)> {
    let offline: bool = app
        .query_row(
            "SELECT offline FROM style_registry WHERE id = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .is_some_and(|value| value == 1);
    let mut statement =
        app.prepare("SELECT kind, location FROM style_source ORDER BY created_at, rowid")?;
    let mut sources = vec![Source::Builtin];
    for row in statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })? {
        let (kind, location) = row?;
        sources.push(match kind.as_str() {
            "folder" => Source::Folder(location),
            _ => Source::Url(location),
        });
    }
    Ok((sources, offline))
}

/// Add a folder or a site to read packs from.
pub fn add_source(app: &Connection, source: &Source) -> Result<()> {
    let (kind, location) = match source {
        Source::Builtin => return Ok(()),
        Source::Folder(path) => ("folder", path.trim()),
        Source::Url(url) => ("url", url.trim()),
    };
    if location.is_empty() {
        return Err(Error::refused("packs.emptySource"));
    }
    app.execute(
        "INSERT OR IGNORE INTO style_source (kind, location, created_at) VALUES (?1, ?2, ?3)",
        params![kind, location, crate::time::now()],
    )?;
    Ok(())
}

pub fn remove_source(app: &Connection, source: &Source) -> Result<()> {
    if let Source::Folder(location) | Source::Url(location) = source {
        app.execute(
            "DELETE FROM style_source WHERE location = ?1",
            params![location],
        )?;
    }
    Ok(())
}

pub fn set_offline(app: &Connection, offline: bool) -> Result<()> {
    app.execute(
        "INSERT INTO style_registry (id, offline) VALUES (1, ?1)
         ON CONFLICT (id) DO UPDATE SET offline = ?1",
        params![i64::from(offline)],
    )?;
    Ok(())
}

/// A pack as the catalogue shows it: where it comes from and where it stands.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct ListedPack {
    pub source: Source,
    pub entry: PackEntry,
    pub standing: PackStanding,
}

/// A pack opened: its bricks and where each stands.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct OpenedPack {
    pub source: Source,
    pub entry: PackEntry,
    pub bricks: Vec<(PackBrick, BrickStanding)>,
}

/// Every pack the sources offer, with where it stands in the open space; a
/// source that cannot be read is said, and the rest are still listed.
pub fn catalogue(
    conn: &Connection,
    sources: &[Source],
    offline: bool,
    builtin: &Path,
) -> (Vec<ListedPack>, Vec<String>) {
    let mut packs = Vec::new();
    let mut failed = Vec::new();
    for source in sources {
        if offline && source.is_remote() {
            continue;
        }
        let index = match index(source, builtin) {
            Ok(index) => index,
            Err(err) => {
                failed.push(err.to_string());
                continue;
            }
        };
        for entry in index.packs {
            let standing = manifest(source, builtin, &entry)
                .and_then(|manifest| {
                    let standings = manifest
                        .bricks
                        .iter()
                        .map(|brick| standing(conn, &entry.id, brick))
                        .collect::<Result<Vec<_>>>()?;
                    Ok(pack_standing(&standings))
                })
                .unwrap_or(PackStanding::Absent);
            packs.push(ListedPack {
                source: source.clone(),
                entry,
                standing,
            });
        }
    }
    (packs, failed)
}

/// One pack from a source, opened.
pub fn open(conn: &Connection, source: &Source, builtin: &Path, pack: &str) -> Result<OpenedPack> {
    let entry = index(source, builtin)?
        .packs
        .into_iter()
        .find(|entry| entry.id == pack)
        .ok_or_else(|| Error::not_found("pack", pack))?;
    let manifest = manifest(source, builtin, &entry)?;
    let bricks = manifest
        .bricks
        .into_iter()
        .map(|brick| {
            let standing = standing(conn, &entry.id, &brick)?;
            Ok((brick, standing))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(OpenedPack {
        source: source.clone(),
        entry,
        bricks,
    })
}

/// Install from a source by the pack's id.
pub fn install_from(
    conn: &Connection,
    source: &Source,
    builtin: &Path,
    pack: &str,
    only: Option<&[String]>,
) -> Result<Installed> {
    let entry = index(source, builtin)?
        .packs
        .into_iter()
        .find(|entry| entry.id == pack)
        .ok_or_else(|| Error::not_found("pack", pack))?;
    let manifest = manifest(source, builtin, &entry)?;
    install(conn, &manifest, &entry, only, &|relative| {
        read(source, builtin, relative).ok()
    })
}

/// The snapshot's folder in the source tree, for tests and the development run.
pub fn snapshot_in_tree() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join(BUILTIN_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, profile};

    fn studio() -> Connection {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn, profile::DEFAULT_KIND).unwrap();
        conn
    }

    fn snapshot() -> PathBuf {
        snapshot_in_tree()
    }

    fn entry(id: &str) -> PackEntry {
        index(&Source::Builtin, &snapshot())
            .unwrap()
            .packs
            .into_iter()
            .find(|e| e.id == id)
            .unwrap()
    }

    #[test]
    fn the_shipped_snapshot_reads_whole_and_every_manifest_is_its_own() {
        let index = index(&Source::Builtin, &snapshot()).unwrap();
        assert!(index.packs.len() >= 4);
        let halves: Vec<&str> = index.packs.iter().map(|p| p.half.as_str()).collect();
        for half in ["picture", "motion", "sound"] {
            assert!(halves.contains(&half), "{half}");
        }
        for entry in &index.packs {
            let manifest = manifest(&Source::Builtin, &snapshot(), entry).unwrap();
            assert_eq!(
                manifest.bricks.len() as u32,
                entry.brick_count,
                "{}",
                entry.id
            );
            for brick in &manifest.bricks {
                assert!(
                    matches!(brick.label, Label::PerLocale(_)),
                    "{}: {}",
                    entry.id,
                    brick.key
                );
                assert_eq!(
                    brick.family, entry.id,
                    "a pack files its bricks under its own family"
                );
            }
        }
    }

    /// A pack "in the spirit of" an artist names the artist in its title
    /// only (FR-06, the catalogue 2.2): the words a generator reads are
    /// production tags, never a person's name.
    #[test]
    fn no_description_in_the_snapshot_names_an_artist() {
        let index = index(&Source::Builtin, &snapshot()).unwrap();
        let mut names: Vec<String> = Vec::new();
        for entry in &index.packs {
            if entry.category != "artist-spirit" {
                continue;
            }
            let Label::PerLocale(words) = &entry.title else {
                panic!("{} has a title in one language", entry.id);
            };
            for title in words.values() {
                let named = title
                    .trim_start_matches("In the spirit of ")
                    .trim_start_matches("В духе ");
                for name in named.split('/') {
                    let name = name.trim().trim_start_matches("The ").to_lowercase();
                    if name.len() > 2 {
                        names.push(name);
                    }
                }
            }
        }
        assert!(names.len() >= 16, "{names:?}");
        for entry in &index.packs {
            for brick in manifest(&Source::Builtin, &snapshot(), entry)
                .unwrap()
                .bricks
            {
                let said = brick.description.to_lowercase();
                for name in &names {
                    assert!(
                        !said.contains(name.as_str()),
                        "{}: {} names {name}: {}",
                        entry.id,
                        brick.key,
                        brick.description
                    );
                }
            }
        }
    }

    /// Every pack of the snapshot installs whole into Studio - no type it
    /// lacks, no name the starter set or another pack already has - and
    /// speaks only the resolver's tags.
    #[test]
    fn every_pack_of_the_snapshot_installs_into_studio() {
        let conn = studio();
        for entry in index(&Source::Builtin, &snapshot()).unwrap().packs {
            let done = install_from(&conn, &Source::Builtin, &snapshot(), &entry.id, None)
                .unwrap_or_else(|e| panic!("{}: {e:?}", entry.id));
            assert_eq!(
                (done.added, done.unfit),
                (entry.brick_count, 0),
                "{}",
                entry.id
            );
            for brick in manifest(&Source::Builtin, &snapshot(), &entry)
                .unwrap()
                .bricks
            {
                for tag in brick
                    .needs
                    .iter()
                    .chain(&brick.provides)
                    .chain(&brick.conflicts)
                {
                    let space = tag.split(':').next().unwrap_or_default();
                    assert!(
                        [
                            "subject", "crop", "scenery", "title", "camera", "color", "depth"
                        ]
                        .contains(&space),
                        "{}: {} speaks {tag}",
                        entry.id,
                        brick.key
                    );
                }
            }
        }
    }

    #[test]
    fn a_pack_installs_whole_under_its_family_once() {
        let conn = studio();
        let film = entry("film-photo");
        let done = install_from(&conn, &Source::Builtin, &snapshot(), "film-photo", None).unwrap();
        assert_eq!(done.added, film.brick_count);
        let config = profile::active(&conn).unwrap().unwrap().config;
        assert!(
            config
                .style_type("image-style")
                .unwrap()
                .families
                .iter()
                .any(|f| f.key == "film-photo"),
            "the family is registered"
        );
        let opened = open(&conn, &Source::Builtin, &snapshot(), "film-photo").unwrap();
        assert!(
            opened
                .bricks
                .iter()
                .all(|(_, s)| *s == BrickStanding::Installed)
        );
        let again = install_from(&conn, &Source::Builtin, &snapshot(), "film-photo", None).unwrap();
        assert_eq!(
            (again.added, again.kept),
            (0, film.brick_count),
            "never doubled"
        );
        let logged: i64 = conn
            .query_row(
                "SELECT count(*) FROM operation WHERE kind = 'style.create'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            logged,
            i64::from(film.brick_count),
            "each brick is in the history"
        );
    }

    #[test]
    fn a_newer_pack_renews_what_nobody_touched_and_keeps_the_owner_s_words() {
        let conn = studio();
        let folder = tempfile::tempdir().unwrap();
        let copy = |description: &str| {
            let film = entry("film-photo");
            let mut manifest = manifest(&Source::Builtin, &snapshot(), &film).unwrap();
            manifest.bricks[0].description = description.into();
            manifest.bricks[1].description = format!("{description} too");
            let text = serde_json::to_vec_pretty(&manifest).unwrap();
            std::fs::create_dir_all(folder.path().join("packs")).unwrap();
            std::fs::write(folder.path().join("packs/film-photo.json"), &text).unwrap();
            let mut listed = film.clone();
            listed.digest = hex(&sha2::Sha256::digest(&text));
            let index = PackIndex {
                format_version: FORMAT,
                packs: vec![listed],
            };
            std::fs::write(
                folder.path().join("index.json"),
                serde_json::to_vec(&index).unwrap(),
            )
            .unwrap();
        };
        let source = Source::Folder(folder.path().display().to_string());
        copy("first words");
        install_from(&conn, &source, &snapshot(), "film-photo", None).unwrap();

        // The owner rewords the second brick; the pack then rewords both.
        let key = profile::active(&conn).unwrap().unwrap().key;
        let second = brick_id(&key, "film-photo", &entry_keys()[1]);
        crate::actions::style::update(
            &conn,
            &second,
            StyleBrickPatch {
                description: Some(Some("my own words".into())),
                ..StyleBrickPatch::default()
            },
        )
        .unwrap();
        copy("newer words");
        let done = install_from(&conn, &source, &snapshot(), "film-photo", None).unwrap();
        assert_eq!(done.renewed, 1, "{done:?}");
        let first = brick_id(&key, "film-photo", &entry_keys()[0]);
        assert_eq!(
            style_brick::get(&conn, &first)
                .unwrap()
                .unwrap()
                .description
                .as_deref(),
            Some("newer words")
        );
        assert_eq!(
            style_brick::get(&conn, &second)
                .unwrap()
                .unwrap()
                .description
                .as_deref(),
            Some("my own words")
        );
    }

    fn entry_keys() -> Vec<String> {
        manifest(&Source::Builtin, &snapshot(), &entry("film-photo"))
            .unwrap()
            .bricks
            .into_iter()
            .map(|b| b.key)
            .collect()
    }

    #[test]
    fn a_pack_s_preview_is_the_first_picture_of_the_brick_it_adds() {
        let (conn, _, _dir) = crate::fixtures::workspace_on_disk();
        let folder = tempfile::tempdir().unwrap();
        let film = entry("film-photo");
        let mut manifest = manifest(&Source::Builtin, &snapshot(), &film).unwrap();
        manifest.bricks[0].preview_webp = Some("previews/first.webp".into());
        manifest.bricks[1].preview_webp = Some("previews/missing.webp".into());
        let text = serde_json::to_vec_pretty(&manifest).unwrap();
        std::fs::create_dir_all(folder.path().join("packs")).unwrap();
        std::fs::create_dir_all(folder.path().join("previews")).unwrap();
        std::fs::write(folder.path().join("packs/film-photo.json"), &text).unwrap();
        std::fs::write(
            folder.path().join("previews/first.webp"),
            b"RIFF0000WEBPVP8 ",
        )
        .unwrap();
        let mut listed = film.clone();
        listed.digest = hex(&sha2::Sha256::digest(&text));
        let index = PackIndex {
            format_version: FORMAT,
            packs: vec![listed],
        };
        std::fs::write(
            folder.path().join("index.json"),
            serde_json::to_vec(&index).unwrap(),
        )
        .unwrap();

        let source = Source::Folder(folder.path().display().to_string());
        let done = install_from(&conn, &source, &snapshot(), "film-photo", None).unwrap();
        assert_eq!((done.added, done.previews), (5, 1), "{done:?}");
        let key = profile::active(&conn).unwrap().unwrap().key;
        let first = brick_id(&key, "film-photo", &manifest.bricks[0].key);
        let pictures = crate::asset::for_style_brick(&conn, &first).unwrap();
        assert_eq!(pictures.len(), 1);
        assert!(
            std::path::Path::new(&pictures[0].path).is_file(),
            "the bytes are in the space"
        );
        let brick = style_brick::get(&conn, &first).unwrap().unwrap();
        assert_eq!(brick.reference_count, 1);
        assert_eq!(
            brick.origin,
            style_brick::Origin::Set,
            "a picture is not a change of words"
        );
    }

    #[test]
    fn a_deleted_brick_stays_out_of_the_pack_and_comes_back_by_hand() {
        let conn = studio();
        install_from(&conn, &Source::Builtin, &snapshot(), "film-photo", None).unwrap();
        let key = profile::active(&conn).unwrap().unwrap().key;
        let first = entry_keys()[0].clone();
        let id = brick_id(&key, "film-photo", &first);
        crate::actions::trash::discard(&conn, crate::trash::Entity::Style, &id).unwrap();

        let whole = install_from(&conn, &Source::Builtin, &snapshot(), "film-photo", None).unwrap();
        assert_eq!(whole.added, 0, "{whole:?}");
        assert!(style_brick::get(&conn, &id).unwrap().is_none());

        let one = install_from(
            &conn,
            &Source::Builtin,
            &snapshot(),
            "film-photo",
            Some(&[first]),
        )
        .unwrap();
        assert_eq!(one.added, 1);
        assert!(style_brick::get(&conn, &id).unwrap().is_some());
    }

    #[test]
    fn a_pack_fits_where_its_type_is_and_is_unfit_elsewhere() {
        // Studio has the camera's own type of movement since v0.133.
        let conn = studio();
        let done =
            install_from(&conn, &Source::Builtin, &snapshot(), "camera-moves", None).unwrap();
        assert_eq!((done.added, done.unfit), (5, 0));

        let novel = db::open_in_memory().unwrap();
        profile::seed(&novel, "novel").unwrap();
        let opened = open(&novel, &Source::Builtin, &snapshot(), "camera-moves").unwrap();
        let standings: Vec<_> = opened.bricks.iter().map(|(_, s)| *s).collect();
        assert_eq!(pack_standing(&standings), PackStanding::Unfit);
        let done =
            install_from(&novel, &Source::Builtin, &snapshot(), "camera-moves", None).unwrap();
        assert_eq!((done.added, done.unfit), (0, 5));
    }

    #[test]
    fn a_registry_is_a_folder_or_a_site_and_offline_reads_only_folders() {
        let app = db::open_app_in_memory().unwrap();
        add_source(&app, &Source::Folder("C:/packs".into())).unwrap();
        add_source(&app, &Source::Url("https://example.org/registry/".into())).unwrap();
        let (sources, offline) = sources(&app).unwrap();
        assert_eq!(sources.len(), 3);
        assert_eq!(sources[0], Source::Builtin);
        assert!(!offline);
        set_offline(&app, true).unwrap();
        assert!(super::sources(&app).unwrap().1);
        let conn = studio();
        let (listed, failed) =
            catalogue(&conn, &super::sources(&app).unwrap().0, true, &snapshot());
        assert!(
            listed
                .iter()
                .all(|p| p.source != Source::Url("https://example.org/registry/".into()))
        );
        assert_eq!(failed.len(), 1, "the missing folder is said: {failed:?}");
        remove_source(&app, &Source::Folder("C:/packs".into())).unwrap();
        assert_eq!(super::sources(&app).unwrap().0.len(), 2);
    }
}
