//! The folders of songs a previous system kept, brought in as notes and
//! phrases of their works (v0.146, FR-10 10.1).
//!
//! A song was a folder - `<id>/info.md` beside `reviews/`, `shorts/`,
//! `audio-release/` and `clip/` - and its texts and styles came in long ago.
//! What did not is the rest: the sections of `info.md` (the song's mythology,
//! its images), the councils' reviews, the notes on its shorts, audio and
//! clip. Each folder is matched to a work of the space - by `source_id` in
//! the work's fields when one says the folder's name, else by the title
//! `info.md` opens with - and brings:
//!
//! - every section of `info.md` as a note on the work, named after its
//!   heading; the section named by `--images-heading`, when given, as fresh
//!   phrases that arose at the work instead - one for each item of its list,
//!   or for the first cell of each row of its table (v0.163.1). A table says
//!   more than its first column (what an image stands for), so a section
//!   written as one is also kept as its note;
//! - every page of `reviews/`, `shorts/`, `audio-release/` and `clip/` as a
//!   note on the work, named after its folder and file.
//!
//! A note the work already holds under the same name, and a phrase the space
//! already holds, are passed over. Nothing is written unless `--apply` is
//! said: the plan and its report come first; applied, a copy of the space is
//! kept beside the report and everything lands as one gesture.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::note::{NewNote, NoteFilter, NoteState};

/// The folders of a song's pages brought in as notes, in this order.
const PAGES: [&str; 4] = ["reviews", "shorts", "audio-release", "clip"];

/// What one song folder holds.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Folder {
    /// The folder's name: the old system's id.
    pub id: String,
    /// The title `info.md` opens with.
    pub title: Option<String>,
    /// The sections of `info.md`: heading and text.
    pub sections: Vec<(String, String)>,
    /// The other pages: a name and a text.
    pub pages: Vec<(String, String)>,
}

/// Read every song folder under `root`: those holding an `info.md`.
pub fn read(root: &Path) -> Result<Vec<Folder>> {
    let mut out = Vec::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(root)?
        .filter_map(|entry| entry.ok().map(|one| one.path()))
        .filter(|path| path.join("info.md").is_file())
        .collect();
    entries.sort();
    for dir in entries {
        let id = dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let info = std::fs::read_to_string(dir.join("info.md"))?;
        let (title, sections) = sections_of(&info);
        let mut pages = Vec::new();
        for folder in PAGES {
            let Ok(found) = std::fs::read_dir(dir.join(folder)) else {
                continue;
            };
            let mut files: Vec<PathBuf> = found
                .filter_map(|entry| entry.ok().map(|one| one.path()))
                .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
                .collect();
            files.sort();
            for file in files {
                let stem = file
                    .file_stem()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let text = std::fs::read_to_string(&file)?;
                if !text.trim().is_empty() {
                    pages.push((format!("{folder}: {stem}"), text.trim().to_owned()));
                }
            }
        }
        out.push(Folder {
            id,
            title,
            sections,
            pages,
        });
    }
    Ok(out)
}

/// The title a page opens with (`# ...`) and its `## ` sections.
pub fn sections_of(text: &str) -> (Option<String>, Vec<(String, String)>) {
    let mut title = None;
    let mut sections: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("# ") {
            if title.is_none() && sections.is_empty() {
                title = Some(heading.trim().to_owned());
            }
            continue;
        }
        if let Some(heading) = line.strip_prefix("## ") {
            sections.push((heading.trim().to_owned(), String::new()));
            continue;
        }
        if let Some((_, body)) = sections.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    let kept = sections
        .into_iter()
        .map(|(heading, body)| (heading, body.trim().to_owned()))
        .filter(|(_, body)| !body.is_empty())
        .collect();
    (title, kept)
}

/// What an import of song folders would do.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Plan {
    pub notes: Vec<PlannedNote>,
    pub phrases: Vec<PlannedPhrase>,
    pub phrase_kind: Option<String>,
    /// Notes and phrases passed over: held already.
    pub passed_over: usize,
    /// Folders no work of the space answers to.
    pub unmatched: Vec<String>,
}

/// A note to make on a work.
#[derive(Debug, Clone, Serialize)]
pub struct PlannedNote {
    pub work_id: String,
    pub work_title: String,
    pub title: String,
    pub body: String,
}

/// A phrase to make, arisen at a work.
#[derive(Debug, Clone, Serialize)]
pub struct PlannedPhrase {
    pub origin_work_id: String,
    pub body: String,
}

/// What bringing `folders` into this space would make.
pub fn plan(
    conn: &Connection,
    profile_id: &str,
    folders: &[Folder],
    images_heading: Option<&str>,
) -> Result<Plan> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let fold = |text: &str| {
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim_end_matches(['.', '!', '?', '…'])
            .to_lowercase()
    };
    let works = crate::work::list(conn, profile_id, &crate::work::WorkFilter::default())?;
    let by_source: BTreeMap<String, &crate::work::Work> = works
        .iter()
        .filter_map(|work| {
            work.meta
                .get("source_id")
                .and_then(serde_json::Value::as_str)
                .map(|id| (id.trim().to_owned(), work))
        })
        .collect();
    let by_title: BTreeMap<String, &crate::work::Work> =
        works.iter().map(|work| (fold(&work.title), work)).collect();
    // What the space holds already: notes by work and name, phrases by words.
    let mut held_notes: BTreeSet<(String, String)> = BTreeSet::new();
    for note in crate::note::list(conn, profile_id, &NoteFilter::default())? {
        if let (Some(work), Some(title)) = (&note.work_id, &note.title) {
            held_notes.insert((work.clone(), fold(title)));
        }
    }
    let phrase_kind = config.line_kinds().first().map(|kind| (*kind).to_owned());
    let mut held_phrases: BTreeSet<String> = BTreeSet::new();
    if let Some(kind) = &phrase_kind {
        let filter = NoteFilter {
            kind: Some(kind.clone()),
            ..NoteFilter::default()
        };
        for note in crate::note::list(conn, profile_id, &filter)? {
            held_phrases.insert(fold(&note.body));
        }
    }
    let images = images_heading.map(fold);

    let mut out = Plan {
        phrase_kind: phrase_kind.clone(),
        ..Plan::default()
    };
    for folder in folders {
        let found = by_source.get(&folder.id).copied().or_else(|| {
            folder
                .title
                .as_deref()
                .and_then(|title| by_title.get(&fold(title)).copied())
        });
        let Some(work) = found else {
            out.unmatched.push(match &folder.title {
                Some(title) => format!("{} ({title})", folder.id),
                None => folder.id.clone(),
            });
            continue;
        };
        let mut note = |title: &str, body: &str, out: &mut Plan| {
            if held_notes.insert((work.id.clone(), fold(title))) {
                out.notes.push(PlannedNote {
                    work_id: work.id.clone(),
                    work_title: work.title.clone(),
                    title: title.to_owned(),
                    body: body.to_owned(),
                });
            } else {
                out.passed_over += 1;
            }
        };
        for (heading, body) in &folder.sections {
            if images.as_deref() == Some(fold(heading).as_str()) && phrase_kind.is_some() {
                let (items, tabled) = image_items(body);
                for words in items {
                    if held_phrases.insert(fold(&words)) {
                        out.phrases.push(PlannedPhrase {
                            origin_work_id: work.id.clone(),
                            body: words,
                        });
                    } else {
                        out.passed_over += 1;
                    }
                }
                if tabled {
                    note(heading, body, &mut out);
                }
                continue;
            }
            note(heading, body, &mut out);
        }
        for (title, body) in &folder.pages {
            note(title, body, &mut out);
        }
    }
    Ok(out)
}

/// The images a section names: each item of its list, and the first cell of
/// each row of its table - not the header row, not the rule under it. The
/// flag says the section held a table, whose other columns are not phrases.
fn image_items(body: &str) -> (Vec<String>, bool) {
    let clean = |text: &str| text.replace("**", "").trim().to_owned();
    let lines: Vec<&str> = body.lines().map(str::trim).collect();
    let is_rule = |line: &str| {
        line.starts_with('|')
            && line.chars().all(|one| matches!(one, '|' | '-' | ':' | ' '))
            && line.contains('-')
    };
    let mut items = Vec::new();
    let mut tabled = false;
    for (at, line) in lines.iter().enumerate() {
        if let Some(item) = ["- ", "* ", "• "]
            .iter()
            .find_map(|marker| line.strip_prefix(marker))
        {
            let words = clean(item);
            if !words.is_empty() {
                items.push(words);
            }
            continue;
        }
        if !line.starts_with('|') || is_rule(line) {
            continue;
        }
        // A header is the row a rule follows.
        if lines.get(at + 1).is_some_and(|next| is_rule(next)) {
            continue;
        }
        tabled = true;
        let first = line
            .trim_matches('|')
            .split('|')
            .next()
            .map(clean)
            .unwrap_or_default();
        if !first.is_empty() {
            items.push(first);
        }
    }
    (items, tabled)
}

/// What an import made.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Applied {
    pub notes: usize,
    pub phrases: usize,
}

/// Make the notes and the phrases, as one gesture.
pub fn apply(conn: &Connection, plan: &Plan) -> Result<Applied> {
    crate::db::unit::atomically(conn, |tx| {
        let mut done = Applied::default();
        for note in &plan.notes {
            crate::actions::note::create(
                tx,
                NewNote {
                    body: note.body.clone(),
                    title: Some(note.title.clone()),
                    work_id: Some(note.work_id.clone()),
                    ..NewNote::default()
                },
            )?;
            done.notes += 1;
        }
        if let Some(kind) = &plan.phrase_kind {
            for phrase in &plan.phrases {
                crate::actions::note::create(
                    tx,
                    NewNote {
                        body: phrase.body.clone(),
                        kind: Some(kind.clone()),
                        state: Some(NoteState::Fresh),
                        origin_work_id: Some(phrase.origin_work_id.clone()),
                        ..NewNote::default()
                    },
                )?;
                done.phrases += 1;
            }
        }
        Ok(done)
    })
}

/// Read the folders under `root` against a space and write the plan and its
/// report; with `apply_it`, keep a copy of the space and apply. The folder
/// the report is written to comes back.
pub fn run(
    app_dir: &Path,
    space: Option<&str>,
    root: &Path,
    images_heading: Option<&str>,
    apply_it: bool,
) -> Result<PathBuf> {
    use crate::space::registry;
    let folders = read(root)?;
    let app = crate::db::open_app(&app_dir.join(crate::db::APP_DATABASE_FILE))?;
    let found = super::target_space(&app, app_dir, space, apply_it)?;
    let folder = registry::folder(&app, app_dir, &found.id)?;
    let (conn, held_by) = crate::state::open_space(&folder, crate::state::Role::Companion)?;
    if apply_it && held_by.is_some() {
        return Err(Error::refused("space.unreachable").param("path", found.path.clone()));
    }
    let profile_id = crate::actions::active_profile_id(&conn)?;
    let moment = crate::time::now().replace([':', '.'], "-");
    let dir = app_dir.join("import-songs").join(&moment);
    let planned = plan(&conn, &profile_id, &folders, images_heading)?;
    let applied = if apply_it {
        crate::exchange::backup::write_database(&conn, &dir.join("kilna.before-import.db"))?;
        Some(apply(&conn, &planned)?)
    } else {
        None
    };
    report(&dir, folders.len(), &planned, applied.as_ref())?;
    Ok(dir)
}

/// The plan as `import_plan.json` and a page, `import_plan.md`.
fn report(dir: &Path, folders: usize, plan: &Plan, applied: Option<&Applied>) -> Result<()> {
    use std::fmt::Write as _;
    std::fs::create_dir_all(dir)?;
    let whole = serde_json::json!({ "folders": folders, "plan": plan, "applied": applied });
    std::fs::write(
        dir.join("import_plan.json"),
        serde_json::to_string_pretty(&whole)?,
    )?;
    let mut page = String::new();
    let _ = writeln!(page, "# Bringing in the folders of songs\n");
    match applied {
        None => {
            let _ = writeln!(
                page,
                "A plan: nothing is written until it is run again with `--apply`.\n"
            );
        }
        Some(done) => {
            let _ = writeln!(
                page,
                "Applied: {} notes and {} phrases made. A copy of the space from before is `kilna.before-import.db` beside this page.\n",
                done.notes, done.phrases
            );
        }
    }
    let _ = writeln!(page, "- Folders read: {folders}");
    let _ = writeln!(page, "- Notes to make on their works: {}", plan.notes.len());
    let _ = writeln!(page, "- Phrases to make: {}", plan.phrases.len());
    let _ = writeln!(page, "- Passed over, held already: {}", plan.passed_over);
    let _ = writeln!(
        page,
        "- Folders no work answers to: {}\n",
        plan.unmatched.len()
    );
    if !plan.unmatched.is_empty() {
        let _ = writeln!(page, "## Folders not matched\n");
        for one in &plan.unmatched {
            let _ = writeln!(page, "- {one}");
        }
        let _ = writeln!(page);
    }
    if !plan.notes.is_empty() {
        let _ = writeln!(page, "## Notes\n");
        for note in &plan.notes {
            let _ = writeln!(page, "- {} · {}", note.work_title, note.title);
        }
    }
    std::fs::write(dir.join("import_plan.md"), page)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    #[test]
    fn a_song_folder_brings_its_sections_pages_and_images() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Orbit");
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("s0042");
        std::fs::create_dir_all(folder.join("reviews")).unwrap();
        std::fs::create_dir_all(folder.join("shorts")).unwrap();
        std::fs::write(
            folder.join("info.md"),
            "# Orbit\n\n## Mythology\nA waltz of two moons.\n\n## Images\n- a pendulum over the table\n- **a frozen beat**\n\n## Empty\n\n",
        )
        .unwrap();
        std::fs::write(
            folder.join("reviews").join("council-1.md"),
            "Keep the bell.",
        )
        .unwrap();
        std::fs::write(folder.join("shorts").join("hook.md"), "The drop at 0:40.").unwrap();
        std::fs::create_dir_all(root.path().join("s0099")).unwrap();
        std::fs::write(root.path().join("s0099").join("info.md"), "# Nowhere\n").unwrap();

        let folders = read(root.path()).unwrap();
        assert_eq!(folders.len(), 2);
        let planned = plan(&conn, &profile_id, &folders, Some("images")).unwrap();
        let titles: Vec<&str> = planned
            .notes
            .iter()
            .map(|note| note.title.as_str())
            .collect();
        assert_eq!(titles, ["Mythology", "reviews: council-1", "shorts: hook"]);
        assert_eq!(planned.phrases.len(), 2);
        assert_eq!(planned.unmatched, ["s0099 (Nowhere)"]);

        let done = apply(&conn, &planned).unwrap();
        assert_eq!((done.notes, done.phrases), (3, 2));
        let again = plan(&conn, &profile_id, &folders, Some("images")).unwrap();
        assert!(
            again.notes.is_empty() && again.phrases.is_empty(),
            "held now"
        );
        assert_eq!(again.passed_over, 5);
        let phrases = crate::note::list(
            &conn,
            &profile_id,
            &NoteFilter {
                origin_work_id: Some(song.id.clone()),
                ..NoteFilter::default()
            },
        )
        .unwrap();
        assert_eq!(phrases.len(), 2);
    }

    #[test]
    fn images_written_as_a_table_become_phrases_and_the_table_stays_a_note() {
        let (conn, profile_id) = fixtures::workspace();
        fixtures::song(&conn, &profile_id, "Until tomorrow");
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("s0033");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(
            folder.join("info.md"),
            "# Until tomorrow\n\n## Images\n\n| Image | Stands for |\n|-------|---------------|\n| Neon lights | City romance |\n| **A cafe before dawn** | Not wanting to part |\n| | nothing named |\n",
        )
        .unwrap();

        let folders = read(root.path()).unwrap();
        let planned = plan(&conn, &profile_id, &folders, Some("images")).unwrap();
        let bodies: Vec<&str> = planned
            .phrases
            .iter()
            .map(|phrase| phrase.body.as_str())
            .collect();
        assert_eq!(bodies, ["Neon lights", "A cafe before dawn"]);
        let titles: Vec<&str> = planned
            .notes
            .iter()
            .map(|note| note.title.as_str())
            .collect();
        assert_eq!(titles, ["Images"], "what the images stand for is kept");
        assert!(planned.notes[0].body.contains("City romance"));
    }
}
