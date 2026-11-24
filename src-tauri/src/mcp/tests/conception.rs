//! The order a writer works from and the cards of its boards.

use super::*;

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
