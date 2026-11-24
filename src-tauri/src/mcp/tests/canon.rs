//! The canon, read and proposed.

use super::*;

#[test]
fn the_canon_is_read_card_by_card_and_through_a_lens() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let wren = fixtures::card(&conn, &profile_id, "character", "Wren");
    fixtures::fact(&conn, &wren.id, "looks", "Freckles across the nose.");
    crate::canon::fact::create_minted(
        &conn,
        crate::canon::NewFact {
            note_id: wren.id.clone(),
            section: "bio".into(),
            body: "Born on the seventeenth of June.".into(),
            layer: Some(crate::canon::Layer::Internal),
            ..crate::canon::NewFact::default()
        },
        crate::minted::Minted::fresh(),
    )
    .unwrap();

    let list = run_tool(&conn, &claude(), "canon", &args(json!({}))).unwrap();
    assert!(
        list.contains("\"Wren\"") && list.contains("\"sections\""),
        "{list}"
    );

    let whole = run_tool(&conn, &claude(), "canon", &args(json!({ "card": "wren" }))).unwrap();
    assert!(whole.contains("seventeenth"), "{whole}");
    let cover = run_tool(
        &conn,
        &claude(),
        "canon",
        &args(json!({ "card": wren.id, "lens": "cover" })),
    )
    .unwrap();
    assert!(
        cover.contains("Freckles") && !cover.contains("seventeenth"),
        "{cover}"
    );

    let found = run_tool(
        &conn,
        &claude(),
        "canon",
        &args(json!({ "query": "freckles" })),
    )
    .unwrap();
    assert!(found.contains("Freckles across the nose."), "{found}");
}

/// A long card is read in parts, the passport by `root`, and the list
/// says whether there is one and what each shape carries (v0.144).
#[test]
fn the_canon_is_read_in_parts_and_says_its_passport_and_schemas() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let wren = fixtures::card(&conn, &profile_id, "character", "Wren");
    for body in ["Freckles.", "A grey coat.", "Gradient hair."] {
        fixtures::fact(&conn, &wren.id, "looks", body);
    }
    fixtures::fact(&conn, &wren.id, "bio", "Born in June.");

    let list: Value =
        serde_json::from_str(&run_tool(&conn, &claude(), "canon", &args(json!({}))).unwrap())
            .unwrap();
    assert_eq!(list["has_root_card"], false);
    assert_eq!(
        list["section_schemas"]["palette"]["data"]["color"],
        "#RRGGBB"
    );
    assert!(
        list["section_schemas"]["bans"]["data"]
            .as_object()
            .unwrap()
            .is_empty()
    );

    let summed: Value = serde_json::from_str(
        &run_tool(
            &conn,
            &claude(),
            "canon",
            &args(json!({ "card": "Wren", "summary": true })),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(summed["total_facts"], 4);
    assert!(summed.get("facts").is_none(), "a summary lists no facts");
    let looks = summed["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|one| one["key"] == "looks")
        .unwrap()
        .clone();
    assert_eq!(looks["facts"], 3);

    let page: Value = serde_json::from_str(
        &run_tool(
            &conn,
            &claude(),
            "canon",
            &args(json!({ "card": "Wren", "section": "looks", "limit": 2, "offset": 1 })),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(page["total_facts"], 3);
    assert_eq!(page["facts"].as_array().unwrap().len(), 2);
    assert_eq!(page["has_more"], false);

    let channel = fixtures::card(&conn, &profile_id, "channel", "Night Shift");
    let root: Value = serde_json::from_str(
        &run_tool(
            &conn,
            &claude(),
            "canon",
            &args(json!({ "card": "root", "summary": true })),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(root["id"], json!(channel.id));
}

/// What `propose_canon` learned in v0.144: card updates, the passport by
/// `root` and a second one turned into an update, a fact moved and one
/// deleted, a near-duplicate marked, a relation taken away.
#[test]
fn a_canon_proposal_updates_cards_moves_and_deletes_and_marks_the_similar() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let wren = fixtures::card(&conn, &profile_id, "character", "Wren");
    let mill = fixtures::card(&conn, &profile_id, "character", "Mill");
    let channel = fixtures::card(&conn, &profile_id, "channel", "Night Shift");
    let freckles = fixtures::fact(&conn, &wren.id, "looks", "Freckles across the nose.");
    let coat = fixtures::fact(&conn, &wren.id, "looks", "A grey coat.");
    crate::actions::canon::relate(
        &conn,
        crate::canon::NewCanonLink {
            from_id: wren.id.clone(),
            to_id: mill.id.clone(),
            kind: None,
            label: Some("neighbour".into()),
            back_label: None,
            layer: None,
        },
    )
    .unwrap();

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_canon",
        &args(json!({
            "cards": [{ "kind": "channel", "title": "Night Shift FM", "note": "A late radio." }],
            "card_updates": [
                { "card": "Wren", "aliases": ["the girl"], "note": "A freckled girl." },
                { "card": "Mill" }
            ],
            "facts": [
                { "change": "move", "fact": coat.id, "section": "outfits" },
                { "change": "delete", "fact": freckles.id },
                { "card": "Wren", "section": "looks", "text": "freckles across her nose" },
                { "card": "root", "section": "voice", "text": "Calm, late, close." }
            ],
            "relations": [{ "from": "Wren", "to": "Mill", "action": "delete" }]
        })),
    )
    .unwrap();
    assert!(answer.contains("2 card update(s)"), "{answer}");
    assert!(answer.contains("similar_to"), "{answer}");
    assert!(answer.contains("changes nothing"), "{answer}");

    let message = first_message_anywhere(&conn);
    let Proposal::Canon { package } =
        serde_json::from_value(message.meta["proposal"].clone()).unwrap()
    else {
        panic!("not a canon proposal");
    };
    assert!(package.cards.is_empty(), "a second passport made a card");
    assert_eq!(package.card_updates[0].card, channel.id);
    assert_eq!(
        package.card_updates[1].note.as_deref(),
        Some("A freckled girl.")
    );
    assert_eq!(
        package.facts[2].similar_to.as_ref().unwrap().fact_id,
        freckles.id
    );
    assert_eq!(package.facts[3].card.as_deref(), Some(channel.id.as_str()));
    assert!(package.links[0].delete);

    crate::actions::proposal::apply(
        &conn,
        &message.id,
        crate::actions::proposal::Overrides::default(),
    )
    .unwrap();
    assert_eq!(
        crate::canon::fact::get(&conn, &coat.id)
            .unwrap()
            .unwrap()
            .section,
        "outfits"
    );
    assert!(
        crate::canon::fact::get(&conn, &freckles.id)
            .unwrap()
            .is_none()
    );
    let card = crate::note::get(&conn, &wren.id).unwrap().unwrap();
    assert_eq!(card.body, "A freckled girl.");
    assert_eq!(card.aliases, ["the girl"]);
    assert_eq!(
        crate::note::get(&conn, &channel.id)
            .unwrap()
            .unwrap()
            .title
            .as_deref(),
        Some("Night Shift FM")
    );
    assert!(
        crate::canon::link::between(&conn, &wren.id, &mill.id)
            .unwrap()
            .is_none()
    );
}

#[test]
fn a_canon_proposed_from_outside_waits_in_the_chat_and_names_what_it_left_out() {
    let (conn, work_id) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    fixtures::card(&conn, &profile_id, "character", "Wren");

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_canon",
        &args(json!({
            "work": work_id,
            "facts": [
                { "card": "Wren", "section": "tastes", "text": "Counts the pauses.", "line": "one line" },
                { "card": "Wren", "section": "horoscope", "text": "Gemini" }
            ]
        })),
    )
    .unwrap();
    assert!(
        answer.contains("1 fact(s)") && answer.contains("Left out"),
        "{answer}"
    );

    let message = first_message(&conn, &work_id);
    let proposal = message.meta.get("proposal").expect("a proposal waits");
    assert_eq!(proposal["kind"], "canon");
    assert!(message.body.contains("Counts the pauses."));
    assert!(
        crate::canon::fact::for_profile(&conn, &profile_id)
            .unwrap()
            .is_empty(),
        "an agent wrote a fact"
    );
    let lines: i64 = conn
        .query_row(
            "SELECT count(*) FROM journal WHERE action = 'proposal.canon'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(lines, 1);
}
