//! The armed receiver (v0.156, FR-21 21.2 p.1): a press on «Copy … and wait»
//! starts looking at the Downloads folder for the one file a generator is
//! about to save for one work - its track or its cover - and stops at the
//! first that arrives, at a press on Cancel, or when its time runs out.
//!
//! Only files that arrive after the press are looked at, only of the kind
//! the work waits for, and only once they have stopped growing: a browser
//! writes a download in pieces and renames it at the end. One work waits at
//! a time; arming another puts the first one down.
//!
//! How it works on this machine - which folder, how long, and whether what
//! arrives is taken without asking - is kept in a file beside the
//! application: a Downloads folder belongs to a machine, not to a space.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::actions::media::{MediaRole, TakenMedia};
use crate::error::{Error, Result};

/// How long a receiver waits unless set otherwise, in minutes.
pub const DEFAULT_MINUTES: u32 = 15;
/// The longest a receiver may be set to wait, in minutes.
pub const MOST_MINUTES: u32 = 240;

/// How the receiver works on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct ReceiverSettings {
    /// The folder looked at; none for the system's Downloads.
    pub folder: Option<String>,
    /// What arrives is moved in without asking.
    pub auto: bool,
    /// How long a receiver waits, in minutes.
    pub minutes: u32,
}

impl Default for ReceiverSettings {
    fn default() -> Self {
        Self {
            folder: None,
            auto: false,
            minutes: DEFAULT_MINUTES,
        }
    }
}

const FILE: &str = "receiver.json";

/// How the receiver works on this machine, as kept beside the application.
pub fn settings(app_dir: &Path) -> ReceiverSettings {
    std::fs::read_to_string(app_dir.join(FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Keep how the receiver works. A folder must be one that is there; the
/// time is one minute to four hours - refused rather than held to the
/// bounds, so what was typed is what is kept.
pub fn set_settings(app_dir: &Path, wanted: ReceiverSettings) -> Result<ReceiverSettings> {
    if !(1..=MOST_MINUTES).contains(&wanted.minutes) {
        return Err(Error::refused("receiver.badMinutes")
            .param("most", MOST_MINUTES)
            .param("value", wanted.minutes));
    }
    let folder = wanted
        .folder
        .as_deref()
        .map(str::trim)
        .filter(|folder| !folder.is_empty())
        .map(str::to_owned);
    if let Some(folder) = folder.as_deref()
        && !Path::new(folder).is_dir()
    {
        return Err(Error::refused("receiver.noFolder").param("path", folder));
    }
    let kept = ReceiverSettings { folder, ..wanted };
    std::fs::create_dir_all(app_dir)?;
    std::fs::write(app_dir.join(FILE), serde_json::to_string_pretty(&kept)?)?;
    Ok(kept)
}

/// A receiver waiting now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct ArmedReceiver {
    pub work_id: String,
    pub title: String,
    pub role: MediaRole,
    /// The folder looked at.
    pub folder: String,
    /// When it stops waiting, as a moment.
    pub until: String,
}

/// What arrived for a receiver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct ReceiverFound {
    pub work_id: String,
    pub title: String,
    pub role: MediaRole,
    /// Where it arrived, whole.
    pub path: String,
    pub name: String,
    /// Moved in already, by the setting that takes without asking.
    pub taken: Option<TakenMedia>,
}

/// Why a receiver stopped waiting with nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct ReceiverEnded {
    pub work_id: String,
    /// `expired`: its time ran out.
    pub reason: String,
}

/// How much earlier than the press a file's stamp may read and still be
/// counted as arriving after it.
const SLACK: std::time::Duration = std::time::Duration::from_millis(250);

/// The event a file that arrived is told under.
pub const FOUND_EVENT: &str = "receiver-found";
/// The event a receiver that stopped by itself is told under.
pub const ENDED_EVENT: &str = "receiver-ended";

/// The receiver waiting now, with the number it was armed under: a watch
/// that finds another number has been put down.
static ARMED: LazyLock<Mutex<Option<(u64, ArmedReceiver)>>> = LazyLock::new(|| Mutex::new(None));
static NEXT: AtomicU64 = AtomicU64::new(1);

/// The receiver waiting now, if one is.
pub fn armed() -> Option<ArmedReceiver> {
    ARMED
        .lock()
        .ok()
        .and_then(|armed| armed.as_ref().map(|(_, one)| one.clone()))
}

/// Arm a receiver, putting down the one before. The number it returns is
/// what its watch asks after.
pub fn arm(waiting: ArmedReceiver) -> u64 {
    let number = NEXT.fetch_add(1, Ordering::SeqCst);
    if let Ok(mut armed) = ARMED.lock() {
        *armed = Some((number, waiting));
    }
    number
}

/// Put the receiver down, whichever it is.
pub fn disarm() {
    if let Ok(mut armed) = ARMED.lock() {
        *armed = None;
    }
}

/// Put down the receiver armed under `number`, and only that one; whether it
/// was still the one waiting.
pub fn disarm_if(number: u64) -> bool {
    let Ok(mut armed) = ARMED.lock() else {
        return false;
    };
    if armed.as_ref().is_some_and(|(armed, _)| *armed == number) {
        *armed = None;
        return true;
    }
    false
}

/// Whether the receiver armed under `number` is still the one waiting.
pub fn still(number: u64) -> bool {
    ARMED
        .lock()
        .ok()
        .is_some_and(|armed| armed.as_ref().is_some_and(|(armed, _)| *armed == number))
}

/// The first file in `folder` that arrived at `since` or after, fits `role`,
/// and has stopped growing: the same length, more than nothing, at this
/// look as at the one before - `sizes` remembers the looks.
pub fn arrived(
    folder: &Path,
    since: SystemTime,
    role: MediaRole,
    sizes: &mut HashMap<PathBuf, u64>,
) -> Option<PathBuf> {
    let entries = std::fs::read_dir(folder).ok()?;
    let mut fresh: Vec<(SystemTime, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !role.fits(name) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        // A download's moment is when it began - its file was made then -
        // or when it was last written; either after the press is new.
        let made = meta.created().ok();
        let written = meta.modified().ok();
        let Some(moment) = made.into_iter().chain(written).max() else {
            continue;
        };
        // A file system stamps a file by a coarser clock than the one the
        // press was read from, and may stamp one written right after it a
        // few milliseconds before (CI's Linux did): a little slack, far
        // less than any download made before the press.
        if moment + SLACK < since {
            continue;
        }
        let length = meta.len();
        let before = sizes.insert(path.clone(), length);
        if length > 0 && before == Some(length) {
            fresh.push((moment, path));
        }
    }
    fresh.sort();
    fresh.into_iter().next().map(|(_, path)| path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_kept_beside_the_application_and_held_to_their_bounds() {
        let app = tempfile::tempdir().unwrap();
        assert_eq!(settings(app.path()), ReceiverSettings::default());

        let downloads = tempfile::tempdir().unwrap();
        let kept = set_settings(
            app.path(),
            ReceiverSettings {
                folder: Some(downloads.path().to_string_lossy().into_owned()),
                auto: true,
                minutes: 30,
            },
        )
        .unwrap();
        assert_eq!(settings(app.path()), kept);

        let too_long = set_settings(
            app.path(),
            ReceiverSettings {
                minutes: MOST_MINUTES + 1,
                ..ReceiverSettings::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            too_long.refusal().map(|one| one.code),
            Some("receiver.badMinutes")
        );
        let nowhere = set_settings(
            app.path(),
            ReceiverSettings {
                folder: Some(app.path().join("missing").to_string_lossy().into_owned()),
                ..ReceiverSettings::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            nowhere.refusal().map(|one| one.code),
            Some("receiver.noFolder")
        );
        assert_eq!(settings(app.path()), kept, "a refused change keeps nothing");
    }

    #[test]
    fn only_a_new_file_of_the_kind_waited_for_is_taken_once_it_stops_growing() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("old.mp3"), b"old").unwrap();
        // Coarse file clocks: the press is a moment after the old file.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let since = SystemTime::now();
        let mut sizes = HashMap::new();

        std::fs::write(folder.path().join("cover.png"), b"png").unwrap();
        std::fs::write(folder.path().join("take.mp3.crdownload"), b"part").unwrap();
        std::fs::write(folder.path().join("take.mp3"), b"").unwrap();
        assert_eq!(
            arrived(folder.path(), since, MediaRole::FinalAudio, &mut sizes),
            None
        );

        std::fs::write(folder.path().join("take.mp3"), b"ID3 one").unwrap();
        assert_eq!(
            arrived(folder.path(), since, MediaRole::FinalAudio, &mut sizes),
            None
        );
        assert_eq!(
            arrived(folder.path(), since, MediaRole::FinalAudio, &mut sizes),
            Some(folder.path().join("take.mp3"))
        );
    }

    #[test]
    fn a_receiver_armed_later_puts_the_first_down() {
        let first = arm(ArmedReceiver {
            work_id: "w1".into(),
            title: "Dawn".into(),
            role: MediaRole::FinalAudio,
            folder: "C:/Downloads".into(),
            until: "2030-01-01T00:15:00Z".into(),
        });
        let second = arm(ArmedReceiver {
            work_id: "w2".into(),
            title: "Dusk".into(),
            role: MediaRole::FinalCover,
            folder: "C:/Downloads".into(),
            until: "2030-01-01T00:15:00Z".into(),
        });
        assert!(!still(first));
        assert!(still(second));
        assert!(!disarm_if(first));
        assert_eq!(armed().map(|one| one.work_id), Some("w2".into()));
        assert!(disarm_if(second));
        assert_eq!(armed(), None);
    }
}
