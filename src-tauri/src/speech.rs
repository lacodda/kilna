//! The language something is written out in for a person to read.
//!
//! Nearly everything the backend says travels as a key and is said by the
//! window (ADR 0041). A few things are whole texts built here - the body a
//! proposal is shown with in a chat, a table with a heading over each column -
//! and those are built in a language: English when the text is stored, sent to
//! an agent or logged, and the window's own when the window asks to read it
//! (v0.112, ADR 0080). The words come from the window's locales either way, so
//! there is one wording per language and it is the locale's.

use serde_json::{Map, Value};

use crate::error::{SOURCE_LOCALE, said_in};
use crate::profile::config::Label;

/// A language to write in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Speech<'a> {
    locale: &'a str,
}

impl Speech<'static> {
    /// English: what is stored, logged, and read by an agent.
    pub const ENGLISH: Self = Self {
        locale: SOURCE_LOCALE,
    };
}

impl<'a> Speech<'a> {
    /// The language a window speaks, as it names it: `ru`, `en-GB`.
    pub fn of(locale: &'a str) -> Self {
        Self { locale }
    }

    /// Whether this is the language texts are stored in.
    pub fn is_source(&self) -> bool {
        self.locale.split(['-', '_']).next() == Some(SOURCE_LOCALE)
    }

    /// A sentence of the locale by its key. A key no locale has is said as
    /// the key: a hole that names itself, in a text a person reads, rather
    /// than a blank.
    pub fn say(&self, key: &str) -> String {
        self.say_with(key, &[])
    }

    /// A sentence with its values put in.
    pub fn say_with(&self, key: &str, values: &[(&str, Value)]) -> String {
        let params: Map<String, Value> = values
            .iter()
            .map(|(name, value)| ((*name).to_owned(), value.clone()))
            .collect();
        said_in(self.locale, key, &params).unwrap_or_else(|| key.to_owned())
    }

    /// A word of the profile, in this language.
    pub fn word<'l>(&self, label: &'l Label) -> &'l str {
        label.in_locale(self.locale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_sentence_is_said_in_the_language_asked_and_in_english_otherwise() {
        let english = Speech::ENGLISH.say("proposalBody.fields");
        let russian = Speech::of("ru").say("proposalBody.fields");
        assert_eq!(english, "Fields");
        assert_ne!(russian, english);
        assert_eq!(Speech::of("ru-RU").say("proposalBody.fields"), russian);
        // A language the window does not speak reads English.
        assert_eq!(Speech::of("de").say("proposalBody.fields"), english);
        assert!(Speech::of("en-GB").is_source());
        assert!(!Speech::of("ru").is_source());
    }

    #[test]
    fn values_are_put_in_and_a_key_nobody_has_names_itself() {
        let said = Speech::ENGLISH.say_with("proposalBody.scene", &[("n", json!("3"))]);
        assert_eq!(said, "Scene 3");
        assert_eq!(Speech::of("ru").say("nowhere.atAll"), "nowhere.atAll");
    }

    #[test]
    fn a_word_of_the_profile_is_said_in_this_language() {
        let label: Label = serde_json::from_value(json!({ "en": "Song", "ru": "Песня" })).unwrap();
        assert_eq!(Speech::of("ru").word(&label), "Песня");
        assert_eq!(Speech::ENGLISH.word(&label), "Song");
        assert_eq!(Speech::of("ru").word(&Label::from("Track")), "Track");
    }
}
