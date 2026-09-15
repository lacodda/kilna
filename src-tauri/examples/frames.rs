//! Hangs pictures on a real scene and reads them back.
//!
//!   cargo run --example frames -- <path-to-kilna.db>
//!
//! Exists because the tests run on a database made three rows ago. A
//! storyboard on a real catalogue has scenes written months apart, a schema
//! that arrived by migration rather than by creation, and an asset table that
//! was empty until last week — and those are the conditions a frame has to
//! work in. Everything it writes, it takes back.

use std::path::PathBuf;

use kilna_lib::{db, scene, scene_frame, work};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path: PathBuf = std::env::args()
        .nth(1)
        .ok_or("usage: frames <path-to-kilna.db>")?
        .into();

    let conn = db::open(&path)?;
    let workspace = kilna_lib::profile::workspace(&conn)?;
    println!("schema version  {}", workspace.schema_version);
    println!("works           {}", workspace.works);

    // The media directory the application would use: beside the database.
    let media = path
        .parent()
        .ok_or("the database has no directory")?
        .join("media");
    std::fs::create_dir_all(&media)?;

    // A scene of a real board — the first one the catalogue has.
    let profile_id = kilna_lib::profile::active(&conn)?
        .ok_or("no active profile")?
        .id;
    let boards: Vec<_> = work::list(&conn, &profile_id, &work::WorkFilter::default())?
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
    let (chosen, scenes) = boards
        .first()
        .ok_or("no work in this workspace has scenes")?;
    let target = &scenes[0];
    println!(
        "board           {} ({} scenes), scene #{}",
        chosen.title,
        scenes.len(),
        target.position
    );

    let before = scene_frame::for_scene(&conn, &target.id)?.len();

    // Four candidates, the way a generator answers one prompt.
    let holding = tempfile::tempdir()?;
    let mut made = Vec::new();
    for n in 1..=4 {
        let file = holding.path().join(format!("still-v{n}.png"));
        std::fs::write(&file, format!("pretend png {n}").as_bytes())?;
        made.push(scene_frame::attach(&conn, &media, &target.id, &file)?);
    }
    let frames = scene_frame::for_scene(&conn, &target.id)?;
    println!("frames hung     {} (was {before})", frames.len() - before);
    println!(
        "positions       {:?}",
        frames.iter().map(|one| one.position).collect::<Vec<_>>()
    );
    println!(
        "copied to media {}",
        made.iter()
            .filter(|one| std::path::Path::new(&one.path).is_file())
            .count()
    );

    // A pasted picture: bytes, no file of its own.
    let pasted = scene_frame::attach_bytes(
        &conn,
        &media,
        &target.id,
        b"pretend pasted png",
        "clipboard.png",
    )?;
    println!(
        "pasted          {} as {:?}",
        std::path::Path::new(&pasted.path).is_file(),
        pasted.original_name
    );

    // The verdict, and that a second one moves the mark rather than adding it.
    scene_frame::select(&conn, &made[0].id)?;
    scene_frame::select(&conn, &made[2].id)?;
    let chosen_now: Vec<_> = scene_frame::for_scene(&conn, &target.id)?
        .into_iter()
        .filter(|one| one.is_selected)
        .collect();
    println!(
        "chosen          {} ({})",
        chosen_now.len(),
        chosen_now
            .first()
            .and_then(|one| one.original_name.clone())
            .unwrap_or_default()
    );

    // Reading the whole board at once, the way the storyboard draws it.
    let all = scene_frame::for_work(&conn, &chosen.id)?;
    println!("board frames    {}", all.len());

    // And the frame names its scene, from the other end.
    let back = scene_frame::scene_of(&conn, &made[0].asset_id)?;
    println!(
        "frame knows it  {}",
        back.as_deref() == Some(target.id.as_str())
    );

    // Put the workspace back exactly as it was found.
    for frame in made.iter().chain(std::iter::once(&pasted)) {
        scene_frame::detach(&conn, &frame.id)?;
    }
    let left = scene_frame::for_scene(&conn, &target.id)?.len();
    let stray = made
        .iter()
        .chain(std::iter::once(&pasted))
        .filter(|one| std::path::Path::new(&one.path).exists())
        .count();
    println!("cleaned up      {} left, {stray} stray files", left);
    assert_eq!(left, before, "the scene is as it was found");
    assert_eq!(stray, 0, "no bytes left behind");

    Ok(())
}
