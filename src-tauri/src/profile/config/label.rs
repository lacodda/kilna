//! A word in every language the window speaks, and a kind's key with its label.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A word of the craft's vocabulary, in the languages the profile carries it in.
///
/// A profile's words are the author's data, not the interface's strings: they
/// are edited in Settings and they survive an upgrade untouched. But a
/// *shipped* profile is written by us, and writing it in English only meant a
/// Russian window read "Scored · Song" — the interface translated around a
/// hole in its own vocabulary.
///
/// So a label is either one string, as every profile written before this said
/// it, or a map from locale to string. Both shapes parse and both round-trip:
/// a word the author retyped comes back as the plain string they typed, and is
/// never silently promoted to a map on their behalf.
///
/// Rust resolves to English and only English. The window picks the language —
/// it is the only side that knows which one is showing — while this side uses
/// labels for prompts and exports, where English is what we want anyway: a
/// system prompt is written against a model's English, not the author's
/// window, and translating one changes what the model does.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[serde(untagged)]
pub enum Label {
    /// One word, in whatever language it was typed in.
    One(String),
    /// The word per locale, as a shipped profile carries it.
    PerLocale(BTreeMap<String, String>),
}

/// The locale a shipped profile is authored in, and what Rust reads a label as.
pub const SOURCE_LOCALE: &str = crate::error::SOURCE_LOCALE;

impl Label {
    /// The word in the source language: what a prompt and an export say.
    ///
    /// A map that somehow lacks English falls back to any word it does hold
    /// rather than to emptiness — a label is a name, and a nameless entry in
    /// a prompt is worse than one named in the wrong language.
    pub fn as_str(&self) -> &str {
        match self {
            Self::One(word) => word,
            Self::PerLocale(words) => words
                .get(SOURCE_LOCALE)
                .or_else(|| words.values().next())
                .map_or("", String::as_str),
        }
    }

    /// The word in `locale`, or in the source language when the label does
    /// not carry that one. For the few words the application writes into a
    /// person's own data - the title of a work made from another - where the
    /// window says which language it is showing.
    pub fn in_locale(&self, locale: &str) -> &str {
        match self {
            Self::One(word) => word,
            Self::PerLocale(words) => words
                .get(locale)
                .map_or_else(|| self.as_str(), String::as_str),
        }
    }

    /// Every word the label carries, whatever the language.
    pub fn words(&self) -> Vec<&str> {
        match self {
            Self::One(word) => vec![word.as_str()],
            Self::PerLocale(words) => words.values().map(String::as_str).collect(),
        }
    }

    /// Whether the label says nothing at all — what validation rejects.
    pub fn is_blank(&self) -> bool {
        match self {
            Self::One(word) => word.trim().is_empty(),
            Self::PerLocale(words) => {
                words.is_empty() || words.values().all(|word| word.trim().is_empty())
            }
        }
    }
}

/// Comparison against a plain string, so a test and a lookup can both ask
/// "is this the word?" without unwrapping the shape first. Compares the
/// source-language word, which is the one Rust works in.
impl PartialEq<str> for Label {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Label {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for Label {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl std::fmt::Display for Label {
    /// Through `f.pad`, so `{:<12}` lines a column up rather than being
    /// silently ignored the way `write_str` would.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(self.as_str())
    }
}

impl From<&str> for Label {
    fn from(word: &str) -> Self {
        Self::One(word.to_owned())
    }
}

impl From<String> for Label {
    fn from(word: String) -> Self {
        Self::One(word)
    }
}

/// A vocabulary entry: a stable key with a label the user may rename.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Kind {
    pub key: String,
    pub label: Label,
}

impl Kind {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
        }
    }
}
