//! The rules of a space as a check reads them (v0.151, FR-20 20.2).

use rusqlite::Connection;
use serde_json::{Value, json};

use super::{Metric, Subject, check};
use crate::canon::{FactPatch, FactStatus, NewFact, fact};
use crate::collection::{self, NewCollection};
use crate::fixtures;
use crate::minted::Minted;
use crate::work::{self, NewWork};

fn channel(conn: &Connection, profile_id: &str) -> String {
    fixtures::card(conn, profile_id, "channel", "The channel").id
}

fn rule(
    conn: &Connection,
    card: &str,
    label: &str,
    data: Value,
) -> crate::Result<crate::canon::Fact> {
    fact::create_minted(
        conn,
        NewFact {
            note_id: card.to_owned(),
            section: "quotas".into(),
            body: label.to_owned(),
            data: data.as_object().cloned().unwrap(),
            ..NewFact::default()
        },
        Minted::fresh(),
    )
}

fn work_in(
    conn: &Connection,
    profile_id: &str,
    kind: &str,
    title: &str,
    collection: Option<&str>,
    tags: &[&str],
) -> work::Work {
    work::create(
        conn,
        profile_id,
        NewWork {
            kind: kind.into(),
            title: title.into(),
            collection_id: collection.map(str::to_owned),
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            ..NewWork::default()
        },
    )
    .unwrap()
}

fn code(result: crate::Result<crate::canon::Fact>) -> String {
    result.unwrap_err().refusal().unwrap().code.to_owned()
}

/// A rule is a predicate a machine can read: a field, a metric, a
/// comparison it can make. One that does not read is refused when it is
/// written, not found broken when it is checked.
#[test]
fn a_rule_that_cannot_be_measured_is_refused() {
    let (conn, profile_id) = fixtures::workspace();
    let card = channel(&conn, &profile_id);
    let cases = [
        (
            json!({ "metric": "char_len", "op": "max", "value": 3 }),
            "canon.quotaNeedsField",
        ),
        (
            json!({ "field": "title", "metric": "length", "op": "max", "value": 3 }),
            "canon.quotaBadMetric",
        ),
        (
            json!({ "field": "title", "metric": "forbidden_terms", "op": "max", "value": 3 }),
            "canon.quotaBadOp",
        ),
        (
            json!({ "field": "title", "metric": "char_len", "op": "not_contains", "value": ["a"] }),
            "canon.quotaBadOp",
        ),
        (
            json!({ "field": "title", "metric": "char_len", "op": "between", "value": [9, 2] }),
            "canon.quotaBadValue",
        ),
        (
            json!({ "field": "title", "metric": "char_len", "op": "max", "value": -1 }),
            "canon.quotaBadValue",
        ),
        (
            json!({ "field": "title", "metric": "match_count", "op": "max", "value": 1 }),
            "canon.quotaNeedsPattern",
        ),
        (
            json!({ "field": "title", "metric": "match_count", "op": "max", "value": 1, "pattern": "(" }),
            "canon.quotaBadPattern",
        ),
        (
            json!({ "field": "tags", "metric": "share", "op": "max", "value": 30 }),
            "canon.quotaNeedsSet",
        ),
        (
            json!({ "field": "tags", "metric": "share", "op": "max", "value": 130, "scope": "global" }),
            "canon.quotaBadValue",
        ),
        (
            json!({ "field": "title", "metric": "char_len", "op": "max", "value": 3, "scope": "week" }),
            "canon.quotaBadScope",
        ),
        (
            json!({ "field": "title", "metric": "char_len", "op": "max", "value": 3, "severity": "loud" }),
            "canon.quotaBadSeverity",
        ),
        (
            json!({ "field": "title", "metric": "char_len", "op": "max", "value": 3, "filter": { "field": "kind" } }),
            "canon.quotaBadFilter",
        ),
        (
            json!({ "field": "title", "metric": "char_len", "op": "max", "value": 3, "stray": 1 }),
            "canon.dataUnknownKey",
        ),
    ];
    for (data, expected) in cases {
        assert_eq!(
            code(rule(&conn, &card, "A rule", data.clone())),
            expected,
            "{data}"
        );
    }

    // One that reads is kept in one spelling: its scope, its loudness and
    // whether it is on, said even when the person said nothing.
    let kept = rule(
        &conn,
        &card,
        "Short titles",
        json!({ "field": " title ", "metric": "char_len", "op": "max", "value": 40,
                "filter": { "field": "kind", "equals": "audio" } }),
    )
    .unwrap();
    assert_eq!(
        Value::Object(kept.data),
        json!({ "field": "title", "metric": "char_len", "op": "max", "value": 40.0,
                "filter": { "field": "kind", "equals": ["audio"] },
                "scope": "item", "severity": "warn", "active": true })
    );
}

/// A rule of each work measures each work alone; one turned off, or a fact
/// retired, measures nothing.
#[test]
fn a_rule_of_each_work_names_the_works_that_break_it() {
    let (conn, profile_id) = fixtures::workspace();
    let card = channel(&conn, &profile_id);
    let long = fixtures::song(&conn, &profile_id, "A title far too long for the rule");
    fixtures::song(&conn, &profile_id, "Short");
    rule(
        &conn,
        &card,
        "Short titles",
        json!({ "field": "title", "metric": "char_len", "op": "max", "value": 10 }),
    )
    .unwrap();
    rule(
        &conn,
        &card,
        "Off",
        json!({ "field": "title", "metric": "char_len", "op": "max", "value": 1, "active": false }),
    )
    .unwrap();
    let retired = rule(
        &conn,
        &card,
        "Retired",
        json!({ "field": "title", "metric": "char_len", "op": "max", "value": 1 }),
    )
    .unwrap();
    fact::update_at(
        &conn,
        &retired.id,
        FactPatch {
            status: Some(FactStatus::Retired),
            retired_reason: Some(Some("gone".into())),
            ..FactPatch::default()
        },
        "2026-11-19T10:00:00.000Z",
    )
    .unwrap();

    let report = check(&conn, &Subject::default()).unwrap();
    assert_eq!(report.rules, 1);
    assert_eq!(report.works, 2);
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    let found = &report.findings[0];
    assert_eq!(found.work_id.as_deref(), Some(long.id.as_str()));
    assert_eq!(found.label, "Short titles");
    assert_eq!(found.measured, 33.0);
    assert_eq!(found.expected, "<= 10");

    // Asked about one work, the rule is asked of it alone.
    let alone = check(
        &conn,
        &Subject {
            work_id: Some(long.id.clone()),
            ..Subject::default()
        },
    )
    .unwrap();
    assert_eq!(alone.works, 1);
    assert_eq!(alone.findings.len(), 1);
}

/// A text a work must not say is read in its role's version, and the words
/// found are named.
#[test]
fn forbidden_words_are_found_in_a_texts_role() {
    let (conn, profile_id) = fixtures::workspace();
    let card = channel(&conn, &profile_id);
    let song = fixtures::song(&conn, &profile_id, "Harbour lights");
    fixtures::version(
        &conn,
        &song.id,
        "lyrics",
        "Neon over the harbour, neon in the rain",
    );
    rule(
        &conn,
        &card,
        "No neon",
        json!({ "field": "lyrics", "metric": "forbidden_terms", "op": "not_contains",
                "value": ["NEON", "chrome"], "severity": "error" }),
    )
    .unwrap();

    let report = check(&conn, &Subject::default()).unwrap();
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].value.as_deref(), Some("NEON"));
    assert_eq!(report.findings[0].metric, Metric::ForbiddenTerms);
}

/// A rule of a period counts the works of a collection together: how many
/// carry each value, how many values differ, which part of them match.
#[test]
fn a_rule_of_a_period_counts_a_collections_works_together() {
    let (conn, profile_id) = fixtures::workspace();
    let card = channel(&conn, &profile_id);
    let album = collection::create(
        &conn,
        &profile_id,
        NewCollection {
            kind: "album".into(),
            title: "Night drives".into(),
            description: None,
            meta: None,
            target_size: None,
            due_on: None,
        },
    )
    .unwrap();
    work_in(
        &conn,
        &profile_id,
        "song",
        "Night one",
        Some(&album.id),
        &["night", "rain"],
    );
    work_in(
        &conn,
        &profile_id,
        "song",
        "Night two",
        Some(&album.id),
        &["Night", "night"],
    );
    work_in(&conn, &profile_id, "song", "Day", Some(&album.id), &[]);
    // Outside the collection: not counted by the period, counted by the space.
    work_in(
        &conn,
        &profile_id,
        "song",
        "Elsewhere night",
        None,
        &["night"],
    );

    let period = |label: &str, data: Value| {
        let mut data = data;
        data["scope"] = json!("collection_period");
        rule(&conn, &card, label, data).unwrap();
    };
    period(
        "One night a month",
        json!({ "field": "tags", "metric": "count_per_value", "op": "max", "value": 1 }),
    );
    period(
        "Varied tags",
        json!({ "field": "tags", "metric": "unique_count", "op": "min", "value": 3 }),
    );
    period(
        "Few nights in titles",
        json!({ "field": "title", "metric": "match_count", "op": "max", "value": 1, "pattern": "\\bnight\\b" }),
    );
    period(
        "Most are tagged",
        json!({ "field": "tags", "metric": "share", "op": "min", "value": 80 }),
    );
    rule(
        &conn,
        &card,
        "Night everywhere",
        json!({ "field": "tags", "metric": "count_per_value", "op": "max", "value": 2, "scope": "global" }),
    )
    .unwrap();

    let report = check(
        &conn,
        &Subject {
            collection_id: Some(album.id.clone()),
            ..Subject::default()
        },
    )
    .unwrap();
    assert_eq!(report.works, 3);
    let by = |label: &str| {
        report
            .findings
            .iter()
            .find(|found| found.label == label)
            .unwrap_or_else(|| panic!("{label} in {:?}", report.findings))
    };
    // A work counts once for a value however often its list names it.
    assert_eq!(by("One night a month").value.as_deref(), Some("night"));
    assert_eq!(by("One night a month").measured, 2.0);
    assert_eq!(by("Varied tags").measured, 2.0);
    assert_eq!(by("Few nights in titles").measured, 2.0);
    assert!((by("Most are tagged").measured - 66.7).abs() < 0.01);
    // The space's rule counts the work outside the collection too.
    assert_eq!(by("Night everywhere").measured, 3.0);
    assert!(by("One night a month").work_id.is_none());
}

/// A month is the works going out or planned in it; a field a kind does not
/// have does not read its works, and a filter keeps the works it names.
#[test]
fn a_month_reads_its_works_and_a_rule_reads_the_works_that_have_its_field() {
    let (conn, profile_id) = fixtures::workspace();
    let card = channel(&conn, &profile_id);
    let audio = fixtures::work(&conn, &profile_id, "audio", "October audio");
    fixtures::release(&conn, &audio.id, "youtube", Some("2026-10-05"));
    let clip = fixtures::video(&conn, &profile_id, "October clip");
    fixtures::release(&conn, &clip.id, "youtube", Some("2026-10-20"));
    let later = fixtures::work(&conn, &profile_id, "audio", "November audio");
    fixtures::release(&conn, &later.id, "youtube", Some("2026-11-02"));
    let song = fixtures::song(&conn, &profile_id, "Planned song");
    crate::work_plan::create_minted(
        &conn,
        crate::work_plan::NewWorkPlan {
            work_id: song.id.clone(),
            scheduled_at: "2026-10-12".into(),
            note: None,
        },
        Minted::fresh(),
    )
    .unwrap();

    // Every YouTube title must be written: the song has no door and is not
    // read; the clip is kept out by the filter.
    rule(
        &conn,
        &card,
        "A YouTube title",
        json!({ "field": "youtube.title", "metric": "char_len", "op": "min", "value": 1,
                "filter": { "field": "kind", "equals": ["video"], "not": true } }),
    )
    .unwrap();

    let report = check(
        &conn,
        &Subject {
            month: Some("2026-10".into()),
            ..Subject::default()
        },
    )
    .unwrap();
    assert_eq!(report.works, 3, "the audio, the clip and the planned song");
    let named: Vec<&str> = report
        .findings
        .iter()
        .filter_map(|found| found.title.as_deref())
        .collect();
    assert_eq!(named, vec!["October audio"]);

    let refused = check(
        &conn,
        &Subject {
            month: Some("October".into()),
            ..Subject::default()
        },
    );
    assert_eq!(
        refused.unwrap_err().refusal().unwrap().code,
        "canon.quotaBadMonth"
    );
}

/// A rule is told to a prompt as the check measures it, and one turned off
/// is not told at all.
#[test]
fn a_prompt_is_told_the_rules_that_are_on() {
    let (conn, profile_id) = fixtures::workspace();
    let card = channel(&conn, &profile_id);
    rule(
        &conn,
        &card,
        "Suno prompt length",
        json!({ "field": "style", "metric": "char_len", "op": "between", "value": [400, 500], "severity": "error" }),
    )
    .unwrap();
    rule(
        &conn,
        &card,
        "Off",
        json!({ "field": "title", "metric": "char_len", "op": "max", "value": 1, "active": false }),
    )
    .unwrap();

    let read =
        crate::canon::view::render(&conn, &card, crate::profile::config::Lens::Work).unwrap();
    assert!(
        read.contains("- Suno prompt length: `style` characters 400-500 (each work) [must]"),
        "{read}"
    );
    assert!(!read.contains("Off"), "{read}");
}

/// An item of a bank used by a work rests its months, and a prompt is
/// handed only what is free; one undo takes the use back.
#[test]
fn a_bank_item_used_rests_and_the_undo_takes_the_use_back() {
    let (conn, profile_id) = fixtures::workspace();
    let card = channel(&conn, &profile_id);
    let item = |name: &str, data: Value| {
        fact::create_minted(
            &conn,
            NewFact {
                note_id: card.clone(),
                section: "bank".into(),
                body: name.to_owned(),
                data: data.as_object().cloned().unwrap(),
                ..NewFact::default()
            },
            Minted::fresh(),
        )
        .unwrap()
    };
    let mande = item(
        "Mande",
        json!({ "group": "West Africa", "cooldown_months": 6 }),
    );
    item("Gnawa", json!({ "group": "North Africa", "locked": true }));
    item("Fado", json!({ "group": "Iberia" }));
    let song = fixtures::song(&conn, &profile_id, "Kora at dawn");

    let today = crate::canon::bank::today();
    let day = format!(
        "{:04}-{:02}-{:02}",
        today.year(),
        u8::from(today.month()),
        today.day()
    );
    let used = crate::actions::canon::use_bank_item(&conn, &mande.id, &song.id, &day).unwrap();
    assert_eq!(used.data["last_used_at"], json!(day));
    assert_eq!(used.data["used_in"][0]["title"], json!("Kora at dawn"));

    let read =
        crate::canon::view::render(&conn, &card, crate::profile::config::Lens::Work).unwrap();
    assert!(read.contains("- Fado (Iberia)"), "{read}");
    assert!(!read.contains("Mande"), "resting: {read}");
    assert!(!read.contains("Gnawa"), "locked: {read}");
    assert!(read.contains("(2 more resting or locked"), "{read}");

    let offer = crate::undo::last(&conn).unwrap().unwrap();
    crate::undo::undo(&conn, &offer.operation_id).unwrap();
    let back = fact::get(&conn, &mande.id).unwrap().unwrap();
    assert!(!back.data.contains_key("used_in"), "{:?}", back.data);

    // A fact of another section is not an item of a bank.
    let quota = rule(
        &conn,
        &card,
        "A rule",
        json!({ "field": "title", "metric": "char_len", "op": "max", "value": 9 }),
    )
    .unwrap();
    let refused =
        crate::actions::canon::use_bank_item(&conn, &quota.id, &song.id, &day).unwrap_err();
    assert_eq!(refused.refusal().unwrap().code, "canon.notABank");
}

/// A bank that names a field of a work marks its item used when a release
/// of the work goes out, in the same unit: one undo takes back the mark and
/// the use (v0.151, FR-20 20.2).
#[test]
fn a_release_going_out_marks_the_bank_item_its_work_names() {
    let conn = crate::db::open_in_memory().unwrap();
    crate::profile::seed(&conn, "track-channel").unwrap();
    let profile_id = crate::profile::active(&conn).unwrap().unwrap().id;
    let card = channel(&conn, &profile_id);
    let mande = fact::create_minted(
        &conn,
        NewFact {
            note_id: card.clone(),
            section: "bank".into(),
            body: "Mande".into(),
            data: json!({ "cooldown_months": 6 })
                .as_object()
                .cloned()
                .unwrap(),
            ..NewFact::default()
        },
        Minted::fresh(),
    )
    .unwrap();
    let track = work::create(
        &conn,
        &profile_id,
        NewWork {
            kind: "track".into(),
            title: "Kora at dawn".into(),
            meta: Some(
                json!({ "tradition": "mande" })
                    .as_object()
                    .cloned()
                    .unwrap(),
            ),
            ..NewWork::default()
        },
    )
    .unwrap();
    let release = fixtures::release(&conn, &track.id, "youtube", Some("2026-10-05"));

    // Marked out on the day it was booked for: the use is dated that day.
    crate::actions::release::mark_released(
        &conn,
        &release.id,
        None,
        Some("2026-10-05".into()),
        None,
    )
    .unwrap();
    let used = fact::get(&conn, &mande.id).unwrap().unwrap();
    assert_eq!(used.data["last_used_at"], json!("2026-10-05"));
    assert_eq!(used.data["used_in"][0]["work_id"], json!(track.id));

    let offer = crate::undo::last(&conn).unwrap().unwrap();
    crate::undo::undo(&conn, &offer.operation_id).unwrap();
    assert!(
        !fact::get(&conn, &mande.id)
            .unwrap()
            .unwrap()
            .data
            .contains_key("used_in")
    );
    assert_eq!(
        crate::release::get(&conn, &release.id)
            .unwrap()
            .unwrap()
            .status,
        crate::release::PLANNED,
        "one undo takes back the mark with the use"
    );
}
