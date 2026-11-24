//! The calendar, the quotas, a month's drafts and the text of a release.

use super::*;

/// The calendar an agent reads says which days are closed, beside what is
/// booked: a plan proposed without them would land on a holiday.
#[test]
fn the_calendar_names_the_closed_days_ahead() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let clip = video(&conn);
    fixtures::release(&conn, &clip, "youtube", Some("2026-10-02"));
    for (from, to, label) in [
        ("2026-09-01", "2026-09-03", "Gone by"),
        ("2026-09-30", "2026-10-04", "Away"),
    ] {
        crate::time_off::create_minted(
            &conn,
            &profile_id,
            crate::time_off::NewTimeOff {
                starts_on: from.into(),
                ends_on: to.into(),
                label: label.into(),
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap();
    }

    let read = run_tool(
        &conn,
        &claude(),
        "calendar",
        &args(json!({ "from": "2026-10-01" })),
    )
    .unwrap();
    let read: Value = serde_json::from_str(&read).unwrap();

    assert_eq!(read["releases"].as_array().unwrap().len(), 1);
    let closed = read["closed"].as_array().unwrap();
    assert_eq!(closed.len(), 1, "a span over by then is not ahead");
    assert_eq!(closed[0]["label"], "Away", "one still running on `from` is");
}

/// An agent checks the space's rules the way the window does (v0.151):
/// the breach names the rule, the work and what was measured.
/// The calendar reads what a neighbouring space puts out on the same
/// days, in compact fields only, when asked (v0.151, FR-20 20.2).
#[test]
fn the_calendar_reads_the_neighbours_on_the_same_days() {
    let (conn, _) = workspace();
    let clip = video(&conn);
    fixtures::release(&conn, &clip, "youtube", Some("2026-10-02"));

    let (other, other_profile, dir) = fixtures::workspace_on_disk();
    let same_day = fixtures::video(&other, &other_profile, "Lighthouse in fog").id;
    fixtures::release(&other, &same_day, "youtube", Some("2026-10-02"));
    let elsewhere = fixtures::video(&other, &other_profile, "Another day").id;
    fixtures::release(&other, &elsewhere, "youtube", Some("2026-10-09"));

    let session = Session {
        client: Some("Claude Code".into()),
        space: Some(SpaceInfo {
            name: "Here".into(),
            kind: "music".into(),
            read_only_held_by: None,
            others: vec!["There".into()],
            neighbours: vec![("There".into(), dir.path().join("kilna.db"))],
        }),
    };
    let read = run_tool(
        &conn,
        &session,
        "calendar",
        &args(json!({ "cross_spaces": true })),
    )
    .unwrap();
    let read: Value = serde_json::from_str(&read).unwrap();
    let cross = read["cross_spaces"].as_array().unwrap();
    assert_eq!(cross.len(), 1, "{cross:?}");
    assert_eq!(cross[0]["space_name"], "There");
    assert_eq!(cross[0]["date"], "2026-10-02");
    assert_eq!(cross[0]["title"], "Lighthouse in fog");
    assert!(cross[0].get("body").is_none());

    // Not asked, not read; a name that is no neighbour reads nothing.
    let plain: Value =
        serde_json::from_str(&run_tool(&conn, &session, "calendar", &args(json!({}))).unwrap())
            .unwrap();
    assert!(plain.get("cross_spaces").is_none());
    let named: Value = serde_json::from_str(
        &run_tool(
            &conn,
            &session,
            "calendar",
            &args(json!({ "cross_spaces": ["Nowhere"] })),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(named["cross_spaces"], json!([]));
}

#[test]
fn an_agent_checks_the_quotas() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let channel = fixtures::card(&conn, &profile_id, "channel", "The channel");
    crate::canon::fact::create_minted(
        &conn,
        crate::canon::NewFact {
            note_id: channel.id.clone(),
            section: "quotas".into(),
            body: "Short titles".into(),
            data: json!({ "field": "title", "metric": "char_len", "op": "max", "value": 5 })
                .as_object()
                .cloned()
                .unwrap(),
            ..crate::canon::NewFact::default()
        },
        crate::minted::Minted::fresh(),
    )
    .unwrap();
    fixtures::song(&conn, &profile_id, "A title too long");

    let read = run_tool(&conn, &claude(), "quotas", &args(json!({}))).unwrap();
    let read: Value = serde_json::from_str(&read).unwrap();
    assert_eq!(read["rules"], 1);
    let found = read["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|one| one["title"] == "A title too long")
        .unwrap_or_else(|| panic!("{read}"));
    assert_eq!(found["label"], "Short titles");
    assert_eq!(found["measured"], 16.0);

    let missing = run_tool(
        &conn,
        &claude(),
        "quotas",
        &args(json!({ "collection": "Nowhere" })),
    );
    assert!(missing.is_err());
}

/// A month's drafts proposed at once (v0.153): the checklist reads the
/// month as it would stand, and one click writes every work and marks
/// the bank; one undo takes the month back.
#[test]
fn a_month_is_proposed_checked_and_applied_in_one() {
    let conn = crate::db::open_in_memory().unwrap();
    profile::seed(&conn, "track-channel").unwrap();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let channel = fixtures::card(&conn, &profile_id, "channel", "The channel");
    let fact = |section: &str, body: &str, data: Value| {
        crate::canon::fact::create_minted(
            &conn,
            crate::canon::NewFact {
                note_id: channel.id.clone(),
                section: section.into(),
                body: body.into(),
                data: data.as_object().cloned().unwrap(),
                ..crate::canon::NewFact::default()
            },
            crate::minted::Minted::fresh(),
        )
        .unwrap()
    };
    let mande = fact("bank", "Mande", json!({ "cooldown_months": 6 }));
    fact("bank", "Gnawa", json!({ "locked": true }));
    crate::register::create(
        &conn,
        &profile_id,
        crate::register::NewTerm {
            word: "dawn".into(),
            scope: crate::register::TermScope::Title,
            ..crate::register::NewTerm::default()
        },
    )
    .unwrap();
    fact(
        "quotas",
        "Suno prompt length",
        json!({ "field": "suno_prompt", "metric": "char_len", "op": "min", "value": 20 }),
    );
    let first = fixtures::work(&conn, &profile_id, "track", "Draft one");
    let second = fixtures::work(&conn, &profile_id, "track", "Draft two");

    let said = run_tool(
        &conn,
        &claude(),
        "propose_batch_prompts",
        &args(json!({ "works": [
            { "work": first.id, "title": "Kora at dawn", "fields": { "tradition": "Mande", "nonsense": 1 },
              "versions": [{ "role": "suno_prompt", "body": "kora, balafon" }], "bank": ["mande", "Atlantis", "Gnawa"] },
            { "work": "Draft two", "fields": { "mood": "calm" },
              "versions": [{ "role": "suno_prompt", "body": "slow strings over a warm sub bass" }] },
        ], "register": [{ "word": "sub bass", "scope": "sound_combo" }] })),
    )
    .unwrap();
    assert!(said.contains("Proposed 2 works"), "{said}");
    assert!(said.contains("Atlantis"), "{said}");

    let chats = assistant::summaries(&conn, &profile_id, None).unwrap();
    let chat = chats.iter().find(|c| c.work_id.is_none()).unwrap();
    let proposed = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    // The checklist: the first prompt is too short, as it would stand; a
    // locked tradition is taken; a spent title word is said.
    let crate::assistant::proposal::Proposal::Month { items, .. } =
        crate::actions::proposal::stored_proposal(&proposed.meta).unwrap()
    else {
        panic!("not a month")
    };
    let checked = crate::month::check(&conn, None, None, &items).unwrap();
    assert_eq!(checked.bank.len(), 1, "{:?}", checked.bank);
    assert_eq!(checked.bank[0].item, "Gnawa");
    assert_eq!(checked.register.len(), 1, "{:?}", checked.register);
    assert_eq!(checked.register[0].word, "dawn");
    assert_eq!(checked.register[0].scope, crate::register::TermScope::Title);
    let report = checked.quotas;
    let short: Vec<&str> = report
        .findings
        .iter()
        .filter_map(|found| found.title.as_deref())
        .collect();
    assert_eq!(short, vec!["Draft one"], "{report:?}");

    crate::actions::proposal::apply(&conn, &proposed.id, Default::default()).unwrap();
    let renamed = work::get(&conn, &first.id).unwrap().unwrap();
    assert_eq!(renamed.title, "Kora at dawn");
    assert_eq!(renamed.meta.get("tradition"), Some(&json!("Mande")));
    assert_eq!(
        crate::work::version::for_role(&conn, &second.id, "suno_prompt")
            .unwrap()
            .unwrap()
            .body,
        "slow strings over a warm sub bass"
    );
    let used = crate::canon::fact::get(&conn, &mande.id).unwrap().unwrap();
    assert_eq!(used.data["used_in"][0]["work_id"], json!(first.id));
    let kept = crate::register::find_word(&conn, &profile_id, "sub bass", None)
        .unwrap()
        .unwrap();
    assert_eq!(kept.scope, crate::register::TermScope::SoundCombo);

    let offer = crate::undo::last(&conn).unwrap().unwrap();
    crate::undo::undo(&conn, &offer.operation_id).unwrap();
    assert_eq!(
        work::get(&conn, &first.id).unwrap().unwrap().title,
        "Draft one"
    );
    assert!(
        !crate::canon::fact::get(&conn, &mande.id)
            .unwrap()
            .unwrap()
            .data
            .contains_key("used_in")
    );
}

/// What a release goes out under, proposed by an agent: it waits in the
/// chat on the work, every field beside what is written now, and nothing
/// is written - an agent never fills a field, even an empty one.
#[test]
fn a_proposed_release_waits_beside_what_is_written() {
    let (conn, _) = workspace();
    let video_id = video(&conn);
    let release = fixtures::release(&conn, &video_id, "youtube", None);

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_release",
        &args(json!({
            "work": video_id,
            "fields": { "title": "Harbour lights", "tags": "sea, night", "mood": "grey" },
        })),
    )
    .unwrap();

    assert!(answer.contains("Proposed 2 fields"), "{answer}");
    assert!(
        answer.contains("mood"),
        "the field it left out is named: {answer}"
    );
    let message = first_message(&conn, &video_id);
    assert_eq!(message.meta["proposal"]["kind"], "release");
    assert!(message.body.contains("Harbour lights"), "{}", message.body);
    let waiting = crate::release_meta::pending(&conn, &release.id).unwrap();
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0].client.as_deref(), Some("Claude Code"));
    assert!(
        crate::release_meta::fields(&conn, &release.id)
            .unwrap()
            .iter()
            .all(|field| field.value.is_empty()),
        "nothing is written until the person takes it"
    );
}

/// A work with two releases needs the one named; a work with none, and a
/// song, which never goes out itself, have nothing to propose for.
#[test]
fn a_release_proposal_needs_a_work_that_goes_out() {
    let (conn, song) = workspace();
    let fields = json!({ "title": "Harbour lights" });

    let none = run_tool(
        &conn,
        &claude(),
        "propose_release",
        &args(json!({ "work": song, "fields": fields })),
    )
    .unwrap_err();
    assert_eq!(none.refusal().map(|r| r.code), Some("mcp.workHasNoRelease"));
}
