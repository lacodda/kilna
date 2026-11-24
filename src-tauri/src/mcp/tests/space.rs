//! The space as `workspace` says it.

use super::*;

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
fn the_workspace_lists_the_overview_fields() {
    let (conn, _) = workspace();
    let answer = run_tool(&conn, &claude(), "workspace", &args(json!({}))).unwrap();
    let answer: Value = serde_json::from_str(&answer).unwrap();
    assert!(answer["fields"].as_array().is_some_and(|f| !f.is_empty()));
    assert!(answer["fields"][0]["key"].is_string());
}
