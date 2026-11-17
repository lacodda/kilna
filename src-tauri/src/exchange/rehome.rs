//! Moving the folders on disk to where the profile's templates now put them
//! (FR-04, v0.120).
//!
//! A work's folder is found, not attached ([`crate::folder`]): the kind's
//! template names it, and a template changed today points every work at a
//! new place at once - while the files stay where the old one put them.
//! Until v0.120 the shipped templates named a folder by the work's title,
//! `songs/{origin.title}/video`; they name it by its id now, `{kind}s/{id}`,
//! so a rename never moves it. A space made before that keeps its
//! templates, since changing them alone would lose sight of every file,
//! until this moves the files and switches the templates together.
//!
//! What it does is read off what kilna itself sees:
//!
//! * a work whose folder the **stored** template finds has every file under
//!   it moved to the folder the **shipped** template names, each keeping its
//!   place inside. Where one work's folder lies inside another's - a clip's
//!   inside its song's - a file belongs to the deepest work that holds it;
//! * what lies under the areas the old templates wrote into (`songs/` of
//!   `songs/{title}`) and belongs to no work goes to
//!   `_unsorted_migration/`, by the path it had. Nothing is deleted: a file
//!   is somewhere after this, where it was told to go or there;
//! * a folder outside the media folder is taken in only when it was named
//!   to look into and is called exactly what one work is called: what is
//!   finished at its top goes to the top of the work's folder, the rest
//!   under `raw/`;
//! * a file that would land on one that is already there is not written
//!   over: it goes to `_unsorted_migration/_conflicts/`.
//!
//! Everything else on the disk - `_shared/`, anything beside the areas, a
//! folder two works both resolve to - is left exactly as it stands and said
//! so.
//!
//! A plan first, always: [`plan`] reads and writes nothing, and [`report`]
//! writes it out as `migration_plan.json` and `migration_plan.md`. [`apply`]
//! takes a copy of the database, moves the files, removes the folders the
//! moves emptied, and only then - if every move landed - switches the stored
//! templates, through the same logged edit the profile editor makes. Run
//! again after a failure, it plans from the disk as it now is and finishes.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::folder::{self, Resolved};
use crate::profile::config::ProfileConfig;
use crate::work::{Work, WorkFilter};

/// Where what belongs to no work goes, under the media folder.
pub const UNSORTED: &str = "_unsorted_migration";
/// Where a file goes that would have landed on another, under [`UNSORTED`].
const CONFLICTS: &str = "_conflicts";
/// Where the files of a named folder go that are not what is finished.
const RAW: &str = "raw";
/// Shared material no work owns: never looked into, never moved.
const SHARED: &str = "_shared";

/// The files a walk goes to before it says the tree is too large to plan:
/// a media folder holds tens of thousands, a drive's root holds millions.
const MOST_FILES: usize = 400_000;

/// How a file's move was decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    /// It lies in the folder the stored template finds for a work.
    WorkFolder,
    /// It lies in a folder, outside the media folder, called what one work is.
    NamedFolder,
    /// It lies where the old templates wrote and belongs to no work.
    Unsorted,
    /// Where it was going, a file already is.
    Conflict,
}

/// One file, from where it is to where it goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileMove {
    pub from: String,
    pub to: String,
    pub rule: Rule,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_id: Option<String>,
    pub bytes: u64,
}

/// A template the move switches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TemplateChange {
    pub kind: String,
    pub before: Option<String>,
    pub after: String,
}

/// A work and what the move does for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkPlan {
    pub work_id: String,
    pub title: String,
    pub kind: String,
    /// Where its files are now, when the stored template finds them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Where the shipped template puts them.
    pub to: String,
    pub files: usize,
    pub bytes: u64,
    /// The finished file chosen by hand, and whether it is there after.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_there: Option<bool>,
}

/// Something looked at and left alone, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Left {
    pub path: String,
    pub reason: String,
}

/// What a move would do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Plan {
    pub root: String,
    /// The folders outside the media folder that were looked into.
    pub also: Vec<String>,
    pub templates: Vec<TemplateChange>,
    /// Every work the move touches: one with files to move, or a finished
    /// file to check.
    pub works: Vec<WorkPlan>,
    pub moves: Vec<FileMove>,
    pub left: Vec<Left>,
}

impl Plan {
    /// How many files go by each rule.
    pub fn counts(&self) -> BTreeMap<&'static str, usize> {
        let mut counts = BTreeMap::new();
        for one in &self.moves {
            *counts.entry(rule_name(one.rule)).or_insert(0) += 1;
        }
        counts
    }
}

fn rule_name(rule: Rule) -> &'static str {
    match rule {
        Rule::WorkFolder => "work_folder",
        Rule::NamedFolder => "named_folder",
        Rule::Unsorted => "unsorted",
        Rule::Conflict => "conflict",
    }
}

/// What applying a plan did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Applied {
    pub backup: String,
    pub moved: usize,
    /// Moves that did not happen, each with why. The templates are switched
    /// only when there are none.
    pub failed: Vec<Left>,
    pub folders_removed: usize,
    pub templates_switched: bool,
}

/// The extensions of what is finished, at the top of a named folder.
const FINISHED: &[&str] = &["mp4", "mov", "mkv", "wav", "flac", "mp3", "m4a"];
const PICTURES: &[&str] = &["png", "jpg", "jpeg", "webp"];

/// Plan the move of a profile's folders from its stored templates to those
/// of `target`, looking also into the folders `also` names.
pub fn plan(
    conn: &Connection,
    profile_id: &str,
    target: &ProfileConfig,
    also: &[PathBuf],
) -> Result<Plan> {
    let root = folder::root(conn, profile_id)?.ok_or_else(|| Error::refused("folder.noRoot"))?;
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err(
            Error::refused("folder.rootMissing").param("path", root.to_string_lossy().into_owned())
        );
    }
    let stored = crate::profile::config_for(conn, profile_id)?;

    let mut templates = Vec::new();
    let mut new_of: BTreeMap<String, String> = BTreeMap::new();
    for kind in &stored.work_kinds {
        let shipped = target
            .work_kinds
            .iter()
            .find(|one| one.key == kind.key)
            .and_then(|one| one.folder.clone());
        // A kind the target does not know keeps its template: there is
        // nowhere to move its works to.
        let after = shipped.or_else(|| kind.folder.clone());
        if let Some(after) = after {
            if kind.folder.as_deref() != Some(after.as_str()) {
                templates.push(TemplateChange {
                    kind: kind.key.clone(),
                    before: kind.folder.clone(),
                    after: after.clone(),
                });
            }
            new_of.insert(kind.key.clone(), after);
        }
    }

    // Where every work's files are, and where they go.
    let works = crate::work::list(conn, profile_id, &WorkFilter::default())?;
    let mut places: Vec<Place> = Vec::new();
    for work in &works {
        let origin = crate::publication::origin(conn, &stored, &work.id)?;
        let origin = origin.as_ref().unwrap_or(work);
        let at = |template: Option<&str>| -> Result<Option<PathBuf>> {
            Ok(match template {
                None => None,
                Some(template) => match folder::resolve(&root, template, work, origin)? {
                    Resolved::At(path) => Some(path),
                    Resolved::Unfilled(_) => None,
                },
            })
        };
        let old = at(stored
            .work_kinds
            .iter()
            .find(|kind| kind.key == work.kind)
            .and_then(|kind| kind.folder.as_deref()))?
        .filter(|path| path.is_dir());
        let Some(new) = at(new_of.get(&work.kind).map(String::as_str))? else {
            continue;
        };
        places.push(Place {
            work: work.clone(),
            old,
            new,
        });
    }

    let mut left = Vec::new();
    // A folder two works both resolve to is nobody's to move.
    let mut claims: BTreeMap<PathBuf, Vec<usize>> = BTreeMap::new();
    for (index, place) in places.iter().enumerate() {
        if let Some(old) = &place.old
            && *old != place.new
        {
            claims.entry(old.clone()).or_default().push(index);
        }
    }
    let mut owner_of: BTreeMap<PathBuf, usize> = BTreeMap::new();
    let mut held: BTreeSet<PathBuf> = BTreeSet::new();
    for (path, indexes) in &claims {
        if let [only] = indexes.as_slice() {
            owner_of.insert(path.clone(), *only);
        } else {
            held.insert(path.clone());
            left.push(Left {
                path: shown(path),
                reason: format!(
                    "the folder of {} works at once ({}): left where it is",
                    indexes.len(),
                    indexes
                        .iter()
                        .map(|index| places[*index].work.title.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            });
        }
    }
    // Where the files go is never swept as unsorted, and neither is what a
    // work already keeps under the template it will have.
    let kept: BTreeSet<PathBuf> = places.iter().map(|place| place.new.clone()).collect();

    let mut budget = MOST_FILES;
    let mut moves: Vec<FileMove> = Vec::new();
    let mut taken: BTreeSet<PathBuf> = BTreeSet::new();

    // The files of each work: the walk starts at the outermost folders, and
    // a file is the deepest work's that holds it.
    let outermost: Vec<&PathBuf> = owner_of
        .keys()
        .chain(held.iter())
        .filter(|path| {
            !owner_of
                .keys()
                .chain(held.iter())
                .any(|other| other != *path && path.starts_with(other))
        })
        .collect();
    for top in outermost {
        for file in files_under(top, &mut budget)? {
            let Some(deepest) = owner_of
                .keys()
                .chain(held.iter())
                .filter(|claim| file.starts_with(claim))
                .max_by_key(|claim| claim.components().count())
            else {
                continue;
            };
            let Some(index) = owner_of.get(deepest) else {
                // In a folder two works share.
                continue;
            };
            let place = &places[*index];
            let inside = file.strip_prefix(deepest).unwrap_or(&file);
            moves.push(landing(
                &root,
                &file,
                place.new.join(inside),
                Rule::WorkFolder,
                Some(&place.work.id),
                &mut taken,
            ));
        }
    }

    // What lies where the old templates wrote and belongs to no work.
    let mut areas: BTreeSet<PathBuf> = BTreeSet::new();
    for change in &templates {
        if let Some(before) = &change.before
            && let Some(area) = area_of(&root, before)
        {
            areas.insert(area);
        }
    }
    let unsorted = root.join(UNSORTED);
    let shared = root.join(SHARED);
    for area in areas.iter().filter(|area| {
        !areas
            .iter()
            .any(|other| other != *area && area.starts_with(other))
    }) {
        if !area.is_dir() || *area == root {
            continue;
        }
        for file in files_under(area, &mut budget)? {
            let spoken_for = claims.keys().any(|claim| file.starts_with(claim))
                || kept.iter().any(|new| file.starts_with(new))
                || file.starts_with(&unsorted)
                || file.starts_with(&shared);
            if spoken_for {
                continue;
            }
            let inside = file.strip_prefix(&root).unwrap_or(&file);
            moves.push(landing(
                &root,
                &file,
                unsorted.join(inside),
                Rule::Unsorted,
                None,
                &mut taken,
            ));
        }
    }

    // Folders named to look into: one called exactly what one work is.
    for dir in also {
        let Ok(entries) = std::fs::read_dir(dir) else {
            left.push(Left {
                path: shown(dir),
                reason: "could not be read".into(),
            });
            continue;
        };
        let mut named: Vec<PathBuf> = entries
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .map(|entry| entry.path())
            .collect();
        named.sort();
        for candidate in named {
            let name = candidate
                .file_name()
                .map(|name| name.to_string_lossy().trim().to_lowercase())
                .unwrap_or_default();
            let matching: Vec<&Place> = places
                .iter()
                .filter(|place| place.work.title.trim().to_lowercase() == name)
                .collect();
            match matching.as_slice() {
                [] => {
                    // Said only when it looks like it was meant: a folder
                    // that begins with a work's title.
                    if places.iter().any(|place| {
                        let title = place.work.title.trim().to_lowercase();
                        title.chars().count() >= 4 && name.starts_with(&title)
                    }) {
                        left.push(Left {
                            path: shown(&candidate),
                            reason:
                                "begins with a work's title, and no work is called exactly this"
                                    .into(),
                        });
                    }
                }
                [place] => {
                    for file in files_under(&candidate, &mut budget)? {
                        let inside = file.strip_prefix(&candidate).unwrap_or(&file);
                        let at_top = inside.components().count() == 1;
                        let to = if at_top && is_finished(inside) {
                            place.new.join(inside)
                        } else {
                            place.new.join(RAW).join(inside)
                        };
                        moves.push(landing(
                            &root,
                            &file,
                            to,
                            Rule::NamedFolder,
                            Some(&place.work.id),
                            &mut taken,
                        ));
                    }
                }
                several => left.push(Left {
                    path: shown(&candidate),
                    reason: format!("{} works are called this", several.len()),
                }),
            }
        }
    }

    // Each work the move touches, and its finished file once it is done.
    let landed: BTreeSet<&str> = moves.iter().map(|one| one.to.as_str()).collect();
    let mut plans = Vec::new();
    for place in &places {
        let own: Vec<&FileMove> = moves
            .iter()
            .filter(|one| one.work_id.as_deref() == Some(place.work.id.as_str()))
            .collect();
        let file_there = place.work.file.as_ref().map(|chosen| {
            let after = place.new.join(chosen);
            landed.contains(shown(&after).as_str()) || after.is_file()
        });
        if own.is_empty() && place.work.file.is_none() {
            continue;
        }
        plans.push(WorkPlan {
            work_id: place.work.id.clone(),
            title: place.work.title.clone(),
            kind: place.work.kind.clone(),
            from: place
                .old
                .as_ref()
                .filter(|old| **old != place.new)
                .map(|old| shown(old)),
            to: shown(&place.new),
            files: own.len(),
            bytes: own.iter().map(|one| one.bytes).sum(),
            file: place.work.file.clone(),
            file_there,
        });
    }

    Ok(Plan {
        root: shown(&root),
        also: also.iter().map(|dir| shown(dir)).collect(),
        templates,
        works: plans,
        moves,
        left,
    })
}

/// A work, where the stored template finds its folder, and where the new
/// one puts it.
struct Place {
    work: Work,
    old: Option<PathBuf>,
    new: PathBuf,
}

/// A path as a report writes it.
fn shown(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// One file's move, sent to the conflicts when where it is going is taken -
/// by a file on the disk, or by a move already planned.
fn landing(
    root: &Path,
    from: &Path,
    to: PathBuf,
    rule: Rule,
    work_id: Option<&str>,
    taken: &mut BTreeSet<PathBuf>,
) -> FileMove {
    let bytes = std::fs::metadata(from).map_or(0, |found| found.len());
    let (to, rule) = if to.exists() || taken.contains(&to) {
        let inside = to.strip_prefix(root).unwrap_or(&to).to_path_buf();
        let mut aside = root.join(UNSORTED).join(CONFLICTS).join(&inside);
        // Two files heading for one taken place: the second is numbered.
        let mut number = 2;
        while aside.exists() || taken.contains(&aside) {
            let mut name = inside
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            name.push_str(&format!(" ({number})"));
            if let Some(extension) = inside.extension() {
                name.push('.');
                name.push_str(&extension.to_string_lossy());
            }
            aside = root
                .join(UNSORTED)
                .join(CONFLICTS)
                .join(inside.with_file_name(name));
            number += 1;
        }
        (aside, Rule::Conflict)
    } else {
        (to, rule)
    };
    taken.insert(to.clone());
    FileMove {
        from: shown(from),
        to: shown(&to),
        rule,
        work_id: work_id.map(str::to_owned),
        bytes,
    }
}

/// Whether a file at the top of a named folder is what is finished: a
/// render, a master, or a picture called a cover.
fn is_finished(name: &Path) -> bool {
    let extension = name
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if FINISHED.contains(&extension.as_str()) {
        return true;
    }
    PICTURES.contains(&extension.as_str())
        && name
            .file_stem()
            .is_some_and(|stem| stem.to_string_lossy().to_lowercase().starts_with("cover"))
}

/// The folder a template always writes under: its steps before the first
/// that reads a placeholder. `songs` of `songs/{title}/video`; nothing for a
/// template that begins with a placeholder, which could write anywhere.
fn area_of(root: &Path, template: &str) -> Option<PathBuf> {
    let mut area = root.to_path_buf();
    let mut any = false;
    for step in template.split(['/', '\\']) {
        if step.contains('{') {
            break;
        }
        area.push(step.trim());
        any = true;
    }
    any.then_some(area)
}

/// Every file under `top`, sorted. Links are not followed and not listed:
/// what a link points at lies somewhere else and is moved, or not, there.
fn files_under(top: &Path, budget: &mut usize) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut folders = vec![top.to_path_buf()];
    while let Some(dir) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                folders.push(entry.path());
            } else if kind.is_file() {
                if *budget == 0 {
                    return Err(Error::refused("rehome.tooMany").param("path", shown(top)));
                }
                *budget -= 1;
                files.push(entry.path());
            }
        }
    }
    files.sort();
    Ok(files)
}

/// Carry a plan out: a copy of the database to `backup` first, then the
/// files, then the folders the moves emptied, then the templates, when
/// every move landed.
pub fn apply(
    conn: &Connection,
    profile_id: &str,
    target: &ProfileConfig,
    plan: &Plan,
    backup: &Path,
) -> Result<Applied> {
    super::backup::write_database(conn, backup)?;

    let mut moved = 0;
    let mut failed = Vec::new();
    let mut emptied: BTreeSet<PathBuf> = BTreeSet::new();
    for one in &plan.moves {
        let (from, to) = (Path::new(&one.from), Path::new(&one.to));
        match move_file(from, to) {
            Ok(()) => {
                moved += 1;
                if let Some(parent) = from.parent() {
                    emptied.insert(parent.to_path_buf());
                }
            }
            Err(cause) => failed.push(Left {
                path: one.from.clone(),
                reason: cause,
            }),
        }
    }

    // The folders the files left, deepest first, and those above them: a
    // walk up stops below the media folder, and below a folder that was
    // named to look into - neither is ever removed, nor anything above
    // them. Only a folder that holds nothing goes, by the call that refuses
    // any other.
    let bounds: Vec<&Path> = std::iter::once(Path::new(&plan.root))
        .chain(plan.also.iter().map(Path::new))
        .collect();
    let mut folders_removed = 0;
    let mut candidates: Vec<PathBuf> = emptied.into_iter().collect();
    candidates.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for start in candidates {
        let Some(bound) = bounds
            .iter()
            .filter(|bound| start.starts_with(bound) && start != **bound)
            .max_by_key(|bound| bound.components().count())
        else {
            continue;
        };
        let mut dir = start.as_path();
        while dir != *bound && std::fs::remove_dir(dir).is_ok() {
            folders_removed += 1;
            match dir.parent() {
                Some(parent) => dir = parent,
                None => break,
            }
        }
    }

    let mut templates_switched = false;
    if failed.is_empty() && !plan.templates.is_empty() {
        let mut config = crate::profile::config_for(conn, profile_id)?;
        for kind in &mut config.work_kinds {
            if let Some(change) = plan.templates.iter().find(|one| one.kind == kind.key) {
                kind.folder = Some(change.after.clone());
                // The rule for the finished file goes with the place it is
                // a path inside of, where the kind still named the shipped
                // place's rule or none.
                if kind.file.is_none() {
                    kind.file = target
                        .work_kinds
                        .iter()
                        .find(|one| one.key == kind.key)
                        .and_then(|one| one.file.clone());
                }
            }
        }
        crate::actions::profile::update_config(conn, profile_id, &config)?;
        templates_switched = true;
    }

    Ok(Applied {
        backup: shown(backup),
        moved,
        failed,
        folders_removed,
        templates_switched,
    })
}

/// Move one file, never onto another. Across drives - a folder in Downloads
/// to a media folder on another disk - it is copied, checked by its length,
/// and only then taken from where it was.
fn move_file(from: &Path, to: &Path) -> std::result::Result<(), String> {
    if to.exists() {
        return Err(format!("{} is already there", to.display()));
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|cause| cause.to_string())?;
    }
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    let mut part = to.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let copied = std::fs::copy(from, &part).map_err(|cause| cause.to_string())?;
    let meant = std::fs::metadata(from)
        .map_err(|cause| cause.to_string())?
        .len();
    if copied != meant {
        let _ = std::fs::remove_file(&part);
        return Err(format!("copied {copied} of {meant} bytes"));
    }
    std::fs::rename(&part, to).map_err(|cause| cause.to_string())?;
    std::fs::remove_file(from)
        .map_err(|cause| format!("copied, and the original could not be taken away: {cause}"))
}

/// Write a plan, and what applying it did when it was applied, into `dir`:
/// `migration_plan.json` whole, and `migration_plan.md` to read.
pub fn report(dir: &Path, plan: &Plan, applied: Option<&Applied>) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let whole = serde_json::json!({ "plan": plan, "applied": applied });
    std::fs::write(
        dir.join("migration_plan.json"),
        serde_json::to_string_pretty(&whole)?,
    )?;
    std::fs::write(dir.join("migration_plan.md"), page(plan, applied))?;
    Ok(())
}

/// The plan as a page.
fn page(plan: &Plan, applied: Option<&Applied>) -> String {
    use std::fmt::Write as _;
    let cell = |text: &str| text.replace('|', "\\|");
    let mut page = String::new();
    let _ = writeln!(page, "# Moving the folders of `{}`\n", plan.root);
    match applied {
        None => {
            let _ = writeln!(
                page,
                "A plan: nothing was moved and nothing was written but this report.\n"
            );
        }
        Some(done) => {
            let _ = writeln!(
                page,
                "Applied: {} files moved, {} failed, {} emptied folders removed, templates {}. \
                 The database as it was: `{}`.\n",
                done.moved,
                done.failed.len(),
                done.folders_removed,
                if done.templates_switched {
                    "switched"
                } else {
                    "not switched"
                },
                done.backup
            );
        }
    }

    let _ = writeln!(page, "## Files\n\n| Rule | Files |\n| --- | ---: |");
    for (rule, count) in plan.counts() {
        let _ = writeln!(page, "| {rule} | {count} |");
    }
    let bytes: u64 = plan.moves.iter().map(|one| one.bytes).sum();
    let _ = writeln!(
        page,
        "| all | {} |\n\n{} MB in all.\n",
        plan.moves.len(),
        bytes / 1_048_576
    );

    let _ = writeln!(
        page,
        "## Templates\n\n| Kind | Before | After |\n| --- | --- | --- |"
    );
    for change in &plan.templates {
        let _ = writeln!(
            page,
            "| {} | {} | {} |",
            change.kind,
            change
                .before
                .as_deref()
                .map_or("-".into(), |before| format!("`{}`", cell(before))),
            format_args!("`{}`", cell(&change.after))
        );
    }

    let _ = writeln!(
        page,
        "\n## Works\n\n| Work | Kind | From | To | Files | Finished file |\n| --- | --- | --- | --- | ---: | --- |"
    );
    for work in &plan.works {
        let file = match (&work.file, work.file_there) {
            (Some(file), Some(true)) => format!("`{}`", cell(file)),
            (Some(file), _) => format!("`{}` - **not there**", cell(file)),
            (None, _) => "-".into(),
        };
        let _ = writeln!(
            page,
            "| {} | {} | {} | `{}` | {} | {} |",
            cell(&work.title),
            work.kind,
            work.from
                .as_deref()
                .map_or("-".into(), |from| format!("`{}`", cell(from))),
            cell(&work.to),
            work.files,
            file
        );
    }

    for (title, rule) in [
        ("Belongs to no work", Rule::Unsorted),
        ("Would land on a file that is there", Rule::Conflict),
    ] {
        let these: Vec<&FileMove> = plan.moves.iter().filter(|one| one.rule == rule).collect();
        if these.is_empty() {
            continue;
        }
        let _ = writeln!(page, "\n## {title}\n\n| From | To |\n| --- | --- |");
        for one in these.iter().take(SHOWN) {
            let _ = writeln!(page, "| `{}` | `{}` |", cell(&one.from), cell(&one.to));
        }
        if these.len() > SHOWN {
            let _ = writeln!(
                page,
                "\n...and {} more, in `migration_plan.json`.",
                these.len() - SHOWN
            );
        }
    }

    if !plan.left.is_empty() {
        let _ = writeln!(
            page,
            "\n## Left where it is\n\n| Path | Why |\n| --- | --- |"
        );
        for one in &plan.left {
            let _ = writeln!(page, "| `{}` | {} |", cell(&one.path), cell(&one.reason));
        }
    }
    if let Some(done) = applied
        && !done.failed.is_empty()
    {
        let _ = writeln!(page, "\n## Failed\n\n| File | Why |\n| --- | --- |");
        for one in &done.failed {
            let _ = writeln!(page, "| `{}` | {} |", cell(&one.path), cell(&one.reason));
        }
    }
    page
}

/// How many lines of a long list the page writes out; the rest are in the
/// JSON beside it.
const SHOWN: usize = 300;

/// `kilna --rehome-media`: plan the move of a space's folders, write the
/// report, and carry it out when asked to. Answers with where the report is.
///
/// A plan unless `apply`: the default writes nothing but the report.
pub fn run(
    app_dir: &Path,
    space: Option<&str>,
    apply_it: bool,
    also: &[PathBuf],
) -> Result<PathBuf> {
    use crate::space::registry;
    let app = crate::db::open_app(&app_dir.join(crate::db::APP_DATABASE_FILE))?;
    let found = match space {
        Some(name) => registry::by_name(&app, app_dir, name)?,
        None => registry::newest(&app, app_dir)?.ok_or_else(|| Error::refused("space.none"))?,
    };
    let folder = registry::folder(&app, app_dir, &found.id)?;
    let (conn, held_by) = crate::state::open_space(&folder, crate::state::Role::Companion)?;
    if apply_it && held_by.is_some() {
        return Err(Error::refused("space.unreachable").param("path", found.path.clone()));
    }
    let profile =
        crate::profile::active(&conn)?.ok_or_else(|| Error::refused("profile.noneActive"))?;
    let target = crate::profile::builtin()?
        .into_iter()
        .find(|shipped| shipped.key == profile.key)
        .map_or_else(|| profile.config.clone(), |shipped| shipped.config);

    let moment = crate::time::now().replace([':', '.'], "-");
    let dir = app_dir.join("rehome").join(&moment);
    let planned = plan(&conn, &profile.id, &target, also)?;
    let applied = if apply_it {
        Some(apply(
            &conn,
            &profile.id,
            &target,
            &planned,
            &dir.join("kilna.before-rehome.db"),
        )?)
    } else {
        None
    };
    report(&dir, &planned, applied.as_ref())?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    /// A space as one made before v0.120 holds it: folders named by titles,
    /// a clip's inside its song's, and files in them.
    struct Old {
        conn: Connection,
        profile_id: String,
        root: tempfile::TempDir,
        song: Work,
        clip: Work,
        target: ProfileConfig,
    }

    fn old() -> Old {
        let (conn, profile_id) = fixtures::workspace();
        let root = tempfile::tempdir().unwrap();
        folder::set_root(
            &conn,
            &profile_id,
            Some(&root.path().to_string_lossy()),
            "2030-01-01T00:00:00.000Z",
        )
        .unwrap();
        let target = crate::profile::config_for(&conn, &profile_id).unwrap();
        // The templates as they shipped until v0.120.
        let mut stored = target.clone();
        for kind in &mut stored.work_kinds {
            kind.folder = match kind.key.as_str() {
                "song" => Some("songs/{origin.title}".into()),
                "video" => Some("songs/{origin.title}/video".into()),
                "audio" => Some("songs/{origin.title}/audio".into()),
                "short" => Some("songs/{origin.title}/shorts/{n}".into()),
                _ => None,
            };
        }
        conn.execute(
            "UPDATE profile SET config = ?2 WHERE id = ?1",
            rusqlite::params![profile_id, serde_json::to_string(&stored).unwrap()],
        )
        .unwrap();

        let song = fixtures::song(&conn, &profile_id, "Tide");
        let clip = crate::actions::work::derive(&conn, &song.id, "video", None, None, None)
            .unwrap()
            .work;
        Old {
            conn,
            profile_id,
            root,
            song,
            clip,
            target,
        }
    }

    fn write(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    fn read(path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }

    #[test]
    fn a_plan_moves_nothing_and_says_where_every_file_goes() {
        let old = old();
        let root = old.root.path();
        write(&root.join("songs/Tide/master.wav"), "song");
        write(&root.join("songs/Tide/drafts/v1.txt"), "draft");
        write(&root.join("songs/Tide/video/final.mp4"), "clip");
        write(&root.join("songs/Tide/video/scenes/take.mp4"), "take");
        write(&root.join("songs/Gone/old.wav"), "nobody's");
        write(&root.join("_shared/logo.png"), "shared");
        write(&root.join("beside.txt"), "beside");

        let plan = plan(&old.conn, &old.profile_id, &old.target, &[]).unwrap();

        let to = |from: &str| {
            plan.moves
                .iter()
                .find(|one| Path::new(&one.from) == root.join(from))
                .map(|one| (PathBuf::from(&one.to), one.rule))
        };
        let song = root.join("songs").join(&old.song.id);
        let clip = root.join("videos").join(&old.clip.id);
        assert_eq!(
            to("songs/Tide/master.wav"),
            Some((song.join("master.wav"), Rule::WorkFolder))
        );
        assert_eq!(
            to("songs/Tide/drafts/v1.txt"),
            Some((song.join("drafts").join("v1.txt"), Rule::WorkFolder)),
            "a file keeps its place inside the folder"
        );
        // The clip's folder lies inside the song's: its files are the clip's.
        assert_eq!(
            to("songs/Tide/video/final.mp4"),
            Some((clip.join("final.mp4"), Rule::WorkFolder))
        );
        assert_eq!(
            to("songs/Tide/video/scenes/take.mp4"),
            Some((clip.join("scenes").join("take.mp4"), Rule::WorkFolder))
        );
        // Under `songs/`, and no work's: kept, aside, by the path it had.
        assert_eq!(
            to("songs/Gone/old.wav"),
            Some((
                root.join(UNSORTED)
                    .join("songs")
                    .join("Gone")
                    .join("old.wav"),
                Rule::Unsorted
            ))
        );
        // What is shared, and what lies beside the areas, is not looked at.
        assert_eq!(to("_shared/logo.png"), None);
        assert_eq!(to("beside.txt"), None);
        assert_eq!(plan.moves.len(), 5);

        assert_eq!(
            plan.templates
                .iter()
                .map(|one| (one.kind.as_str(), one.after.as_str()))
                .collect::<Vec<_>>(),
            [
                ("song", "{kind}s/{id}"),
                ("video", "{kind}s/{id}"),
                ("audio", "{kind}s/{id}"),
                ("short", "{kind}s/{id}"),
                ("experiment", "{kind}s/{id}"),
            ]
        );
        // A plan is a plan: every file is where it was.
        assert_eq!(
            read(&root.join("songs/Tide/master.wav")).as_deref(),
            Some("song")
        );
        assert!(!root.join(UNSORTED).exists());
        assert!(!song.exists());
    }

    #[test]
    fn applying_moves_the_files_switches_the_templates_and_loses_nothing() {
        let old = old();
        let root = old.root.path();
        write(&root.join("songs/Tide/master.wav"), "song");
        write(&root.join("songs/Tide/video/final.mp4"), "clip");
        write(&root.join("songs/Gone/old.wav"), "nobody's");
        write(&root.join("_shared/logo.png"), "shared");
        // The clip's finished file was chosen by hand, inside its folder.
        crate::work::update(
            &old.conn,
            &old.clip.id,
            crate::work::WorkPatch {
                file: Some(Some("final.mp4".into())),
                ..Default::default()
            },
        )
        .unwrap();
        // A file is already where the song's master is going.
        let song = root.join("songs").join(&old.song.id);
        write(&song.join("master.wav"), "already here");

        let planned = plan(&old.conn, &old.profile_id, &old.target, &[]).unwrap();
        let clip_plan = planned
            .works
            .iter()
            .find(|work| work.work_id == old.clip.id)
            .unwrap();
        assert_eq!(clip_plan.file_there, Some(true), "it will be there after");

        let backup = old.root.path().join("report").join("before.db");
        let done = apply(&old.conn, &old.profile_id, &old.target, &planned, &backup).unwrap();

        assert_eq!((done.moved, done.failed.len()), (3, 0));
        assert!(done.templates_switched);
        assert!(backup.is_file(), "the database as it was is kept");
        let clip = root.join("videos").join(&old.clip.id);
        assert_eq!(read(&clip.join("final.mp4")).as_deref(), Some("clip"));
        // Not written over: what was there stays, what came goes aside.
        assert_eq!(
            read(&song.join("master.wav")).as_deref(),
            Some("already here")
        );
        assert_eq!(
            read(
                &root
                    .join(UNSORTED)
                    .join(CONFLICTS)
                    .join("songs")
                    .join(&old.song.id)
                    .join("master.wav")
            )
            .as_deref(),
            Some("song")
        );
        assert_eq!(
            read(&root.join(UNSORTED).join("songs/Gone/old.wav")).as_deref(),
            Some("nobody's")
        );
        assert_eq!(
            read(&root.join("_shared/logo.png")).as_deref(),
            Some("shared")
        );
        // The folders the files left are gone; the area itself stays.
        assert!(!root.join("songs/Tide").exists());
        assert!(!root.join("songs/Gone").exists());
        assert!(root.join("songs").is_dir());

        // kilna finds each work's folder and finished file where they are.
        let config = crate::profile::config_for(&old.conn, &old.profile_id).unwrap();
        let clip_now = crate::work::get(&old.conn, &old.clip.id).unwrap().unwrap();
        let found = folder::locate(&old.conn, &config, &clip_now).unwrap();
        assert_eq!(found.path.as_deref(), Some(clip.to_str().unwrap()));
        assert_eq!(found.file.relative.as_deref(), Some("final.mp4"));

        // Again, there is nothing left to do.
        let again = plan(&old.conn, &old.profile_id, &old.target, &[]).unwrap();
        assert_eq!((again.moves.len(), again.templates.len()), (0, 0));
    }

    #[test]
    fn a_named_folder_outside_is_taken_in_only_when_one_work_is_called_that() {
        let old = old();
        let root = old.root.path();
        let downloads = tempfile::tempdir().unwrap();
        let dir = downloads.path();
        // Called exactly what the clip is called.
        write(&dir.join(&old.clip.title).join("render.mp4"), "render");
        write(&dir.join(&old.clip.title).join("cover-final.png"), "cover");
        write(&dir.join(&old.clip.title).join("notes.txt"), "notes");
        write(&dir.join(&old.clip.title).join("takes/a.mp4"), "take");
        // Begins like it, and is not it; and somebody else's altogether.
        write(&dir.join("Tide (video) old").join("x.mp4"), "x");
        write(&dir.join("Invoices").join("march.pdf"), "pdf");
        write(&dir.join("loose.mp4"), "loose");

        let also = [dir.to_path_buf()];
        let planned = plan(&old.conn, &old.profile_id, &old.target, &also).unwrap();
        let clip = root.join("videos").join(&old.clip.id);
        let to = |name: &str| {
            planned
                .moves
                .iter()
                .find(|one| Path::new(&one.from) == dir.join(&old.clip.title).join(name))
                .map(|one| PathBuf::from(&one.to))
        };
        // What is finished, at the top; the rest under `raw/`.
        assert_eq!(to("render.mp4"), Some(clip.join("render.mp4")));
        assert_eq!(to("cover-final.png"), Some(clip.join("cover-final.png")));
        assert_eq!(to("notes.txt"), Some(clip.join("raw").join("notes.txt")));
        assert_eq!(
            to("takes/a.mp4"),
            Some(clip.join("raw").join("takes").join("a.mp4"))
        );
        assert_eq!(
            planned.moves.len(),
            4,
            "nothing else in the folder is touched"
        );
        assert_eq!(
            planned.left.len(),
            1,
            "the near miss is said: {:?}",
            planned.left
        );

        let backup = root.join("report").join("before.db");
        let done = apply(&old.conn, &old.profile_id, &old.target, &planned, &backup).unwrap();
        assert_eq!((done.moved, done.failed.len()), (4, 0));
        assert_eq!(read(&clip.join("render.mp4")).as_deref(), Some("render"));
        assert!(
            !dir.join(&old.clip.title).exists(),
            "the emptied folder is gone"
        );
        assert!(dir.is_dir(), "the folder that was looked into is not");
        assert_eq!(
            read(&dir.join("Invoices/march.pdf")).as_deref(),
            Some("pdf")
        );
        assert_eq!(read(&dir.join("loose.mp4")).as_deref(), Some("loose"));
        assert_eq!(
            read(&dir.join("Tide (video) old/x.mp4")).as_deref(),
            Some("x")
        );
    }

    #[test]
    fn a_folder_two_works_share_is_left_and_the_report_says_so() {
        let old = old();
        let root = old.root.path();
        // A second song with the same title resolves to the same folder.
        let twin = fixtures::song(&old.conn, &old.profile_id, "Tide");
        write(&root.join("songs/Tide/master.wav"), "whose?");

        let planned = plan(&old.conn, &old.profile_id, &old.target, &[]).unwrap();

        assert!(planned.moves.is_empty(), "{:?}", planned.moves);
        assert_eq!(planned.left.len(), 1);
        assert!(planned.left[0].reason.contains("2 works"));
        let dir = root.join("report");
        report(&dir, &planned, None).unwrap();
        let page = read(&dir.join("migration_plan.md")).unwrap();
        assert!(page.contains("Left where it is"));
        assert!(page.contains("nothing was moved"));
        assert!(dir.join("migration_plan.json").is_file());
        let _ = twin;
    }
}
