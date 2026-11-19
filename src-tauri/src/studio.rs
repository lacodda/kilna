//! The YouTube Studio pack (v0.159, FR-22 22.1 p.1): putting out a batch of
//! ready releases by hand, with no API - the platform's quota is gone and a
//! browser driven by a robot is how a channel gets banned. Each release of
//! the batch comes as one step: its video and cover to drag into the
//! browser, every field a press from the clipboard, the day and the time in
//! the channel's zone - said when summer time makes the offset another than
//! today's - and a place for the link the platform gave back.
//!
//! This module reads; the steps write through the release's own actions:
//! the link kept, or the release marked out on its day.

use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;

use crate::error::Result;
use crate::release::{self, Release};

const VIDEOS: [&str; 5] = ["mp4", "mov", "webm", "mkv", "avi"];

/// One release of a pack, as its step shows it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct StudioItem {
    pub release: Release,
    pub work_title: String,
    /// The day it goes out, `YYYY-MM-DD`.
    pub date: Option<String>,
    /// The time it goes out, `HH:MM`: the release's own, else the rhythm's.
    pub time: Option<String>,
    /// Whose day: the release's zone, else the rhythm's.
    pub time_zone: Option<String>,
    /// The zone's offset at that day and time, `-03:00`.
    pub offset: Option<String>,
    /// The zone's offset today at that time: another than `offset` when
    /// summer time lies between.
    pub offset_now: Option<String>,
    /// The video to drag into the platform, whole.
    pub video: Option<String>,
    /// The chosen cover, whole.
    pub cover: Option<String>,
    /// The work's folder, whole.
    pub folder: Option<String>,
    /// Every field of the release with what it holds.
    pub fields: Vec<crate::release_meta::Field>,
}

/// A work of the batch that has no step, and why.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct StudioSkip {
    pub work_id: String,
    pub title: String,
    /// `noRelease` (nothing goes out on the door), `released` (it is out),
    /// `noVideo` (its video is not made yet).
    pub reason: String,
}

/// A batch put out by hand.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct StudioPack {
    pub items: Vec<StudioItem>,
    pub skipped: Vec<StudioSkip>,
}

/// The offset of `zone` on `date` at `clock`, `-03:00`.
pub fn offset_at(zone: &str, date: &str, clock: &str) -> Option<String> {
    let zone = jiff::tz::TimeZone::get(zone).ok()?;
    let (year, rest) = date.split_once('-')?;
    let (month, day) = rest.split_once('-')?;
    let (hour, minute) = clock.split_once(':')?;
    let at = jiff::civil::date(year.parse().ok()?, month.parse().ok()?, day.parse().ok()?).at(
        hour.parse().ok()?,
        minute.parse().ok()?,
        0,
        0,
    );
    let seconds = at.to_zoned_datetime(zone).ok()?.offset().seconds();
    let sign = if seconds < 0 { '-' } else { '+' };
    let minutes = seconds.unsigned_abs() / 60;
    Some(format!("{sign}{:02}:{:02}", minutes / 60, minutes % 60))
}

fn is_video(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| VIDEOS.contains(&ext.to_lowercase().as_str()))
}

/// The pack of these works' releases on `door`: one step for each that is
/// still to go out and has its video, in the order they go out.
pub fn pack(conn: &Connection, work_ids: &[String], door: &str, today: &str) -> Result<StudioPack> {
    let mut items = Vec::new();
    let mut skipped = Vec::new();
    for work_id in work_ids {
        let Some(work) = crate::work::get(conn, work_id)? else {
            continue;
        };
        let skip = |reason: &str| StudioSkip {
            work_id: work.id.clone(),
            title: work.title.clone(),
            reason: reason.to_owned(),
        };
        let Some(found) = release::of_work(conn, &work.id)?.filter(|one| one.kind == door) else {
            skipped.push(skip("noRelease"));
            continue;
        };
        if found.status == release::RELEASED {
            skipped.push(skip("released"));
            continue;
        }
        let config = crate::profile::config_for(conn, &work.profile_id)?;
        let place = crate::folder::locate(conn, &config, &work)?;
        let attached = crate::asset::for_work(conn, &work.id)?
            .into_iter()
            .map(|asset| asset.path)
            .find(|path| is_video(path));
        let Some(video) = place
            .file
            .path
            .clone()
            .filter(|path| is_video(path))
            .or(attached)
        else {
            skipped.push(skip("noVideo"));
            continue;
        };
        let rhythm = config.rhythm.as_ref();
        let time = found
            .scheduled_time
            .clone()
            .or_else(|| rhythm.and_then(|one| one.default_time.clone()));
        let time_zone = found
            .time_zone
            .clone()
            .or_else(|| rhythm.and_then(|one| one.timezone.clone()));
        let date = found
            .scheduled_at
            .as_deref()
            .and_then(|at| at.get(..10))
            .map(str::to_owned);
        let at = |day: &str| {
            time_zone
                .as_deref()
                .zip(time.as_deref())
                .and_then(|(zone, clock)| offset_at(zone, day, clock))
        };
        items.push(StudioItem {
            offset: date.as_deref().and_then(at),
            offset_now: at(today),
            date,
            time,
            time_zone,
            video: Some(video),
            cover: crate::asset::cover_of(conn, &work.id)?.map(|asset| asset.path),
            folder: place.path.clone(),
            fields: crate::release_meta::fields(conn, &found.id)?,
            work_title: work.title.clone(),
            release: found,
        });
    }
    items.sort_by(|a, b| {
        a.date
            .cmp(&b.date)
            .then_with(|| a.time.cmp(&b.time))
            .then_with(|| a.work_title.cmp(&b.work_title))
    });
    Ok(StudioPack { items, skipped })
}

/// Whether a link is one the platform gave back for a video: a watch page,
/// a short, or the short link.
pub fn is_video_link(link: &str) -> bool {
    let link = link.trim();
    let rest = link
        .strip_prefix("https://")
        .or_else(|| link.strip_prefix("http://"))
        .unwrap_or(link);
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    let rest = rest.strip_prefix("m.").unwrap_or(rest);
    let id = if let Some(after) = rest.strip_prefix("youtu.be/") {
        after
    } else if let Some(after) = rest.strip_prefix("youtube.com/watch?v=") {
        after
    } else if let Some(after) = rest.strip_prefix("youtube.com/shorts/") {
        after
    } else {
        return false;
    };
    let id: String = id
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    id.len() == 11
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zones_offset_follows_summer_time() {
        // New York: -05:00 in January, -04:00 in July.
        assert_eq!(
            offset_at("America/New_York", "2027-01-15", "18:00").as_deref(),
            Some("-05:00")
        );
        assert_eq!(
            offset_at("America/New_York", "2027-07-15", "18:00").as_deref(),
            Some("-04:00")
        );
        assert_eq!(offset_at("Nowhere/At_All", "2027-07-15", "18:00"), None);
    }

    #[test]
    fn only_a_link_to_a_video_is_taken() {
        assert!(is_video_link("https://youtu.be/dQw4w9WgXcQ"));
        assert!(is_video_link(
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=3"
        ));
        assert!(is_video_link("youtube.com/shorts/dQw4w9WgXcQ"));
        assert!(!is_video_link("https://youtube.com/@channel"));
        assert!(!is_video_link("https://youtu.be/short"));
        assert!(!is_video_link("https://example.com/watch?v=dQw4w9WgXcQ"));
    }

    #[test]
    fn a_work_with_nothing_to_put_out_is_said_and_skipped() {
        let (conn, profile_id) = crate::fixtures::workspace();
        let song = crate::fixtures::song(&conn, &profile_id, "Dawn");
        let pack = pack(
            &conn,
            std::slice::from_ref(&song.id),
            "youtube",
            "2026-11-19",
        )
        .unwrap();
        assert!(pack.items.is_empty());
        assert_eq!(pack.skipped.len(), 1);
        assert_eq!(pack.skipped[0].reason, "noRelease");
    }
}
