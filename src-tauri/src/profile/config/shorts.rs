//! What a space's shorts look like: the style, built of blocks, and its
//! variants (v0.168).
//!
//! A channel's shorts are one frame worn many ways. The frame - where the
//! picture stands, where the words light up, where the sign lives - is the
//! same for every short, and is the application's. What is drawn in it is
//! the space's: a header, a sign that moves with the sound, a ground, a
//! stamp, a wave, the words, the effects, the cover at the end - each a
//! block with its own settings and its own switch. A variant is a few values
//! laid over the style - the colours, the sign, the ground, the tilt of the
//! stamp, the kind of wave - so that the shorts of neighbouring days differ
//! and are still one channel's.
//!
//! On the profile, as the rhythm and the kinds of style are: it is the
//! space's vocabulary, saved, logged and taken back as one document.

use serde::{Deserialize, Serialize};

/// The style of a space's shorts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct ShortStyle {
    /// The two colours everything is drawn in, `#RRGGBB`: the wave, the
    /// sung words and half the blots take the first, the call and the other
    /// half the second.
    pub accent: String,
    pub second: String,
    pub header: ShortHeader,
    pub sign: ShortSign,
    pub ground: ShortGround,
    pub stamp: ShortStamp,
    pub wave: ShortWave,
    pub karaoke: ShortKaraoke,
    pub effects: ShortEffects,
    pub tail: ShortTail,
    /// The ways the style is worn. With none, every short wears the style as
    /// it is.
    pub variants: Vec<ShortVariant>,
}

impl Default for ShortStyle {
    /// What a space that never set a style builds its shorts in: every block
    /// that needs nothing of the channel's own is on.
    fn default() -> Self {
        Self {
            accent: "#F2C14E".into(),
            second: "#4ED1F2".into(),
            header: ShortHeader::default(),
            sign: ShortSign::default(),
            ground: ShortGround::default(),
            stamp: ShortStamp::default(),
            wave: ShortWave::default(),
            karaoke: ShortKaraoke::default(),
            effects: ShortEffects::default(),
            tail: ShortTail::default(),
            variants: Vec::new(),
        }
    }
}

/// The words at the top of the frame: the title of the song and a call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct ShortHeader {
    pub on: bool,
    /// Whether the title is lettered in capitals.
    pub title_upper: bool,
    /// The call under the title. Empty draws none.
    pub call: String,
    /// The font the words are lettered in, as a stylesheet names one.
    pub font: String,
}

impl Default for ShortHeader {
    fn default() -> Self {
        Self {
            on: true,
            title_upper: true,
            call: "the whole song is on the channel".into(),
            font: "Arial".into(),
        }
    }
}

/// The channel's sign, under the words: it grows with the loudness, sways
/// and blinks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct ShortSign {
    pub on: bool,
    /// The sign itself: the text of an SVG, or a picture as a `data:` address.
    /// Empty draws none.
    pub art: String,
    /// The same with its eyes shut, for a blink. Empty never blinks.
    pub blink: String,
    /// Its side at rest, in pixels of the 1080 frame.
    pub size: f64,
    /// How much larger the loudest moment makes it: 0.24 is a quarter; 0
    /// stands still.
    pub pulse: f64,
    /// How far it sways either way, in degrees.
    pub sway: f64,
}

impl Default for ShortSign {
    fn default() -> Self {
        Self {
            on: false,
            art: String::new(),
            blink: String::new(),
            size: 310.0,
            pulse: 0.24,
            sway: 5.0,
        }
    }
}

/// What a ground is covered with.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum GroundPattern {
    /// The colour alone.
    Plain,
    /// Ink blots, each where the style puts it.
    #[default]
    Blots,
    /// A scatter of small dots in the two colours.
    Dots,
    /// A fine grid.
    Grid,
    /// Slanted stripes.
    Stripes,
}

/// Which of the style's two colours a blot takes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum ShortColor {
    #[default]
    Accent,
    Second,
}

/// The shape a blot is thrown in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum BlotShape {
    #[default]
    Splat,
    Drip,
    Spray,
    Smear,
}

/// One blot of the ground: where its heart is and how large it is, in pixels
/// of the 1080x1920 frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct ShortBlot {
    pub x: f64,
    pub y: f64,
    pub size: f64,
    #[serde(default)]
    pub color: ShortColor,
    #[serde(default)]
    pub shape: BlotShape,
}

/// What is under everything.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct ShortGround {
    /// The colour of the ground, `#RRGGBB`.
    pub color: String,
    pub pattern: GroundPattern,
    /// What the pattern is drawn from: the same number draws the same
    /// blots, another draws others in the same places.
    pub seed: u32,
    /// The blots of the `blots` pattern.
    pub blots: Vec<ShortBlot>,
    /// Whether the blots drift and are thrown sideways where the sound hits.
    pub motion: bool,
}

impl Default for ShortGround {
    fn default() -> Self {
        Self {
            color: "#121114".into(),
            pattern: GroundPattern::Blots,
            seed: 1,
            blots: vec![
                ShortBlot {
                    x: 60.0,
                    y: 340.0,
                    size: 1250.0,
                    color: ShortColor::Accent,
                    shape: BlotShape::Splat,
                },
                ShortBlot {
                    x: 1010.0,
                    y: 110.0,
                    size: 820.0,
                    color: ShortColor::Second,
                    shape: BlotShape::Drip,
                },
                ShortBlot {
                    x: 1005.0,
                    y: 1330.0,
                    size: 1000.0,
                    color: ShortColor::Accent,
                    shape: BlotShape::Drip,
                },
                ShortBlot {
                    x: 90.0,
                    y: 1770.0,
                    size: 860.0,
                    color: ShortColor::Second,
                    shape: BlotShape::Spray,
                },
            ],
            motion: true,
        }
    }
}

/// How the channel's name is set over the picture's top edge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum StampShape {
    /// A rubber stamp: a double frame, the name inside, worn.
    #[default]
    Stamp,
    /// A solid plate with the name cut out of it.
    Plate,
}

/// The channel's name, stamped between the header and the picture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct ShortStamp {
    pub on: bool,
    pub shape: StampShape,
    /// The name. Empty draws none, unless the stamp is a picture of its own.
    pub text: String,
    /// A second, smaller line under the name.
    pub sub: String,
    /// The stamp as a picture of the channel's own - the text of an SVG or a
    /// `data:` address - drawn in place of the lettered one.
    pub art: String,
    /// The colour of its ink, `#RRGGBB`.
    pub color: String,
    /// Its tilt, in degrees: negative leans it back.
    pub angle: f64,
    /// Where its middle stands across the 1080 frame.
    pub x: f64,
    /// How worn it is, 0 to 1: 0 is a clean print.
    pub wear: f64,
}

impl Default for ShortStamp {
    fn default() -> Self {
        Self {
            on: false,
            shape: StampShape::Stamp,
            text: String::new(),
            sub: String::new(),
            art: String::new(),
            color: "#D7262E".into(),
            angle: -6.0,
            x: 610.0,
            wear: 0.6,
        }
    }
}

/// How the sound is drawn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum WaveKind {
    /// One filled wave in the accent.
    #[default]
    Solid,
    /// Two lines, one in each colour.
    Lines,
    /// Mirrored bars: the bass in the middle, the highs at both edges.
    Bars,
}

/// The sound, drawn under the picture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct ShortWave {
    pub on: bool,
    pub kind: WaveKind,
    /// The strip under the picture that fills as the short plays.
    pub strip: bool,
}

impl Default for ShortWave {
    fn default() -> Self {
        Self {
            on: true,
            kind: WaveKind::Solid,
            strip: true,
        }
    }
}

/// The words of the song, lit as they are sung.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct ShortKaraoke {
    pub on: bool,
    /// The font's name, as the system knows it.
    pub font: String,
    /// The size of the letters, in pixels of the 1920 frame.
    pub size: u32,
}

impl Default for ShortKaraoke {
    fn default() -> Self {
        Self {
            on: true,
            font: "Arial".into(),
            size: 60,
        }
    }
}

/// What is done to the picture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct ShortEffects {
    /// How far the picture pushes in over the short: 0.1 is a tenth, 0 holds
    /// it still.
    pub zoom: f64,
    /// Film grain, 0 to 40; 0 is none.
    pub grain: u32,
    pub vignette: bool,
    /// The picture tears for a moment where the sound hits.
    pub glitch: bool,
}

impl Default for ShortEffects {
    fn default() -> Self {
        Self {
            zoom: 0.1,
            grain: 7,
            vignette: true,
            glitch: true,
        }
    }
}

/// The cover, standing at the end.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default)]
pub struct ShortTail {
    pub on: bool,
    /// How long it stands, in seconds.
    pub seconds: f64,
}

impl Default for ShortTail {
    fn default() -> Self {
        Self {
            on: true,
            seconds: 1.2,
        }
    }
}

/// The ground of a variant, in place of the style's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct VariantGround {
    #[serde(default)]
    pub pattern: GroundPattern,
    #[serde(default)]
    pub seed: u32,
    #[serde(default)]
    pub blots: Vec<ShortBlot>,
}

/// One way the style is worn: what it says replaces the style's, what it
/// leaves out is the style's.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[ts(optional_fields)]
pub struct ShortVariant {
    pub key: String,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub second: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sign_art: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sign_blink: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground: Option<VariantGround>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stamp_angle: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stamp_x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wave: Option<WaveKind>,
}

impl ShortVariant {
    /// What it is called: its label, or its key when it has none.
    pub fn name(&self) -> &str {
        if self.label.trim().is_empty() {
            &self.key
        } else {
            &self.label
        }
    }
}

impl ShortStyle {
    /// The variant keyed `key`.
    pub fn variant(&self, key: &str) -> Option<&ShortVariant> {
        self.variants.iter().find(|variant| variant.key == key)
    }

    /// The style as `variant` wears it: one look, with no variants of its
    /// own. With none, the style as it is.
    pub fn worn(&self, variant: Option<&ShortVariant>) -> Self {
        let mut look = Self {
            variants: Vec::new(),
            ..self.clone()
        };
        let Some(variant) = variant else {
            return look;
        };
        if let Some(accent) = &variant.accent {
            look.accent.clone_from(accent);
        }
        if let Some(second) = &variant.second {
            look.second.clone_from(second);
        }
        if let Some(art) = &variant.sign_art {
            look.sign.art.clone_from(art);
            // A sign of its own blinks with its own eyes, or not at all.
            look.sign.blink = variant.sign_blink.clone().unwrap_or_default();
        } else if let Some(blink) = &variant.sign_blink {
            look.sign.blink.clone_from(blink);
        }
        if let Some(ground) = &variant.ground {
            look.ground.pattern = ground.pattern;
            look.ground.seed = ground.seed;
            look.ground.blots.clone_from(&ground.blots);
        }
        if let Some(angle) = variant.stamp_angle {
            look.stamp.angle = angle;
        }
        if let Some(x) = variant.stamp_x {
            look.stamp.x = x;
        }
        if let Some(wave) = variant.wave {
            look.wave.kind = wave;
        }
        look
    }

    /// The variant the round gives the day `days` since the epoch: one after
    /// another, so neighbouring days differ. None when the style has none.
    pub fn round(&self, days: i64) -> Option<&ShortVariant> {
        if self.variants.is_empty() {
            return None;
        }
        let at = days.rem_euclid(self.variants.len() as i64) as usize;
        self.variants.get(at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_variants() -> ShortStyle {
        ShortStyle {
            variants: vec![
                ShortVariant {
                    key: "day".into(),
                    label: "Day".into(),
                    accent: Some("#FFD23F".into()),
                    stamp_angle: Some(5.0),
                    wave: Some(WaveKind::Bars),
                    ..ShortVariant::default()
                },
                ShortVariant {
                    key: "night".into(),
                    sign_art: Some("<svg id=\"night\"/>".into()),
                    ground: Some(VariantGround {
                        pattern: GroundPattern::Dots,
                        seed: 9,
                        blots: Vec::new(),
                    }),
                    ..ShortVariant::default()
                },
            ],
            sign: ShortSign {
                art: "<svg id=\"own\"/>".into(),
                blink: "<svg id=\"shut\"/>".into(),
                ..ShortSign::default()
            },
            ..ShortStyle::default()
        }
    }

    /// A variant replaces what it names and leaves the rest of the style as
    /// it is; the look has no variants of its own.
    #[test]
    fn a_variant_is_laid_over_the_style() {
        let style = two_variants();

        let day = style.worn(style.variant("day"));
        assert_eq!(day.accent, "#FFD23F");
        assert_eq!(
            day.second, style.second,
            "what it does not name is the style's"
        );
        assert_eq!(day.stamp.angle, 5.0);
        assert_eq!(day.wave.kind, WaveKind::Bars);
        assert_eq!(day.sign.blink, "<svg id=\"shut\"/>");
        assert!(day.variants.is_empty());

        let night = style.worn(style.variant("night"));
        assert_eq!(night.sign.art, "<svg id=\"night\"/>");
        assert_eq!(
            night.sign.blink, "",
            "another sign does not blink with the style's eyes"
        );
        assert_eq!(night.ground.pattern, GroundPattern::Dots);
        assert_eq!(night.ground.seed, 9);
        assert_eq!(night.ground.color, style.ground.color);

        assert_eq!(style.worn(None).accent, style.accent);
    }

    /// The round gives neighbouring days different variants, and a style
    /// with none gives none.
    #[test]
    fn the_round_goes_by_the_day() {
        let style = two_variants();
        assert_eq!(style.round(20_000).unwrap().key, "day");
        assert_eq!(style.round(20_001).unwrap().key, "night");
        assert_eq!(
            style.round(-1).unwrap().key,
            "night",
            "a day before the epoch still has one"
        );
        assert!(ShortStyle::default().round(20_000).is_none());
        assert_eq!(
            style.variants[1].name(),
            "night",
            "a variant with no label is called by its key"
        );
    }

    /// A document that names one block keeps the defaults of the rest.
    #[test]
    fn a_style_reads_from_what_it_names() {
        let style: ShortStyle =
            serde_json::from_str(r##"{"accent": "#FF2E63", "wave": {"kind": "lines"}}"##).unwrap();
        assert_eq!(style.accent, "#FF2E63");
        assert_eq!(style.wave.kind, WaveKind::Lines);
        assert!(style.wave.on && style.wave.strip);
        assert!(style.header.on);
        assert_eq!(style.ground.blots.len(), 4);
    }
}
