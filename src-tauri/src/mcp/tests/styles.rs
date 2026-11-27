//! The style dictionary, and a text written from its bricks.

use super::*;

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
    let rows = motion["bricks"].as_array().unwrap();
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
    let found = found["bricks"].as_array().unwrap();
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
    let sound = sound["bricks"].as_array().unwrap();
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
fn the_dictionary_comes_a_page_at_a_time_short_or_one_brick_whole() {
    let (conn, _) = workspace();
    crate::style_set::seed(&conn).unwrap();

    let page = read(
        &conn,
        "styles",
        json!({ "branch": "motion", "limit": 5, "offset": 5, "summary": true }),
    );
    assert_eq!(page["total"], 20);
    assert_eq!(page["has_more"], true);
    let bricks = page["bricks"].as_array().unwrap();
    assert_eq!(bricks.len(), 5);
    assert!(
        bricks
            .iter()
            .all(|brick| brick["name"].is_string() && brick.get("phrase").is_none()),
        "a summary leaves the phrases out"
    );

    let id = bricks[0]["id"].as_str().unwrap();
    let whole = read(&conn, "styles", json!({ "brick": id }));
    assert_eq!(whole["id"], id);
    assert!(whole.get("phrase").is_some() && whole.get("explanation").is_some());
    let refused = run_tool(
        &conn,
        &claude(),
        "styles",
        &args(json!({ "brick": "nope" })),
    )
    .unwrap_err();
    assert_eq!(refused.reason().key, "refusal.mcp.unknownBrick");
}
