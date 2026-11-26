//! A short as it is built: the look it wears, what it still lacks, and what
//! is handed to whoever builds it (v0.168).
//!
//! The style of a space's shorts is the profile's ([`ShortStyle`]). A short
//! wears one variant of it - the one a person gave it, or the one the round
//! gives the day it goes out, so that neighbouring days differ. The core
//! never opens the video (decision of 2026-09-11): the window draws the still
//! layers of the frame, this module says what the short looks like and what
//! its words are, and kilna-plugin-cut moves it all.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::{Value, json};

use crate::error::{Error, Result};
use crate::karaoke;
use crate::profile::config::{ProfileConfig, ShortStyle, ShortVariant};
use crate::work::Work;

/// The key of `work.meta` a short's variant is kept under when a person gave
/// it one. Without it the round decides.
pub const VARIANT_KEY: &str = "short_variant";

/// The file the window writes beside the layers it drew: what each is and
/// where it stands.
const LAYERS_FILE: &str = "layers.json";

/// The variant a short wears, and who chose it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct WornVariant {
    pub key: String,
    pub label: String,
    /// A person gave it; otherwise the round did.
    pub by_hand: bool,
}

/// A variant as a picker lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct VariantName {
    pub key: String,
    pub label: String,
}

/// What a short cannot be built without, or is built poorer without.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(tag = "what", rename_all = "camelCase")]
pub enum Missing {
    /// No stretch is marked: there is nothing to splice.
    Cuts,
    /// A work a stretch is cut from has no finished file.
    Source { title: String },
    /// The short has no folder on disk to be built into.
    Folder,
    /// No cover is chosen, and the look ends on one: the short is built
    /// without its tail.
    Cover,
}

impl Missing {
    /// Whether the build cannot start without it.
    pub fn stops(&self) -> bool {
        !matches!(self, Self::Cover)
    }
}

/// Where the words of a short come from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum WordsState {
    /// The look has no karaoke.
    Off,
    /// No file of timings lies beside the song or the video.
    NoTimings,
    /// The song has no text to take the words from.
    NoText,
    /// The file of timings does not read.
    Unreadable { file: String },
    /// Found: how many lines, and the file the times are from.
    Found { lines: u32, file: String },
}

/// A short on its Build tab.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct ShortView {
    /// The title as the header letters it.
    pub title: String,
    /// The style as this short wears it.
    pub look: ShortStyle,
    /// The variants of the style, in order.
    pub variants: Vec<VariantName>,
    /// The one this short wears; none when the style has none.
    pub variant: Option<WornVariant>,
    /// The cover, whole, when one is chosen.
    pub cover: Option<String>,
    /// How long the stretches run together, in seconds.
    pub seconds: f64,
    pub missing: Vec<Missing>,
    pub words: WordsState,
    /// The newest build, whole, when there is one.
    pub file: Option<String>,
}

/// The day a short goes out, as days since the epoch: its first release's
/// day, or else the day it was made.
fn day_of(conn: &Connection, work: &Work) -> Result<i64> {
    let day: Option<String> = conn
        .query_row(
            "SELECT substr(coalesce(scheduled_at, released_at), 1, 10) FROM release
              WHERE work_id = ?1 AND coalesce(scheduled_at, released_at) IS NOT NULL
              ORDER BY coalesce(scheduled_at, released_at), rowid LIMIT 1",
            params![work.id],
            |row| row.get(0),
        )
        .optional()?;
    let day = day.unwrap_or_else(|| work.created_at.chars().take(10).collect());
    let days: Option<f64> =
        conn.query_row("SELECT julianday(?1) - 2440587.5", params![day], |row| {
            row.get(0)
        })?;
    Ok(days.unwrap_or(0.0).floor() as i64)
}

/// The variant `work` wears: the one kept on it while the style still has
/// it, or else the round's.
pub fn variant_of<'a>(
    conn: &Connection,
    style: &'a ShortStyle,
    work: &Work,
) -> Result<Option<(&'a ShortVariant, bool)>> {
    if let Some(kept) = work
        .meta
        .get(VARIANT_KEY)
        .and_then(Value::as_str)
        .and_then(|key| style.variant(key))
    {
        return Ok(Some((kept, true)));
    }
    Ok(style.round(day_of(conn, work)?).map(|found| (found, false)))
}

/// The title the header letters: that of what the short is all made from.
fn title_of(
    conn: &Connection,
    config: &ProfileConfig,
    work: &Work,
    style: &ShortStyle,
) -> Result<String> {
    let title = crate::publication::origin_title(conn, config, work)?;
    Ok(if style.header.title_upper {
        title.to_uppercase()
    } else {
        title
    })
}

/// The folders a file of timings is looked for in: the short's own, those of
/// what it is cut from, and that of the song - the nearest first.
fn timing_folders(conn: &Connection, config: &ProfileConfig, work: &Work) -> Result<Vec<PathBuf>> {
    let mut works = vec![work.clone()];
    for id in crate::link::ancestors(conn, &work.id)? {
        if let Some(up) = crate::work::get(conn, &id)? {
            works.push(up);
        }
    }
    let mut folders = Vec::new();
    for one in &works {
        if let Some(path) = crate::folder::locate(conn, config, one)?.path {
            let path = PathBuf::from(path);
            if path.is_dir() && !folders.contains(&path) {
                folders.push(path);
            }
        }
    }
    Ok(folders)
}

/// The text the words are taken from: the current text of what the short is
/// all made from, or else the newest version in its first role.
fn sung_text(conn: &Connection, config: &ProfileConfig, work: &Work) -> Result<Option<String>> {
    let Some(origin) = crate::publication::origin(conn, config, &work.id)? else {
        return Ok(None);
    };
    let current = match &origin.current_version_id {
        Some(id) => crate::work::version::get(conn, id)?,
        None => None,
    };
    let version = match current {
        Some(version) => Some(version),
        None => match config.vocabulary(&origin.kind).version_roles.first() {
            Some(role) => crate::work::version::latest(conn, &origin.id, &role.key)?,
            None => None,
        },
    };
    Ok(version
        .map(|version| version.body)
        .filter(|body| !body.trim().is_empty()))
}

/// The words of a short, with the file they were timed by.
struct Words {
    lines: Vec<Vec<karaoke::Timed>>,
    file: PathBuf,
    audio: Option<PathBuf>,
    offset: Option<f64>,
}

/// Find and match the words, or say why there are none.
fn words_of(
    conn: &Connection,
    config: &ProfileConfig,
    work: &Work,
    style: &ShortStyle,
) -> Result<std::result::Result<Words, WordsState>> {
    if !style.karaoke.on {
        return Ok(Err(WordsState::Off));
    }
    let Some(file) = karaoke::find(&timing_folders(conn, config, work)?) else {
        return Ok(Err(WordsState::NoTimings));
    };
    let Ok(alignment) = karaoke::read(&file) else {
        return Ok(Err(WordsState::Unreadable {
            file: file.to_string_lossy().into_owned(),
        }));
    };
    let Some(text) = sung_text(conn, config, work)? else {
        return Ok(Err(WordsState::NoText));
    };
    let lines = karaoke::timed(
        &karaoke::song_lines(&text, &alignment.replace),
        &alignment.words,
    );
    if lines.is_empty() {
        return Ok(Err(WordsState::NoText));
    }
    Ok(Ok(Words {
        audio: karaoke::recording(&file, &alignment),
        offset: alignment.offset,
        lines,
        file,
    }))
}

/// What `work` lacks to be built in `style`.
fn missing(
    conn: &Connection,
    config: &ProfileConfig,
    work: &Work,
    style: &ShortStyle,
    cover: Option<&str>,
) -> Result<Vec<Missing>> {
    let mut out = Vec::new();
    let shots = crate::cut::shot_list(conn, config, &work.id)?;
    if shots.is_empty() {
        out.push(Missing::Cuts);
    }
    for shot in &shots {
        let lacks = Missing::Source {
            title: shot.source_title.clone(),
        };
        if shot.path.is_none() && !out.contains(&lacks) {
            out.push(lacks);
        }
    }
    if crate::folder::locate(conn, config, work)?.path.is_none() {
        out.push(Missing::Folder);
    }
    if style.tail.on && cover.is_none() {
        out.push(Missing::Cover);
    }
    Ok(out)
}

/// The work, its space's vocabulary and the style of its shorts.
fn read(conn: &Connection, work_id: &str) -> Result<(Work, ProfileConfig, ShortStyle)> {
    let work = crate::work::get(conn, work_id)?.ok_or_else(|| Error::not_found("work", work_id))?;
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let style = config.short_style();
    Ok((work, config, style))
}

/// A short on its Build tab: the look it wears, what it lacks, its words and
/// its newest build.
pub fn view(conn: &Connection, work_id: &str) -> Result<ShortView> {
    let (work, config, style) = read(conn, work_id)?;
    let chosen = variant_of(conn, &style, &work)?;
    let look = style.worn(chosen.map(|(variant, _)| variant));
    let cover = crate::asset::cover_of(conn, work_id)?.map(|asset| asset.path);
    let seconds = crate::cut::shot_list(conn, &config, work_id)?
        .iter()
        .map(|shot| shot.ends_at - shot.starts_at)
        .sum();
    let words = match words_of(conn, &config, &work, &look)? {
        Ok(words) => WordsState::Found {
            lines: words.lines.len() as u32,
            file: words.file.to_string_lossy().into_owned(),
        },
        Err(state) => state,
    };
    Ok(ShortView {
        title: title_of(conn, &config, &work, &look)?,
        variants: style
            .variants
            .iter()
            .map(|variant| VariantName {
                key: variant.key.clone(),
                label: variant.name().to_owned(),
            })
            .collect(),
        variant: chosen.map(|(variant, by_hand)| WornVariant {
            key: variant.key.clone(),
            label: variant.name().to_owned(),
            by_hand,
        }),
        missing: missing(conn, &config, &work, &look, cover.as_deref())?,
        cover,
        seconds,
        words,
        file: crate::folder::locate(conn, &config, &work)?.file.path,
        look,
    })
}

/// Where the window lays the layers it drew for `work_id`: a folder of the
/// system's temporary one, the same from one build to the next.
pub fn layers_dir(work_id: &str) -> PathBuf {
    let safe: String = work_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    std::env::temp_dir().join("kilna-short").join(safe)
}

/// One layer the window drew.
#[derive(Debug, Clone, serde::Deserialize, ts_rs::TS)]
pub struct Layer {
    /// Its file's name: letters, digits, a hyphen and `.png`.
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Lay the layers of a short into `dir`, in place of the ones before, with
/// what each is and where it stands (`placed`: the ground, the header, the
/// blots, the stamp, the sign - by file name).
pub fn stage(dir: &Path, placed: &Value, layers: &[Layer]) -> Result<()> {
    let plain = |name: &str| {
        name.ends_with(".png")
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
            && !name.contains("..")
    };
    if let Some(odd) = layers.iter().find(|layer| !plain(&layer.name)) {
        return Err(Error::refused("short.layerName").param("name", odd.name.as_str()));
    }
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    std::fs::create_dir_all(dir)?;
    for layer in layers {
        std::fs::write(dir.join(&layer.name), &layer.bytes)?;
    }
    std::fs::write(dir.join(LAYERS_FILE), serde_json::to_vec(placed)?)?;
    Ok(())
}

/// What a short is handed to its builder with, as the `short` of a plugin's
/// subject: the look, the layers the window laid in `dir`, and the words.
///
/// The look travels without its pictures - the sign's and the stamp's are
/// drawn into the layers already, and a builder has no use for the text of
/// an SVG.
pub fn brief(conn: &Connection, work_id: &str, dir: &Path) -> Result<Value> {
    let (work, config, style) = read(conn, work_id)?;
    let chosen = variant_of(conn, &style, &work)?;
    let mut look = style.worn(chosen.map(|(variant, _)| variant));
    let words = words_of(conn, &config, &work, &look)?.ok();
    look.sign.art.clear();
    look.sign.blink.clear();
    look.stamp.art.clear();

    let mut layers = std::fs::read(dir.join(LAYERS_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    layers["dir"] = Value::String(dir.to_string_lossy().into_owned());

    Ok(json!({
        "variant": chosen.map(|(variant, _)| variant.key.clone()),
        "look": look,
        "layers": layers,
        "karaoke": words.map(|words| json!({
            "lines": words.lines,
            "audio": words.audio.map(|path| path.to_string_lossy().into_owned()),
            "offset": words.offset,
        })),
    }))
}

/// A short as an assistant reads it: its stretches, the variant it wears and
/// where its build stands. None for a work that is not cut from another.
pub fn sheet(conn: &Connection, work_id: &str) -> Result<Option<Value>> {
    let (work, config, _) = read(conn, work_id)?;
    if !config.vocabulary(&work.kind).excerpt {
        return Ok(None);
    }
    let view = view(conn, work_id)?;
    let cuts: Vec<Value> = crate::cut::shot_list(conn, &config, work_id)?
        .into_iter()
        .map(|shot| {
            json!({
                "from": shot.source_title,
                "starts_at": shot.starts_at,
                "ends_at": shot.ends_at,
                "has_file": shot.path.is_some(),
            })
        })
        .collect();
    Ok(Some(json!({
        "title": view.title,
        "cuts": cuts,
        "seconds": view.seconds,
        "variant": view.variant,
        "variants": view.variants,
        "cover": view.cover.is_some(),
        "build": {
            "file": view.file,
            "missing": view.missing,
            "words": view.words,
        },
    })))
}

/// What stands in an agent's reading in place of a picture kept as text.
const KEPT: &str = "<kept:";

/// The keys of a style under which a picture is kept as text.
const ART_KEYS: [&str; 4] = ["art", "blink", "sign_art", "sign_blink"];

/// Replace every picture of a style's document by a mark of its length, or
/// - on the way back - take every such mark out again.
fn pictures(value: &mut Value, mask: bool) {
    match value {
        Value::Object(fields) => {
            let keys: Vec<String> = fields.keys().cloned().collect();
            for key in keys {
                let is_art = ART_KEYS.contains(&key.as_str());
                let text = fields[&key].as_str().map(str::to_owned);
                match (is_art, text) {
                    (true, Some(text)) if mask && !text.is_empty() => {
                        fields.insert(key, json!(format!("{KEPT} {} chars>", text.len())));
                    }
                    (true, Some(text)) if !mask && text.starts_with(KEPT) => {
                        fields.remove(&key);
                    }
                    _ => {
                        if let Some(child) = fields.get_mut(&key) {
                            pictures(child, mask);
                        }
                    }
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                pictures(item, mask);
            }
        }
        _ => {}
    }
}

/// The style as an agent reads it: every block and variant, the pictures -
/// the sign's, the stamp's - as marks of their length, since the text of an
/// SVG is nothing to read and a great deal to carry.
pub fn readable(style: &ShortStyle) -> Result<Value> {
    let mut value = serde_json::to_value(style)?;
    pictures(&mut value, true);
    Ok(value)
}

/// Lay `over` on `under`: an object key by key, anything else whole.
fn laid_over(under: &mut Value, over: Value) {
    match (under, over) {
        (Value::Object(under), Value::Object(over)) => {
            for (key, value) in over {
                match under.get_mut(&key) {
                    Some(kept) if kept.is_object() && value.is_object() => laid_over(kept, value),
                    _ => {
                        under.insert(key, value);
                    }
                }
            }
        }
        (under, over) => *under = over,
    }
}

/// The style with what an agent proposed laid over it.
///
/// A block is changed by the keys it names and no others; a variant is
/// changed by its `key`, added when the style has none of that key, and
/// taken out when `remove_variants` names it. A picture's mark, read back,
/// leaves the picture as it is.
pub fn merged(current: &ShortStyle, proposed: &Value) -> Result<ShortStyle> {
    let unreadable = |cause: String| Error::refused("short.styleUnreadable").param("cause", cause);
    let mut proposed = proposed.clone();
    pictures(&mut proposed, false);
    let Value::Object(mut fields) = proposed else {
        return Err(unreadable("a style is an object of blocks".into()));
    };
    let variants = fields.remove("variants");
    let removed: Vec<String> = fields
        .remove("remove_variants")
        .and_then(|keys| serde_json::from_value(keys).ok())
        .unwrap_or_default();

    let mut whole = serde_json::to_value(current)?;
    let kept = whole
        .as_object_mut()
        .and_then(|style| style.remove("variants"))
        .and_then(|list| match list {
            Value::Array(list) => Some(list),
            _ => None,
        })
        .unwrap_or_default();
    laid_over(&mut whole, Value::Object(fields));

    let mut list: Vec<Value> = kept
        .into_iter()
        .filter(|variant| {
            variant["key"]
                .as_str()
                .is_none_or(|key| !removed.iter().any(|gone| gone == key))
        })
        .collect();
    match variants {
        None | Some(Value::Null) => {}
        Some(Value::Array(named)) => {
            for variant in named {
                let key = variant["key"]
                    .as_str()
                    .map(str::trim)
                    .filter(|key| !key.is_empty())
                    .ok_or_else(|| unreadable("a variant needs a key".into()))?
                    .to_owned();
                match list.iter_mut().find(|kept| kept["key"] == key.as_str()) {
                    Some(kept) => laid_over(kept, variant),
                    None => list.push(variant),
                }
            }
        }
        Some(_) => return Err(unreadable("`variants` is a list".into())),
    }
    whole["variants"] = Value::Array(list);
    let style: ShortStyle =
        serde_json::from_value(whole).map_err(|cause| unreadable(cause.to_string()))?;
    let mut seen = std::collections::BTreeSet::new();
    if let Some(twice) = style.variants.iter().find(|v| !seen.insert(v.key.as_str())) {
        return Err(unreadable(format!(
            "the variant `{}` is named twice",
            twice.key
        )));
    }
    Ok(style)
}

/// A proposed style in a few lines a person reads before taking it: its
/// colours, the blocks it draws, its variants.
pub fn render(style: &ShortStyle) -> String {
    let blocks = [
        ("header", style.header.on),
        ("sign", style.sign.on && !style.sign.art.is_empty()),
        ("stamp", style.stamp.on),
        ("wave", style.wave.on),
        ("karaoke", style.karaoke.on),
        ("cover at the end", style.tail.on),
    ];
    let drawn: Vec<&str> = blocks
        .iter()
        .filter(|(_, on)| *on)
        .map(|(name, _)| *name)
        .collect();
    let mut out = format!(
        "**Short style** - colours {} and {}, ground {} ({:?}).\n\nDraws: {}.",
        style.accent,
        style.second,
        style.ground.color,
        style.ground.pattern,
        if drawn.is_empty() {
            "the ground and the picture alone".to_owned()
        } else {
            drawn.join(", ")
        }
    )
    .to_lowercase()
    .replacen("**short style**", "**Short style**", 1)
    .replacen("draws:", "Draws:", 1);
    if !style.variants.is_empty() {
        out.push_str("\n\nVariants:\n");
        for variant in &style.variants {
            let mut own = Vec::new();
            if let Some(accent) = &variant.accent {
                own.push(format!("accent {accent}"));
            }
            if let Some(second) = &variant.second {
                own.push(format!("second {second}"));
            }
            if variant.sign_art.is_some() {
                own.push("its own sign".into());
            }
            if variant.ground.is_some() {
                own.push("its own ground".into());
            }
            if let Some(angle) = variant.stamp_angle {
                own.push(format!("stamp at {angle}°"));
            }
            if let Some(wave) = variant.wave {
                own.push(format!("wave {wave:?}").to_lowercase());
            }
            out.push_str(&format!(
                "- **{}** (`{}`){}\n",
                variant.name(),
                variant.key,
                if own.is_empty() {
                    String::new()
                } else {
                    format!(": {}", own.join(", "))
                }
            ));
        }
    }
    out.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::profile::config::{ShortVariant, WaveKind};

    /// A short made from a clip made from a song, with a media folder and a
    /// style of two variants.
    fn a_family() -> (Connection, String, tempfile::TempDir, Work, Work, Work) {
        let (conn, profile_id) = fixtures::workspace();
        let root = tempfile::tempdir().unwrap();
        crate::folder::set_root(
            &conn,
            &profile_id,
            Some(&root.path().to_string_lossy()),
            "2026-10-26T00:00:00.000Z",
        )
        .unwrap();
        let mut config = crate::profile::config_for(&conn, &profile_id).unwrap();
        config.shorts = Some(ShortStyle {
            variants: vec![
                ShortVariant {
                    key: "day".into(),
                    label: "Day".into(),
                    accent: Some("#FFD23F".into()),
                    ..ShortVariant::default()
                },
                ShortVariant {
                    key: "night".into(),
                    wave: Some(WaveKind::Bars),
                    ..ShortVariant::default()
                },
            ],
            ..ShortStyle::default()
        });
        crate::actions::profile::update_config(&conn, &profile_id, &config).unwrap();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        let clip = crate::actions::work::derive(&conn, &song.id, "video", None, None, None)
            .unwrap()
            .work;
        let short = crate::actions::work::derive(&conn, &clip.id, "short", None, None, None)
            .unwrap()
            .work;
        (conn, profile_id, root, song, clip, short)
    }

    fn cut(conn: &Connection, profile_id: &str, short: &Work, clip: &Work) {
        crate::cut::create(
            conn,
            profile_id,
            crate::cut::NewCut {
                work_id: short.id.clone(),
                source_id: clip.id.clone(),
                starts_at: 12.0,
                ends_at: 20.5,
                position: None,
                label: None,
            },
        )
        .unwrap();
    }

    /// A short with nothing yet says everything it lacks; with a stretch, the
    /// donor's file and a cover it lacks nothing.
    #[test]
    fn a_short_says_what_it_lacks() {
        let (conn, profile_id, root, _, clip, short) = a_family();

        let bare = view(&conn, &short.id).unwrap();
        assert_eq!(bare.missing, vec![Missing::Cuts, Missing::Cover]);
        assert_eq!(
            bare.title, "TIDE",
            "the song's title, as the header letters it"
        );
        assert_eq!(bare.file, None);

        cut(&conn, &profile_id, &short, &clip);
        let half = view(&conn, &short.id).unwrap();
        assert_eq!(
            half.missing,
            vec![
                Missing::Source {
                    title: clip.title.clone()
                },
                Missing::Cover
            ]
        );
        assert!(half.missing[0].stops() && !half.missing[1].stops());
        assert_eq!(half.seconds, 8.5);

        let clip_folder = root.path().join("videos").join(&clip.id);
        std::fs::create_dir_all(&clip_folder).unwrap();
        std::fs::write(clip_folder.join("v1.mp4"), b"not really an mp4").unwrap();
        fixtures::cover(&conn, &profile_id, &short.id);
        let whole = view(&conn, &short.id).unwrap();
        assert_eq!(whole.missing, Vec::new());
        assert!(whole.cover.is_some());
    }

    /// The round gives a short the variant of the day it goes out, a
    /// neighbouring day another; one given by hand stays, until the style
    /// drops it.
    #[test]
    fn a_short_wears_the_variant_of_its_day_or_the_one_it_was_given() {
        let (conn, profile_id, _root, _, clip, short) = a_family();
        let next = crate::actions::work::derive(&conn, &clip.id, "short", None, None, None)
            .unwrap()
            .work;
        // A short is made with its release; the day is what the round reads.
        for (work, day) in [(&short, "2026-11-26"), (&next, "2026-11-27")] {
            let release = crate::release::of_work(&conn, &work.id).unwrap().unwrap();
            crate::actions::release::update(
                &conn,
                &release.id,
                crate::release::ReleasePatch {
                    scheduled_at: Some(Some(day.into())),
                    ..crate::release::ReleasePatch::default()
                },
            )
            .unwrap();
        }

        let first = view(&conn, &short.id).unwrap().variant.unwrap();
        let second = view(&conn, &next.id).unwrap().variant.unwrap();
        assert_ne!(first.key, second.key, "neighbouring days differ");
        assert!(!first.by_hand);

        crate::actions::work::update(
            &conn,
            &next.id,
            crate::work::WorkPatch {
                meta: json!({ VARIANT_KEY: first.key }).as_object().cloned(),
                ..crate::work::WorkPatch::default()
            },
        )
        .unwrap();
        let given = view(&conn, &next.id).unwrap();
        assert_eq!(given.variant.as_ref().unwrap().key, first.key);
        assert!(given.variant.unwrap().by_hand);

        let mut config = crate::profile::config_for(&conn, &profile_id).unwrap();
        config
            .shorts
            .as_mut()
            .unwrap()
            .variants
            .retain(|v| v.key != first.key);
        crate::actions::profile::update_config(&conn, &profile_id, &config).unwrap();
        let after = view(&conn, &next.id).unwrap().variant.unwrap();
        assert_ne!(
            after.key, first.key,
            "a variant the style dropped is not worn"
        );
        assert!(!after.by_hand);
    }

    /// The words are the song's text, timed by the file laid beside the song;
    /// with no file the short has no words, and says so.
    #[test]
    fn the_words_come_from_the_song_and_the_timings_beside_it() {
        let (conn, profile_id, root, song, clip, short) = a_family();
        cut(&conn, &profile_id, &short, &clip);
        assert_eq!(view(&conn, &short.id).unwrap().words, WordsState::NoTimings);

        let song_folder = root.path().join("songs").join(&song.id);
        std::fs::create_dir_all(&song_folder).unwrap();
        let file = song_folder.join("tide.alignment.json");
        std::fs::write(
            &file,
            r#"{"words": [{"word": "The", "start_s": 12.5, "end_s": 13.0},
                          {"word": " tide", "start_s": 13.0, "end_s": 14.0}], "offset": 1.5}"#,
        )
        .unwrap();
        assert_eq!(view(&conn, &short.id).unwrap().words, WordsState::NoText);

        fixtures::version(&conn, &song.id, "lyrics", "[Verse]\nThe tIde\n");
        assert_eq!(
            view(&conn, &short.id).unwrap().words,
            WordsState::Found {
                lines: 1,
                file: file.to_string_lossy().into_owned()
            }
        );

        let dir = root.path().join("layers");
        stage(
            &dir,
            &json!({ "ground": "ground.png", "blots": [] }),
            &[Layer {
                name: "ground.png".into(),
                bytes: vec![1, 2, 3],
            }],
        )
        .unwrap();
        let brief = brief(&conn, &short.id, &dir).unwrap();
        assert_eq!(brief["karaoke"]["lines"][0][1]["word"], "tide");
        assert_eq!(brief["karaoke"]["lines"][0][1]["at"], 13.0);
        assert_eq!(brief["karaoke"]["offset"], 1.5);
        assert_eq!(brief["layers"]["ground"], "ground.png");
        assert_eq!(brief["layers"]["dir"], json!(dir.to_string_lossy()));
        assert_eq!(brief["look"]["wave"]["on"], true);
        assert!(
            brief["look"]
                .get("variants")
                .is_some_and(|v| v.as_array().unwrap().is_empty())
        );
        assert_eq!(
            std::fs::read(dir.join("ground.png")).unwrap(),
            vec![1, 2, 3]
        );

        std::fs::write(&file, "not json").unwrap();
        assert!(matches!(
            view(&conn, &short.id).unwrap().words,
            WordsState::Unreadable { .. }
        ));
    }

    /// A layer is a plain file name; anything else is refused before a file
    /// is written.
    #[test]
    fn a_layer_is_laid_only_under_a_plain_name() {
        let dir = tempfile::tempdir().unwrap();
        let odd = stage(
            &dir.path().join("layers"),
            &json!({}),
            &[Layer {
                name: "../outside.png".into(),
                bytes: vec![1],
            }],
        );
        assert!(odd.is_err());
        assert!(!dir.path().join("outside.png").exists());
        assert!(layers_dir("../a-b/c").ends_with("a-bc"));
    }

    /// An assistant reads a short's stretches, its variant and its build; a
    /// work that is not cut from another has no such sheet.
    #[test]
    fn a_short_is_read_as_a_sheet() {
        let (conn, profile_id, _root, song, clip, short) = a_family();
        cut(&conn, &profile_id, &short, &clip);

        let sheet = sheet(&conn, &short.id).unwrap().unwrap();
        assert_eq!(sheet["cuts"][0]["starts_at"], 12.0);
        assert_eq!(sheet["cuts"][0]["has_file"], false);
        assert_eq!(sheet["variants"].as_array().unwrap().len(), 2);
        assert_eq!(sheet["build"]["missing"][0]["what"], "source");
        assert_eq!(sheet["build"]["words"]["state"], "noTimings");
        assert!(super::sheet(&conn, &song.id).unwrap().is_none());
    }

    /// An agent reads the style without its pictures, names only what it
    /// changes, and the pictures it did not touch stay as they were.
    #[test]
    fn a_proposed_style_is_laid_over_the_one_there() {
        let current = ShortStyle {
            sign: crate::profile::config::ShortSign {
                on: true,
                art: "<svg id=\"cat\"/>".into(),
                ..crate::profile::config::ShortSign::default()
            },
            variants: vec![
                ShortVariant {
                    key: "day".into(),
                    sign_art: Some("<svg id=\"day\"/>".into()),
                    ..ShortVariant::default()
                },
                ShortVariant {
                    key: "night".into(),
                    ..ShortVariant::default()
                },
            ],
            ..ShortStyle::default()
        };

        let read = readable(&current).unwrap();
        assert_eq!(read["sign"]["art"], "<kept: 15 chars>");
        assert_eq!(read["variants"][0]["sign_art"], "<kept: 15 chars>");

        let proposed = json!({
            "accent": "#FF2E63",
            "sign": { "art": read["sign"]["art"], "sway": 9 },
            "wave": { "kind": "bars" },
            "variants": [
                { "key": "day", "accent": "#FFD23F", "sign_art": read["variants"][0]["sign_art"] },
                { "key": "dusk", "label": "Dusk", "stamp_angle": 4 }
            ],
            "remove_variants": ["night"],
        });
        let style = merged(&current, &proposed).unwrap();

        assert_eq!(style.accent, "#FF2E63");
        assert_eq!(style.second, current.second, "what is not named stays");
        assert_eq!(
            style.sign.art, "<svg id=\"cat\"/>",
            "a mark read back keeps the picture"
        );
        assert_eq!(style.sign.sway, 9.0);
        assert!(style.sign.on);
        assert_eq!(style.wave.kind, WaveKind::Bars);
        assert!(style.wave.strip);
        let keys: Vec<&str> = style.variants.iter().map(|v| v.key.as_str()).collect();
        assert_eq!(keys, vec!["day", "dusk"]);
        assert_eq!(style.variants[0].accent.as_deref(), Some("#FFD23F"));
        assert_eq!(
            style.variants[0].sign_art.as_deref(),
            Some("<svg id=\"day\"/>")
        );
        assert_eq!(style.variants[1].stamp_angle, Some(4.0));

        let said = render(&style);
        assert!(
            said.starts_with("**Short style** - colours #ff2e63 and"),
            "{said}"
        );
        assert!(said.contains("- **Dusk** (`dusk`): stamp at 4°"), "{said}");

        assert!(merged(&current, &json!({ "variants": [{ "label": "no key" }] })).is_err());
        assert!(merged(&current, &json!({ "wave": { "kind": "spiral" } })).is_err());
        assert!(merged(&current, &json!("a style")).is_err());
    }
}
