//! The protocol itself: what a message is answered with before any tool runs.

use super::*;

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

#[test]
fn the_registration_command_quotes_the_executable() {
    let command = registration_command().unwrap();
    assert!(
        command.starts_with("claude mcp add -s user kilna -- \""),
        "{command}"
    );
    assert!(command.ends_with("\" --mcp"), "{command}");
}

/// Every tool a client is told about is one the server runs, and a name it
/// was never told is refused as unknown: the list and the table of functions
/// are two places, and nothing else holds them together.
#[test]
fn every_listed_tool_is_run_and_no_other() {
    let (conn, _) = workspace();
    let unknown = |name: &str| {
        run_tool(&conn, &claude(), name, &Map::new())
            .err()
            .and_then(|err| err.refusal().map(|refusal| refusal.code.to_string()))
            .is_some_and(|code| code == "mcp.unknownTool")
    };
    let listed = tools();
    assert!(listed.len() > 20, "the list went somewhere else");
    for tool in listed {
        let name = tool["name"].as_str().unwrap();
        assert!(!unknown(name), "`{name}` is listed and not run");
    }
    assert!(unknown("propose_nothing"));
}
