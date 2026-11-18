use super::*;
use crate::fixtures;

fn note(
    conn: &Connection,
    profile_id: &str,
    title: &str,
    body: &str,
    work_id: Option<&str>,
) -> Note {
    crate::note::create(
        conn,
        profile_id,
        NewNote {
            title: Some(title.into()),
            body: body.into(),
            work_id: work_id.map(str::to_owned),
            ..NewNote::default()
        },
    )
    .unwrap()
}

fn rules() -> Rules {
    serde_json::from_value(json!({
        "cards": [{
            "title_prefix": "Place · ",
            "kind": "location",
            "headings": {
                "Look": "looks",
                "Palette": "passport:palette",
                "Prompt": "prompt:description",
                "Negative": "picture_bans",
                "Ref-Wide": "angles:wide",
                "Ref-Detail": "angles:detail"
            },
            "fallback": "identity",
            "variants": true
        }],
        "kits": {
            "title_prefix": "Banners · ",
            "archive_marker": "Archive",
            "kinds": { "Avatar": "avatar", "Watermark": "watermark" },
            "prompt": ["Prompt"],
            "negative": ["Negative"],
            "about": ["Idea"],
            "skip": ["README"]
        },
        "documents": [{ "title_prefix": "Analytics · Video review · ", "kind": "review" }],
        "splits": [{
            "title": "Analytics · Edits",
            "kind": "review",
            "note_title": "Edits",
            "work_kind": "video"
        }],
        "marks": [{
            "title": "Analytics · Covers to redraw",
            "mark": "redraw-cover",
            "kind": "review",
            "note_title": "Cover to redraw",
            "work_kind": "video"
        }]
    }))
    .unwrap()
}

#[test]
fn a_note_reads_as_parts_under_its_headings_and_a_fence_is_text() {
    let found =
        blocks("# Harbour\nintro\n## Prompt\n```\n# not a heading\n```\n## Negative\nblur\n");
    let headings: Vec<Option<&str>> = found.iter().map(|b| b.heading.as_deref()).collect();
    assert_eq!(
        headings,
        [Some("Harbour"), Some("Prompt"), Some("Negative")]
    );
    assert!(found[1].text.contains("# not a heading"));
    assert_eq!(tidy(&found[1].text), "# not a heading");
    assert_eq!(
        statements("- one\n- two\n\nA paragraph\nthat goes on."),
        ["one", "two", "A paragraph that goes on."]
    );
    assert_eq!(unnumbered("12. Harbour lights ⚠️"), "Harbour lights ⚠️");
}

#[test]
fn places_become_cards_section_by_section_and_a_variant_reads_over_its_base() {
    let (conn, profile_id) = fixtures::workspace();
    note(
        &conn,
        &profile_id,
        "Place · Harbour",
        "# The harbour at night\n\n## Look\n- wet stone\n- one lamp\n\n## Palette\nrust and fog\n\n## Prompt\n```\na harbour at night, one lamp\n```\n\n## Negative\nno people, no text\n\n## Odd heading\nkept with its words\n\n# Ref-Wide | Harbour\nthe whole quay\n",
        None,
    );
    note(
        &conn,
        &profile_id,
        "Place · Harbour-Winter",
        "## Prompt\nthe harbour under snow\n",
        None,
    );

    let planned = plan(&conn, &profile_id, &rules()).unwrap();
    assert_eq!(planned.cards.len(), 2);
    let harbour = planned.cards.iter().find(|c| c.key == "Harbour").unwrap();
    assert_eq!(harbour.title, "The harbour at night");
    let said: Vec<(String, Option<String>, String)> = harbour
        .facts
        .iter()
        .map(|f| (f.section.clone(), f.slot.clone(), f.body.clone()))
        .collect();
    assert!(
        said.contains(&("looks".into(), None, "wet stone".into())),
        "{said:?}"
    );
    assert!(said.contains(&("looks".into(), None, "one lamp".into())));
    assert!(said.contains(&(
        "passport".into(),
        Some("palette".into()),
        "rust and fog".into()
    )));
    assert!(said.contains(&(
        "prompt".into(),
        Some("description".into()),
        "a harbour at night, one lamp".into()
    )));
    assert!(said.contains(&("picture_bans".into(), None, "no people, no text".into())));
    assert!(said.contains(&(
        "identity".into(),
        None,
        "Odd heading: kept with its words".into()
    )));
    assert!(said.contains(&(
        "angles".into(),
        Some("wide".into()),
        "the whole quay".into()
    )));
    let winter = planned
        .cards
        .iter()
        .find(|c| c.key == "Harbour-Winter")
        .unwrap();
    assert_eq!(winter.variant_of.as_deref(), Some("Harbour"));

    let done = apply(&conn, &profile_id, &planned, true).unwrap();
    assert_eq!((done.cards, done.variants, done.removed), (2, 1, 2));
    let cards = crate::note::list(
        &conn,
        &profile_id,
        &NoteFilter {
            kind: Some("location".into()),
            ..NoteFilter::default()
        },
    )
    .unwrap();
    let winter = cards
        .iter()
        .find(|c| c.title.as_deref() == Some("Harbour-Winter"))
        .unwrap();
    let read =
        crate::canon::view::render(&conn, &winter.id, crate::profile::config::Lens::Cover).unwrap();
    assert!(
        read.contains("the harbour under snow") && read.contains("one lamp"),
        "{read}"
    );
    let base = cards
        .iter()
        .find(|c| c.title.as_deref() == Some("The harbour at night"))
        .unwrap();
    assert_eq!(base.aliases, ["Harbour"]);

    // Run again, nothing is made twice.
    let again = plan(&conn, &profile_id, &rules()).unwrap();
    assert!(again.cards.is_empty(), "{:?}", again.cards);
}

#[test]
fn banners_become_kits_of_the_channels_dressing() {
    let (conn, profile_id) = fixtures::workspace();
    let channel = fixtures::card(&conn, &profile_id, "channel", "The channel");
    note(
        &conn,
        &profile_id,
        "Banners · Channel-Avatar-A",
        "---\ntype: asset\nasset: avatar\n---\n# Avatar A - eyes closed\n\n## Idea\nCalm, inward.\n\n## Prompt\n```\na girl with closed eyes\n```\n\n## Negative Prompt\nno text\n",
        None,
    );
    note(
        &conn,
        &profile_id,
        "Banners · Archive · Banner-01",
        "Some words.\n\n**Prompt:** neon rain over a city\n",
        None,
    );
    note(
        &conn,
        &profile_id,
        "Banners · README",
        "How the banners work.",
        None,
    );

    let planned = plan(&conn, &profile_id, &rules()).unwrap();
    assert_eq!(planned.kits.len(), 2);
    let avatar = planned.kits.iter().find(|k| k.kind == "avatar").unwrap();
    assert_eq!(avatar.name, "Avatar A - eyes closed");
    assert_eq!(avatar.prompt.as_deref(), Some("a girl with closed eyes"));
    assert_eq!(avatar.negative.as_deref(), Some("no text"));
    assert_eq!(avatar.about.as_deref(), Some("Calm, inward."));
    let old = planned.kits.iter().find(|k| k.kind == "banner").unwrap();
    assert!(old.archived);
    assert_eq!(old.prompt.as_deref(), Some("neon rain over a city"));
    assert!(
        planned
            .passed_over
            .iter()
            .any(|one| one.ends_with("README"))
    );

    let done = apply(&conn, &profile_id, &planned, false).unwrap();
    assert_eq!((done.kits, done.removed), (2, 0));
    let kits = crate::canon::fact::in_section(&conn, &channel.id, "branding").unwrap();
    assert_eq!(kits.len(), 2);
    assert!(
        kits.iter()
            .any(|kit| kit.data.get("archived") == Some(&json!(true)))
    );

    // Run again: the channel holds both kits, none is made twice.
    let again = plan(&conn, &profile_id, &rules()).unwrap();
    assert!(again.kits.is_empty(), "{:?}", again.kits);
}

#[test]
fn reviews_edits_and_covers_to_redraw_land_on_their_works() {
    let (conn, profile_id) = fixtures::workspace();
    // A clip is titled as its song with a word in brackets (v0.147 live run).
    let clip = fixtures::video(&conn, &profile_id, "Harbour lights (video)");
    let other = fixtures::video(&conn, &profile_id, "Low water");
    let review = note(
        &conn,
        &profile_id,
        "Analytics · Video review · Harbour lights",
        "# Review\n\n## 1. The cover\nReads at a glance.",
        Some(&clip.id),
    );
    note(
        &conn,
        &profile_id,
        "Analytics · Edits",
        "# Edits of the videos\n\n## The standard\nShared rules.\n\n## 1. Harbour lights\n### Titles\nThree ways to say it.\n\n## 2. Low water\nA new description.\n\n## 3. Nowhere\nNo such work.\n",
        None,
    );
    note(
        &conn,
        &profile_id,
        "Analytics · Covers to redraw",
        "## 1. Low water ⚠️ first\nThe title is unreadable.\n",
        None,
    );

    let planned = plan(&conn, &profile_id, &rules()).unwrap();
    assert_eq!(planned.documents.len(), 1);
    assert_eq!(planned.documents[0].note_id, review.id);
    let edits: Vec<(&str, &str)> = planned
        .notes
        .iter()
        .map(|n| (n.work_title.as_str(), n.title.as_str()))
        .collect();
    assert_eq!(
        edits,
        [
            ("Harbour lights (video)", "Edits"),
            ("Low water", "Edits"),
            ("Low water", "Cover to redraw")
        ]
    );
    let harbour = planned.notes.iter().find(|n| n.work_id == clip.id).unwrap();
    assert!(harbour.body.contains("## Titles"), "{}", harbour.body);
    assert!(planned.unmatched.contains(&"The standard".to_owned()));
    assert!(planned.unmatched.contains(&"3. Nowhere".to_owned()));
    assert_eq!(planned.marks.len(), 1);
    assert_eq!(planned.marks[0].work_id, other.id);

    let done = apply(&conn, &profile_id, &planned, false).unwrap();
    assert_eq!((done.documents, done.notes, done.marks), (1, 3, 1));
    assert_eq!(
        crate::note::get(&conn, &review.id).unwrap().unwrap().kind,
        "review"
    );
    let low = crate::work::get(&conn, &other.id).unwrap().unwrap();
    assert!(low.marks.contains(&"redraw-cover".to_owned()));

    let again = plan(&conn, &profile_id, &rules()).unwrap();
    assert!(again.documents.is_empty() && again.notes.is_empty() && again.marks.is_empty());
}

#[test]
fn a_rule_naming_a_section_the_kind_lacks_is_refused_before_anything() {
    let (conn, profile_id) = fixtures::workspace();
    let wrong: Rules = serde_json::from_value(json!({
        "cards": [{ "title_prefix": "Place · ", "kind": "location", "fallback": "nowhere" }]
    }))
    .unwrap();
    let refused = plan(&conn, &profile_id, &wrong).unwrap_err();
    assert_eq!(refused.refusal().unwrap().code, "convert.unknownSection");
}

/// A place the canon holds already - by an alias, the note's own name - is
/// added to rather than made again, and what it says already is not said
/// twice (v0.147 live run: the vault's fifty places were in the canon).
#[test]
fn a_place_the_canon_holds_is_added_to_and_says_nothing_twice() {
    let (conn, profile_id) = fixtures::workspace();
    let card = fixtures::card(&conn, &profile_id, "location", "The quay at night");
    crate::note::update(
        &conn,
        &card.id,
        NotePatch {
            aliases: Some(vec!["Harbour".into()]),
            ..NotePatch::default()
        },
    )
    .unwrap();
    fixtures::fact(&conn, &card.id, "looks", "wet stone");
    note(
        &conn,
        &profile_id,
        "Place · Harbour",
        "## Look\n- wet stone\n- one lamp\n",
        None,
    );

    let planned = plan(&conn, &profile_id, &rules()).unwrap();
    assert_eq!(planned.cards.len(), 1);
    let one = &planned.cards[0];
    assert_eq!(one.existing.as_deref(), Some(card.id.as_str()));
    assert_eq!(one.title, "The quay at night");
    let bodies: Vec<&str> = one.facts.iter().map(|f| f.body.as_str()).collect();
    assert_eq!(bodies, ["one lamp"]);

    let done = apply(&conn, &profile_id, &planned, false).unwrap();
    assert_eq!((done.cards, done.facts), (0, 1));
    let again = plan(&conn, &profile_id, &rules()).unwrap();
    assert!(again.cards.is_empty());
}
