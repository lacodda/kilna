//! The application's own log: what went wrong where nobody was looking.
//!
//! Until v0.83 every failure that was not worth failing a command over - a
//! journal line that could not be written, a file the trash could not remove,
//! a queued task that would not start - went to `eprintln!`. A release build on
//! Windows has no console, so all of it went nowhere: the one place a person
//! could have looked after "something odd happened yesterday" did not exist.
//!
//! Now it goes to `logs/kilna.log` beside the workspace (`kilna-mcp.log` for
//! the headless server, a separate process that must not interleave its lines
//! with the window's). Beside the workspace for the reason the media and the
//! assistant directory are: one directory is the whole workspace, and a live
//! run on a copy logs into the copy rather than into the original's history.
//!
//! Plain lines, one per event, rotated once at start when the file has grown
//! past a megabyte - there is no scheduler in this application to rotate on,
//! and a log that grows without end is the next thing somebody has to clean.
//! Nothing here fails: a log that cannot be written falls back to stderr, and
//! from there to nothing, rather than take down what it was reporting on.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Where the log lives inside the workspace directory.
pub const DIRECTORY: &str = "logs";

/// The window's log file.
pub const APP_FILE: &str = "kilna.log";

/// The headless server's log file.
pub const MCP_FILE: &str = "kilna-mcp.log";

/// Past this, the file is moved aside at the next start.
const ROTATE_AT: u64 = 1024 * 1024;

struct Sink {
    path: PathBuf,
    file: File,
}

static SINK: Mutex<Option<Sink>> = Mutex::new(None);

/// How much an event matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// Start writing to `<workspace_dir>/logs/<file>`.
///
/// Called once, when the workspace is known. Before it - and in tests, which
/// never call it - lines go to stderr. A second call moves the log.
pub fn init(workspace_dir: &Path, file: &str) {
    let dir = workspace_dir.join(DIRECTORY);
    let path = dir.join(file);
    let opened = std::fs::create_dir_all(&dir).and_then(|()| {
        rotate(&path);
        OpenOptions::new().create(true).append(true).open(&path)
    });
    match opened {
        Ok(file) => {
            let mut sink = SINK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *sink = Some(Sink { path, file });
        }
        Err(cause) => eprintln!("log: could not open {}: {cause}", path.display()),
    }
}

/// Move a grown log aside, replacing the previous one.
fn rotate(path: &Path) {
    let grown = std::fs::metadata(path).is_ok_and(|meta| meta.len() > ROTATE_AT);
    if grown {
        let mut aside = path.as_os_str().to_owned();
        aside.push(".1");
        let _ = std::fs::rename(path, PathBuf::from(aside));
    }
}

/// The file being written, once there is one.
pub fn path() -> Option<PathBuf> {
    SINK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .map(|sink| sink.path.clone())
}

/// Write one event.
///
/// `target` names the part of the application it came from - `trash`,
/// `assistant` - the way the old `eprintln!` lines began, so a log can be
/// read by grepping for one subsystem.
pub fn write(level: Level, target: &str, message: &str) {
    let line = format!(
        "{} {} {target}: {}\n",
        crate::time::now(),
        level.as_str(),
        // One event, one line: a message carrying a CLI's stderr would
        // otherwise break the file into lines that belong to nothing.
        message.replace(['\r', '\n'], " ")
    );
    let mut sink = SINK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let written = sink
        .as_mut()
        .is_some_and(|sink| sink.file.write_all(line.as_bytes()).is_ok());
    if !written {
        eprint!("{line}");
    }
}

pub fn info(target: &str, message: &str) {
    write(Level::Info, target, message);
}

pub fn warn(target: &str, message: &str) {
    write(Level::Warn, target, message);
}

pub fn error(target: &str, message: &str) {
    write(Level::Error, target, message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grown_log_is_moved_aside_and_a_small_one_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kilna.log");

        std::fs::write(&path, "small").unwrap();
        rotate(&path);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "small");

        std::fs::write(&path, vec![b'x'; (ROTATE_AT + 1) as usize]).unwrap();
        rotate(&path);
        assert!(!path.exists(), "the grown file was moved");
        assert!(dir.path().join("kilna.log.1").exists());
    }
}
