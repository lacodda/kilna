//! An experiment's trials.

use super::*;

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
