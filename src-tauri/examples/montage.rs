//! Clips, the montage list and a clone, on a real board.
//!
//!   cargo run --example montage -- <path-to-kilna.db>
//!
//! Exists for the reason `frames` does: the tests run on a database made
//! three rows ago, and a board on a real catalogue has scenes written months
//! apart, a schema that arrived by migration, and fifty rows where a test has
//! one. Everything it writes, it takes back.

use std::path::PathBuf;

use kilna_lib::{clone, db, scene, scene_frame, work};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path: PathBuf = std::env::args()
        .nth(1)
        .ok_or("usage: montage <path-to-kilna.db>")?
        .into();

    let mut conn = db::open(&path)?;
    let workspace = kilna_lib::profile::workspace(&conn)?;
    println!("schema version  {}", workspace.schema_version);
    println!("works           {}", workspace.works);

    let media = path
        .parent()
        .ok_or("the database has no directory")?
        .join("media");
    std::fs::create_dir_all(&media)?;

    let profile_id = kilna_lib::profile::active(&conn)?
        .ok_or("no active profile")?
        .id;

    // The biggest real board: the conditions that matter are many scenes, not
    // one made for the occasion.
    let mut boards: Vec<_> = work::list(&conn, &profile_id, &work::WorkFilter::default())?
        .into_iter()
        .filter_map(|one| {
            let scenes = scene::for_work(&conn, &one.id).ok()?;
            if scenes.is_empty() {
                None
            } else {
                Some((one, scenes))
            }
        })
        .collect();
    // A board that has a donor if the catalogue holds one, because carrying
    // the donor across is half of what a clone is for; the biggest board
    // otherwise.
    boards.sort_by_key(|(one, scenes)| {
        let has_donor = kilna_lib::link::sources(&conn, &one.id)
            .map(|links| !links.is_empty())
            .unwrap_or(false);
        (
            std::cmp::Reverse(has_donor),
            std::cmp::Reverse(scenes.len()),
        )
    });
    let (board, scenes) = boards.into_iter().next().ok_or("no board with scenes")?;
    println!(
        "donor on it     {}",
        !kilna_lib::link::sources(&conn, &board.id)?.is_empty()
    );
    println!(
        "board           {:?} — {} scenes",
        board.title,
        scenes.len()
    );

    let target = &scenes[0];
    let before = scene_frame::for_scene(&conn, &target.id)?.len();

    // A clip on a real scene, beside whatever stills it already has.
    let holding = tempfile::tempdir()?;
    let file = holding.path().join("take-1.mp4");
    std::fs::write(&file, b"pretend mp4")?;
    let clip = scene_frame::attach(&conn, &media, &target.id, scene_frame::VIDEO, &file)?;
    scene_frame::select(&conn, &clip.id)?;

    let stills = scene_frame::of_kind(&conn, &target.id, scene_frame::FRAME)?;
    let clips = scene_frame::of_kind(&conn, &target.id, scene_frame::VIDEO)?;
    println!(
        "scene 1         {} stills, {} clips",
        stills.len(),
        clips.len()
    );
    println!(
        "chosen still    {:?}",
        scene_frame::selected(&conn, &target.id, scene_frame::FRAME)?
            .and_then(|one| one.original_name)
    );
    println!(
        "chosen clip     {:?}  (the still above is untouched)",
        scene_frame::selected(&conn, &target.id, scene_frame::VIDEO)?
            .and_then(|one| one.original_name)
    );

    // The clone: every scene, every candidate, no new bytes.
    let files_before = std::fs::read_dir(&media)?.count();
    let cloned = clone::clone_work(&conn, &board.id, "A second attempt (example)")?;
    let files_after = std::fs::read_dir(&media)?.count();
    println!(
        "cloned          {} scenes, {} pictures and clips",
        cloned.scenes, cloned.materials
    );
    println!("files on disk   {files_before} before, {files_after} after — the copy shares them");

    // And the copy stands on its own donor.
    println!(
        "donor carried   {}",
        !kilna_lib::link::sources(&conn, &cloned.work.id)?.is_empty()
    );

    // Put everything back.
    let entry =
        kilna_lib::trash::discard(&mut conn, kilna_lib::trash::Entity::Work, &cloned.work.id)?;
    kilna_lib::trash::purge(&mut conn, &entry, None)?;
    scene_frame::detach(&conn, &clip.id)?;

    let after = scene_frame::for_scene(&conn, &target.id)?.len();
    println!("scene 1 again   {after} (was {before})");
    println!(
        "files again     {} — nothing left behind",
        std::fs::read_dir(&media)?.count()
    );

    Ok(())
}
