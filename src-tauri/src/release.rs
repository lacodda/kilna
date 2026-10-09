use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::minted::Minted;
use crate::time::now;

/// What ships, where and when. The predecessor spread this across three tables;
/// here it is one row per unit of release.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Release {
    pub id: String,
    pub work_id: String,
    pub kind: String,
    pub status: String,
    /// Calendar slot. `None` means queued but unscheduled.
    pub scheduled_at: Option<String>,
    pub released_at: Option<String>,
    pub url: Option<String>,
    /// Set when a person settled this date. A pinned slot is not contested —
    /// see [`schedule`].
    pub slot_pinned_at: Option<String>,
    /// When in the day it goes out (`HH:MM`), for the platforms that ask. The
    /// slot stays a date; this sits beside it.
    pub scheduled_time: Option<String>,
    /// Whose day: an IANA zone name, so the date and time still mean one
    /// instant when read on another machine.
    pub time_zone: Option<String>,
    pub meta: Map<String, Value>,
    pub created_at: String,
    pub updated_at: String,
}

/// A release with the context the calendar needs to draw it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct ScheduledRelease {
    #[serde(flatten)]
    pub release: Release,
    pub work_title: String,
    /// The kind of the work — a song, a video — so a chip can say what the
    /// thing going out *is*, not only what kind of release it is. Two kinds
    /// of work may ship the same kind of release under the same glyph.
    pub work_kind: String,
    /// Latest total for the work, so a slot can be judged against its neighbours.
    pub total: Option<f64>,
    pub tier: Option<String>,
    /// How far this release is from shippable — see [`crate::readiness`].
    pub readiness: crate::readiness::Readiness,
    /// How finished the work itself is, as its author judges it, 0..=100.
    /// Different from `readiness`, which asks whether the release could go out:
    /// a work can have every role filled and still be three verses of
    /// placeholder, and only the author knows that.
    pub work_stage: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct NewRelease {
    pub work_id: String,
    pub kind: String,
    #[serde(default)]
    pub scheduled_at: Option<String>,
    #[serde(default)]
    pub meta: Option<Map<String, Value>>,
    #[serde(default)]
    pub scheduled_time: Option<String>,
    #[serde(default)]
    pub time_zone: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
pub struct ReleasePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub scheduled_at: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub url: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub scheduled_time: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "crate::reversal::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub time_zone: Option<Option<String>>,
}

/// What happened when a slot was claimed.
///
/// Once carried the release that lost the slot; nothing loses a slot since
/// the contest went in v0.44, and the field went with the model package.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Scheduling {
    pub release: Release,
}

/// Refuse a time of day or a zone that would not read back as one.
///
/// The zone is checked for shape only — a name with a slash, or `UTC` — not
/// against the IANA list: the app does not carry the list, and a name the
/// list gains next year must not be refused by a build from this one.
fn check_when(time: Option<&str>, zone: Option<&str>) -> Result<()> {
    if let Some(time) = time
        && !crate::time::is_clock_time(time)
    {
        return Err(Error::refused("release.badTime").param("value", time));
    }
    if let Some(zone) = zone {
        let plausible = zone == "UTC"
            || (zone.contains('/')
                && zone
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '+')));
        if !plausible {
            return Err(Error::refused("release.badTimeZone").param("value", zone));
        }
    }
    Ok(())
}

/// How claiming a slot would end. One vocabulary for the dry run and the real
/// one: v0.24 shipped a contest the screens described with a different rule,
/// and a preview that can disagree with the drop is worse than none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// Nothing planned sits on the day.
    Empty,
    /// Something is already there. Not a refusal — a day holds as many
    /// releases as are put on it — but the auto-layout leaves it alone and a
    /// person is told before adding a second.
    Taken,
    /// Something is there and its date was settled by hand. The auto-layout
    /// stays away; a person may still add beside it, having seen the lock.
    Pinned,
}

/// What a day holds, shaped for the calendar to show it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct SlotPreview {
    pub verdict: Verdict,
    /// Who is on the day, when anything is.
    pub holder_title: Option<String>,
    /// The rare words the release's song would say within the guard's window
    /// of a song out or booked near that day (ADR 0054): what the calendar
    /// warns about before the release lands.
    pub repeats: Vec<crate::register::guard::RepeatFinding>,
}

/// What a look at a day found: who is on it, and in what state.
struct Contest {
    occupant: Option<Release>,
    verdict: Verdict,
}

/// Status values a release moves through. These are fixed rather than profile
/// vocabulary: they describe the mechanism, not the craft.
pub const PLANNED: &str = "planned";
pub const RELEASED: &str = "released";

const SELECT_RELEASE: &str = "SELECT id, work_id, kind, status, scheduled_at, released_at, \
     url, slot_pinned_at, meta, created_at, updated_at, scheduled_time, time_zone FROM release";

pub fn create(conn: &Connection, new: NewRelease) -> Result<Release> {
    create_minted(conn, new, Minted::fresh())
}

/// Create a release with the id and timestamp already decided.
///
/// The seam a replay comes back through: live, `create` mints them; replaying,
/// the log supplies what the first run generated, so the release lands under
/// the id everything else already names. See ADR 0014.
pub fn create_minted(conn: &Connection, new: NewRelease, minted: Minted) -> Result<Release> {
    let title: Option<String> = conn
        .query_row(
            "SELECT title FROM work WHERE id = ?1",
            params![new.work_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(title) = title else {
        return Err(Error::not_found("work", new.work_id.clone()));
    };
    // A publication goes out once (ADR 0051): another place is another
    // publication, made from the same thing. Said in words rather than left
    // to the index, which would say it as a broken constraint.
    if of_work(conn, &new.work_id)?.is_some() {
        return Err(Error::refused("release.onePerPublication").param("title", title));
    }

    let id = minted.id().to_owned();
    check_when(new.scheduled_time.as_deref(), new.time_zone.as_deref())?;
    let timestamp = minted.at().to_owned();
    let mut meta = new.meta.unwrap_or_default();
    crate::release_meta::keep_suffixes(conn, &new.work_id, None, &new.kind, &mut meta)?;

    conn.execute(
        "INSERT INTO release (id, work_id, kind, status, scheduled_at, meta, created_at, updated_at,
                              scheduled_time, time_zone)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8, ?9)",
        params![
            id,
            new.work_id,
            new.kind,
            PLANNED,
            new.scheduled_at,
            Value::Object(meta).to_string(),
            timestamp,
            new.scheduled_time,
            new.time_zone,
        ],
    )?;

    get(conn, &id)?.ok_or_else(|| Error::Internal("the release vanished after insert".into()))
}

/// The release a work goes out as, when it has one: a work has at most one
/// (ADR 0051).
pub fn of_work(conn: &Connection, work_id: &str) -> Result<Option<Release>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_RELEASE} WHERE work_id = ?1"),
            params![work_id],
            read_row,
        )
        .optional()?;
    raw.map(RawRelease::into_release).transpose()
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Release>> {
    let raw = conn
        .query_row(
            &format!("{SELECT_RELEASE} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?;

    raw.map(RawRelease::into_release).transpose()
}

/// Put a release in a slot.
///
/// A day holds as many releases as are put on it. It used to hold one, and a
/// stronger work took the date from a weaker one — the contest of v0.22. That
/// rule answered "who gets this day when I am not the one deciding", and the
/// only place it was ever asked from was a person pointing at a day. A rule
/// that argues with a deliberate gesture reads as a fault, not as care, so the
/// owner retired it on 2026-09-01.
///
/// What survives is the pin, with a narrower promise: `slot_pinned_at` now
/// means "the auto-layout does not put anything here", nothing more. A person
/// dropping a second release on a pinned day can see the lock and means it.
pub fn schedule(conn: &Connection, id: &str, slot: &str) -> Result<Scheduling> {
    schedule_at(conn, id, slot, &now())
}

/// Put a release in a slot with the change's timestamp already decided.
///
/// The seam a replay comes back through: live, `schedule` stamps `now()`;
/// replaying, the log supplies the moment the first run recorded. See ADR 0014.
pub fn schedule_at(conn: &Connection, id: &str, slot: &str, at: &str) -> Result<Scheduling> {
    conn.execute(
        "UPDATE release SET scheduled_at = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, slot, at],
    )?;

    Ok(Scheduling {
        release: get(conn, id)?.ok_or_else(|| unknown_release(id))?,
    })
}

/// What a day already holds, without moving anything.
///
/// This used to be the dry run of [`schedule`] back when scheduling was a
/// contest. Nothing contests a date any more, so the question it answers is
/// narrower and plainer: is that day free, does something already sit there,
/// and is it pinned. Two callers need exactly that — the auto-layout, which
/// only fills free days, and the calendar, which says what a day holds before
/// a release lands beside it.
pub fn preview(conn: &Connection, id: &str, slot: &str) -> Result<SlotPreview> {
    let judged = judge(conn, id, slot)?;

    let holder_title = judged
        .occupant
        .as_ref()
        .map(|occupant| crate::journal::work_title(conn, &occupant.work_id).unwrap_or_default());

    let work_id: String = conn.query_row(
        "SELECT work_id FROM release WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    let repeats = crate::register::guard::at_slot(conn, &work_id, slot)?;
    Ok(SlotPreview {
        verdict: judged.verdict,
        holder_title,
        repeats,
    })
}

/// Look at what `slot` already holds, from the point of view of `id`.
///
/// It decided a contest until v0.44 and now only reports: nothing there, or
/// something there, or something there that is pinned. The reading is still
/// done in one place because two callers depend on it agreeing with itself —
/// the auto-layout refuses a day that is not `Empty`, and the calendar tells a
/// person what a day holds.
fn judge(conn: &Connection, id: &str, slot: &str) -> Result<Contest> {
    // Read first purely so an unknown id errs rather than reporting an empty
    // day. Nothing else is needed from the row: it used to supply the work
    // whose score was weighed against the occupant's, and nothing is weighed
    // any more.
    if conn
        .query_row(
            &format!("{SELECT_RELEASE} WHERE id = ?1"),
            params![id],
            read_row,
        )
        .optional()?
        .is_none()
    {
        return Err(unknown_release(id));
    }

    // Whoever is already on the day, ignoring this release itself.
    let occupant = conn
        .query_row(
            &format!(
                "{SELECT_RELEASE} WHERE scheduled_at = ?1 AND id <> ?2 AND status = '{PLANNED}' LIMIT 1"
            ),
            params![slot, id],
            read_row,
        )
        .optional()?
        .map(RawRelease::into_release)
        .transpose()?;

    let Some(occupant) = occupant else {
        return Ok(Contest {
            occupant: None,
            verdict: Verdict::Empty,
        });
    };

    // Strength used to be weighed here. Nothing takes a date from anything any
    // more, so the only distinction left is whether the day was settled by
    // hand: a pin tells the auto-layout to stay away, and tells a person that
    // someone already meant this date.
    let verdict = if occupant.slot_pinned_at.is_some() {
        Verdict::Pinned
    } else {
        Verdict::Taken
    };

    Ok(Contest {
        occupant: Some(occupant),
        verdict,
    })
}

/// Settle a date, or hand it back to the contest.
///
/// Pinning a release with no date would pin nothing, so it is refused rather
/// than silently accepted.
pub fn set_slot_pin(conn: &Connection, id: &str, pinned: bool) -> Result<Release> {
    set_slot_pin_at(conn, id, pinned, &now())
}

/// Pin or unpin a slot with the change's timestamp already decided.
///
/// The seam a replay comes back through: live, `set_slot_pin` stamps `now()`;
/// replaying, the log supplies the moment the first run recorded. See ADR 0014.
pub fn set_slot_pin_at(conn: &Connection, id: &str, pinned: bool, at: &str) -> Result<Release> {
    let scheduled: Option<Option<String>> = conn
        .query_row(
            "SELECT scheduled_at FROM release WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?;

    let Some(scheduled) = scheduled else {
        return Err(unknown_release(id));
    };
    if pinned && scheduled.is_none() {
        return Err(Error::refused("release.noSlotToPin"));
    }

    conn.execute(
        "UPDATE release SET slot_pinned_at = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, pinned.then(|| at.to_owned()), at],
    )?;

    get(conn, id)?.ok_or_else(|| unknown_release(id))
}

/// Take a release out of the calendar without deleting it.
pub fn unschedule(conn: &Connection, id: &str) -> Result<Release> {
    unschedule_at(conn, id, &now())
}

/// Take a release out of the calendar with the change's timestamp already
/// decided.
///
/// The seam a replay comes back through: live, `unschedule` stamps `now()`;
/// replaying, the log supplies the moment the first run recorded. See ADR 0014.
pub fn unschedule_at(conn: &Connection, id: &str, at: &str) -> Result<Release> {
    // The pin goes with the date. A pin describes a date that was decided, and
    // there is no longer a date — leaving it behind creates a state nothing
    // else in the app can produce and `set_slot_pin` explicitly refuses.
    if conn.execute(
        "UPDATE release SET scheduled_at = NULL, slot_pinned_at = NULL, updated_at = ?2 WHERE id = ?1",
        params![id, at],
    )? == 0
    {
        return Err(unknown_release(id));
    }

    get(conn, id)?.ok_or_else(|| unknown_release(id))
}

/// Mark a release as out, optionally with the link it went out on.
///
/// Shipping is a state, not an integration: nothing is published from here.
/// Records that a release went out, optionally on a day the caller names.
///
/// `at` is a plain date the person picks — a release marked days after it
/// shipped, or an archive imported with its real history, would otherwise all
/// claim to have gone out the moment the button was pressed. Left out, the
/// moment is now.
///
/// A `None` url keeps whatever link is already recorded rather than clearing
/// it, so marking a release twice does not lose the link the first pass wrote.
pub fn mark_released(
    conn: &Connection,
    id: &str,
    url: Option<String>,
    at: Option<String>,
) -> Result<Release> {
    mark_released_at(conn, id, url, at, &now())
}

/// Mark a release as out with the change's timestamp already decided.
///
/// The seam a replay comes back through: live, `mark_released` stamps
/// `now()` for `updated_at` (and, when the caller names no release day, for
/// `released_at` too); replaying, the log supplies the moment the first run
/// recorded. `on_day` is the release's own day, a separate piece of intent
/// the person picks — it is untouched here. See ADR 0014.
pub fn mark_released_at(
    conn: &Connection,
    id: &str,
    url: Option<String>,
    on_day: Option<String>,
    at: &str,
) -> Result<Release> {
    let released_at = match on_day {
        Some(day) => day_stamp(&day)?,
        None => at.to_owned(),
    };

    if conn.execute(
        "UPDATE release SET status = ?2, released_at = ?3, url = coalesce(?4, url), updated_at = ?5
         WHERE id = ?1",
        params![id, RELEASED, released_at, url, at],
    )? == 0
    {
        return Err(unknown_release(id));
    }

    get(conn, id)?.ok_or_else(|| unknown_release(id))
}

/// Takes back the mark: the release is planned again, as if it never went out.
///
/// **The link is kept.** A mark undone is usually a mis-click or a release
/// pulled after the fact, and the address it was published under is the one
/// piece of the episode worth nothing to retype. Clearing the url is a separate
/// edit, which the release form already offers.
///
/// Both fields move together on purpose: `status` alone would leave a row
/// calling itself planned while still carrying the day it shipped, and every
/// reader — the work's derived status, the dashboard, the export — would get a
/// different answer depending on which field it happened to look at.
pub fn unmark_released(conn: &Connection, id: &str) -> Result<Release> {
    unmark_released_at(conn, id, &now())
}

/// Take back the release mark with the change's timestamp already decided.
///
/// The seam a replay comes back through: live, `unmark_released` stamps
/// `now()`; replaying, the log supplies the moment the first run recorded.
/// See ADR 0014.
pub fn unmark_released_at(conn: &Connection, id: &str, at: &str) -> Result<Release> {
    if conn.execute(
        "UPDATE release SET status = ?2, released_at = NULL, updated_at = ?3 WHERE id = ?1",
        params![id, PLANNED, at],
    )? == 0
    {
        return Err(unknown_release(id));
    }

    get(conn, id)?.ok_or_else(|| unknown_release(id))
}

/// A calendar day (`2026-09-02`) as a stored timestamp.
///
/// Slots are days — the whole calendar is built on that — but `released_at`
/// holds a full instant, and mixing the two shapes in one column is what makes
/// text ordering stop meaning anything. So a picked day becomes midday UTC:
/// far enough from either edge that no reasonable timezone shifts it onto the
/// neighbouring date.
fn day_stamp(day: &str) -> Result<String> {
    let plausible = day.len() == 10
        && day.as_bytes()[4] == b'-'
        && day.as_bytes()[7] == b'-'
        && day
            .as_bytes()
            .iter()
            .enumerate()
            .all(|(at, byte)| at == 4 || at == 7 || byte.is_ascii_digit());

    if !plausible {
        return Err(Error::refused("release.badDay").param("value", day));
    }

    Ok(format!("{day}T12:00:00.000Z"))
}

pub fn update(conn: &Connection, id: &str, patch: ReleasePatch) -> Result<Release> {
    update_at(conn, id, patch, &now())
}

/// Apply a patch with the change's timestamp already decided.
///
/// The seam a replay comes back through: live, `update` stamps `now()`;
/// replaying, the log supplies the moment the first run recorded. See ADR 0014.
pub fn update_at(conn: &Connection, id: &str, patch: ReleasePatch, at: &str) -> Result<Release> {
    let mut patch = patch;
    // The fields keep the tails their door keeps, whoever wrote them and
    // whichever door the release moves to (v0.90): normalised here, where
    // every writer passes, rather than by each screen.
    if patch.meta.is_some() || patch.kind.is_some() {
        let found = get(conn, id)?.ok_or_else(|| unknown_release(id))?;
        let door = patch.kind.clone().unwrap_or_else(|| found.kind.clone());
        let mut meta = patch.meta.clone().unwrap_or_else(|| found.meta.clone());
        crate::release_meta::keep_suffixes(
            conn,
            &found.work_id,
            Some(&found.kind),
            &door,
            &mut meta,
        )?;
        if patch.meta.is_some() || meta != found.meta {
            patch.meta = Some(meta);
        }
    }
    let mut assignments: Vec<String> = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    fn set(
        assignments: &mut Vec<String>,
        values: &mut Vec<Box<dyn rusqlite::ToSql>>,
        column: &str,
        value: Box<dyn rusqlite::ToSql>,
    ) {
        values.push(value);
        assignments.push(format!("{column} = ?{}", values.len()));
    }

    if let Some(kind) = patch.kind {
        set(&mut assignments, &mut values, "kind", Box::new(kind));
    }
    if let Some(status) = patch.status {
        set(&mut assignments, &mut values, "status", Box::new(status));
    }
    check_when(
        patch.scheduled_time.as_ref().and_then(|t| t.as_deref()),
        patch.time_zone.as_ref().and_then(|z| z.as_deref()),
    )?;
    if let Some(time) = patch.scheduled_time {
        set(
            &mut assignments,
            &mut values,
            "scheduled_time",
            Box::new(time),
        );
    }
    if let Some(zone) = patch.time_zone {
        set(&mut assignments, &mut values, "time_zone", Box::new(zone));
    }
    if let Some(scheduled_at) = patch.scheduled_at {
        // Clearing the date clears the pin with it, for the reason given in
        // `unschedule`: a pin without a date is a state nothing can act on.
        let clearing = scheduled_at.is_none();
        set(
            &mut assignments,
            &mut values,
            "scheduled_at",
            Box::new(scheduled_at),
        );
        if clearing {
            set(
                &mut assignments,
                &mut values,
                "slot_pinned_at",
                Box::new(None::<String>),
            );
        }
    }
    if let Some(url) = patch.url {
        set(&mut assignments, &mut values, "url", Box::new(url));
    }
    if let Some(meta) = patch.meta {
        set(
            &mut assignments,
            &mut values,
            "meta",
            Box::new(Value::Object(meta).to_string()),
        );
    }

    if assignments.is_empty() {
        return get(conn, id)?.ok_or_else(|| unknown_release(id));
    }

    set(
        &mut assignments,
        &mut values,
        "updated_at",
        Box::new(at.to_owned()),
    );
    values.push(Box::new(id.to_owned()));

    let sql = format!(
        "UPDATE release SET {} WHERE id = ?{}",
        assignments.join(", "),
        values.len()
    );
    let params = rusqlite::params_from_iter(values.iter().map(AsRef::as_ref));

    if conn.execute(&sql, params)? == 0 {
        return Err(unknown_release(id));
    }

    get(conn, id)?.ok_or_else(|| unknown_release(id))
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    if conn.execute("DELETE FROM release WHERE id = ?1", params![id])? == 0 {
        return Err(unknown_release(id));
    }
    Ok(())
}

const SELECT_SCHEDULED_TEMPLATE: &str = "SELECT r.id, r.work_id, r.kind, r.status, r.scheduled_at, \
     r.released_at, r.url, r.slot_pinned_at, r.meta, r.created_at, r.updated_at, \
     w.title, s.total, s.tier, r.scheduled_time, r.time_zone \
     FROM release r \
     JOIN work w ON w.id = r.work_id \
     LEFT JOIN work_score s ON s.id = {speaking} \
     WHERE w.profile_id = ?1";

/// The listing query with the shared scoring rule filled in.
///
/// A release of a work nobody judges - an audio release, a clip in a profile
/// whose clips name no axes - is weighed by the score of what it was made
/// from: it goes out for that song, and it is as strong in the queue, the
/// calendar and the auto-layout as the song is (v0.86). A work of a judged
/// kind speaks with its own score, scored or not.
fn select_scheduled(config: &crate::profile::config::ProfileConfig) -> String {
    let unjudged: Vec<String> = config
        .work_kinds
        .iter()
        .filter(|kind| kind.axes.is_empty())
        .map(|kind| format!("'{}'", kind.key.replace('\'', "''")))
        .collect();
    let judged_by = if unjudged.is_empty() {
        "r.work_id".to_owned()
    } else {
        format!(
            "(CASE WHEN w.kind IN ({}) THEN coalesce((SELECT l.source_id FROM work_link l \
              WHERE l.work_id = r.work_id ORDER BY l.created_at, l.rowid LIMIT 1), r.work_id) \
              ELSE r.work_id END)",
            unjudged.join(", ")
        )
    };
    SELECT_SCHEDULED_TEMPLATE.replace("{speaking}", &crate::score::speaking_score_for(&judged_by))
}

/// Everything with a slot, in calendar order.
pub fn calendar(conn: &Connection, profile_id: &str) -> Result<Vec<ScheduledRelease>> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let mut statement = conn.prepare(&format!(
        "{} AND r.scheduled_at IS NOT NULL ORDER BY r.scheduled_at, w.title",
        select_scheduled(&config)
    ))?;
    read_scheduled(conn, &mut statement, profile_id)
}

/// Everything planned but unscheduled, strongest first — the queue that feeds
/// the calendar.
pub fn queue(conn: &Connection, profile_id: &str) -> Result<Vec<ScheduledRelease>> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let mut statement = conn.prepare(&format!(
        "{} AND r.scheduled_at IS NULL AND r.status = '{PLANNED}' \
         ORDER BY s.total IS NULL, s.total DESC, w.title",
        select_scheduled(&config)
    ))?;
    read_scheduled(conn, &mut statement, profile_id)
}

/// The ids of one work's releases that currently hold a slot.
///
/// Deliberately narrow where [`for_work`] is rich: taking a work off the
/// calendar needs nothing but the ids, and asking for readiness and scores to
/// throw them away would cost a join per work in a batch.
///
/// Anything already released is left out. Its date is a record of what happened,
/// not a booking, and unscheduling it would rewrite history.
pub fn scheduled_for(conn: &Connection, work_id: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT id FROM release
          WHERE work_id = ?1 AND scheduled_at IS NOT NULL AND status = ?2
          ORDER BY scheduled_at, rowid",
    )?;
    let rows = statement.query_map(params![work_id, PLANNED], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Releases of one work, whatever their state, with the context the card needs
/// to act on them.
///
/// The same shape the calendar reads. The work's own tab used to get the bare
/// row, which left it showing a release the chip could describe far better —
/// no readiness, no score, nothing to tell a plan from something that shipped.
/// Two views onto one release disagreeing about what it is was the whole
/// complaint.
pub fn for_work(
    conn: &Connection,
    profile_id: &str,
    work_id: &str,
) -> Result<Vec<ScheduledRelease>> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let mut statement = conn.prepare(&format!(
        "{} AND r.work_id = ?2 ORDER BY coalesce(r.scheduled_at, r.created_at), r.rowid",
        select_scheduled(&config)
    ))?;
    read_scheduled_where(
        conn,
        &mut statement,
        params![profile_id, work_id],
        profile_id,
    )
}

fn read_scheduled(
    conn: &Connection,
    statement: &mut rusqlite::Statement<'_>,
    profile_id: &str,
) -> Result<Vec<ScheduledRelease>> {
    read_scheduled_where(conn, statement, params![profile_id], profile_id)
}

/// The listing readers share, for queries that bind more than the profile.
fn read_scheduled_where(
    conn: &Connection,
    statement: &mut rusqlite::Statement<'_>,
    bound: impl rusqlite::Params,
    profile_id: &str,
) -> Result<Vec<ScheduledRelease>> {
    let rows = statement
        .query_map(bound, |row| {
            Ok((
                RawRelease {
                    id: row.get(0)?,
                    work_id: row.get(1)?,
                    kind: row.get(2)?,
                    status: row.get(3)?,
                    scheduled_at: row.get(4)?,
                    released_at: row.get(5)?,
                    url: row.get(6)?,
                    slot_pinned_at: row.get(7)?,
                    meta: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    scheduled_time: row.get(14)?,
                    time_zone: row.get(15)?,
                },
                row.get::<_, String>(11)?,
                row.get::<_, Option<f64>>(12)?,
                row.get::<_, Option<String>>(13)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    // Judged here rather than by the screens: the warning the journal writes
    // about an unready release must mean exactly what the chip shows.
    let config = crate::profile::config_for(conn, profile_id)?;
    let roles = crate::readiness::roles_present(conn, profile_id)?;
    let none = std::collections::BTreeSet::new();
    // Which kind each work is, so a release is judged by its work's own
    // vocabulary rather than by one the profile no longer keeps flat.
    let kinds: std::collections::HashMap<String, String> = conn
        .prepare("SELECT id, kind FROM work WHERE profile_id = ?1")?
        .query_map(params![profile_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    // And how far along each one is, so a chip can show the dial beside the
    // readiness marks. Only the works that have been judged are in the map.
    let stages: std::collections::HashMap<String, i64> = conn
        .prepare("SELECT id, stage FROM work WHERE profile_id = ?1 AND stage IS NOT NULL")?
        .query_map(params![profile_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;

    rows.into_iter()
        .map(|(raw, work_title, total, tier)| {
            let readiness = crate::readiness::assess(
                &config,
                kinds.get(&raw.work_id).map_or("", String::as_str),
                &raw.kind,
                roles.get(&raw.work_id).unwrap_or(&none),
                total.is_some(),
            );
            Ok(ScheduledRelease {
                work_kind: kinds.get(&raw.work_id).cloned().unwrap_or_default(),
                work_stage: stages.get(&raw.work_id).copied(),
                release: raw.into_release()?,
                work_title,
                total,
                tier,
                readiness,
            })
        })
        .collect()
}

fn unknown_release(id: &str) -> Error {
    Error::not_found("release", id)
}

struct RawRelease {
    id: String,
    work_id: String,
    kind: String,
    status: String,
    scheduled_at: Option<String>,
    released_at: Option<String>,
    url: Option<String>,
    slot_pinned_at: Option<String>,
    meta: String,
    created_at: String,
    updated_at: String,
    scheduled_time: Option<String>,
    time_zone: Option<String>,
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawRelease> {
    Ok(RawRelease {
        id: row.get(0)?,
        work_id: row.get(1)?,
        kind: row.get(2)?,
        status: row.get(3)?,
        scheduled_at: row.get(4)?,
        released_at: row.get(5)?,
        url: row.get(6)?,
        slot_pinned_at: row.get(7)?,
        meta: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        scheduled_time: row.get(11)?,
        time_zone: row.get(12)?,
    })
}

impl RawRelease {
    fn into_release(self) -> Result<Release> {
        Ok(Release {
            meta: serde_json::from_str(&self.meta)?,
            id: self.id,
            work_id: self.work_id,
            kind: self.kind,
            status: self.status,
            scheduled_at: self.scheduled_at,
            released_at: self.released_at,
            url: self.url,
            slot_pinned_at: self.slot_pinned_at,
            scheduled_time: self.scheduled_time,
            time_zone: self.time_zone,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::score::{self, NewScore};
    use crate::work::{self, NewWork};
    use serde_json::json;

    /// A work with an optional score, and one planned clip release.
    fn planned(conn: &Connection, profile_id: &str, title: &str, hook: Option<f64>) -> Release {
        let work = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "song".into(),
                title: title.into(),
                ..NewWork::default()
            },
        )
        .unwrap();

        if let Some(hook) = hook {
            score::create(
                conn,
                &work.id,
                NewScore {
                    axes: json!({ "hook": hook }).as_object().cloned().unwrap(),
                    version_id: None,
                    note: None,
                    rater: None,
                },
            )
            .unwrap();
        }

        create(
            conn,
            NewRelease {
                work_id: work.id,
                kind: "audio".into(),
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn a_new_release_starts_planned_and_unscheduled() {
        let (conn, profile_id) = fixtures::workspace();

        let release = planned(&conn, &profile_id, "Subject", None);

        assert_eq!(release.status, PLANNED);
        assert!(release.scheduled_at.is_none());
        assert!(release.released_at.is_none());
    }

    #[test]
    fn scheduling_an_empty_slot_displaces_nothing() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Subject", Some(8.0));

        let result = schedule(&conn, &release.id, "2026-09-01").unwrap();
        assert_eq!(result.release.scheduled_at.as_deref(), Some("2026-09-01"));
    }

    /// A day holds as many releases as are put on it.
    ///
    /// It held exactly one until v0.44, and a stronger work took the date from
    /// a weaker one. Four tests stood here for that rule; the owner retired it
    /// on 2026-09-01 because the only thing that ever asked for a date was a
    /// person pointing at a day, and a rule arguing with a deliberate gesture
    /// reads as a fault. What replaces them is the opposite guarantee: nothing
    /// is refused, and nothing is quietly evicted.
    #[test]
    fn a_second_release_joins_a_day_rather_than_taking_it() {
        let (conn, profile_id) = fixtures::workspace();
        let first = planned(&conn, &profile_id, "First", Some(9.0));
        let second = planned(&conn, &profile_id, "Second", Some(4.0));

        schedule(&conn, &first.id, "2026-09-01").unwrap();
        schedule(&conn, &second.id, "2026-09-01").unwrap();
        let calendar = calendar(&conn, &profile_id).unwrap();
        assert_eq!(calendar.len(), 2, "both hold the day");
        assert!(
            calendar
                .iter()
                .all(|entry| entry.release.scheduled_at.as_deref() == Some("2026-09-01"))
        );

        // And the weaker one is not in the queue, because it was not sent back.
        let queued = queue(&conn, &profile_id).unwrap();
        assert!(
            queued.is_empty(),
            "neither release was returned to the queue"
        );
    }

    /// The order the two arrived in does not change the outcome.
    ///
    /// The old rule was asymmetric on purpose — who was already there mattered.
    /// This checks the new one is not, because an asymmetry nobody declared is
    /// the kind that shows up as a mystery months later.
    #[test]
    fn the_weaker_one_first_ends_the_same_way() {
        let (conn, profile_id) = fixtures::workspace();
        let weak = planned(&conn, &profile_id, "Weak", Some(4.0));
        let strong = planned(&conn, &profile_id, "Strong", Some(9.0));

        schedule(&conn, &weak.id, "2026-09-01").unwrap();
        schedule(&conn, &strong.id, "2026-09-01").unwrap();

        assert_eq!(calendar(&conn, &profile_id).unwrap().len(), 2);
        assert!(queue(&conn, &profile_id).unwrap().is_empty());
    }

    /// An unscored release is no longer a lesser citizen of the calendar.
    #[test]
    fn an_unscored_release_can_share_a_day_with_a_scored_one() {
        let (conn, profile_id) = fixtures::workspace();
        let scored = planned(&conn, &profile_id, "Scored", Some(3.0));
        let unscored = planned(&conn, &profile_id, "Unscored", None);

        schedule(&conn, &scored.id, "2026-09-01").unwrap();
        schedule(&conn, &unscored.id, "2026-09-01").unwrap();

        assert_eq!(calendar(&conn, &profile_id).unwrap().len(), 2);
    }

    #[test]
    fn rescheduling_the_same_release_does_not_displace_itself() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Subject", Some(6.0));
        schedule(&conn, &release.id, "2026-09-01").unwrap();

        let result = schedule(&conn, &release.id, "2026-09-01").unwrap();
        assert_eq!(result.release.scheduled_at.as_deref(), Some("2026-09-01"));
    }

    /// The contest and the screens must agree about how strong a work is.
    ///
    /// Until v0.24 the calendar read the latest snapshot while the catalogue
    /// read the current version's, so one work showed 68.1 on one screen and
    /// 77.8 on the next. The contest that made the disagreement expensive is
    /// gone, but the disagreement itself would still be a bug: the queue is
    /// ordered by this number and a person chooses what to ship by it.
    #[test]
    fn one_score_speaks_for_a_work_on_every_screen() {
        let (conn, profile_id) = fixtures::workspace();
        let held = planned(&conn, &profile_id, "Subject", Some(7.0));

        // A second, weaker score taken later: with no current version named,
        // the strongest still speaks for the work.
        score::create(
            &conn,
            &held.work_id,
            NewScore {
                axes: json!({ "hook": 3.0 }).as_object().cloned().unwrap(),
                version_id: None,
                note: None,
                rater: None,
            },
        )
        .unwrap();

        let strongest = score::history(&conn, &held.work_id)
            .unwrap()
            .into_iter()
            .map(|score| score.total)
            .fold(f64::NEG_INFINITY, f64::max);

        // The queue is where the number is read from before a date exists...
        let queued = queue(&conn, &profile_id).unwrap();
        let waiting = queued
            .iter()
            .find(|row| row.release.id == held.id)
            .expect("it is waiting for a date");
        assert_eq!(
            waiting.total,
            Some(strongest),
            "the queue reads another number"
        );

        // ...and the calendar is where it is read from afterwards.
        schedule(&conn, &held.id, "2026-09-01").unwrap();
        let shown = calendar(&conn, &profile_id).unwrap();
        let entry = shown.iter().find(|row| row.release.id == held.id).unwrap();
        assert_eq!(
            entry.total,
            Some(strongest),
            "the calendar reads another number"
        );
    }

    /// A pin is a message to the auto-layout, not a barrier to a person.
    ///
    /// It refused a stronger challenger until v0.44, when the contest went. The
    /// promise is narrower now and the test says exactly how narrow: the day
    /// still reads as `Pinned`, which is what `layout::plan` checks — but a
    /// person dropping something there is not stopped, because they can see the
    /// lock and mean it.
    #[test]
    fn a_pinned_day_warns_the_layout_without_refusing_a_person() {
        let (conn, profile_id) = fixtures::workspace();

        let held = planned(&conn, &profile_id, "Held", Some(4.0));
        schedule(&conn, &held.id, "2026-09-01").unwrap();
        set_slot_pin(&conn, &held.id, true).unwrap();

        // What the auto-layout reads — always on behalf of something else, so
        // that is how it is asked here. (Asked on behalf of the pinned release
        // itself the day reads empty, because nothing occupies a date against
        // itself; that is checked in `the_preview_reports_what_the_day_holds`.)
        let other = planned(&conn, &profile_id, "Other", Some(9.0));
        let seen = preview(&conn, &other.id, "2026-09-01").unwrap();
        assert_ne!(seen.verdict, Verdict::Empty, "the layout must skip it");
        assert_eq!(seen.verdict, Verdict::Pinned);
        assert_eq!(seen.holder_title.as_deref(), Some("Held"));

        // What a person gets: the date, beside the pinned release.
        schedule(&conn, &other.id, "2026-09-01").unwrap();
        assert_eq!(calendar(&conn, &profile_id).unwrap().len(), 2);

        // And the pinned one is untouched — neither evicted nor unpinned.
        let holder = get(&conn, &held.id).unwrap().unwrap();
        assert_eq!(holder.scheduled_at.as_deref(), Some("2026-09-01"));
        assert!(holder.slot_pinned_at.is_some());
    }

    /// A pin describes a date, so losing the date loses the pin — otherwise the
    /// release sits in a state `set_slot_pin` itself refuses to create. Found on
    /// a live database rather than reasoned about.
    ///
    /// Displacement is not listed here: a pinned slot refuses the challenge, so
    /// nothing displaced was ever pinned.
    #[test]
    fn losing_the_date_loses_the_pin() {
        let (conn, profile_id) = fixtures::workspace();

        // By unscheduling.
        let first = planned(&conn, &profile_id, "First", Some(5.0));
        schedule(&conn, &first.id, "2026-09-01").unwrap();
        set_slot_pin(&conn, &first.id, true).unwrap();
        let back = unschedule(&conn, &first.id).unwrap();
        assert_eq!(back.scheduled_at, None);
        assert_eq!(back.slot_pinned_at, None, "unscheduling kept the pin");

        // By clearing the date through a patch.
        schedule(&conn, &first.id, "2026-09-02").unwrap();
        set_slot_pin(&conn, &first.id, true).unwrap();
        let cleared = update(
            &conn,
            &first.id,
            ReleasePatch {
                scheduled_at: Some(None),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            cleared.slot_pinned_at, None,
            "clearing the date kept the pin"
        );
    }

    /// The two ways a date gets written now agree.
    ///
    /// This test is older than the rule it checks. In v0.24 `update` wrote a
    /// date without asking who held it while `schedule` ran a contest, and
    /// wiring dragging to the wrong one shipped a calendar where two releases
    /// could quietly share a day — the bug it was written to prevent. The
    /// owner retired the contest on 2026-09-01, so sharing a day is the
    /// intended outcome and the two paths differ only in what else they do:
    /// `update` edits a booking, `schedule` books one. Neither refuses.
    #[test]
    fn both_ways_of_writing_a_date_let_a_day_be_shared() {
        let (conn, profile_id) = fixtures::workspace();

        let holder = planned(&conn, &profile_id, "Holder", Some(9.0));
        schedule(&conn, &holder.id, "2026-09-05").unwrap();

        let other = planned(&conn, &profile_id, "Other", Some(1.0));
        update(
            &conn,
            &other.id,
            ReleasePatch {
                scheduled_at: Some(Some("2026-09-05".into())),
                ..Default::default()
            },
        )
        .unwrap();

        let sharing = calendar(&conn, &profile_id)
            .unwrap()
            .into_iter()
            .filter(|row| row.release.scheduled_at.as_deref() == Some("2026-09-05"))
            .count();
        assert_eq!(sharing, 2, "update put it on the day beside the holder");

        // And the other path, for the same pair, ends the same way: scheduling
        // the weaker one onto the taken day is not refused and evicts nothing.
        schedule(&conn, &other.id, "2026-09-05").unwrap();
        assert_eq!(
            calendar(&conn, &profile_id)
                .unwrap()
                .into_iter()
                .filter(|row| row.release.scheduled_at.as_deref() == Some("2026-09-05"))
                .count(),
            2,
            "scheduling onto a taken day agrees with editing onto it"
        );
    }

    /// There is no slot to pin on something that holds no date.
    #[test]
    fn pinning_needs_a_date() {
        let (conn, profile_id) = fixtures::workspace();
        let queued = planned(&conn, &profile_id, "Subject", Some(5.0));

        assert!(set_slot_pin(&conn, &queued.id, true).is_err());
    }

    #[test]
    fn a_released_slot_does_not_block_a_new_one() {
        let (conn, profile_id) = fixtures::workspace();
        let out = planned(&conn, &profile_id, "Already out", Some(9.0));
        schedule(&conn, &out.id, "2026-09-01").unwrap();
        mark_released(
            &conn,
            &out.id,
            Some("https://example.invalid/1".into()),
            None,
        )
        .unwrap();
        let next = planned(&conn, &profile_id, "Next", Some(2.0));

        // History occupies the date, but it is no longer a plan competing for it.
        schedule(&conn, &next.id, "2026-09-01").unwrap();
    }

    #[test]
    fn marking_released_records_the_link_and_the_date() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Subject", None);

        let out = mark_released(
            &conn,
            &release.id,
            Some("https://example.invalid/x".into()),
            None,
        )
        .unwrap();

        assert_eq!(out.status, RELEASED);
        assert!(out.released_at.is_some());
        assert_eq!(out.url.as_deref(), Some("https://example.invalid/x"));
    }

    #[test]
    fn marking_released_can_name_the_day_it_went_out() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Subject", None);

        let out = mark_released(&conn, &release.id, None, Some("2026-07-18".into())).unwrap();

        // The day the person named, not the moment they pressed the button.
        assert_eq!(
            out.released_at.as_deref().unwrap()[..10].to_owned(),
            "2026-07-18"
        );
        assert_eq!(out.status, RELEASED);
    }

    /// Without a day, the mark still means "just now" — the common case must
    /// not have been broken by making the uncommon one possible.
    #[test]
    fn marking_released_without_a_day_uses_this_moment() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Subject", None);
        let before = now();

        let out = mark_released(&conn, &release.id, None, None).unwrap();

        let stamp = out.released_at.unwrap();
        assert!(
            stamp >= before,
            "{stamp} is older than the call that wrote it"
        );
        assert_eq!(stamp.len(), before.len(), "a stored stamp keeps one width");
    }

    /// A day picked by hand and a moment recorded by the app have to end up the
    /// same shape, or ordering by this column stops meaning anything — the bug
    /// that cost a chat its order on CI.
    #[test]
    fn a_named_day_is_stored_the_same_width_as_a_recorded_moment() {
        let (conn, profile_id) = fixtures::workspace();
        let picked = planned(&conn, &profile_id, "Picked", None);
        let recorded = planned(&conn, &profile_id, "Recorded", None);

        let picked = mark_released(&conn, &picked.id, None, Some("2026-07-18".into())).unwrap();
        let recorded = mark_released(&conn, &recorded.id, None, None).unwrap();

        assert_eq!(
            picked.released_at.unwrap().len(),
            recorded.released_at.unwrap().len()
        );
    }

    #[test]
    fn a_day_that_is_not_a_day_is_refused() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Subject", None);

        for bad in ["18-07-2026", "2026-07-18T12:00:00Z", "tomorrow", "2026-7-8"] {
            assert!(
                mark_released(&conn, &release.id, None, Some(bad.into())).is_err(),
                "{bad} was accepted as a calendar day"
            );
        }

        // And the refusal left the release alone.
        assert_eq!(get(&conn, &release.id).unwrap().unwrap().status, PLANNED);
    }

    #[test]
    fn a_mark_can_be_taken_back() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Subject", None);
        mark_released(&conn, &release.id, None, None).unwrap();

        let back = unmark_released(&conn, &release.id).unwrap();

        assert_eq!(back.status, PLANNED);
        assert!(
            back.released_at.is_none(),
            "the day it went out survived the undo"
        );
    }

    /// Both halves move together. Status alone would leave a row calling itself
    /// planned while still carrying a release date, and every reader would then
    /// get a different answer depending on which field it looked at.
    #[test]
    fn taking_a_mark_back_leaves_no_half_released_row() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Subject", None);
        mark_released(&conn, &release.id, None, Some("2026-07-18".into())).unwrap();

        unmark_released(&conn, &release.id).unwrap();

        let row = get(&conn, &release.id).unwrap().unwrap();
        assert_eq!(
            (row.status.as_str(), row.released_at.is_some()),
            (PLANNED, false)
        );
    }

    /// The one thing an undo keeps. A mis-click should not cost the address the
    /// release was published under.
    #[test]
    fn taking_a_mark_back_keeps_the_link() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Subject", None);
        mark_released(
            &conn,
            &release.id,
            Some("https://example.invalid/x".into()),
            None,
        )
        .unwrap();

        let back = unmark_released(&conn, &release.id).unwrap();

        assert_eq!(back.url.as_deref(), Some("https://example.invalid/x"));
    }

    /// Unscheduled releases keep their date through the undo: the mark and the
    /// slot are different facts, and only one of them was taken back.
    #[test]
    fn taking_a_mark_back_leaves_the_slot_alone() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Subject", None);
        update(
            &conn,
            &release.id,
            ReleasePatch {
                scheduled_at: Some(Some("2026-07-18".into())),
                ..ReleasePatch::default()
            },
        )
        .unwrap();
        mark_released(&conn, &release.id, None, None).unwrap();

        let back = unmark_released(&conn, &release.id).unwrap();

        assert_eq!(back.scheduled_at.as_deref(), Some("2026-07-18"));
    }

    #[test]
    fn taking_back_a_mark_on_a_release_that_is_not_there_is_refused() {
        let (conn, _) = fixtures::workspace();

        assert!(unmark_released(&conn, "no-such-release").is_err());
    }

    #[test]
    fn marking_released_without_a_link_keeps_the_previous_one() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Subject", None);
        mark_released(
            &conn,
            &release.id,
            Some("https://example.invalid/x".into()),
            None,
        )
        .unwrap();

        let again = mark_released(&conn, &release.id, None, None).unwrap();

        assert_eq!(again.url.as_deref(), Some("https://example.invalid/x"));
    }

    #[test]
    fn the_queue_is_strongest_first_with_unscored_last() {
        let (conn, profile_id) = fixtures::workspace();
        planned(&conn, &profile_id, "Middle", Some(5.0));
        planned(&conn, &profile_id, "Unscored", None);
        planned(&conn, &profile_id, "Best", Some(9.0));

        let queue = queue(&conn, &profile_id).unwrap();

        assert_eq!(queue[0].work_title, "Best");
        assert_eq!(queue[1].work_title, "Middle");
        assert_eq!(queue[2].work_title, "Unscored");
        assert!(
            queue.iter().all(|entry| entry.work_kind == "song"),
            "a slot names the kind of its work"
        );
    }

    #[test]
    fn unscheduling_keeps_the_release_but_frees_the_slot() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Subject", Some(7.0));
        schedule(&conn, &release.id, "2026-09-01").unwrap();

        let freed = unschedule(&conn, &release.id).unwrap();

        assert!(freed.scheduled_at.is_none());
        assert_eq!(freed.status, PLANNED);
        assert!(calendar(&conn, &profile_id).unwrap().is_empty());
    }

    /// What `preview` reports is what the day actually holds.
    ///
    /// It was the dry run of a contest until v0.44 and each verdict was checked
    /// against the refusal it predicted. There are no refusals now, so the
    /// agreement worth checking is with the calendar: a day the preview calls
    /// empty holds nothing, a day it calls taken holds something, and the name
    /// it gives is the name of what is there. The auto-layout acts on this, so
    /// a drift would show up as a plan booking days that are not free.
    #[test]
    fn the_preview_reports_what_the_day_holds() {
        let (conn, profile_id) = fixtures::workspace();

        let first = planned(&conn, &profile_id, "First", Some(9.0));
        let second = planned(&conn, &profile_id, "Second", Some(4.0));
        let pinned = planned(&conn, &profile_id, "Pinned", Some(5.0));

        // An empty day, and the calendar agrees nothing is on it.
        let dry = preview(&conn, &first.id, "2026-09-01").unwrap();
        assert_eq!(dry.verdict, Verdict::Empty);
        assert!(dry.holder_title.is_none());
        assert!(calendar(&conn, &profile_id).unwrap().is_empty());

        // Once taken, the preview names what is there — and the drop still goes
        // through, because nothing is refused any more.
        schedule(&conn, &first.id, "2026-09-01").unwrap();
        let dry = preview(&conn, &second.id, "2026-09-01").unwrap();
        assert_eq!(dry.verdict, Verdict::Taken);
        assert_eq!(dry.holder_title.as_deref(), Some("First"));
        schedule(&conn, &second.id, "2026-09-01").unwrap();
        assert_eq!(calendar(&conn, &profile_id).unwrap().len(), 2);

        // A pinned day reads differently from a merely taken one: that is the
        // whole of what a pin now means.
        schedule(&conn, &pinned.id, "2026-09-03").unwrap();
        set_slot_pin(&conn, &pinned.id, true).unwrap();
        let dry = preview(&conn, &first.id, "2026-09-03").unwrap();
        assert_eq!(dry.verdict, Verdict::Pinned);
        assert_eq!(dry.holder_title.as_deref(), Some("Pinned"));

        // Looking at its own day reads as empty: a release does not occupy a
        // date against itself, or moving one a day sideways would report a
        // collision with the version of itself it is leaving.
        let own = preview(&conn, &first.id, "2026-09-01").unwrap();
        assert_eq!(own.verdict, Verdict::Taken, "the other one is still there");
        let alone = preview(&conn, &pinned.id, "2026-09-03").unwrap();
        assert_eq!(alone.verdict, Verdict::Empty, "only itself is on that day");

        // And an unknown release errs rather than reporting an empty day.
        assert!(preview(&conn, "nope", "2026-09-01").is_err());
    }

    /// A video with a YouTube release planned for it: a door that asks for
    /// a plot before it can go out. Scored along the video's own axes.
    fn planned_video(
        conn: &Connection,
        profile_id: &str,
        title: &str,
        mark: Option<f64>,
    ) -> Release {
        let work = work::create(
            conn,
            profile_id,
            NewWork {
                kind: "video".into(),
                title: title.into(),
                ..NewWork::default()
            },
        )
        .unwrap();
        if let Some(mark) = mark {
            score::create(
                conn,
                &work.id,
                NewScore {
                    axes: json!({ "dynamics": mark }).as_object().cloned().unwrap(),
                    version_id: None,
                    note: None,
                    rater: None,
                },
            )
            .unwrap();
        }
        create(
            conn,
            NewRelease {
                work_id: work.id,
                kind: "youtube".into(),
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn the_calendar_reports_how_ready_each_release_is() {
        let (conn, profile_id) = fixtures::workspace();

        // Scored but missing the role a YouTube release requires.
        let bare = planned_video(&conn, &profile_id, "Bare", Some(6.0));
        schedule(&conn, &bare.id, "2026-09-01").unwrap();

        // Scored, with a version for every required role.
        let full = planned_video(&conn, &profile_id, "Full", Some(7.0));
        crate::work::version::create(
            &conn,
            &full.work_id,
            crate::work::version::NewVersion {
                role: "plot".into(),
                body: "body".into(),
                label: None,
                meta: None,
                make_current: true,
                parent_version_id: None,
                trial_id: None,
            },
        )
        .unwrap();
        schedule(&conn, &full.id, "2026-09-02").unwrap();

        let shown = calendar(&conn, &profile_id).unwrap();
        let of = |id: &str| shown.iter().find(|row| row.release.id == id).unwrap();

        assert!(!of(&bare.id).readiness.ready);
        assert!(
            of(&bare.id)
                .readiness
                .roles
                .iter()
                .any(|mark| mark.present == Some(false)),
            "a missing required role must be marked missing"
        );
        assert!(of(&full.id).readiness.ready);

        // The queue is judged the same way.
        let queued = planned_video(&conn, &profile_id, "Queued unscored", None);
        let queue = queue(&conn, &profile_id).unwrap();
        let entry = queue
            .iter()
            .find(|row| row.release.id == queued.id)
            .unwrap();
        assert!(!entry.readiness.scored);
        assert!(!entry.readiness.ready);
    }

    /// A release of a work nobody judges goes out for what it was made from,
    /// and is weighed by that score; a judged work speaks for itself.
    #[test]
    fn an_unjudged_publication_is_weighed_by_its_songs_score() {
        let (conn, profile_id) = fixtures::workspace();
        let song = fixtures::song(&conn, &profile_id, "Harbour lights");
        fixtures::score(&conn, &song.id, json!({ "hook": 9.0 }));
        let made =
            crate::actions::work::derive(&conn, &song.id, "audio", None, Some("en"), None).unwrap();
        let release_id = made.release_id.unwrap();
        let clip = fixtures::video(&conn, &profile_id, "Harbour lights — clip");
        crate::link::create(
            &conn,
            &profile_id,
            crate::link::NewLink {
                work_id: clip.id.clone(),
                source_id: song.id.clone(),
                role: None,
                source_version_id: None,
            },
        )
        .unwrap();
        let clip_release = fixtures::release(&conn, &clip.id, "youtube", None);

        let queued = queue(&conn, &profile_id).unwrap();
        let audio = queued
            .iter()
            .find(|row| row.release.id == release_id)
            .unwrap();
        let song_total = crate::score::latest(&conn, &song.id)
            .unwrap()
            .unwrap()
            .total;
        assert_eq!(audio.total, Some(song_total));
        assert!(
            audio.readiness.ready,
            "nothing is asked of it that is not there"
        );
        let video = queued
            .iter()
            .find(|row| row.release.id == clip_release.id)
            .unwrap();
        assert_eq!(
            video.total, None,
            "the shipped video kind is judged: its own score, and it has none yet"
        );
    }

    #[test]
    fn deleting_a_work_takes_its_releases_with_it() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Doomed", None);

        work::delete(&conn, &release.work_id).unwrap();

        assert!(get(&conn, &release.id).unwrap().is_none());
    }

    #[test]
    fn a_release_needs_an_existing_work() {
        let (conn, _) = fixtures::workspace();

        let result = create(
            &conn,
            NewRelease {
                work_id: "nope".into(),
                kind: "audio".into(),
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        );

        assert!(result.is_err());
    }

    #[test]
    fn scheduled_for_lists_only_what_holds_a_slot() {
        let (conn, profile_id) = fixtures::workspace();
        let booked = planned(&conn, &profile_id, "Booked", Some(9.0));
        let waiting = planned(&conn, &profile_id, "Waiting", Some(8.0));

        schedule(&conn, &booked.id, "2026-09-10").unwrap();

        let held = scheduled_for(&conn, &booked.work_id).unwrap();
        assert_eq!(held, vec![booked.id]);

        // Planned but unscheduled: nothing to take off the calendar.
        assert!(scheduled_for(&conn, &waiting.work_id).unwrap().is_empty());
    }

    #[test]
    fn scheduled_for_leaves_a_released_date_alone() {
        let (conn, profile_id) = fixtures::workspace();
        let out = planned(&conn, &profile_id, "Shipped", Some(9.0));
        schedule(&conn, &out.id, "2026-09-10").unwrap();
        mark_released(&conn, &out.id, None, None).unwrap();

        // A released date records what happened. Unscheduling it would rewrite
        // history, so a batch must not be able to reach it.
        assert!(scheduled_for(&conn, &out.work_id).unwrap().is_empty());
    }

    #[test]
    fn a_release_carries_its_time_of_day_and_zone() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Subject", None);

        let timed = update(
            &conn,
            &release.id,
            ReleasePatch {
                scheduled_time: Some(Some("18:30".into())),
                time_zone: Some(Some("Europe/Lisbon".into())),
                ..ReleasePatch::default()
            },
        )
        .unwrap();
        assert_eq!(timed.scheduled_time.as_deref(), Some("18:30"));
        assert_eq!(timed.time_zone.as_deref(), Some("Europe/Lisbon"));

        let cleared = update(
            &conn,
            &release.id,
            ReleasePatch {
                scheduled_time: Some(None),
                ..ReleasePatch::default()
            },
        )
        .unwrap();
        assert_eq!(cleared.scheduled_time, None);
        assert_eq!(
            cleared.time_zone.as_deref(),
            Some("Europe/Lisbon"),
            "one field at a time"
        );

        let created = create(
            &conn,
            NewRelease {
                work_id: fixtures::video(&conn, &profile_id, "Another").id,
                kind: "youtube".into(),
                scheduled_at: None,
                meta: None,
                scheduled_time: Some("07:05".into()),
                time_zone: Some("UTC".into()),
            },
        )
        .unwrap();
        assert_eq!(created.scheduled_time.as_deref(), Some("07:05"));
        assert_eq!(created.time_zone.as_deref(), Some("UTC"));
    }

    /// A publication goes out once (ADR 0051): a second release is refused
    /// in words, before the index would refuse it as a constraint.
    #[test]
    fn a_publication_goes_out_once() {
        let (conn, profile_id) = fixtures::workspace();
        let video = fixtures::video(&conn, &profile_id, "Harbour lights (video)");
        let first = fixtures::release(&conn, &video.id, "youtube", None);

        let refused = create(
            &conn,
            NewRelease {
                work_id: video.id.clone(),
                kind: "premiere".into(),
                scheduled_at: None,
                meta: None,
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap_err();

        assert_eq!(
            refused.refusal().map(|r| r.code),
            Some("release.onePerPublication")
        );
        assert_eq!(
            of_work(&conn, &video.id).unwrap().map(|r| r.id),
            Some(first.id)
        );
    }

    /// The tail a door keeps is the release's to keep: typed without it, the
    /// title ends with it; moved to a door that keeps none, the title loses
    /// it; a title nobody wrote stays empty.
    #[test]
    fn a_field_keeps_the_tail_its_door_keeps() {
        let (conn, profile_id) = fixtures::workspace();
        let audio = fixtures::work(&conn, &profile_id, "audio", "Tide (audio)");
        let release = create(
            &conn,
            NewRelease {
                work_id: audio.id.clone(),
                kind: "youtube".into(),
                scheduled_at: None,
                meta: json!({ "title": "Tide - one line" }).as_object().cloned(),
                scheduled_time: None,
                time_zone: None,
            },
        )
        .unwrap();
        assert_eq!(release.meta["title"], "Tide - one line (audio)");

        let typed = update(
            &conn,
            &release.id,
            ReleasePatch {
                meta: json!({ "title": "Tide - another line", "description": "" })
                    .as_object()
                    .cloned(),
                ..ReleasePatch::default()
            },
        )
        .unwrap();
        assert_eq!(typed.meta["title"], "Tide - another line (audio)");
        assert_eq!(
            typed.meta["description"], "",
            "a field with no tail is left as typed"
        );

        let moved = update(
            &conn,
            &release.id,
            ReleasePatch {
                kind: Some("streaming".into()),
                ..ReleasePatch::default()
            },
        )
        .unwrap();
        assert_eq!(moved.meta["title"], "Tide - another line");

        let back = update(
            &conn,
            &release.id,
            ReleasePatch {
                kind: Some("youtube".into()),
                meta: json!({ "title": "" }).as_object().cloned(),
                ..ReleasePatch::default()
            },
        )
        .unwrap();
        assert_eq!(
            back.meta["title"], "",
            "a title nobody wrote is not \" (audio)\""
        );
    }

    #[test]
    fn a_time_or_a_zone_that_would_not_read_back_is_refused() {
        let (conn, profile_id) = fixtures::workspace();
        let release = planned(&conn, &profile_id, "Subject", None);

        for (time, zone) in [
            (Some("25:00"), None),
            (Some("noon"), None),
            (None, Some("Lisbon")),
            (None, Some("here/ there")),
        ] {
            let refused = update(
                &conn,
                &release.id,
                ReleasePatch {
                    scheduled_time: time.map(|t| Some(t.to_owned())),
                    time_zone: zone.map(|z| Some(z.to_owned())),
                    ..ReleasePatch::default()
                },
            )
            .unwrap_err();
            assert!(
                matches!(
                    refused.refusal().map(|r| r.code),
                    Some("release.badTime" | "release.badTimeZone")
                ),
                "{time:?} {zone:?}: {refused}"
            );
        }
        let untouched = get(&conn, &release.id).unwrap().unwrap();
        assert_eq!(untouched.scheduled_time, None);
        assert_eq!(untouched.time_zone, None);
    }
}
