use super::*;
use crate::fixtures;

/// A workspace with its media folder on disk, the folder works are kept in,
/// and a folder standing for Downloads.
struct Place {
    conn: Connection,
    profile_id: String,
    media: tempfile::TempDir,
    _root: tempfile::TempDir,
    downloads: tempfile::TempDir,
}

fn place() -> Place {
    let (conn, profile_id, media) = fixtures::workspace_with_media();
    let root = tempfile::tempdir().unwrap();
    folder::set_root(
        &conn,
        &profile_id,
        Some(&root.path().to_string_lossy()),
        "2030-01-01T00:00:00.000Z",
    )
    .unwrap();
    Place {
        conn,
        profile_id,
        media,
        _root: root,
        downloads: tempfile::tempdir().unwrap(),
    }
}

impl Place {
    fn download(&self, name: &str) -> PathBuf {
        let path = self.downloads.path().join(name);
        std::fs::write(&path, name.as_bytes()).unwrap();
        path
    }

    fn take(&self, work_id: &str, source: &Path, role: MediaRole) -> Result<TakenMedia> {
        take(&self.conn, self.media.path(), work_id, source, role, true)
    }
}

#[test]
fn a_songs_track_is_moved_in_under_its_title_and_finishes_it() {
    let at = place();
    let song = fixtures::song(&at.conn, &at.profile_id, "Harbour: lights?");
    let source = at.download("suno-7f3a.mp3");

    let taken = at.take(&song.id, &source, MediaRole::FinalAudio).unwrap();

    assert_eq!(taken.relative, "Harbour lights.mp3");
    assert!(taken.finished);
    assert!(!source.exists());
    assert!(Path::new(&taken.path).is_file());
    let work = crate::work::get(&at.conn, &song.id).unwrap().unwrap();
    assert_eq!(work.file.as_deref(), Some("Harbour lights.mp3"));
}

#[test]
fn a_videos_track_lies_beside_it_and_is_not_chosen_over_the_rule() {
    let at = place();
    let video = fixtures::work(&at.conn, &at.profile_id, "video", "Dawn");
    let source = at.download("take.wav");

    let taken = at.take(&video.id, &source, MediaRole::FinalAudio).unwrap();

    assert!(!taken.finished);
    assert!(Path::new(&taken.path).is_file());
    let work = crate::work::get(&at.conn, &video.id).unwrap().unwrap();
    assert_eq!(work.file, None);
}

#[test]
fn a_video_the_rule_takes_finishes_the_work_without_a_choice() {
    let at = place();
    let video = fixtures::work(&at.conn, &at.profile_id, "video", "Dawn");
    let source = at.download("render.mp4");

    let taken = at.take(&video.id, &source, MediaRole::Video).unwrap();

    assert_eq!(taken.relative, "Dawn.mp4");
    assert!(taken.finished);
    let work = crate::work::get(&at.conn, &video.id).unwrap().unwrap();
    assert_eq!(work.file, None, "found by the rule, nothing chosen by hand");
}

#[test]
fn a_cover_is_moved_in_and_kept_as_the_works_cover() {
    let at = place();
    let video = fixtures::work(&at.conn, &at.profile_id, "video", "Dawn");
    let source = at.download("202604112318.png");

    let taken = at.take(&video.id, &source, MediaRole::FinalCover).unwrap();

    assert_eq!(taken.relative, "cover.png");
    let covers = crate::asset::covers_of(&at.conn, &at.profile_id).unwrap();
    assert!(covers.iter().any(|(work_id, _)| *work_id == video.id));
    assert!(taken.asset_id.is_some());
}

#[test]
fn a_picture_to_choose_from_is_set_beside_the_cover() {
    let at = place();
    let video = fixtures::work(&at.conn, &at.profile_id, "video", "Dawn");
    let source = at.download("variant.webp");

    let taken = at
        .take(&video.id, &source, MediaRole::CandidateCover)
        .unwrap();

    assert_eq!(taken.relative, "candidates/cover.webp");
    let covers = crate::asset::covers_of(&at.conn, &at.profile_id).unwrap();
    assert!(covers.iter().all(|(work_id, _)| *work_id != video.id));
    let kept = crate::asset::for_work(&at.conn, &video.id).unwrap();
    assert!(kept.iter().any(|one| one.kind == crate::asset::CANDIDATE));
}

#[test]
fn a_second_take_gets_a_name_of_its_own() {
    let at = place();
    let song = fixtures::song(&at.conn, &at.profile_id, "Dawn");
    let first = at
        .take(&song.id, &at.download("a.mp3"), MediaRole::CandidateAudio)
        .unwrap();
    let second = at
        .take(&song.id, &at.download("b.mp3"), MediaRole::CandidateAudio)
        .unwrap();

    assert_eq!(first.relative, "candidates/Dawn - take.mp3");
    assert_eq!(second.relative, "candidates/Dawn - take (2).mp3");
}

#[test]
fn a_file_of_the_wrong_kind_is_refused_and_left_where_it_was() {
    let at = place();
    let song = fixtures::song(&at.conn, &at.profile_id, "Dawn");
    let source = at.download("track.mp3");

    let refused = at
        .take(&song.id, &source, MediaRole::FinalCover)
        .unwrap_err();

    assert_eq!(
        refused.refusal().map(|one| one.code),
        Some("media.wrongKind")
    );
    assert!(source.exists());
}

#[test]
fn without_a_media_folder_nothing_is_moved() {
    let (conn, profile_id, media) = fixtures::workspace_with_media();
    let song = fixtures::song(&conn, &profile_id, "Dawn");
    let downloads = tempfile::tempdir().unwrap();
    let source = downloads.path().join("track.mp3");
    std::fs::write(&source, b"ID3").unwrap();

    let refused = take(
        &conn,
        media.path(),
        &song.id,
        &source,
        MediaRole::FinalAudio,
        true,
    )
    .unwrap_err();

    assert_eq!(refused.refusal().map(|one| one.code), Some("folder.noRoot"));
    assert!(source.exists());
}
