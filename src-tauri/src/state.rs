use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;

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

        // Runs the previous life of the application was carrying died with it.
        // Leaving their rows as `running` would show work that nothing is doing.
        if let Err(cause) = crate::assistant::run::sweep(&conn) {
            eprintln!("assistant: could not sweep abandoned runs: {cause}");
        }

        Ok(Self {
            connection: Mutex::new(conn),
            path: path.to_path_buf(),
            runs: Arc::new(Runs::new()),
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

    pub fn runs(&self) -> &Arc<Runs> {
        &self.runs
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
