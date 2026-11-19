use super::*;
use crate::fixtures;

#[test]
fn a_header_is_read_in_english_and_in_russian() {
    assert_eq!(column_of("Content"), Some("id"));
    assert_eq!(column_of("Название видео"), Some("title"));
    assert_eq!(column_of("Impressions click-through rate (%)"), Some("ctr"));
    assert_eq!(column_of("Impressions"), Some("impressions"));
    assert_eq!(
        column_of("Средний процент просмотра (%)"),
        Some("avg_view_pct")
    );
    assert_eq!(
        column_of("Duration"),
        None,
        "a column it does not know is left out"
    );
}

#[test]
fn a_cell_is_read_as_a_number_however_it_is_written() {
    assert_eq!(number_of("1,234", "views"), Some(1234.0));
    assert_eq!(number_of("5,4", "ctr"), Some(5.4));
    assert_eq!(number_of("12.5%", "avg_view_pct"), Some(12.5));
    assert_eq!(number_of("0:03:05", "avg_view_seconds"), Some(185.0));
    assert_eq!(number_of("", "views"), None);
    assert_eq!(number_of("n/a", "views"), None);
}

#[test]
fn a_videos_id_is_read_from_any_link_to_it() {
    assert_eq!(
        video_id("https://youtu.be/dQw4w9WgXcQ").as_deref(),
        Some("dQw4w9WgXcQ")
    );
    assert_eq!(
        video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=3").as_deref(),
        Some("dQw4w9WgXcQ")
    );
    assert_eq!(video_id("https://youtube.com/@channel"), None);
}

#[test]
fn rows_are_matched_by_the_link_then_the_title_and_the_total_is_left_out() {
    let (conn, profile_id) = fixtures::workspace();
    let linked = fixtures::work(&conn, &profile_id, "video", "Dawn");
    let named = fixtures::work(&conn, &profile_id, "video", "Dusk");
    let release = |work_id: &str, url: Option<&str>| {
        let made = crate::release::create(
            &conn,
            crate::release::NewRelease {
                work_id: work_id.to_owned(),
                kind: "youtube".into(),
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap();
        conn.execute(
            "UPDATE release SET url = ?1 WHERE id = ?2",
            rusqlite::params![url, made.id],
        )
        .unwrap();
        made
    };
    let by_link = release(&linked.id, Some("https://youtu.be/dQw4w9WgXcQ"));
    let by_title = release(&named.id, None);

    let csv = "Content,Video title,Views,Impressions click-through rate (%),Average view duration\n\
               Total,,3000,5.1,0:02:00\n\
               dQw4w9WgXcQ,Something else,1200,6.2,0:03:05\n\
               xxxxxxxxxxx,Dusk,800,4.0,0:01:30\n\
               yyyyyyyyyyy,Nobody's,1000,3.0,0:01:00\n";
    let read = read(&conn, &profile_id, csv).unwrap();

    assert_eq!(read.columns, vec!["views", "ctr", "avg_view_seconds"]);
    assert_eq!(read.rows.len(), 3, "the total is no video");
    assert_eq!(
        read.rows[0].release_id.as_deref(),
        Some(by_link.id.as_str())
    );
    assert_eq!(read.rows[0].by.as_deref(), Some("id"));
    assert_eq!(read.rows[0].metrics.get("avg_view_seconds"), Some(&185.0));
    assert_eq!(
        read.rows[1].release_id.as_deref(),
        Some(by_title.id.as_str())
    );
    assert_eq!(read.rows[1].by.as_deref(), Some("title"));
    assert_eq!(read.rows[2].release_id, None);
}

#[test]
fn a_file_with_no_video_column_is_refused() {
    let (conn, profile_id) = fixtures::workspace();
    let refused = read(&conn, &profile_id, "Views,Likes\n10,2\n").unwrap_err();
    assert_eq!(
        refused.refusal().map(|one| one.code),
        Some("analytics.noVideoColumn")
    );
}

/// Numbers kept are the release's, all in one undo, and the catalogue reads
/// its views (v0.160).
#[test]
fn kept_numbers_land_on_the_release_and_in_the_catalogue() {
    let (conn, profile_id) = fixtures::workspace();
    let video = fixtures::work(&conn, &profile_id, "video", "Dawn");
    let made = crate::release::create(
        &conn,
        crate::release::NewRelease {
            work_id: video.id.clone(),
            kind: "youtube".into(),
            scheduled_at: None,
            meta: None,
            scheduled_time: None,
            time_zone: None,
        },
    )
    .unwrap();
    let read = read(
        &conn,
        &profile_id,
        "Content,Video title,Views,Likes
xxxxxxxxxxx,Dawn,1200,40
",
    )
    .unwrap();

    assert_eq!(crate::actions::analytics::keep(&conn, &read).unwrap(), 1);
    let kept = crate::release::get(&conn, &made.id).unwrap().unwrap();
    let numbers = kept.meta.get("analytics").unwrap();
    assert_eq!(numbers.get("views"), Some(&json!(1200.0)));
    assert_eq!(numbers.get("from"), Some(&json!("youtube_studio")));
    let row = crate::score::catalogue(&conn, &profile_id)
        .unwrap()
        .into_iter()
        .find(|row| row.work_id == video.id)
        .unwrap();
    assert_eq!(row.views, Some(1200.0));
}
