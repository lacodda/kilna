use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

use rusqlite::Connection;

use crate::assistant::queue::Queue;
use crate::assistant::run::Runs;
use crate::db;
use crate::error::{Error, Result};
use crate::journal;
use crate::profile;
use crate::space::lock::{self, Hold, Holder};
use crate::space::{self, Folder, Manifest, registry};

/// The directory of files beside a space's database, by name.
pub const MEDIA_DIR: &str = "media";

/// Who opens a space, which decides what opening it may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The window: it holds the space (`kilna.lock`), and the runs a window
    /// left behind as `running` died with it, so they are marked broken.
    Window,
    /// `kilna --mcp`, beside the window or alone: it takes no hold, and it
    /// leaves the runs alone - one may be going in the window this minute
    /// (wish 6646).
    Companion,
}

/// The space the window has open.
#[derive(Debug, Clone)]
struct Open {
    id: String,
    folder: Folder,
    /// The machine that holds it, when another does: then it is open for
    /// reading only, and nothing here writes it.
    held_by: Option<Holder>,
}

/// The application's state, shared by every command: its own database, the
/// space open in the window, and the runs in flight.
///
/// A single connection behind a mutex is enough for a space: this is a
/// desktop application with one user, and SQLite writes serialise anyway.
/// The connection is swapped whole when the window opens another space
/// (ADR 0077); a command that holds it finishes in the space it began in.
pub struct AppState {
    /// `app.db`: the spaces this machine knows, and the person's settings.
    app: Mutex<Connection>,
    app_dir: PathBuf,
    connection: Mutex<Connection>,
    open: RwLock<Open>,
    /// Assistant runs in flight. Held apart from the connection on purpose: a
    /// run lasting minutes must not sit on the lock the rest of the
    /// application writes through.
    runs: Arc<Runs>,
    /// Tasks asked for while every slot was taken. Memory only — see
    /// [`crate::assistant::queue`].
    queue: Arc<Queue>,
}

impl AppState {
    /// Start on the application's directory: carry a workspace from before
    /// spaces into them, then open the space `requested` names - a
    /// `space.kilna` double-clicked, or its folder - or the one opened last,
    /// or a first one made now.
    pub fn start(app_dir: &Path, requested: Option<&Path>) -> Result<Self> {
        let app = db::open_app(&app_dir.join(db::APP_DATABASE_FILE))?;
        space::legacy::adopt(app_dir, &app)?;

        let id = match requested {
            Some(path) => {
                let folder = Folder::of(path);
                let manifest = Manifest::read(folder.path())?;
                registry::add(&app, app_dir, &manifest.id, folder.path())?
            }
            None => match registry::newest(&app, app_dir)? {
                Some(space) => space.id,
                None => first_space(&app, app_dir)?,
            },
        };
        let folder = registry::folder(&app, app_dir, &id)?;
        let (conn, held_by) = open_space(&folder, Role::Window)?;
        registry::touch(&app, &id)?;

        let runs = Arc::new(Runs::new());
        apply_limit(&runs, &app);
        Ok(Self {
            app: Mutex::new(app),
            app_dir: app_dir.to_path_buf(),
            connection: Mutex::new(conn),
            open: RwLock::new(Open {
                id,
                folder,
                held_by,
            }),
            runs,
            queue: Arc::new(Queue::new()),
        })
    }

    /// Open one space's database by itself, outside any registry, with the
    /// application's own database in memory - for tests and for a live run
    /// on a copy of a space.
    pub fn open(database: &Path) -> Result<Self> {
        let folder = Folder::new(database.parent().unwrap_or_else(|| Path::new(".")));
        let manifest = Manifest::read(folder.path()).ok();
        let kind = manifest
            .as_ref()
            .map_or(profile::DEFAULT_KIND, |manifest| manifest.kind.as_str())
            .to_owned();
        let mut conn = db::open(database)?;
        prepare(&mut conn, &kind, &folder, Role::Window)?;
        let app = db::open_app_in_memory()?;
        let runs = Arc::new(Runs::new());
        apply_limit(&runs, &app);
        Ok(Self {
            app: Mutex::new(app),
            app_dir: folder.path().to_path_buf(),
            connection: Mutex::new(conn),
            open: RwLock::new(Open {
                id: manifest.map_or_else(|| "local".to_owned(), |manifest| manifest.id),
                folder,
                held_by: None,
            }),
            runs,
            queue: Arc::new(Queue::new()),
        })
    }

    /// Open another space in the window, without a restart: its database
    /// takes the place of the one open, which is let go.
    ///
    /// The caller makes sure nothing is going - a run, a task waiting for a
    /// slot, a plugin - since each would finish into the space it began in
    /// while the window showed another.
    pub fn switch(&self, id: &str) -> Result<()> {
        let folder = registry::folder(&self.app(), &self.app_dir, id)?;
        let (conn, held_by) = open_space(&folder, Role::Window)?;
        let previous = {
            let mut connection = self.conn();
            let mut open = self
                .open
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *connection = conn;
            std::mem::replace(
                &mut *open,
                Open {
                    id: id.to_owned(),
                    folder: folder.clone(),
                    held_by,
                },
            )
        };
        if previous.held_by.is_none() && previous.folder != folder {
            lock::release(&previous.folder);
        }
        registry::touch(&self.app(), id)?;
        Ok(())
    }

    /// Borrow the open space's connection.
    ///
    /// A poisoned mutex means another command panicked mid-query. The connection
    /// itself is still usable, so recover rather than take the whole app down.
    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Borrow the application's own database.
    pub fn app(&self) -> MutexGuard<'_, Connection> {
        self.app
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The application's directory: `app.db`, the log, the plugins, and the
    /// spaces it made.
    pub fn app_dir(&self) -> &Path {
        &self.app_dir
    }

    fn open_now(&self) -> Open {
        self.open
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// The id of the open space.
    pub fn space_id(&self) -> String {
        self.open_now().id
    }

    /// The open space's folder.
    pub fn folder(&self) -> Folder {
        self.open_now().folder
    }

    /// The machine that holds the open space, when another does.
    pub fn held_by(&self) -> Option<Holder> {
        self.open_now().held_by
    }

    /// Where the open space's database is.
    pub fn path(&self) -> PathBuf {
        self.folder().database()
    }

    /// Say the window still holds its space - once a minute, from the
    /// window's own clock.
    pub fn beat(&self) {
        let open = self.open_now();
        if open.held_by.is_none()
            && let Err(cause) = lock::beat(&open.folder)
        {
            crate::log::warn("space", &format!("could not renew the hold: {cause}"));
        }
    }

    /// Let the open space go, as the application ends.
    pub fn release(&self) {
        let open = self.open_now();
        if open.held_by.is_none() {
            lock::release(&open.folder);
        }
    }

    /// Where the files a person attaches live: `media/`, in the space's folder.
    ///
    /// Beside rather than inside: the database holds the path and the bytes
    /// stay on disk (ADR 0002), because binaries in the database bloat every
    /// backup and every future sync of it. Beside rather than anywhere else
    /// on the machine, so that one directory is the whole space — the backup
    /// takes it with the database (decision of 2026-09-15), and moving a
    /// space is moving a folder (ADR 0077).
    ///
    /// An error rather than `None`: a run with no directory can still start
    /// where kilna did, but a file with nowhere to go has nowhere to go.
    pub fn media_dir(&self) -> Result<PathBuf> {
        media_dir_beside(&self.path())
    }

    /// Where assistant runs start: a directory in the space's folder that is
    /// kept empty on purpose.
    ///
    /// A spawned CLI inherits kilna's own working directory otherwise — in
    /// development that is `src-tauri`, and a run asked about "the README"
    /// went and read kilna's sources. Everything a run should know arrives in
    /// the prompt, so what it sees from its directory is pure exposure. This
    /// is a default, not a sandbox — see ADR 0008.
    ///
    /// `None` when the directory cannot be prepared: the run then starts
    /// wherever kilna did, which is worse but not worth refusing the run over.
    pub fn assistant_dir(&self) -> Option<PathBuf> {
        let dir = self.folder().path().join("assistant");
        if let Err(cause) = std::fs::create_dir_all(&dir) {
            crate::log::warn(
                "assistant",
                &format!("could not prepare the run directory: {cause}"),
            );
            return None;
        }
        Some(dir)
    }

    pub fn runs(&self) -> &Arc<Runs> {
        &self.runs
    }

    pub fn queue(&self) -> &Arc<Queue> {
        &self.queue
    }

    /// What opens connections of their own to the open space, for work that
    /// runs alongside the commands - into the space it began in, whatever
    /// the window opens after.
    ///
    /// A streamed assistant run writes every few seconds for as long as it
    /// lasts; going through [`Self::conn`] would hold the shared lock across a
    /// process it does not control.
    pub fn alongside(&self) -> Alongside {
        let open = self.open_now();
        Alongside {
            folder: open.folder,
            read_only: open.held_by.is_some(),
        }
    }
}

/// Connections to one space, for a run that writes beside the commands.
#[derive(Debug, Clone)]
pub struct Alongside {
    folder: Folder,
    read_only: bool,
}

impl Alongside {
    pub fn open(&self) -> Option<Connection> {
        let opened = if self.read_only {
            db::open_read_only(&self.folder.database())
        } else {
            db::open(&self.folder.database())
        };
        match opened {
            Ok(conn) => Some(conn),
            Err(cause) => {
                crate::log::error(
                    "assistant",
                    &format!("could not open the space for a run: {cause}"),
                );
                None
            }
        }
    }
}

/// How many runs may go at once, as the person set it.
///
/// A row that cannot be read leaves the default: nothing about it is worth not
/// opening the application over.
fn apply_limit(runs: &Runs, app: &Connection) {
    match crate::assistant::settings::read(app) {
        Ok(settings) => runs.set_limit(settings.parallel_runs),
        Err(cause) => crate::log::warn(
            "assistant",
            &format!("could not read the assistant's settings: {cause}"),
        ),
    }
}

/// The space a machine with none starts in: Studio, its name asked the first
/// time the window opens it.
pub(crate) fn first_space(app: &Connection, app_dir: &Path) -> Result<String> {
    let made = space::create(
        app,
        app_dir,
        &space::NewSpace {
            name: "Studio".into(),
            kind: profile::DEFAULT_KIND.into(),
            colour: None,
            mark: None,
        },
        None,
    )?;
    registry::ask_name(app, &made.id)?;
    Ok(made.id)
}

/// Open the database of the space at `folder`: for writing, prepared, when
/// nobody else holds it; for reading only when another machine does.
pub fn open_space(folder: &Folder, role: Role) -> Result<(Connection, Option<Holder>)> {
    let manifest = Manifest::read(folder.path())?;
    if !folder.database().is_file() {
        return Err(
            Error::refused("space.noDatabase").param("path", folder.path().display().to_string())
        );
    }
    let hold = match role {
        Role::Window => lock::take(folder)?,
        Role::Companion => lock::check(folder),
    };
    match hold {
        Hold::Elsewhere(holder) => {
            crate::log::info(
                "space",
                &format!(
                    "{} is held by {}: open for reading only",
                    folder.path().display(),
                    holder.machine
                ),
            );
            Ok((db::open_read_only(&folder.database())?, Some(holder)))
        }
        Hold::Ours => {
            let mut conn = db::open(&folder.database())?;
            prepare(&mut conn, &manifest.kind, folder, role)?;
            Ok((conn, None))
        }
    }
}

/// What opening a space for writing does before anything reads it: the
/// profile seeded or carried forward, the upgrades that run once, the files
/// found where the folder is now, and the sweeps of what nothing removes.
fn prepare(conn: &mut Connection, kind: &str, folder: &Folder, role: Role) -> Result<()> {
    profile::seed(conn, kind)?;
    // The style dictionary's starter set, after the profiles: a set brick
    // is of a type the carried-forward document names (ADR 0048).
    crate::style_set::seed(conn)?;
    // Publications named the way v0.90 names them, once, and the fields
    // of their releases given the tails their doors keep (ADR 0051).
    // After the seed on purpose: the carry-forward is what gives a stored
    // field its tail.
    crate::publication::upgrade(conn)?;
    // Every work made from another before v0.101 given the number its
    // title carries, once (ADR 0070). After the renaming above: the
    // numbers are read off the titles v0.90 gave.
    crate::ordinal::upgrade(conn)?;
    // A work whose current version is no draft of it - a style prompt, a
    // brief - points at its newest draft instead (v0.99). After the seed:
    // what a role is, is read off the carried-forward profile.
    crate::work::version::settle_current(conn)?;
    // A folder that moved - into a space by v0.109, or to another machine -
    // finds its files where they are now (ADR 0077).
    match crate::asset::rebase(conn, &folder.path().join(MEDIA_DIR)) {
        Ok(0) => {}
        Ok(moved) => crate::log::info(
            "space",
            &format!("{moved} file path(s) now name this space's media"),
        ),
        Err(cause) => crate::log::warn("space", &format!("could not rebase the files: {cause}")),
    }

    // Old, already-read journal entries go at startup rather than on a
    // timer: there is no scheduler in this app, and an app that is left open
    // for a fortnight can carry a fortnight of history without complaint.
    // Failing to sweep is not a reason to fail to start.
    if let Ok(Some(profile)) = profile::active(conn)
        && let Err(cause) = journal::sweep(conn, &profile.id)
    {
        crate::log::warn("journal", &format!("could not sweep old entries: {cause}"));
    }

    // Dismissed complaints about works that have since been deleted. They
    // are harmless — nothing raises a complaint about a missing work — but
    // nothing removes them either, since the table deliberately holds no
    // foreign key.
    if let Err(cause) = crate::focus::sweep(conn) {
        crate::log::warn(
            "focus",
            &format!("could not sweep dismissals for deleted works: {cause}"),
        );
    }

    // What the person said about repeats in songs that are gone for good.
    if let Err(cause) = crate::register::guard::sweep(conn) {
        crate::log::warn(
            "register",
            &format!("could not sweep kept repeats of deleted works: {cause}"),
        );
    }

    // Runs the previous life of the window was carrying died with it.
    // Leaving their rows as `running` would show work that nothing is doing.
    // Only the window says so: a companion opened while a run goes would
    // mark a live run broken (wish 6646).
    if role == Role::Window
        && let Err(cause) = crate::assistant::run::sweep(conn)
    {
        crate::log::warn(
            "assistant",
            &format!("could not sweep abandoned runs: {cause}"),
        );
    }
    Ok(())
}

/// The directory of files beside a space's database, made when missing.
pub fn media_dir_beside(database: &Path) -> Result<PathBuf> {
    let dir = database
        .parent()
        .ok_or_else(|| Error::Internal("the workspace has no directory to keep files in".into()))?
        .join(MEDIA_DIR);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// The directory of files of the workspace a connection is open on - for a
/// gesture that reaches a file without the window's state in hand, such as
/// keeping a proposal that brings pictures.
pub fn media_dir_of(conn: &Connection) -> Result<PathBuf> {
    match conn.path().filter(|path| !path.is_empty()) {
        Some(path) => media_dir_beside(Path::new(path)),
        None => Err(Error::Internal(
            "a workspace held in memory has no directory to keep files in".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_space(name: &str, kind: &str) -> space::NewSpace {
        space::NewSpace {
            name: name.into(),
            kind: kind.into(),
            colour: None,
            mark: None,
        }
    }

    /// The limit a person set is the one the next start of kilna works under,
    /// whatever space it opens.
    #[test]
    fn the_limit_a_person_set_stands_when_the_application_starts_again() {
        let dir = tempfile::tempdir().unwrap();
        {
            let state = AppState::start(dir.path(), None).unwrap();
            assert_eq!(
                state.runs().limit(),
                3,
                "three until someone says otherwise"
            );
            crate::assistant::settings::set_parallel_runs(&state.app(), 6).unwrap();
        }

        let state = AppState::start(dir.path(), None).unwrap();

        assert_eq!(state.runs().limit(), 6);
    }

    #[test]
    fn a_first_start_makes_a_studio_and_asks_its_name() {
        let dir = tempfile::tempdir().unwrap();

        let state = AppState::start(dir.path(), None).unwrap();

        let open = registry::get(&state.app(), dir.path(), &state.space_id(), None).unwrap();
        assert_eq!(open.name, "Studio");
        assert_eq!(open.kind.as_deref(), Some("music"));
        assert!(open.ask_name);
        assert!(
            state
                .folder()
                .path()
                .starts_with(dir.path().join(space::SPACES_DIR))
        );
        assert!(lock::read(&state.folder()).is_some(), "the window holds it");
    }

    #[test]
    fn a_start_opens_the_space_opened_last() {
        let dir = tempfile::tempdir().unwrap();
        let second = {
            let state = AppState::start(dir.path(), None).unwrap();
            let second = space::create(
                &state.app(),
                dir.path(),
                &new_space("Second", "novel"),
                None,
            )
            .unwrap();
            state.switch(&second.id).unwrap();
            state.release();
            second
        };

        let state = AppState::start(dir.path(), None).unwrap();

        assert_eq!(state.space_id(), second.id);
        assert_eq!(
            profile::active(&state.conn()).unwrap().unwrap().key,
            "novel"
        );
    }

    #[test]
    fn switching_opens_the_other_database_and_lets_the_first_go() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::start(dir.path(), None).unwrap();
        let first = state.folder();
        let profile_id = profile::active(&state.conn()).unwrap().unwrap().id;
        let song = crate::fixtures::song(&state.conn(), &profile_id, "Harbour lights");
        let other =
            space::create(&state.app(), dir.path(), &new_space("Blog", "blog"), None).unwrap();

        state.switch(&other.id).unwrap();

        assert_eq!(state.space_id(), other.id);
        assert!(crate::work::get(&state.conn(), &song.id).unwrap().is_none());
        assert_eq!(profile::active(&state.conn()).unwrap().unwrap().key, "blog");
        assert_eq!(lock::read(&first), None, "the first is let go");
        assert!(lock::read(&state.folder()).is_some());
        assert!(
            state
                .media_dir()
                .unwrap()
                .starts_with(Path::new(&other.path))
        );
    }

    #[test]
    fn a_double_clicked_space_is_registered_and_opened() {
        let dir = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let made = {
            let state = AppState::start(dir.path(), None).unwrap();
            let made = space::create(
                &state.app(),
                dir.path(),
                &new_space("Carried", "podcast"),
                Some(elsewhere.path()),
            )
            .unwrap();
            registry::trash(&state.app(), &made.id, &state.space_id()).unwrap();
            registry::forget(&state.app(), &made.id).unwrap();
            state.release();
            made
        };
        let file = Path::new(&made.path).join(space::MANIFEST_FILE);

        let state = AppState::start(dir.path(), Some(&file)).unwrap();

        assert_eq!(state.space_id(), made.id);
        assert_eq!(
            profile::active(&state.conn()).unwrap().unwrap().key,
            "podcast"
        );
    }

    /// A space another machine holds opens for reading: every write is
    /// refused in one sentence, and the hold is not taken from it.
    #[test]
    fn a_space_held_elsewhere_opens_for_reading_only() {
        let dir = tempfile::tempdir().unwrap();
        let (id, folder) = {
            let state = AppState::start(dir.path(), None).unwrap();
            state.release();
            (state.space_id(), state.folder())
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        std::fs::write(
            folder.path().join(lock::LOCK_FILE),
            serde_json::to_string(&Holder {
                machine: "another-machine".into(),
                since: now,
                beat: now,
            })
            .unwrap(),
        )
        .unwrap();

        let state = AppState::start(dir.path(), None).unwrap();

        assert_eq!(state.space_id(), id);
        assert_eq!(state.held_by().unwrap().machine, "another-machine");
        let refused = state
            .conn()
            .execute("UPDATE profile SET name = 'Not here'", [])
            .map_err(Error::from)
            .unwrap_err();
        assert_eq!(refused.refusal().unwrap().code, "space.readOnly");
        state.release();
        assert_eq!(lock::read(&folder).unwrap().machine, "another-machine");
    }

    #[test]
    fn a_connection_finds_the_files_beside_its_database() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::open(&dir.path().join("workspace.db")).unwrap();
        let from_conn = media_dir_of(&state.conn()).unwrap();
        assert_eq!(
            std::fs::canonicalize(from_conn).unwrap(),
            std::fs::canonicalize(state.media_dir().unwrap()).unwrap()
        );
    }

    #[test]
    fn the_assistant_directory_is_created_next_to_the_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::open(&dir.path().join("workspace.db")).unwrap();

        let assistant = state.assistant_dir().unwrap();

        assert!(assistant.is_dir());
        assert_eq!(assistant.parent(), Some(dir.path()));
    }

    #[test]
    fn the_assistant_directory_survives_being_asked_for_twice() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::open(&dir.path().join("workspace.db")).unwrap();

        assert_eq!(state.assistant_dir(), state.assistant_dir());
    }
}
