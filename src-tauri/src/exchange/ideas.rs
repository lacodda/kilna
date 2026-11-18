//! A bank of phrases kept as one markdown file, brought in as phrases - and
//! long notes marked as documents (v0.145, FR-09 9.1 and 9.3).
//!
//! The bank is a page of sections, one a work: a heading that names the work
//! in quotes - `## From «Orbit» (draft)` - and the phrases that came while it
//! was written, a line each. Each becomes a phrase of the space, fresh, with
//! the work it arose at as its origin when the space has a work of that name;
//! a phrase the space already holds is passed over, and so is one the file
//! says twice. Sections without a quoted name are left alone: a general list
//! moved in before, notes on themes.
//!
//! Nothing is written unless `--apply` is said: the plan and its report come
//! first, and the report says what would be made, what is passed over and
//! which works the file names that the space does not have. Applied, a copy
//! of the space is kept beside the report first, and everything lands as one
//! gesture an undo takes back.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::note::{NewNote, NoteFilter, NotePatch, NoteState};
use crate::profile::config::ProfileConfig;

/// One section of the bank: the work it names and the phrases under it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Section {
    /// The work's name, the first quoted one in the heading.
    pub title: String,
    /// Other names the heading quotes - a working title.
    pub also: Vec<String>,
    pub phrases: Vec<Phrase>,
}

/// A phrase of the bank, with the tags a line may close on: `(longing, dream)`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Phrase {
    pub body: String,
    pub tags: Vec<String>,
}

/// The sections of a bank, in its order.
pub fn parse(text: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut open = false;
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(heading) = line.strip_prefix("## ") {
            let named = quoted(heading);
            open = !named.is_empty();
            if let Some((title, also)) = named.split_first() {
                sections.push(Section {
                    title: title.clone(),
                    also: also.to_vec(),
                    phrases: Vec::new(),
                });
            }
            continue;
        }
        if line.starts_with('#') {
            // A heading of a deeper level stays inside its section; one at
            // the top ends it.
            if line.starts_with("# ") {
                open = false;
            }
            continue;
        }
        if !open {
            continue;
        }
        let Some(item) = ["- ", "* ", "• "]
            .iter()
            .find_map(|marker| line.strip_prefix(marker))
        else {
            continue;
        };
        if let Some(phrase) = phrase_of(item)
            && let Some(section) = sections.last_mut()
        {
            section.phrases.push(phrase);
        }
    }
    sections
}

/// The names a heading quotes, in its order: «...», “...” or "...".
fn quoted(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(['«', '“', '"']) {
        let open = rest[start..].chars().next().unwrap_or('"');
        let close = match open {
            '«' => '»',
            '“' => '”',
            _ => '"',
        };
        let after = &rest[start + open.len_utf8()..];
        let Some(end) = after.find(close) else { break };
        let name = after[..end].trim();
        if !name.is_empty() {
            out.push(name.to_owned());
        }
        rest = &after[end + close.len_utf8()..];
    }
    out
}

/// A line of a list as a phrase: its emphasis taken off, and a closing
/// parenthesis of short words read as its tags.
fn phrase_of(item: &str) -> Option<Phrase> {
    let mut body = item.replace("**", "").trim().to_owned();
    let mut tags = Vec::new();
    if body.ends_with(')')
        && let Some(start) = body.rfind('(')
    {
        let inside = &body[start + 1..body.len() - 1];
        let words: Vec<String> = inside
            .split(',')
            .map(|one| one.trim().to_lowercase())
            .filter(|one| !one.is_empty())
            .collect();
        let short =
            !words.is_empty() && words.iter().all(|one| one.split_whitespace().count() <= 2);
        if short && start > 0 {
            tags = words;
            body = body[..start].trim_end().to_owned();
        }
    }
    (!body.is_empty()).then_some(Phrase { body, tags })
}

/// What an import would do.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Plan {
    /// The kind the phrases are made in.
    pub phrase_kind: String,
    pub phrases: Vec<Planned>,
    /// Phrases passed over: the space holds them, or the file said them twice.
    pub duplicates: Vec<String>,
    /// Works the file names that the space does not have: their phrases come
    /// in with no origin.
    pub unmatched: Vec<String>,
    /// The kind long notes are marked as, when the profile has one.
    pub document_kind: Option<String>,
    /// Notes to mark as documents.
    pub documents: Vec<Marked>,
}

/// A phrase to make.
#[derive(Debug, Clone, Serialize)]
pub struct Planned {
    pub body: String,
    pub tags: Vec<String>,
    /// The work it arose at, by id, when the space has it.
    pub origin_work_id: Option<String>,
    /// The work's name as the file gives it.
    pub origin_title: String,
}

/// A note to mark as a document.
#[derive(Debug, Clone, Serialize)]
pub struct Marked {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
}

/// What an import would make of `sections` in this space, and which notes
/// carrying any of `documents_tagged` it would mark as documents.
pub fn plan(
    conn: &Connection,
    profile_id: &str,
    config: &ProfileConfig,
    sections: &[Section],
    documents_tagged: &[String],
) -> Result<Plan> {
    let phrase_kind = config
        .line_kinds()
        .first()
        .map(|kind| (*kind).to_owned())
        .ok_or_else(|| Error::refused("import.noPhraseKind"))?;
    let fold = |text: &str| {
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim_end_matches(['.', '!', '?', '…'])
            .to_lowercase()
    };
    let mut held: BTreeSet<String> = BTreeSet::new();
    for kind in config.line_kinds() {
        let filter = NoteFilter {
            kind: Some(kind.to_owned()),
            ..NoteFilter::default()
        };
        for note in crate::note::list(conn, profile_id, &filter)? {
            held.insert(fold(&note.body));
        }
    }
    let works: BTreeMap<String, String> =
        crate::work::list(conn, profile_id, &crate::work::WorkFilter::default())?
            .into_iter()
            .map(|work| (fold(&work.title), work.id))
            .collect();

    let mut out = Plan {
        phrase_kind,
        ..Plan::default()
    };
    for section in sections {
        let origin = std::iter::once(&section.title)
            .chain(section.also.iter())
            .find_map(|name| works.get(&fold(name)).cloned());
        if origin.is_none() && !out.unmatched.contains(&section.title) {
            out.unmatched.push(section.title.clone());
        }
        for phrase in &section.phrases {
            if !held.insert(fold(&phrase.body)) {
                out.duplicates.push(phrase.body.clone());
                continue;
            }
            out.phrases.push(Planned {
                body: phrase.body.clone(),
                tags: phrase.tags.clone(),
                origin_work_id: origin.clone(),
                origin_title: section.title.clone(),
            });
        }
    }

    out.document_kind = config
        .note_kinds
        .iter()
        .find(|kind| kind.document)
        .map(|kind| kind.key.clone());
    if let Some(document) = &out.document_kind
        && !documents_tagged.is_empty()
    {
        let wanted: BTreeSet<String> = documents_tagged.iter().map(|tag| fold(tag)).collect();
        let plain = NoteFilter {
            canon: Some(false),
            line: Some(false),
            ..NoteFilter::default()
        };
        for note in crate::note::list(conn, profile_id, &plain)? {
            if note.kind == *document
                || config
                    .note_kind(&note.kind)
                    .is_some_and(|kind| kind.library)
            {
                continue;
            }
            if note.tags.iter().any(|tag| wanted.contains(&fold(tag))) {
                out.documents.push(Marked {
                    id: note.id,
                    title: note.title.unwrap_or_default(),
                    tags: note.tags,
                });
            }
        }
    }
    Ok(out)
}

/// What an import did.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Applied {
    pub phrases: usize,
    pub documents: usize,
}

/// Make the phrases and mark the documents, as one gesture: each phrase its
/// own `note.create`, each document its own `note.update`, in one unit.
pub fn apply(conn: &Connection, plan: &Plan) -> Result<Applied> {
    crate::db::unit::atomically(conn, |tx| {
        let mut done = Applied::default();
        for phrase in &plan.phrases {
            crate::actions::note::create(
                tx,
                NewNote {
                    body: phrase.body.clone(),
                    kind: Some(plan.phrase_kind.clone()),
                    tags: phrase.tags.clone(),
                    state: Some(NoteState::Fresh),
                    origin_work_id: phrase.origin_work_id.clone(),
                    ..NewNote::default()
                },
            )?;
            done.phrases += 1;
        }
        if let Some(document) = &plan.document_kind {
            for marked in &plan.documents {
                crate::actions::note::update(
                    tx,
                    &marked.id,
                    NotePatch {
                        kind: Some(document.clone()),
                        ..NotePatch::default()
                    },
                )?;
                done.documents += 1;
            }
        }
        Ok(done)
    })
}

/// Read `file` against a space - the last opened, or `space` - and write the
/// plan and its report; with `apply_it`, keep a copy of the space and apply.
/// The folder the report is written to comes back.
pub fn run(
    app_dir: &Path,
    space: Option<&str>,
    file: &Path,
    documents_tagged: &[String],
    apply_it: bool,
) -> Result<PathBuf> {
    use crate::space::registry;
    let text = std::fs::read_to_string(file)?;
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

    let moment = crate::time::now().replace([':', '.'], "-");
    let dir = app_dir.join("import-ideas").join(&moment);
    let planned = plan(
        &conn,
        &profile.id,
        &profile.config,
        &parse(&text),
        documents_tagged,
    )?;
    let applied = if apply_it {
        crate::exchange::backup::write_database(&conn, &dir.join("kilna.before-import.db"))?;
        Some(apply(&conn, &planned)?)
    } else {
        None
    };
    report(&dir, &planned, applied.as_ref())?;
    Ok(dir)
}

/// The plan as `import_plan.json` and a page, `import_plan.md`.
pub fn report(dir: &Path, plan: &Plan, applied: Option<&Applied>) -> Result<()> {
    use std::fmt::Write as _;
    std::fs::create_dir_all(dir)?;
    let whole = serde_json::json!({ "plan": plan, "applied": applied });
    std::fs::write(
        dir.join("import_plan.json"),
        serde_json::to_string_pretty(&whole)?,
    )?;
    let mut page = String::new();
    let _ = writeln!(page, "# Bringing in a bank of phrases\n");
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
                "Applied: {} phrases made, {} notes marked as documents. A copy of the space from before is `kilna.before-import.db` beside this page.\n",
                done.phrases, done.documents
            );
        }
    }
    let with_origin = plan
        .phrases
        .iter()
        .filter(|phrase| phrase.origin_work_id.is_some())
        .count();
    let _ = writeln!(
        page,
        "- Phrases to make, as `{}`: {} ({} with the work they arose at)",
        plan.phrase_kind,
        plan.phrases.len(),
        with_origin
    );
    let _ = writeln!(
        page,
        "- Passed over, already held or said twice: {}",
        plan.duplicates.len()
    );
    let _ = writeln!(
        page,
        "- Works the file names that the space does not have: {}",
        plan.unmatched.len()
    );
    let _ = writeln!(
        page,
        "- Notes to mark as documents: {}\n",
        plan.documents.len()
    );
    if !plan.unmatched.is_empty() {
        let _ = writeln!(page, "## Works not found\n");
        for title in &plan.unmatched {
            let _ = writeln!(page, "- {title}");
        }
        let _ = writeln!(page);
    }
    if !plan.documents.is_empty() {
        let _ = writeln!(page, "## Notes to mark as documents\n");
        for marked in &plan.documents {
            let _ = writeln!(page, "- {} ({})", marked.title, marked.tags.join(", "));
        }
        let _ = writeln!(page);
    }
    if !plan.duplicates.is_empty() {
        let _ = writeln!(page, "## Passed over\n");
        for body in &plan.duplicates {
            let _ = writeln!(page, "- {body}");
        }
    }
    std::fs::write(dir.join("import_plan.md"), page)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    const BANK: &str = "# Bank\n\n## Phrases\n- moved in before\n\n## From «Orbit» (2026-05-30, draft)\n\n- a pendulum over the table\n- **a waltz on broken glass**\n- the half-life of feelings (longing, loss)\n\n### Images\n- a frozen beat\n\n## From «Harbour» (working title «Concrete», done)\n- salt on the railing\n- a pendulum over the table\n\n## From «Nowhere»\n- a lost line\n\n## Themes\n- not a phrase of a work\n";

    #[test]
    fn a_bank_is_read_section_by_section_by_the_work_it_names() {
        let sections = parse(BANK);
        assert_eq!(
            sections
                .iter()
                .map(|one| one.title.as_str())
                .collect::<Vec<_>>(),
            ["Orbit", "Harbour", "Nowhere"]
        );
        assert_eq!(sections[1].also, ["Concrete"]);
        let orbit: Vec<&str> = sections[0]
            .phrases
            .iter()
            .map(|one| one.body.as_str())
            .collect();
        assert_eq!(
            orbit,
            [
                "a pendulum over the table",
                "a waltz on broken glass",
                "the half-life of feelings",
                "a frozen beat"
            ]
        );
        assert_eq!(sections[0].phrases[2].tags, ["longing", "loss"]);
    }

    #[test]
    fn the_plan_finds_the_works_passes_over_what_is_held_and_marks_documents() {
        let (conn, profile_id) = fixtures::workspace();
        let orbit = fixtures::song(&conn, &profile_id, "Orbit");
        fixtures::song(&conn, &profile_id, "Concrete");
        crate::actions::note::create(
            &conn,
            NewNote {
                body: "Salt on the railing.".into(),
                kind: Some("phrase".into()),
                ..NewNote::default()
            },
        )
        .unwrap();
        let long = crate::actions::note::create(
            &conn,
            NewNote {
                body: "A long page.".into(),
                title: Some("Strategy".into()),
                tags: vec!["Strategy".into()],
                ..NewNote::default()
            },
        )
        .unwrap();
        let profile = crate::profile::active(&conn).unwrap().unwrap();

        let planned = plan(
            &conn,
            &profile_id,
            &profile.config,
            &parse(BANK),
            &["strategy".into()],
        )
        .unwrap();
        assert_eq!(planned.phrases.len(), 5);
        assert_eq!(planned.duplicates.len(), 2, "held, and said twice");
        assert_eq!(planned.unmatched, ["Nowhere"]);
        assert!(
            planned.phrases[..4]
                .iter()
                .all(|one| one.origin_work_id.as_deref() == Some(orbit.id.as_str()))
        );
        assert_eq!(planned.documents.len(), 1);

        let done = apply(&conn, &planned).unwrap();
        assert_eq!((done.phrases, done.documents), (5, 1));
        assert_eq!(
            crate::note::get(&conn, &long.id).unwrap().unwrap().kind,
            planned.document_kind.clone().unwrap()
        );
        let offer = crate::undo::last(&conn)
            .unwrap()
            .expect("one undo for the import");
        assert_eq!(
            offer.also, 5,
            "every phrase and the document go back together"
        );
    }
}
