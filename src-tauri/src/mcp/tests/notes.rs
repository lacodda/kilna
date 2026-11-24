//! Notes and phrases, and the long readings read short.

use super::*;

/// `notes` narrowed by a state gives only the notes standing in it, and a
/// word that is no state is refused rather than read as "all".
#[test]
fn notes_are_narrowed_by_the_state_they_stand_in() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let note = |body: &str, state: Option<note::NoteState>| {
        note::create(
            &conn,
            &profile_id,
            note::NewNote {
                body: body.into(),
                state,
                ..note::NewNote::default()
            },
        )
        .unwrap()
        .id
    };
    let fresh = note("a thought still fresh", None);
    let parked = note("a thought put aside", note::NoteState::from_word("parked"));
    let ids = |with: Value| -> Vec<String> {
        read(&conn, "notes", with)["notes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|note| note["id"].as_str().unwrap().to_owned())
            .collect()
    };

    assert_eq!(ids(json!({ "state": "fresh" })), [fresh.clone()]);
    assert_eq!(ids(json!({ "state": "parked" })), [parked.clone()]);
    let both = ids(json!({}));
    assert!(both.contains(&fresh) && both.contains(&parked), "{both:?}");

    let refused = run_tool(
        &conn,
        &claude(),
        "notes",
        &args(json!({ "state": "somewhere" })),
    )
    .unwrap_err();
    assert_eq!(refused.refusal().unwrap().code, "mcp.unknownNoteState");
}

/// A summary leaves the long texts out and says how long they are; one
/// entry is then read whole by its id or its name.
#[test]
fn a_summary_leaves_the_texts_out_and_one_entry_is_read_whole() {
    let (conn, work_id) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let body = "a long thought about harbours\n".repeat(40);
    let kept = note::create(
        &conn,
        &profile_id,
        note::NewNote {
            body: body.clone(),
            title: Some("On harbours".into()),
            tags: vec!["sea".into()],
            ..note::NewNote::default()
        },
    )
    .unwrap();
    note::create(
        &conn,
        &profile_id,
        note::NewNote {
            body: "no title here, only a first line\nand a second".into(),
            ..note::NewNote::default()
        },
    )
    .unwrap();

    let short = read(&conn, "notes", json!({ "summary": true }));
    assert_eq!(short["total"], 2);
    let listed = short["notes"].as_array().unwrap();
    let harbours = listed
        .iter()
        .find(|one| one["id"] == json!(kept.id))
        .unwrap();
    assert!(harbours.get("body").is_none(), "a summary carries a body");
    assert_eq!(harbours["title"], "On harbours");
    assert_eq!(harbours["length"], body.chars().count());
    assert_eq!(harbours["tags"], json!(["sea"]));
    let untitled = listed
        .iter()
        .find(|one| one["id"] != json!(kept.id))
        .unwrap();
    assert_eq!(untitled["title"], "no title here, only a first line");
    // With no question asked, the shelves are still said.
    assert!(short.get("shelves").is_some());

    // Whole, by id and by its exact title.
    for named in [kept.id.as_str(), "On harbours"] {
        let whole = read(&conn, "notes", json!({ "note": named }));
        assert_eq!(whole["id"], json!(kept.id));
        assert_eq!(whole["body"], json!(body));
    }
    let unknown = run_tool(
        &conn,
        &claude(),
        "notes",
        &args(json!({ "note": "No such" })),
    );
    assert_eq!(unknown.unwrap_err().reason().key, "refusal.mcp.unknownNote");

    // The register: a word short, then whole with the works that carry it.
    let term = crate::register::create(
        &conn,
        &profile_id,
        crate::register::NewTerm {
            word: "lines".into(),
            note: Some("the song's own word".into()),
            ..crate::register::NewTerm::default()
        },
    )
    .unwrap();
    let record = read(&conn, "register", json!({ "summary": true }));
    assert_eq!(record["total"], 1);
    let word = &record["terms"][0];
    assert_eq!(word["word"], "lines");
    assert!(word.get("note").is_none() && word.get("forms").is_none());
    for named in [term.id.as_str(), "Lines"] {
        let whole = read(&conn, "register", json!({ "term": named }));
        assert_eq!(whole["note"], "the song's own word");
        assert_eq!(
            whole["works"][0]["work_id"],
            json!(work_id),
            "the work whose text says it: {whole}"
        );
    }
}

/// `propose_note` changes a note that is there and proposes several at
/// once (FR-08 8.2): a change names its note by id or exact title and is
/// shown against what the note holds; the notes are kept one by one, and
/// the change writes only what it says.
#[test]
fn notes_are_proposed_several_at_once_and_a_note_is_changed() {
    let (conn, work_id) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let kept = note::create(
        &conn,
        &profile_id,
        note::NewNote {
            body: "the road as a witness".into(),
            title: Some("Concept".into()),
            tags: vec!["idea".into()],
            work_id: Some(work_id.clone()),
            ..note::NewNote::default()
        },
    )
    .unwrap();

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_note",
        &args(json!({
            "work": work_id,
            "notes": [
                { "note": "Concept", "body": "the road as a witness, and a judge", "tags": ["idea", "#road"] },
                { "title": "Bridge", "body": "try a key change", "tags": ["music"], "layer": "internal" },
                { "title": "Left out", "body": "not this one" }
            ],
        })),
    )
    .unwrap();
    assert!(
        answer.contains("Proposed 3 notes (1 of them a change"),
        "{answer}"
    );

    let message = first_message(&conn, &work_id);
    let proposal = &message.meta["proposal"];
    assert_eq!(proposal["kind"], "notes");
    // The change carries what the note held, for was against becomes.
    assert_eq!(proposal["notes"][0]["note"], json!(kept.id));
    assert_eq!(
        proposal["notes"][0]["before"]["body"],
        "the road as a witness"
    );
    assert_eq!(proposal["notes"][0]["tags"], json!(["idea", "road"]));
    assert!(
        message.body.contains("tags: idea → idea, road"),
        "{}",
        message.body
    );
    assert!(
        message.body.contains("the text is rewritten"),
        "{}",
        message.body
    );
    // Nothing is written until it is kept.
    assert_eq!(
        note::get(&conn, &kept.id).unwrap().unwrap().body,
        "the road as a witness"
    );

    let outcome = crate::actions::proposal::apply(
        &conn,
        &message.id,
        crate::actions::proposal::Overrides {
            items: Some(vec!["note:0".into(), "note:1".into()]),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(outcome.notes.len(), 2);

    let changed = note::get(&conn, &kept.id).unwrap().unwrap();
    assert_eq!(changed.body, "the road as a witness, and a judge");
    assert_eq!(changed.tags, ["idea", "road"]);
    assert_eq!(
        changed.title.as_deref(),
        Some("Concept"),
        "the title was not said"
    );
    let all = note::list(&conn, &profile_id, &NoteFilter::default()).unwrap();
    let bridge = all
        .iter()
        .find(|one| one.title.as_deref() == Some("Bridge"))
        .unwrap();
    assert_eq!(bridge.tags, ["music"]);
    assert_eq!(bridge.layer, crate::canon::Layer::Internal);
    assert_eq!(bridge.work_id.as_deref(), Some(work_id.as_str()));
    assert!(
        !all.iter()
            .any(|one| one.title.as_deref() == Some("Left out")),
        "a note that was not taken was written"
    );
    assert_eq!(all.len(), 2, "the change made a second note");

    // A note nobody can find is said, not made.
    let unknown = run_tool(
        &conn,
        &claude(),
        "propose_note",
        &args(json!({ "note": "No such note", "body": "x" })),
    );
    assert_eq!(unknown.unwrap_err().reason().key, "refusal.mcp.unknownNote");
}
