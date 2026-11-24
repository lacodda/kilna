//! Ideas for a cover.

use super::*;

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
