//! The frame of a work that plays under one picture for its whole length.
//!
//! An audio release on a video platform is a track with a picture on it: one
//! still that stays on screen for the whole song, and a short loop of what
//! moves in it - a leaf drifting across, dust turning in a beam - repeated end
//! to end. The still and what must stay out of it are text a person writes.
//! The loop is written from settings rather than typed: how long one turn
//! runs, whether the camera holds still, whether the last frame meets the
//! first. Those are not words of a craft but facts of a loop, which is why the
//! frame's shape is the application's and not the profile's (the owner's
//! decision of 2026-09-27: the frame is built in, the way a cover's layout is -
//! ADR 0046). A kind says only whether its works have one (`WorkKind::frame`).

use serde::{Deserialize, Serialize};

/// How long one turn of a loop runs when nobody has said: long enough for a
/// leaf to cross the frame, short enough to join without a visible seam.
pub const DEFAULT_SECONDS: u32 = 6;

/// The lengths a loop may run, in seconds. A generator makes clips of a few
/// seconds; a turn shorter than two reads as a stutter and one longer than
/// twenty is a scene, not a loop.
pub const SECONDS: std::ops::RangeInclusive<u32> = 2..=20;

/// A work's frame: the still, the loop, and what to keep out of both.
///
/// Every part has a value even before anything is written - the loop's
/// settings start at what a loop under a song nearly always is (six seconds,
/// a camera that does not move, a seamless join) - so a frame read from `{}`
/// is a frame, not an absence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct Frame {
    /// The picture that stands for the whole track: who or what, where, the
    /// light, the lens, the mood.
    pub still: String,
    /// What moves in it, round and round - the one thing alive in a picture
    /// that otherwise holds still.
    pub motion: String,
    /// How long one turn of the loop runs.
    pub seconds: u32,
    /// The camera does not move: no push-in, no drift, no shake.
    pub still_camera: bool,
    /// The last frame matches the first, so the loop can repeat for the whole
    /// track without a jump.
    pub seamless: bool,
    /// What must not appear in the still or the loop.
    pub negative: String,
    /// Whether the still is built from the cover's concept without its
    /// words - the idea, the hero, the style, the ground, the mark - with the
    /// still's text added after it as the person's own words; or is the still
    /// as written, whole (v0.88). Absent - every frame written before - it is
    /// built when nobody wrote a still, and theirs when somebody did: see
    /// [`Frame::from_cover`]. A flag rather than a word, so the search, which
    /// reads a frame's words, does not find "cover" in every frame.
    pub built: Option<bool>,
    /// The frame's own layout, when it is built from the cover and a
    /// different one suits a picture with no words on it. Absent, the
    /// cover's.
    pub framing: Option<crate::cover::Framing>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            still: String::new(),
            motion: String::new(),
            seconds: DEFAULT_SECONDS,
            still_camera: true,
            seamless: true,
            negative: String::new(),
            built: None,
            framing: None,
        }
    }
}

/// The frame as it is copied into a generator: three blocks, each pasted into
/// its own box - the picture, the motion made from it, what to keep out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct FramePrompts {
    /// The still, as written.
    pub still: String,
    /// The loop, written from its settings around what moves.
    #[serde(rename = "loop")]
    pub loop_: String,
    /// What to keep out, as written.
    pub negative: String,
}

impl Frame {
    /// Whether a person has written anything into the frame yet: the
    /// settings alone are not a frame anyone asked for.
    pub fn is_written(&self) -> bool {
        !(self.still.trim().is_empty()
            && self.motion.trim().is_empty()
            && self.negative.trim().is_empty())
    }

    /// Whether the still is built from the cover. A frame that never said -
    /// written before v0.88 - is built from the cover unless a person wrote
    /// its still, which stays theirs.
    pub fn from_cover(&self) -> bool {
        self.built.unwrap_or_else(|| self.still.trim().is_empty())
    }

    /// The loop's length, held to the lengths a loop may run.
    pub fn seconds(&self) -> u32 {
        self.seconds.clamp(*SECONDS.start(), *SECONDS.end())
    }

    /// The frame as three blocks to copy.
    ///
    /// The loop is the one block the application writes: what moves is the
    /// person's, the rest follows from the settings, in the generator's
    /// English - a still camera and a seamless join are instructions a model
    /// reads, not a mood it guesses from the picture. A loop with nothing
    /// named to move still breathes: an empty motion is the light, barely,
    /// rather than a sentence with a hole in it.
    pub fn prompts(&self) -> FramePrompts {
        let motion = self.motion.trim().trim_end_matches('.');
        let motion = if motion.is_empty() {
            "barely noticeable breathing of the light"
        } else {
            motion
        };
        let mut sentences = vec![format!("LOOP ({} s): {motion}.", self.seconds())];
        if self.still_camera {
            sentences.push("The camera does not move at all.".to_owned());
        }
        if self.seamless {
            sentences.push(
                "Seamless loop: the last frame matches the first exactly, so it can repeat for the whole track."
                    .to_owned(),
            );
        }
        sentences.push(
            "Keep every other element of the picture still; no cuts, no zoom, no new objects."
                .to_owned(),
        );
        FramePrompts {
            still: self.still.trim().to_owned(),
            loop_: sentences.join(" "),
            negative: self.negative.trim().to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_read_from_nothing_starts_at_what_a_loop_under_a_song_is() {
        let frame: Frame = serde_json::from_str("{}").unwrap();
        assert_eq!(frame, Frame::default());
        assert_eq!(frame.seconds, 6);
        assert!(frame.still_camera && frame.seamless);
        assert!(!frame.is_written(), "settings alone are not a frame");
    }

    #[test]
    fn the_loop_is_written_from_what_moves_and_the_settings() {
        let frame = Frame {
            motion: "autumn leaves drift across the window.".into(),
            seconds: 8,
            ..Frame::default()
        };

        let prompts = frame.prompts();

        assert!(
            prompts
                .loop_
                .starts_with("LOOP (8 s): autumn leaves drift across the window."),
            "{}",
            prompts.loop_
        );
        assert!(prompts.loop_.contains("The camera does not move at all."));
        assert!(prompts.loop_.contains("Seamless loop"));
    }

    /// Each setting says its sentence and nothing else does: a camera allowed
    /// to move is not told to hold still, and a loop allowed to jump is not
    /// promised a seamless join.
    #[test]
    fn a_setting_turned_off_takes_its_sentence_with_it() {
        let frame = Frame {
            motion: "rain on the glass".into(),
            still_camera: false,
            seamless: false,
            ..Frame::default()
        };

        let prompts = frame.prompts();

        assert!(!prompts.loop_.contains("camera does not move"));
        assert!(!prompts.loop_.contains("Seamless"));
        assert!(prompts.loop_.contains("rain on the glass"));
    }

    #[test]
    fn a_loop_with_nothing_named_to_move_still_breathes() {
        let prompts = Frame::default().prompts();
        assert!(
            prompts
                .loop_
                .contains("barely noticeable breathing of the light"),
            "an empty motion is a sentence, not a hole: {}",
            prompts.loop_
        );
    }

    /// A frame that never said where its still comes from keeps a still a
    /// person wrote, and is built from the cover otherwise; one that says is
    /// taken at its word.
    #[test]
    fn a_frame_is_built_from_the_cover_unless_its_still_was_written() {
        assert!(Frame::default().from_cover());
        let written = Frame {
            still: "a lamp on a windowsill".into(),
            ..Frame::default()
        };
        assert!(!written.from_cover());
        let told = Frame {
            built: Some(true),
            ..written.clone()
        };
        assert!(told.from_cover());
        let own = Frame {
            built: Some(false),
            ..Frame::default()
        };
        assert!(!own.from_cover());
    }

    #[test]
    fn a_length_outside_what_a_loop_can_be_is_held_to_it() {
        let long = Frame {
            seconds: 90,
            ..Frame::default()
        };
        let none = Frame {
            seconds: 0,
            ..Frame::default()
        };
        assert_eq!(long.seconds(), 20);
        assert_eq!(none.seconds(), 2);
        assert!(long.prompts().loop_.starts_with("LOOP (20 s)"));
    }
}
