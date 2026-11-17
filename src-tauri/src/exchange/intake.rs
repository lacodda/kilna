//! Works coming in from files and from the clipboard (v0.113).
//!
//! Three things can be handed over, and each becomes a
//! [parcel](super::parcel) before it lands:
//!
//! * a page kilna wrote - a work's page of the export, or "Save as
//!   Markdown" - which carries its own parcel and comes back as the work it
//!   was: its kind, its versions with their numbers, its scores, its release;
//! * a `.kilna.zip`, which is the parcel with the work's files beside it;
//! * a text from outside - a `.md`, a `.txt`, what is on the clipboard -
//!   which says nothing but a title and a body, and becomes a work of the
//!   kind the person names, with the text as its first version.
//!
//! A folder is its files: the pages and texts and parcels that lie in it,
//! not what lies in the folders under it. A folder of the export holds
//! `works/` beside pages that are no works - the notes, the register, the
//! chats - and reading it through would have made a work of each.
//!
//! Nothing here merges. A work that is already in the space - the same work
//! by its id, or for a text from outside one of the same title - is passed
//! over and said so, and so is one the person threw away: bringing works in
//! only ever adds.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::export::{BODY_END, BODY_OPEN, PARCEL_OPEN};
use super::parcel::{self, Parcel, ParcelStanding, Zip};
use crate::error::{Error, Reason, Result};
use crate::profile::config::ProfileConfig;

/// What the person handed over, and what they said about it.
#[derive(Debug, Clone, Default, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields)]
pub struct IntakeRequest {
    /// Files and folders.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub paths: Vec<String>,
    /// A text that came with no file: the clipboard's.
    #[serde(default)]
    pub text: Option<String>,
    /// The kind a text from outside becomes. A page of kilna's and a parcel
    /// say their own.
    #[serde(default)]
    pub kind: Option<String>,
    /// What to call the text that came with no file, when the person said.
    #[serde(default)]
    pub title: Option<String>,
}

/// One thing found in what was handed over, before anything is written.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct IntakeFound {
    /// The file's name, or nothing for the text that came with none.
    pub name: Option<String>,
    pub title: String,
    /// The kind it says it is; nothing for a text from outside, which
    /// becomes what the person names.
    pub kind: Option<String>,
    pub versions: usize,
    pub scores: usize,
    /// Files it would bring with it.
    pub files: usize,
    /// Where it stands to this space; nothing when it cannot be read.
    pub standing: Option<ParcelStanding>,
    /// Why it cannot be read or cannot land here.
    pub reason: Option<Reason>,
}

/// What bringing works in did.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct IntakeBrought {
    pub works: Vec<parcel::LandedWork>,
    /// Passed over as already here.
    pub here: usize,
    /// Passed over as thrown away here.
    pub deleted: usize,
    /// Could not be read or could not land, one by one.
    pub refused: Vec<IntakeRefused>,
}

/// One thing that was not brought in, and why.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct IntakeRefused {
    pub name: Option<String>,
    pub reason: Reason,
}

/// A text as it was read: a page of kilna's, or a text from outside.
#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    Whole(Box<Parcel>),
    Plain(Plain),
}

/// What a text from outside says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plain {
    /// From its front matter or its first heading; nothing when it has
    /// neither, and the file's name is what it is called.
    pub title: Option<String>,
    pub body: String,
    pub tags: Vec<String>,
}

/// Read a text: a page of kilna's by the parcel at its foot, anything else
/// as a text from outside.
pub fn read(text: &str) -> Result<Page> {
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let lines: Vec<&str> = text.split('\n').collect();

    if let Some(at) = lines.iter().rposition(|line| *line == PARCEL_OPEN) {
        let json = lines
            .get(at + 1)
            .ok_or_else(|| Error::refused("intake.brokenPage"))?;
        let mut parcel: Parcel = serde_json::from_str(json).map_err(|cause| {
            Error::refused("intake.brokenParcel").param("cause", cause.to_string())
        })?;
        fill_bodies(&mut parcel, &lines[..at]);
        return Ok(Page::Whole(Box::new(parcel)));
    }

    let (front, rest) = front_matter(&lines);
    // A page of the export that is no work - a chat, a card of the canon -
    // or a work's page written before a page could be read back.
    if front.iter().any(|(key, _)| key == "format") {
        return Err(Error::refused("intake.notAWorkPage"));
    }
    let mut title = front
        .iter()
        .find(|(key, _)| key == "title")
        .map(|(_, value)| value.clone())
        .filter(|title| !title.is_empty());
    let tags = front
        .iter()
        .find(|(key, _)| key == "tags")
        .map(|(_, value)| list_of(value))
        .unwrap_or_default();

    // A heading that opens the text is what it is called, not its first
    // line: `# Title` over a poem is the page's name.
    let mut body: &[&str] = rest;
    while body.first().is_some_and(|line| line.trim().is_empty()) {
        body = &body[1..];
    }
    if let Some(heading) = body.first().and_then(|line| line.strip_prefix("# ")) {
        if title.is_none() {
            title = Some(heading.trim().to_owned()).filter(|title| !title.is_empty());
        }
        body = &body[1..];
    }
    let body = body.join("\n").trim_matches('\n').to_owned();

    if body.trim().is_empty() && title.is_none() {
        return Err(Error::refused("intake.empty"));
    }
    Ok(Page::Plain(Plain { title, body, tags }))
}

/// Put each version's body back from between its marks on the page.
fn fill_bodies(parcel: &mut Parcel, lines: &[&str]) {
    let mut at = 0;
    while at < lines.len() {
        let Some((id, count)) = body_mark(lines[at]) else {
            at += 1;
            continue;
        };
        let from = at + 1;
        // As it was written: exactly the lines the mark counted, closed by
        // the closing mark - so a body may hold a line that reads as one.
        let counted = count
            .map(|count| from + count)
            .filter(|end| lines.get(*end).is_some_and(|line| *line == BODY_END));
        // Changed outside: up to the first closing mark, or the page's end.
        let end = counted.unwrap_or_else(|| {
            lines[from..]
                .iter()
                .position(|line| *line == BODY_END)
                .map_or(lines.len(), |offset| from + offset)
        });
        if let Some(version) = parcel.versions.iter_mut().find(|one| one.id == id) {
            version.body = lines[from..end].join("\n");
        }
        at = end + 1;
    }
}

/// A line that opens a body: the version's id, and the lines it counted.
fn body_mark(line: &str) -> Option<(&str, Option<usize>)> {
    let inside = line.strip_prefix(BODY_OPEN)?.strip_suffix("-->")?.trim();
    let mut words = inside.split_whitespace();
    let id = words.next()?;
    let count = words
        .find_map(|word| word.strip_prefix("lines="))
        .and_then(|count| count.parse().ok());
    Some((id, count))
}

/// The front matter of a text as its `key: value` lines, and what follows.
/// A value keeps no quotes; a list written under its key, an item a line,
/// is joined with commas.
fn front_matter<'a, 'b>(lines: &'a [&'b str]) -> (Vec<(String, String)>, &'a [&'b str]) {
    if lines.first().map(|line| line.trim_end()) != Some("---") {
        return (Vec::new(), lines);
    }
    let Some(close) = lines[1..].iter().position(|line| line.trim_end() == "---") else {
        return (Vec::new(), lines);
    };
    let mut pairs: Vec<(String, String)> = Vec::new();
    for line in &lines[1..=close] {
        // An item of the list the key above opened.
        if let Some(item) = line.trim_start().strip_prefix("- ") {
            if let Some((_, value)) = pairs.last_mut() {
                if !value.is_empty() {
                    value.push(',');
                }
                value.push_str(unquoted(item));
            }
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            pairs.push((key.trim().to_owned(), unquoted(value).to_owned()));
        }
    }
    (pairs, &lines[close + 2..])
}

fn unquoted(value: &str) -> &str {
    let value = value.trim();
    for quote in ['"', '\''] {
        if value.len() >= 2 && value.starts_with(quote) && value.ends_with(quote) {
            return &value[1..value.len() - 1];
        }
    }
    value
}

/// A list as front matter writes one on a line: `[a, b]` or `a, b`.
fn list_of(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|item| unquoted(item).trim_start_matches('#').to_owned())
        .filter(|item| !item.is_empty())
        .collect()
}

/// What a text with no name is called: its first line that says anything,
/// without the marks of a heading, as far as a title goes.
fn first_line(body: &str) -> Option<String> {
    let line = body.lines().map(str::trim).find(|line| !line.is_empty())?;
    let line = line.trim_start_matches('#').trim();
    let cut: String = line.chars().take(80).collect();
    Some(cut.trim().to_owned()).filter(|title| !title.is_empty())
}

/// The role a text from outside lands in: the first of the kind's roles
/// that is a draft of the work.
fn text_role<'c>(config: &'c ProfileConfig, kind: &str) -> Result<&'c str> {
    let Some(found) = config.kind(kind) else {
        return Err(Error::refused("parcel.unknownKind").param("kind", kind));
    };
    found
        .version_roles
        .iter()
        .find(|role| role.counts_as_a_version())
        .map(|role| role.key.as_str())
        .ok_or_else(|| Error::refused("intake.kindHasNoText").param("kind", kind))
}

/// The files a path stands for: itself, or for a folder the pages, texts and
/// parcels that lie in it, by name.
fn files_of(path: &Path) -> Result<Vec<PathBuf>> {
    if !path.is_dir() {
        return Ok(vec![path.to_owned()]);
    }
    let mut found: Vec<PathBuf> = std::fs::read_dir(path)?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && (is_text(path) || is_parcel(path)))
        .collect();
    found.sort();
    Ok(found)
}

fn is_text(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ["md", "markdown", "txt"]
                .iter()
                .any(|known| ext.eq_ignore_ascii_case(known))
        })
}

fn is_parcel(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
}

/// One thing handed over, read.
struct Item {
    name: Option<String>,
    loaded: Result<Loaded>,
}

/// What was read of it.
enum Loaded {
    /// A parcel with its files.
    Zip(Box<Parcel>, Box<Zip>),
    /// A page of kilna's.
    Whole(Box<Parcel>),
    /// A text from outside, and what it is called when it does not say.
    Plain(Plain, Option<String>),
}

/// Read everything the request names, in the order it names it.
fn items(request: &IntakeRequest) -> Result<Vec<Item>> {
    let mut found = Vec::new();
    for path in &request.paths {
        for file in files_of(Path::new(path))? {
            let name = file
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            let stem = file
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned());
            let loaded = if is_parcel(&file) {
                Zip::open(&file).map(|(zip, parcel)| Loaded::Zip(Box::new(parcel), Box::new(zip)))
            } else {
                std::fs::read(&file)
                    .map_err(|cause| {
                        Error::refused("intake.notRead").param("cause", cause.to_string())
                    })
                    .and_then(|bytes| {
                        String::from_utf8(bytes).map_err(|_| Error::refused("intake.notText"))
                    })
                    .and_then(|text| read(&text))
                    .map(|page| match page {
                        Page::Whole(parcel) => Loaded::Whole(parcel),
                        Page::Plain(plain) => Loaded::Plain(plain, stem),
                    })
            };
            found.push(Item { name, loaded });
        }
    }
    if let Some(text) = request
        .text
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        let loaded = read(text).map(|page| match page {
            Page::Whole(parcel) => Loaded::Whole(parcel),
            Page::Plain(plain) => {
                let named = request
                    .title
                    .as_deref()
                    .map(str::trim)
                    .filter(|title| !title.is_empty())
                    .map(str::to_owned)
                    .or_else(|| first_line(&plain.body));
                Loaded::Plain(plain, named)
            }
        });
        found.push(Item { name: None, loaded });
    }
    Ok(found)
}

/// What a text from outside is called: what it says, or what it was named.
fn title_of(plain: &Plain, named: Option<&str>) -> Result<String> {
    plain
        .title
        .clone()
        .or_else(|| named.map(str::to_owned))
        .map(|title| title.trim().to_owned())
        .filter(|title| !title.is_empty())
        .ok_or_else(|| Error::refused("parcel.needsTitle"))
}

/// The parcel a text from outside becomes, in the kind the person named.
fn parcel_of(
    config: &ProfileConfig,
    plain: &Plain,
    named: Option<&str>,
    kind: Option<&str>,
) -> Result<Parcel> {
    let kind = kind.ok_or_else(|| Error::refused("intake.needsKind"))?;
    let role = text_role(config, kind)?;
    Ok(Parcel::of_text(
        kind,
        role,
        &title_of(plain, named)?,
        &plain.body,
        plain.tags.clone(),
    ))
}

/// Look at what was handed over and say what bringing it in would do.
/// Writes nothing.
pub fn survey(
    conn: &Connection,
    profile_id: &str,
    request: &IntakeRequest,
) -> Result<Vec<IntakeFound>> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let kind = request.kind.as_deref();
    let mut listed = Vec::new();
    for item in items(request)? {
        let name = item.name;
        let unread = |name: Option<String>, cause: &Error| IntakeFound {
            name,
            title: String::new(),
            kind: None,
            versions: 0,
            scores: 0,
            files: 0,
            standing: None,
            reason: Some(cause.reason()),
        };
        let found = match item.loaded {
            Err(cause) => unread(name, &cause),
            Ok(Loaded::Zip(parcel, _)) | Ok(Loaded::Whole(parcel)) => IntakeFound {
                name,
                title: parcel.work.title.clone(),
                kind: Some(parcel.work.kind.clone()),
                versions: parcel.versions.len(),
                scores: parcel.scores.len(),
                files: parcel.assets.len(),
                standing: Some(parcel::standing(conn, profile_id, &parcel, false)?),
                reason: parcel::check(&config, &parcel)
                    .err()
                    .map(|cause| cause.reason()),
            },
            Ok(Loaded::Plain(plain, named)) => match title_of(&plain, named.as_deref()) {
                Err(cause) => unread(name, &cause),
                Ok(title) => {
                    // Known by its title alone, whatever kind it becomes.
                    let probe = Parcel::of_text("", "", &title, "", Vec::new());
                    IntakeFound {
                        name,
                        title,
                        kind: None,
                        versions: usize::from(!plain.body.trim().is_empty()),
                        scores: 0,
                        files: 0,
                        standing: Some(parcel::standing(conn, profile_id, &probe, true)?),
                        // A kind that cannot take a text is said before the
                        // person agrees, not after.
                        reason: kind
                            .and_then(|kind| text_role(&config, kind).err())
                            .map(|cause| cause.reason()),
                    }
                }
            },
        };
        listed.push(found);
    }
    Ok(listed)
}

/// Bring in everything the request names that is not here yet, each work by
/// `land` - the gesture that puts one parcel into the space.
///
/// Each work is its own unit: one that cannot land is said with its reason
/// and the rest still arrive, as a batch does everywhere else in kilna.
pub fn bring(
    conn: &Connection,
    profile_id: &str,
    request: &IntakeRequest,
    mut land: impl FnMut(&Parcel, &mut dyn parcel::Files, bool) -> Result<parcel::LandedWork>,
) -> Result<IntakeBrought> {
    let config = crate::profile::config_for(conn, profile_id)?;
    let mut brought = IntakeBrought {
        works: Vec::new(),
        here: 0,
        deleted: 0,
        refused: Vec::new(),
    };
    for item in items(request)? {
        let name = item.name;
        let outcome = item.loaded.and_then(|loaded| {
            let (parcel, by_title, mut files): (Parcel, bool, Box<dyn parcel::Files>) = match loaded
            {
                Loaded::Zip(parcel, zip) => (*parcel, false, zip),
                Loaded::Whole(parcel) => (*parcel, false, Box::new(parcel::NoFiles)),
                Loaded::Plain(plain, named) => (
                    parcel_of(&config, &plain, named.as_deref(), request.kind.as_deref())?,
                    true,
                    Box::new(parcel::NoFiles),
                ),
            };
            match parcel::standing(conn, profile_id, &parcel, by_title)? {
                ParcelStanding::Here => Ok(Err(ParcelStanding::Here)),
                ParcelStanding::Deleted => Ok(Err(ParcelStanding::Deleted)),
                // A text from outside begins here; the rest is carried.
                ParcelStanding::New => land(&parcel, files.as_mut(), by_title).map(Ok),
            }
        });
        match outcome {
            Ok(Ok(landed)) => brought.works.push(landed),
            Ok(Err(ParcelStanding::Deleted)) => brought.deleted += 1,
            Ok(Err(_)) => brought.here += 1,
            Err(cause) => brought.refused.push(IntakeRefused {
                name,
                reason: cause.reason(),
            }),
        }
    }
    Ok(brought)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::score::NewScore;
    use crate::work::version::NewVersion;
    use crate::{actions, scene, scene_frame, score, work};
    use serde_json::json;

    /// Bring a request into a workspace the way the window's command does.
    fn bring_in(conn: &Connection, profile_id: &str, request: &IntakeRequest) -> IntakeBrought {
        let media = crate::state::media_dir_of(conn).ok();
        bring(conn, profile_id, request, |parcel, files, fresh| {
            actions::work::bring(conn, media.as_deref(), parcel, files, fresh)
        })
        .unwrap()
    }

    fn paths(path: &Path) -> IntakeRequest {
        IntakeRequest {
            paths: vec![path.display().to_string()],
            ..IntakeRequest::default()
        }
    }

    /// A song with everything a page says about one: two drafts in a line,
    /// a text of another role, two scores. The bodies hold what would break
    /// a reader that told a body from the page by its headings.
    fn a_song_with_a_history(conn: &Connection, profile_id: &str) -> String {
        let song = work::create(
            conn,
            profile_id,
            work::NewWork {
                kind: "song".into(),
                title: "Harbour: lights \"and\" tides".into(),
                tags: vec!["sea".into(), "night".into()],
                meta: json!({ "bpm": 96, "language": "en" }).as_object().cloned(),
                ..work::NewWork::default()
            },
        )
        .unwrap();
        let first = fixtures::version(
            conn,
            &song.id,
            "lyrics",
            "the first draft\n\n## Scores\n\n| not | a | table |\n",
        );
        work::version::create(
            conn,
            &song.id,
            NewVersion {
                role: "lyrics".into(),
                body: "### Revision 9 (current)\n<!-- kilna:end -->\nstill the body --> here"
                    .into(),
                label: Some("second pass".into()),
                parent_version_id: Some(first.id.clone()),
                ..NewVersion::default()
            },
        )
        .unwrap();
        fixtures::version(conn, &song.id, "style", "indie rock, female vocal");
        fixtures::score(conn, &song.id, json!({ "hook": 7.0 }));
        score::create(
            conn,
            &song.id,
            NewScore {
                axes: json!({ "hook": 9.0 }).as_object().cloned().unwrap(),
                version_id: Some(first.id),
                note: Some("sharp | and\nsecond line".into()),
                rater: Some("A critic".into()),
            },
        )
        .unwrap();
        // As a gesture leaves it: its status drawn from what it holds.
        actions::restate(conn, profile_id, &song.id);
        song.id
    }

    #[test]
    fn a_page_kilna_wrote_comes_back_as_the_work_it_was() {
        let (from, from_profile, _from_dir) = fixtures::workspace_on_disk();
        let (into, into_profile, _into_dir) = fixtures::workspace_on_disk();
        let song = a_song_with_a_history(&from, &from_profile);
        let page = crate::exchange::export::page_of(&from, &song).unwrap();

        // What the page says is the whole parcel, bodies included.
        let Page::Whole(read_back) = read(&page).unwrap() else {
            panic!("a page of kilna's reads as one");
        };
        assert_eq!(*read_back, parcel::of_work(&from, &song).unwrap());

        let brought = bring_in(
            &into,
            &into_profile,
            &IntakeRequest {
                text: Some(page.clone()),
                ..IntakeRequest::default()
            },
        );
        assert_eq!(brought.refused, Vec::new());
        assert_eq!(brought.works.len(), 1);
        assert_eq!(brought.works[0].id, song);
        assert_eq!(brought.works[0].versions, 3);
        assert_eq!(brought.works[0].scores, 2);

        // The round trip: the page the second space writes for the work is
        // the page it was brought in from, to the byte.
        assert_eq!(
            crate::exchange::export::page_of(&into, &song).unwrap(),
            page
        );
    }

    #[test]
    fn a_body_changed_outside_is_what_comes_back() {
        let (from, from_profile, _dir) = fixtures::workspace_on_disk();
        let song = a_song_with_a_history(&from, &from_profile);
        let page = crate::exchange::export::page_of(&from, &song).unwrap();
        let edited = page.replace("indie rock, female vocal", "slow waltz\nbrushed drums");
        assert_ne!(edited, page);

        let Page::Whole(read_back) = read(&edited).unwrap() else {
            panic!("a page of kilna's reads as one");
        };
        let style = read_back
            .versions
            .iter()
            .find(|one| one.role == "style")
            .unwrap();
        assert_eq!(style.body, "slow waltz\nbrushed drums");
        // And the one beside it, whose body holds a closing mark, is whole.
        assert!(
            read_back
                .versions
                .iter()
                .any(|one| one.body.ends_with("still the body --> here"))
        );
    }

    #[test]
    fn a_work_brought_twice_is_passed_over_and_one_thrown_away_stays_away() {
        let (from, from_profile, _from_dir) = fixtures::workspace_on_disk();
        let (into, into_profile, _into_dir) = fixtures::workspace_on_disk();
        let song = a_song_with_a_history(&from, &from_profile);
        let request = IntakeRequest {
            text: Some(crate::exchange::export::page_of(&from, &song).unwrap()),
            ..IntakeRequest::default()
        };

        assert_eq!(bring_in(&into, &into_profile, &request).works.len(), 1);
        let again = bring_in(&into, &into_profile, &request);
        assert_eq!((again.works.len(), again.here, again.deleted), (0, 1, 0));
        assert_eq!(
            survey(&into, &into_profile, &request).unwrap()[0].standing,
            Some(ParcelStanding::Here)
        );

        // Bringing a work in is one gesture, and taking it back is the
        // trash - where a second arrival does not dig it out.
        assert!(crate::undo::reversible("work.bring"));
        actions::work::discard(&into, std::slice::from_ref(&song)).unwrap();
        let after = bring_in(&into, &into_profile, &request);
        assert_eq!((after.works.len(), after.here, after.deleted), (0, 0, 1));
    }

    #[test]
    fn a_parcel_carries_a_board_with_its_pictures_to_another_space() {
        let (from, from_profile, from_dir) = fixtures::workspace_on_disk();
        let (into, into_profile, _into_dir) = fixtures::workspace_on_disk();
        let media = crate::state::media_dir_of(&from).unwrap();
        let video = fixtures::video(&from, &from_profile, "Harbour lights, the clip");
        fixtures::version(&from, &video.id, "plot", "a boat leaves at dawn");
        let opening = scene::create(
            &from,
            &from_profile,
            scene::NewScene {
                work_id: video.id.clone(),
                position: None,
                section: Some("intro".into()),
                starts_at: Some(0.0),
                ends_at: Some(4.5),
                shot_type: None,
                layer: None,
                description: Some("the quay, empty".into()),
                blocks: None,
            },
        )
        .unwrap();
        let still = fixtures::file(from_dir.path(), "scene-01-still.png");
        let frame =
            scene_frame::attach(&from, &media, &opening.id, scene_frame::FRAME, &still).unwrap();
        scene_frame::select(&from, &frame.id).unwrap();
        fixtures::release(&from, &video.id, "youtube", Some("2026-09-03"));
        actions::restate(&from, &from_profile, &video.id);

        let zip = from_dir
            .path()
            .join(parcel::file_name(&from, &video.id).unwrap());
        let packed = parcel::write_zip(&from, &video.id, &zip).unwrap();
        assert_eq!((packed.files, packed.missing), (1, 0));

        let found = survey(&into, &into_profile, &paths(&zip)).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind.as_deref(), Some("video"));
        assert_eq!(
            (found[0].files, found[0].standing),
            (1, Some(ParcelStanding::New))
        );

        let brought = bring_in(&into, &into_profile, &paths(&zip));
        assert_eq!(brought.refused, Vec::new());
        assert_eq!(brought.works.len(), 1);
        assert_eq!((brought.works[0].files, brought.works[0].missing), (1, 0));

        // The board came with its picture: the same frame, chosen, and its
        // bytes in the second space's own folder.
        let frames = scene_frame::for_work(&into, &video.id).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].id, frame.id);
        assert!(frames[0].is_selected);
        assert_eq!(
            frames[0].original_name.as_deref(),
            Some("scene-01-still.png")
        );
        let landed = Path::new(&frames[0].path);
        // Against the folder as the space names it: a temporary directory
        // has two spellings on macOS.
        let own = crate::state::media_dir_of(&into).unwrap();
        assert!(landed.starts_with(&own), "{}", landed.display());
        assert_eq!(
            std::fs::read(landed).unwrap(),
            std::fs::read(&still).unwrap()
        );
        // And everything else a parcel names, as it stood.
        assert_eq!(
            parcel::of_work(&into, &video.id).unwrap(),
            parcel::of_work(&from, &video.id).unwrap()
        );
    }

    #[test]
    fn a_folder_of_texts_becomes_works_of_the_kind_named() {
        let (conn, profile_id, dir) = fixtures::workspace_on_disk();
        let folder = dir.path().join("drafts");
        std::fs::create_dir_all(folder.join("deeper")).unwrap();
        std::fs::write(
            folder.join("a-heading.md"),
            "\u{feff}# Harbour lights\r\n\r\nthe tide is out\r\nthe boats lean\r\n",
        )
        .unwrap();
        std::fs::write(folder.join("b-plain.txt"), "no heading here\nonly lines\n").unwrap();
        std::fs::write(
            folder.join("c-front.md"),
            "---\ntitle: \"Named above\"\ntags:\n  - sea\n  - '#night'\n---\n\nthe body\n",
        )
        .unwrap();
        // A page of the export that is no work, a picture, and a text one
        // folder down: none of them a work.
        std::fs::write(
            folder.join("d-chat.md"),
            "---\nformat: 8\nchat: \"x\"\n---\n\n# x\n",
        )
        .unwrap();
        std::fs::write(folder.join("e.png"), b"png").unwrap();
        std::fs::write(folder.join("deeper").join("f.md"), "# Deeper\n\nnot read\n").unwrap();

        let mut request = paths(&folder);
        // No kind named yet: seen, but nothing can land.
        let unnamed = bring_in(&conn, &profile_id, &request);
        assert_eq!(unnamed.works.len(), 0);
        assert_eq!(unnamed.refused.len(), 4);
        assert_eq!(unnamed.refused[0].reason.key, "refusal.intake.needsKind");

        request.kind = Some("song".into());
        let found = survey(&conn, &profile_id, &request).unwrap();
        let titles: Vec<&str> = found.iter().map(|one| one.title.as_str()).collect();
        assert_eq!(titles, ["Harbour lights", "b-plain", "Named above", ""]);
        assert_eq!(
            found[3].reason.as_ref().map(|reason| reason.key.as_str()),
            Some("refusal.intake.notAWorkPage")
        );

        let brought = bring_in(&conn, &profile_id, &request);
        assert_eq!(brought.works.len(), 3);
        assert_eq!(brought.refused.len(), 1);
        let works = work::list(&conn, &profile_id, &work::WorkFilter::default()).unwrap();
        let harbour = works
            .iter()
            .find(|one| one.title == "Harbour lights")
            .unwrap();
        assert_eq!(harbour.kind, "song");
        let text = work::version::get(&conn, harbour.current_version_id.as_deref().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!((text.role.as_str(), text.revision), ("lyrics", 1));
        assert_eq!(text.body, "the tide is out\nthe boats lean");
        // It begins here, so it starts as a work made here does: with the
        // fields its kind starts at.
        let made_here = fixtures::song(&conn, &profile_id, "Made by hand");
        let keys = |one: &work::Work| one.meta.keys().cloned().collect::<Vec<_>>();
        assert_eq!(keys(harbour), keys(&made_here));
        let named = works.iter().find(|one| one.title == "Named above").unwrap();
        assert_eq!(named.tags, ["sea", "night"]);

        // A second pass over the same folder brings nothing twice.
        let again = bring_in(&conn, &profile_id, &request);
        assert_eq!((again.works.len(), again.here), (0, 3));
    }

    #[test]
    fn a_pasted_text_is_called_by_its_first_line_unless_told_otherwise() {
        let (conn, profile_id, _dir) = fixtures::workspace_on_disk();
        let mut request = IntakeRequest {
            text: Some("\n  the tide is out\nthe boats lean\n".into()),
            kind: Some("song".into()),
            ..IntakeRequest::default()
        };
        assert_eq!(
            survey(&conn, &profile_id, &request).unwrap()[0].title,
            "the tide is out"
        );
        request.title = Some("  Low water ".into());
        let brought = bring_in(&conn, &profile_id, &request);
        assert_eq!(brought.works[0].title, "Low water");
        // The first line is the text's own: a title given beside it takes
        // nothing out of the body.
        let made = work::get(&conn, &brought.works[0].id).unwrap().unwrap();
        let text = work::version::get(&conn, made.current_version_id.as_deref().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(text.body, "  the tide is out\nthe boats lean");
    }
}
