//! What a release says about itself: the title it goes out under, the text
//! under it, the words it is found by, the comment pinned beneath it.
//!
//! The fields are the craft's, named by the release kind in the profile
//! ([`ReleaseField`]); the values live in `release.meta` under the field keys.
//! There is no table here and no migration, deliberately: `release.meta` is
//! already a map on the row, already patched through `release::update`, which
//! the operation log and undo already know how to replay and reverse. A
//! second table would have been a second truth about one release, and a
//! second thing to keep in step with it.
//!
//! Generating is rendering a template against the work, with the same
//! renderer an AI action uses -- see [`crate::assistant::prompt`]. A release
//! title that reads `{title}` and an action that reads `{title}` are reading
//! the same word, because there is one vocabulary and one renderer, not two
//! that look alike.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::assistant::prompt;
use crate::error::{Error, Result};
use crate::profile::config::{ReleaseField, ReleaseFieldType};
use crate::release;
use crate::work;

/// One field of a release, as a screen needs it: what it is, what is written
/// in it, and whether the profile could write it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: ReleaseFieldType,
    /// What is stored on the release now. Empty is empty -- a field never
    /// written and a field written blank are the same thing to a reader, and
    /// telling them apart would only let one of them hide.
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Whether the profile can fill this field on its own.
    pub has_template: bool,
}

/// Every field of a release, in the order the profile lists them.
///
/// A release whose kind the profile no longer has gets an empty list rather
/// than an error: the release is still a real row with a real date, and the
/// tab around it must still draw.
pub fn fields(conn: &Connection, release_id: &str) -> Result<Vec<Field>> {
    let (release, defined) = definition(conn, release_id)?;
    Ok(defined
        .iter()
        .map(|field| Field {
            key: field.key.clone(),
            label: field.label.clone(),
            field_type: field.field_type,
            value: stored(&release.meta, &field.key),
            hint: field.hint.clone(),
            limit: field.limit,
            has_template: field.template().is_some(),
        })
        .collect())
}

/// The release, and the fields its kind declares.
fn definition(
    conn: &Connection,
    release_id: &str,
) -> Result<(release::Release, Vec<ReleaseField>)> {
    let release =
        release::get(conn, release_id)?.ok_or_else(|| Error::not_found("release", release_id))?;
    let work = work::get(conn, &release.work_id)?
        .ok_or_else(|| Error::not_found("work", &release.work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let defined = config
        .vocabulary(&work.kind)
        .release_kinds
        .iter()
        .find(|kind| kind.key == release.kind)
        .map(|kind| kind.fields.clone())
        .unwrap_or_default();
    Ok((release, defined))
}

/// What the release holds under a key, as text.
///
/// The map is JSON, so a value may have been written as a number or a list by
/// something other than the screen -- by an agent's package, by a hand-edited
/// export. Each reads back as the text it obviously is rather than as
/// `["a","b"]`: the field is text on every platform it is going to, and a
/// reader who sees JSON in a description box learns nothing useful from it.
fn stored(meta: &Map<String, Value>, key: &str) -> String {
    match meta.get(key) {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join(", "),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

/// What a generation produced, field by field.
#[derive(Debug, Clone, Serialize)]
pub struct Generated {
    /// The values, by field key, ready to be written.
    pub values: Map<String, Value>,
    /// Fields that could not be rendered, and why: a role the work has no
    /// version in yet, most often. Reported rather than silently skipped --
    /// "the description is empty" and "the description could not be written
    /// because there are no lyrics" are different problems, and only one of
    /// them is fixed by typing in the box.
    pub refused: Vec<Refusal>,
}

/// A field the profile could not fill, and the reason in the renderer's own
/// words.
#[derive(Debug, Clone, Serialize)]
pub struct Refusal {
    pub key: String,
    pub label: String,
    pub reason: String,
}

/// Render every templated field of a release against its work.
///
/// Fields without a template are left out entirely rather than rendered
/// empty: they are the ones only ever typed by hand, and overwriting what
/// someone typed with nothing is the one thing a generate button must never
/// do.
///
/// One field failing does not take the rest with it. A video whose lyrics are
/// not written yet should still get its title and its tags, and be told what
/// the description is waiting for.
pub fn generate(conn: &Connection, release_id: &str) -> Result<Generated> {
    let (release, defined) = definition(conn, release_id)?;
    let mut values = Map::new();
    let mut refused = Vec::new();

    for field in &defined {
        let Some(template) = field.template() else {
            continue;
        };
        match prompt::for_work(conn, &release.work_id, template, prompt::Context::default()) {
            Ok(rendered) => {
                values.insert(field.key.clone(), Value::String(tidy(&rendered, field)));
            }
            Err(cause) => refused.push(Refusal {
                key: field.key.clone(),
                label: field.label.clone(),
                reason: cause.to_string(),
            }),
        }
    }

    Ok(Generated { values, refused })
}

/// The rendered text as the field's shape wants it.
///
/// A line is a line: a template that reads `{title}` and a work whose title
/// someone pasted a newline into would otherwise put a second line in a
/// one-line box, where it is invisible until the platform rejects it. Tags
/// lose their blanks and their repeats, keeping the first spelling of each,
/// because a list holding `lighthouse, Lighthouse, lighthouse` is a list
/// someone will have to clean by hand.
fn tidy(rendered: &str, field: &ReleaseField) -> String {
    match field.field_type {
        ReleaseFieldType::Line => rendered.split_whitespace().collect::<Vec<_>>().join(" "),
        ReleaseFieldType::Text => rendered.trim().to_owned(),
        ReleaseFieldType::Tags => {
            let mut kept: Vec<String> = Vec::new();
            for tag in rendered.split(',') {
                let tag = tag.trim();
                if tag.is_empty() {
                    continue;
                }
                if kept.iter().any(|seen| seen.eq_ignore_ascii_case(tag)) {
                    continue;
                }
                kept.push(tag.to_owned());
            }
            kept.join(", ")
        }
    }
}

/// The values a release would be patched with, its own meta kept underneath.
///
/// Generation replaces the fields it produced and touches nothing else: a
/// release may carry meta no field of the profile describes -- written by an
/// older profile, by an agent, by hand -- and a generate button is not a
/// reason to lose it.
pub fn merged(existing: &Map<String, Value>, values: &Map<String, Value>) -> Map<String, Value> {
    let mut merged = existing.clone();
    for (key, value) in values {
        merged.insert(key.clone(), value.clone());
    }
    merged
}

/// How full a release's metadata is, for a screen that has to say whether
/// this one is ready to go out.
///
/// Counted over the fields the kind declares, so a kind that declares none is
/// complete rather than empty: nothing was asked of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Fullness {
    pub written: usize,
    pub total: usize,
}

impl Fullness {
    pub fn complete(self) -> bool {
        self.written == self.total
    }
}

/// What of a release's metadata is written.
pub fn fullness(fields: &[Field]) -> Fullness {
    Fullness {
        written: fields
            .iter()
            .filter(|field| !field.value.trim().is_empty())
            .count(),
        total: fields.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::release::NewRelease;
    use crate::work::{self, NewWork, version};
    use serde_json::json;

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    /// A song with an audio release planned for it, and the lyrics its
    /// description template reads.
    fn song_with_audio(conn: &mut Connection, profile_id: &str, lyrics: Option<&str>) -> String {
        let work = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap();

        if let Some(lyrics) = lyrics {
            version::create(
                conn,
                &work.id,
                version::NewVersion {
                    role: "lyrics".into(),
                    body: lyrics.into(),
                    label: None,
                    meta: None,
                    make_current: true,
                    parent_version_id: None,
                },
            )
            .unwrap();
        }

        release::create(
            conn,
            NewRelease {
                work_id: work.id,
                kind: "audio".into(),
                title: Some("Harbour lights".into()),
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn the_fields_are_the_release_kinds_own() {
        let (mut conn, profile_id) = workspace();
        let id = song_with_audio(&mut conn, &profile_id, None);

        let fields = fields(&conn, &id).unwrap();

        let keys: Vec<&str> = fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(
            keys,
            vec!["title", "description", "tags"],
            "an audio release is asked for what it goes out as, in the profile's order"
        );
    }

    #[test]
    fn a_release_of_a_kind_the_profile_lost_still_reads() {
        let (mut conn, profile_id) = workspace();
        let id = song_with_audio(&mut conn, &profile_id, None);
        release::update(
            &conn,
            &id,
            crate::release::ReleasePatch {
                kind: Some("a kind nobody ships".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let fields = fields(&conn, &id).unwrap();

        assert!(
            fields.is_empty(),
            "the release is still a real row with a real date, and the tab around it must draw"
        );
    }

    #[test]
    fn generating_fills_the_templated_fields_and_leaves_the_rest_alone() {
        let (mut conn, profile_id) = workspace();
        let id = song_with_audio(&mut conn, &profile_id, Some("the lamps come on at four"));

        let generated = generate(&conn, &id).unwrap();

        assert_eq!(
            generated.values.get("title").and_then(|v| v.as_str()),
            Some("Harbour lights")
        );
        assert_eq!(
            generated.values.get("description").and_then(|v| v.as_str()),
            Some("the lamps come on at four")
        );
        assert!(
            !generated.values.contains_key("tags"),
            "a field with no template is never written by generating: \
             overwriting what someone typed with nothing is the one thing it must not do"
        );
        assert!(generated.refused.is_empty());
    }

    #[test]
    fn a_field_waiting_on_a_missing_role_is_named_rather_than_written_blank() {
        let (mut conn, profile_id) = workspace();
        // No lyrics: the description template reads `{role:lyrics}`.
        let id = song_with_audio(&mut conn, &profile_id, None);

        let generated = generate(&conn, &id).unwrap();

        assert_eq!(
            generated.values.get("title").and_then(|v| v.as_str()),
            Some("Harbour lights"),
            "one field failing must not take the rest with it"
        );
        assert!(!generated.values.contains_key("description"));
        let refused: Vec<&str> = generated
            .refused
            .iter()
            .map(|refusal| refusal.key.as_str())
            .collect();
        assert_eq!(refused, vec!["description"]);
        assert!(
            generated.refused[0].reason.contains("Harbour lights"),
            "the reason says which work is waiting on what, not just that something failed: {}",
            generated.refused[0].reason
        );
    }

    #[test]
    fn generating_keeps_meta_no_field_describes() {
        let (mut conn, profile_id) = workspace();
        let id = song_with_audio(&mut conn, &profile_id, Some("a body"));
        // What a plugin left behind: `release.meta` is open, and a generate
        // button is not a reason to lose it.
        release::update(
            &conn,
            &id,
            crate::release::ReleasePatch {
                meta: Some(
                    json!({ "uploaded_by": "a plugin", "title": "typed by hand" })
                        .as_object()
                        .cloned()
                        .unwrap(),
                ),
                ..Default::default()
            },
        )
        .unwrap();

        let before = release::get(&conn, &id).unwrap().unwrap();
        let generated = generate(&conn, &id).unwrap();
        let merged = merged(&before.meta, &generated.values);

        assert_eq!(
            merged.get("uploaded_by").and_then(|v| v.as_str()),
            Some("a plugin"),
            "meta the profile does not describe survives a generation"
        );
        assert_eq!(
            merged.get("title").and_then(|v| v.as_str()),
            Some("Harbour lights"),
            "a field the profile does describe is replaced by what it makes"
        );
    }

    #[test]
    fn a_line_stays_one_line_however_the_title_was_typed() {
        let field = ReleaseField::new("title", "Title", ReleaseFieldType::Line);

        assert_eq!(
            tidy("Harbour\n  lights  ", &field),
            "Harbour lights",
            "a newline pasted into a title is invisible in a one-line box until the platform rejects it"
        );
    }

    #[test]
    fn tags_lose_their_blanks_and_their_repeats() {
        let field = ReleaseField::new("tags", "Tags", ReleaseFieldType::Tags);

        assert_eq!(
            tidy("lighthouse, , Lighthouse,  sea , lighthouse", &field),
            "lighthouse, sea",
            "the first spelling of each is kept; a list to clean by hand is not a list"
        );
    }

    #[test]
    fn a_stored_list_reads_back_as_the_text_it_obviously_is() {
        let meta = json!({ "tags": ["sea", "night"], "count": 3 })
            .as_object()
            .cloned()
            .unwrap();

        assert_eq!(stored(&meta, "tags"), "sea, night");
        assert_eq!(stored(&meta, "count"), "3");
        assert_eq!(stored(&meta, "nothing"), "");
    }

    #[test]
    fn fullness_counts_what_is_written() {
        let (mut conn, profile_id) = workspace();
        let id = song_with_audio(&mut conn, &profile_id, Some("a body"));

        let empty = fullness(&fields(&conn, &id).unwrap());
        assert_eq!(
            empty,
            Fullness {
                written: 0,
                total: 3
            }
        );
        assert!(!empty.complete());

        let generated = generate(&conn, &id).unwrap();
        let before = release::get(&conn, &id).unwrap().unwrap();
        release::update(
            &conn,
            &id,
            crate::release::ReleasePatch {
                meta: Some(merged(&before.meta, &generated.values)),
                ..Default::default()
            },
        )
        .unwrap();

        let after = fullness(&fields(&conn, &id).unwrap());
        assert_eq!(
            after,
            Fullness {
                written: 2,
                total: 3
            },
            "the two templated fields are written; the one typed by hand is not"
        );
    }

    #[test]
    fn a_blank_field_counts_as_unwritten() {
        let (mut conn, profile_id) = workspace();
        let id = song_with_audio(&mut conn, &profile_id, None);
        release::update(
            &conn,
            &id,
            crate::release::ReleasePatch {
                meta: Some(json!({ "title": "   " }).as_object().cloned().unwrap()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(fullness(&fields(&conn, &id).unwrap()).written, 0);
    }
}
