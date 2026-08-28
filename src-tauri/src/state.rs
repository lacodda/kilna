use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;

use crate::assistant::queue::Queue;
use crate::assistant::run::Runs;
use crate::db;
use crate::error::Result;
use crate::journal;
use crate::profile;

/// The open workspace, shared by every command.
///
/// A single connection behind a mutex is enough here: this is a desktop
/// application with one user, and SQLite writes serialise anyway.
pub struct AppState {
    connection: Mutex<Connection>,
    path: PathBuf,
    /// Assistant runs in flight. Held apart from the connection on purpose: a
    /// run lasting minutes must not sit on the lock the rest of the
    /// application writes through.
    runs: Arc<Runs>,
    /// Tasks asked for while every slot was taken. Memory only — see
    /// [`crate::assistant::queue`].
    queue: Arc<Queue>,
}

impl AppState {
    /// Open the workspace at `path`, migrate it, and make sure a profile is active.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = db::open(path)?;
        profile::seed(&conn)?;

        // Old, already-read journal entries go at startup rather than on a
        // timer: there is no scheduler in this app, and an app that is left open
        // for a fortnight can carry a fortnight of history without complaint.
        // Failing to sweep is not a reason to fail to start.
        if let Ok(Some(profile)) = profile::active(&conn) {
            if let Err(cause) = journal::sweep(&conn, &profile.id) {
                eprintln!("journal: could not sweep old entries: {cause}");
            }
        }

        // Dismissed complaints about works that have since been deleted. They
        // are harmless — nothing raises a complaint about a missing work — but
        // nothing removes them either, since the table deliberately holds no
        // foreign key.
        if let Err(cause) = crate::focus::sweep(&conn) {
            eprintln!("focus: could not sweep dismissals for deleted works: {cause}");
        }

        // Runs the previous life of the application was carrying died with it.
        // Leaving their rows as `running` would show work that nothing is doing.
        if let Err(cause) = crate::assistant::run::sweep(&conn) {
            eprintln!("assistant: could not sweep abandoned runs: {cause}");
        }

        Ok(Self {
            connection: Mutex::new(conn),
            path: path.to_path_buf(),
            runs: Arc::new(Runs::new()),
            queue: Arc::new(Queue::new()),
        })
    }

    /// Borrow the connection.
    ///
    /// A poisoned mutex means another command panicked mid-query. The connection
    /// itself is still usable, so recover rather than take the whole app down.
    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Where assistant runs start: a directory next to the workspace that is
    /// kept empty on purpose.
    ///
    /// A spawned CLI inherits kilna's own working directory otherwise — in
    /// development that is `src-tauri`, and a run asked about "the README"
    /// went and read kilna's sources. Everything a run should know arrives in
    /// the prompt, so what it sees from its directory is pure exposure. This
    /// is a default, not a sandbox — see ADR 0008.
    ///
    /// `None` when the directory cannot be prepared: the run then starts
    /// wherever kilna did, which is worse but not worth refusing the run over.
    pub fn assistant_dir(&self) -> Option<PathBuf> {
        let dir = self.path.parent()?.join("assistant");
        if let Err(cause) = std::fs::create_dir_all(&dir) {
            eprintln!("assistant: could not prepare the run directory: {cause}");
            return None;
        }
        Some(dir)
    }

    pub fn runs(&self) -> &Arc<Runs> {
        &self.runs
    }

    pub fn queue(&self) -> &Arc<Queue> {
        &self.queue
    }

    /// A connection of its own, for work that runs alongside the commands.
    ///
    /// A streamed assistant run writes every few seconds for as long as it
    /// lasts; going through [`Self::conn`] would hold the shared lock across a
    /// process it does not control.
    pub fn open_alongside(&self) -> Option<Connection> {
        match db::open(&self.path) {
            Ok(conn) => Some(conn),
            Err(cause) => {
                eprintln!("assistant: could not open the workspace for a run: {cause}");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_assistant_directory_is_created_next_to_the_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::open(&dir.path().join("workspace.db")).unwrap();

        let assistant = state.assistant_dir().unwrap();

        assert!(assistant.is_dir());
        assert_eq!(assistant.parent(), Some(dir.path()));
    }

    #[test]
    fn the_assistant_directory_survives_being_asked_for_twice() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::open(&dir.path().join("workspace.db")).unwrap();

        assert_eq!(state.assistant_dir(), state.assistant_dir());
    }
}
