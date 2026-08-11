use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::db;
use crate::error::Result;
use crate::profile;

/// The open workspace, shared by every command.
///
/// A single connection behind a mutex is enough here: this is a desktop
/// application with one user, and SQLite writes serialise anyway.
pub struct AppState {
    connection: Mutex<Connection>,
    path: PathBuf,
}

impl AppState {
    /// Open the workspace at `path`, migrate it, and make sure a profile is active.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = db::open(path)?;
        profile::seed(&conn)?;

        Ok(Self {
            connection: Mutex::new(conn),
            path: path.to_path_buf(),
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
}
