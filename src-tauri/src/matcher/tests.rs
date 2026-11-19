use super::*;
use crate::fixtures;

fn works() -> (Connection, Vec<Work>) {
    let (conn, profile_id) = fixtures::workspace();
    let dawn = fixtures::work(&conn, &profile_id, "video", "Dawn over the Harbour");
    let dusk = fixtures::work(&conn, &profile_id, "video", "Dusk");
    let mut lantern = fixtures::work(&conn, &profile_id, "video", "Ёлка");
    lantern.cover.scene = "Paper lanterns over a river at night, warm light".into();
    (conn, vec![dawn, dusk, lantern])
}

fn by_name<'a>(matched: &'a [MediaMatch], name: &str) -> &'a MediaMatch {
    matched.iter().find(|one| one.name == name).unwrap()
}

#[test]
fn words_are_compared_without_case_punctuation_or_yo() {
    assert_eq!(
        normalized("Dawn: over the HARBOUR!"),
        "dawn over the harbour"
    );
    assert_eq!(normalized("Ёлка_у_дома"), "елка у дома");
    assert_eq!(normalized("  --  "), "");
}

#[test]
fn a_file_named_by_its_works_id_is_that_works() {
    let (_conn, works) = works();
    let short = &works[1].id[..8];
    let matched = match_files(&[format!("C:/dl/render-{short}.mp4")], &works);

    assert_eq!(matched[0].work_id.as_deref(), Some(works[1].id.as_str()));
    assert_eq!(matched[0].by, Some(MatchedBy::Id));
    assert_eq!(matched[0].role, MediaRole::Video);
}

#[test]
fn a_title_in_the_name_matches_the_longest_title_it_holds() {
    let (_conn, works) = works();
    let matched = match_files(
        &[
            "C:/dl/Dawn over the Harbour.mp3".to_owned(),
            "C:/dl/dusk-final-mix.wav".to_owned(),
            "C:/dl/ёлка (1).mp3".to_owned(),
            "C:/dl/ёлка.mp3".to_owned(),
        ],
        &works,
    );

    let dawn = by_name(&matched, "Dawn over the Harbour.mp3");
    assert_eq!(dawn.work_id.as_deref(), Some(works[0].id.as_str()));
    assert_eq!(dawn.by, Some(MatchedBy::Title));
    assert_eq!(dawn.role, MediaRole::FinalAudio);
    let dusk = by_name(&matched, "dusk-final-mix.wav");
    assert_eq!(dusk.work_id.as_deref(), Some(works[1].id.as_str()));
    let again = by_name(&matched, "ёлка (1).mp3");
    assert_eq!(again.work_id.as_deref(), Some(works[2].id.as_str()));
    assert!(again.duplicate, "a second download is marked");
    assert_eq!(
        again.role,
        MediaRole::CandidateAudio,
        "a duplicate is no final"
    );
    let original = by_name(&matched, "ёлка.mp3");
    assert_eq!(
        original.role,
        MediaRole::FinalAudio,
        "a duplicate sorted first takes no place from the original"
    );
}

#[test]
fn a_picture_named_after_its_cover_prompt_is_that_works_cover() {
    let (_conn, works) = works();
    let matched = match_files(
        &[
            "C:/dl/Paper_lanterns_over_a_river_at_night.png".to_owned(),
            "C:/dl/paper_lanterns_over_a.webp".to_owned(),
            "C:/dl/202604112318.png".to_owned(),
        ],
        &works,
    );

    let first = by_name(&matched, "Paper_lanterns_over_a_river_at_night.png");
    assert_eq!(first.work_id.as_deref(), Some(works[2].id.as_str()));
    assert_eq!(first.by, Some(MatchedBy::CoverPrompt));
    assert_eq!(first.role, MediaRole::FinalCover);
    let second = by_name(&matched, "paper_lanterns_over_a.webp");
    assert_eq!(
        second.role,
        MediaRole::CandidateCover,
        "the next one is to choose from"
    );
    let blind = by_name(&matched, "202604112318.png");
    assert_eq!(blind.work_id, None, "a moment for a name is not guessed");
    assert_eq!(blind.kind, "image");
}

#[test]
fn a_name_two_works_answer_equally_is_left_to_the_person() {
    let (conn, mut works) = works();
    let profile_id = works[0].profile_id.clone();
    works.push(fixtures::work(&conn, &profile_id, "video", "Dusk"));
    let matched = match_files(&["C:/dl/Dusk.mp3".to_owned()], &works);

    assert_eq!(matched[0].work_id, None);
    assert_eq!(matched[0].ambiguous.len(), 2);
}
