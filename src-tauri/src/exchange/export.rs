use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::error::Result;
use crate::note::{self, NoteFilter};
use crate::profile;
use crate::release;
use crate::score;
use crate::work::{self, WorkFilter, version};

/// What an export produced, so the user can be told rather than guess.
#[derive(Debug, Clone, Serialize)]
pub struct ExportReport {
    pub directory: String,
    pub works: usize,
    pub files: usize,
}

/// Write everything in the active profile as markdown.
///
/// This is the "you are not locked in" promise made concrete: one directory of
/// plain files, readable without kilna, with the structural facts in YAML front
/// matter and the bodies underneath.
pub fn to_markdown(conn: &Connection, directory: &Path) -> Result<ExportReport> {
    let profile =
        profile::active(conn)?.ok_or_else(|| crate::Error::Other("no active profile".into()))?;

    std::fs::create_dir_all(directory)?;
    let works_dir = directory.join("works");
    std::fs::create_dir_all(&works_dir)?;

    let works = work::list(conn, &profile.id, &WorkFilter::default())?;
    let mut files = 0;

    for work in &works {
        let mut page = String::new();

        page.push_str("---\n");
        push_field(&mut page, "title", &work.title);
        push_field(&mut page, "kind", &work.kind);
        push_field(&mut page, "status", &work.status);
        push_field(&mut page, "created", &work.created_at);
        push_field(&mut page, "updated", &work.updated_at);
        for (key, value) in &work.meta {
            // A JSON string carries its own quotes; taking them along would
            // export `"ru"` as `"\"ru\""`.
            let rendered = match value {
                serde_json::Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            push_field(&mut page, key, &rendered);
        }
        page.push_str("---\n\n");

        page.push_str(&format!("# {}\n", work.title));

        // Versions, newest first within each role, bodies in full.
        let versions = version::list(conn, &work.id)?;
        let mut current_role = String::new();
        for summary in &versions {
            if summary.role != current_role {
                current_role = summary.role.clone();
                page.push_str(&format!("\n## {current_role}\n"));
            }

            let Some(full) = version::get(conn, &summary.id)? else {
                continue;
            };
            page.push_str(&format!(
                "\n### Revision {}{}\n\n{}\n",
                full.revision,
                if summary.is_current { " (current)" } else { "" },
                full.body
            ));
        }

        let scores = score::history(conn, &work.id)?;
        if !scores.is_empty() {
            page.push_str("\n## Scores\n\n");
            page.push_str("| Date | Total | Tier | Axes |\n|---|---|---|---|\n");
            for score in &scores {
                let axes = score
                    .axes
                    .iter()
                    .map(|(key, value)| format!("{key} {value}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                page.push_str(&format!(
                    "| {} | {:.1} | {} | {} |\n",
                    &score.scored_at[..10.min(score.scored_at.len())],
                    score.total,
                    score.tier.as_deref().unwrap_or("—"),
                    axes
                ));
            }
        }

        let releases = release::for_work(conn, &work.id)?;
        if !releases.is_empty() {
            page.push_str("\n## Releases\n\n");
            for release in &releases {
                page.push_str(&format!(
                    "- **{}** — {}{}\n",
                    release.kind,
                    release
                        .released_at
                        .as_deref()
                        .or(release.scheduled_at.as_deref())
                        .unwrap_or("not scheduled"),
                    release
                        .url
                        .as_deref()
                        .map(|url| format!(" — {url}"))
                        .unwrap_or_default()
                ));
            }
        }

        let notes = note::list(
            conn,
            &profile.id,
            &NoteFilter {
                work_id: Some(work.id.clone()),
                ..Default::default()
            },
        )?;
        if !notes.is_empty() {
            page.push_str("\n## Notes\n\n");
            for note in &notes {
                page.push_str(&format!("- {}", note.body.replace('\n', "\n  ")));
                if !note.tags.is_empty() {
                    page.push_str(&format!(" _({})_", note.tags.join(", ")));
                }
                page.push('\n');
            }
        }

        let path = works_dir.join(format!("{}.md", slug(&work.title, &work.id)));
        std::fs::write(&path, page)?;
        files += 1;
    }

    // Loose notes — those not attached to a work — would otherwise be lost.
    let loose: Vec<_> = note::list(conn, &profile.id, &NoteFilter::default())?
        .into_iter()
        .filter(|note| note.work_id.is_none())
        .collect();
    if !loose.is_empty() {
        let mut page = String::from("# Notes\n\n");
        for note in &loose {
            if let Some(title) = &note.title {
                page.push_str(&format!("## {title}\n\n"));
            }
            page.push_str(&format!("{}\n", note.body));
            if !note.tags.is_empty() {
                page.push_str(&format!("\n_{}_\n", note.tags.join(", ")));
            }
            page.push('\n');
        }
        std::fs::write(directory.join("notes.md"), page)?;
        files += 1;
    }

    // The profile itself, so the vocabulary the export speaks in is legible.
    std::fs::write(
        directory.join("profile.json"),
        serde_json::to_string_pretty(&profile.config)?,
    )?;
    files += 1;

    Ok(ExportReport {
        directory: directory.display().to_string(),
        works: works.len(),
        files,
    })
}

/// A file name from a title: readable, unique, and safe on every platform.
///
/// Non-ASCII is kept — a Cyrillic title should stay readable — but characters
/// a filesystem rejects are not. The id suffix keeps two works with the same
/// title from overwriting each other.
fn slug(title: &str, id: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\n' | '\r' | '\t' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect();

    let trimmed = cleaned.trim().trim_matches('.');
    let short = trimmed.chars().take(60).collect::<String>();
    let head = id.split('-').next().unwrap_or(id);

    if short.is_empty() {
        format!("untitled-{head}")
    } else {
        format!("{}-{head}", short.trim())
    }
}

fn push_field(page: &mut String, key: &str, value: &str) {
    // Quote everything: a title with a colon is otherwise invalid YAML.
    page.push_str(&format!("{key}: \"{}\"\n", value.replace('"', "\\\"")));
}

/// Default directory to suggest for an export.
pub fn default_directory(documents: &Path) -> PathBuf {
    documents.join("kilna-export")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::note::NewNote;
    use crate::score::NewScore;
    use crate::work::NewWork;
    use crate::work::version::NewVersion;
    use serde_json::json;

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    #[test]
    fn an_export_writes_one_file_per_work_plus_the_profile() {
        let (conn, profile_id) = workspace();
        let dir = tempfile::tempdir().unwrap();
        work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "First".into(),
                ..NewWork::default()
            },
        )
        .unwrap();

        let report = to_markdown(&conn, dir.path()).unwrap();

        assert_eq!(report.works, 1);
        assert!(dir.path().join("profile.json").exists());
        let works: Vec<_> = std::fs::read_dir(dir.path().join("works"))
            .unwrap()
            .filter_map(std::result::Result::ok)
            .collect();
        assert_eq!(works.len(), 1);
    }

    #[test]
    fn a_page_carries_the_bodies_scores_and_notes() {
        let (mut conn, profile_id) = workspace();
        let dir = tempfile::tempdir().unwrap();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Harbour lights".into(),
                meta: json!({ "bpm": 96, "language": "English" })
                    .as_object()
                    .cloned(),
                ..NewWork::default()
            },
        )
        .unwrap();
        version::create(
            &mut conn,
            &work.id,
            NewVersion {
                role: "lyrics".into(),
                body: "the cranes go still".into(),
                label: None,
                meta: None,
                make_current: true,
            },
        )
        .unwrap();
        score::create(
            &conn,
            &work.id,
            NewScore {
                axes: json!({ "hook": 8 }).as_object().cloned().unwrap(),
                version_id: None,
                note: None,
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
                tags: vec!["idea".into()],
            },
        )
        .unwrap();

        to_markdown(&conn, dir.path()).unwrap();

        let page = std::fs::read_dir(dir.path().join("works"))
            .unwrap()
            .filter_map(std::result::Result::ok)
            .map(|entry| std::fs::read_to_string(entry.path()).unwrap())
            .next()
            .unwrap();

        assert!(page.contains("title: \"Harbour lights\""));
        assert!(page.contains("bpm: \"96\""));
        // A JSON string must not arrive carrying its own quotes.
        assert!(
            page.contains("language: \"English\""),
            "expected a plain value, got:\n{page}"
        );
        assert!(
            page.contains("the cranes go still"),
            "the body must be there in full"
        );
        assert!(page.contains("## Scores"));
        assert!(page.contains("a thought"));
        assert!(page.contains("idea"));
    }

    #[test]
    fn loose_notes_are_not_lost() {
        let (conn, profile_id) = workspace();
        let dir = tempfile::tempdir().unwrap();
        note::create(
            &conn,
            &profile_id,
            NewNote {
                body: "unattached".into(),
                kind: None,
                title: Some("Stray".into()),
                work_id: None,
                tags: vec![],
            },
        )
        .unwrap();

        to_markdown(&conn, dir.path()).unwrap();

        let notes = std::fs::read_to_string(dir.path().join("notes.md")).unwrap();
        assert!(notes.contains("unattached"));
    }

    #[test]
    fn a_title_that_is_not_a_filename_still_produces_one() {
        assert!(!slug("Who? / What: \"this\"", "abcdef12-0000").contains('/'));
        assert!(!slug("a\nb", "abcdef12-0000").contains('\n'));
        assert!(slug("", "abcdef12-0000").starts_with("untitled-"));
    }

    #[test]
    fn two_works_with_the_same_title_do_not_overwrite_each_other() {
        assert_ne!(slug("Same", "aaaaaaaa-0000"), slug("Same", "bbbbbbbb-0000"));
    }

    #[test]
    fn a_cyrillic_title_stays_readable() {
        let name = slug("Тёплые соты", "abcdef12-0000");

        assert!(name.starts_with("Тёплые соты-"), "got {name}");
    }
}
