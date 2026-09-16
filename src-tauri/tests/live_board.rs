//! Renumbering and checking a real storyboard, by hand.
//!
//! Ignored by default: it needs a workspace to point at, which CI has none of.
//! Run it before a release with `KILNA_LIVE_DB` set to a *copy* of one.
//!
//! What it is for: the counts and the shift are arithmetic over a board, and
//! a board built by fixtures is a board built by whoever wrote the fixtures.
//! A real one has scenes nobody finished, spans nobody typed, and pictures
//! hanging off scenes that were reordered months ago — the shapes a test
//! invents last.

use std::collections::BTreeMap;

/// The board of the work with the most scenes: renumbered, checked, put back.
///
/// Puts the board back where it found it before finishing, so a copy can be
/// run against twice and so a mistaken run against something that is not a
/// copy does less harm. That is a courtesy, not a guarantee — the file is
/// still written to, which is why the variable says *copy*.
#[test]
#[ignore]
fn a_real_board_is_renumbered_and_comes_back() {
    let Ok(path) = std::env::var("KILNA_LIVE_DB") else {
        panic!("set KILNA_LIVE_DB to a copy of a real workspace");
    };
    let mut conn = rusqlite::Connection::open(&path).unwrap();
    let schema = kilna_lib::db::migrations::apply(&mut conn).unwrap();
    println!("schema now {schema}");

    // The busiest board in the workspace: the one most likely to be crooked.
    let (work_id, title, count): (String, String, i64) = conn
        .query_row(
            "SELECT w.id, w.title, count(s.id) AS scenes FROM work w
             JOIN scene s ON s.work_id = w.id
             GROUP BY w.id ORDER BY scenes DESC, w.rowid LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("the workspace has at least one board");
    println!("board of “{title}”: {count} scenes");

    let numbers = |conn: &rusqlite::Connection| -> Vec<(String, i64)> {
        kilna_lib::scene::for_work(conn, &work_id)
            .unwrap()
            .into_iter()
            .map(|scene| (scene.id, scene.position))
            .collect()
    };
    let before = numbers(&conn);

    // What the board holds, so it can be shown to have held it afterwards.
    let material_of = |conn: &rusqlite::Connection| -> BTreeMap<String, Vec<String>> {
        before
            .iter()
            .map(|(id, _)| {
                let held: Vec<String> = kilna_lib::scene_frame::for_scene(conn, id)
                    .unwrap()
                    .into_iter()
                    .map(|frame| format!("{}:{}:{}", frame.kind, frame.is_selected, frame.path))
                    .collect();
                (id.clone(), held)
            })
            .collect()
    };
    let held = material_of(&conn);
    let pictures: usize = held.values().map(Vec::len).sum();
    let spans: Vec<(Option<f64>, Option<f64>)> = kilna_lib::scene::for_work(&conn, &work_id)
        .unwrap()
        .into_iter()
        .map(|scene| (scene.starts_at, scene.ends_at))
        .collect();
    println!("it carries {pictures} pictures and clips");

    // Turn the board back to front: the biggest move a board of this size
    // can be asked for, and the one that would expose an off-by-one.
    let at = kilna_lib::time::now();
    let reversed: Vec<String> = before.iter().rev().map(|(id, _)| id.clone()).collect();
    kilna_lib::scene::renumber(&mut conn, &work_id, &reversed, &at, None).unwrap();

    let after = numbers(&conn);
    assert_eq!(
        after.iter().map(|(_, at)| *at).collect::<Vec<_>>(),
        (1..=count).collect::<Vec<_>>(),
        "a real board comes out numbered 1..N with no hole and no twin"
    );
    assert_eq!(
        after.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>(),
        reversed,
        "and in the order it was given"
    );
    assert_eq!(
        material_of(&conn),
        held,
        "every picture and clip is still on the scene it was on, unrenamed"
    );
    assert_eq!(
        kilna_lib::scene::for_work(&conn, &work_id)
            .unwrap()
            .into_iter()
            .map(|scene| (scene.starts_at, scene.ends_at))
            .collect::<Vec<_>>(),
        // The spans belong to the SCENES, so reversing the board reverses the
        // list of them. Each scene kept its own.
        spans.iter().rev().copied().collect::<Vec<_>>(),
        "no scene's seconds were touched; they only travelled with their rows"
    );

    // Back the way it was, numbers and all.
    let places: Vec<(String, i64)> = before.clone();
    kilna_lib::scene::restore_numbers_in(&conn, &places, &at).unwrap();
    assert_eq!(
        numbers(&conn),
        before,
        "the board is back where it was found"
    );
    println!("renumbered {count} scenes and put them back");
}

/// What the real boards still owe, counted the way the screen counts it.
///
/// Prints rather than asserts, mostly: the point is to see the numbers a real
/// workspace produces and to find out whether the report is readable or a
/// wall. The one thing it does hold to is that counting cannot exceed the
/// board — a tally larger than the number of scenes would be a double count.
#[test]
#[ignore]
fn the_real_boards_are_counted() {
    let Ok(path) = std::env::var("KILNA_LIVE_DB") else {
        panic!("set KILNA_LIVE_DB to a copy of a real workspace");
    };
    let mut conn = rusqlite::Connection::open(&path).unwrap();
    kilna_lib::db::migrations::apply(&mut conn).unwrap();

    let mut statement = conn
        .prepare(
            "SELECT w.id, w.title, count(s.id) FROM work w
             JOIN scene s ON s.work_id = w.id GROUP BY w.id ORDER BY count(s.id) DESC",
        )
        .unwrap();
    let boards: Vec<(String, String, i64)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    for (work_id, title, count) in boards {
        let scenes = kilna_lib::scene::for_work(&conn, &work_id).unwrap();
        let mut framed = 0;
        let mut filmed = 0;
        let mut timed = 0;
        for scene in &scenes {
            let material = kilna_lib::scene_frame::for_scene(&conn, &scene.id).unwrap();
            if material
                .iter()
                .any(|one| one.kind == kilna_lib::scene_frame::FRAME && one.is_selected)
            {
                framed += 1;
            }
            if material
                .iter()
                .any(|one| one.kind == kilna_lib::scene_frame::VIDEO && one.is_selected)
            {
                filmed += 1;
            }
            if scene.starts_at.is_some() && scene.ends_at.is_some() {
                timed += 1;
            }
        }
        assert!(
            framed <= count && filmed <= count && timed <= count,
            "a count may never exceed the board it counts"
        );
        println!("“{title}”: {count} scenes · {framed} framed · {filmed} filmed · {timed} timed");
    }
}
