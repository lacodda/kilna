//! The section parser read against a real workspace, by hand.
//!
//! Ignored by default. `KILNA_LIVE_DB` points at a *copy* of one; nothing
//! here prints a line of anyone's text, only the shape it has.

#[test]
#[ignore]
fn real_lyrics_divide_into_parts() {
    let Ok(path) = std::env::var("KILNA_LIVE_DB") else {
        panic!("set KILNA_LIVE_DB to a copy of a real workspace");
    };
    let conn = rusqlite::Connection::open(&path).unwrap();
    let mut statement = conn
        .prepare("SELECT role, body FROM work_version WHERE body IS NOT NULL AND body != ''")
        .unwrap();
    let bodies: Vec<(String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    let mut divided = 0_usize;
    let mut empty_parts = 0_usize;
    let mut counts: Vec<usize> = Vec::new();
    let mut other_roles = 0_usize;

    for (role, body) in &bodies {
        let sections = kilna_lib::work::version::sections(body);
        if sections.is_empty() {
            continue;
        }
        if role == "lyrics" {
            divided += 1;
            counts.push(sections.len());
            empty_parts += sections.iter().filter(|s| s.body.is_empty()).count();
        } else {
            other_roles += 1;
        }
    }

    counts.sort_unstable();
    let middle = counts.get(counts.len() / 2).copied().unwrap_or(0);
    println!(
        "{divided} lyrics divide into parts (median {middle}, {empty_parts} parts with no lines); \
         {other_roles} bodies of other roles also carry markers"
    );
    assert!(
        divided > 300,
        "the parser must find the markup the workspace actually uses"
    );
    // A part with no lines under it is the material, not a misread: `[End]`,
    // `[Instrumental]`, `[Guitar Solo]` and a `[Chorus]` written once and
    // repeated by marker alone. They are parts of the song and become scenes
    // like any other.
    //
    // A text with a single part, though, is a line that happens to sit in
    // brackets rather than a song with one section. One such note in 346 is
    // the material too; a handful would mean the marker rule is too loose.
    let single = counts.iter().filter(|count| **count < 2).count();
    assert!(
        single <= 1,
        "{single} texts came back as one part: the marker rule is reading lines it should not"
    );
}

/// Framing a real video from its real donor, on a copy of a workspace.
///
/// Ignored by default, like its neighbour. Prints shapes and counts only —
/// never a section name or a line, which are the owner's words.
#[test]
#[ignore]
fn a_real_video_frames_from_its_donor() {
    let Ok(path) = std::env::var("KILNA_LIVE_DB") else {
        panic!("set KILNA_LIVE_DB to a copy of a real workspace");
    };
    let mut conn = rusqlite::Connection::open(&path).unwrap();
    kilna_lib::db::migrations::apply(&mut conn).unwrap();

    // A video that is made from something. The boards in a real workspace are
    // already built by hand, so the frame is compared against one: the copy's
    // scenes are cleared first, and what the frame produces is held up against
    // the count the person arrived at.
    let candidates: Vec<(String, i64)> = {
        let mut statement = conn
            .prepare(
                "SELECT w.id, (SELECT COUNT(*) FROM scene s WHERE s.work_id = w.id) FROM work w
                 WHERE w.kind IN ('video', 'short')
                   AND EXISTS (SELECT 1 FROM work_link l WHERE l.work_id = w.id)",
            )
            .unwrap();
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    println!("{} videos are made from a source", candidates.len());

    let mut framed_any = false;
    for (work_id, by_hand) in candidates {
        let Ok(parts) = kilna_lib::scene::parts_of_source(&conn, &work_id, "lyrics") else {
            continue;
        };
        if parts == 0 {
            continue;
        }
        conn.execute("DELETE FROM scene WHERE work_id = ?1", [&work_id])
            .unwrap();
        let minted: Vec<kilna_lib::minted::Minted> = (0..parts)
            .map(|_| kilna_lib::minted::Minted::fresh())
            .collect();
        let scenes =
            kilna_lib::scene::frame_from_text(&mut conn, &work_id, "lyrics", &minted, None)
                .unwrap();
        println!(
            "framed a real video into {} scenes (the person built {by_hand} by hand);              every one named and left to be described: {}",
            scenes.len(),
            scenes
                .iter()
                .all(|s| s.section.is_some() && s.description.is_empty())
        );
        assert_eq!(scenes.len(), parts);
        assert!(scenes.iter().all(|scene| scene.section.is_some()));
        framed_any = true;
        break;
    }
    assert!(
        framed_any,
        "no real video could be framed — check the donor roles"
    );
}
