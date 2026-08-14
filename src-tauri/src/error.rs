use serde::ser::SerializeStruct;
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

    /// Something was addressed by id that is not there — usually because it was
    /// deleted in another view while this one still held the id.
    #[error("no {entity} with id `{id}`")]
    NotFound { entity: &'static str, id: String },

    /// The assistant CLI is missing, unusable, or refused the request.
    #[error("{0}")]
    Assistant(String),

    #[error("{0}")]
    Other(String),
}

impl Error {
    /// A stable tag for the frontend to branch on.
    ///
    /// The message is written for a human and may be reworded at any time; this
    /// is the part code is allowed to match against.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Database(_) => "database",
            Self::Serde(_) => "serde",
            Self::Io(_) => "io",
            Self::SchemaTooNew { .. } => "schemaTooNew",
            Self::NotFound { .. } => "notFound",
            Self::Assistant(_) => "assistant",
            Self::Other(_) => "other",
        }
    }

    /// `no <entity> with id <id>` without hand-writing the sentence each time.
    pub fn not_found(entity: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity,
            id: id.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// Tauri commands must return something serialisable. The frontend gets both the
// tag and the message: it maps the tag to a sentence it can translate, and keeps
// the message as the detail when it has nothing better to say.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut error = serializer.serialize_struct("Error", 2)?;
        error.serialize_field("kind", self.kind())?;
        error.serialize_field("message", &self.to_string())?;
        error.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_serialises_with_its_kind_and_message() {
        let error = Error::not_found("work", "w1");
        let json = serde_json::to_value(&error).expect("serialises");

        assert_eq!(json["kind"], "notFound");
        assert_eq!(json["message"], "no work with id `w1`");
    }

    #[test]
    fn every_variant_has_a_distinct_kind() {
        let kinds = [
            Error::Other(String::new()).kind(),
            Error::Assistant(String::new()).kind(),
            Error::not_found("work", "w1").kind(),
            Error::SchemaTooNew {
                found: 2,
                supported: 1,
            }
            .kind(),
        ];

        let unique: std::collections::HashSet<_> = kinds.iter().collect();
        assert_eq!(
            unique.len(),
            kinds.len(),
            "kinds must not collide: {kinds:?}"
        );
    }
}
