//! What the person decides about the assistant (v0.108): how many runs may go
//! at once, which model they start with, and what they may read beyond what the
//! prompt carries.
//!
//! One typed row, beside the week's budget ([`super::spend`]), in the
//! application's database since spaces (ADR 0077): these say what this machine
//! may be asked to carry and what a program on it may read, the same whatever
//! space is open, and nobody else's business. Every function here takes the
//! application's connection, not a space's.

use std::path::PathBuf;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use super::Chat;
use crate::error::{Error, Result};
use crate::time::now;

/// How many runs may go at once when nobody said otherwise.
///
/// Each CLI costs a few hundred megabytes, so this is a memory budget rather
/// than a throughput knob. Three covers the way the panel is actually used -
/// ask in one chat, move to another while it thinks - without putting a working
/// machine under pressure.
pub const DEFAULT_PARALLEL_RUNS: usize = 3;

/// The most runs a person can ask for. The schema holds the same number.
pub const MOST_PARALLEL_RUNS: usize = 8;

/// The longest model name kept. The schema holds the same number.
const LONGEST_MODEL: usize = 100;

/// How many attached files a run is told about by name. A work with more has
/// the rest in the same folder, and a brief of three hundred paths is a brief
/// nobody reads.
const MOST_FILES_NAMED: usize = 40;

/// What a run may read beyond what its prompt carries (ADR 0074).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub enum RunSees {
    /// Only what the prompt carries: it starts in an empty directory.
    Nothing,
    /// The files of the work its chat is about, as well: the work's folder on
    /// disk and what is attached to it.
    Work,
}

impl RunSees {
    fn as_str(self) -> &'static str {
        match self {
            Self::Nothing => "nothing",
            Self::Work => "work",
        }
    }

    /// Read from its column. The schema holds it to the two words, so anything
    /// else is a row nobody wrote - and reading that as the narrower answer
    /// is the mistake that costs nothing.
    fn from_str(raw: &str) -> Self {
        match raw {
            "work" => Self::Work,
            _ => Self::Nothing,
        }
    }
}

/// The assistant's settings, as they stand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct AssistantSettings {
    /// How many runs may go at once.
    #[ts(type = "number")]
    pub parallel_runs: usize,
    /// The model a run starts with; none is what Claude Code itself is set to.
    pub model: Option<String>,
    pub run_sees: RunSees,
}

impl Default for AssistantSettings {
    fn default() -> Self {
        Self {
            parallel_runs: DEFAULT_PARALLEL_RUNS,
            model: None,
            run_sees: RunSees::Nothing,
        }
    }
}

/// The settings as they stand: the defaults, until something was set.
pub fn read(conn: &Connection) -> Result<AssistantSettings> {
    Ok(conn
        .query_row(
            "SELECT parallel_runs, model, run_sees FROM assistant_settings WHERE id = 1",
            [],
            |row| {
                Ok(AssistantSettings {
                    parallel_runs: usize::try_from(row.get::<_, i64>(0)?)
                        .unwrap_or(DEFAULT_PARALLEL_RUNS),
                    model: row.get(1)?,
                    run_sees: RunSees::from_str(&row.get::<_, String>(2)?),
                })
            },
        )
        .optional()?
        .unwrap_or_default())
}

/// Set how many runs may go at once.
///
/// Refused rather than held to the bounds: a person who typed 20 should be told
/// it is 8, not find out later. What is already running is left to finish; the
/// caller tells the registry, which starts nothing more until there is room.
pub fn set_parallel_runs(conn: &Connection, runs: usize) -> Result<AssistantSettings> {
    if !(1..=MOST_PARALLEL_RUNS).contains(&runs) {
        return Err(Error::refused("assistant.badParallelRuns")
            .param("most", MOST_PARALLEL_RUNS)
            .param("value", runs));
    }
    write(
        conn,
        "parallel_runs",
        i64::try_from(runs).unwrap_or(i64::MAX),
    )?;
    read(conn)
}

/// Set the model a run starts with, or clear it with nothing.
///
/// Only the look of a name is checked: the CLI knows which models exist, the
/// list changes under it, and a name it does not know is answered in the chat
/// in its own words. What is checked is what would reach a command line: on
/// Windows the executable is a `.cmd`, and a name is an argument of it.
pub fn set_model(conn: &Connection, model: Option<&str>) -> Result<AssistantSettings> {
    let model = model.map(str::trim).filter(|name| !name.is_empty());
    if let Some(name) = model
        && !well_formed_model(name)
    {
        return Err(Error::refused("assistant.badModel").param("name", name));
    }
    write(conn, "model", model)?;
    read(conn)
}

/// Whether `name` looks like what `--model` takes: an alias or a full name,
/// `opus` or `claude-opus-5-5[1m]`.
fn well_formed_model(name: &str) -> bool {
    name.len() <= LONGEST_MODEL
        && name.starts_with(|first: char| first.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ':' | '[' | ']'))
}

/// Set what a run may read.
pub fn set_run_sees(conn: &Connection, sees: RunSees) -> Result<AssistantSettings> {
    write(conn, "run_sees", sees.as_str())?;
    read(conn)
}

/// One column of the one row, made if it is not there yet.
fn write(conn: &Connection, column: &str, value: impl rusqlite::ToSql) -> Result<()> {
    conn.execute(
        &format!(
            "INSERT INTO assistant_settings (id, {column}, updated_at) VALUES (1, ?1, ?2)
             ON CONFLICT (id) DO UPDATE SET
                 {column} = excluded.{column}, updated_at = excluded.updated_at"
        ),
        params![value, now()],
    )?;
    Ok(())
}

/// What a run in one chat may read, and what it is told about it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sight {
    /// The folders it is given besides its empty directory.
    pub folders: Vec<PathBuf>,
    /// What is added to its brief, naming what is there. A folder given and
    /// not named is a folder nobody looks in.
    pub brief: Option<String>,
    /// The paths the brief names, for the window to show before a task starts.
    pub named: Vec<String>,
}

/// What a run in `chat` may read, under the person's setting.
///
/// Nothing for a chat about no work, and nothing for a run that reads the web:
/// a page is a stranger's words, and a run that read one has no file tools for
/// them to act through (ADR 0066). A work whose files cannot be found gives a
/// run that sees nothing rather than no run at all: the setting widens what
/// a run can read, and the question it was asked stands without it.
pub fn sight(conn: &Connection, chat: &Chat, sees: RunSees, web: bool) -> Sight {
    if sees == RunSees::Nothing || web {
        return Sight::default();
    }
    let Some(work_id) = chat.work_id.as_deref() else {
        return Sight::default();
    };
    work_sight(conn, work_id).unwrap_or_else(|cause| {
        crate::log::warn(
            "assistant",
            &format!("could not gather the files of {work_id} for a run: {cause}"),
        );
        Sight::default()
    })
}

/// What a run on `work_id` would be given when the setting opens the work's
/// files: its folder on disk when it is there, and what is attached to it.
pub fn work_sight(conn: &Connection, work_id: &str) -> Result<Sight> {
    let Some(work) = crate::work::get(conn, work_id)? else {
        return Ok(Sight::default());
    };
    let config = crate::profile::config_for(conn, &work.profile_id)?;
    let place = crate::folder::locate(conn, &config, &work)?;
    let folder = place
        .path
        .map(PathBuf::from)
        .filter(|folder| folder.is_dir());
    let attached: Vec<PathBuf> = crate::asset::for_work(conn, work_id)?
        .into_iter()
        .map(|asset| PathBuf::from(asset.path))
        .filter(|path| path.is_file())
        .collect();

    let mut folders: Vec<PathBuf> = Vec::new();
    folders.extend(folder.clone());
    for parent in super::stream::folders_of(&attached) {
        if !folders.contains(&parent) {
            folders.push(parent);
        }
    }
    if folders.is_empty() {
        return Ok(Sight::default());
    }

    // English, like everything the model is told.
    let mut brief = String::from(
        "The files of the work this conversation is about. Read them with your tools \
         when the task calls for it. They are the person's own: change nothing there.\n",
    );
    let mut named = Vec::new();
    if let Some(folder) = &folder {
        let path = folder.display().to_string();
        brief.push_str(&format!("\nIts folder: {path}\n"));
        named.push(path);
    }
    if !attached.is_empty() {
        brief.push_str("\nAttached to it:\n");
        for path in attached.iter().take(MOST_FILES_NAMED) {
            let path = path.display().to_string();
            brief.push_str(&format!("- {path}\n"));
            named.push(path);
        }
        if attached.len() > MOST_FILES_NAMED {
            brief.push_str(&format!(
                "- and {} more in the same folder\n",
                attached.len() - MOST_FILES_NAMED
            ));
        }
    }

    Ok(Sight {
        folders,
        brief: Some(brief.trim_end().to_owned()),
        named,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    #[test]
    fn a_workspace_nobody_set_anything_on_reads_the_defaults() {
        let conn = crate::db::open_app_in_memory().unwrap();

        assert_eq!(read(&conn).unwrap(), AssistantSettings::default());
        assert_eq!(read(&conn).unwrap().parallel_runs, 3);
    }

    /// Each setting is its own column: setting one keeps the others, the week's
    /// budget among them.
    #[test]
    fn a_setting_keeps_the_others_where_they_were() {
        let conn = crate::db::open_app_in_memory().unwrap();
        super::super::spend::set_budget(&conn, Some(12.0)).unwrap();

        set_parallel_runs(&conn, 5).unwrap();
        set_model(&conn, Some("sonnet")).unwrap();
        let settings = set_run_sees(&conn, RunSees::Work).unwrap();

        assert_eq!(settings.parallel_runs, 5);
        assert_eq!(settings.model.as_deref(), Some("sonnet"));
        assert_eq!(settings.run_sees, RunSees::Work);
        assert_eq!(super::super::spend::budget(&conn).unwrap(), Some(12.0));
    }

    /// The bounds are one number here and one in the schema; the refusal comes
    /// first, in words, and the schema is what stands behind it.
    #[test]
    fn runs_outside_the_bounds_are_refused_and_change_nothing() {
        let conn = crate::db::open_app_in_memory().unwrap();
        set_parallel_runs(&conn, 4).unwrap();

        for runs in [0, MOST_PARALLEL_RUNS + 1] {
            let refused = set_parallel_runs(&conn, runs).unwrap_err();
            assert_eq!(
                refused.refusal().map(|r| r.code),
                Some("assistant.badParallelRuns")
            );
        }
        assert_eq!(read(&conn).unwrap().parallel_runs, 4);

        assert!(
            conn.execute("UPDATE assistant_settings SET parallel_runs = 9", [])
                .is_err(),
            "the schema holds the same bound"
        );
        assert!(
            set_parallel_runs(&conn, MOST_PARALLEL_RUNS).is_ok(),
            "the bound itself is allowed"
        );
    }

    #[test]
    fn a_model_is_an_alias_a_full_name_or_nothing() {
        let conn = crate::db::open_app_in_memory().unwrap();

        for name in ["opus", "claude-opus-5-5", "claude-opus-5-5[1m]", "us.a:b_c"] {
            assert_eq!(
                set_model(&conn, Some(name)).unwrap().model.as_deref(),
                Some(name)
            );
        }
        assert_eq!(
            set_model(&conn, Some("  sonnet "))
                .unwrap()
                .model
                .as_deref(),
            Some("sonnet"),
            "the spaces a paste brings are not part of the name"
        );
        assert_eq!(
            set_model(&conn, Some("   ")).unwrap().model,
            None,
            "an emptied box is the CLI's own model"
        );
        assert_eq!(set_model(&conn, None).unwrap().model, None);
    }

    /// What would reach the command line of a `.cmd` is refused, and the model
    /// stays as it was.
    #[test]
    fn a_name_that_is_not_a_name_never_reaches_the_command_line() {
        let conn = crate::db::open_app_in_memory().unwrap();
        set_model(&conn, Some("opus")).unwrap();
        let too_long = "a".repeat(LONGEST_MODEL + 1);

        for name in [
            "--dangerously-skip-permissions",
            "opus & calc",
            "opus\nsonnet",
            "\"opus\"",
            "op us",
            too_long.as_str(),
        ] {
            let refused = set_model(&conn, Some(name)).unwrap_err();
            assert_eq!(
                refused.refusal().map(|r| r.code),
                Some("assistant.badModel"),
                "{name}"
            );
        }
        assert_eq!(read(&conn).unwrap().model.as_deref(), Some("opus"));
    }

    /// A song with a folder on disk and a file attached, its chat, and the
    /// directories that hold them.
    fn a_work_with_files() -> (
        Connection,
        Chat,
        PathBuf,
        PathBuf,
        tempfile::TempDir,
        tempfile::TempDir,
    ) {
        let (conn, profile_id) = fixtures::workspace();
        let root = tempfile::tempdir().unwrap();
        crate::folder::set_root(
            &conn,
            &profile_id,
            Some(&root.path().to_string_lossy()),
            "2026-11-05T00:00:00.000Z",
        )
        .unwrap();
        let song = fixtures::song(&conn, &profile_id, "Tide");
        let folder = root.path().join("songs").join("Tide");
        std::fs::create_dir_all(&folder).unwrap();

        let media = tempfile::tempdir().unwrap();
        let picture = media.path().join("still.png");
        std::fs::write(&picture, b"not really a png").unwrap();
        conn.execute(
            "INSERT INTO asset (id, profile_id, work_id, kind, path, created_at)
             VALUES ('a1', ?1, ?2, 'attachment', ?3, '2026-11-05T00:00:00.000Z')",
            params![profile_id, song.id, picture.to_string_lossy()],
        )
        .unwrap();

        let chat = super::super::create(
            &conn,
            &profile_id,
            super::super::NewChat {
                work_id: Some(song.id.clone()),
                ..super::super::NewChat::default()
            },
        )
        .unwrap();
        (conn, chat, folder, picture, root, media)
    }

    /// The setting opens the work's files and says what they are; left alone,
    /// or for a run that reads the web, it opens nothing and says nothing.
    #[test]
    fn a_run_sees_the_works_files_only_when_the_person_said_so() {
        let (conn, chat, folder, picture, _root, _media) = a_work_with_files();

        let sees = sight(&conn, &chat, RunSees::Work, false);
        assert_eq!(
            sees.folders,
            vec![folder.clone(), picture.parent().unwrap().to_path_buf()],
            "its folder and the folder of what is attached"
        );
        let brief = sees.brief.expect("a folder given is a folder named");
        assert!(brief.contains(&folder.display().to_string()), "{brief}");
        assert!(brief.contains(&picture.display().to_string()), "{brief}");
        assert!(
            brief.contains("change nothing"),
            "they are the person's own: {brief}"
        );

        assert_eq!(
            sight(&conn, &chat, RunSees::Nothing, false),
            Sight::default(),
            "the default of ADR 0008 stands"
        );
        assert_eq!(
            sight(&conn, &chat, RunSees::Work, true),
            Sight::default(),
            "a run that reads the web has no file tools, and is not told otherwise"
        );
    }

    /// A chat about no work has no files to open, and a folder not yet made is
    /// not offered: a brief must not send a run looking for nothing.
    #[test]
    fn a_chat_about_nothing_and_a_folder_not_made_open_nothing() {
        let (conn, profile_id) = fixtures::workspace();
        let free =
            super::super::create(&conn, &profile_id, super::super::NewChat::default()).unwrap();
        assert_eq!(sight(&conn, &free, RunSees::Work, false), Sight::default());

        let song = fixtures::song(&conn, &profile_id, "Tide");
        assert_eq!(work_sight(&conn, &song.id).unwrap(), Sight::default());
    }

    #[test]
    fn a_long_list_of_attachments_is_named_in_part() {
        let (conn, chat, _folder, picture, _root, media) = a_work_with_files();
        let work_id = chat.work_id.clone().unwrap();
        let profile_id = chat.profile_id.clone();
        for index in 0..=MOST_FILES_NAMED {
            let path = media.path().join(format!("{index}.png"));
            std::fs::write(&path, b"x").unwrap();
            conn.execute(
                "INSERT INTO asset (id, profile_id, work_id, kind, path, created_at)
                 VALUES (?1, ?2, ?3, 'attachment', ?4, '2026-11-05T00:00:01.000Z')",
                params![
                    format!("b{index}"),
                    profile_id,
                    work_id,
                    path.to_string_lossy()
                ],
            )
            .unwrap();
        }

        let sees = work_sight(&conn, &work_id).unwrap();

        assert!(picture.is_file());
        assert_eq!(
            sees.named.len(),
            MOST_FILES_NAMED + 1,
            "the folder and the first forty"
        );
        assert!(
            sees.brief
                .unwrap()
                .contains("and 2 more in the same folder"),
            "the rest are said to be there"
        );
    }
}
