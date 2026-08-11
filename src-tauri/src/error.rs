use serde::{Serialize, Serializer};

/// Every failure that can reach the frontend.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("could not serialise data: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("could not access the workspace directory: {0}")]
    Io(#[from] std::io::Error),

    /// The database was created by a newer build of kilna.
    #[error(
        "this workspace was written by a newer version of kilna (schema {found}, this build understands {supported})"
    )]
    SchemaTooNew { found: i64, supported: i64 },

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

// Tauri commands must return something serialisable; the frontend gets the message.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
