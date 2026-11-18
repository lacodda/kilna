//! Obsidian's `[[links]]` in the bodies of notes, turned into kilna's own
//! (v0.145, FR-09 9.2).
//!
//! A note moved in from a vault says `[[Studio/Night drive]]`,
//! `[[Night drive]]` or `[[Night drive|the drive]]`. Each is looked for among
//! the notes, the works and the cards of the canon - by the name as written,
//! then by its last part without `.md`, then by that part with its quotes
//! and stray marks taken off - and a link found becomes a markdown link
//! `[the drive](kilna://note/<id>)`, `kilna://work/<id>` or
//! `kilna://canon/<id>`, which the window follows. A link not found stays as
//! it was written and is listed in the report, to be sorted by hand. kilna's
//! own `[[work:<id>]]` links are left alone.
//!
//! Asked to, it only reports; applied, every note it changes is one
//! `note.update` of one gesture, taken back by one undo.

use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::Serialize;

use crate::error::Result;
use crate::note::{NoteFilter, NotePatch};

/// A link the converter could not find a home for.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Unresolved {
    pub note_id: String,
    pub note_title: Option<String>,
    /// The link as written between the brackets.
    pub target: String,
}

/// What a conversion found, and did when it was applied.
#[derive(Debug, Clone, Default, PartialEq, Serialize, ts_rs::TS)]
pub struct WikiReport {
    /// Notes whose bodies change.
    pub notes: usize,
    /// Links that become kilna's.
    pub resolved: usize,
    pub unresolved: Vec<Unresolved>,
    /// Whether the changes were written.
    pub applied: bool,
}

/// Where a name leads: the scheme's word and the id.
type Home = (&'static str, String);

/// Look for every `[[link]]` in the notes of the profile; with `apply`, write
/// the links found as kilna's.
pub fn convert(conn: &Connection, profile_id: &str, apply: bool) -> Result<WikiReport> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let notes = crate::note::list(conn, profile_id, &NoteFilter::default())?;
    // The places a name can lead, the first claim winning: documents and
    // notes, then works, then the cards of the canon.
    let mut homes: BTreeMap<String, Home> = BTreeMap::new();
    let mut claim = |name: &str, home: &Home| {
        let key = fold(name);
        if !key.is_empty() {
            homes.entry(key).or_insert_with(|| home.clone());
        }
    };
    for note in notes
        .iter()
        .filter(|note| config.card_kind(&note.kind).is_none())
    {
        if let Some(title) = &note.title {
            claim(title, &("note", note.id.clone()));
        }
    }
    for work in crate::work::list(conn, profile_id, &crate::work::WorkFilter::default())? {
        claim(&work.title, &("work", work.id));
    }
    for card in notes
        .iter()
        .filter(|note| config.card_kind(&note.kind).is_some())
    {
        let home: Home = ("canon", card.id.clone());
        for name in card.title.iter().chain(card.aliases.iter()) {
            claim(name, &home);
        }
    }
    let find = |target: &str| -> Option<&Home> {
        let whole = target.trim();
        let last = whole.rsplit('/').next().unwrap_or(whole);
        let last = last.strip_suffix(".md").unwrap_or(last);
        homes
            .get(&fold(whole))
            .or_else(|| homes.get(&fold(last)))
            .or_else(|| homes.get(&fold(&bare(last))))
    };

    let mut report = WikiReport::default();
    let mut changes: Vec<(String, String)> = Vec::new();
    for note in &notes {
        if !note.body.contains("[[") {
            continue;
        }
        let mut changed = false;
        let body = replace_links(&note.body, |inside| {
            let (target, label) = match inside.split_once('|') {
                Some((target, label)) => (target.trim(), Some(label.trim())),
                None => (inside.trim(), None),
            };
            if is_own(target) {
                return None;
            }
            match find(target) {
                Some((scheme, id)) => {
                    changed = true;
                    report.resolved += 1;
                    let label = label
                        .filter(|said| !said.is_empty())
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            let last = target.rsplit('/').next().unwrap_or(target);
                            last.strip_suffix(".md").unwrap_or(last).to_owned()
                        })
                        .replace(['[', ']'], "");
                    Some(format!("[{label}](kilna://{scheme}/{id})"))
                }
                None => {
                    report.unresolved.push(Unresolved {
                        note_id: note.id.clone(),
                        note_title: note.title.clone(),
                        target: target.to_owned(),
                    });
                    None
                }
            }
        });
        if changed {
            changes.push((note.id.clone(), body));
        }
    }
    report.notes = changes.len();
    if apply && !changes.is_empty() {
        crate::db::unit::atomically(conn, |tx| {
            for (id, body) in &changes {
                crate::actions::note::update(
                    tx,
                    id,
                    NotePatch {
                        body: Some(body.clone()),
                        ..NotePatch::default()
                    },
                )?;
            }
            Ok(())
        })?;
        report.applied = true;
    }
    Ok(report)
}

/// The text with every `[[...]]` handed to `draw` - what is between the
/// brackets, on one line, with no bracket inside - and replaced by what it
/// answers; `None` leaves the link as it was written.
fn replace_links(text: &str, mut draw: impl FnMut(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("]]") else { break };
        let inside = &after[..end];
        out.push_str(&rest[..start]);
        if inside.is_empty() || inside.contains(['[', ']', '\n']) {
            // Not a link: keep the opening brackets and look on after them.
            out.push_str("[[");
            rest = after;
            continue;
        }
        match draw(inside) {
            Some(drawn) => out.push_str(&drawn),
            None => {
                out.push_str("[[");
                out.push_str(inside);
                out.push_str("]]");
            }
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

/// Whether a link is kilna's own: `work:<id>`, `version:<id>`.
fn is_own(target: &str) -> bool {
    target.split_once(':').is_some_and(|(kind, id)| {
        !kind.is_empty()
            && kind.chars().all(|c| c.is_ascii_lowercase())
            && !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    })
}

/// A name as it is compared: spaces collapsed, in lower case.
fn fold(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// A name with its quotes and stray marks taken off: `«Night drive»!` is
/// `Night drive`.
fn bare(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::note::NewNote;

    #[test]
    fn links_are_found_by_name_by_their_last_part_and_without_quotes() {
        let (conn, profile_id) = fixtures::workspace();
        let drive = crate::actions::note::create(
            &conn,
            NewNote {
                body: "A plan.".into(),
                title: Some("Night drive".into()),
                ..NewNote::default()
            },
        )
        .unwrap();
        let song = fixtures::song(&conn, &profile_id, "Orbit");
        let wren = fixtures::card(&conn, &profile_id, "character", "Wren");
        let page = crate::actions::note::create(
            &conn,
            NewNote {
                body: "See [[Studio/Night drive.md]], [[«Orbit»|the waltz]], [[wren]], \
                       [[Nowhere]] and [[work:abc]]."
                    .into(),
                title: Some("Index".into()),
                ..NewNote::default()
            },
        )
        .unwrap();

        let looked = convert(&conn, &profile_id, false).unwrap();
        assert_eq!(
            (looked.notes, looked.resolved, looked.applied),
            (1, 3, false)
        );
        assert_eq!(looked.unresolved.len(), 1);
        assert_eq!(looked.unresolved[0].target, "Nowhere");
        assert!(
            crate::note::get(&conn, &page.id)
                .unwrap()
                .unwrap()
                .body
                .contains("[[Studio/")
        );

        let done = convert(&conn, &profile_id, true).unwrap();
        assert!(done.applied);
        let body = crate::note::get(&conn, &page.id).unwrap().unwrap().body;
        assert_eq!(
            body,
            format!(
                "See [Night drive](kilna://note/{}), [the waltz](kilna://work/{}), \
                 [wren](kilna://canon/{}), [[Nowhere]] and [[work:abc]].",
                drive.id, song.id, wren.id
            )
        );
        assert!(
            crate::undo::last(&conn).unwrap().is_some(),
            "one undo takes it back"
        );
    }
}
