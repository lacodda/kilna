//! Where a proposal lands, whoever sent it.

use super::*;

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
