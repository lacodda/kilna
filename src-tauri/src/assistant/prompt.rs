use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::work::{self, version};

/// A named action the panel offers, defined by the profile.
///
/// A template is a string with `{placeholders}` filled from the work in
/// context, so "Critique the lyrics" means the right thing in a music profile
/// and something else entirely in a novel one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub key: String,
    pub label: String,
    pub template: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Placeholders a template may use.
///
/// Unknown placeholders are left as written rather than blanked: a visible
/// `{typo}` in the prompt is easier to diagnose than a silent gap.
pub fn render(template: &str, values: &[(&str, String)]) -> String {
    let mut rendered = template.to_owned();
    for (key, value) in values {
        rendered = rendered.replace(&format!("{{{key}}}"), value);
    }
    rendered
}

/// Build the prompt for `template` in the context of `work_id`.
pub fn for_work(conn: &Connection, work_id: &str, template: &str) -> Result<String> {
    let work = work::get(conn, work_id)?
        .ok_or_else(|| Error::Other(format!("no work with id `{work_id}`")))?;

    // The current version is what the user is looking at; roles beyond it are
    // fetched by name so a template can ask for the style as well as the text.
    let current = match &work.current_version_id {
        Some(id) => version::get(conn, id)?,
        None => None,
    };

    let values: Vec<(&str, String)> = vec![
        ("title", work.title.clone()),
        ("kind", work.kind.clone()),
        ("status", work.status.clone()),
        (
            "body",
            current.as_ref().map(|v| v.body.clone()).unwrap_or_default(),
        ),
    ];

    // Every role the work has, latest revision, as {role:lyrics} and friends.
    let roles = version::list(conn, work_id)?;
    let mut seen: Vec<String> = Vec::new();
    for summary in roles {
        if seen.contains(&summary.role) {
            continue;
        }
        seen.push(summary.role.clone());
    }

    let mut rendered = render(template, &values);
    for role in seen {
        if let Some(latest) = version::latest(conn, work_id, &role)? {
            rendered = rendered.replace(&format!("{{role:{role}}}"), &latest.body);
        }
    }

    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::work::NewWork;
    use crate::work::version::NewVersion;

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    #[test]
    fn placeholders_are_replaced() {
        let rendered = render(
            "Rewrite {title} in {kind}",
            &[("title", "Harbour lights".into()), ("kind", "song".into())],
        );

        assert_eq!(rendered, "Rewrite Harbour lights in song");
    }

    #[test]
    fn an_unknown_placeholder_is_left_visible() {
        let rendered = render("Check {typo}", &[("title", "x".into())]);

        assert_eq!(
            rendered, "Check {typo}",
            "a visible placeholder is easier to diagnose than a silent gap"
        );
    }

    #[test]
    fn a_work_prompt_carries_the_title_and_the_current_body() {
        let (mut conn, profile_id) = workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Harbour lights".into(),
                status: None,
                collection_id: None,
                meta: None,
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

        let prompt = for_work(&conn, &work.id, "About {title}:\n{body}").unwrap();

        assert_eq!(prompt, "About Harbour lights:\nthe cranes go still");
    }

    #[test]
    fn a_prompt_can_ask_for_a_role_the_work_is_not_currently_on() {
        let (mut conn, profile_id) = workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Subject".into(),
                status: None,
                collection_id: None,
                meta: None,
            },
        )
        .unwrap();
        version::create(
            &mut conn,
            &work.id,
            NewVersion {
                role: "lyrics".into(),
                body: "the words".into(),
                label: None,
                meta: None,
                make_current: true,
            },
        )
        .unwrap();
        version::create(
            &mut conn,
            &work.id,
            NewVersion {
                role: "style".into(),
                body: "slow indie folk".into(),
                label: None,
                meta: None,
                make_current: false,
            },
        )
        .unwrap();

        let prompt = for_work(&conn, &work.id, "Text: {role:lyrics}\nStyle: {role:style}").unwrap();

        assert_eq!(prompt, "Text: the words\nStyle: slow indie folk");
    }

    #[test]
    fn a_work_without_versions_renders_an_empty_body_rather_than_failing() {
        let (conn, profile_id) = workspace();
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Empty".into(),
                status: None,
                collection_id: None,
                meta: None,
            },
        )
        .unwrap();

        let prompt = for_work(&conn, &work.id, "{title}:{body}").unwrap();

        assert_eq!(prompt, "Empty:");
    }

    #[test]
    fn an_unknown_work_fails() {
        let (conn, _) = workspace();

        assert!(for_work(&conn, "nope", "{title}").is_err());
    }
}
