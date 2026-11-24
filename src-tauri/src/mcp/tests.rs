//! What the tools answer and what a proposal leaves behind, called as a
//! client calls them: `run_tool` and `handle` against a workspace in memory.

use super::*;
use crate::fixtures;

/// [`fixtures::workspace`], with a song and its lyrics already on it - most
/// tests here read the transcript or the board against a work in hand.
fn workspace() -> (Connection, String) {
    let (conn, profile_id) = fixtures::workspace();
    let work_id = fixtures::song(&conn, &profile_id, "Harbour lights").id;
    fixtures::version(&conn, &work_id, "lyrics", "one line\ntwo lines");
    (conn, work_id)
}

fn claude() -> Session {
    Session {
        client: Some("Claude Code".into()),
        space: None,
    }
}

/// A video in the workspace's profile — a kind with a storyboard.
fn video(conn: &Connection) -> String {
    let profile_id = profile::active(conn).unwrap().unwrap().id;
    fixtures::video(conn, &profile_id, "Harbour lights — the clip").id
}

fn first_message(conn: &Connection, work_id: &str) -> assistant::Message {
    let profile_id = profile::active(conn).unwrap().unwrap().id;
    let chats = assistant::summaries(conn, &profile_id, Some(work_id)).unwrap();
    assert_eq!(chats.len(), 1, "one chat, the client's");
    assistant::transcript(conn, &chats[0].id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0)
}

/// The first message of the one chat not on a work - where a proposal of
/// an agent that named no work waits.
fn first_message_anywhere(conn: &Connection) -> assistant::Message {
    let profile_id = profile::active(conn).unwrap().unwrap().id;
    let chats = assistant::summaries(conn, &profile_id, None).unwrap();
    let chat = chats.first().expect("a chat");
    assistant::transcript(conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0)
}

/// An agent reads the order of a song prepared on a conception - null
/// while nothing of it is written, the whole markdown once it is - and
/// proposes cards onto its board; they wait until the person puts them
/// on it, and a field the board does not have is left out and said.
#[test]
fn an_agent_reads_the_order_and_proposes_cards() {
    let (conn, song) = workspace();
    let read = run_tool(&conn, &claude(), "brief", &args(json!({ "work": song }))).unwrap();
    let read: Value = serde_json::from_str(&read).unwrap();
    assert_eq!(read["order"], Value::Null, "nothing is written yet");
    assert_eq!(read["brief_role"], "brief");
    assert_eq!(read["boards"][0]["key"], "research");
    assert_eq!(read["boards"][1]["key"], "scenarios");

    fixtures::version(
        &conn,
        &song,
        "brief",
        "## What it is about\nthe harbour at night",
    );
    let read = run_tool(&conn, &claude(), "brief", &args(json!({ "work": song }))).unwrap();
    let read: Value = serde_json::from_str(&read).unwrap();
    assert!(
        read["order"]
            .as_str()
            .unwrap()
            .contains("the harbour at night"),
        "{read}"
    );

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_cards",
        &args(json!({
            "work": song,
            "board": "scenarios",
            "cards": [
                { "topic": "Sea", "fields": { "line": "A keeper counts the ships", "mood": "grey" } },
                { "fields": { "line": "A boat repainted every spring" } }
            ],
            "directions": ["old maps"],
        })),
    )
    .unwrap();
    assert!(answer.contains("Proposed 2 cards"), "{answer}");
    assert!(answer.contains("mood"), "{answer}");
    let message = first_message(&conn, &song);
    assert_eq!(message.meta["proposal"]["kind"], "cards");
    assert!(
        crate::board::for_board(&conn, &song, "scenarios")
            .unwrap()
            .is_empty()
    );

    crate::actions::proposal::apply(
        &conn,
        &message.id,
        crate::actions::proposal::Overrides::default(),
    )
    .unwrap();
    let board = crate::board::for_board(&conn, &song, "scenarios").unwrap();
    assert_eq!(board.len(), 2);
    assert_eq!(board[0].topic, "Sea");
    assert_eq!(
        crate::board::directions(&conn, &song, "scenarios").unwrap(),
        ["old maps"]
    );

    let refused = run_tool(
        &conn,
        &claude(),
        "propose_cards",
        &args(
            json!({ "work": song, "board": "sources", "cards": [{ "fields": { "line": "x" } }] }),
        ),
    )
    .unwrap_err();
    assert_eq!(refused.refusal().unwrap().code, "board.notOfKind");
}

/// An agent carries research over: cards of the research board naming
/// their works of culture, one starred, a song's quote left out - and a
/// new song proposed whole with its research. Applied, the entries are
/// the library's, the star the person's choice, and `brief` reads the
/// entries back with where else they worked.
#[test]
fn research_arrives_over_mcp_with_its_entries_and_a_new_song_with_it() {
    let (conn, song) = workspace();
    let card = |title: &str, starred: bool| {
        json!({
            "kind": "work", "topic": "Quiet endings", "starred": starred,
            "entry": { "title": title, "author": "Kelly Reichardt", "year": "2006", "medium": "film" },
            "fields": { "why": "two friends drift apart", "quote": "Sorrow is nothing but worn-out joy" }
        })
    };
    let answer = run_tool(
        &conn,
        &claude(),
        "propose_cards",
        &args(json!({
            "work": song,
            "board": "research",
            "cards": [
                card("Old Joy", true),
                { "kind": "work", "entry": { "title": "A Song", "medium": "music" },
                  "fields": { "why": "the same question", "quote": "the words of it" } },
                { "kind": "concept", "fields": { "term": "Pharisee", "meaning": "a hypocrite" } }
            ],
        })),
    )
    .unwrap();
    assert!(answer.contains("Proposed 3 cards"), "{answer}");
    assert!(
        answer.contains("quote"),
        "a song's quote is said to be left out: {answer}"
    );
    let message = first_message(&conn, &song);
    crate::actions::proposal::apply(
        &conn,
        &message.id,
        crate::actions::proposal::Overrides::default(),
    )
    .unwrap();
    let board = crate::board::for_board(&conn, &song, "research").unwrap();
    assert_eq!(board.len(), 3);
    assert_eq!(board[0].verdict, Some(crate::board::Verdict::Star));
    assert_eq!(
        board[0].entry.as_ref().unwrap().year.as_deref(),
        Some("2006")
    );
    assert!(!board[1].fields.contains_key("quote"));

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "title": "The Pharisee",
            "kind": "song",
            "versions": [{ "role": "brief", "body": "## What it is about\na pilgrimage" }],
            "boards": [{ "board": "research", "cards": [card("old joy", false)] }],
        })),
    )
    .unwrap();
    assert!(answer.contains("1 card for `research`"), "{answer}");
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, None).unwrap();
    let chat = chats.iter().find(|c| c.work_id.is_none()).unwrap();
    let proposed = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    assert!(proposed.body.contains("### Research"), "{}", proposed.body);
    let outcome = crate::actions::proposal::apply(&conn, &proposed.id, Default::default()).unwrap();
    let made = outcome.work_id.unwrap();
    let cards = crate::board::for_board(&conn, &made, "research").unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(
        cards[0].note_id, board[0].note_id,
        "one entry for both songs"
    );

    let read = run_tool(&conn, &claude(), "brief", &args(json!({ "work": made }))).unwrap();
    let read: Value = serde_json::from_str(&read).unwrap();
    let research = read["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|view| view["key"] == "research")
        .unwrap();
    assert_eq!(research["cards"][0]["entry"]["title"], "Old Joy");
    assert_eq!(research["elsewhere"][0]["works"][0]["work_id"], json!(song));
}

/// An agent reads a board and what an idea is built from, proposes ideas
/// onto it, and they wait: the board stays as it was until the person
/// puts them on it. A name not in the workspace is left out and said.
#[test]
fn proposed_ideas_wait_until_the_person_puts_them_on_the_board() {
    let (conn, song) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let audio = fixtures::work(&conn, &profile_id, "audio", "Harbour lights — audio");

    let read = run_tool(
        &conn,
        &claude(),
        "cover",
        &args(json!({ "work": audio.id })),
    )
    .unwrap();
    let read: Value = serde_json::from_str(&read).unwrap();
    assert_eq!(read["ideas"], json!([]));
    assert!(
        read["choices"].as_str().unwrap().contains("`masthead`"),
        "the choices name the layouts"
    );

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_cover",
        &args(json!({
            "work": audio.id,
            "ideas": [
                { "idea": "Огни гавани", "scene": "a keeper's face by the lamp", "layout": "closeUp", "angle": "close" },
                { "idea": "Один на молу", "scene": "a small figure on a pier", "style": "No such style" }
            ],
            "note": "two distances",
        })),
    )
    .unwrap();
    assert!(answer.contains("Proposed 2 ideas"), "{answer}");
    assert!(answer.contains("No such style"), "{answer}");
    let message = first_message(&conn, &audio.id);
    assert_eq!(message.meta["proposal"]["kind"], "coverIdeas");
    assert!(message.body.contains("Огни гавани"), "{}", message.body);
    assert!(
        crate::cover::idea::for_work(&conn, &audio.id)
            .unwrap()
            .is_empty(),
        "nothing lands until the person says so"
    );

    crate::actions::proposal::apply(
        &conn,
        &message.id,
        crate::actions::proposal::Overrides {
            items: Some(vec!["idea:1".into()]),
            ..Default::default()
        },
    )
    .unwrap();
    let board = crate::cover::idea::for_work(&conn, &audio.id).unwrap();
    assert_eq!(board.len(), 1, "only the idea kept");
    assert_eq!(board[0].concept.idea, "Один на молу");

    let refused = run_tool(
        &conn,
        &claude(),
        "propose_cover",
        &args(json!({ "work": song, "ideas": [{ "idea": "x", "scene": "y" }] })),
    )
    .unwrap_err();
    assert_eq!(refused.refusal().map(|r| r.code), Some("idea.noCover"));
}

/// An agent reads an experiment's board and proposes trials onto it:
/// they wait until the person puts them on the board, a later trial
/// varying an earlier one of the same proposal lands as its child, and a
/// work that is no experiment takes none.
#[test]
fn proposed_trials_wait_and_land_as_a_tree() {
    let (conn, song) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    crate::style_set::seed(&conn).unwrap();
    let lab = fixtures::work(&conn, &profile_id, "experiment", "Breaks under guitars");

    let read = run_tool(&conn, &claude(), "trials", &args(json!({ "work": lab.id }))).unwrap();
    let read: Value = serde_json::from_str(&read).unwrap();
    assert_eq!(read["trials"], json!([]));
    assert!(
        read["choices"]
            .as_str()
            .unwrap()
            .contains("chopped amen break"),
        "the choices name the phrases of sound"
    );

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_trials",
        &args(json!({
            "work": lab.id,
            "trials": [
                { "body": "chopped amen break, fuzz bass", "angle": "core", "series": "sweep", "bricks": ["chopped amen break", "no such phrase"] },
                { "body": "chopped amen break, fuzz bass, 80 bpm", "angle": "slower", "series": "sweep", "parent": 1, "verdict": "keep", "outcome": "heavier" }
            ],
        })),
    )
    .unwrap();
    assert!(answer.contains("Proposed 2 trials"), "{answer}");
    assert!(answer.contains("no such phrase"), "{answer}");
    assert!(
        crate::lab::trial::for_work(&conn, &lab.id)
            .unwrap()
            .is_empty()
    );

    let message = first_message(&conn, &lab.id);
    assert_eq!(message.meta["proposal"]["kind"], "trials");
    crate::actions::proposal::apply(&conn, &message.id, Default::default()).unwrap();
    let board = crate::lab::trial::for_work(&conn, &lab.id).unwrap();
    assert_eq!(board.len(), 2);
    assert_eq!(board[1].parent_id.as_deref(), Some(board[0].id.as_str()));
    assert_eq!(board[1].verdict, Some(crate::lab::Verdict::Keep));
    assert_eq!(board[0].bricks.len(), 1);

    let refused = run_tool(
        &conn,
        &claude(),
        "propose_trials",
        &args(json!({ "work": song, "trials": [{ "body": "x" }] })),
    )
    .unwrap_err();
    assert_eq!(refused.refusal().map(|r| r.code), Some("trial.noLab"));
}

/// A whole experiment proposed at once - its brief, its anchors and its
/// trials by series - is made in one click, the board and all.
#[test]
fn a_proposed_experiment_arrives_with_its_board() {
    let (conn, _) = workspace();
    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "title": "Breaks under guitars",
            "kind": "experiment",
            "fields": { "anchors": "fuzz bass\namen break" },
            "versions": [{ "role": "brief", "body": "## Direction\n\nbreaks under guitars" }],
            "trials": [
                { "body": "amen break, fuzz bass", "series": "sweep" },
                { "body": "amen break, sub bass", "series": "sweep" }
            ],
        })),
    )
    .unwrap();
    assert!(answer.contains("2 trials"), "{answer}");
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, None).unwrap();
    let message = assistant::transcript(&conn, &chats[0].id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    let outcome = crate::actions::proposal::apply(&conn, &message.id, Default::default()).unwrap();
    let work_id = outcome.work_id.unwrap();
    let board = crate::lab::trial::board(&conn, &work_id).unwrap();
    assert_eq!(board.trials.len(), 2);
    assert_eq!(board.anchors, vec!["fuzz bass", "amen break"]);
    assert_eq!(board.trials[1].lost_anchors, vec!["fuzz bass"]);

    let refused = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({ "title": "A song", "kind": "song", "trials": [{ "body": "x" }] })),
    )
    .unwrap_err();
    assert_eq!(refused.refusal().map(|r| r.code), Some("trial.noLabKind"));
}

/// What a release goes out under, proposed by an agent: it waits in the
/// chat on the work, every field beside what is written now, and nothing
/// is written - an agent never fills a field, even an empty one.
#[test]
fn a_proposed_release_waits_beside_what_is_written() {
    let (conn, _) = workspace();
    let video_id = video(&conn);
    let release = fixtures::release(&conn, &video_id, "youtube", None);

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_release",
        &args(json!({
            "work": video_id,
            "fields": { "title": "Harbour lights", "tags": "sea, night", "mood": "grey" },
        })),
    )
    .unwrap();

    assert!(answer.contains("Proposed 2 fields"), "{answer}");
    assert!(
        answer.contains("mood"),
        "the field it left out is named: {answer}"
    );
    let message = first_message(&conn, &video_id);
    assert_eq!(message.meta["proposal"]["kind"], "release");
    assert!(message.body.contains("Harbour lights"), "{}", message.body);
    let waiting = crate::release_meta::pending(&conn, &release.id).unwrap();
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0].client.as_deref(), Some("Claude Code"));
    assert!(
        crate::release_meta::fields(&conn, &release.id)
            .unwrap()
            .iter()
            .all(|field| field.value.is_empty()),
        "nothing is written until the person takes it"
    );
}

/// A work with two releases needs the one named; a work with none, and a
/// song, which never goes out itself, have nothing to propose for.
#[test]
fn a_release_proposal_needs_a_work_that_goes_out() {
    let (conn, song) = workspace();
    let fields = json!({ "title": "Harbour lights" });

    let none = run_tool(
        &conn,
        &claude(),
        "propose_release",
        &args(json!({ "work": song, "fields": fields })),
    )
    .unwrap_err();
    assert_eq!(none.refusal().map(|r| r.code), Some("mcp.workHasNoRelease"));
}

/// What kilna wrote out itself from an agent's proposal is stored in
/// English, and read in the language of the window that opens the chat
/// (v0.112, ADR 0080): the headings from the locale, the kind's words in
/// that language, what the agent wrote left as written.
#[test]
fn a_proposed_board_is_stored_in_english_and_read_in_the_windows_language() {
    let (conn, _) = workspace();
    let video_id = video(&conn);
    run_tool(
        &conn,
        &claude(),
        "propose_scenes",
        &args(json!({
            "work": video_id,
            "scenes": [
                { "section": "intro", "shot_type": "wide", "layer": "story",
                  "description": "the harbour at dawn",
                  "blocks": { "still": "a harbour, first light, 35mm" } }
            ]
        })),
    )
    .unwrap();
    let stored = first_message(&conn, &video_id);
    assert!(
        stored
            .body
            .contains("| # | Section | Time | Shot | Layer | Description |"),
        "{}",
        stored.body
    );

    let body_in = |speech: Speech<'_>| {
        crate::assistant::transcript_in(&conn, &stored.chat_id, speech)
            .unwrap()
            .unwrap()
            .messages
            .into_iter()
            .find(|message| message.id == stored.id)
            .unwrap()
            .body
    };
    let russian = body_in(Speech::of("ru"));
    for expected in [
        "| # | Раздел | Время | Кадр | Слой | Описание |",
        "| + | intro |  | Общий | Сюжет | the harbour at dawn |",
        "**Сцена +**",
        "a harbour, first light, 35mm",
    ] {
        assert!(
            russian.contains(expected),
            "missing {expected:?} in:\n{russian}"
        );
    }
    // English, and a language the window does not speak, read what is
    // stored; and reading changed nothing that is stored.
    assert_eq!(body_in(Speech::ENGLISH), stored.body);
    assert_eq!(body_in(Speech::of("en-GB")), stored.body);
    assert_eq!(first_message(&conn, &video_id).body, stored.body);
}

#[test]
fn a_proposed_storyboard_waits_in_the_chat_as_a_readable_board() {
    let (conn, _) = workspace();
    let video_id = video(&conn);
    let answer = run_tool(
        &conn,
        &claude(),
        "propose_scenes",
        &args(json!({
            "work": video_id,
            "scenes": [
                { "section": "intro", "starts_at": 0, "ends_at": 4.5, "shot_type": "wide", "layer": "story",
                  "description": "the harbour at dawn",
                  "blocks": { "still": "a harbour, first light, 35mm", "negative": "no people" } },
                { "section": "verse 1", "shot_type": "close", "description": "hands on a rope" }
            ]
        })),
    )
    .unwrap();
    assert!(
        answer.contains("Proposed 2 scenes to add after the 0"),
        "{answer}"
    );

    assert_eq!(
        scene::count(&conn, &video_id).unwrap(),
        0,
        "a proposal is not a scene"
    );
    let message = first_message(&conn, &video_id);
    assert_eq!(message.meta["proposal"]["kind"], "scenes");
    assert_eq!(message.meta["proposal"]["change"], "add");
    assert_eq!(message.meta["proposal"]["scenes"][0]["shot_type"], "wide");
    assert_eq!(message.meta["proposal"]["scenes"][0]["layer"], "story");
    assert_eq!(
        message.meta["proposal"]["scenes"][0]["blocks"]["still"],
        "a harbour, first light, 35mm"
    );
    assert!(apply::is_pending(&message));
    for expected in [
        "| # | Section | Time | Shot | Layer | Description |",
        "| + | intro | 0:00–0:04.5 | Wide | Story | the harbour at dawn |",
        "| + | verse 1 |  | Close-up |  | hands on a rope |",
        "_Still frame_",
        "a harbour, first light, 35mm",
    ] {
        assert!(
            message.body.contains(expected),
            "readable before applied — missing {expected:?} in:\n{}",
            message.body
        );
    }

    let lines: i64 = conn
        .query_row(
            "SELECT count(*) FROM journal WHERE action = 'proposal.scenes'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(lines, 1);
}

#[test]
fn a_replacing_storyboard_says_so_and_numbers_its_scenes_in_order() {
    let (conn, _) = workspace();
    let video_id = video(&conn);
    let answer = run_tool(
        &conn,
        &claude(),
        "propose_scenes",
        &args(json!({
            "work": video_id, "change": "replace",
            "scenes": [{ "description": "one" }, { "description": "two" }]
        })),
    )
    .unwrap();
    assert!(answer.contains("to replace the 0 on the board"), "{answer}");
    let message = first_message(&conn, &video_id);
    assert_eq!(message.meta["proposal"]["change"], "replace");
    assert!(
        // Section, time, shot and layer are all unset: four empty cells.
        message.body.contains("| 2 |  |  |  |  | two |"),
        "{}",
        message.body
    );
}

#[test]
fn a_storyboard_in_the_wrong_words_is_refused_whole() {
    let (conn, song_id) = workspace();
    let video_id = video(&conn);

    let err = run_tool(
        &conn,
        &claude(),
        "propose_scenes",
        &args(json!({ "work": video_id, "scenes": [{ "shot_type": "closeup" }] })),
    )
    .unwrap_err();
    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("proposal.unknownShotType")
    );

    let err = run_tool(
        &conn,
        &claude(),
        "propose_scenes",
        &args(json!({ "work": video_id, "scenes": [{ "blocks": { "prompt": "x" } }] })),
    )
    .unwrap_err();
    assert_eq!(err.refusal().map(|r| r.code), Some("proposal.unknownBlock"));

    let err = run_tool(
        &conn,
        &claude(),
        "propose_scenes",
        &args(json!({ "work": video_id, "scenes": [{ "starts_at": 9, "ends_at": 3 }] })),
    )
    .unwrap_err();
    assert_eq!(err.refusal().map(|r| r.code), Some("proposal.badSpan"));

    let err = run_tool(
        &conn,
        &claude(),
        "propose_scenes",
        &args(json!({ "work": song_id, "scenes": [{ "description": "x" }] })),
    )
    .unwrap_err();
    assert_eq!(err.refusal().map(|r| r.code), Some("scene.noStoryboard"));

    let err = run_tool(
        &conn,
        &claude(),
        "propose_scenes",
        &args(json!({ "work": video_id, "scenes": [] })),
    )
    .unwrap_err();
    assert_eq!(err.refusal().map(|r| r.code), Some("mcp.scenesNotArray"));

    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    assert!(
        assistant::summaries(&conn, &profile_id, Some(&video_id))
            .unwrap()
            .is_empty(),
        "nothing landed"
    );
}

#[test]
fn a_new_video_can_bring_its_storyboard_in_the_package() {
    let (conn, _) = workspace();
    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "title": "Winter road — the clip", "kind": "video",
            "versions": [{ "role": "plot", "body": "a road, a car, a light" }],
            "scenes": [{ "section": "intro", "shot_type": "wide", "description": "the road" }]
        })),
    )
    .unwrap();
    assert!(answer.contains("1 scene"), "{answer}");

    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, None).unwrap();
    let chat = chats.iter().find(|c| c.work_id.is_none()).unwrap();
    let message = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    assert_eq!(message.meta["proposal"]["scenes"][0]["section"], "intro");
    assert!(message.body.contains("### Scenes"), "{}", message.body);

    let err = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({ "title": "x", "kind": "song", "scenes": [{ "description": "x" }] })),
    )
    .unwrap_err();
    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("scene.noStoryboard"),
        "a song takes no scenes: {err}"
    );
}

#[test]
fn a_package_can_plan_a_release_and_say_what_it_goes_out_as() {
    let (conn, _) = workspace();

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "title": "Winter road — the clip", "kind": "video",
            "versions": [{ "role": "plot", "body": "a road, a car, a light" }],
            "releases": [{
                "kind": "youtube",
                "scheduled_at": "2026-10-02",
                "fields": {
                    "title": "Winter road",
                    "description": "a road, a car, a light",
                    "tags": "winter, road, night",
                },
            }],
        })),
    )
    .unwrap();

    assert!(answer.contains("1 release (youtube)"), "{answer}");

    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, None).unwrap();
    let chat = chats.iter().find(|c| c.work_id.is_none()).unwrap();
    let message = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);

    assert_eq!(message.meta["proposal"]["releases"][0]["kind"], "youtube");
    assert_eq!(
        message.meta["proposal"]["releases"][0]["fields"]["title"],
        "Winter road"
    );
    assert!(
        message.body.contains("### YouTube — 2026-10-02"),
        "the kind's own word and its day, in what the person reads before applying: {}",
        message.body
    );
    assert!(message.body.contains("Winter road"), "{}", message.body);
}

#[test]
fn a_release_of_a_kind_this_work_does_not_ship_refuses_the_package() {
    let (conn, _) = workspace();

    let err = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "title": "Winter road", "kind": "song",
            "releases": [{ "kind": "youtube" }],
        })),
    )
    .unwrap_err();

    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("release.noDoors"),
        "a song ships nothing itself - its clips, audio and shorts do: {err}"
    );
}

#[test]
fn a_release_field_the_kind_does_not_have_is_named_and_left_out() {
    let (conn, _) = workspace();

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "title": "Winter road — the clip", "kind": "video",
            "releases": [{
                "kind": "premiere",
                // `premiere` ships a title and a description, not tags.
                "fields": { "title": "Winter road", "tags": "winter" },
            }],
        })),
    )
    .unwrap();

    assert!(
        answer.contains("`premiere` fields ignored (not of that kind of release): tags"),
        "a package that only half fits is worth applying, but not silently: {answer}"
    );

    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, None).unwrap();
    let chat = chats.iter().find(|c| c.work_id.is_none()).unwrap();
    let message = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    assert!(
        message.meta["proposal"]["releases"][0]["fields"]
            .get("tags")
            .is_none(),
        "a value under a key no box will ever show is not stored"
    );
}

#[test]
fn a_day_that_is_not_a_day_is_refused() {
    let (conn, _) = workspace();

    let err = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "title": "Winter road — the clip", "kind": "video",
            "releases": [{ "kind": "youtube", "scheduled_at": "next Thursday" }],
        })),
    )
    .unwrap_err();

    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("mcp.badReleaseDate"),
        "{err}"
    );
}

#[test]
fn a_package_holding_only_a_release_is_not_empty() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let work = crate::work::create(
        &conn,
        &profile_id,
        crate::work::NewWork {
            kind: "video".into(),
            title: "Winter road — the clip".into(),
            ..crate::work::NewWork::default()
        },
    )
    .unwrap();

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "work": work.id,
            "releases": [{ "kind": "youtube" }],
        })),
    )
    .unwrap();

    assert!(answer.contains("1 release"), "{answer}");
}

#[test]
fn an_agent_reads_the_lines_of_a_vibe_proposes_one_and_finds_works_by_it() {
    let (conn, work_id) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;

    let said = run_tool(&conn, &claude(), "workspace", &Map::new()).unwrap();
    let said: Value = serde_json::from_str(&said).unwrap();
    assert_eq!(said["vibe"]["moods"].as_array().unwrap().len(), 5);
    assert_eq!(said["vibe"]["complexities"][2]["strokes"], 3);

    // A word the profile does not have is refused, with the ones it has.
    let refused = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({ "work": work_id, "vibe": { "mood": "sunny" } })),
    )
    .unwrap_err();
    assert_eq!(refused.refusal().map(|r| r.code), Some("work.unknownVibe"));

    // A vibe alone is a package worth proposing; kept, it is the work's.
    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({ "work": work_id, "vibe": { "mood": "dark", "character": "edgy" } })),
    )
    .unwrap();
    assert!(answer.contains("Proposed"), "{answer}");
    let message = first_message(&conn, &work_id);
    assert!(message.body.contains("Dark · Raw"), "{}", message.body);
    crate::actions::proposal::apply(
        &conn,
        &message.id,
        crate::actions::proposal::Overrides::default(),
    )
    .unwrap();

    let catalogue = |filter: Value| -> Value {
        serde_json::from_str(&run_tool(&conn, &claude(), "catalogue", &args(filter)).unwrap())
            .unwrap()
    };
    let found = catalogue(json!({ "mood": "dark" }));
    assert_eq!(found["works"].as_array().unwrap().len(), 1);
    assert_eq!(found["works"][0]["vibe"]["vibe"]["character"], "edgy");
    assert_eq!(
        catalogue(json!({ "mood": "joyful" }))["works"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    let work: Value = serde_json::from_str(
        &run_tool(&conn, &claude(), "work", &args(json!({ "work": work_id }))).unwrap(),
    )
    .unwrap();
    assert_eq!(work["vibe"]["mood"], "dark");
    assert_eq!(work["shown_vibe"]["vibe"]["mood"], "dark");
    let _ = profile_id;
}

#[test]
fn workspace_gives_the_axes_whole_and_the_skills_without_their_methods() {
    let (conn, _) = workspace();
    let said: Value =
        serde_json::from_str(&run_tool(&conn, &claude(), "workspace", &Map::new()).unwrap())
            .unwrap();
    let song = said["work_kinds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["key"] == "song")
        .unwrap();
    assert!(
        song["axes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["rubric"].as_array().is_some_and(|r| !r.is_empty())),
        "{}",
        song["axes"]
    );
    let guides = said["guides"].as_array().unwrap();
    let score = guides.iter().find(|g| g["key"] == "score").unwrap();
    assert_eq!(score["produces"], "score");
    assert_eq!(score["has_method"], true);
    assert!(score.get("method").is_none(), "the index carries no method");
    assert!(score.get("template").is_none());
}

#[test]
fn guide_lists_the_skills_and_opens_one_with_the_live_context() {
    let (conn, _) = workspace();
    let catalogue: Value = serde_json::from_str(
        &run_tool(
            &conn,
            &claude(),
            "guide",
            &args(json!({ "scope": "scene" })),
        )
        .unwrap(),
    )
    .unwrap();
    let listed = catalogue["guides"].as_array().unwrap();
    assert!(!listed.is_empty());
    assert!(listed.iter().all(|g| g["scope"] == "scene"));

    let opened: Value = serde_json::from_str(
        &run_tool(
            &conn,
            &claude(),
            "guide",
            &args(json!({ "skill": "score", "kind": "song" })),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(opened["method"].as_str().is_some_and(|m| m.len() > 100));
    assert_eq!(
        opened["answer_with"],
        "`propose_score`, along the kind's axes"
    );
    let kinds = opened["workspace_context"]["kinds"].as_array().unwrap();
    assert_eq!(kinds.len(), 1);
    assert!(kinds[0]["axes"][0]["rubric"].is_array());
    assert!(
        kinds[0]["statuses"]
            .as_array()
            .is_some_and(|s| !s.is_empty())
    );

    let refused = run_tool(
        &conn,
        &claude(),
        "guide",
        &args(json!({ "skill": "nothing" })),
    )
    .unwrap_err();
    assert_eq!(refused.refusal().map(|r| r.code), Some("mcp.unknownSkill"));
}

#[test]
fn the_skills_are_served_as_prompts_filled_from_a_work_when_one_is_named() {
    let (conn, work_id) = workspace();
    let mut session = Session::default();
    let listed = handle(
        &conn,
        &mut session,
        &json!({ "jsonrpc": "2.0", "id": 1, "method": "prompts/list" }),
    )
    .unwrap();
    let prompts = listed["result"]["prompts"].as_array().unwrap();
    let critique = prompts.iter().find(|p| p["name"] == "critique").unwrap();
    assert_eq!(critique["arguments"][0]["name"], "work");

    let got = handle(
        &conn,
        &mut session,
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "prompts/get",
                 "params": { "name": "critique", "arguments": { "work": work_id } } }),
    )
    .unwrap();
    let text = got["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap();
    assert!(
        text.contains("Harbour lights"),
        "the message is filled: {text}"
    );
    assert!(text.contains("---"), "the method comes first: {text}");

    let bare = handle(
        &conn,
        &mut session,
        &json!({ "jsonrpc": "2.0", "id": 3, "method": "prompts/get",
                 "params": { "name": "critique" } }),
    )
    .unwrap();
    let text = bare["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap();
    assert!(text.contains("the work the person names"), "{text}");

    let unknown = handle(
        &conn,
        &mut session,
        &json!({ "jsonrpc": "2.0", "id": 4, "method": "prompts/get",
                 "params": { "name": "nothing" } }),
    )
    .unwrap();
    assert_eq!(unknown["error"]["code"], INVALID_PARAMS);
}

fn args(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// `propose_note` changes a note that is there and proposes several at
/// once (FR-08 8.2): a change names its note by id or exact title and is
/// shown against what the note holds; the notes are kept one by one, and
/// the change writes only what it says.
#[test]
fn notes_are_proposed_several_at_once_and_a_note_is_changed() {
    let (conn, work_id) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let kept = note::create(
        &conn,
        &profile_id,
        note::NewNote {
            body: "the road as a witness".into(),
            title: Some("Concept".into()),
            tags: vec!["idea".into()],
            work_id: Some(work_id.clone()),
            ..note::NewNote::default()
        },
    )
    .unwrap();

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_note",
        &args(json!({
            "work": work_id,
            "notes": [
                { "note": "Concept", "body": "the road as a witness, and a judge", "tags": ["idea", "#road"] },
                { "title": "Bridge", "body": "try a key change", "tags": ["music"], "layer": "internal" },
                { "title": "Left out", "body": "not this one" }
            ],
        })),
    )
    .unwrap();
    assert!(
        answer.contains("Proposed 3 notes (1 of them a change"),
        "{answer}"
    );

    let message = first_message(&conn, &work_id);
    let proposal = &message.meta["proposal"];
    assert_eq!(proposal["kind"], "notes");
    // The change carries what the note held, for was against becomes.
    assert_eq!(proposal["notes"][0]["note"], json!(kept.id));
    assert_eq!(
        proposal["notes"][0]["before"]["body"],
        "the road as a witness"
    );
    assert_eq!(proposal["notes"][0]["tags"], json!(["idea", "road"]));
    assert!(
        message.body.contains("tags: idea → idea, road"),
        "{}",
        message.body
    );
    assert!(
        message.body.contains("the text is rewritten"),
        "{}",
        message.body
    );
    // Nothing is written until it is kept.
    assert_eq!(
        note::get(&conn, &kept.id).unwrap().unwrap().body,
        "the road as a witness"
    );

    let outcome = crate::actions::proposal::apply(
        &conn,
        &message.id,
        crate::actions::proposal::Overrides {
            items: Some(vec!["note:0".into(), "note:1".into()]),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(outcome.notes.len(), 2);

    let changed = note::get(&conn, &kept.id).unwrap().unwrap();
    assert_eq!(changed.body, "the road as a witness, and a judge");
    assert_eq!(changed.tags, ["idea", "road"]);
    assert_eq!(
        changed.title.as_deref(),
        Some("Concept"),
        "the title was not said"
    );
    let all = note::list(&conn, &profile_id, &NoteFilter::default()).unwrap();
    let bridge = all
        .iter()
        .find(|one| one.title.as_deref() == Some("Bridge"))
        .unwrap();
    assert_eq!(bridge.tags, ["music"]);
    assert_eq!(bridge.layer, crate::canon::Layer::Internal);
    assert_eq!(bridge.work_id.as_deref(), Some(work_id.as_str()));
    assert!(
        !all.iter()
            .any(|one| one.title.as_deref() == Some("Left out")),
        "a note that was not taken was written"
    );
    assert_eq!(all.len(), 2, "the change made a second note");

    // A note nobody can find is said, not made.
    let unknown = run_tool(
        &conn,
        &claude(),
        "propose_note",
        &args(json!({ "note": "No such note", "body": "x" })),
    );
    assert_eq!(unknown.unwrap_err().reason().key, "refusal.mcp.unknownNote");
}

/// A tool's answer, read back as the JSON it is.
fn read(conn: &Connection, name: &str, with: Value) -> Value {
    serde_json::from_str(&run_tool(conn, &claude(), name, &args(with)).unwrap()).unwrap()
}

/// The long readings come a page at a time (FR-08 8.4): how many there
/// are, whether more follow, and the next page by its offset - so an
/// agent walks a record of thousands rather than swallowing it.
#[test]
fn a_long_reading_comes_a_page_at_a_time() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    for n in 1..=6 {
        fixtures::song(&conn, &profile_id, &format!("Song {n}"));
    }

    let first = read(&conn, "catalogue", json!({ "limit": 3 }));
    assert_eq!(first["total"], 7);
    assert_eq!(
        (first["limit"].clone(), first["offset"].clone()),
        (json!(3), json!(0))
    );
    assert_eq!(first["has_more"], true);
    assert_eq!(first["works"].as_array().unwrap().len(), 3);

    let last = read(&conn, "catalogue", json!({ "limit": 3, "offset": 6 }));
    assert_eq!(last["works"].as_array().unwrap().len(), 1);
    assert_eq!(last["has_more"], false);

    // Every work once across the pages, none twice.
    let mut seen: Vec<String> = Vec::new();
    for offset in [0, 3, 6] {
        let page = read(&conn, "catalogue", json!({ "limit": 3, "offset": offset }));
        seen.extend(
            page["works"]
                .as_array()
                .unwrap()
                .iter()
                .map(|work| work["id"].as_str().unwrap().to_owned()),
        );
    }
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 7);

    // A filter narrows what is counted, not only what is shown.
    let narrowed = read(&conn, "catalogue", json!({ "query": "song 1" }));
    assert_eq!(narrowed["total"], 1);
    // And a page can ask for no more than the most.
    let greedy = read(&conn, "catalogue", json!({ "limit": 100_000 }));
    assert_eq!(greedy["limit"], MOST);
}

/// A summary leaves the long texts out and says how long they are; one
/// entry is then read whole by its id or its name.
#[test]
fn a_summary_leaves_the_texts_out_and_one_entry_is_read_whole() {
    let (conn, work_id) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let body = "a long thought about harbours\n".repeat(40);
    let kept = note::create(
        &conn,
        &profile_id,
        note::NewNote {
            body: body.clone(),
            title: Some("On harbours".into()),
            tags: vec!["sea".into()],
            ..note::NewNote::default()
        },
    )
    .unwrap();
    note::create(
        &conn,
        &profile_id,
        note::NewNote {
            body: "no title here, only a first line\nand a second".into(),
            ..note::NewNote::default()
        },
    )
    .unwrap();

    let short = read(&conn, "notes", json!({ "summary": true }));
    assert_eq!(short["total"], 2);
    let listed = short["notes"].as_array().unwrap();
    let harbours = listed
        .iter()
        .find(|one| one["id"] == json!(kept.id))
        .unwrap();
    assert!(harbours.get("body").is_none(), "a summary carries a body");
    assert_eq!(harbours["title"], "On harbours");
    assert_eq!(harbours["length"], body.chars().count());
    assert_eq!(harbours["tags"], json!(["sea"]));
    let untitled = listed
        .iter()
        .find(|one| one["id"] != json!(kept.id))
        .unwrap();
    assert_eq!(untitled["title"], "no title here, only a first line");
    // With no question asked, the shelves are still said.
    assert!(short.get("shelves").is_some());

    // Whole, by id and by its exact title.
    for named in [kept.id.as_str(), "On harbours"] {
        let whole = read(&conn, "notes", json!({ "note": named }));
        assert_eq!(whole["id"], json!(kept.id));
        assert_eq!(whole["body"], json!(body));
    }
    let unknown = run_tool(
        &conn,
        &claude(),
        "notes",
        &args(json!({ "note": "No such" })),
    );
    assert_eq!(unknown.unwrap_err().reason().key, "refusal.mcp.unknownNote");

    // The register: a word short, then whole with the works that carry it.
    let term = crate::register::create(
        &conn,
        &profile_id,
        crate::register::NewTerm {
            word: "lines".into(),
            note: Some("the song's own word".into()),
            ..crate::register::NewTerm::default()
        },
    )
    .unwrap();
    let record = read(&conn, "register", json!({ "summary": true }));
    assert_eq!(record["total"], 1);
    let word = &record["terms"][0];
    assert_eq!(word["word"], "lines");
    assert!(word.get("note").is_none() && word.get("forms").is_none());
    for named in [term.id.as_str(), "Lines"] {
        let whole = read(&conn, "register", json!({ "term": named }));
        assert_eq!(whole["note"], "the song's own word");
        assert_eq!(
            whole["works"][0]["work_id"],
            json!(work_id),
            "the work whose text says it: {whole}"
        );
    }
}

#[test]
fn initialize_names_the_protocol_the_build_and_remembers_the_client() {
    let (conn, _) = workspace();
    let mut session = Session::default();
    let response = handle(
        &conn,
        &mut session,
        &json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
                 "params": { "clientInfo": { "name": "Claude Code", "version": "1" } } }),
    )
    .unwrap();
    assert_eq!(response["result"]["protocolVersion"], PROTOCOL_VERSION);
    assert_eq!(
        response["result"]["serverInfo"]["version"],
        env!("CARGO_PKG_VERSION")
    );
    assert!(response["result"]["capabilities"]["tools"].is_object());
    assert_eq!(
        response["result"]["capabilities"]["prompts"]["listChanged"],
        false
    );
    let said = response["result"]["instructions"].as_str().unwrap();
    assert!(
        said.contains("`guide`") && said.contains("never in your own files"),
        "{said}"
    );
    assert_eq!(session.client.as_deref(), Some("Claude Code"));
}

#[test]
fn a_tool_and_a_prompt_are_answered_from_a_space_and_the_rest_from_none() {
    for method in ["tools/call", "prompts/list", "prompts/get"] {
        assert!(reads_a_space(Some(method)), "{method}");
    }
    for method in [
        "initialize",
        "ping",
        "tools/list",
        "notifications/initialized",
    ] {
        assert!(!reads_a_space(Some(method)), "{method}");
    }
    assert!(!reads_a_space(None));
}

#[test]
fn a_notification_is_not_answered() {
    let (conn, _) = workspace();
    let mut session = Session::default();
    let note = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
    assert!(handle(&conn, &mut session, &note).is_none());
}

#[test]
fn an_unknown_method_is_a_protocol_error_not_a_crash() {
    let (conn, _) = workspace();
    let mut session = Session::default();
    let response = handle(
        &conn,
        &mut session,
        &json!({ "jsonrpc": "2.0", "id": 1, "method": "kilna/nothing" }),
    )
    .unwrap();
    assert_eq!(response["error"]["code"], METHOD_NOT_FOUND);
    // The prompts are served now; resources are not.
    let resources = handle(
        &conn,
        &mut session,
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "resources/list" }),
    )
    .unwrap();
    assert_eq!(resources["error"]["code"], METHOD_NOT_FOUND);
}

#[test]
fn every_tool_describes_what_it_requires() {
    for tool in tools() {
        let schema = &tool["inputSchema"];
        for name in schema["required"].as_array().unwrap() {
            let name = name.as_str().unwrap();
            assert!(
                schema["properties"].get(name).is_some(),
                "{} requires `{name}` without describing it",
                tool["name"]
            );
        }
        assert!(
            tool["description"].as_str().is_some_and(|d| d.len() > 60),
            "{} needs a description a model can act on",
            tool["name"]
        );
    }
}

#[test]
fn a_work_is_found_by_id_or_by_exact_title_and_read_with_its_text() {
    let (conn, work_id) = workspace();
    let by_title = run_tool(
        &conn,
        &claude(),
        "work",
        &args(json!({ "work": "Harbour lights" })),
    )
    .unwrap();
    let by_id = run_tool(&conn, &claude(), "work", &args(json!({ "work": work_id }))).unwrap();
    assert_eq!(by_title, by_id);
    let card: Value = serde_json::from_str(&by_id).unwrap();
    assert_eq!(card["versions"]["lyrics"][0]["revision"], 1);
    assert!(card.get("body").is_none(), "the card must not carry bodies");

    let text = run_tool(&conn, &claude(), "text", &args(json!({ "work": work_id }))).unwrap();
    let text: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(text["body"], "one line\ntwo lines");
    assert_eq!(text["reads_as"], "plain");
}

#[test]
fn a_missing_work_answers_the_agent_rather_than_the_client() {
    let (conn, _) = workspace();
    let call = json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/call",
                       "params": { "name": "text", "arguments": { "work": "nowhere" } } });
    let response = handle(&conn, &mut claude(), &call).unwrap();
    assert!(response["error"].is_null(), "{response}");
    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("nowhere"), "{text}");
}

/// `text` reads a version by its id only when it is the named work's: a
/// version of another work is refused, never handed over under the wrong
/// title - and a role the kind lacks is refused by name.
#[test]
fn a_version_is_read_by_id_only_from_its_own_work() {
    let (conn, work_id) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let other = fixtures::song(&conn, &profile_id, "Night ferry").id;
    let first = fixtures::version(&conn, &work_id, "lyrics", "an older text");
    let foreign = fixtures::version(&conn, &other, "lyrics", "somebody else's text");

    let own = read(
        &conn,
        "text",
        json!({ "work": work_id, "version": first.id }),
    );
    assert_eq!(own["version"], json!(first.id));
    assert_eq!(own["body"], "an older text");

    let refused = run_tool(
        &conn,
        &claude(),
        "text",
        &args(json!({ "work": work_id, "version": foreign.id })),
    )
    .unwrap_err();
    assert_eq!(refused.refusal().unwrap().code, "mcp.versionOtherWork");

    let refused = run_tool(
        &conn,
        &claude(),
        "text",
        &args(json!({ "work": work_id, "role": "sonnet" })),
    )
    .unwrap_err();
    assert_eq!(refused.refusal().unwrap().code, "mcp.unknownVersionRole");
}

/// The catalogue is narrowed by a substring of the title, by kind and by
/// status, each leaving only the works that match.
#[test]
fn the_catalogue_is_narrowed_by_title_kind_and_status() {
    let (conn, song) = workspace();
    let clip = video(&conn);
    let ids = |with: Value| -> Vec<String> {
        read(&conn, "catalogue", with)["works"]
            .as_array()
            .unwrap()
            .iter()
            .map(|work| work["id"].as_str().unwrap().to_owned())
            .collect()
    };
    let all = read(&conn, "catalogue", json!({}));
    assert_eq!(all["total"], 2);
    let status = all["works"]
        .as_array()
        .unwrap()
        .iter()
        .find(|work| work["id"] == json!(song))
        .unwrap()["status"]
        .as_str()
        .unwrap()
        .to_owned();

    assert_eq!(ids(json!({ "kind": "song" })), [song.clone()]);
    assert_eq!(ids(json!({ "kind": "video" })), [clip.clone()]);
    assert_eq!(ids(json!({ "query": "THE CLIP" })), [clip]);
    assert!(ids(json!({ "kind": "song", "status": status })).contains(&song));
    assert!(ids(json!({ "status": "no-such-status" })).is_empty());
    assert!(ids(json!({ "kind": "no-such-kind" })).is_empty());
}

/// `notes` narrowed by a state gives only the notes standing in it, and a
/// word that is no state is refused rather than read as "all".
#[test]
fn notes_are_narrowed_by_the_state_they_stand_in() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let note = |body: &str, state: Option<note::NoteState>| {
        note::create(
            &conn,
            &profile_id,
            note::NewNote {
                body: body.into(),
                state,
                ..note::NewNote::default()
            },
        )
        .unwrap()
        .id
    };
    let fresh = note("a thought still fresh", None);
    let parked = note("a thought put aside", note::NoteState::from_word("parked"));
    let ids = |with: Value| -> Vec<String> {
        read(&conn, "notes", with)["notes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|note| note["id"].as_str().unwrap().to_owned())
            .collect()
    };

    assert_eq!(ids(json!({ "state": "fresh" })), [fresh.clone()]);
    assert_eq!(ids(json!({ "state": "parked" })), [parked.clone()]);
    let both = ids(json!({}));
    assert!(both.contains(&fresh) && both.contains(&parked), "{both:?}");

    let refused = run_tool(
        &conn,
        &claude(),
        "notes",
        &args(json!({ "state": "somewhere" })),
    )
    .unwrap_err();
    assert_eq!(refused.refusal().unwrap().code, "mcp.unknownNoteState");
}

#[test]
fn a_proposed_version_is_a_message_in_the_clients_chat_not_a_version() {
    let (conn, work_id) = workspace();
    let before = version::list(&conn, &work_id).unwrap().len();

    run_tool(
        &conn,
        &claude(),
        "propose_version",
        &args(json!({ "work": work_id, "role": "lyrics", "body": "one line\ntwo lines\nthree", "note": "added a third" })),
    )
    .unwrap();

    assert_eq!(
        version::list(&conn, &work_id).unwrap().len(),
        before,
        "a proposal is not a version"
    );
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, Some(&work_id)).unwrap();
    assert_eq!(chats.len(), 1);
    assert_eq!(chats[0].title.as_deref(), Some("Claude Code"));
    let transcript = assistant::transcript(&conn, &chats[0].id).unwrap().unwrap();
    let message = &transcript.messages[0];
    assert_eq!(message.role, "assistant");
    assert_eq!(
        message.body, "one line\ntwo lines\nthree",
        "the body is the text, verbatim"
    );
    assert_eq!(message.meta["proposal"]["kind"], "version");
    assert_eq!(message.meta["proposal"]["role"], "lyrics");
    assert_eq!(message.meta["note"], "added a third");
    assert_eq!(message.meta["source"], "mcp");
}

/// A style named by its bricks is written by kilna, as the constructor
/// writes it: the phrases in the order named, the song's fields closing
/// the line - and a body beside the bricks is two answers, refused.
#[test]
fn a_style_proposed_by_its_bricks_is_written_by_kilna() {
    let (conn, work_id) = workspace();
    crate::style_set::seed(&conn).unwrap();
    let mut meta = serde_json::Map::new();
    meta.insert("bpm".into(), json!(84));
    crate::work::update(
        &conn,
        &work_id,
        crate::work::WorkPatch {
            meta: Some(meta),
            ..Default::default()
        },
    )
    .unwrap();

    let said = run_tool(
        &conn,
        &claude(),
        "propose_version",
        &args(json!({
            "work": work_id,
            "role": "style",
            "bricks": ["Breathy whisper vocal", "dream pop", "chopped amen break"],
        })),
    )
    .unwrap();
    assert!(said.contains("Proposed a `style` version"), "{said}");
    assert!(
        said.contains("`instrument`: 0 picked"),
        "a block short is said: {said}"
    );

    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, Some(&work_id)).unwrap();
    let transcript = assistant::transcript(&conn, &chats[0].id).unwrap().unwrap();
    assert_eq!(
        transcript.messages[0].body,
        "breathy whisper vocal, dream pop, chopped amen break, 84 bpm"
    );

    let both = run_tool(
        &conn,
        &claude(),
        "propose_version",
        &args(json!({ "work": work_id, "role": "style", "body": "x", "bricks": ["dream pop"] })),
    )
    .unwrap_err();
    assert_eq!(both.refusal().map(|r| r.code), Some("mcp.bodyAndBricks"));
    let lyric = run_tool(
        &conn,
        &claude(),
        "propose_version",
        &args(json!({ "work": work_id, "role": "lyrics", "bricks": ["dream pop"] })),
    )
    .unwrap_err();
    assert_eq!(lyric.refusal().map(|r| r.code), Some("mcp.roleNotComposed"));
    let unknown = run_tool(
        &conn,
        &claude(),
        "propose_version",
        &args(json!({ "work": work_id, "role": "style", "bricks": ["a sound nobody named"] })),
    )
    .unwrap_err();
    assert_eq!(unknown.refusal().map(|r| r.code), Some("mcp.unknownBrick"));
}

#[test]
fn styles_are_proposed_with_their_contracts_and_kept_by_the_person() {
    let (conn, _) = workspace();
    let answer = run_tool(
        &conn,
        &claude(),
        "propose_style",
        &args(json!({ "styles": [
            { "type": "pose", "name": "Arms crossed", "description": "arms crossed",
              "contract": { "needs": ["subject:person", "crop:min:waist"] } },
            { "type": "nothing-of-the-kind", "name": "X" }
        ]})),
    )
    .unwrap();
    assert!(
        answer.contains("Proposed 1 style") && answer.contains("1 left out"),
        "{answer}"
    );
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chat = assistant::summaries(&conn, &profile_id, None)
        .unwrap()
        .into_iter()
        .find(|c| c.work_id.is_none())
        .unwrap();
    let proposed = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    assert!(
        proposed.body.contains("**Arms crossed** (pose)"),
        "{}",
        proposed.body
    );
    let outcome = crate::actions::proposal::apply(&conn, &proposed.id, Default::default()).unwrap();
    let made = crate::style_brick::get(&conn, &outcome.style_bricks[0])
        .unwrap()
        .unwrap();
    assert_eq!(made.contract.needs, ["subject:person", "crop:min:waist"]);
}

#[test]
fn the_dictionary_says_each_brick_s_branch_and_is_read_by_it() {
    let (conn, _) = workspace();
    crate::style_set::seed(&conn).unwrap();
    let summary: Value =
        serde_json::from_str(&run_tool(&conn, &claude(), "styles", &args(json!({}))).unwrap())
            .unwrap();
    let branch_of = |key: &str| {
        summary["types"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["type"] == key)
            .map(|t| t["branch"].clone())
            .unwrap()
    };
    assert_eq!(branch_of("image-style"), "picture");
    assert_eq!(branch_of("motion-camera"), "motion");
    assert_eq!(branch_of("groove"), "sound");

    let motion: Value = serde_json::from_str(
        &run_tool(
            &conn,
            &claude(),
            "styles",
            &args(json!({ "branch": "motion" })),
        )
        .unwrap(),
    )
    .unwrap();
    let rows = motion.as_array().unwrap();
    assert_eq!(rows.len(), 20, "the four types of movement, five each");
    assert!(rows.iter().all(
        |row| row["branch"] == "motion" && row["type"].as_str().unwrap().starts_with("motion-")
    ));
}

#[test]
fn the_dictionary_is_read_by_type_by_composition_and_by_meaning() {
    let (conn, _) = workspace();
    crate::style_set::seed(&conn).unwrap();

    let summary: Value =
        serde_json::from_str(&run_tool(&conn, &claude(), "styles", &args(json!({}))).unwrap())
            .unwrap();
    assert!(
        summary["types"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["type"] == "groove"
                && t["form"] == "phrase"
                && t["bricks"].as_u64().unwrap() > 20)
    );
    assert_eq!(summary["compose"][0]["key"], "sound");

    let found: Value = serde_json::from_str(
        &run_tool(
            &conn,
            &claude(),
            "styles",
            &args(json!({ "query": "барабан" })),
        )
        .unwrap(),
    )
    .unwrap();
    let found = found.as_array().unwrap();
    assert!(
        !found.is_empty(),
        "a Russian word finds the phrases it explains"
    );
    assert!(found.iter().all(|b| {
        b["explanation"]["ru"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase()
            .contains("барабан")
            || b["name"].as_str().unwrap_or_default().contains("барабан")
    }));

    let sound: Value = serde_json::from_str(
        &run_tool(
            &conn,
            &claude(),
            "styles",
            &args(json!({ "composition": "sound" })),
        )
        .unwrap(),
    )
    .unwrap();
    let sound = sound.as_array().unwrap();
    assert_eq!(sound[0]["type"], "genre", "in the composition's order");
    assert!(sound.iter().all(|b| b["phrase"].is_string()));

    let workspace: Value =
        serde_json::from_str(&run_tool(&conn, &claude(), "workspace", &args(json!({}))).unwrap())
            .unwrap();
    assert_eq!(workspace["compose"][0]["role"], "style");
    assert!(
        workspace["style_types"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["key"] == "knob" && t["form"] == "phrase")
    );
}

/// The version an agent says it rewrote travels with the proposal, and
/// the work's card names each version's parent; a version of another
/// role is refused now, in words, rather than dropped at apply time.
#[test]
fn a_proposed_version_names_what_it_was_written_from() {
    let (conn, work_id) = workspace();
    let lyric = version::latest(&conn, &work_id, "lyrics")
        .unwrap()
        .unwrap()
        .id;

    run_tool(
        &conn,
        &claude(),
        "propose_version",
        &args(json!({ "work": work_id, "role": "lyrics", "body": "sharper", "from": lyric })),
    )
    .unwrap();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, Some(&work_id)).unwrap();
    let transcript = assistant::transcript(&conn, &chats[0].id).unwrap().unwrap();
    assert_eq!(
        transcript.messages[0].meta["proposal"]["from"],
        lyric.as_str()
    );

    let refused = run_tool(
        &conn,
        &claude(),
        "propose_version",
        &args(json!({ "work": work_id, "role": "style", "body": "warm", "from": lyric })),
    )
    .unwrap_err();
    assert_eq!(
        refused.refusal().map(|r| r.code),
        Some("version.parentNotSameRole"),
        "{refused}"
    );

    let card = run_tool(&conn, &claude(), "work", &args(json!({ "work": work_id }))).unwrap();
    let card: Value = serde_json::from_str(&card).unwrap();
    assert!(card["versions"]["lyrics"][0]["parent"].is_null());
}

#[test]
fn a_second_proposal_joins_the_same_chat() {
    let (conn, work_id) = workspace();
    for body in ["first", "second"] {
        run_tool(
            &conn,
            &claude(),
            "propose_version",
            &args(json!({ "work": work_id, "role": "lyrics", "body": body })),
        )
        .unwrap();
    }
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, Some(&work_id)).unwrap();
    assert_eq!(chats.len(), 1, "one chat per client per work");
    let transcript = assistant::transcript(&conn, &chats[0].id).unwrap().unwrap();
    assert_eq!(transcript.messages.len(), 2);
}

#[test]
fn a_proposed_score_is_checked_against_the_profile_and_never_written() {
    let (conn, work_id) = workspace();
    let answer = run_tool(
        &conn,
        &claude(),
        "propose_score",
        &args(json!({ "work": work_id, "axes": { "hook": 7, "lyrics": 11, "sparkle": 3 }, "note": "strong chorus" })),
    )
    .unwrap();
    assert!(
        answer.contains("sparkle"),
        "unknown axes are named: {answer}"
    );
    assert!(
        score::history(&conn, &work_id).unwrap().is_empty(),
        "a proposal is not a score"
    );

    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chat = assistant::summaries(&conn, &profile_id, Some(&work_id))
        .unwrap()
        .remove(0);
    let message = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    assert_eq!(message.meta["proposal"]["kind"], "score");
    assert_eq!(message.meta["proposal"]["axes"]["hook"], 7.0);
    assert_eq!(
        message.meta["proposal"]["axes"]["lyrics"], 10.0,
        "clamped to the scale"
    );
    assert_eq!(message.meta["proposal"]["unknown"][0], "sparkle");
}

#[test]
fn a_score_naming_no_known_axis_is_refused() {
    let (conn, work_id) = workspace();
    let err = run_tool(
        &conn,
        &claude(),
        "propose_score",
        &args(json!({ "work": work_id, "axes": { "sparkle": 3 } })),
    )
    .unwrap_err();
    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("mcp.noKnownAxes"),
        "{err}"
    );
}

#[test]
fn a_proposal_leaves_a_line_in_the_journal() {
    let (conn, work_id) = workspace();
    run_tool(
        &conn,
        &claude(),
        "propose_note",
        &args(json!({ "work": work_id, "title": "Bridge idea", "body": "try a key change" })),
    )
    .unwrap();
    let lines: i64 = conn
        .query_row(
            "SELECT count(*) FROM journal WHERE action = 'proposal.note' AND entity_id = ?1",
            [&work_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(lines, 1);
}

#[test]
fn a_proposal_from_an_unnamed_client_still_lands_somewhere() {
    let (conn, work_id) = workspace();
    run_tool(
        &conn,
        &Session::default(),
        "propose_note",
        &args(json!({ "work": work_id, "body": "anonymous" })),
    )
    .unwrap();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chat = assistant::summaries(&conn, &profile_id, Some(&work_id))
        .unwrap()
        .remove(0);
    assert_eq!(chat.title.as_deref(), Some(UNNAMED_CLIENT));
}

#[test]
fn a_proposed_work_waits_in_the_clients_chat_on_nothing_with_everything_readable() {
    let (conn, _) = workspace();
    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "title": "Winter road", "kind": "song",
            "fields": { "premise": "a road at night", "colour": "blue" },
            "versions": [{ "role": "lyrics", "body": "snow on the road", "label": "first" }],
            "score": { "axes": { "hook": 7, "sparkle": 2 }, "note": "quiet" },
            "notes": [{ "title": "Concept", "body": "the road as witness" }]
        })),
    )
    .unwrap();
    assert!(answer.contains("Proposed a new work"), "{answer}");
    assert!(
        answer.contains("colour"),
        "ignored fields are named: {answer}"
    );
    assert!(
        answer.contains("sparkle"),
        "ignored axes are named: {answer}"
    );

    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    assert_eq!(
        work::list(&conn, &profile_id, &WorkFilter::default())
            .unwrap()
            .len(),
        1,
        "a proposal is not a work"
    );
    let chats = assistant::summaries(&conn, &profile_id, None).unwrap();
    let chat = chats
        .iter()
        .find(|c| c.work_id.is_none())
        .expect("a chat on nothing");
    assert_eq!(chat.title.as_deref(), Some("Claude Code"));
    let message = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    assert_eq!(message.meta["proposal"]["kind"], "work");
    assert_eq!(message.meta["proposal"]["title"], "Winter road");
    assert_eq!(
        message.meta["proposal"]["fields"]["premise"],
        "a road at night"
    );
    assert!(message.meta["proposal"]["fields"].get("colour").is_none());
    assert_eq!(
        message.meta["proposal"]["versions"][0]["body"],
        "snow on the road"
    );
    assert_eq!(message.meta["proposal"]["score"]["axes"]["hook"], 7.0);
    assert!(
        message.body.contains("snow on the road"),
        "readable before applied: {}",
        message.body
    );
    assert!(message.body.contains("Winter road"), "{}", message.body);
    assert!(apply::is_pending(&message));

    let lines: i64 = conn
        .query_row(
            "SELECT count(*) FROM journal WHERE action = 'proposal.work'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(lines, 1);
}

#[test]
fn a_package_for_a_work_joins_its_chat() {
    let (conn, work_id) = workspace();
    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(
            json!({ "work": work_id, "title": "ignored", "kind": "video",
                       "versions": [{ "role": "style", "body": "brushed drums" }] }),
        ),
    )
    .unwrap();
    assert!(
        answer.contains("Proposed a package for “Harbour lights”"),
        "{answer}"
    );

    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, Some(&work_id)).unwrap();
    assert_eq!(chats.len(), 1);
    let message = assistant::transcript(&conn, &chats[0].id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    assert_eq!(message.meta["proposal"]["kind"], "work");
    assert!(
        message.meta["proposal"].get("title").is_none(),
        "an existing work keeps its title"
    );
    assert_eq!(message.meta["proposal"]["kind"], "work");
    assert_eq!(message.meta["proposal"]["versions"][0]["role"], "style");
}

#[test]
fn a_package_that_names_a_role_the_kind_lacks_is_refused_whole() {
    let (conn, work_id) = workspace();
    let err = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({ "work": work_id, "versions": [{ "role": "storyboard", "body": "x" }] })),
    )
    .unwrap_err();
    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("mcp.unknownVersionRole"),
        "{err}"
    );
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    assert!(
        assistant::summaries(&conn, &profile_id, Some(&work_id))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_new_work_needs_a_title_and_a_kind_the_profile_has() {
    let (conn, _) = workspace();
    let err = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({ "kind": "song" })),
    )
    .unwrap_err();
    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("proposal.newWorkNeedsTitle"),
        "{err}"
    );
    let err = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({ "title": "x", "kind": "sculpture" })),
    )
    .unwrap_err();
    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("work.unknownKind"),
        "{err}"
    );
}

#[test]
fn an_empty_package_for_a_work_is_refused() {
    let (conn, work_id) = workspace();
    let err = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({ "work": work_id })),
    )
    .unwrap_err();
    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("mcp.emptyPackage"),
        "{err}"
    );
}

#[test]
fn the_workspace_lists_the_overview_fields() {
    let (conn, _) = workspace();
    let answer = run_tool(&conn, &claude(), "workspace", &args(json!({}))).unwrap();
    let answer: Value = serde_json::from_str(&answer).unwrap();
    assert!(answer["fields"].as_array().is_some_and(|f| !f.is_empty()));
    assert!(answer["fields"][0]["key"].is_string());
}

#[test]
fn the_registration_command_quotes_the_executable() {
    let command = registration_command().unwrap();
    assert!(
        command.starts_with("claude mcp add -s user kilna -- \""),
        "{command}"
    );
    assert!(command.ends_with("\" --mcp"), "{command}");
}

#[test]
fn the_canon_is_read_card_by_card_and_through_a_lens() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let wren = fixtures::card(&conn, &profile_id, "character", "Wren");
    fixtures::fact(&conn, &wren.id, "looks", "Freckles across the nose.");
    crate::canon::fact::create_minted(
        &conn,
        crate::canon::NewFact {
            note_id: wren.id.clone(),
            section: "bio".into(),
            body: "Born on the seventeenth of June.".into(),
            layer: Some(crate::canon::Layer::Internal),
            ..crate::canon::NewFact::default()
        },
        crate::minted::Minted::fresh(),
    )
    .unwrap();

    let list = run_tool(&conn, &claude(), "canon", &args(json!({}))).unwrap();
    assert!(
        list.contains("\"Wren\"") && list.contains("\"sections\""),
        "{list}"
    );

    let whole = run_tool(&conn, &claude(), "canon", &args(json!({ "card": "wren" }))).unwrap();
    assert!(whole.contains("seventeenth"), "{whole}");
    let cover = run_tool(
        &conn,
        &claude(),
        "canon",
        &args(json!({ "card": wren.id, "lens": "cover" })),
    )
    .unwrap();
    assert!(
        cover.contains("Freckles") && !cover.contains("seventeenth"),
        "{cover}"
    );

    let found = run_tool(
        &conn,
        &claude(),
        "canon",
        &args(json!({ "query": "freckles" })),
    )
    .unwrap();
    assert!(found.contains("Freckles across the nose."), "{found}");
}

/// A long card is read in parts, the passport by `root`, and the list
/// says whether there is one and what each shape carries (v0.144).
#[test]
fn the_canon_is_read_in_parts_and_says_its_passport_and_schemas() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let wren = fixtures::card(&conn, &profile_id, "character", "Wren");
    for body in ["Freckles.", "A grey coat.", "Gradient hair."] {
        fixtures::fact(&conn, &wren.id, "looks", body);
    }
    fixtures::fact(&conn, &wren.id, "bio", "Born in June.");

    let list: Value =
        serde_json::from_str(&run_tool(&conn, &claude(), "canon", &args(json!({}))).unwrap())
            .unwrap();
    assert_eq!(list["has_root_card"], false);
    assert_eq!(
        list["section_schemas"]["palette"]["data"]["color"],
        "#RRGGBB"
    );
    assert!(
        list["section_schemas"]["bans"]["data"]
            .as_object()
            .unwrap()
            .is_empty()
    );

    let summed: Value = serde_json::from_str(
        &run_tool(
            &conn,
            &claude(),
            "canon",
            &args(json!({ "card": "Wren", "summary": true })),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(summed["total_facts"], 4);
    assert!(summed.get("facts").is_none(), "a summary lists no facts");
    let looks = summed["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|one| one["key"] == "looks")
        .unwrap()
        .clone();
    assert_eq!(looks["facts"], 3);

    let page: Value = serde_json::from_str(
        &run_tool(
            &conn,
            &claude(),
            "canon",
            &args(json!({ "card": "Wren", "section": "looks", "limit": 2, "offset": 1 })),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(page["total_facts"], 3);
    assert_eq!(page["facts"].as_array().unwrap().len(), 2);
    assert_eq!(page["has_more"], false);

    let channel = fixtures::card(&conn, &profile_id, "channel", "Night Shift");
    let root: Value = serde_json::from_str(
        &run_tool(
            &conn,
            &claude(),
            "canon",
            &args(json!({ "card": "root", "summary": true })),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(root["id"], json!(channel.id));
}

/// What `propose_canon` learned in v0.144: card updates, the passport by
/// `root` and a second one turned into an update, a fact moved and one
/// deleted, a near-duplicate marked, a relation taken away.
#[test]
fn a_canon_proposal_updates_cards_moves_and_deletes_and_marks_the_similar() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let wren = fixtures::card(&conn, &profile_id, "character", "Wren");
    let mill = fixtures::card(&conn, &profile_id, "character", "Mill");
    let channel = fixtures::card(&conn, &profile_id, "channel", "Night Shift");
    let freckles = fixtures::fact(&conn, &wren.id, "looks", "Freckles across the nose.");
    let coat = fixtures::fact(&conn, &wren.id, "looks", "A grey coat.");
    crate::actions::canon::relate(
        &conn,
        crate::canon::NewCanonLink {
            from_id: wren.id.clone(),
            to_id: mill.id.clone(),
            kind: None,
            label: Some("neighbour".into()),
            back_label: None,
            layer: None,
        },
    )
    .unwrap();

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_canon",
        &args(json!({
            "cards": [{ "kind": "channel", "title": "Night Shift FM", "note": "A late radio." }],
            "card_updates": [
                { "card": "Wren", "aliases": ["the girl"], "note": "A freckled girl." },
                { "card": "Mill" }
            ],
            "facts": [
                { "change": "move", "fact": coat.id, "section": "outfits" },
                { "change": "delete", "fact": freckles.id },
                { "card": "Wren", "section": "looks", "text": "freckles across her nose" },
                { "card": "root", "section": "voice", "text": "Calm, late, close." }
            ],
            "relations": [{ "from": "Wren", "to": "Mill", "action": "delete" }]
        })),
    )
    .unwrap();
    assert!(answer.contains("2 card update(s)"), "{answer}");
    assert!(answer.contains("similar_to"), "{answer}");
    assert!(answer.contains("changes nothing"), "{answer}");

    let message = first_message_anywhere(&conn);
    let Proposal::Canon { package } =
        serde_json::from_value(message.meta["proposal"].clone()).unwrap()
    else {
        panic!("not a canon proposal");
    };
    assert!(package.cards.is_empty(), "a second passport made a card");
    assert_eq!(package.card_updates[0].card, channel.id);
    assert_eq!(
        package.card_updates[1].note.as_deref(),
        Some("A freckled girl.")
    );
    assert_eq!(
        package.facts[2].similar_to.as_ref().unwrap().fact_id,
        freckles.id
    );
    assert_eq!(package.facts[3].card.as_deref(), Some(channel.id.as_str()));
    assert!(package.links[0].delete);

    crate::actions::proposal::apply(
        &conn,
        &message.id,
        crate::actions::proposal::Overrides::default(),
    )
    .unwrap();
    assert_eq!(
        crate::canon::fact::get(&conn, &coat.id)
            .unwrap()
            .unwrap()
            .section,
        "outfits"
    );
    assert!(
        crate::canon::fact::get(&conn, &freckles.id)
            .unwrap()
            .is_none()
    );
    let card = crate::note::get(&conn, &wren.id).unwrap().unwrap();
    assert_eq!(card.body, "A freckled girl.");
    assert_eq!(card.aliases, ["the girl"]);
    assert_eq!(
        crate::note::get(&conn, &channel.id)
            .unwrap()
            .unwrap()
            .title
            .as_deref(),
        Some("Night Shift FM")
    );
    assert!(
        crate::canon::link::between(&conn, &wren.id, &mill.id)
            .unwrap()
            .is_none()
    );
}

#[test]
fn a_canon_proposed_from_outside_waits_in_the_chat_and_names_what_it_left_out() {
    let (conn, work_id) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    fixtures::card(&conn, &profile_id, "character", "Wren");

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_canon",
        &args(json!({
            "work": work_id,
            "facts": [
                { "card": "Wren", "section": "tastes", "text": "Counts the pauses.", "line": "one line" },
                { "card": "Wren", "section": "horoscope", "text": "Gemini" }
            ]
        })),
    )
    .unwrap();
    assert!(
        answer.contains("1 fact(s)") && answer.contains("Left out"),
        "{answer}"
    );

    let message = first_message(&conn, &work_id);
    let proposal = message.meta.get("proposal").expect("a proposal waits");
    assert_eq!(proposal["kind"], "canon");
    assert!(message.body.contains("Counts the pauses."));
    assert!(
        crate::canon::fact::for_profile(&conn, &profile_id)
            .unwrap()
            .is_empty(),
        "an agent wrote a fact"
    );
    let lines: i64 = conn
        .query_row(
            "SELECT count(*) FROM journal WHERE action = 'proposal.canon'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(lines, 1);
}

/// The calendar an agent reads says which days are closed, beside what is
/// booked: a plan proposed without them would land on a holiday.
#[test]
fn the_calendar_names_the_closed_days_ahead() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let clip = video(&conn);
    fixtures::release(&conn, &clip, "youtube", Some("2026-10-02"));
    for (from, to, label) in [
        ("2026-09-01", "2026-09-03", "Gone by"),
        ("2026-09-30", "2026-10-04", "Away"),
    ] {
        crate::time_off::create_minted(
            &conn,
            &profile_id,
            crate::time_off::NewTimeOff {
                starts_on: from.into(),
                ends_on: to.into(),
                label: label.into(),
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap();
    }

    let read = run_tool(
        &conn,
        &claude(),
        "calendar",
        &args(json!({ "from": "2026-10-01" })),
    )
    .unwrap();
    let read: Value = serde_json::from_str(&read).unwrap();

    assert_eq!(read["releases"].as_array().unwrap().len(), 1);
    let closed = read["closed"].as_array().unwrap();
    assert_eq!(closed.len(), 1, "a span over by then is not ahead");
    assert_eq!(closed[0]["label"], "Away", "one still running on `from` is");
}

/// An agent checks the space's rules the way the window does (v0.151):
/// the breach names the rule, the work and what was measured.
/// The calendar reads what a neighbouring space puts out on the same
/// days, in compact fields only, when asked (v0.151, FR-20 20.2).
#[test]
fn the_calendar_reads_the_neighbours_on_the_same_days() {
    let (conn, _) = workspace();
    let clip = video(&conn);
    fixtures::release(&conn, &clip, "youtube", Some("2026-10-02"));

    let (other, other_profile, dir) = fixtures::workspace_on_disk();
    let same_day = fixtures::video(&other, &other_profile, "Lighthouse in fog").id;
    fixtures::release(&other, &same_day, "youtube", Some("2026-10-02"));
    let elsewhere = fixtures::video(&other, &other_profile, "Another day").id;
    fixtures::release(&other, &elsewhere, "youtube", Some("2026-10-09"));

    let session = Session {
        client: Some("Claude Code".into()),
        space: Some(SpaceInfo {
            name: "Here".into(),
            kind: "music".into(),
            read_only_held_by: None,
            others: vec!["There".into()],
            neighbours: vec![("There".into(), dir.path().join("kilna.db"))],
        }),
    };
    let read = run_tool(
        &conn,
        &session,
        "calendar",
        &args(json!({ "cross_spaces": true })),
    )
    .unwrap();
    let read: Value = serde_json::from_str(&read).unwrap();
    let cross = read["cross_spaces"].as_array().unwrap();
    assert_eq!(cross.len(), 1, "{cross:?}");
    assert_eq!(cross[0]["space_name"], "There");
    assert_eq!(cross[0]["date"], "2026-10-02");
    assert_eq!(cross[0]["title"], "Lighthouse in fog");
    assert!(cross[0].get("body").is_none());

    // Not asked, not read; a name that is no neighbour reads nothing.
    let plain: Value =
        serde_json::from_str(&run_tool(&conn, &session, "calendar", &args(json!({}))).unwrap())
            .unwrap();
    assert!(plain.get("cross_spaces").is_none());
    let named: Value = serde_json::from_str(
        &run_tool(
            &conn,
            &session,
            "calendar",
            &args(json!({ "cross_spaces": ["Nowhere"] })),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(named["cross_spaces"], json!([]));
}

#[test]
fn an_agent_checks_the_quotas() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let channel = fixtures::card(&conn, &profile_id, "channel", "The channel");
    crate::canon::fact::create_minted(
        &conn,
        crate::canon::NewFact {
            note_id: channel.id.clone(),
            section: "quotas".into(),
            body: "Short titles".into(),
            data: json!({ "field": "title", "metric": "char_len", "op": "max", "value": 5 })
                .as_object()
                .cloned()
                .unwrap(),
            ..crate::canon::NewFact::default()
        },
        crate::minted::Minted::fresh(),
    )
    .unwrap();
    fixtures::song(&conn, &profile_id, "A title too long");

    let read = run_tool(&conn, &claude(), "quotas", &args(json!({}))).unwrap();
    let read: Value = serde_json::from_str(&read).unwrap();
    assert_eq!(read["rules"], 1);
    let found = read["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|one| one["title"] == "A title too long")
        .unwrap_or_else(|| panic!("{read}"));
    assert_eq!(found["label"], "Short titles");
    assert_eq!(found["measured"], 16.0);

    let missing = run_tool(
        &conn,
        &claude(),
        "quotas",
        &args(json!({ "collection": "Nowhere" })),
    );
    assert!(missing.is_err());
}

/// A month's drafts proposed at once (v0.153): the checklist reads the
/// month as it would stand, and one click writes every work and marks
/// the bank; one undo takes the month back.
#[test]
fn a_month_is_proposed_checked_and_applied_in_one() {
    let conn = crate::db::open_in_memory().unwrap();
    profile::seed(&conn, "track-channel").unwrap();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let channel = fixtures::card(&conn, &profile_id, "channel", "The channel");
    let fact = |section: &str, body: &str, data: Value| {
        crate::canon::fact::create_minted(
            &conn,
            crate::canon::NewFact {
                note_id: channel.id.clone(),
                section: section.into(),
                body: body.into(),
                data: data.as_object().cloned().unwrap(),
                ..crate::canon::NewFact::default()
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap()
    };
    let mande = fact("bank", "Mande", json!({ "cooldown_months": 6 }));
    fact("bank", "Gnawa", json!({ "locked": true }));
    crate::register::create(
        &conn,
        &profile_id,
        crate::register::NewTerm {
            word: "dawn".into(),
            scope: crate::register::TermScope::Title,
            ..crate::register::NewTerm::default()
        },
    )
    .unwrap();
    fact(
        "quotas",
        "Suno prompt length",
        json!({ "field": "suno_prompt", "metric": "char_len", "op": "min", "value": 20 }),
    );
    let first = fixtures::work(&conn, &profile_id, "track", "Draft one");
    let second = fixtures::work(&conn, &profile_id, "track", "Draft two");

    let said = run_tool(
        &conn,
        &claude(),
        "propose_batch_prompts",
        &args(json!({ "works": [
            { "work": first.id, "title": "Kora at dawn", "fields": { "tradition": "Mande", "nonsense": 1 },
              "versions": [{ "role": "suno_prompt", "body": "kora, balafon" }], "bank": ["mande", "Atlantis", "Gnawa"] },
            { "work": "Draft two", "fields": { "mood": "calm" },
              "versions": [{ "role": "suno_prompt", "body": "slow strings over a warm sub bass" }] },
        ], "register": [{ "word": "sub bass", "scope": "sound_combo" }] })),
    )
    .unwrap();
    assert!(said.contains("Proposed 2 works"), "{said}");
    assert!(said.contains("Atlantis"), "{said}");

    let chats = assistant::summaries(&conn, &profile_id, None).unwrap();
    let chat = chats.iter().find(|c| c.work_id.is_none()).unwrap();
    let proposed = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    // The checklist: the first prompt is too short, as it would stand; a
    // locked tradition is taken; a spent title word is said.
    let crate::assistant::proposal::Proposal::Month { items, .. } =
        crate::actions::proposal::stored_proposal(&proposed.meta).unwrap()
    else {
        panic!("not a month")
    };
    let checked = crate::month::check(&conn, None, None, &items).unwrap();
    assert_eq!(checked.bank.len(), 1, "{:?}", checked.bank);
    assert_eq!(checked.bank[0].item, "Gnawa");
    assert_eq!(checked.register.len(), 1, "{:?}", checked.register);
    assert_eq!(checked.register[0].word, "dawn");
    assert_eq!(checked.register[0].scope, crate::register::TermScope::Title);
    let report = checked.quotas;
    let short: Vec<&str> = report
        .findings
        .iter()
        .filter_map(|found| found.title.as_deref())
        .collect();
    assert_eq!(short, vec!["Draft one"], "{report:?}");

    crate::actions::proposal::apply(&conn, &proposed.id, Default::default()).unwrap();
    let renamed = work::get(&conn, &first.id).unwrap().unwrap();
    assert_eq!(renamed.title, "Kora at dawn");
    assert_eq!(renamed.meta.get("tradition"), Some(&json!("Mande")));
    assert_eq!(
        crate::work::version::for_role(&conn, &second.id, "suno_prompt")
            .unwrap()
            .unwrap()
            .body,
        "slow strings over a warm sub bass"
    );
    let used = crate::canon::fact::get(&conn, &mande.id).unwrap().unwrap();
    assert_eq!(used.data["used_in"][0]["work_id"], json!(first.id));
    let kept = crate::register::find_word(&conn, &profile_id, "sub bass", None)
        .unwrap()
        .unwrap();
    assert_eq!(kept.scope, crate::register::TermScope::SoundCombo);

    let offer = crate::undo::last(&conn).unwrap().unwrap();
    crate::undo::undo(&conn, &offer.operation_id).unwrap();
    assert_eq!(
        work::get(&conn, &first.id).unwrap().unwrap().title,
        "Draft one"
    );
    assert!(
        !crate::canon::fact::get(&conn, &mande.id)
            .unwrap()
            .unwrap()
            .data
            .contains_key("used_in")
    );
}

/// Files an assistant made beside kilna wait as one proposal, and move
/// into their works only when it is applied (v0.156).
#[test]
fn files_are_proposed_and_moved_into_their_works_when_applied() {
    let (conn, profile_id) = fixtures::workspace();
    let root = tempfile::tempdir().unwrap();
    crate::folder::set_root(
        &conn,
        &profile_id,
        Some(&root.path().to_string_lossy()),
        "2030-01-01T00:00:00.000Z",
    )
    .unwrap();
    let song = fixtures::song(&conn, &profile_id, "Harbour lights");
    let downloads = tempfile::tempdir().unwrap();
    let take = downloads.path().join("suno-7f3a.mp3");
    std::fs::write(&take, b"ID3").unwrap();
    let path = take.to_string_lossy().into_owned();

    let wrong = run_tool(
        &conn,
        &claude(),
        "propose_media",
        &args(
            json!({ "items": [{ "work": "Harbour lights", "path": path, "role": "final_cover" }] }),
        ),
    )
    .unwrap_err();
    assert_eq!(wrong.refusal().map(|one| one.code), Some("media.wrongKind"));

    let said = run_tool(
        &conn,
        &claude(),
        "propose_media",
        &args(
            json!({ "items": [{ "work": "Harbour lights", "path": path, "role": "final_audio" }] }),
        ),
    )
    .unwrap();
    assert!(said.contains("Proposed 1 files"), "{said}");
    assert!(take.exists(), "nothing moves before it is applied");

    let chats = assistant::summaries(&conn, &profile_id, None).unwrap();
    let chat = chats.iter().find(|c| c.work_id.is_none()).unwrap();
    let proposed = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    crate::actions::proposal::apply(&conn, &proposed.id, Default::default()).unwrap();

    assert!(!take.exists());
    let moved = work::get(&conn, &song.id).unwrap().unwrap();
    assert_eq!(moved.file.as_deref(), Some("Harbour lights.mp3"));
}

/// Pictures an assistant matched by what they show carry why and how
/// sure, and only the ones ticked are taken (v0.157).
#[test]
fn only_the_ticked_files_of_a_proposal_are_taken() {
    let (conn, profile_id) = fixtures::workspace();
    let root = tempfile::tempdir().unwrap();
    crate::folder::set_root(
        &conn,
        &profile_id,
        Some(&root.path().to_string_lossy()),
        "2030-01-01T00:00:00.000Z",
    )
    .unwrap();
    fixtures::song(&conn, &profile_id, "Harbour lights");
    let downloads = tempfile::tempdir().unwrap();
    let kept = downloads.path().join("a.mp3");
    let left = downloads.path().join("b.mp3");
    std::fs::write(&kept, b"ID3").unwrap();
    std::fs::write(&left, b"ID3").unwrap();
    run_tool(
        &conn,
        &claude(),
        "propose_media",
        &args(json!({ "items": [
            { "work": "Harbour lights", "path": kept.to_string_lossy(), "role": "candidate_audio",
              "reason": "the hook it was asked for", "confidence": 1.7 },
            { "work": "Harbour lights", "path": left.to_string_lossy(), "role": "candidate_audio" },
        ] })),
    )
    .unwrap();
    let chats = assistant::summaries(&conn, &profile_id, None).unwrap();
    let chat = chats.iter().find(|c| c.work_id.is_none()).unwrap();
    let proposed = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    let crate::assistant::proposal::Proposal::Media { items } =
        crate::actions::proposal::stored_proposal(&proposed.meta).unwrap()
    else {
        panic!("not files")
    };
    assert_eq!(
        items[0].reason.as_deref(),
        Some("the hook it was asked for")
    );
    assert_eq!(items[0].confidence, Some(1.0), "held to 0..1");

    crate::actions::proposal::apply(
        &conn,
        &proposed.id,
        crate::actions::proposal::Overrides {
            items: Some(vec!["media:0".into()]),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!kept.exists());
    assert!(left.exists(), "an unticked file stays where it was");
}
