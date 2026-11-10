//! Spaces (v0.109, ADR 0077): the folders this machine knows, and opening
//! another in the window without a restart.
//!
//! None of this writes a space's workspace: what a space is lives in its
//! `space.kilna`, and where it is lives in the application's database - this
//! machine's record, kept off the operations log as the open profile was.

use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::error::{Error, Result};
use crate::space::lock::Holder;
use crate::space::{self, Folder, Manifest, NewSpace, Space, SpaceLook, registry};
use crate::state::AppState;

/// The space the window has open, with what the window says about it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct CurrentSpace {
    pub space: Space,
    /// The machine that holds it, when another does: it is open for reading.
    pub held_by: Option<Holder>,
    /// This copy of kilna keeps its data beside its executable.
    pub portable: bool,
}

/// What a link or a double-click asked the window to open, once the space it
/// names is open: where to go in it.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Opened {
    pub current: CurrentSpace,
    /// The screen the link names - `/works/<id>` - or none for the space alone.
    pub route: Option<String>,
}

/// What kilna was started with that the window opens once it stands: a link
/// (`kilna://...`) handed to the first start.
#[derive(Debug, Default)]
pub struct Launch(pub Mutex<Option<String>>);

fn current(state: &AppState) -> Result<CurrentSpace> {
    let id = state.space_id();
    Ok(CurrentSpace {
        space: registry::get(&state.app(), state.app_dir(), &id, Some(&id))?,
        held_by: state.held_by(),
        portable: space::portable_dir().is_some(),
    })
}

/// Let the window show the files of the open space, and of the media folders
/// it keeps on this machine: granted when a space opens, because which folder
/// that is is known only then.
pub fn grant(app: &AppHandle, state: &AppState) {
    match state.media_dir() {
        Ok(media) => {
            if let Err(cause) = app.asset_protocol_scope().allow_directory(&media, true) {
                crate::log::error(
                    "assets",
                    &format!("the window may not read {}: {cause}", media.display()),
                );
            }
        }
        Err(cause) => crate::log::error("assets", &format!("no directory for files: {cause}")),
    }
    match crate::folder::roots(&state.conn()) {
        Ok(roots) => {
            for root in roots {
                super::folders::allow(app, &root);
            }
        }
        Err(cause) => crate::log::error("folder", &format!("no media folders read: {cause}")),
    }
}

/// Refuse to leave a space while something is going in it: a run, a task in
/// line for a slot, a plugin. Each would finish into the space it began in
/// while the window showed another.
fn nothing_going(state: &AppState) -> Result<()> {
    let going = state.runs().active() + state.queue().len() + super::plugins::running_count();
    if going > 0 {
        return Err(Error::refused("space.busy").param("count", going));
    }
    Ok(())
}

/// Every space this machine knows, the trashed ones too, the newest opened
/// first.
#[tauri::command]
pub fn list_spaces(state: State<'_, AppState>) -> Result<Vec<Space>> {
    registry::list(&state.app(), state.app_dir(), Some(&state.space_id()))
}

/// The space the window has open.
#[tauri::command]
pub fn current_space(state: State<'_, AppState>) -> Result<CurrentSpace> {
    current(&state)
}

/// Make a space: a name and a kind, a colour and a mark if chosen. Made and
/// listed, not opened - opening is its own gesture.
#[tauri::command]
pub fn create_space(state: State<'_, AppState>, space: NewSpace) -> Result<Space> {
    space::create(&state.app(), state.app_dir(), &space, None)
}

/// Open another space in the window.
#[tauri::command]
pub fn open_space(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<CurrentSpace> {
    if id != state.space_id() {
        nothing_going(&state)?;
        state.switch(&id)?;
        grant(&app, &state);
    }
    current(&state)
}

/// Name a space - and, for one kilna named, stop asking.
#[tauri::command]
pub fn rename_space(state: State<'_, AppState>, id: String, name: String) -> Result<Space> {
    let app = state.app();
    let folder = registry::folder(&app, state.app_dir(), &id)?;
    space::rename(&folder, &name)?;
    registry::named(&app, &id)?;
    registry::get(&app, state.app_dir(), &id, Some(&state.space_id()))
}

/// Give a space another colour or mark.
#[tauri::command]
pub fn restyle_space(state: State<'_, AppState>, id: String, look: SpaceLook) -> Result<Space> {
    let app = state.app();
    let folder = registry::folder(&app, state.app_dir(), &id)?;
    space::restyle(&folder, &look)?;
    registry::get(&app, state.app_dir(), &id, Some(&state.space_id()))
}

/// Put a space in the trash: off the switcher, its folder untouched.
#[tauri::command]
pub fn trash_space(state: State<'_, AppState>, id: String) -> Result<()> {
    registry::trash(&state.app(), &id, &state.space_id())
}

/// Take a space back out of the trash.
#[tauri::command]
pub fn restore_space(state: State<'_, AppState>, id: String) -> Result<()> {
    registry::restore(&state.app(), &id)
}

/// Forget a space in the trash. Its folder stays on disk.
#[tauri::command]
pub fn forget_space(state: State<'_, AppState>, id: String) -> Result<()> {
    registry::forget(&state.app(), &id)
}

/// The link the first start was handed, once: the window opens it when it
/// stands.
#[tauri::command]
pub fn take_launch_target(launch: State<'_, Launch>) -> Option<String> {
    launch
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
}

/// Open what a link or a double-click names: a `space.kilna` (or its folder),
/// or `kilna://<space>/work/<id>` - the space opened when it is not, and the
/// screen to go to in it.
#[tauri::command]
pub fn open_target(app: AppHandle, state: State<'_, AppState>, target: String) -> Result<Opened> {
    let (id, route) = match parse_link(&target) {
        Some(link) => {
            let space = registry::by_name(&state.app(), state.app_dir(), &link.space)?;
            (space.id, link.route)
        }
        None => {
            let folder = Folder::of(Path::new(target.trim()));
            let manifest = Manifest::read(folder.path())?;
            let id = registry::add(&state.app(), state.app_dir(), &manifest.id, folder.path())?;
            registry::restore(&state.app(), &id)?;
            (id, None)
        }
    };
    if id != state.space_id() {
        nothing_going(&state)?;
        state.switch(&id)?;
        grant(&app, &state);
    }
    Ok(Opened {
        current: current(&state)?,
        route,
    })
}

/// A link into kilna, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// The space, by its name.
    pub space: String,
    /// Where in it, as the window's address.
    pub route: Option<String>,
}

/// The scheme kilna's links are written in.
pub const SCHEME: &str = "kilna";

/// Read `kilna://<space>/work/<id>[/<tab>]` (#15). The space is its name,
/// percent-encoded as a link must carry a space or a letter beyond ASCII;
/// what follows names a work, and anything else opens the space alone.
pub fn parse_link(target: &str) -> Option<Link> {
    let rest = target.trim().strip_prefix(SCHEME)?.strip_prefix("://")?;
    let (space, path) = rest.split_once('/').unwrap_or((rest, ""));
    let space = percent_decoded(space)?;
    if space.trim().is_empty() {
        return None;
    }
    let parts: Vec<&str> = path
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    let safe = |part: &str| {
        part.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    };
    let route = match parts.as_slice() {
        ["work", id] if safe(id) => Some(format!("/works/{id}")),
        ["work", id, tab] if safe(id) && safe(tab) => Some(format!("/works/{id}/{tab}")),
        _ => None,
    };
    Some(Link {
        space: space.trim().to_owned(),
        route,
    })
}

/// `%XX` turned back into the bytes it stands for, read as UTF-8; none when
/// it is not.
fn percent_decoded(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'%' => {
                let hex = text.get(at + 1..at + 3)?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                at += 3;
            }
            b'+' => {
                out.push(b' ');
                at += 1;
            }
            byte => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// What in the arguments kilna was started with names something to open: a
/// link, or a `space.kilna` (or the folder holding one).
pub fn target_in(args: &[String]) -> Option<String> {
    let mut skip = false;
    for arg in args {
        if skip {
            skip = false;
            continue;
        }
        if arg == "--workspace" {
            skip = true;
            continue;
        }
        if arg.starts_with("--") {
            continue;
        }
        if parse_link(arg).is_some() {
            return Some(arg.clone());
        }
        let path = Path::new(arg);
        let names_a_space = path
            .file_name()
            .is_some_and(|name| name == space::MANIFEST_FILE)
            || path.join(space::MANIFEST_FILE).is_file();
        if names_a_space {
            return Some(arg.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_names_a_space_and_a_work_in_it() {
        assert_eq!(
            parse_link("kilna://Night%20Shift/work/abc-123"),
            Some(Link {
                space: "Night Shift".into(),
                route: Some("/works/abc-123".into()),
            })
        );
        assert_eq!(
            parse_link("kilna://%D0%91%D0%BB%D0%BE%D0%B3/work/w1/score"),
            Some(Link {
                space: "Блог".into(),
                route: Some("/works/w1/score".into()),
            })
        );
        assert_eq!(
            parse_link("kilna://Studio"),
            Some(Link {
                space: "Studio".into(),
                route: None,
            })
        );
        assert_eq!(
            parse_link("kilna://Studio/work/w1?from=mail")
                .unwrap()
                .route
                .as_deref(),
            Some("/works/w1")
        );
    }

    #[test]
    fn what_is_not_a_link_into_kilna_is_not_read_as_one() {
        for target in [
            "https://example.com/work/1",
            "kilna:/Studio",
            "kilna://",
            "kilna://%ZZ/work/1",
            "C:/spaces/blog/space.kilna",
        ] {
            assert_eq!(parse_link(target), None, "{target}");
        }
        assert_eq!(
            parse_link("kilna://Studio/work/../../etc").unwrap().route,
            None,
            "a path that would step out is no route"
        );
        assert_eq!(parse_link("kilna://Studio/work/a%2Fb").unwrap().route, None);
    }

    #[test]
    fn the_arguments_name_a_link_or_a_space_file() {
        let args = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            target_in(&args(&["--workspace", "D:/copy", "kilna://Studio/work/w1"])),
            Some("kilna://Studio/work/w1".into())
        );
        assert_eq!(
            target_in(&args(&["C:/spaces/blog/space.kilna"])),
            Some("C:/spaces/blog/space.kilna".into())
        );
        assert_eq!(target_in(&args(&["--workspace", "D:/copy"])), None);
        assert_eq!(target_in(&args(&["--mcp"])), None);
    }
}
