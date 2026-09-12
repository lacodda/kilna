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
    /// What the action asks the assistant to produce, beyond prose.
    ///
    /// Absent means an ordinary action: the answer is text and the reader
    /// decides what to do with it. `"score"` asks for values along the
    /// profile's axes, which kilna can offer to apply — with a confirmation,
    /// because the assistant never writes to the workspace itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produces: Option<String>,
    /// How the action is done: the role the assistant takes, what it checks
    /// and in what order, the shape of the answer, what it must never say.
    /// A method is the craft's own and lives in the profile so it ships with
    /// it and is edited with it; it reaches the model as a system
    /// instruction, not as part of the message. See ADR 0021.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
}

/// What an action asks the answer to be, read off `produces`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Produces {
    /// Text, for a person to read.
    Prose,
    /// Marks along the kind's axes, in a block the application reads.
    Score,
    /// The whole answer, kept as a version in this role.
    Version(String),
}

impl PromptTemplate {
    /// `produces` as the application understands it. An unknown value reads
    /// as prose rather than failing: a profile written for a later kilna
    /// still loads, and its answer is still an answer.
    pub fn produces(&self) -> Produces {
        match self.produces.as_deref().map(str::trim) {
            Some("score") => Produces::Score,
            Some(value) => match value.strip_prefix("version:") {
                Some(role) if !role.trim().is_empty() => Produces::Version(role.trim().to_owned()),
                _ => Produces::Prose,
            },
            None => Produces::Prose,
        }
    }

    /// The method, when the action states one worth sending: blank is none.
    pub fn method(&self) -> Option<&str> {
        self.method
            .as_deref()
            .map(str::trim)
            .filter(|method| !method.is_empty())
    }
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
///
/// `version_id` names the version the action is about — the one open on the
/// versions tab — and it stands in for `{body}` and for `{role:<its role>}`.
/// Without it the work's current version is `{body}` and every role reads
/// its latest revision, which is what an action started from the overview
/// means.
pub fn for_work(
    conn: &Connection,
    work_id: &str,
    template: &str,
    version_id: Option<&str>,
) -> Result<String> {
    let work = work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;

    // The named version, or the current one: what the user is looking at.
    // Roles beyond it are fetched by name so a template can ask for the style
    // as well as the text.
    let named = match version_id {
        Some(id) => {
            let found = version::get(conn, id)?.ok_or_else(|| Error::not_found("version", id))?;
            if found.work_id != work.id {
                return Err(Error::Other(format!(
                    "version `{id}` is not a version of “{}”",
                    work.title
                )));
            }
            Some(found)
        }
        None => None,
    };
    let current = match (&named, &work.current_version_id) {
        (Some(named), _) => Some(named.clone()),
        (None, Some(id)) => version::get(conn, id)?,
        (None, None) => None,
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
        // The named version is its role's text, whatever revision is latest:
        // a critique of revision 2 must read revision 2.
        if let Some(named) = named.as_ref().filter(|named| named.role == role) {
            rendered = rendered.replace(&format!("{{role:{role}}}"), &named.body);
            continue;
        }
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
                parent_version_id: None,
            },
        )
        .unwrap();

        let prompt = for_work(&conn, &work.id, "About {title}:\n{body}", None).unwrap();

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
                ..NewWork::default()
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
                parent_version_id: None,
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
                parent_version_id: None,
            },
        )
        .unwrap();

        let prompt = for_work(
            &conn,
            &work.id,
            "Text: {role:lyrics}\nStyle: {role:style}",
            None,
        )
        .unwrap();

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
                ..NewWork::default()
            },
        )
        .unwrap();

        let prompt = for_work(&conn, &work.id, "{title}:{body}", None).unwrap();

        assert_eq!(prompt, "Empty:");
    }

    #[test]
    fn an_unknown_work_fails() {
        let (conn, _) = workspace();

        assert!(for_work(&conn, "nope", "{title}", None).is_err());
    }
}

#[cfg(test)]
mod version_tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::work::NewWork;
    use crate::work::version::NewVersion;

    /// An action started on revision 1 reads revision 1 — as `{body}` and as
    /// `{role:lyrics}` — while the work has moved on to revision 2. Another
    /// role still reads its latest, and a version of another work is refused.
    #[test]
    fn a_named_version_is_what_the_template_reads() {
        let mut conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        let work = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Harbour lights".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        let make = |conn: &mut Connection, role: &str, body: &str| {
            version::create(
                conn,
                &work.id,
                NewVersion {
                    role: role.into(),
                    body: body.into(),
                    label: None,
                    meta: None,
                    make_current: true,
                    parent_version_id: None,
                },
            )
            .unwrap()
            .id
        };
        let first = make(&mut conn, "lyrics", "first draft");
        make(&mut conn, "lyrics", "second draft");
        make(&mut conn, "style", "warm tape");

        let rendered = for_work(
            &conn,
            &work.id,
            "B:{body}|L:{role:lyrics}|S:{role:style}",
            Some(&first),
        )
        .unwrap();
        assert_eq!(rendered, "B:first draft|L:first draft|S:warm tape");

        let latest = for_work(&conn, &work.id, "L:{role:lyrics}", None).unwrap();
        assert_eq!(latest, "L:second draft", "without a version, the latest");

        let other = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Other".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        let err = for_work(&conn, &other.id, "{body}", Some(&first)).unwrap_err();
        assert!(err.to_string().contains("not a version of"), "{err}");
    }

    #[test]
    fn produces_is_read_as_the_application_understands_it() {
        let action = |produces: Option<&str>| PromptTemplate {
            key: "k".into(),
            label: "K".into(),
            template: String::new(),
            description: None,
            produces: produces.map(str::to_owned),
            method: None,
        };
        assert_eq!(action(None).produces(), Produces::Prose);
        assert_eq!(action(Some("score")).produces(), Produces::Score);
        assert_eq!(
            action(Some("version:critique")).produces(),
            Produces::Version("critique".into())
        );
        assert_eq!(action(Some("version:")).produces(), Produces::Prose);
        assert_eq!(
            action(Some("something-later")).produces(),
            Produces::Prose,
            "an unknown value is prose, not a refusal"
        );
    }
}
