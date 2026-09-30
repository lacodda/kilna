//! The canon as a person and a task meet it: cards, facts, relations, the
//! lenses, the description going stale, where a card appears, and a proposal
//! read against the canon.

use serde_json::json;

use super::proposal::{self, Defaults, FactChange};
use super::{
    FactPatch, FactStatus, Layer, NewCanonLink, NewFact, Source, SourceKind, When, fact, link, view,
};
use crate::fixtures;
use crate::minted::Minted;
use crate::note::{self, NewNote, NotePatch};
use crate::profile::config::Lens;

fn wren(conn: &rusqlite::Connection, profile_id: &str) -> note::Note {
    fixtures::card(conn, profile_id, "character", "Wren")
}

fn add(conn: &rusqlite::Connection, new: NewFact) -> crate::Result<super::Fact> {
    fact::create_minted(conn, new, Minted::fresh())
}

#[test]
fn a_note_of_a_kind_with_sections_is_a_card_and_lists_apart_from_notes() {
    let (conn, profile_id) = fixtures::workspace();
    wren(&conn, &profile_id);
    note::create(
        &conn,
        &profile_id,
        NewNote {
            body: "a stray thought".into(),
            ..NewNote::default()
        },
    )
    .unwrap();

    let cards = note::list(
        &conn,
        &profile_id,
        &note::NoteFilter {
            canon: Some(true),
            ..Default::default()
        },
    )
    .unwrap();
    let notes = note::list(
        &conn,
        &profile_id,
        &note::NoteFilter {
            canon: Some(false),
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].title.as_deref(), Some("Wren"));
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].body, "a stray thought");
}

#[test]
fn the_canon_has_one_root() {
    let (conn, profile_id) = fixtures::workspace();
    fixtures::card(&conn, &profile_id, "channel", "The channel");

    let second = note::create(
        &conn,
        &profile_id,
        NewNote {
            kind: Some("channel".into()),
            title: Some("Another".into()),
            ..NewNote::default()
        },
    );
    assert!(second.is_err(), "a second root was made");

    let other = wren(&conn, &profile_id);
    let turned = note::update(
        &conn,
        &other.id,
        NotePatch {
            kind: Some("channel".into()),
            ..NotePatch::default()
        },
    );
    assert!(turned.is_err(), "a card was turned into a second root");
}

#[test]
fn a_fact_lands_at_the_end_of_its_section_and_a_plain_note_holds_none() {
    let (conn, profile_id) = fixtures::workspace();
    let card = wren(&conn, &profile_id);
    let first = fixtures::fact(&conn, &card.id, "looks", "Long gradient hair.");
    let second = fixtures::fact(&conn, &card.id, "looks", "Freckles.");
    assert_eq!((first.position, second.position), (1, 2));

    let plain = note::create(
        &conn,
        &profile_id,
        NewNote {
            body: "plain".into(),
            ..NewNote::default()
        },
    )
    .unwrap();
    let refused = add(
        &conn,
        NewFact {
            note_id: plain.id,
            section: "looks".into(),
            body: "nope".into(),
            ..NewFact::default()
        },
    );
    assert!(refused.is_err(), "a plain note took a fact");

    let nowhere = add(
        &conn,
        NewFact {
            note_id: card.id.clone(),
            section: "horoscope".into(),
            body: "Gemini".into(),
            ..NewFact::default()
        },
    );
    assert!(
        nowhere.is_err(),
        "a fact went under a section the kind lacks"
    );

    let counted = add(
        &conn,
        NewFact {
            note_id: card.id,
            section: "where".into(),
            body: "everywhere".into(),
            ..NewFact::default()
        },
    );
    assert!(
        counted.is_err(),
        "a fact was written into a counted section"
    );
}

#[test]
fn a_retirement_says_why_and_a_live_fact_carries_no_reason() {
    let (conn, profile_id) = fixtures::workspace();
    let card = wren(&conn, &profile_id);
    let fact_row = fixtures::fact(&conn, &card.id, "looks", "A black ponytail.");

    let bare = fact::update_at(
        &conn,
        &fact_row.id,
        FactPatch {
            status: Some(FactStatus::Retired),
            ..FactPatch::default()
        },
        "2026-09-29T10:00:00Z",
    );
    assert!(bare.is_err(), "a fact was retired without a reason");

    let retired = fact::update_at(
        &conn,
        &fact_row.id,
        FactPatch {
            status: Some(FactStatus::Retired),
            retired_reason: Some(Some("the loose hair stays".into())),
            ..FactPatch::default()
        },
        "2026-09-29T10:00:00Z",
    )
    .unwrap();
    assert_eq!(
        retired.retired_reason.as_deref(),
        Some("the loose hair stays")
    );

    let back = fact::update_at(
        &conn,
        &fact_row.id,
        FactPatch {
            status: Some(FactStatus::Canon),
            ..FactPatch::default()
        },
        "2026-09-29T10:01:00Z",
    )
    .unwrap();
    assert_eq!(
        back.retired_reason, None,
        "the reason outlived the retirement"
    );
}

#[test]
fn a_source_keeps_the_work_by_id_and_a_document_by_its_name() {
    let (conn, profile_id) = fixtures::workspace();
    let card = wren(&conn, &profile_id);
    let song = fixtures::song(&conn, &profile_id, "Silurian");

    let from_song = add(
        &conn,
        NewFact {
            note_id: card.id.clone(),
            section: "tastes".into(),
            body: "Reads about deep time at night.".into(),
            source: Some(Source {
                kind: SourceKind::Work,
                work_id: Some(song.id.clone()),
                version_id: None,
                line: Some("the silurian nights".into()),
                label: None,
            }),
            ..NewFact::default()
        },
    )
    .unwrap();
    assert_eq!(
        from_song.source.as_ref().and_then(|s| s.label.as_deref()),
        Some("Silurian"),
        "the work's title stands in for it when it is gone"
    );

    let unnamed = add(
        &conn,
        NewFact {
            note_id: card.id,
            section: "bio".into(),
            body: "Born in white nights.".into(),
            source: Some(Source {
                kind: SourceKind::Document,
                work_id: None,
                version_id: None,
                line: None,
                label: None,
            }),
            ..NewFact::default()
        },
    );
    assert!(
        unnamed.is_err(),
        "a document with no name was kept as a source"
    );
}

#[test]
fn a_time_told_in_words_finds_its_place_on_the_line() {
    let (conn, profile_id) = fixtures::workspace();
    let card = wren(&conn, &profile_id);
    let dated = |body: &str, label: &str| {
        add(
            &conn,
            NewFact {
                note_id: card.id.clone(),
                section: "bio".into(),
                body: body.into(),
                when: Some(When {
                    label: Some(label.into()),
                    sort: None,
                }),
                ..NewFact::default()
            },
        )
        .unwrap()
    };
    dated("Moved into grandmother's room.", "осень 2022");
    dated("First real lyrics.", "2020");
    dated("Summers in Samara.", "every summer, 2008–2019");
    dated("A notebook of days.", "since childhood");

    let line = view::timeline(&conn, &profile_id, Some(&card.id)).unwrap();
    let order: Vec<&str> = line.iter().map(|d| d.fact.body.as_str()).collect();
    assert_eq!(
        order,
        [
            "Summers in Samara.",
            "First real lyrics.",
            "Moved into grandmother's room.",
            "A notebook of days.",
        ],
        "undated words sort last"
    );
}

#[test]
fn a_detail_carries_its_template_and_a_colour_is_a_colour() {
    let (conn, profile_id) = fixtures::workspace();
    let channel = fixtures::card(&conn, &profile_id, "channel", "The channel");

    let detail = add(
        &conn,
        NewFact {
            note_id: channel.id.clone(),
            section: "details".into(),
            body: "A pink pixel".into(),
            data: json!({ "template": "a tiny pink pixel on the left eye" })
                .as_object()
                .cloned()
                .unwrap(),
            ..NewFact::default()
        },
    )
    .unwrap();
    assert_eq!(detail.data["on"], json!(true), "a detail is on unless said");
    assert_eq!(detail.data["places"], json!(["cover", "frame", "scene"]));

    let colour = |value: &str| {
        add(
            &conn,
            NewFact {
                note_id: channel.id.clone(),
                section: "palette".into(),
                body: "accent".into(),
                data: json!({ "color": value }).as_object().cloned().unwrap(),
                ..NewFact::default()
            },
        )
    };
    assert_eq!(colour("#ff2e63").unwrap().data["color"], json!("#FF2E63"));
    assert!(colour("pink").is_err());

    let stray = add(
        &conn,
        NewFact {
            note_id: channel.id,
            section: "bans".into(),
            body: "No brands".into(),
            data: json!({ "color": "#000000" }).as_object().cloned().unwrap(),
            ..NewFact::default()
        },
    );
    assert!(stray.is_err(), "a statement kept a colour no screen shows");
}

#[test]
fn a_moved_fact_goes_to_the_end_of_its_new_section_and_reorder_takes_the_whole_section() {
    let (conn, profile_id) = fixtures::workspace();
    let card = wren(&conn, &profile_id);
    let a = fixtures::fact(&conn, &card.id, "looks", "a");
    let b = fixtures::fact(&conn, &card.id, "looks", "b");
    fixtures::fact(&conn, &card.id, "symbols", "c");

    let moved = fact::update_at(
        &conn,
        &a.id,
        FactPatch {
            section: Some("symbols".into()),
            ..FactPatch::default()
        },
        "2026-09-29T10:00:00Z",
    )
    .unwrap();
    assert_eq!(moved.position, 2);

    assert!(
        fact::reorder(&conn, &card.id, "symbols", std::slice::from_ref(&a.id), "t").is_err(),
        "an order leaving a fact out was taken"
    );
    let symbols: Vec<String> = fact::in_section(&conn, &card.id, "symbols")
        .unwrap()
        .into_iter()
        .map(|f| f.id)
        .collect();
    let reversed: Vec<String> = symbols.iter().rev().cloned().collect();
    fact::reorder(&conn, &card.id, "symbols", &reversed, "t").unwrap();
    let now: Vec<String> = fact::in_section(&conn, &card.id, "symbols")
        .unwrap()
        .into_iter()
        .map(|f| f.id)
        .collect();
    assert_eq!(now, reversed);
    assert_eq!(
        fact::in_section(&conn, &card.id, "looks").unwrap()[0].id,
        b.id
    );
}

#[test]
fn a_relation_is_one_row_per_pair_and_says_each_side_in_its_own_words() {
    let (conn, profile_id) = fixtures::workspace();
    let card = wren(&conn, &profile_id);
    let otto = fixtures::card(&conn, &profile_id, "character", "Otto");

    let drawn = link::create_minted(
        &conn,
        NewCanonLink {
            from_id: card.id.clone(),
            to_id: otto.id.clone(),
            kind: Some("neighbour".into()),
            label: Some("neighbour, first listener".into()),
            back_label: Some("neighbour".into()),
            layer: None,
        },
        Minted::fresh(),
    )
    .unwrap();
    assert_eq!(
        drawn.label_from(&card.id),
        Some("neighbour, first listener")
    );
    assert_eq!(drawn.label_from(&otto.id), Some("neighbour"));

    let again = link::create_minted(
        &conn,
        NewCanonLink {
            from_id: otto.id.clone(),
            to_id: card.id.clone(),
            ..NewCanonLink::default()
        },
        Minted::fresh(),
    );
    assert!(
        again.is_err(),
        "the pair was drawn twice, the other way round"
    );

    let unknown = link::create_minted(
        &conn,
        NewCanonLink {
            from_id: card.id.clone(),
            to_id: fixtures::card(&conn, &profile_id, "character", "Pashka").id,
            kind: Some("nemesis".into()),
            ..NewCanonLink::default()
        },
        Minted::fresh(),
    );
    assert!(
        unknown.is_err(),
        "a kind of relation the profile does not name was kept"
    );
}

#[test]
fn a_task_reads_through_its_lens_and_the_screen_says_the_same() {
    let (conn, profile_id) = fixtures::workspace();
    let card = wren(&conn, &profile_id);
    fixtures::fact(&conn, &card.id, "looks", "Freckles across the nose.");
    add(
        &conn,
        NewFact {
            note_id: card.id.clone(),
            section: "bio".into(),
            body: "Born on the seventeenth of June.".into(),
            layer: Some(Layer::Internal),
            ..NewFact::default()
        },
    )
    .unwrap();
    add(
        &conn,
        NewFact {
            note_id: card.id.clone(),
            section: "tastes".into(),
            body: "Counts the pauses.".into(),
            status: Some(FactStatus::Draft),
            ..NewFact::default()
        },
    )
    .unwrap();

    let seen = view::card(&conn, &card.id).unwrap();
    let lenses_of = |body: &str| {
        seen.facts
            .iter()
            .find(|f| f.fact.body == body)
            .map(|f| f.lenses.clone())
            .unwrap()
    };
    assert_eq!(
        lenses_of("Freckles across the nose."),
        [Lens::Cover, Lens::Work]
    );
    assert_eq!(lenses_of("Born on the seventeenth of June."), [Lens::Work]);
    assert_eq!(lenses_of("Counts the pauses."), [Lens::Work]);

    let cover = view::render(&conn, &card.id, Lens::Cover).unwrap();
    assert!(cover.contains("Freckles"));
    assert!(
        !cover.contains("seventeenth"),
        "the cover read the internal layer: {cover}"
    );
    let work = view::render(&conn, &card.id, Lens::Work).unwrap();
    assert!(
        work.contains("seventeenth") && work.contains("never the name, the date or the address")
    );
    assert!(work.contains("a draft, not settled"));
}

#[test]
fn a_card_that_publicly_does_not_exist_gives_a_public_task_nothing() {
    let (conn, profile_id) = fixtures::workspace();
    let den = note::create(
        &conn,
        &profile_id,
        NewNote {
            kind: Some("character".into()),
            title: Some("Den".into()),
            layer: Some(Layer::Internal),
            ..NewNote::default()
        },
    )
    .unwrap();
    fixtures::fact(&conn, &den.id, "identity", "A frontman.");

    assert_eq!(view::render(&conn, &den.id, Lens::Public).unwrap(), "");
    assert!(view::card(&conn, &den.id).unwrap().facts[0].lenses == [Lens::Work]);
}

#[test]
fn a_description_goes_stale_when_the_facts_it_was_written_from_change() {
    let (conn, profile_id) = fixtures::workspace();
    let card = wren(&conn, &profile_id);
    let hair = fixtures::fact(&conn, &card.id, "looks", "Gradient hair.");
    crate::actions::canon::describe(&conn, &card.id, Some("a young woman".into()), None).unwrap();
    assert!(!view::card(&conn, &card.id).unwrap().prompt_stale);

    // A fact of another section does not touch it.
    fixtures::fact(&conn, &card.id, "bio", "Born in Petersburg.");
    assert!(!view::card(&conn, &card.id).unwrap().prompt_stale);

    fact::update_at(
        &conn,
        &hair.id,
        FactPatch {
            body: Some("Gradient hair, violet to cyan.".into()),
            ..FactPatch::default()
        },
        "2026-09-29T10:00:00Z",
    )
    .unwrap();
    assert!(view::card(&conn, &card.id).unwrap().prompt_stale);
}

#[test]
fn a_card_appears_where_the_works_name_it_stage_it_or_cite_it() {
    let (conn, profile_id) = fixtures::workspace();
    let card = note::create(
        &conn,
        &profile_id,
        NewNote {
            kind: Some("character".into()),
            title: Some("Лев".into()),
            aliases: vec!["Льва".into()],
            ..NewNote::default()
        },
    )
    .unwrap();
    let named = fixtures::song(&conn, &profile_id, "Kiln");
    fixtures::version(&conn, &named.id, "lyrics", "у Льва в подвале печь");
    let unnamed = fixtures::song(&conn, &profile_id, "Other");
    fixtures::version(&conn, &unnamed.id, "lyrics", "Левон пришёл");
    let cited = fixtures::song(&conn, &profile_id, "Cited");
    add(
        &conn,
        NewFact {
            note_id: card.id.clone(),
            section: "identity".into(),
            body: "A ceramicist.".into(),
            source: Some(Source {
                kind: SourceKind::Work,
                work_id: Some(cited.id.clone()),
                version_id: None,
                line: None,
                label: None,
            }),
            ..NewFact::default()
        },
    )
    .unwrap();

    let card = note::get(&conn, &card.id).unwrap().unwrap();
    let found = super::appearances::of_card(&conn, &card).unwrap();
    let titles: Vec<&str> = found.iter().map(|a| a.title.as_str()).collect();
    assert_eq!(titles, ["Cited", "Kiln"], "Левон is not Лев");
    assert!(found.iter().any(|a| a.title == "Kiln" && a.named));
    assert!(found.iter().any(|a| a.title == "Cited" && a.facts == 1));
}

#[test]
fn a_card_reference_in_a_template_becomes_its_description() {
    let (conn, profile_id) = fixtures::workspace();
    let cat = fixtures::card(&conn, &profile_id, "character", "Tabby");
    crate::actions::canon::describe(&conn, &cat.id, Some("a small black cat".into()), None)
        .unwrap();
    let bare = fixtures::card(&conn, &profile_id, "character", "Nobody");

    let expanded = view::expand_cards(
        &conn,
        &format!(
            "hidden: [[card:{}]] and [[card:{}]] and [[card:gone]]",
            cat.id, bare.id
        ),
    )
    .unwrap();
    assert_eq!(
        expanded,
        "hidden: a small black cat and Nobody and [[card:gone]]"
    );
}

#[test]
fn a_proposal_is_read_against_the_canon_and_names_what_it_leaves_out() {
    let (conn, profile_id) = fixtures::workspace();
    let card = wren(&conn, &profile_id);
    let held = fixtures::fact(&conn, &card.id, "looks", "Loose gradient hair");
    let song = fixtures::song(&conn, &profile_id, "Elegy");

    let raw = json!({
        "cards": [{ "handle": "new-1", "kind": "character", "title": "Alex", "on_work": true }],
        "facts": [
            { "card": "Wren", "section": "looks", "text": "loose gradient hair.", "line": "her hair" },
            { "card": "new-1", "section": "identity", "text": "A courier." },
            { "card": "Wren", "section": "horoscope", "text": "Gemini" },
            { "change": "retire", "fact": held.id, "reason": "a ponytail now" },
            { "card": "Wren", "section": "looks", "text": "A ponytail.",
              "contradicts": [{ "fact": held.id, "why": "the hair is loose" }] }
        ],
        "relations": [{ "from": "new-1", "to": card.id, "label": "courier" }]
    });
    let package = proposal::read(
        &conn,
        &profile_id,
        &raw,
        &Defaults {
            work_id: Some(song.id.clone()),
            ..Defaults::default()
        },
    )
    .unwrap();

    assert_eq!(package.cards[0].work_id.as_deref(), Some(song.id.as_str()));
    assert_eq!(
        package.facts.len(),
        4,
        "the fact under a section the kind lacks is left out"
    );
    assert_eq!(package.dropped.len(), 1);
    assert_eq!(package.dropped[0].key, "refusal.canon.unknownSection");
    assert_eq!(
        package.facts[0]
            .source
            .as_ref()
            .and_then(|s| s.work_id.as_deref()),
        Some(song.id.as_str()),
        "a fact with no source cites the work it was read from"
    );
    assert_eq!(package.facts[2].change, FactChange::Retire);

    let review = proposal::review(&conn, &package).unwrap();
    assert!(
        review[0].duplicate,
        "the same words were not seen as the same fact"
    );
    assert_eq!(review[1].card_title.as_deref(), Some("Alex"));
    assert_eq!(
        review[2].target.as_ref().map(|f| f.id.as_str()),
        Some(held.id.as_str())
    );
    assert_eq!(review[3].contradicted.len(), 1);
}

#[test]
fn a_proposal_with_nothing_the_canon_can_hold_is_refused() {
    let (conn, profile_id) = fixtures::workspace();
    let raw = json!({ "facts": [{ "card": "nobody", "section": "looks", "text": "x" }] });
    assert!(proposal::read(&conn, &profile_id, &raw, &Defaults::default()).is_err());
}

#[test]
fn a_card_to_the_trash_takes_what_it_knows_and_brings_it_back() {
    use crate::trash::{self, Entity};

    let (conn, profile_id, media) = fixtures::workspace_with_media();
    let card = wren(&conn, &profile_id);
    let otto = fixtures::card(&conn, &profile_id, "character", "Otto");
    let outfit = fixtures::fact(&conn, &card.id, "outfits", "A burgundy hoodie.");
    link::create_minted(
        &conn,
        NewCanonLink {
            from_id: otto.id.clone(),
            to_id: card.id.clone(),
            ..NewCanonLink::default()
        },
        Minted::fresh(),
    )
    .unwrap();
    let picture = crate::asset::attach(
        &conn,
        &profile_id,
        media.path(),
        &fixtures::file(media.path(), "hoodie.png"),
        crate::asset::NewAsset {
            canon_fact_id: Some(outfit.id.clone()),
            kind: Some("outfit".into()),
            ..crate::asset::NewAsset::default()
        },
    )
    .unwrap();
    assert_eq!(
        picture.note_id.as_deref(),
        Some(card.id.as_str()),
        "a picture of a fact is of its card"
    );

    let entry = trash::discard(&conn, Entity::Note, &card.id).unwrap();
    assert!(fact::for_card(&conn, &card.id).unwrap().is_empty());
    assert!(link::for_card(&conn, &otto.id).unwrap().is_empty());

    trash::restore(&conn, &entry).unwrap();
    assert_eq!(fact::for_card(&conn, &card.id).unwrap().len(), 1);
    assert_eq!(link::for_card(&conn, &otto.id).unwrap().len(), 1);
    assert_eq!(crate::asset::for_card(&conn, &card.id).unwrap().len(), 1);
}

#[test]
fn a_relation_to_a_card_still_in_the_trash_waits_for_it() {
    use crate::trash::{self, Entity};

    let (conn, profile_id) = fixtures::workspace();
    let card = wren(&conn, &profile_id);
    let otto = fixtures::card(&conn, &profile_id, "character", "Otto");
    link::create_minted(
        &conn,
        NewCanonLink {
            from_id: card.id.clone(),
            to_id: otto.id.clone(),
            ..NewCanonLink::default()
        },
        Minted::fresh(),
    )
    .unwrap();

    let first = trash::discard(&conn, Entity::Note, &card.id).unwrap();
    let second = trash::discard(&conn, Entity::Note, &otto.id).unwrap();
    trash::restore(&conn, &first).unwrap();
    assert!(
        link::for_card(&conn, &card.id).unwrap().is_empty(),
        "a relation to nobody came back"
    );
    trash::restore(&conn, &second).unwrap();
    assert_eq!(
        link::for_card(&conn, &card.id).unwrap().len(),
        1,
        "the relation came back with the other end"
    );
}

#[test]
fn a_fact_waits_in_the_trash_for_its_card_and_goes_when_the_card_is_purged() {
    use crate::trash::{self, Entity};

    let (conn, profile_id) = fixtures::workspace();
    let card = wren(&conn, &profile_id);
    let one = fixtures::fact(&conn, &card.id, "looks", "Freckles.");
    let fact_entry = trash::discard(&conn, Entity::Fact, &one.id).unwrap();
    let card_entry = trash::discard(&conn, Entity::Note, &card.id).unwrap();

    assert!(
        trash::restore(&conn, &fact_entry).is_err(),
        "a fact came back onto no card"
    );
    trash::purge(&conn, &card_entry).unwrap();
    assert!(
        trash::list(&conn, &profile_id).unwrap().is_empty(),
        "the fact of a purged card stayed in the trash forever"
    );
}

#[test]
fn a_work_to_the_trash_takes_its_hero_and_all_the_hero_knows() {
    use crate::trash::{self, Entity};

    let (conn, profile_id) = fixtures::workspace();
    let song = fixtures::song(&conn, &profile_id, "Spyglass dust");
    let hero = note::create(
        &conn,
        &profile_id,
        NewNote {
            kind: Some("character".into()),
            title: Some("Alex".into()),
            work_id: Some(song.id.clone()),
            ..NewNote::default()
        },
    )
    .unwrap();
    fixtures::fact(&conn, &hero.id, "identity", "A courier.");

    let entry = trash::discard(&conn, Entity::Work, &song.id).unwrap();
    assert!(note::get(&conn, &hero.id).unwrap().is_none());
    trash::restore(&conn, &entry).unwrap();
    assert_eq!(
        fact::for_card(&conn, &hero.id).unwrap().len(),
        1,
        "the hero came back knowing nothing"
    );
}

#[test]
fn a_fact_is_found_by_its_words_and_opens_its_card() {
    let (conn, profile_id) = fixtures::workspace();
    let card = wren(&conn, &profile_id);
    fixtures::fact(
        &conn,
        &card.id,
        "tastes",
        "Counts the pauses in transcripts.",
    );

    let hits = crate::search::find(&conn, &profile_id, "pauses").unwrap();
    let hit = hits
        .iter()
        .find(|hit| hit.kind == crate::search::Kind::Fact)
        .expect("the fact was found");
    assert_eq!(hit.card_id.as_deref(), Some(card.id.as_str()));
    assert!(
        crate::search::works_matching(&conn, &profile_id, "pauses")
            .unwrap()
            .is_empty(),
        "a fact made a work match"
    );
}

#[test]
fn two_works_trashed_one_after_the_other_come_back_linked() {
    use crate::trash::{self, Entity};

    let (conn, profile_id) = fixtures::workspace();
    let song = fixtures::song(&conn, &profile_id, "Spyglass dust");
    let clip = fixtures::video(&conn, &profile_id, "Spyglass dust - clip");
    crate::link::create(
        &conn,
        &profile_id,
        crate::link::NewLink {
            work_id: clip.id.clone(),
            source_id: song.id.clone(),
            role: None,
            source_version_id: None,
        },
    )
    .unwrap();

    let first = trash::discard(&conn, Entity::Work, &song.id).unwrap();
    let second = trash::discard(&conn, Entity::Work, &clip.id).unwrap();
    trash::restore(&conn, &first).unwrap();
    trash::restore(&conn, &second).unwrap();

    assert_eq!(
        crate::link::for_work(&conn, &clip.id)
            .unwrap()
            .sources
            .len(),
        1,
        "the link was spent with the first entry"
    );
}
