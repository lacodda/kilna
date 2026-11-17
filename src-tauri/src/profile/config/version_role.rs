//! The roles a version plays in a work, and how each body is read.

use serde::{Deserialize, Serialize};

use super::*;

/// An independent body a work carries.
///
/// Most roles stand alone — lyrics and style advance separately, and showing
/// them interleaved would suggest otherwise. A role that names `comments_on`
/// does not: it is written *about* another role, and reading it away from what
/// it discusses is reading half of it. That is the whole reason the field
/// exists rather than the code knowing which keys are commentary: the craft
/// says what comments on what, the same way it says everything else (ADR 0001).
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct VersionRole {
    pub key: String,
    pub label: Label,
    /// The role this one discusses, if any. A key no role defines is ignored,
    /// which keeps a half-edited profile from breaking the card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comments_on: Option<String>,
    /// How a body in this role reads: `plain` (a monospace column, exactly as
    /// typed — lyrics, a style prompt) or `markdown` (headings, quotes and
    /// tables drawn — a review, a chapter). The craft says, because the code
    /// cannot tell a lyric sheet from an essay by looking at it (ADR 0001).
    /// Absent reads as plain, which is what every role was before the field
    /// existed; a value outside the two reads as plain too, so a typo costs
    /// headings rather than a screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Whether a body in this role is a time the WORK was written, and so
    /// belongs in the count the catalogue shows.
    ///
    /// A critique answers this by commenting on another role, and always did.
    /// A style prompt does not: it stands on its own, is written about the
    /// song rather than being a draft of it, and counting it made a song with
    /// four texts read six. The craft says which of its roles are the work
    /// itself, because the code cannot tell a lyric sheet from a production
    /// note by looking at it — the same reason `body` exists (ADR 0001).
    ///
    /// Absent reads as "yes, unless it comments on something", which is what
    /// every role meant before the field existed: a profile that never sets
    /// it counts exactly what it counted before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counts_as_version: Option<bool>,
    /// Whether a body in this role is sung (v0.90, ADR 0053): its words are
    /// checked for where the stress falls and how the owner sings them, the
    /// stress is marked for the singer with a capital vowel, and a public
    /// text made from it has the marks taken off. A lyric is; a style
    /// prompt, a plot, a review are not. Absent is not sung.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sung: bool,
    /// The text a version written from nothing in this role starts from
    /// (v0.91): a lyric's parts in the markers its generator reads, a
    /// chapter's plan and draft. The craft knows how its texts are laid out;
    /// the code does not (ADR 0001). A word or a map per locale, like a
    /// label - a lyric's markers stay in the language the generator reads,
    /// whatever the window says. Absent, a new version starts empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skeleton: Option<Label>,
}

/// The two ways a body is read. See [`VersionRole::body`].
pub const BODY_KINDS: [&str; 2] = ["plain", "markdown"];

impl VersionRole {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            comments_on: None,
            body: None,
            counts_as_version: None,
            sung: false,
            skeleton: None,
        }
    }

    /// Whether bodies in this role are drawn as markdown.
    pub fn reads_as_markdown(&self) -> bool {
        self.body.as_deref() == Some("markdown")
    }

    /// Whether a body in this role counts as a time the work was written.
    ///
    /// The profile's explicit answer wins; without one, anything that
    /// comments on another role is not a draft and everything else is.
    pub fn counts_as_a_version(&self) -> bool {
        self.counts_as_version
            .unwrap_or_else(|| self.comments_on.is_none())
    }
}
