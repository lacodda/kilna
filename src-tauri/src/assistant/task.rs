//! Profile actions started as tasks rather than as conversation.
//!
//! The panel's own action buttons fill the composer and wait: what is about to
//! be sent — and paid for — is read first. That is right when the panel is
//! already open and the answer is the point. It is wrong for the other way an
//! action is used: from the card, hands on the work, wanting the thing done
//! and not wanting to move.
//!
//! A task is that second way. It renders the profile's template, opens a chat
//! of its own for it, and starts a run — all before returning, so the caller
//! has something to show at once. The chat is always new: dropping a task into
//! whatever conversation happened to be open would bury it in someone else's
//! thread and, worse, hand it that thread's session as context.

use rusqlite::Connection;

use crate::error::{Error, Result};
use crate::profile;
use crate::work;
use crate::work::version;

use super::prompt::Produces;

/// A task the caller asked for, before anything was started.
pub struct Prepared {
    /// The chat opened to hold it.
    pub chat_id: String,
    /// The rendered prompt.
    pub prompt: String,
    /// What this task is, for the duplicate check.
    pub key: String,
    /// The chat's name, so the list does not show a task as an untitled chat.
    pub title: String,
}

/// The `produces` value naming a scoring action.
pub const SCORE: &str = "score";

/// What a task is, as a key: this action, on this work.
///
/// Two clicks on the same button produce the same key; the same action on
/// another work does not. That is the whole rule — the key says what is being
/// done, never which run is doing it.
pub fn key(action: &str, work_id: &str) -> String {
    format!("{action}:{work_id}")
}

/// Render `action` of the active profile against `work_id` and open a chat for
/// it.
///
/// `version_id` is the version the action is about — the one open on the
/// versions tab. It is what the template reads and what the answer's
/// proposal will bind to; without it the action is about the work as it
/// stands.
///
/// Fails when the profile has no such action: a card offering a button the
/// profile dropped is a card that has to be told, not one that should quietly
/// send an empty prompt.
pub fn prepare(
    conn: &Connection,
    work_id: &str,
    action: &str,
    version_id: Option<&str>,
) -> Result<Prepared> {
    let profile =
        profile::active(conn)?.ok_or_else(|| Error::Other("no profile is active".into()))?;

    let template = profile
        .config
        .prompts
        .iter()
        .find(|prompt| prompt.key == action)
        .ok_or_else(|| Error::not_found("prompt", action))?;

    let work = work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    // Checked here rather than left to the render: a version of another work
    // must be refused before a chat is opened for it.
    if let Some(id) = version_id {
        let found = version::get(conn, id)?.ok_or_else(|| Error::not_found("version", id))?;
        if found.work_id != work.id {
            return Err(Error::Other(format!(
                "version `{id}` is not a version of “{}”",
                work.title
            )));
        }
    }
    let mut prompt = super::prompt::for_work(conn, work_id, &template.template, version_id)?;

    // An action that asks for something the application can act on says the
    // shape it needs. Ordinary actions say nothing and get prose.
    match template.produces() {
        Produces::Score => prompt.push_str(&super::proposal::scoring_instruction(
            &profile.config,
            &work.kind,
        )),
        Produces::Version(role) => {
            let label = profile
                .config
                .vocabulary(&work.kind)
                .version_roles
                .iter()
                .find(|r| r.key == role)
                .map(|r| r.label.clone())
                .unwrap_or(role);
            prompt.push_str(&super::proposal::version_instruction(&label));
        }
        Produces::Prose => {}
    }

    // The instruction that lets the assistant mark its own question. Only
    // tasks carry it: a prompt typed in the panel is read before it is sent,
    // and appending words the person did not write would break that.
    let prompt = super::waiting::instruct(&prompt);

    // Named on creation rather than left to borrow its first question: a
    // rendered template can open with pages of the work's own text, and a
    // chat list full of lyrics tells nobody which task produced what.
    let title = format!("{} · {}", template.label, work.title);

    let chat = super::create(
        conn,
        &profile.id,
        super::NewChat {
            work_id: Some(work_id.to_owned()),
            title: Some(title.clone()),
            action: Some(action.to_owned()),
            version_id: version_id.map(str::to_owned),
        },
    )?;

    Ok(Prepared {
        chat_id: chat.id,
        prompt,
        key: key(action, work_id),
        title,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::work::NewWork;
    use crate::work::version::{self, NewVersion};

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    fn work_with_body(conn: &mut Connection, profile_id: &str, title: &str, body: &str) -> String {
        let work = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: title.into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        version::create(
            conn,
            &work.id,
            NewVersion {
                role: "lyrics".into(),
                body: body.into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
        work.id
    }

    /// The first action the seeded profile offers, whatever it is called.
    fn some_action(conn: &Connection) -> crate::assistant::prompt::PromptTemplate {
        profile::active(conn)
            .unwrap()
            .unwrap()
            .config
            .prompts
            .first()
            .cloned()
            .expect("the seeded profile has actions")
    }

    #[test]
    fn a_task_gets_a_chat_of_its_own_tied_to_the_work() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");
        let action = some_action(&conn);

        let prepared = prepare(&conn, &work_id, &action.key, None).unwrap();

        let chat = super::super::get(&conn, &prepared.chat_id)
            .unwrap()
            .unwrap();
        assert_eq!(chat.work_id.as_deref(), Some(work_id.as_str()));
        assert!(chat.session_id.is_none(), "a fresh chat carries no session");
    }

    #[test]
    fn every_task_opens_a_new_chat_rather_than_reusing_one() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");
        let action = some_action(&conn);

        let first = prepare(&conn, &work_id, &action.key, None).unwrap();
        let second = prepare(&conn, &work_id, &action.key, None).unwrap();

        assert_ne!(
            first.chat_id, second.chat_id,
            "a task must never land in a conversation already going"
        );
    }

    #[test]
    fn the_chat_is_named_after_the_action_and_the_work() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");
        let action = some_action(&conn);

        let prepared = prepare(&conn, &work_id, &action.key, None).unwrap();

        let chat = super::super::get(&conn, &prepared.chat_id)
            .unwrap()
            .unwrap();
        let title = chat.title.expect("a task chat is named on creation");
        assert!(title.contains(&action.label), "{title}");
        assert!(title.contains("Harbour lights"), "{title}");
    }

    #[test]
    fn the_prompt_is_rendered_against_the_work() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(
            &mut conn,
            &profile_id,
            "Harbour lights",
            "the cranes go still",
        );
        let action = some_action(&conn);

        let prepared = prepare(&conn, &work_id, &action.key, None).unwrap();

        assert!(
            !prepared.prompt.contains('{'),
            "an unrendered placeholder means the template never saw the work: {}",
            prepared.prompt
        );
        assert!(
            prepared.prompt.contains("the cranes go still"),
            "{}",
            prepared.prompt
        );
    }

    #[test]
    fn a_task_prompt_carries_the_marker_instruction() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");
        let action = some_action(&conn);

        let prepared = prepare(&conn, &work_id, &action.key, None).unwrap();

        assert!(
            prepared.prompt.contains(crate::assistant::waiting::MARKER),
            "a task must be able to say it stopped to ask: {}",
            prepared.prompt
        );
        assert!(
            prepared.prompt.starts_with("Here are the lyrics"),
            "the instruction goes after the action, never in front of it"
        );
    }

    #[test]
    fn an_action_the_profile_does_not_have_fails() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");

        let refused = prepare(&conn, &work_id, "no-such-action", None);

        assert!(refused.is_err());
    }

    #[test]
    fn a_failed_task_leaves_no_chat_behind() {
        let (mut conn, profile_id) = workspace();
        let work_id = work_with_body(&mut conn, &profile_id, "Harbour lights", "the cranes");

        let _ = prepare(&conn, &work_id, "no-such-action", None);

        let chats: i64 = conn
            .query_row("SELECT count(*) FROM chat", [], |row| row.get(0))
            .unwrap();
        assert_eq!(chats, 0, "the chat is opened only once there is a prompt");
        let _ = work_id;
    }

    #[test]
    fn an_unknown_work_fails() {
        let (conn, _) = workspace();
        let action = some_action(&conn);

        assert!(prepare(&conn, "nope", &action.key, None).is_err());
    }

    #[test]
    fn the_key_names_the_action_and_the_work_not_the_run() {
        assert_eq!(key("critique", "w1"), key("critique", "w1"));
        assert_ne!(key("critique", "w1"), key("critique", "w2"));
        assert_ne!(key("critique", "w1"), key("score", "w1"));
    }
}

#[cfg(test)]
mod version_tests {
    use super::*;
    use crate::db;
    use crate::work::NewWork;
    use crate::work::version::{self, NewVersion};

    fn workspace() -> (Connection, String) {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id)
    }

    /// A task started on a version reads that version, remembers it on the
    /// chat with the action, and a task started on the work as a whole
    /// remembers the action alone.
    #[test]
    fn a_task_on_a_version_reads_it_and_the_chat_remembers_it() {
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
        let mut make = |body: &str| {
            version::create(
                &mut conn,
                &work.id,
                NewVersion {
                    role: "lyrics".into(),
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
        let first = make("the cranes go still");
        make("the cranes go on");

        let prepared = prepare(&conn, &work.id, "critique", Some(&first)).unwrap();
        assert!(
            prepared.prompt.contains("the cranes go still")
                && !prepared.prompt.contains("the cranes go on"),
            "{}",
            prepared.prompt
        );
        let chat = super::super::get(&conn, &prepared.chat_id)
            .unwrap()
            .unwrap();
        assert_eq!(chat.action.as_deref(), Some("critique"));
        assert_eq!(chat.version_id.as_deref(), Some(first.as_str()));

        let whole = prepare(&conn, &work.id, "critique", None).unwrap();
        let chat = super::super::get(&conn, &whole.chat_id).unwrap().unwrap();
        assert_eq!(chat.action.as_deref(), Some("critique"));
        assert!(chat.version_id.is_none());
        assert!(whole.prompt.contains("the cranes go on"), "the latest");
    }

    /// The shipped critique produces a version in the `critique` role, and
    /// the prompt says so: the whole answer is what is kept.
    #[test]
    fn an_action_that_produces_a_version_says_so_in_the_prompt() {
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
                body: "x".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
            },
        )
        .unwrap();
        let prepared = prepare(&conn, &work.id, "critique", None).unwrap();
        assert!(
            prepared.prompt.contains("kept as the Critique"),
            "{}",
            prepared.prompt
        );
        let scored = prepare(&conn, &work.id, "score", None).unwrap();
        assert!(
            scored
                .prompt
                .contains("exactly as the profile defines them"),
            "{}",
            scored.prompt
        );
    }
}
