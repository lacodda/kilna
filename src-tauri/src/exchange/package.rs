//! A work packed into a folder: what someone needs in front of them to make
//! the thing, outside kilna.
//!
//! A video is the case this was built for. The prompts live on the board, the
//! pictures live in `media/` under names that are ids, the metadata lives on
//! the release, and the order lives in the scene numbers. Making the video
//! means having all four at once, in a program that has never heard of kilna.
//! Until now that meant copying each prompt by hand and hunting for the
//! pictures by their id.
//!
//! What goes in the folder is text and files, not a project file for one
//! editing program: the owner's decision of 2026-09-16, recorded against the
//! montage list and holding for the same reasons. Text is what every program
//! reads, what a person can check with their eyes before trusting it, and
//! what still says something in five years.
//!
//! The pictures are copied under names that say what they are — the scene
//! number, whether it is the chosen one — rather than under the ids they are
//! stored as. A folder of `9f3a1c…png` is a folder someone has to open one by
//! one.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::profile::config::WorkKind;
use crate::release;
use crate::release_meta;
use crate::scene::{self, Scene};
use crate::scene_frame::{self, SceneFrame, VIDEO};
use crate::work;

/// What a package export produced, so the person can be told rather than
/// guess where it went.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageReport {
    /// The folder that was written, absolute.
    pub directory: String,
    pub scenes: usize,
    /// Pictures and clips copied beside the text.
    pub files: usize,
    /// Releases whose metadata went into the folder.
    pub releases: usize,
    /// Scenes with nothing chosen: named here because a package is also how
    /// someone finds out what is still missing, and a gap that reports
    /// nothing reads as a gap that is not there.
    pub without_material: usize,
}

/// Write a work's whole package into `directory`.
///
/// The folder is made inside it, named after the work, so choosing the same
/// parent twice does not put two works in one heap.
pub fn write(conn: &Connection, work_id: &str, directory: &Path) -> Result<PackageReport> {
    let work = work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let kind = config.vocabulary(&work.kind);

    let folder = directory.join(slug(&work.title, &work.id));
    std::fs::create_dir_all(&folder)?;

    let scenes = scene::for_work(conn, work_id)?;
    let frames = scene_frame::for_work(conn, work_id)?;

    // The board, prompts and all. One file rather than one per scene: the
    // board is read top to bottom while the video is cut, and fifty files
    // are fifty things to open in order.
    std::fs::write(
        folder.join("board.md"),
        board(&work.title, &scenes, kind, &frames),
    )?;

    // What it goes out as, one section per release. Left out entirely when
    // the work has no release yet, rather than written as an empty heading:
    // the folder should not carry a question it cannot answer.
    let releases = release::for_work(conn, &work.profile_id, work_id)?;
    let mut described = 0usize;
    let mut page = String::new();
    for entry in &releases {
        let fields = release_meta::fields(conn, &entry.release.id)?;
        if fields.iter().all(|field| field.value.trim().is_empty()) {
            continue;
        }
        page.push_str(&metadata(&entry.release, &fields, &config));
        described += 1;
    }
    if described > 0 {
        std::fs::write(folder.join("release.md"), page)?;
    }

    // The pictures, renamed to say what they are.
    let media = folder.join("material");
    let mut files = 0usize;
    let mut without = 0usize;
    for scene in &scenes {
        let mine: Vec<&SceneFrame> = frames
            .iter()
            .filter(|frame| frame.scene_id == scene.id)
            .collect();
        if mine.is_empty() {
            without += 1;
            continue;
        }
        for frame in mine {
            if files == 0 {
                std::fs::create_dir_all(&media)?;
            }
            let source = Path::new(&frame.path);
            let target = media.join(material_name(scene, frame, source));
            // A picture whose file has gone missing does not take the package
            // with it: the rest is still worth having, and `board.md` already
            // says which scene it belonged to.
            match std::fs::copy(source, &target) {
                Ok(_) => files += 1,
                Err(cause) => eprintln!("package: {} could not be copied: {cause}", frame.path),
            }
        }
    }

    Ok(PackageReport {
        directory: folder.display().to_string(),
        scenes: scenes.len(),
        files,
        releases: described,
        without_material: without,
    })
}

/// The board as one readable page: a table to find a scene by, then every
/// scene in full with its prompts.
fn board(title: &str, scenes: &[Scene], kind: &WorkKind, frames: &[SceneFrame]) -> String {
    let mut page = format!("# {title}\n");

    if scenes.is_empty() {
        page.push_str("\nThe board is empty.\n");
        return page;
    }

    page.push_str("\n## The board\n\n");
    page.push_str(&crate::assistant::prompt::board_table(scenes, kind));
    page.push_str("\n\n## Scenes\n");

    for scene in scenes {
        page.push_str(&format!(
            "\n{}\n",
            crate::assistant::prompt::scene_sheet(scene, kind)
        ));

        // What was chosen for this scene, by the name it is copied under, so
        // the page and the folder beside it agree.
        let mine: Vec<&SceneFrame> = frames
            .iter()
            .filter(|frame| frame.scene_id == scene.id)
            .collect();
        for frame in mine.iter().filter(|frame| frame.is_selected) {
            let what = if frame.kind == VIDEO { "Clip" } else { "Still" };
            page.push_str(&format!(
                "\n{what}: `material/{}`\n",
                material_name(scene, frame, Path::new(&frame.path))
            ));
        }
        if mine.is_empty() {
            page.push_str("\nNothing chosen yet.\n");
        }
    }

    page
}

/// One release's metadata as a section someone can copy out of.
fn metadata(
    release: &release::Release,
    fields: &[release_meta::Field],
    config: &crate::profile::config::ProfileConfig,
) -> String {
    let label = config
        .all_release_kinds()
        .iter()
        .find(|kind| kind.key == release.kind)
        .map_or(release.kind.clone(), |kind| kind.label.as_str().to_owned());

    let when = release
        .released_at
        .as_deref()
        .or(release.scheduled_at.as_deref())
        .unwrap_or("not scheduled");

    let mut page = format!("# {label} — {when}\n");

    for field in fields {
        if field.value.trim().is_empty() {
            continue;
        }
        page.push_str(&format!("\n## {}\n\n{}\n", field.label, field.value.trim()));
    }

    page
}

/// What a picture is called in the package.
///
/// The scene number leads, so the folder sorts into the order the video is
/// cut in; the chosen one says so, because that is the question asked of a
/// folder holding four near-identical pictures. The original name is kept
/// after it when there is one — that is what the person recognises.
fn material_name(scene: &Scene, frame: &SceneFrame, source: &Path) -> String {
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("bin");

    let what = if frame.kind == VIDEO { "clip" } else { "still" };
    let chosen = if frame.is_selected { "-chosen" } else { "" };

    // The name it arrived under, cleaned of anything a filesystem rejects and
    // of its own extension, so the result does not read `…png.png`.
    let original = frame
        .original_name
        .as_deref()
        .map(|name| {
            let stem = Path::new(name)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or(name);
            format!("-{}", clean(stem))
        })
        .unwrap_or_default();

    format!(
        "{:02}-{what}{}{chosen}{}.{extension}",
        scene.position,
        if frame.kind == VIDEO || frame.position > 1 {
            format!("-{}", frame.position)
        } else {
            String::new()
        },
        original
    )
}

/// A folder name for a work: its title, minus what a filesystem rejects.
///
/// The same shape the markdown export uses, and for the same reasons —
/// non-ASCII is kept because a Cyrillic title should stay readable, and the
/// id suffix keeps two works with one title out of each other's folder.
fn slug(title: &str, id: &str) -> String {
    let short = clean(title).chars().take(60).collect::<String>();
    let head = id.split('-').next().unwrap_or(id);

    if short.trim().is_empty() {
        format!("untitled-{head}")
    } else {
        format!("{}-{head}", short.trim())
    }
}

/// Text a filesystem will take as part of a name.
fn clean(text: &str) -> String {
    let replaced: String = text
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\n' | '\r' | '\t' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect();
    // Trailing dashes and dots go too: `a-b-c-d-` and `name..` are what a
    // title ending in a character the filesystem rejects would otherwise
    // leave behind, and neither is a name anyone would type.
    replaced
        .trim()
        .trim_matches(|c| c == '.' || c == '-')
        .to_owned()
}

/// Where to suggest putting a package.
pub fn default_directory(documents: &Path) -> PathBuf {
    documents.join("kilna-packages")
}

/// Whether a work is worth packing: it has a board, or something written
/// about a release. A song with neither would produce a folder holding one
/// nearly empty page.
pub fn has_anything(conn: &Connection, work_id: &str) -> Result<bool> {
    if scene::count(conn, work_id)? > 0 {
        return Ok(true);
    }
    let work = work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    for entry in release::for_work(conn, &work.profile_id, work_id)? {
        let fields = release_meta::fields(conn, &entry.release.id)?;
        if fields.iter().any(|field| !field.value.trim().is_empty()) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::profile;
    use crate::release::NewRelease;
    use crate::scene::NewScene;
    use crate::work::NewWork;
    use serde_json::json;

    fn workspace() -> (Connection, String, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn).unwrap();
        let profile_id = profile::active(&conn).unwrap().unwrap().id;
        (conn, profile_id, dir)
    }

    /// A video with two scenes, the second carrying a still prompt.
    fn video(conn: &mut Connection, profile_id: &str) -> String {
        let work = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "video".into(),
                title: "The long way round".into(),
                ..NewWork::default()
            },
        )
        .unwrap();

        scene::create(
            conn,
            profile_id,
            NewScene {
                work_id: work.id.clone(),
                description: Some("a harbour at first light".into()),
                section: Some("open".into()),
                starts_at: Some(0.0),
                ends_at: Some(4.0),
                ..NewScene::default()
            },
        )
        .unwrap();

        scene::create(
            conn,
            profile_id,
            NewScene {
                work_id: work.id.clone(),
                description: Some("the boat leaves".into()),
                blocks: json!({ "still": "wide, cold light, no people" })
                    .as_object()
                    .cloned(),
                ..NewScene::default()
            },
        )
        .unwrap();

        work.id
    }

    #[test]
    fn a_package_carries_the_board_with_its_prompts() {
        let (mut conn, profile_id, dir) = workspace();
        let work_id = video(&mut conn, &profile_id);

        let report = write(&conn, &work_id, dir.path()).unwrap();

        assert_eq!(report.scenes, 2);
        let board = std::fs::read_to_string(Path::new(&report.directory).join("board.md")).unwrap();
        assert!(board.contains("The long way round"), "{board}");
        assert!(
            board.contains("a harbour at first light"),
            "the table names every scene: {board}"
        );
        assert!(
            board.contains("wide, cold light, no people"),
            "a prompt cut to a table cell is no use; the sheets carry it whole: {board}"
        );
    }

    #[test]
    fn the_folder_is_named_after_the_work_rather_than_being_the_one_chosen() {
        let (mut conn, profile_id, dir) = workspace();
        let work_id = video(&mut conn, &profile_id);

        let report = write(&conn, &work_id, dir.path()).unwrap();

        let folder = Path::new(&report.directory);
        assert_eq!(folder.parent(), Some(dir.path()));
        assert!(
            folder
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("The long way round-")),
            "choosing one parent twice must not put two works in one heap: {report:?}"
        );
    }

    #[test]
    fn a_work_with_no_metadata_written_gets_no_release_page() {
        let (mut conn, profile_id, dir) = workspace();
        let work_id = video(&mut conn, &profile_id);
        release::create(
            &conn,
            NewRelease {
                work_id: work_id.clone(),
                kind: "youtube".into(),
                title: None,
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap();

        let report = write(&conn, &work_id, dir.path()).unwrap();

        assert_eq!(report.releases, 0);
        assert!(
            !Path::new(&report.directory).join("release.md").exists(),
            "the folder must not carry a heading it has no answer for"
        );
    }

    #[test]
    fn what_a_release_goes_out_as_is_written_beside_the_board() {
        let (mut conn, profile_id, dir) = workspace();
        let work_id = video(&mut conn, &profile_id);
        let release = release::create(
            &conn,
            NewRelease {
                work_id: work_id.clone(),
                kind: "youtube".into(),
                title: None,
                scheduled_at: Some("2026-10-02".into()),
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap();
        release::update(
            &conn,
            &release.id,
            crate::release::ReleasePatch {
                meta: Some(
                    json!({ "title": "The long way round", "tags": "sea, night" })
                        .as_object()
                        .cloned()
                        .unwrap(),
                ),
                ..Default::default()
            },
        )
        .unwrap();

        let report = write(&conn, &work_id, dir.path()).unwrap();

        assert_eq!(report.releases, 1);
        let page =
            std::fs::read_to_string(Path::new(&report.directory).join("release.md")).unwrap();
        assert!(
            page.contains("YouTube"),
            "the kind's own word, not its key: {page}"
        );
        assert!(page.contains("2026-10-02"), "{page}");
        assert!(page.contains("The long way round"), "{page}");
        assert!(page.contains("sea, night"), "{page}");
        assert!(
            !page.contains("Pinned comment"),
            "a field nobody wrote is left out rather than shown empty: {page}"
        );
    }

    #[test]
    fn a_board_with_nothing_chosen_says_how_many_scenes_are_waiting() {
        let (mut conn, profile_id, dir) = workspace();
        let work_id = video(&mut conn, &profile_id);

        let report = write(&conn, &work_id, dir.path()).unwrap();

        assert_eq!(
            report.without_material, 2,
            "a package is also how someone finds out what is still missing"
        );
        assert_eq!(report.files, 0);
        assert!(!Path::new(&report.directory).join("material").exists());
    }

    #[test]
    fn a_picture_is_copied_under_a_name_that_says_what_it_is() {
        let scene = Scene {
            id: "s".into(),
            profile_id: "p".into(),
            work_id: "w".into(),
            position: 4,
            section: None,
            starts_at: None,
            ends_at: None,
            shot_type: None,
            description: String::new(),
            blocks: serde_json::Map::new(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        let frame = SceneFrame {
            id: "f".into(),
            scene_id: "s".into(),
            asset_id: "a".into(),
            kind: "frame".into(),
            position: 1,
            is_selected: true,
            path: "C:/media/9f3a1c.png".into(),
            original_name: Some("harbour-v3.png".into()),
            created_at: String::new(),
        };

        assert_eq!(
            material_name(&scene, &frame, Path::new(&frame.path)),
            "04-still-chosen-harbour-v3.png",
            "a folder of ids is a folder someone opens one by one"
        );
    }

    #[test]
    fn an_unchosen_second_still_keeps_its_number_and_says_it_is_not_chosen() {
        let scene = Scene {
            id: "s".into(),
            profile_id: "p".into(),
            work_id: "w".into(),
            position: 4,
            section: None,
            starts_at: None,
            ends_at: None,
            shot_type: None,
            description: String::new(),
            blocks: serde_json::Map::new(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        let frame = SceneFrame {
            id: "f".into(),
            scene_id: "s".into(),
            asset_id: "a".into(),
            kind: "frame".into(),
            position: 2,
            is_selected: false,
            path: "C:/media/9f3a1c.png".into(),
            original_name: None,
            created_at: String::new(),
        };

        assert_eq!(
            material_name(&scene, &frame, Path::new(&frame.path)),
            "04-still-2.png"
        );
    }

    #[test]
    fn a_name_the_filesystem_would_reject_is_cleaned() {
        assert_eq!(clean("a/b:c*d?"), "a-b-c-d");
        assert_eq!(clean("  spaced  "), "spaced");
        assert_eq!(clean("..hidden.."), "hidden");
    }

    #[test]
    fn a_title_that_cleans_to_nothing_still_names_a_folder() {
        let named = slug("///", "9f3a1c2b-0000");

        assert_eq!(named, "untitled-9f3a1c2b");
    }

    #[test]
    fn two_works_with_one_title_do_not_share_a_folder() {
        assert_ne!(
            slug("Harbour lights", "aaaa-1111"),
            slug("Harbour lights", "bbbb-2222")
        );
    }

    #[test]
    fn a_work_worth_packing_is_one_with_a_board_or_something_written() {
        let (mut conn, profile_id, _dir) = workspace();

        let bare = work::create(
            &conn,
            &profile_id,
            NewWork {
                kind: "song".into(),
                title: "Nothing yet".into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        assert!(
            !has_anything(&conn, &bare.id).unwrap(),
            "a folder holding one nearly empty page is not worth offering"
        );

        let with_board = video(&mut conn, &profile_id);
        assert!(has_anything(&conn, &with_board).unwrap());
    }
}
