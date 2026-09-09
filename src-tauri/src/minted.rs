//! What carrying an operation out generates, and where a replay puts it back.
//!
//! A domain function that mints its own uuid and stamps its own `now()` cannot
//! be replayed: run it again and it produces a different id, and everything
//! that named the original — versions, scores, releases — parts ways with the
//! row it means. See ADR 0014.
//!
//! So the two generated values travel in this one type. Live, it mints them;
//! replaying, it hands back what the log recorded. Nothing else changes: the
//! function does the same work either way, and there is no branch inside it
//! saying which mode it is in.

use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::time::now;

/// An id and a timestamp — minted now, or read back from the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Minted {
    id: String,
    at: String,
}

impl Minted {
    /// Fresh values, for an operation happening for the first time.
    pub fn fresh() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            at: now(),
        }
    }

    /// The values an earlier run generated, taken from a logged operation.
    ///
    /// A missing value is an error, not a fresh one. Minting a replacement
    /// would rebuild the row under an id nothing else names, and say nothing:
    /// the log would look replayed and the database would be quietly wrong,
    /// which is the exact failure this whole scheme is against. Better to
    /// refuse and name the operation.
    pub fn from_params(params: &Map<String, Value>) -> Result<Self> {
        let string = |key: &str| {
            params
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| {
                    Error::Other(format!(
                        "the operation carries no `{key}`, so replaying it would invent one"
                    ))
                })
        };
        Ok(Self {
            id: string("id")?,
            at: string("at")?,
        })
    }

    /// Exactly these values — for a caller that has both already.
    pub fn of(id: impl Into<String>, at: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            at: at.into(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn at(&self) -> &str {
        &self.at
    }

    /// Write both into an operation's params, under the names
    /// [`Minted::from_params`] reads.
    pub fn into_params(self, params: &mut Map<String, Value>) {
        params.insert("id".into(), Value::String(self.id));
        params.insert("at".into(), Value::String(self.at));
    }
}

impl Default for Minted {
    fn default() -> Self {
        Self::fresh()
    }
}
