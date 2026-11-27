//! Works read and proposed: the catalogue, a card, a text, a version, a score, a package.

use super::*;

#[test]
fn a_work_is_found_by_id_or_by_exact_title_and_read_with_its_text() {
    let (conn, work_id) = workspace();
    let by_title = run_tool(
        &conn,
        &claude(),
        "work",
        &args(json!({ "work": "Harbour lights" })),
    )
    .unwrap();
    let by_id = run_tool(&conn, &claude(), "work", &args(json!({ "work": work_id }))).unwrap();
    assert_eq!(by_title, by_id);
    let card: Value = serde_json::from_str(&by_id).unwrap();
    assert_eq!(card["versions"]["lyrics"][0]["revision"], 1);
    assert!(card.get("body").is_none(), "the card must not carry bodies");

    let text = run_tool(&conn, &claude(), "text", &args(json!({ "work": work_id }))).unwrap();
    let text: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(text["body"], "one line\ntwo lines");
    assert_eq!(text["reads_as"], "plain");
}

/// `text` reads a version by its id only when it is the named work's: a
/// version of another work is refused, never handed over under the wrong
/// title - and a role the kind lacks is refused by name.
#[test]
fn a_version_is_read_by_id_only_from_its_own_work() {
    let (conn, work_id) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let other = fixtures::song(&conn, &profile_id, "Night ferry").id;
    let first = fixtures::version(&conn, &work_id, "lyrics", "an older text");
    let foreign = fixtures::version(&conn, &other, "lyrics", "somebody else's text");

    let own = read(
        &conn,
        "text",
        json!({ "work": work_id, "version": first.id }),
    );
    assert_eq!(own["version"], json!(first.id));
    assert_eq!(own["body"], "an older text");

    let refused = run_tool(
        &conn,
        &claude(),
        "text",
        &args(json!({ "work": work_id, "version": foreign.id })),
    )
    .unwrap_err();
    assert_eq!(refused.refusal().unwrap().code, "mcp.versionOtherWork");

    let refused = run_tool(
        &conn,
        &claude(),
        "text",
        &args(json!({ "work": work_id, "role": "sonnet" })),
    )
    .unwrap_err();
    assert_eq!(refused.refusal().unwrap().code, "mcp.unknownVersionRole");
}

/// The catalogue is narrowed by a substring of the title, by kind and by
/// status, each leaving only the works that match.
#[test]
fn the_catalogue_is_narrowed_by_title_kind_and_status() {
    let (conn, song) = workspace();
    let clip = video(&conn);
    let ids = |with: Value| -> Vec<String> {
        read(&conn, "catalogue", with)["works"]
            .as_array()
            .unwrap()
            .iter()
            .map(|work| work["id"].as_str().unwrap().to_owned())
            .collect()
    };
    let all = read(&conn, "catalogue", json!({}));
    assert_eq!(all["total"], 2);
    let status = all["works"]
        .as_array()
        .unwrap()
        .iter()
        .find(|work| work["id"] == json!(song))
        .unwrap()["status"]
        .as_str()
        .unwrap()
        .to_owned();

    assert_eq!(ids(json!({ "kind": "song" })), [song.as_str()]);
    assert_eq!(ids(json!({ "kind": "video" })), [clip.as_str()]);
    assert_eq!(ids(json!({ "query": "THE CLIP" })), [clip]);
    assert!(ids(json!({ "kind": "song", "status": status })).contains(&song));
    assert!(ids(json!({ "status": "no-such-status" })).is_empty());
    assert!(ids(json!({ "kind": "no-such-kind" })).is_empty());
}

/// The long readings come a page at a time (FR-08 8.4): how many there
/// are, whether more follow, and the next page by its offset - so an
/// agent walks a record of thousands rather than swallowing it.
#[test]
fn a_long_reading_comes_a_page_at_a_time() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    for n in 1..=6 {
        fixtures::song(&conn, &profile_id, &format!("Song {n}"));
    }

    let first = read(&conn, "catalogue", json!({ "limit": 3 }));
    assert_eq!(first["total"], 7);
    assert_eq!(
        (first["limit"].clone(), first["offset"].clone()),
        (json!(3), json!(0))
    );
    assert_eq!(first["has_more"], true);
    assert_eq!(first["works"].as_array().unwrap().len(), 3);

    let last = read(&conn, "catalogue", json!({ "limit": 3, "offset": 6 }));
    assert_eq!(last["works"].as_array().unwrap().len(), 1);
    assert_eq!(last["has_more"], false);

    // Every work once across the pages, none twice.
    let mut seen: Vec<String> = Vec::new();
    for offset in [0, 3, 6] {
        let page = read(&conn, "catalogue", json!({ "limit": 3, "offset": offset }));
        seen.extend(
            page["works"]
                .as_array()
                .unwrap()
                .iter()
                .map(|work| work["id"].as_str().unwrap().to_owned()),
        );
    }
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 7);

    // A filter narrows what is counted, not only what is shown.
    let narrowed = read(&conn, "catalogue", json!({ "query": "song 1" }));
    assert_eq!(narrowed["total"], 1);
    // And a page can ask for no more than the most.
    let greedy = read(&conn, "catalogue", json!({ "limit": 100_000 }));
    assert_eq!(greedy["limit"], MOST);
}

#[test]
fn an_agent_reads_the_lines_of_a_vibe_proposes_one_and_finds_works_by_it() {
    let (conn, work_id) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;

    let said = run_tool(&conn, &claude(), "workspace", &Map::new()).unwrap();
    let said: Value = serde_json::from_str(&said).unwrap();
    assert_eq!(said["vibe"]["moods"].as_array().unwrap().len(), 5);
    assert_eq!(said["vibe"]["complexities"][2]["strokes"], 3);

    // A word the profile does not have is refused, with the ones it has.
    let refused = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({ "work": work_id, "vibe": { "mood": "sunny" } })),
    )
    .unwrap_err();
    assert_eq!(refused.refusal().map(|r| r.code), Some("work.unknownVibe"));

    // A vibe alone is a package worth proposing; kept, it is the work's.
    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({ "work": work_id, "vibe": { "mood": "dark", "character": "edgy" } })),
    )
    .unwrap();
    assert!(answer.contains("Proposed"), "{answer}");
    let message = first_message(&conn, &work_id);
    assert!(message.body.contains("Dark · Raw"), "{}", message.body);
    crate::actions::proposal::apply(
        &conn,
        &message.id,
        crate::actions::proposal::Overrides::default(),
    )
    .unwrap();

    let catalogue = |filter: Value| -> Value {
        serde_json::from_str(&run_tool(&conn, &claude(), "catalogue", &args(filter)).unwrap())
            .unwrap()
    };
    let found = catalogue(json!({ "mood": "dark" }));
    assert_eq!(found["works"].as_array().unwrap().len(), 1);
    assert_eq!(found["works"][0]["vibe"]["vibe"]["character"], "edgy");
    assert_eq!(
        catalogue(json!({ "mood": "joyful" }))["works"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    let work: Value = serde_json::from_str(
        &run_tool(&conn, &claude(), "work", &args(json!({ "work": work_id }))).unwrap(),
    )
    .unwrap();
    assert_eq!(work["vibe"]["mood"], "dark");
    assert_eq!(work["shown_vibe"]["vibe"]["mood"], "dark");
    let _ = profile_id;
}

#[test]
fn a_proposed_version_is_a_message_in_the_clients_chat_not_a_version() {
    let (conn, work_id) = workspace();
    let before = version::list(&conn, &work_id).unwrap().len();

    run_tool(
        &conn,
        &claude(),
        "propose_version",
        &args(json!({ "work": work_id, "role": "lyrics", "body": "one line\ntwo lines\nthree", "note": "added a third" })),
    )
    .unwrap();

    assert_eq!(
        version::list(&conn, &work_id).unwrap().len(),
        before,
        "a proposal is not a version"
    );
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, Some(&work_id)).unwrap();
    assert_eq!(chats.len(), 1);
    assert_eq!(chats[0].title.as_deref(), Some("Claude Code"));
    let transcript = assistant::transcript(&conn, &chats[0].id).unwrap().unwrap();
    let message = &transcript.messages[0];
    assert_eq!(message.role, "assistant");
    assert_eq!(
        message.body, "one line\ntwo lines\nthree",
        "the body is the text, verbatim"
    );
    assert_eq!(message.meta["proposal"]["kind"], "version");
    assert_eq!(message.meta["proposal"]["role"], "lyrics");
    assert_eq!(message.meta["note"], "added a third");
    assert_eq!(message.meta["source"], "mcp");
}

/// The version an agent says it rewrote travels with the proposal, and
/// the work's card names each version's parent; a version of another
/// role is refused now, in words, rather than dropped at apply time.
#[test]
fn a_proposed_version_names_what_it_was_written_from() {
    let (conn, work_id) = workspace();
    let lyric = version::latest(&conn, &work_id, "lyrics")
        .unwrap()
        .unwrap()
        .id;

    run_tool(
        &conn,
        &claude(),
        "propose_version",
        &args(json!({ "work": work_id, "role": "lyrics", "body": "sharper", "from": lyric })),
    )
    .unwrap();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, Some(&work_id)).unwrap();
    let transcript = assistant::transcript(&conn, &chats[0].id).unwrap().unwrap();
    assert_eq!(
        transcript.messages[0].meta["proposal"]["from"],
        lyric.as_str()
    );

    let refused = run_tool(
        &conn,
        &claude(),
        "propose_version",
        &args(json!({ "work": work_id, "role": "style", "body": "warm", "from": lyric })),
    )
    .unwrap_err();
    assert_eq!(
        refused.refusal().map(|r| r.code),
        Some("version.parentNotSameRole"),
        "{refused}"
    );

    let card = run_tool(&conn, &claude(), "work", &args(json!({ "work": work_id }))).unwrap();
    let card: Value = serde_json::from_str(&card).unwrap();
    assert!(card["versions"]["lyrics"][0]["parent"].is_null());
}

#[test]
fn a_proposed_score_is_checked_against_the_profile_and_never_written() {
    let (conn, work_id) = workspace();
    let answer = run_tool(
        &conn,
        &claude(),
        "propose_score",
        &args(json!({ "work": work_id, "axes": { "hook": 7, "lyrics": 11, "sparkle": 3 }, "note": "strong chorus" })),
    )
    .unwrap();
    assert!(
        answer.contains("sparkle"),
        "unknown axes are named: {answer}"
    );
    assert!(
        score::history(&conn, &work_id).unwrap().is_empty(),
        "a proposal is not a score"
    );

    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chat = assistant::summaries(&conn, &profile_id, Some(&work_id))
        .unwrap()
        .remove(0);
    let message = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    assert_eq!(message.meta["proposal"]["kind"], "score");
    assert_eq!(message.meta["proposal"]["axes"]["hook"], 7.0);
    assert_eq!(
        message.meta["proposal"]["axes"]["lyrics"], 10.0,
        "clamped to the scale"
    );
    assert_eq!(message.meta["proposal"]["unknown"][0], "sparkle");
}

#[test]
fn a_score_naming_no_known_axis_is_refused() {
    let (conn, work_id) = workspace();
    let err = run_tool(
        &conn,
        &claude(),
        "propose_score",
        &args(json!({ "work": work_id, "axes": { "sparkle": 3 } })),
    )
    .unwrap_err();
    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("mcp.noKnownAxes"),
        "{err}"
    );
}

#[test]
fn a_proposed_work_waits_in_the_clients_chat_on_nothing_with_everything_readable() {
    let (conn, _) = workspace();
    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "title": "Winter road", "kind": "song",
            "fields": { "premise": "a road at night", "colour": "blue" },
            "versions": [{ "role": "lyrics", "body": "snow on the road", "label": "first" }],
            "score": { "axes": { "hook": 7, "sparkle": 2 }, "note": "quiet" },
            "notes": [{ "title": "Concept", "body": "the road as witness" }]
        })),
    )
    .unwrap();
    assert!(answer.contains("Proposed a new work"), "{answer}");
    assert!(
        answer.contains("colour"),
        "ignored fields are named: {answer}"
    );
    assert!(
        answer.contains("sparkle"),
        "ignored axes are named: {answer}"
    );

    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    assert_eq!(
        work::list(&conn, &profile_id, &WorkFilter::default())
            .unwrap()
            .len(),
        1,
        "a proposal is not a work"
    );
    let chats = assistant::summaries(&conn, &profile_id, None).unwrap();
    let chat = chats
        .iter()
        .find(|c| c.work_id.is_none())
        .expect("a chat on nothing");
    assert_eq!(chat.title.as_deref(), Some("Claude Code"));
    let message = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    assert_eq!(message.meta["proposal"]["kind"], "work");
    assert_eq!(message.meta["proposal"]["title"], "Winter road");
    assert_eq!(
        message.meta["proposal"]["fields"]["premise"],
        "a road at night"
    );
    assert!(message.meta["proposal"]["fields"].get("colour").is_none());
    assert_eq!(
        message.meta["proposal"]["versions"][0]["body"],
        "snow on the road"
    );
    assert_eq!(message.meta["proposal"]["score"]["axes"]["hook"], 7.0);
    assert!(
        message.body.contains("snow on the road"),
        "readable before applied: {}",
        message.body
    );
    assert!(message.body.contains("Winter road"), "{}", message.body);
    assert!(apply::is_pending(&message));

    let lines: i64 = conn
        .query_row(
            "SELECT count(*) FROM journal WHERE action = 'proposal.work'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(lines, 1);
}

#[test]
fn a_package_for_a_work_joins_its_chat() {
    let (conn, work_id) = workspace();
    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(
            json!({ "work": work_id, "title": "ignored", "kind": "video",
                       "versions": [{ "role": "style", "body": "brushed drums" }] }),
        ),
    )
    .unwrap();
    assert!(
        answer.contains("Proposed a package for “Harbour lights”"),
        "{answer}"
    );

    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, Some(&work_id)).unwrap();
    assert_eq!(chats.len(), 1);
    let message = assistant::transcript(&conn, &chats[0].id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    assert_eq!(message.meta["proposal"]["kind"], "work");
    assert!(
        message.meta["proposal"].get("title").is_none(),
        "an existing work keeps its title"
    );
    assert_eq!(message.meta["proposal"]["kind"], "work");
    assert_eq!(message.meta["proposal"]["versions"][0]["role"], "style");
}

#[test]
fn a_package_that_names_a_role_the_kind_lacks_is_refused_whole() {
    let (conn, work_id) = workspace();
    let err = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({ "work": work_id, "versions": [{ "role": "storyboard", "body": "x" }] })),
    )
    .unwrap_err();
    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("mcp.unknownVersionRole"),
        "{err}"
    );
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    assert!(
        assistant::summaries(&conn, &profile_id, Some(&work_id))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_new_work_needs_a_title_and_a_kind_the_profile_has() {
    let (conn, _) = workspace();
    let err = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({ "kind": "song" })),
    )
    .unwrap_err();
    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("proposal.newWorkNeedsTitle"),
        "{err}"
    );
    let err = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({ "title": "x", "kind": "sculpture" })),
    )
    .unwrap_err();
    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("work.unknownKind"),
        "{err}"
    );
}

#[test]
fn an_empty_package_for_a_work_is_refused() {
    let (conn, work_id) = workspace();
    let err = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({ "work": work_id })),
    )
    .unwrap_err();
    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("mcp.emptyPackage"),
        "{err}"
    );
}

#[test]
fn a_package_can_plan_a_release_and_say_what_it_goes_out_as() {
    let (conn, _) = workspace();

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "title": "Winter road — the clip", "kind": "video",
            "versions": [{ "role": "plot", "body": "a road, a car, a light" }],
            "releases": [{
                "kind": "youtube",
                "scheduled_at": "2026-10-02",
                "fields": {
                    "title": "Winter road",
                    "description": "a road, a car, a light",
                    "tags": "winter, road, night",
                },
            }],
        })),
    )
    .unwrap();

    assert!(answer.contains("1 release (youtube)"), "{answer}");

    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, None).unwrap();
    let chat = chats.iter().find(|c| c.work_id.is_none()).unwrap();
    let message = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);

    assert_eq!(message.meta["proposal"]["releases"][0]["kind"], "youtube");
    assert_eq!(
        message.meta["proposal"]["releases"][0]["fields"]["title"],
        "Winter road"
    );
    assert!(
        message.body.contains("### YouTube — 2026-10-02"),
        "the kind's own word and its day, in what the person reads before applying: {}",
        message.body
    );
    assert!(message.body.contains("Winter road"), "{}", message.body);
}

#[test]
fn a_release_of_a_kind_this_work_does_not_ship_refuses_the_package() {
    let (conn, _) = workspace();

    let err = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "title": "Winter road", "kind": "song",
            "releases": [{ "kind": "youtube" }],
        })),
    )
    .unwrap_err();

    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("release.noDoors"),
        "a song ships nothing itself - its clips, audio and shorts do: {err}"
    );
}

#[test]
fn a_release_field_the_kind_does_not_have_is_named_and_left_out() {
    let (conn, _) = workspace();

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "title": "Winter road — the clip", "kind": "video",
            "releases": [{
                "kind": "premiere",
                // `premiere` ships a title and a description, not tags.
                "fields": { "title": "Winter road", "tags": "winter" },
            }],
        })),
    )
    .unwrap();

    assert!(
        answer.contains("`premiere` fields ignored (not of that kind of release): tags"),
        "a package that only half fits is worth applying, but not silently: {answer}"
    );

    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let chats = assistant::summaries(&conn, &profile_id, None).unwrap();
    let chat = chats.iter().find(|c| c.work_id.is_none()).unwrap();
    let message = assistant::transcript(&conn, &chat.id)
        .unwrap()
        .unwrap()
        .messages
        .remove(0);
    assert!(
        message.meta["proposal"]["releases"][0]["fields"]
            .get("tags")
            .is_none(),
        "a value under a key no box will ever show is not stored"
    );
}

#[test]
fn a_day_that_is_not_a_day_is_refused() {
    let (conn, _) = workspace();

    let err = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "title": "Winter road — the clip", "kind": "video",
            "releases": [{ "kind": "youtube", "scheduled_at": "next Thursday" }],
        })),
    )
    .unwrap_err();

    assert_eq!(
        err.refusal().map(|r| r.code),
        Some("mcp.badReleaseDate"),
        "{err}"
    );
}

#[test]
fn a_package_holding_only_a_release_is_not_empty() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let work = crate::work::create(
        &conn,
        &profile_id,
        crate::work::NewWork {
            kind: "video".into(),
            title: "Winter road — the clip".into(),
            ..crate::work::NewWork::default()
        },
    )
    .unwrap();

    let answer = run_tool(
        &conn,
        &claude(),
        "propose_work",
        &args(json!({
            "work": work.id,
            "releases": [{ "kind": "youtube" }],
        })),
    )
    .unwrap();

    assert!(answer.contains("1 release"), "{answer}");
}

#[test]
fn a_card_is_read_short_and_its_versions_a_page_at_a_time() {
    let (conn, _) = workspace();
    let profile_id = profile::active(&conn).unwrap().unwrap().id;
    let song = fixtures::song(&conn, &profile_id, "Tide");
    for (n, current) in [(1, false), (2, false), (3, true)] {
        crate::work::version::create(
            &conn,
            &song.id,
            crate::work::version::NewVersion {
                role: "lyrics".into(),
                body: format!("draft {n}"),
                make_current: current,
                ..Default::default()
            },
        )
        .unwrap();
    }

    let short = read(&conn, "work", json!({ "work": song.id, "summary": true }));
    assert_eq!(short["title"], "Tide");
    assert_eq!(short["roles"]["lyrics"]["versions"], 3);
    assert!(short["roles"]["lyrics"]["current"].is_string());
    assert!(short.get("versions").is_none() && short.get("cover").is_none());
    assert_eq!(short["notes"], 0);

    let page = read(&conn, "work", json!({ "work": song.id, "limit": 2 }));
    assert_eq!(page["total_versions"], 3);
    assert_eq!(page["has_more_versions"], true);
    assert_eq!(page["versions"]["lyrics"].as_array().unwrap().len(), 2);
}
