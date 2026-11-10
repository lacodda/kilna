//! Who holds a space open (#102).
//!
//! A space is a folder, and a folder can be open on two machines at once - kept
//! in a synced cloud folder, or on a shared drive. SQLite's own locking does not
//! cross machines, and two windows writing one database through a sync would
//! lose one's writes to the other's. So a window that opens a space leaves a
//! file in it saying which machine has it, and renews it every minute; another
//! machine that finds it fresh opens the space for reading only and says who
//! holds it. A file older than [`STALE_AFTER`] holds nothing - its window died
//! without taking it away - and is taken over.
//!
//! The same machine is never kept out by its own file: one window per machine
//! is what the single instance of the application promises (v0.107), so a file
//! of this machine's is a window that crashed, or this one.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::Folder;
use crate::error::Result;

/// The file in a space's folder that says who has it open.
pub const LOCK_FILE: &str = "kilna.lock";

/// How often a window renews its hold.
pub const BEAT_EVERY: Duration = Duration::from_secs(60);

/// How old a hold may grow before it counts as left behind.
const STALE_AFTER: Duration = Duration::from_secs(5 * 60);

/// Who holds a space, as its file says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Holder {
    /// The machine's name, as its system gives it.
    pub machine: String,
    /// The moment the window took it, in seconds since 1970.
    #[ts(type = "number")]
    pub since: u64,
    /// The moment it last said it still holds it.
    #[ts(type = "number")]
    pub beat: u64,
}

/// What opening a space found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hold {
    /// This window holds it now.
    Ours,
    /// Another machine does; the space is read only here.
    Elsewhere(Holder),
}

/// This machine's name.
pub fn machine() -> String {
    ["COMPUTERNAME", "HOSTNAME"]
        .iter()
        .find_map(|key| {
            std::env::var(key)
                .ok()
                .filter(|name| !name.trim().is_empty())
        })
        .unwrap_or_else(|| "this machine".to_owned())
}

fn seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The holder the file names, when it names one.
pub fn read(folder: &Folder) -> Option<Holder> {
    let raw = std::fs::read_to_string(folder.path().join(LOCK_FILE)).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Whether a holder is another machine that still holds the space.
fn keeps_out(holder: &Holder, here: &str, at: u64) -> bool {
    holder.machine != here && at.saturating_sub(holder.beat) < STALE_AFTER.as_secs()
}

/// What a companion of the window - `kilna --mcp` - finds: whether another
/// machine holds the space. It takes nothing: the window of this machine may
/// hold it, and the companion works beside it.
pub fn check(folder: &Folder) -> Hold {
    match read(folder) {
        Some(holder) if keeps_out(&holder, &machine(), seconds_now()) => Hold::Elsewhere(holder),
        _ => Hold::Ours,
    }
}

/// Take the space for this window, unless another machine holds it.
pub fn take(folder: &Folder) -> Result<Hold> {
    take_as(folder, &machine(), seconds_now())
}

fn take_as(folder: &Folder, here: &str, at: u64) -> Result<Hold> {
    if let Some(holder) = read(folder)
        && keeps_out(&holder, here, at)
    {
        return Ok(Hold::Elsewhere(holder));
    }
    write(
        folder,
        &Holder {
            machine: here.to_owned(),
            since: at,
            beat: at,
        },
    )?;
    Ok(Hold::Ours)
}

/// Say the window still holds the space. A file that names another machine by
/// now - it took over a hold this one let go stale - is left to it.
pub fn beat(folder: &Folder) -> Result<()> {
    let here = machine();
    match read(folder) {
        Some(mut holder) if holder.machine == here => {
            holder.beat = seconds_now();
            write(folder, &holder)
        }
        Some(_) => Ok(()),
        None => write(
            folder,
            &Holder {
                machine: here,
                since: seconds_now(),
                beat: seconds_now(),
            },
        ),
    }
}

/// Let the space go: the file goes, when it is this machine's.
pub fn release(folder: &Folder) {
    if read(folder).is_some_and(|holder| holder.machine == machine()) {
        let _ = std::fs::remove_file(folder.path().join(LOCK_FILE));
    }
}

fn write(folder: &Folder, holder: &Holder) -> Result<()> {
    let path = folder.path().join(LOCK_FILE);
    let draft = folder.path().join(format!("{LOCK_FILE}.new"));
    std::fs::write(&draft, serde_json::to_string(holder)?)?;
    std::fs::rename(&draft, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder() -> (tempfile::TempDir, Folder) {
        let dir = tempfile::tempdir().unwrap();
        let folder = Folder::new(dir.path());
        (dir, folder)
    }

    #[test]
    fn a_space_nobody_holds_is_taken() {
        let (_dir, folder) = folder();

        assert_eq!(take_as(&folder, "desk", 1_000).unwrap(), Hold::Ours);
        assert_eq!(read(&folder).unwrap().machine, "desk");
    }

    #[test]
    fn another_machines_fresh_hold_keeps_this_one_to_reading() {
        let (_dir, folder) = folder();
        take_as(&folder, "laptop", 1_000).unwrap();

        let found = take_as(&folder, "desk", 1_000 + 60).unwrap();

        let Hold::Elsewhere(holder) = found else {
            panic!("the laptop still holds it: {found:?}")
        };
        assert_eq!(holder.machine, "laptop");
        assert_eq!(
            read(&folder).unwrap().machine,
            "laptop",
            "nothing taken over"
        );
    }

    #[test]
    fn a_hold_left_to_go_stale_is_taken_over() {
        let (_dir, folder) = folder();
        take_as(&folder, "laptop", 1_000).unwrap();

        let found = take_as(&folder, "desk", 1_000 + STALE_AFTER.as_secs()).unwrap();

        assert_eq!(found, Hold::Ours);
        assert_eq!(read(&folder).unwrap().machine, "desk");
    }

    #[test]
    fn a_machine_is_not_kept_out_by_its_own_hold() {
        let (_dir, folder) = folder();
        take_as(&folder, "desk", 1_000).unwrap();

        assert_eq!(take_as(&folder, "desk", 1_001).unwrap(), Hold::Ours);
    }

    #[test]
    fn a_hold_is_let_go_only_by_its_machine() {
        let (_dir, folder) = folder();
        take_as(&folder, "another", seconds_now()).unwrap();

        release(&folder);
        assert!(
            read(&folder).is_some(),
            "another machine's hold is not ours to remove"
        );
        assert!(matches!(check(&folder), Hold::Elsewhere(_)));

        write(
            &folder,
            &Holder {
                machine: machine(),
                since: seconds_now(),
                beat: seconds_now(),
            },
        )
        .unwrap();
        assert_eq!(check(&folder), Hold::Ours);
        release(&folder);
        assert_eq!(read(&folder), None);
        assert_eq!(check(&folder), Hold::Ours);
    }
}
