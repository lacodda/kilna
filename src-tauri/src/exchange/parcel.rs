//! A work as a parcel: everything that is the work's own, in one value that
//! can leave a space and land in another (v0.113).
//!
//! Three things go out of kilna as a parcel and come back as one. A page of
//! the export carries it at its foot, so a page written by kilna can be read
//! back by kilna; a `.kilna.zip` carries it as `work.json` beside the files
//! it names; and a plain text brought in from outside is made into one before
//! it lands, so there is one road into the workspace rather than three.
//!
//! A parcel keeps the ids and the moments its rows were made under. A work
//! carried from one space to another is the same work there - the same
//! versions, scored on the same days - and a later merge of two spaces can
//! tell a work it has met from one it has not. That is also what makes
//! bringing the same parcel twice a no-op rather than a second copy.
//!
//! What it carries is what belongs to the work alone: its fields, its
//! versions with their lineage, its scores, its release and what that goes
//! out under, its notes, its board with the pictures drawn for it, and its
//! files. What it leaves behind is what points at something else in the space
//! it came from - the collection it stands in, the works it was made from,
//! the cards and bricks its cover is built of, the trials of an experiment,
//! the boards of its conception, what the audience said. Those are the
//! space's as much as the work's, and carrying them is the merge this is the
//! forerunner of.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::asset::{self, NewAsset};
use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::note::{self, NewNote, NoteFilter};
use crate::profile::config::ProfileConfig;
use crate::release::{self, NewRelease};
use crate::scene::{self, NewScene};
use crate::score::{self, NewScore};
use crate::work::version::{self, NewVersion};
use crate::work::{self, NewWork};
use crate::{scene_frame, tombstone};

/// The shape of a parcel. Bumped when a field changes meaning; a parcel of a
/// later shape is refused rather than half-read.
pub const FORMAT: u32 = 1;

/// The parcel inside a `.kilna.zip`.
pub const MANIFEST: &str = "work.json";

/// The page beside it: the same work for a person's eyes.
pub const PAGE: &str = "work.md";

/// The folder of a `.kilna.zip` the work's files lie in.
pub const MEDIA: &str = "media/";

/// What a packed work's file is called after its title.
pub const EXTENSION: &str = "kilna.zip";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parcel {
    pub format: u32,
    /// The kilna that packed it, for a person reading the file.
    #[serde(default)]
    pub kilna: String,
    pub work: PackedWork,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub versions: Vec<PackedVersion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scores: Vec<PackedScore>,
    /// A publication goes out once (ADR 0051), so a work has one or none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<PackedRelease>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<PackedNote>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenes: Vec<PackedScene>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<PackedAsset>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frames: Vec<PackedFrame>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedWork {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_pinned_at: Option<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub meta: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<i64>,
    /// What the work feels like (FR-02). Its keys are the profile's; a
    /// space that does not have one of them lands the work without it
    /// rather than refusing the parcel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vibe: Option<crate::work::Vibe>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier_pinned: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier_pinned_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier_pin_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmarked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedVersion {
    pub id: String,
    pub role: String,
    pub revision: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Left out of a page's parcel: the body is on the page, between its
    /// marks, and is read from there - what a person changed in the text
    /// outside is what comes back.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub meta: Map<String, Value>,
    /// The version it was written from, when that one travels too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub current: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub main: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedScore {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub axes: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rater: Option<String>,
    pub scored_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedRelease {
    pub id: String,
    pub kind: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot_pinned_at: Option<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub meta: Map<String, Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedNote {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedScene {
    pub id: String,
    pub position: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shot_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub blocks: Map<String, Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedAsset {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_name: Option<String>,
    /// The name its bytes are stored under: the file in `media/`.
    pub file: String,
    /// On the release rather than on the work, when it is.
    #[serde(default, skip_serializing_if = "is_false")]
    pub of_release: bool,
    /// The note whose body shows it, when it is a picture of one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedFrame {
    pub id: String,
    pub scene: String,
    pub asset: String,
    pub kind: String,
    pub position: i64,
    #[serde(default, skip_serializing_if = "is_false")]
    pub selected: bool,
    pub created_at: String,
}

// The signature serde asks of `skip_serializing_if`.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(value: &bool) -> bool {
    !*value
}

impl Parcel {
    /// A parcel for a text that arrived from outside: one work with one
    /// version, minted here and now.
    pub fn of_text(kind: &str, role: &str, title: &str, body: &str, tags: Vec<String>) -> Self {
        let work = Minted::fresh();
        let at = work.at().to_owned();
        let versions = if body.trim().is_empty() {
            Vec::new()
        } else {
            vec![PackedVersion {
                id: Minted::fresh().id().to_owned(),
                role: role.to_owned(),
                revision: 1,
                label: None,
                body: body.to_owned(),
                meta: Map::new(),
                parent: None,
                created_at: at.clone(),
                current: true,
                main: false,
            }]
        };
        Self {
            format: FORMAT,
            kilna: env!("CARGO_PKG_VERSION").to_owned(),
            work: PackedWork {
                id: work.id().to_owned(),
                kind: kind.to_owned(),
                title: title.to_owned(),
                // Empty is "the kind's starting one": the landing fills it in.
                status: String::new(),
                status_pinned_at: None,
                meta: Map::new(),
                tags,
                marks: Vec::new(),
                stage: None,
                vibe: None,
                tier_pinned: None,
                tier_pinned_at: None,
                tier_pin_reason: None,
                bookmarked_at: None,
                created_at: at.clone(),
                updated_at: at,
            },
            versions,
            scores: Vec::new(),
            release: None,
            notes: Vec::new(),
            scenes: Vec::new(),
            assets: Vec::new(),
            frames: Vec::new(),
        }
    }

    /// The parcel as a page carries it: everything but the bodies, which
    /// stand on the page.
    pub fn without_bodies(&self) -> Self {
        let mut bare = self.clone();
        for version in &mut bare.versions {
            version.body.clear();
        }
        bare
    }
}

/// Pack a work: read everything that is its own into a parcel.
pub fn of_work(conn: &Connection, work_id: &str) -> Result<Parcel> {
    let found = work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;

    let summaries = version::list(conn, work_id)?;
    let ids: BTreeSet<&str> = summaries.iter().map(|one| one.id.as_str()).collect();
    let mut versions = Vec::with_capacity(summaries.len());
    for summary in &summaries {
        let Some(full) = version::get(conn, &summary.id)? else {
            continue;
        };
        versions.push(PackedVersion {
            id: full.id,
            role: full.role,
            revision: full.revision,
            label: full.label,
            body: full.body,
            meta: full.meta,
            parent: full
                .parent_version_id
                .filter(|parent| ids.contains(parent.as_str())),
            created_at: full.created_at,
            current: found.current_version_id.as_deref() == Some(summary.id.as_str()),
            main: summary.is_main,
        });
    }
    // In the order they are put back: a parent before what was written from
    // it, each role counting up.
    versions.sort_by(|a, b| (&a.role, a.revision).cmp(&(&b.role, b.revision)));

    let mut scores: Vec<PackedScore> = score::history(conn, work_id)?
        .into_iter()
        .map(|one| PackedScore {
            id: one.id,
            version: one.version_id.filter(|id| ids.contains(id.as_str())),
            axes: one.axes,
            note: one.note,
            rater: one.rater,
            scored_at: one.scored_at,
        })
        .collect();
    // Oldest first, as they are put back: the history reads newest first,
    // and two scores given in one tick keep the order they were given in.
    scores.reverse();

    let packed_release = release::of_work(conn, work_id)?.map(|one| PackedRelease {
        id: one.id,
        kind: one.kind,
        status: one.status,
        scheduled_at: one.scheduled_at,
        scheduled_time: one.scheduled_time,
        time_zone: one.time_zone,
        released_at: one.released_at,
        url: one.url,
        slot_pinned_at: one.slot_pinned_at,
        meta: one.meta,
        created_at: one.created_at,
        updated_at: one.updated_at,
    });

    let mut notes: Vec<PackedNote> = note::list(
        conn,
        &found.profile_id,
        &NoteFilter {
            work_id: Some(work_id.to_owned()),
            // A hero of the work is a card of the canon: the space's.
            canon: Some(false),
            ..NoteFilter::default()
        },
    )?
    .into_iter()
    .map(|one| PackedNote {
        id: one.id,
        kind: one.kind,
        title: one.title,
        body: one.body,
        attribution: one.attribution,
        tags: one.tags,
        created_at: one.created_at,
        updated_at: one.updated_at,
    })
    .collect();
    // A stable sort: notes made in one tick keep the order they are listed in.
    notes.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    let scenes: Vec<PackedScene> = scene::for_work(conn, work_id)?
        .into_iter()
        .map(|one| PackedScene {
            id: one.id,
            position: one.position,
            section: one.section,
            starts_at: one.starts_at,
            ends_at: one.ends_at,
            shot_type: one.shot_type,
            layer: one.layer,
            description: one.description,
            blocks: one.blocks,
            created_at: one.created_at,
        })
        .collect();

    let frames: Vec<PackedFrame> = scene_frame::for_work(conn, work_id)?
        .into_iter()
        .map(|one| PackedFrame {
            id: one.id,
            scene: one.scene_id,
            asset: one.asset_id,
            kind: one.kind,
            position: one.position,
            selected: one.is_selected,
            created_at: one.created_at,
        })
        .collect();

    let mut files = asset::for_work(conn, work_id)?;
    if let Some(one) = &packed_release {
        files.extend(asset::for_release(conn, &one.id)?);
    }
    // The pictures a note's body shows are the note's own files.
    for one in &notes {
        let mut statement =
            conn.prepare("SELECT id FROM asset WHERE note_id = ?1 ORDER BY created_at, rowid")?;
        let ids = statement
            .query_map(params![one.id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for id in ids {
            files.extend(asset::get(conn, &id)?);
        }
    }
    let mut seen = BTreeSet::new();
    let assets: Vec<PackedAsset> = files
        .into_iter()
        // A take of a trial, a reference of a brick, a picture of a fact:
        // those hang on something that stays behind.
        .filter(|one| {
            one.trial_id.is_none() && one.style_brick_id.is_none() && one.canon_fact_id.is_none()
        })
        .filter(|one| seen.insert(one.id.clone()))
        .filter_map(|one| {
            let file = Path::new(&one.path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())?;
            Some(PackedAsset {
                id: one.id,
                kind: one.kind,
                label: one.label,
                original_name: one.original_name,
                file,
                of_release: one.release_id.is_some(),
                note: one.note_id,
                created_at: one.created_at,
            })
        })
        .collect();

    Ok(Parcel {
        format: FORMAT,
        kilna: env!("CARGO_PKG_VERSION").to_owned(),
        work: PackedWork {
            id: found.id,
            kind: found.kind,
            title: found.title,
            status: found.status,
            status_pinned_at: found.status_pinned_at,
            meta: found.meta,
            tags: found.tags,
            marks: found.marks,
            stage: Some(found.stage),
            vibe: found.vibe,
            tier_pinned: found.tier_pinned,
            tier_pinned_at: found.tier_pinned_at,
            tier_pin_reason: found.tier_pin_reason,
            bookmarked_at: found.bookmarked_at,
            created_at: found.created_at,
            updated_at: found.updated_at,
        },
        versions,
        scores,
        release: packed_release,
        notes,
        scenes,
        assets,
        frames,
    })
}

/// How a parcel stands to the space it is about to land in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum ParcelStanding {
    /// Not here: it can be brought in.
    New,
    /// Already here - the same work by its id, or for a text from outside
    /// one of the same title. Passed over rather than merged: bringing a
    /// work in never rewrites what is already there.
    Here,
    /// Was here and was thrown away. Passed over too: a parcel that
    /// resurrects what the person deleted is a merge that lost.
    Deleted,
}

/// Where a parcel stands. `by_title` is for a text from outside, which has
/// no id anybody has met and is known only by what it is called.
pub fn standing(
    conn: &Connection,
    profile_id: &str,
    parcel: &Parcel,
    by_title: bool,
) -> Result<ParcelStanding> {
    if by_title {
        let here: bool = conn
            .query_row(
                "SELECT 1 FROM work WHERE profile_id = ?1 AND title = ?2 LIMIT 1",
                params![profile_id, parcel.work.title],
                |_| Ok(true),
            )
            .map(Some)
            .or_else(|cause| match cause {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?
            .unwrap_or(false);
        if here {
            return Ok(ParcelStanding::Here);
        }
        if tombstone::buried_work_title(conn, profile_id, &parcel.work.title)? {
            return Ok(ParcelStanding::Deleted);
        }
        return Ok(ParcelStanding::New);
    }
    if work::get(conn, &parcel.work.id)?.is_some() {
        return Ok(ParcelStanding::Here);
    }
    if tombstone::get(conn, "work", &parcel.work.id)?.is_some_and(|stone| stone.is_buried()) {
        return Ok(ParcelStanding::Deleted);
    }
    Ok(ParcelStanding::New)
}

/// Refuse a parcel the space's craft has no words for, before anything is
/// written: a kind it does not make, a role or a door that kind does not
/// have, a board on a kind with none.
pub fn check(config: &ProfileConfig, parcel: &Parcel) -> Result<()> {
    if parcel.format > FORMAT {
        return Err(Error::refused("parcel.newerFormat").param("format", parcel.format));
    }
    if parcel.work.title.trim().is_empty() {
        return Err(Error::refused("parcel.needsTitle"));
    }
    let kind = &parcel.work.kind;
    if config.kind(kind).is_none() {
        return Err(Error::refused("parcel.unknownKind").param("kind", kind.as_str()));
    }
    for one in &parcel.versions {
        config.require_role(kind, &one.role)?;
    }
    if let Some(one) = &parcel.release {
        config.require_release_kind(kind, &one.kind)?;
    }
    if !parcel.scenes.is_empty() {
        config.require_storyboard(kind)?;
    }
    Ok(())
}

/// Where a parcel's files are read from while it lands.
pub trait Files {
    /// Write the file stored as `name` to `into`. `false` when the parcel
    /// came without it.
    fn extract(&mut self, name: &str, into: &Path) -> Result<bool>;
}

/// A parcel that came with no files: a page, a text.
pub struct NoFiles;

impl Files for NoFiles {
    fn extract(&mut self, _name: &str, _into: &Path) -> Result<bool> {
        Ok(false)
    }
}

/// What landing a parcel made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct LandedWork {
    pub id: String,
    pub title: String,
    pub versions: usize,
    pub scores: usize,
    pub notes: usize,
    pub scenes: usize,
    /// Files that arrived with it.
    pub files: usize,
    /// Files it names that did not arrive: a page carries none, and a file
    /// missing from the workspace it was packed in is missing from the zip.
    pub missing: usize,
}

/// Put a parcel's rows into the space, under the ids and moments it carries.
///
/// Not a gesture by itself - `actions::work::bring` is, and wraps this in its
/// unit and its operation. Refuses a work that is already here; the caller
/// has asked [`standing`] and passed those over.
pub fn land(
    conn: &Connection,
    profile_id: &str,
    media: Option<&Path>,
    parcel: &Parcel,
    files: &mut dyn Files,
) -> Result<LandedWork> {
    let config = crate::profile::config_for(conn, profile_id)?;
    check(&config, parcel)?;
    let packed = &parcel.work;
    if work::get(conn, &packed.id)?.is_some() {
        return Err(Error::refused("parcel.alreadyHere").param("title", packed.title.as_str()));
    }
    let kind = config.vocabulary(&packed.kind);

    // The status it left with, when this craft has that word; otherwise the
    // one a new work starts in, and no pin - a hold on a status the space
    // does not name would hold nothing.
    let known = kind.statuses.iter().any(|one| one.key == packed.status);
    let made = work::create_minted(
        conn,
        profile_id,
        NewWork {
            kind: packed.kind.clone(),
            title: packed.title.trim().to_owned(),
            tags: packed.tags.clone(),
            marks: packed.marks.clone(),
            status: known.then(|| packed.status.clone()),
            collection_id: None,
            meta: Some(packed.meta.clone()),
            ordinal: None,
        },
        Minted::of(packed.id.clone(), packed.created_at.clone()),
    )?;

    let mut landed = LandedWork {
        id: made.id.clone(),
        title: made.title.clone(),
        versions: 0,
        scores: 0,
        notes: 0,
        scenes: 0,
        files: 0,
        missing: 0,
    };

    // Versions, each role counting up, a parent before its children.
    let mut versions: Vec<&PackedVersion> = parcel.versions.iter().collect();
    versions.sort_by(|a, b| (&a.role, a.revision).cmp(&(&b.role, b.revision)));
    let mut here = BTreeSet::new();
    for one in &versions {
        version::create_minted(
            conn,
            &made.id,
            NewVersion {
                role: one.role.clone(),
                body: one.body.clone(),
                label: one.label.clone(),
                meta: Some(one.meta.clone()),
                // Which one is current is said once, below, as it was packed.
                make_current: false,
                parent_version_id: one
                    .parent
                    .clone()
                    .filter(|parent| here.contains(parent.as_str())),
                trial_id: None,
            },
            Minted::of(one.id.clone(), one.created_at.clone()),
        )?;
        // Its own number, not the next free one: a line that lost a revision
        // keeps the gap, and "revision 4" stays revision 4.
        conn.execute(
            "UPDATE work_version SET revision = ?2 WHERE id = ?1",
            params![one.id, one.revision],
        )?;
        here.insert(one.id.as_str());
        landed.versions += 1;
    }

    // Scores before the current version is named: one given to the work as a
    // whole stays so, rather than being read as given to the current text.
    for one in &parcel.scores {
        score::create_minted(
            conn,
            &made.id,
            NewScore {
                axes: one.axes.clone(),
                version_id: one.version.clone().filter(|id| here.contains(id.as_str())),
                note: one.note.clone(),
                rater: one.rater.clone(),
            },
            Minted::of(one.id.clone(), one.scored_at.clone()),
        )?;
        landed.scores += 1;
    }

    for one in versions.iter().filter(|one| one.main) {
        // A role this craft counts as a draft has no main version; the mark
        // is dropped rather than refusing the whole work.
        if !version::is_draft(conn, &made.id, &one.role)? {
            version::set_main_at(conn, &made.id, &one.id, &packed.updated_at)?;
        }
    }
    if let Some(current) = versions.iter().find(|one| one.current)
        && version::is_draft(conn, &made.id, &current.role)?
    {
        conn.execute(
            "UPDATE work SET current_version_id = ?2 WHERE id = ?1",
            params![made.id, current.id],
        )?;
    }

    if let Some(one) = &parcel.release {
        release::create_minted(
            conn,
            NewRelease {
                work_id: made.id.clone(),
                kind: one.kind.clone(),
                scheduled_at: one.scheduled_at.clone(),
                meta: Some(one.meta.clone()),
                scheduled_time: one.scheduled_time.clone(),
                time_zone: one.time_zone.clone(),
            },
            Minted::of(one.id.clone(), one.created_at.clone()),
        )?;
        // As it stood: out or not, held to its day or not, under the words
        // it carried - a tail given on arrival would be a second premiere.
        conn.execute(
            "UPDATE release SET status = ?2, released_at = ?3, url = ?4, slot_pinned_at = ?5,
                    meta = ?6, updated_at = ?7
              WHERE id = ?1",
            params![
                one.id,
                one.status,
                one.released_at,
                one.url,
                one.slot_pinned_at,
                Value::Object(one.meta.clone()).to_string(),
                one.updated_at,
            ],
        )?;
    }

    let mut noted = BTreeSet::new();
    for one in &parcel.notes {
        // A kind of note this craft does not keep lands as a plain note:
        // the words are what was carried.
        let kind = Some(one.kind.clone())
            .filter(|kind| config.note_kind(kind).is_some_and(|known| !known.is_card()));
        note::create_minted(
            conn,
            profile_id,
            NewNote {
                body: one.body.clone(),
                kind,
                title: one.title.clone(),
                attribution: one.attribution.clone(),
                work_id: Some(made.id.clone()),
                tags: one.tags.clone(),
                ..NewNote::default()
            },
            Minted::of(one.id.clone(), one.created_at.clone()),
        )?;
        conn.execute(
            "UPDATE note SET updated_at = ?2 WHERE id = ?1",
            params![one.id, one.updated_at],
        )?;
        noted.insert(one.id.as_str());
        landed.notes += 1;
    }

    let mut staged = BTreeSet::new();
    for one in &parcel.scenes {
        scene::create_minted(
            conn,
            profile_id,
            NewScene {
                work_id: made.id.clone(),
                position: Some(one.position),
                section: one.section.clone(),
                starts_at: one.starts_at,
                ends_at: one.ends_at,
                shot_type: one.shot_type.clone(),
                layer: one.layer.clone(),
                description: Some(one.description.clone()),
                blocks: Some(one.blocks.clone()),
            },
            Minted::of(one.id.clone(), one.created_at.clone()),
        )?;
        staged.insert(one.id.as_str());
        landed.scenes += 1;
    }

    // The files, each under the name it was stored as - which is its id, so
    // a note's body goes on showing the picture it names.
    let mut arrived = BTreeSet::new();
    if !parcel.assets.is_empty() {
        let holding = tempfile::Builder::new()
            .prefix("kilna-parcel-")
            .tempdir()
            .map_err(|cause| Error::Internal(format!("could not hold the files: {cause}")))?;
        let mut written: Vec<PathBuf> = Vec::new();
        let outcome = (|| -> Result<()> {
            for one in &parcel.assets {
                // A file name and nothing more: a parcel is read from a file
                // somebody handed over, and `../` in it is not a name.
                let name = Path::new(&one.file)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .filter(|name| name == &one.file);
                let (Some(media), Some(name)) = (media, name) else {
                    landed.missing += 1;
                    continue;
                };
                if one.note.as_deref().is_some_and(|id| !noted.contains(id)) {
                    landed.missing += 1;
                    continue;
                }
                let source = holding.path().join(&name);
                if !files.extract(&name, &source)? {
                    landed.missing += 1;
                    continue;
                }
                let stored = asset::attach_minted(
                    conn,
                    profile_id,
                    media,
                    &source,
                    NewAsset {
                        work_id: (one.note.is_none() && !one.of_release).then(|| made.id.clone()),
                        release_id: parcel
                            .release
                            .as_ref()
                            .filter(|_| one.of_release)
                            .map(|release| release.id.clone()),
                        note_id: one.note.clone(),
                        kind: Some(one.kind.clone()),
                        label: one.label.clone(),
                        ..NewAsset::default()
                    },
                    Minted::of(one.id.clone(), one.created_at.clone()),
                )?;
                written.push(PathBuf::from(&stored.path));
                // What the world outside called it, not what the holding
                // folder did.
                conn.execute(
                    "UPDATE asset SET original_name = ?2 WHERE id = ?1",
                    params![one.id, one.original_name],
                )?;
                arrived.insert(one.id.as_str());
                landed.files += 1;
            }
            for one in &parcel.frames {
                if !staged.contains(one.scene.as_str()) || !arrived.contains(one.asset.as_str()) {
                    continue;
                }
                conn.execute(
                    "INSERT INTO scene_frame (id, profile_id, scene_id, asset_id, kind, position,
                                              is_selected, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        one.id,
                        profile_id,
                        one.scene,
                        one.asset,
                        one.kind,
                        one.position,
                        one.selected,
                        one.created_at,
                    ],
                )?;
            }
            Ok(())
        })();
        if let Err(cause) = outcome {
            // The rows roll back with the unit; the bytes would not.
            for path in written {
                let _ = std::fs::remove_file(path);
            }
            return Err(cause);
        }
    }

    // Its status is a conclusion from what just landed - a release that is
    // out, a score - unless it was held by hand; drawn before the work's own
    // moment is put back, since drawing it is a write.
    crate::actions::restate(conn, profile_id, &made.id);

    // The work as it stood, last: every write above moved `updated_at`.
    conn.execute(
        "UPDATE work SET status_pinned_at = ?2, stage = ?3, tier_pinned = ?4, tier_pinned_at = ?5,
                tier_pin_reason = ?6, bookmarked_at = ?7, updated_at = ?8, vibe = ?9
          WHERE id = ?1",
        params![
            made.id,
            packed.status_pinned_at.as_ref().filter(|_| known),
            // A parcel packed by a release before v0.119 carries a stop of the
            // old scale, or none.
            crate::profile::config::normalize_stage(packed.stage),
            packed.tier_pinned,
            packed.tier_pinned_at,
            packed.tier_pin_reason,
            packed.bookmarked_at,
            packed.updated_at,
            // In the words of the space it lands in, or not at all.
            packed
                .vibe
                .clone()
                .and_then(crate::work::Vibe::kept)
                .filter(|vibe| {
                    crate::profile::config_for(conn, profile_id)
                        .is_ok_and(|config| config.vibe().check(vibe).is_ok())
                })
                .and_then(|vibe| serde_json::to_string(&vibe).ok()),
        ],
    )?;

    Ok(landed)
}

/// What packing a work into a file made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct PackedParcel {
    pub path: String,
    /// Files that went in beside the parcel.
    pub files: usize,
    /// Files the work names that are not on the disk any more.
    pub missing: usize,
}

/// The name a packed work's file is offered under.
pub fn file_name(conn: &Connection, work_id: &str) -> Result<String> {
    let found = work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    Ok(format!(
        "{}.{EXTENSION}",
        super::export::slug(&found.title, &found.id)
    ))
}

/// Pack a work into a `.kilna.zip` at `target`: the parcel, the page a
/// person reads, and the work's files.
pub fn write_zip(conn: &Connection, work_id: &str, target: &Path) -> Result<PackedParcel> {
    let parcel = of_work(conn, work_id)?;
    let page = super::export::page_of(conn, work_id)?;
    // Where each stored name lies now: the workspace may have moved since a
    // row's path was written, and the row is what was rebased.
    let mut paths = BTreeMap::new();
    for one in &parcel.assets {
        if let Some(found) = asset::get(conn, &one.id)? {
            paths.insert(one.file.clone(), PathBuf::from(found.path));
        }
    }

    let not_written = |cause: &dyn std::fmt::Display| {
        Error::refused("file.notWritten")
            .param("path", target.display().to_string())
            .param("cause", cause.to_string())
    };
    let file = std::fs::File::create(target).map_err(|cause| not_written(&cause))?;
    let mut zip = zip::ZipWriter::new(file);
    let text = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    // Pictures, sound and video are compressed already; deflating a clip
    // again costs minutes and saves nothing.
    let stored = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .large_file(true);

    zip.start_file(MANIFEST, text)
        .map_err(|cause| not_written(&cause))?;
    zip.write_all(serde_json::to_string_pretty(&parcel)?.as_bytes())?;
    zip.start_file(PAGE, text)
        .map_err(|cause| not_written(&cause))?;
    zip.write_all(page.as_bytes())?;

    let mut packed = PackedParcel {
        path: target.display().to_string(),
        files: 0,
        missing: 0,
    };
    let mut done = BTreeSet::new();
    for one in &parcel.assets {
        // A clone's rows share their bytes: one file, once.
        if !done.insert(one.file.as_str()) {
            continue;
        }
        let Some(mut source) = paths
            .get(&one.file)
            .and_then(|path| std::fs::File::open(path).ok())
        else {
            packed.missing += 1;
            continue;
        };
        zip.start_file(format!("{MEDIA}{}", one.file), stored)
            .map_err(|cause| not_written(&cause))?;
        std::io::copy(&mut source, &mut zip)?;
        packed.files += 1;
    }
    zip.finish().map_err(|cause| not_written(&cause))?;
    Ok(packed)
}

/// A `.kilna.zip` opened for reading: its parcel, and its files on demand.
pub struct Zip {
    archive: zip::ZipArchive<std::fs::File>,
}

impl Zip {
    /// Open the file and read its parcel.
    pub fn open(path: &Path) -> Result<(Self, Parcel)> {
        let unreadable = |cause: &dyn std::fmt::Display| {
            Error::refused("parcel.unreadable")
                .param("path", path.display().to_string())
                .param("cause", cause.to_string())
        };
        let file = std::fs::File::open(path).map_err(|cause| unreadable(&cause))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|cause| unreadable(&cause))?;
        let mut manifest = String::new();
        archive
            .by_name(MANIFEST)
            .map_err(|_| {
                Error::refused("parcel.notAParcel").param("path", path.display().to_string())
            })?
            .read_to_string(&mut manifest)
            .map_err(|cause| unreadable(&cause))?;
        let parcel: Parcel = serde_json::from_str(&manifest).map_err(|cause| unreadable(&cause))?;
        Ok((Self { archive }, parcel))
    }
}

impl Files for Zip {
    fn extract(&mut self, name: &str, into: &Path) -> Result<bool> {
        let Ok(mut entry) = self.archive.by_name(&format!("{MEDIA}{name}")) else {
            return Ok(false);
        };
        let mut target = std::fs::File::create(into)?;
        std::io::copy(&mut entry, &mut target)?;
        Ok(true)
    }
}
