//! The skills, read by `guide` and served as prompts.

use super::*;

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
