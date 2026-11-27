//! Storyboards proposed for a video.

use super::*;

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
fn a_board_is_read_a_page_at_a_time_short_or_one_scene_whole() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let video_id = video(&conn);
    for (section, words) in [
        ("intro", "the harbour at dawn"),
        ("verse 1", "hands on a rope"),
    ] {
        scene::create(
            &conn,
            &profile_id,
            scene::NewScene {
                work_id: video_id.clone(),
                section: Some(section.into()),
                description: Some(words.into()),
                ..Default::default()
            },
        )
        .unwrap();
    }

    let whole = read(&conn, "scenes", json!({ "work": video_id }));
    assert_eq!(whole["total"], 2);
    assert_eq!(whole["has_more"], false);
    assert!(whole["scenes"][0]["blocks"].is_object());

    let first = read(
        &conn,
        "scenes",
        json!({ "work": video_id, "limit": 1, "summary": true }),
    );
    assert_eq!(first["has_more"], true);
    let short = &first["scenes"][0];
    assert_eq!(short["position"], 1);
    assert_eq!(short["description"], "the harbour at dawn");
    assert!(short.get("blocks").is_none(), "a summary has no blocks");

    let second = read(&conn, "scenes", json!({ "work": video_id, "scene": "2" }));
    assert_eq!(second["section"], "verse 1");
    assert!(second["blocks"].is_object());
    let refused = run_tool(
        &conn,
        &claude(),
        "scenes",
        &args(json!({ "work": video_id, "scene": "9" })),
    )
    .unwrap_err();
    assert_eq!(refused.reason().key, "refusal.mcp.unknownScene");
}
