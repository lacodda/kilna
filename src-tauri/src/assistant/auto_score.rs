//! Scoring a text by itself (FR-01, v0.123).
//!
//! A song is scored by the `score` action when a person presses the button.
//! With this on, it is also scored when its text has been left alone for a
//! while - so the catalogue's numbers describe the texts as they stand, not
//! as they stood the last time somebody remembered to ask.
//!
//! Three things keep that from being a way to burn money:
//!
//! * **who is scored** ([`standing`]): a work whose kind is judged along axes
//!   and has a sung text, that has not gone out and is not shelved, that
//!   stands at the stage the person named or further, whose current text is
//!   written and is not the bare skeleton of its role, and whose score does
//!   not already describe it;
//! * **when** ([`Waiting`]): after a silence. Every saved edit puts the
//!   moment off again, so a person still writing is not scored line by line;
//! * **what counts as a change** ([`significant`]): a comma or a typo mended
//!   in a text that was scored is not a reason to score it again.
//!
//! What is asked for is the same task a click asks for, under the same key,
//! so one that is already going or waiting is one request. It waits behind
//! everything a person asked for themselves and leaves them a free slot
//! (`queue`).
//!
//! The timers live with the window, not in a screen of it: leaving the card
//! does not cancel a score that was due. They are memory, like the queue -
//! closing the application drops them, and a text edited in a session that
//! is gone is scored when it is next edited, or when the catch-up is pressed.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::Result;
use crate::profile::config::ProfileConfig;
use crate::work::Work;

use super::settings::AssistantSettings;

/// The action that scores a work.
pub const ACTION: &str = "score";

/// Who a score written without a person is by: the mark of "the assistant's
/// opinion", which never stands over the author's own.
pub const RATER: &str = "auto";

/// How many characters of a text, not counting space, have to differ before
/// a scored text is worth scoring again - more than a typo and a comma.
const WORTH_IT: usize = 15;

/// Why a work is not scored by itself. The words are for the log and the
/// catch-up's count, not for a person mid-sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Not {
    /// Its kind is not judged along axes, or has no sung text.
    NotJudged,
    /// The profile has no `score` action for its kind.
    NoAction,
    /// It has gone out, or was put on the shelf.
    Settled,
    /// It stands before the stage the person named.
    TooEarly,
    /// It has no current text in a sung role, or the text is empty or still
    /// the role's skeleton.
    NothingWritten,
    /// Its score describes the text as it stands.
    Scored,
    /// It changed since it was scored, by less than is worth a run.
    BarelyChanged,
}

/// Where a work stands with being scored by itself: the version to score, or
/// why not.
///
/// `seen` is the text this work had when it was last asked to be scored in
/// this sitting, when it was: what a later change is measured against.
pub fn standing(
    conn: &Connection,
    config: &ProfileConfig,
    settings: &AssistantSettings,
    work: &Work,
    seen: Option<&str>,
) -> Result<std::result::Result<String, Not>> {
    let kind = config.vocabulary(&work.kind);
    let sung: Vec<&crate::profile::config::VersionRole> =
        kind.version_roles.iter().filter(|role| role.sung).collect();
    if kind.axes.is_empty() || sung.is_empty() {
        return Ok(Err(Not::NotJudged));
    }
    let has_action = config.prompts.iter().any(|prompt| {
        prompt.key == ACTION
            && prompt.scope.is_none()
            && (prompt.kinds.is_empty() || prompt.kinds.contains(&work.kind))
    });
    if !has_action {
        return Ok(Err(Not::NoAction));
    }
    if matches!(work.status.as_str(), "released" | "shelved") {
        return Ok(Err(Not::Settled));
    }
    if work.stage < settings.auto_score_min_stage {
        return Ok(Err(Not::TooEarly));
    }

    let Some(version_id) = work.current_version_id.as_deref() else {
        return Ok(Err(Not::NothingWritten));
    };
    let Some(version) = crate::work::version::get(conn, version_id)? else {
        return Ok(Err(Not::NothingWritten));
    };
    let Some(role) = sung.iter().find(|role| role.key == version.role) else {
        return Ok(Err(Not::NothingWritten));
    };
    let skeletons: Vec<String> = role
        .skeleton
        .as_ref()
        .map(|label| label.words().into_iter().map(str::to_owned).collect())
        .unwrap_or_default();
    if is_bare(&version.body, &skeletons) {
        return Ok(Err(Not::NothingWritten));
    }

    // The newest score of this very version, and whether the work was
    // touched after it - the catalogue's own "stale".
    let scored: Option<String> = conn
        .query_row(
            "SELECT scored_at FROM work_score WHERE work_id = ?1 AND version_id = ?2
              ORDER BY scored_at DESC, rowid DESC LIMIT 1",
            params![work.id, version_id],
            |row| row.get(0),
        )
        .optional()?;
    match scored {
        None => Ok(Ok(version.id)),
        Some(at) if work.updated_at <= at => Ok(Err(Not::Scored)),
        Some(_) => match seen {
            Some(before) if !significant(before, &version.body) => Ok(Err(Not::BarelyChanged)),
            _ => Ok(Ok(version.id)),
        },
    }
}

/// Whether a text is nothing to score: empty, or the skeleton its role opens
/// with - `[Intro]`, `[Verse 1]` - in any of the languages it is written in.
/// Space and blank lines are not a difference.
pub fn is_bare(body: &str, skeletons: &[String]) -> bool {
    let pressed = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let body = pressed(body);
    body.is_empty() || skeletons.iter().any(|skeleton| pressed(skeleton) == body)
}

/// Whether `after` differs from `before` by enough to score it again.
///
/// The stretch that changed is what lies between what the two texts begin
/// with in common and what they end with in common. It is enough when it
/// holds fifteen characters that are not space, on either side, or when a
/// line came or went - a new line of a song is a change however short. A
/// comma, a typo, a capital is not.
pub fn significant(before: &str, after: &str) -> bool {
    if before == after {
        return false;
    }
    let a: Vec<char> = before.chars().collect();
    let b: Vec<char> = after.chars().collect();
    let head = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let most = a.len().min(b.len()) - head;
    let tail = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take(most)
        .take_while(|(x, y)| x == y)
        .count();
    let changed = |text: &[char]| -> (usize, bool) {
        let span = &text[head..text.len() - tail];
        (
            span.iter().filter(|c| !c.is_whitespace()).count(),
            span.contains(&'\n'),
        )
    };
    let (gone, line_gone) = changed(&a);
    let (come, line_come) = changed(&b);
    gone.max(come) >= WORTH_IT || line_gone || line_come
}

/// The works waiting for their silence to end, and what this sitting knows
/// of the texts it asked to have scored.
#[derive(Default)]
pub struct Waiting {
    /// When each work's silence ends.
    due: Mutex<HashMap<String, Instant>>,
    /// The text each work had when it was last asked to be scored.
    seen: Mutex<HashMap<String, String>>,
    /// Works whose text changed while they were being scored: scored again
    /// when the run ends, since what it judged is no longer what stands.
    again: Mutex<HashSet<String>>,
}

fn held<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Waiting {
    pub fn new() -> Self {
        Self::default()
    }

    /// Put a work's moment `after` from now - again, if it was waiting: each
    /// edit starts the silence over.
    pub fn put_off(&self, work_id: &str, after: Duration) {
        held(&self.due).insert(work_id.to_owned(), Instant::now() + after);
    }

    /// The works whose silence has ended by `now`, taken off the list.
    pub fn take_due(&self, now: Instant) -> Vec<String> {
        let mut due = held(&self.due);
        let ready: Vec<String> = due
            .iter()
            .filter(|(_, at)| **at <= now)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &ready {
            due.remove(id);
        }
        ready
    }

    /// Forget every waiting work - what turning the setting off does.
    pub fn clear(&self) -> usize {
        let mut due = held(&self.due);
        let waiting = due.len();
        due.clear();
        held(&self.again).clear();
        waiting
    }

    /// How many works wait for their silence to end.
    pub fn len(&self) -> usize {
        held(&self.due).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The text a work had when it was last asked to be scored.
    pub fn seen(&self, work_id: &str) -> Option<String> {
        held(&self.seen).get(work_id).cloned()
    }

    /// Remember the text a work is being asked to be scored with.
    pub fn saw(&self, work_id: &str, body: &str) {
        held(&self.seen).insert(work_id.to_owned(), body.to_owned());
    }

    /// Say a work changed while it was being scored.
    pub fn score_again(&self, work_id: &str) {
        held(&self.again).insert(work_id.to_owned());
    }

    /// Whether a work is to be scored again now that its run has ended; the
    /// mark is taken with the answer.
    pub fn take_again(&self, work_id: &str) -> bool {
        held(&self.again).remove(work_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::score::NewScore;
    use crate::work::version::NewVersion;

    #[test]
    fn a_typo_is_not_a_change_worth_a_run_and_a_new_line_is() {
        let text = "The tide goes out past the harbour wall\nand the boats lie on their sides";
        // A comma, a letter, a capital.
        assert!(!significant(text, &text.replace("wall", "wall,")));
        assert!(!significant(text, &text.replace("boats", "boots")));
        assert!(!significant(text, &text.replace("The tide", "the tide")));
        assert!(!significant(text, text));
        // A line added or taken away, however short.
        assert!(significant(text, &format!("{text}\noh")));
        assert!(significant(text, "The tide goes out past the harbour wall"));
        // A line rewritten.
        assert!(significant(
            text,
            &text.replace(
                "and the boats lie on their sides",
                "and nobody is left to see it go"
            )
        ));
        // From nothing.
        assert!(significant("", "The tide goes out past the harbour wall"));
    }

    #[test]
    fn an_empty_text_and_a_roles_skeleton_are_nothing_to_score() {
        let skeletons = vec![
            "[Intro]\n\n[Verse 1]\n\n[Chorus]".to_owned(),
            "[Интро]\n\n[Куплет 1]\n\n[Припев]".to_owned(),
        ];
        assert!(is_bare("", &skeletons));
        assert!(is_bare("  \n\n ", &skeletons));
        assert!(is_bare("[Intro]\n[Verse 1]\n\n\n[Chorus]\n", &skeletons));
        assert!(is_bare("[Интро] [Куплет 1] [Припев]", &skeletons));
        assert!(!is_bare(
            "[Intro]\n\n[Verse 1]\nThe tide goes out\n\n[Chorus]",
            &skeletons
        ));
    }

    /// A song with a written text, at the stage the setting asks for.
    fn written(conn: &Connection, profile_id: &str) -> Work {
        let song = fixtures::song(conn, profile_id, "Tide");
        crate::work::version::create(
            conn,
            &song.id,
            NewVersion {
                role: "lyrics".into(),
                body: "The tide goes out past the harbour wall\nand the boats lie on their sides"
                    .into(),
                make_current: true,
                ..NewVersion::default()
            },
        )
        .unwrap();
        crate::work::update(
            conn,
            &song.id,
            crate::work::WorkPatch {
                stage: Some(Some(50)),
                ..Default::default()
            },
        )
        .unwrap()
    }

    fn on() -> AssistantSettings {
        AssistantSettings {
            auto_score: true,
            ..AssistantSettings::default()
        }
    }

    #[test]
    fn a_written_song_far_enough_along_is_scored_and_the_rest_are_not() {
        let (conn, profile_id) = fixtures::workspace();
        let config = crate::profile::config_for(&conn, &profile_id).unwrap();
        let song = written(&conn, &profile_id);
        let ask = |work: &Work| standing(&conn, &config, &on(), work, None).unwrap();

        assert_eq!(ask(&song), Ok(song.current_version_id.clone().unwrap()));

        // Before the stage the person named.
        let mut early = song.clone();
        early.stage = 25;
        assert_eq!(ask(&early), Err(Not::TooEarly));
        // Out, or on the shelf.
        for status in ["released", "shelved"] {
            let mut settled = song.clone();
            settled.status = status.into();
            assert_eq!(ask(&settled), Err(Not::Settled), "{status}");
        }
        // Nothing written, or only the skeleton.
        let empty = fixtures::song(&conn, &profile_id, "Empty");
        let mut bare = empty.clone();
        bare.stage = 100;
        assert_eq!(ask(&bare), Err(Not::NothingWritten));
        // A kind that is not judged.
        let mut clip = song.clone();
        clip.kind = "audio".into();
        assert_eq!(ask(&clip), Err(Not::NotJudged));
    }

    #[test]
    fn a_score_that_describes_the_text_is_enough_until_the_text_really_changes() {
        let (conn, profile_id) = fixtures::workspace();
        let config = crate::profile::config_for(&conn, &profile_id).unwrap();
        let song = written(&conn, &profile_id);
        let version = song.current_version_id.clone().unwrap();
        let before = crate::work::version::get(&conn, &version)
            .unwrap()
            .unwrap()
            .body;
        crate::score::create(
            &conn,
            &song.id,
            NewScore {
                axes: serde_json::Map::new(),
                version_id: Some(version.clone()),
                note: None,
                rater: None,
            },
        )
        .unwrap();
        let now = || crate::work::get(&conn, &song.id).unwrap().unwrap();
        let ask = |seen: Option<&str>| standing(&conn, &config, &on(), &now(), seen).unwrap();

        assert_eq!(ask(None), Err(Not::Scored));

        // Touched after the score: a typo is not worth a run, a new line is.
        let edit = |body: &str| {
            conn.execute(
                "UPDATE work_version SET body = ?2 WHERE id = ?1",
                params![version, body],
            )
            .unwrap();
            conn.execute(
                "UPDATE work SET updated_at = '2999-01-01T00:00:00.000Z' WHERE id = ?1",
                params![song.id],
            )
            .unwrap();
        };
        edit(&before.replace("wall", "wall,"));
        assert_eq!(ask(Some(&before)), Err(Not::BarelyChanged));
        edit(&format!("{before}\nand nobody is left to see it go"));
        assert_eq!(ask(Some(&before)), Ok(version.clone()));
        // Not known what it was scored with: scored, rather than left stale.
        edit(&before.replace("wall", "wall,"));
        assert_eq!(ask(None), Ok(version));
    }

    #[test]
    fn a_silence_is_started_over_by_each_edit_and_ends_once() {
        let waiting = Waiting::new();
        let start = Instant::now();
        waiting.put_off("a", Duration::from_secs(600));
        waiting.put_off("b", Duration::from_secs(60));
        assert!(waiting.take_due(start).is_empty());
        // Another edit of `b` puts its moment off again.
        waiting.put_off("b", Duration::from_secs(600));
        assert!(
            waiting
                .take_due(start + Duration::from_secs(120))
                .is_empty()
        );

        let mut due = waiting.take_due(start + Duration::from_secs(700));
        due.sort();
        assert_eq!(due, ["a", "b"]);
        assert!(waiting.is_empty(), "taken once");

        waiting.put_off("c", Duration::from_secs(1));
        waiting.score_again("c");
        assert_eq!(waiting.clear(), 1);
        assert!(!waiting.take_again("c"), "turning it off forgets that too");

        waiting.saw("a", "text");
        assert_eq!(waiting.seen("a").as_deref(), Some("text"));
        waiting.score_again("a");
        assert!(waiting.take_again("a"));
        assert!(!waiting.take_again("a"));
    }
}
