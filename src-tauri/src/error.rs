use std::sync::LazyLock;

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use serde_json::{Map, Value};

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

    /// Something in the trash cannot be put back yet, because what it belonged
    /// to is itself in the trash. Distinct from a plain database error so the
    /// person is told what to do rather than shown a constraint name.
    #[error("{0}")]
    NotRestorable(String),

    /// The calendar moved between previewing an auto-layout and applying it.
    /// Its own kind because the remedy is specific — preview again — and the
    /// frontend should say that rather than show a refusal with no way out.
    #[error("{0}")]
    LayoutStale(String),

    /// The assistant CLI is missing, unusable, or refused the request.
    #[error("{0}")]
    Assistant(String),

    /// The same task was asked for while it is still running. Its own kind:
    /// it used to travel as `Assistant`, and the window then answered a
    /// second click with "check that Claude Code is installed" - a wrong
    /// diagnosis for a refusal that only means "wait".
    #[error("this is already running; wait for it to finish")]
    AlreadyRunning,

    /// Every slot for a run is taken. Not the CLI's fault either, and it
    /// passes by itself.
    #[error("{0} runs are already going; wait for one to finish, or stop one")]
    Busy(usize),

    /// The row is a snapshot something else points at, so it cannot change
    /// in place — a scored version, whose score read exactly this text. Its
    /// own kind because the remedy is specific: start the next revision.
    #[error("{0}")]
    Frozen(String),

    /// A request the rules of the workspace refuse, named by a code the window
    /// translates. See [`Refusal`] and ADR 0041.
    #[error("{0}")]
    Refused(Refusal),

    /// An assumption inside kilna did not hold: a row that vanished after its
    /// own insert, a log entry missing a field it always carries. Nothing the
    /// person did and nothing they can act on, beyond reporting it - so it is
    /// said once, generically, and the detail goes to the log.
    #[error("{0}")]
    Internal(String),
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
            Self::NotRestorable(_) => "notRestorable",
            Self::LayoutStale(_) => "layoutStale",
            Self::Assistant(_) => "assistant",
            Self::AlreadyRunning => "alreadyRunning",
            Self::Busy(_) => "busy",
            Self::Frozen(_) => "frozen",
            Self::Refused(_) => "refused",
            Self::Internal(_) => "internal",
        }
    }

    /// `no <entity> with id <id>` without hand-writing the sentence each time.
    pub fn not_found(entity: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity,
            id: id.into(),
        }
    }

    /// A refusal under `code`, with no values yet: add them with
    /// [`Error::param`]. The code is a key under `refusal` in the locales.
    pub fn refused(code: &'static str) -> Self {
        Self::Refused(Refusal {
            code,
            params: Map::new(),
        })
    }

    /// A value the refusal's sentence interpolates. On anything but a
    /// refusal it does nothing, so a chain never has to check what it holds.
    #[must_use]
    pub fn param(mut self, key: &str, value: impl Into<Value>) -> Self {
        if let Self::Refused(refusal) = &mut self {
            refusal.params.insert(key.to_owned(), value.into());
        }
        self
    }

    /// The refusal this is, when it is one.
    pub fn refusal(&self) -> Option<&Refusal> {
        match self {
            Self::Refused(refusal) => Some(refusal),
            _ => None,
        }
    }

    /// This failure as a reason: why one item of a batch was passed over, or
    /// one problem among several. A refusal keeps its code and values; any
    /// other failure is said with the window's generic sentence for its kind.
    pub fn reason(&self) -> Reason {
        match self {
            Self::Refused(refusal) => Reason {
                key: format!("refusal.{}", refusal.code),
                params: refusal.params.clone(),
            },
            other => Reason {
                key: format!("error.{}", other.kind()),
                params: Map::new(),
            },
        }
    }
}

/// What the rules refused, as a key and the values its sentence needs.
///
/// Stored the way the journal stores what happened (ADR 0007): the backend
/// never writes the sentence a person reads, because the person may be
/// reading Russian. The window looks `refusal.<code>` up in its locale and
/// interpolates `params`. The English sentence - for the log, for tests, and
/// for an agent reading `kilna --mcp` - comes from the same locale file, so
/// there is one English wording, not two that drift.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub code: &'static str,
    pub params: Map<String, Value>,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&english(self.code, &self.params))
    }
}

/// A reason, said by the window: a key of its locale and the values the
/// sentence needs - `refusal.<code>` for a refusal, `error.<kind>` for any
/// other failure, `skip.<why>` for an item a batch passed over without anything
/// going wrong ("already at that status").
///
/// A reason can be a value of another: a list of them is a list of problems,
/// and both the window and [`english`] say each in turn.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Reason {
    pub key: String,
    pub params: Map<String, Value>,
}

impl Reason {
    /// A reason by its locale key.
    pub fn of(key: &str) -> Self {
        Self {
            key: key.to_owned(),
            params: Map::new(),
        }
    }

    #[must_use]
    pub fn param(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.params.insert(key.to_owned(), value.into());
        self
    }
}

impl From<Reason> for Value {
    fn from(reason: Reason) -> Self {
        serde_json::json!({ "key": reason.key, "params": reason.params })
    }
}

/// The English sentences of every refusal, read once from the window's own
/// locale.
static ENGLISH: LazyLock<Map<String, Value>> = LazyLock::new(|| {
    let locale: Value =
        serde_json::from_str(include_str!("../../src/i18n/locales/en.json")).unwrap_or_default();
    let mut sentences = Map::new();
    flatten(&locale, "", &mut sentences);
    sentences
});

fn flatten(value: &Value, prefix: &str, out: &mut Map<String, Value>) {
    if let Value::Object(object) = value {
        for (key, child) in object {
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            flatten(child, &path, out);
        }
    } else {
        out.insert(prefix.to_owned(), value.clone());
    }
}

/// A refusal said in English: the locale's sentence with its values put in,
/// or the code and values when the locale has no sentence for it.
pub fn english(code: &str, params: &Map<String, Value>) -> String {
    said(&format!("refusal.{code}"), params).unwrap_or_else(|| {
        if params.is_empty() {
            code.to_owned()
        } else {
            format!("{code} {}", Value::Object(params.clone()))
        }
    })
}

/// A locale key said in English with its values put in, when the locale has it.
fn said(key: &str, params: &Map<String, Value>) -> Option<String> {
    let Some(Value::String(sentence)) = ENGLISH.get(key) else {
        return None;
    };
    let mut out = sentence.clone();
    for (name, value) in params {
        out = out.replace(&format!("{{{{{name}}}}}"), &say_value(value));
    }
    Some(out)
}

/// One interpolated value, in English.
fn say_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        // A reason inside a reason: said in its own words.
        Value::Object(map) if map.get("key").is_some_and(Value::is_string) => {
            let key = map.get("key").and_then(Value::as_str).unwrap_or_default();
            let params = map
                .get("params")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            said(key, &params).unwrap_or_else(|| key.to_owned())
        }
        // A word of the profile carried in two languages.
        Value::Object(map) => map
            .get("en")
            .and_then(Value::as_str)
            .map_or_else(|| value.to_string(), str::to_owned),
        // A list of reasons, one after another.
        Value::Array(items) => items.iter().map(say_value).collect::<Vec<_>>().join("; "),
        other => other.to_string(),
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// Tauri commands must return something serialisable. The frontend gets the
// tag and the message, and for a refusal its code and values: it says a
// refusal in its own language from those, and keeps the message as the
// detail when it has nothing better to say.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let refusal = self.refusal();
        let mut error =
            serializer.serialize_struct("Error", if refusal.is_some() { 4 } else { 2 })?;
        error.serialize_field("kind", self.kind())?;
        error.serialize_field("message", &self.to_string())?;
        if let Some(refusal) = refusal {
            error.serialize_field("code", refusal.code)?;
            error.serialize_field("params", &refusal.params)?;
        }
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
        assert!(json.get("code").is_none(), "only a refusal carries a code");
    }

    #[test]
    fn a_refusal_travels_as_its_code_and_values_and_reads_in_english() {
        let error = Error::refused("work.unknownKind")
            .param("kind", "opera")
            .param("known", "song, video");
        let json = serde_json::to_value(&error).expect("serialises");

        assert_eq!(json["kind"], "refused");
        assert_eq!(json["code"], "work.unknownKind");
        assert_eq!(json["params"]["kind"], "opera");
        let message = json["message"].as_str().unwrap();
        assert!(
            message.contains("opera") && !message.contains("{{"),
            "the English sentence has its value put in: {message}"
        );
    }

    #[test]
    fn a_code_the_locale_does_not_know_still_says_something() {
        let error = Error::refused("nowhere.atAll").param("n", 3);
        assert_eq!(error.to_string(), r#"nowhere.atAll {"n":3}"#);
    }

    #[test]
    fn a_word_of_the_profile_is_said_in_english() {
        let said = english(
            "work.unknownKind",
            serde_json::json!({ "kind": { "en": "Song", "ru": "Песня" } })
                .as_object()
                .unwrap(),
        );
        assert!(said.contains("Song") && !said.contains("Песня"), "{said}");
    }

    #[test]
    fn every_variant_has_a_distinct_kind() {
        let kinds = [
            Error::Internal(String::new()).kind(),
            Error::refused("x").kind(),
            Error::Assistant(String::new()).kind(),
            Error::Frozen(String::new()).kind(),
            Error::NotRestorable(String::new()).kind(),
            Error::LayoutStale(String::new()).kind(),
            Error::AlreadyRunning.kind(),
            Error::Busy(1).kind(),
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
