//! The three lines a work's feel is said along (FR-02).

use serde::{Deserialize, Serialize};

use super::*;

/// The three lines a work's feel is said along (FR-02), and the words on
/// each. The mark drawn for a work is built from one of each: the colour of
/// its mood, the shape of its character, as many strokes as it is complex.
///
/// In the profile rather than in the window, like the stops: the lines are
/// the same for every craft - five colours, five shapes, three weights - and
/// what they are called is the craft's. A blog calls the third "how deep it
/// goes", a novel "how many layers the plot has".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct VibeAxes {
    pub moods: Vec<VibeMood>,
    pub characters: Vec<VibeCharacter>,
    pub complexities: Vec<VibeComplexity>,
    /// What each line is called, where the craft has its own word for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub labels: Option<VibeLabels>,
    /// The value a line stands at while it is unsaid and another is said -
    /// a mark is built of all three. The middle of each line unless named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub neutral: Option<crate::work::Vibe>,
}

/// A mood: a word, and the colour a mark of it is drawn in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct VibeMood {
    pub key: String,
    pub label: Label,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub hint: Option<Label>,
    /// `#RRGGBB`, as it reads on the dark theme; the window deepens it for
    /// the light one.
    pub color: String,
}

/// A character: a word, and the shape a mark of it takes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct VibeCharacter {
    pub key: String,
    pub label: Label,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub hint: Option<Label>,
    pub glyph: VibeGlyph,
}

/// The five shapes the window draws a character as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum VibeGlyph {
    /// A level wave.
    Calm,
    /// A diamond.
    Playful,
    /// A circle: even all round.
    Steady,
    /// A broken, pointed line.
    Edgy,
    /// A chevron, pointing on.
    Drive,
}

/// How complex: a word, and how many strokes the mark is drawn with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct VibeComplexity {
    pub key: String,
    pub label: Label,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub hint: Option<Label>,
    /// One, two or three.
    pub strokes: u8,
}

/// What the three lines are called, where a craft says so.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct VibeLabels {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub mood: Option<Label>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub character: Option<Label>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub complexity: Option<Label>,
}

impl VibeAxes {
    /// Whether every key a vibe names is one of this profile's. A mood the
    /// profile does not have is a mark nobody could draw, so it is refused
    /// with the ones there are.
    pub fn check(&self, vibe: &crate::work::Vibe) -> crate::error::Result<()> {
        let lines: [(&str, &Option<String>, Vec<&str>); 3] = [
            (
                "mood",
                &vibe.mood,
                self.moods.iter().map(|m| m.key.as_str()).collect(),
            ),
            (
                "character",
                &vibe.character,
                self.characters.iter().map(|c| c.key.as_str()).collect(),
            ),
            (
                "complexity",
                &vibe.complexity,
                self.complexities.iter().map(|c| c.key.as_str()).collect(),
            ),
        ];
        for (line, said, known) in lines {
            if let Some(key) = said
                && !known.contains(&key.as_str())
            {
                return Err(crate::error::Error::refused("work.unknownVibe")
                    .param("line", line)
                    .param("key", key.clone())
                    .param("known", known.join(", ")));
            }
        }
        Ok(())
    }

    /// The value each line stands at while it is unsaid: the one the profile
    /// names, or the middle of the line.
    pub fn neutral(&self) -> crate::work::Vibe {
        let named = self.neutral.clone().unwrap_or_default();
        let middle = |keys: Vec<&String>| keys.get(keys.len() / 2).map(|key| (*key).clone());
        crate::work::Vibe {
            mood: named
                .mood
                .or_else(|| middle(self.moods.iter().map(|m| &m.key).collect())),
            character: named
                .character
                .or_else(|| middle(self.characters.iter().map(|c| &c.key).collect())),
            complexity: named
                .complexity
                .or_else(|| middle(self.complexities.iter().map(|c| &c.key).collect())),
        }
    }
}

/// The lines as the line itself words them: what a craft gets when it names
/// none, and what Studio ships (FR-02).
///
/// The five colours are told apart from one another and from the five
/// colours of the stage dial, which stands beside a mark in every row: the
/// joyful one is a coral, since a lime sat right against the green of a
/// finished stage.
pub fn default_vibe() -> VibeAxes {
    let label = |en: &str, ru: &str| {
        Label::PerLocale(
            [("en", en), ("ru", ru)]
                .into_iter()
                .map(|(language, word)| (language.to_owned(), word.to_owned()))
                .collect(),
        )
    };
    VibeAxes {
        moods: [
            ("dark", "Dark", "Мрачная", "#7A82AB"),
            ("melancholy", "Melancholy", "Меланхоличная", "#608EBA"),
            (
                "contemplative",
                "Contemplative",
                "Созерцательная",
                "#6FAEA0",
            ),
            ("warm", "Bright", "Светлая", "#DEA058"),
            ("joyful", "Joyful", "Весёлая", "#E8786C"),
        ]
        .into_iter()
        .map(|(key, en, ru, color)| VibeMood {
            key: key.to_owned(),
            label: label(en, ru),
            hint: None,
            color: color.to_owned(),
        })
        .collect(),
        characters: [
            ("calm", "Calm", "Спокойная", VibeGlyph::Calm),
            ("playful", "Playful", "Игривая", VibeGlyph::Playful),
            ("steady", "Steady", "Умеренная", VibeGlyph::Steady),
            ("edgy", "Raw", "Надрывная", VibeGlyph::Edgy),
            ("drive", "Drive", "Драйв", VibeGlyph::Drive),
        ]
        .into_iter()
        .map(|(key, en, ru, glyph)| VibeCharacter {
            key: key.to_owned(),
            label: label(en, ru),
            hint: None,
            glyph,
        })
        .collect(),
        complexities: [
            ("simple", "Simple", "Простая", 1),
            ("medium", "Medium", "Средняя", 2),
            ("complex", "Complex", "Сложная", 3),
        ]
        .into_iter()
        .map(|(key, en, ru, strokes)| VibeComplexity {
            key: key.to_owned(),
            label: label(en, ru),
            hint: None,
            strokes,
        })
        .collect(),
        labels: None,
        neutral: None,
    }
}
