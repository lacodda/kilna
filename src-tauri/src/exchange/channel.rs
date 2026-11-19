//! A channel brought in from the folder a previous system kept it in (v0.154,
//! FR-20 20.5): `kilna --import-channel <dir>` reads the channel's README,
//! its register of repeats, its bank of traditions, its plan and its month
//! packs into a space made from the music channel profile - a plan and a
//! report unless `--apply` is said, a copy of the space kept before anything
//! is written, every row it writes one unit.
//!
//! What a channel says is never read for its meaning: a section goes to the
//! passport by the words of its heading, a table's column by the words of its
//! header, a track by its title.

pub mod read;

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::error::{Error, Result};
use read::{Channel, Section};

/// A fact for the passport: its section, its words, and a slot's name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PassportFact {
    pub section: String,
    pub body: String,
    pub slot: Option<String>,
}

/// A track to make or to fill: its plan and what its pack says.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Track {
    pub date: String,
    pub title: String,
    /// The work of that title already in the space: filled where empty,
    /// never made twice.
    pub existing: Option<String>,
    pub fields: Map<String, Value>,
    pub suno: Option<String>,
    pub cover: Option<String>,
    pub release: Map<String, Value>,
    /// The files of its month's media folder named after it: its track,
    /// its picture.
    pub files: Vec<String>,
}

/// A month: its collection and its tracks.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Month {
    pub month: String,
    pub title: String,
    pub arc: Option<String>,
    pub tracks: Vec<Track>,
}

/// An item for the bank.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BankItem {
    pub name: String,
    pub group: String,
    pub attributes: Vec<String>,
    pub note: String,
    /// The last month it was taken in, `YYYY-MM`.
    pub taken: Option<String>,
}

/// What bringing a channel in would do.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Plan {
    pub facts: Vec<PassportFact>,
    /// The README's sections that state rules - to be written as quotas by
    /// a person: a rule in prose is not a predicate a machine can read.
    pub rules: Vec<String>,
    pub terms: Vec<(String, String)>,
    pub traditions: Vec<BankItem>,
    pub cooldown_months: Option<u64>,
    pub months: Vec<Month>,
    /// What could not be placed: a pack track no plan row names, and so on.
    pub conflicts: Vec<String>,
}

/// Which passport section a README section's words go to, by its heading.
fn section_for(heading: &str) -> &'static str {
    let lower = heading.to_lowercase();
    if lower.contains("плейлист") || lower.contains("playlist") {
        "playlists"
    } else if lower.contains("регистр")
        || lower.contains("register")
        || lower.contains("voice")
        || lower.contains("голос")
    {
        "voice"
    } else if lower.contains("запрещ") || lower.contains("ловушк") || lower.contains("ban")
    {
        "bans"
    } else if lower.contains("художник") || lower.contains("artist") {
        "artists"
    } else {
        "identity"
    }
}

/// The statements of a block of words: its bullets and its paragraphs, a
/// table's lines and quotes left out.
fn statements(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('|') && !line.starts_with('>'))
        .map(|line| {
            line.trim_start_matches(['-', '*', '•'])
                .trim()
                .replace("**", "")
                .replace('`', "")
        })
        .filter(|line| !line.is_empty())
        .collect()
}

/// The passport facts of the README: a section's words under the section
/// its heading names - an identity's as named slots, a playlist group's as
/// its slot - and the parts of a section by their own headings.
fn passport(readme: &[Section], plan: &mut Plan) {
    let mut push = |section: &str, slot: Option<String>, text: &str| {
        for body in statements(text) {
            let slot = match section {
                "identity" | "playlists" => slot.clone().or_else(|| Some("-".into())),
                _ => None,
            };
            plan.facts.push(PassportFact {
                section: section.into(),
                body,
                slot,
            });
        }
    };
    for section in readme {
        let lower = section.heading.to_lowercase();
        if lower.contains("правил") || lower.contains("rule") {
            plan.rules.push(section.heading.clone());
        }
        let target = section_for(&section.heading);
        push(target, Some(section.heading.clone()), &section.body);
        for (heading, body) in &section.parts {
            let part = section_for(heading);
            let target = if part == "identity" { target } else { part };
            push(target, Some(heading.clone()), body);
        }
    }
}

/// A plan row's cell by the words of its column.
fn cell<'a>(cells: &'a [(String, String)], words: &[&str]) -> Option<&'a str> {
    cells
        .iter()
        .find(|(header, _)| {
            let header = header.to_lowercase();
            words.iter().any(|word| header.contains(word))
        })
        .map(|(_, value)| value.as_str())
        .filter(|value| !value.is_empty())
}

/// A planned track's fields, read off its row.
fn fields_of(track: &read::PlannedTrack) -> (Map<String, Value>, Map<String, Value>) {
    let mut fields = Map::new();
    let mut release = Map::new();
    if let Some(formula) = cell(&track.cells, &["традиц", "tradition"]) {
        match formula
            .split_once('×')
            .or_else(|| formula.split_once(" x "))
        {
            Some((tradition, genre)) => {
                fields.insert("tradition".into(), json!(tradition.trim()));
                fields.insert("genre".into(), json!(genre.trim()));
            }
            None => {
                fields.insert("tradition".into(), json!(formula.trim()));
            }
        }
    }
    if let Some(sound) = cell(&track.cells, &["bpm"]) {
        let parts: Vec<&str> = sound.split('·').map(str::trim).collect();
        if let Some(bpm) = parts.first().and_then(|bpm| bpm.parse::<f64>().ok()) {
            fields.insert("bpm".into(), json!(bpm));
        }
        if let Some(key) = parts.get(1).filter(|key| !key.is_empty()) {
            fields.insert("key".into(), json!(key));
        }
        if let Some(mood) = parts.get(2).filter(|mood| !mood.is_empty()) {
            fields.insert("mood".into(), json!(mood));
        }
    }
    if let Some(sound) = cell(&track.cells, &["звук", "sound"]) {
        fields.insert("sound_design".into(), json!(sound));
    }
    if let Some(cover) = cell(&track.cells, &["обложк", "cover"]) {
        fields.insert("cover_idea".into(), json!(cover));
    }
    if let Some(artist) = cell(&track.cells, &["худ", "artist"]) {
        fields.insert("artist_register".into(), json!(artist));
    }
    let mut flags = Vec::new();
    if track.peak {
        flags.push(json!("peak"));
    }
    if track.vocal {
        flags.push(json!("vocalise"));
    }
    if !flags.is_empty() {
        fields.insert("slot_flags".into(), Value::Array(flags));
    }
    if let Some(playlists) = cell(&track.cells, &["плейлист", "playlist"]) {
        release.insert("playlists".into(), json!(playlists));
    }
    (fields, release)
}

/// A pack's title out of its heading: `<channel> – <title> (<formula>)`.
fn pack_title(heading: &str) -> String {
    let after = heading
        .split_once(" – ")
        .or_else(|| heading.split_once(" - "))
        .map_or(heading, |(_, rest)| rest);
    let title = match after.rfind(" (") {
        Some(at) if after.ends_with(')') => &after[..at],
        _ => after,
    };
    title.trim().to_owned()
}

/// A block of a pack by the words of its heading.
fn block<'a>(blocks: &'a [(String, String)], words: &[&str]) -> Option<&'a str> {
    blocks
        .iter()
        .find(|(heading, _)| {
            let heading = heading.to_lowercase();
            words.iter().any(|word| heading.contains(word))
        })
        .map(|(_, body)| body.as_str())
        .filter(|body| !body.is_empty())
}

/// The plan of bringing `channel` into the space `conn` holds.
pub fn plan(conn: &Connection, profile_id: &str, channel: &Channel) -> Result<Plan> {
    let mut out = Plan::default();
    passport(&channel.readme, &mut out);
    out.terms = channel
        .patterns
        .iter()
        .map(|one| {
            (
                one.pattern.trim_matches(['«', '»', '"']).trim().to_owned(),
                one.rule.clone(),
            )
        })
        .filter(|(word, _)| !word.is_empty())
        .collect();

    // The bank: each tradition of the tables, and the last month each was
    // taken in; a taken one the tables do not name joins as its own item.
    let last_taken = |name: &str| {
        channel
            .taken
            .iter()
            .filter(|(_, names)| {
                names
                    .iter()
                    .any(|one| one.to_lowercase().starts_with(&name.to_lowercase()))
            })
            .map(|(month, _)| month.clone())
            .max()
    };
    for one in &channel.traditions {
        out.traditions.push(BankItem {
            name: one.name.clone(),
            group: one.group.clone(),
            attributes: one.attributes.clone(),
            note: one.note.clone(),
            taken: last_taken(&one.name),
        });
    }
    for (month, names) in &channel.taken {
        for name in names {
            let known = out
                .traditions
                .iter()
                .any(|known| name.to_lowercase().starts_with(&known.name.to_lowercase()));
            if !known && !out.traditions.iter().any(|known| known.name == *name) {
                out.traditions.push(BankItem {
                    name: name.clone(),
                    taken: Some(month.clone()),
                    ..BankItem::default()
                });
            }
        }
    }

    let works = crate::score::catalogue(conn, profile_id)?;
    let existing = |title: &str| {
        works
            .iter()
            .find(|work| work.title.trim().to_lowercase() == title.trim().to_lowercase())
            .map(|work| work.work_id.clone())
    };
    for month in &channel.plan {
        let mut planned = Month {
            month: month.month.clone(),
            title: month.title.clone().unwrap_or_else(|| month.month.clone()),
            arc: month.arc.clone(),
            tracks: Vec::new(),
        };
        for track in &month.tracks {
            let (fields, release) = fields_of(track);
            planned.tracks.push(Track {
                date: track.date.clone(),
                title: track.title.clone(),
                existing: existing(&track.title),
                fields,
                release,
                ..Track::default()
            });
        }
        out.months.push(planned);
    }
    // The packs fill the plan's tracks of the same title: the prompt, the
    // cover, what the release goes out under.
    for pack in &channel.packs {
        let title = pack_title(&pack.heading);
        let found = out
            .months
            .iter_mut()
            .flat_map(|month| month.tracks.iter_mut())
            .find(|track| track.title.to_lowercase() == title.to_lowercase());
        let Some(track) = found else {
            out.conflicts.push(format!(
                "{} {title}: a pack track no plan row names",
                pack.date
            ));
            continue;
        };
        if track.date != pack.date {
            out.conflicts.push(format!(
                "{title}: planned for {}, packed for {}",
                track.date, pack.date
            ));
        }
        track.suno = block(&pack.blocks, &["suno"]).map(str::to_owned);
        track.cover = block(&pack.blocks, &["cover"]).map(str::to_owned);
        if let Some(name) = block(&pack.blocks, &["назван", "title"]) {
            track.release.insert("title".into(), json!(name));
        }
        if let Some(description) = block(&pack.blocks, &["описан", "description"]) {
            track
                .release
                .insert("description".into(), json!(description));
        }
        if let Some(playlists) = block(&pack.blocks, &["playlist", "плейлист"]) {
            let joined = statements(playlists).join(", ");
            track.release.insert("playlists".into(), json!(joined));
        }
        if let Some(duration) = block(&pack.blocks, &["продолж", "duration"])
            && let Some(first) = statements(duration).first()
        {
            track.fields.insert("duration".into(), json!(first));
        }
    }
    Ok(out)
}

/// The files of a channel's media named after its tracks: under
/// `<root>/<YYYY-MM>/<channel> - <YYYY-MM>/`, each file called as its pack
/// is - `<channel> – <title> (<formula>).<ext>`.
pub fn media(plan: &mut Plan, root: &Path, channel: &str) {
    for month in &mut plan.months {
        let dir = root
            .join(&month.month)
            .join(format!("{channel} - {}", month.month));
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect();
        files.sort();
        for file in files {
            let Some(stem) = file.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let title = pack_title(stem).to_lowercase();
            match month
                .tracks
                .iter_mut()
                .find(|track| track.title.to_lowercase() == title)
            {
                Some(track) => track.files.push(file.display().to_string()),
                None => plan.conflicts.push(format!(
                    "{}: a file no track of the month is named after",
                    file.display()
                )),
            }
        }
    }
}

/// The cooldown a bank states in its own words: the first `N месяц…` or
/// `N month…` of its page.
pub fn cooldown_of(text: &str) -> Option<u64> {
    let lower = text.to_lowercase();
    for marker in ["месяц", "month"] {
        if let Some(at) = lower.find(marker) {
            let before: String = lower[..at]
                .chars()
                .rev()
                .skip_while(|c| c.is_whitespace())
                .take_while(char::is_ascii_digit)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            if let Ok(months) = before.parse() {
                return Some(months);
            }
        }
    }
    None
}

/// What applying wrote.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Applied {
    pub facts: usize,
    pub terms: usize,
    pub traditions: usize,
    pub collections: usize,
    pub works: usize,
    pub releases: usize,
    pub files: usize,
}

/// Write the plan into the space, as one unit; the files of the tracks are
/// copied into `media`, the space's own folder of files.
pub fn apply(conn: &Connection, plan: &Plan) -> Result<Applied> {
    apply_with(conn, plan, None)
}

/// [`apply`], with the space's folder of files when the tracks bring some.
pub fn apply_with(conn: &Connection, plan: &Plan, media: Option<&Path>) -> Result<Applied> {
    use crate::actions;
    crate::db::unit::atomically(conn, |tx| {
        let profile_id = actions::active_profile_id(tx)?;
        let config = crate::profile::config_for(tx, &profile_id)?;
        let mut done = Applied::default();
        let root = match crate::note::root_card(tx, &profile_id, &config)? {
            Some(card) => card,
            None => actions::canon::seed_passport(tx, "Channel")?
                .ok_or_else(|| Error::refused("channel.noPassport"))?,
        };
        let held: Vec<(String, String)> = crate::canon::fact::for_card(tx, &root.id)?
            .into_iter()
            .map(|fact| (fact.section, fact.body))
            .collect();
        for fact in &plan.facts {
            if held
                .iter()
                .any(|(section, body)| *section == fact.section && *body == fact.body)
            {
                continue;
            }
            let mut data = Map::new();
            if let Some(slot) = &fact.slot {
                data.insert("slot".into(), json!(slot));
            }
            actions::canon::add_fact(
                tx,
                crate::canon::NewFact {
                    note_id: root.id.clone(),
                    section: fact.section.clone(),
                    body: fact.body.clone(),
                    data,
                    ..crate::canon::NewFact::default()
                },
            )?;
            done.facts += 1;
        }
        for (word, rule) in &plan.terms {
            if crate::register::find_word(tx, &profile_id, word, None)?.is_some() {
                continue;
            }
            actions::register::create(
                tx,
                crate::register::NewTerm {
                    word: word.clone(),
                    kind: Some(crate::register::TermKind::Pattern),
                    scope: crate::register::TermScope::VisualMotif,
                    note: Some(rule.clone()).filter(|rule| !rule.is_empty()),
                    ..crate::register::NewTerm::default()
                },
            )?;
            done.terms += 1;
        }
        for BankItem {
            name,
            group,
            attributes,
            note,
            taken,
        } in &plan.traditions
        {
            if held
                .iter()
                .any(|(section, body)| section == "bank" && body == name)
            {
                continue;
            }
            let mut data = Map::new();
            if !group.is_empty() {
                data.insert("group".into(), json!(group));
            }
            if !attributes.is_empty() {
                data.insert("attributes".into(), json!(attributes));
            }
            if !note.is_empty() {
                data.insert("note".into(), json!(note));
            }
            if let Some(months) = plan.cooldown_months {
                data.insert("cooldown_months".into(), json!(months));
            }
            if let Some(month) = taken {
                data.insert("last_used_at".into(), json!(format!("{month}-15")));
            }
            actions::canon::add_fact(
                tx,
                crate::canon::NewFact {
                    note_id: root.id.clone(),
                    section: "bank".into(),
                    body: name.clone(),
                    data,
                    ..crate::canon::NewFact::default()
                },
            )?;
            done.traditions += 1;
        }
        let collections = crate::collection::list(tx, &profile_id)?;
        for month in &plan.months {
            let gathered = match collections.iter().find(|one| one.title == month.title) {
                Some(found) => found.id.clone(),
                None => {
                    done.collections += 1;
                    actions::collection::create(
                        tx,
                        crate::collection::NewCollection {
                            kind: "month_pack".into(),
                            title: month.title.clone(),
                            description: month.arc.clone(),
                            meta: Some(
                                [("month".to_owned(), json!(month.month))]
                                    .into_iter()
                                    .collect(),
                            ),
                            target_size: None,
                            due_on: None,
                        },
                    )?
                    .id
                }
            };
            for track in &month.tracks {
                if track.existing.is_some() {
                    continue;
                }
                let work = actions::work::create(
                    tx,
                    crate::work::NewWork {
                        kind: "track".into(),
                        title: track.title.clone(),
                        collection_id: Some(gathered.clone()),
                        meta: Some(track.fields.clone()),
                        ..crate::work::NewWork::default()
                    },
                )?;
                done.works += 1;
                if let Some(suno) = &track.suno {
                    actions::version::create(
                        tx,
                        &work.id,
                        crate::work::version::NewVersion {
                            role: "suno_prompt".into(),
                            body: suno.clone(),
                            ..crate::work::version::NewVersion::default()
                        },
                    )?;
                }
                if let Some(cover) = &track.cover {
                    let mut drawn = work.cover.clone();
                    drawn.scene = cover.clone();
                    actions::work::update(
                        tx,
                        &work.id,
                        crate::work::WorkPatch {
                            cover: Some(drawn),
                            ..crate::work::WorkPatch::default()
                        },
                    )?;
                }
                if let Some(media) = media {
                    for file in &track.files {
                        actions::asset::attach(
                            tx,
                            media,
                            file,
                            crate::asset::NewAsset {
                                work_id: Some(work.id.clone()),
                                ..crate::asset::NewAsset::default()
                            },
                        )?;
                        done.files += 1;
                    }
                }
                if track.release.contains_key("title") {
                    actions::release::create(
                        tx,
                        crate::release::NewRelease {
                            work_id: work.id.clone(),
                            kind: "youtube".into(),
                            scheduled_at: Some(track.date.clone()),
                            meta: Some(track.release.clone()),
                            scheduled_time: None,
                            time_zone: None,
                        },
                    )?;
                    done.releases += 1;
                } else {
                    actions::work_plan::plan(
                        tx,
                        crate::work_plan::NewWorkPlan {
                            work_id: work.id.clone(),
                            scheduled_at: track.date.clone(),
                            note: None,
                        },
                    )?;
                }
            }
        }
        Ok(done)
    })
}

/// Read the channel's folder against a space and write the plan and its
/// report; with `apply_it`, keep a copy of the space and apply. The folder
/// the report is written to comes back.
pub fn run(
    app_dir: &Path,
    space: Option<&str>,
    dir: &Path,
    media_root: Option<&Path>,
    apply_it: bool,
) -> Result<PathBuf> {
    use crate::space::registry;
    let channel = read::read(dir)?;
    let app = crate::db::open_app(&app_dir.join(crate::db::APP_DATABASE_FILE))?;
    let found = match space {
        Some(name) => registry::by_name(&app, app_dir, name)?,
        None => registry::newest(&app, app_dir)?.ok_or_else(|| Error::refused("space.none"))?,
    };
    let folder = registry::folder(&app, app_dir, &found.id)?;
    let (conn, held_by) = crate::state::open_space(&folder, crate::state::Role::Companion)?;
    if apply_it && held_by.is_some() {
        return Err(Error::refused("space.unreachable").param("path", found.path.clone()));
    }
    let profile_id = crate::actions::active_profile_id(&conn)?;
    let config = crate::profile::config_for(&conn, &profile_id)?;
    if config.kind("track").is_none() {
        return Err(Error::refused("channel.notAChannelSpace").param("space", found.name.clone()));
    }
    let mut planned = plan(&conn, &profile_id, &channel)?;
    planned.cooldown_months = std::fs::read_to_string(dir.join("Банк традиций.md"))
        .ok()
        .and_then(|text| cooldown_of(text.lines().take(8).collect::<Vec<_>>().join("\n").as_str()));
    if let Some(root) = media_root {
        let channel_name = dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        media(&mut planned, root, &channel_name);
    }
    let moment = crate::time::now().replace([':', '.'], "-");
    let out = app_dir.join("import-channel").join(&moment);
    let applied = if apply_it {
        crate::exchange::backup::write_database(&conn, &out.join("kilna.before-import.db"))?;
        let files = crate::state::media_dir_beside(&folder.database())?;
        Some(apply_with(&conn, &planned, Some(&files))?)
    } else {
        None
    };
    report(&out, &planned, applied.as_ref())?;
    Ok(out)
}

/// The plan as `import_plan.json` and a page, `import_plan.md`.
fn report(dir: &Path, plan: &Plan, applied: Option<&Applied>) -> Result<()> {
    use std::fmt::Write as _;
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        dir.join("import_plan.json"),
        serde_json::to_string_pretty(&json!({ "plan": plan, "applied": applied }))?,
    )?;
    let tracks: usize = plan.months.iter().map(|month| month.tracks.len()).sum();
    let packed = plan
        .months
        .iter()
        .flat_map(|month| &month.tracks)
        .filter(|track| track.suno.is_some())
        .count();
    let mut page = String::from("# A channel brought in\n\n");
    let _ = writeln!(
        page,
        "- passport facts: {}\n- rules to write as quotas: {}\n- register terms: {}\n- bank items: {} (rest {} months)\n- months: {}\n- tracks: {tracks} ({packed} with a pack)\n- conflicts: {}\n",
        plan.facts.len(),
        plan.rules.len(),
        plan.terms.len(),
        plan.traditions.len(),
        plan.cooldown_months
            .map_or_else(|| "-".into(), |m| m.to_string()),
        plan.months.len(),
        plan.conflicts.len(),
    );
    if !plan.rules.is_empty() {
        page.push_str("## Rules to write as quotas\n\n");
        for rule in &plan.rules {
            let _ = writeln!(page, "- {rule}");
        }
        page.push('\n');
    }
    if !plan.conflicts.is_empty() {
        page.push_str("## Conflicts\n\n");
        for conflict in &plan.conflicts {
            let _ = writeln!(page, "- {conflict}");
        }
        page.push('\n');
    }
    match applied {
        Some(done) => {
            let _ = writeln!(
                page,
                "## Applied\n\nfacts {}, terms {}, bank items {}, collections {}, works {}, releases {}",
                done.facts,
                done.terms,
                done.traditions,
                done.collections,
                done.works,
                done.releases
            );
        }
        None => page.push_str("Nothing was written: run again with `--apply`.\n"),
    }
    std::fs::write(dir.join("import_plan.md"), page)?;
    Ok(())
}

#[cfg(test)]
mod tests;
