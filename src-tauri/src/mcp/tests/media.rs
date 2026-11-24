//! Files proposed for their works.

use super::*;

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
