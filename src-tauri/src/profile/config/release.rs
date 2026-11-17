//! Kinds of release and the fields each goes out with.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::*;

/// A kind of release, and what a release of it cannot ship without.
///
/// `requires` names version roles: a clip needs lyrics and a style prompt, a
/// beta read needs the text. An empty list means the kind states no
/// requirements — readiness is then judged on the universal facts alone, and
/// every role mark reads as "not applicable" rather than "missing". A profile
/// written before this field existed loads that way.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct ReleaseKind {
    pub key: String,
    pub label: Label,
    #[serde(default)]
    pub requires: Vec<String>,
    /// The glyph the calendar draws this kind with, named from a fixed set the
    /// frontend knows. The profile names it because the code is not allowed to
    /// know which kinds exist (ADR 0001) -- a clip and a beta read have nothing
    /// in common but the shape of the row they sit in. Absent, or naming a
    /// glyph the set does not hold, falls back to a neutral one: a profile
    /// written before this field existed still loads, and a typo costs an
    /// icon rather than a screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Axis weights that apply when a work is judged *for this kind* of
    /// release, keyed by axis key. A clip lives or dies on its hook and its
    /// visuals; the same song as an audio release is carried by its lyrics.
    /// An axis not named here keeps the weight the axis itself declares, so a
    /// kind may reweight one axis and say nothing about the rest. Empty — the
    /// state of every profile written before the field — means the axes'
    /// own weights, and one tier for every kind.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub axis_weights: BTreeMap<String, f64>,
    /// What has to be written *about* a release of this kind before it can go
    /// out: a title, a description, tags, a comment to pin under it. The
    /// craft's own list, because the fields a video ships with and the fields
    /// a paperback ships with have nothing in common but being text someone
    /// has to write.
    ///
    /// Each field may carry a template, so the writing starts from the work
    /// rather than from an empty box. Empty — the state of every profile
    /// written before the field existed — means a release of this kind says
    /// nothing about itself, and the tab shows no metadata for it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ReleaseField>,
    /// The shape of the picture a release of this kind goes out with, as
    /// width to height: `16:9` for a video platform's preview, `9:16` for a
    /// vertical short, `1:1` for a streaming cover. The place decides the
    /// shape, so the shape is the door's - a work that goes out in two
    /// places needs two covers (v0.86). Absent means the door shows no
    /// picture of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_format: Option<String>,
    /// Whether a release of this kind cannot go out without a cover chosen
    /// for its work (v0.97). Not read off `cover_format`: a shape says what
    /// a picture would be, not that the place refuses to go without one - a
    /// short takes a frame of itself when nobody gives it a picture. Absent
    /// is no such requirement. `false` is kept as written rather than dropped:
    /// it is how a requirement the owner took off a shipped door stays off
    /// when the upgrade carries the shipped one forward.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_cover: Option<bool>,
    /// What an address of this place looks like, so a link pasted when a
    /// release is marked out can say where it went (v0.97): a host, and the
    /// start of a path when the host alone is shared - `youtube.com/shorts/`,
    /// `youtu.be`, `open.spotify.com`. A host matches itself and its
    /// subdomains; the most specific pattern of the kind's doors wins, and a
    /// tie says nothing rather than guess. Absent is a place no link names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<String>,
}

/// Whether `pattern` is a link pattern a door can name: a host with a dot
/// in it, optionally followed by the start of a path - no scheme, no query,
/// no spaces, nothing a link could not hold.
pub fn is_link_pattern(pattern: &str) -> bool {
    let (host, path) = pattern.split_once('/').unwrap_or((pattern, ""));
    let host_fits = host.contains('.')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'));
    let path_fits = path
        .chars()
        .all(|c| c.is_ascii_graphic() && !matches!(c, '?' | '#'));
    host_fits && path_fits
}

/// Whether `format` is a shape a cover can take: two whole numbers above
/// zero with a colon between them, `16:9`.
pub fn is_cover_format(format: &str) -> bool {
    let Some((width, height)) = format.split_once(':') else {
        return false;
    };
    let whole = |side: &str| {
        !side.is_empty()
            && side.bytes().all(|byte| byte.is_ascii_digit())
            && side.parse::<u32>().is_ok_and(|value| value > 0)
    };
    whole(width) && whole(height)
}

/// One thing written about a release: the box it is typed in, and the
/// template it starts from.
///
/// The key is what the value is stored under in `release.meta`, so renaming
/// the label never loses what was written. The template is in the same
/// language as an AI action's prompt — `{title}`, `{role:lyrics}`, `{scenes}`
/// — because a placeholder that meant one thing in one box and another thing
/// in the next box would be two vocabularies wearing one syntax. See
/// [`crate::assistant::prompt`].
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct ReleaseField {
    pub key: String,
    pub label: Label,
    /// The shape of the box. Absent is a line: the state a field written
    /// before the type existed reads as.
    #[serde(default, rename = "type")]
    pub field_type: ReleaseFieldType,
    /// What the field is filled with when it is generated, in the prompt
    /// language. Absent means the field is only ever typed by hand — which is
    /// the honest answer for a link, and a bad one for a description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// A line under the box saying what goes in it — the platform's limit,
    /// the house style. The same role `hint` plays on a scene block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<Label>,
    /// How many characters the place this is going will accept. Absent means
    /// nobody is counting. Shown as a count beside the box rather than
    /// enforced: kilna is not the authority on what YouTube accepts this
    /// month, and a field it refuses to hold is a field typed somewhere else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// A tail the value always ends with, whoever writes the rest: an audio
    /// track's title on a video platform ends with " (audio)" when it is
    /// typed, generated or proposed, and a clip's has none (v0.90). Kept by
    /// the release itself (`release_meta::keep_suffixes`), so a title written
    /// by hand cannot lose it. Absent is no tail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// The tail instead of `suffix` when the release opens its song - the
    /// first of what was made from it to go out (v0.100.1): the audio track a
    /// song premieres with is "Song (Song premiere 2026)", the one that follows
    /// its clip keeps " (audio)". `{year}` reads the year it goes out. Kept by
    /// the release as `suffix` is, in the language of the title it ends: a
    /// title in Cyrillic letters takes the Russian word when there is one.
    /// Absent is the plain tail always. See ADR 0067.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premiere: Option<Label>,
    /// Whether a release cannot go out with this field empty (v0.97): a
    /// video without a title, a newsletter without a subject. Kept on the
    /// field rather than in a list on the kind, so a field taken out of the
    /// profile takes its requirement with it. Absent is optional; `false` is
    /// kept as written, so a field the owner made optional stays optional when
    /// the upgrade carries a shipped requirement forward.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// The shape of a release field's box.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseFieldType {
    /// One line: a title.
    #[default]
    Line,
    /// Paragraphs: a description, a comment to pin.
    Text,
    /// A list of words, kept as text with a comma between them — the form
    /// every platform's box takes, and the form that survives a copy.
    Tags,
}

impl ReleaseField {
    pub fn new(key: &str, label: &str, field_type: ReleaseFieldType) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            field_type,
            template: None,
            hint: None,
            limit: None,
            suffix: None,
            premiere: None,
            required: None,
        }
    }

    /// Whether a release cannot go out with this field empty.
    pub fn is_required(&self) -> bool {
        self.required == Some(true)
    }

    /// Whether `value`, as stored under this field, says anything: blank is
    /// nothing, and so is the field's own tail standing alone - " (audio)"
    /// is not a title anyone wrote.
    pub fn is_filled(&self, value: Option<&serde_json::Value>) -> bool {
        let Some(text) = value.and_then(serde_json::Value::as_str) else {
            return false;
        };
        let body = self.without_tail(text.trim());
        // A list of words is filled when one of them is a word.
        match self.field_type {
            ReleaseFieldType::Tags => body.split(',').any(|word| !word.trim().is_empty()),
            _ => !body.trim().is_empty(),
        }
    }

    /// The tail the field keeps, when it names one worth keeping: blank is
    /// none.
    pub fn suffix(&self) -> Option<&str> {
        self.suffix
            .as_deref()
            .filter(|tail| !tail.trim().is_empty())
    }

    /// `value` ending with the field's tail: added where the value is written
    /// and does not end with it yet, never to an empty value - a title nobody
    /// wrote is not " (audio)". Within the limit, the tail is what stays: the
    /// words before it are cut to make room.
    pub fn with_suffix(&self, value: &str) -> String {
        match self.suffix() {
            Some(tail) => self.with_tail(value, tail),
            None => value.to_owned(),
        }
    }

    /// Whether the field keeps any tail at all: the plain one or a
    /// premiere's.
    pub fn keeps_a_tail(&self) -> bool {
        self.suffix().is_some() || self.premiere.as_ref().is_some_and(|tail| !tail.is_blank())
    }

    /// The tail a premiere in `year` ends `title` with, in the title's own
    /// language. None when the field names no premiere tail.
    pub fn premiere_tail(&self, year: i32, title: &str) -> Option<String> {
        let tail = self.premiere.as_ref().filter(|tail| !tail.is_blank())?;
        let locale = if title.chars().any(is_cyrillic) {
            "ru"
        } else {
            SOURCE_LOCALE
        };
        Some(tail.in_locale(locale).replace("{year}", &year.to_string()))
    }

    /// `text` without the tail it ends with, when it ends with one this field
    /// keeps: the plain tail, or a premiere's in any of its languages and
    /// any year - what a release moving to another year, another door or
    /// out of its premiere has to take off before it takes the right one.
    pub fn without_tail<'t>(&self, text: &'t str) -> &'t str {
        if let Some(tail) = self.suffix()
            && let Some(body) = text.strip_suffix(tail.trim_start())
        {
            return body.trim_end();
        }
        if let Some(premiere) = &self.premiere {
            for word in premiere.words() {
                if let Some(body) = strip_dated(text, word) {
                    return body.trim_end();
                }
            }
        }
        text
    }

    /// `value` ending with `tail`, on the terms of [`with_suffix`]: never on
    /// an empty value, and within the limit the tail is what stays.
    ///
    /// [`with_suffix`]: Self::with_suffix
    pub fn with_tail(&self, value: &str, tail: &str) -> String {
        let body = value.trim_end();
        if body.trim().is_empty() {
            return value.to_owned();
        }
        if body.ends_with(tail) {
            return body.to_owned();
        }
        let mut body = body.to_owned();
        if let Some(limit) = self.limit {
            let room = (limit as usize).saturating_sub(tail.chars().count());
            if body.chars().count() > room {
                body = body
                    .chars()
                    .take(room)
                    .collect::<String>()
                    .trim_end()
                    .to_owned();
            }
        }
        format!("{body}{tail}")
    }

    /// The same field, filled from a template.
    pub fn from_template(mut self, template: &str) -> Self {
        self.template = Some(template.to_owned());
        self
    }

    /// The template, when the field states one worth rendering: blank is none.
    pub fn template(&self) -> Option<&str> {
        self.template
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
    }
}

/// `text` without a tail written as `word` with any year in its `{year}`:
/// " (Song premiere {year})" comes off "Tide (Song premiere 2026)" as it
/// would off "Tide (Song premiere 2027)".
pub(super) fn strip_dated<'t>(text: &'t str, word: &str) -> Option<&'t str> {
    let Some((before, after)) = word.split_once("{year}") else {
        return text.strip_suffix(word.trim_start());
    };
    let rest = text.strip_suffix(after)?;
    let digits = rest.len() - rest.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 {
        return None;
    }
    rest[..rest.len() - digits].strip_suffix(before.trim_start())
}

pub(super) fn is_cyrillic(c: char) -> bool {
    matches!(c, '\u{0400}'..='\u{04FF}' | '\u{0500}'..='\u{052F}')
}

impl ReleaseKind {
    pub fn new(key: &str, label: &str, requires: &[&str]) -> Self {
        Self {
            key: key.to_owned(),
            label: Label::from(label),
            requires: requires.iter().map(|role| (*role).to_owned()).collect(),
            icon: None,
            axis_weights: BTreeMap::new(),
            fields: Vec::new(),
            cover_format: None,
            requires_cover: None,
            links: Vec::new(),
        }
    }

    /// Whether a release of this kind cannot go out without a cover.
    pub fn needs_cover(&self) -> bool {
        self.requires_cover == Some(true)
    }

    /// The same kind, drawn with a named glyph.
    pub fn with_icon(mut self, icon: &str) -> Self {
        self.icon = Some(icon.to_owned());
        self
    }
}
