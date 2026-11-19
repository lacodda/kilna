//! The armed receiver's commands (v0.156, FR-21 21.2 p.1): arm it for a
//! work, put it down, say what waits, take what arrived, and how it works on
//! this machine.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, Emitter, Manager, State};

use crate::actions::media::{self, MediaRole, TakenMedia};
use crate::error::{Error, Result};
use crate::receiver::{self, ArmedReceiver, ReceiverEnded, ReceiverFound, ReceiverSettings};
use crate::state::AppState;

/// How often the folder is looked at.
const LOOK_EVERY: Duration = Duration::from_secs(1);

/// How the receiver works on this machine.
#[tauri::command]
pub fn receiver_settings(state: State<'_, AppState>) -> ReceiverSettings {
    receiver::settings(state.app_dir())
}

/// Keep how the receiver works on this machine.
#[tauri::command]
pub fn set_receiver_settings(
    state: State<'_, AppState>,
    settings: ReceiverSettings,
) -> Result<ReceiverSettings> {
    receiver::set_settings(state.app_dir(), settings)
}

/// The receiver waiting now, if one is.
#[tauri::command]
pub fn armed_receiver() -> Option<ArmedReceiver> {
    receiver::armed()
}

/// Put the receiver down.
#[tauri::command]
pub fn disarm_receiver() {
    receiver::disarm();
}

/// Arm the receiver for a work's track or cover: the folder is looked at
/// every second for a file of that kind that arrives from now on, until one
/// does, the receiver is put down or another armed, or its time runs out.
#[tauri::command]
pub fn arm_receiver(
    app: AppHandle,
    state: State<'_, AppState>,
    work_id: String,
    role: MediaRole,
) -> Result<ArmedReceiver> {
    let settings = receiver::settings(state.app_dir());
    let folder = match settings.folder.as_deref() {
        Some(folder) => PathBuf::from(folder),
        None => app
            .path()
            .download_dir()
            .map_err(|_| Error::refused("receiver.noDownloads"))?,
    };
    if !folder.is_dir() {
        return Err(Error::refused("receiver.noFolder").param("path", folder.display().to_string()));
    }
    let work = crate::work::get(&state.conn(), &work_id)?
        .ok_or_else(|| Error::not_found("work", work_id.clone()))?;
    let since = SystemTime::now();
    let lasts = Duration::from_secs(u64::from(settings.minutes) * 60);
    let until = time::OffsetDateTime::now_utc() + lasts;
    let waiting = ArmedReceiver {
        work_id: work.id.clone(),
        title: work.title.clone(),
        role,
        folder: folder.to_string_lossy().into_owned(),
        until: until
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
    };
    let number = receiver::arm(waiting.clone());
    let watched = waiting.clone();
    std::thread::spawn(move || watch(&app, number, &watched, &folder, since, lasts, settings.auto));
    Ok(waiting)
}

/// Look at the folder until a file arrives, the receiver is put down, or its
/// time runs out.
fn watch(
    app: &AppHandle,
    number: u64,
    waiting: &ArmedReceiver,
    folder: &std::path::Path,
    since: SystemTime,
    lasts: Duration,
    auto: bool,
) {
    let mut sizes = HashMap::new();
    loop {
        std::thread::sleep(LOOK_EVERY);
        if !receiver::still(number) {
            return;
        }
        if since.elapsed().unwrap_or_default() > lasts {
            if receiver::disarm_if(number) {
                let _ = app.emit(
                    receiver::ENDED_EVENT,
                    ReceiverEnded {
                        work_id: waiting.work_id.clone(),
                        reason: "expired".into(),
                    },
                );
            }
            return;
        }
        let Some(path) = receiver::arrived(folder, since, waiting.role, &mut sizes) else {
            continue;
        };
        if !receiver::disarm_if(number) {
            return;
        }
        // TakenMedia without asking only when the person said so; when that
        // fails, the window asks as it would have.
        let taken = auto
            .then(|| {
                let state = app.state::<AppState>();
                let media_dir = state.media_dir().ok()?;
                media::take(
                    &state.conn(),
                    &media_dir,
                    &waiting.work_id,
                    &path,
                    waiting.role,
                    true,
                )
                .ok()
            })
            .flatten();
        let _ = app.emit(
            receiver::FOUND_EVENT,
            ReceiverFound {
                work_id: waiting.work_id.clone(),
                title: waiting.title.clone(),
                role: waiting.role,
                name: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                path: path.to_string_lossy().into_owned(),
                taken,
            },
        );
        return;
    }
}

/// Move a file that arrived into the work, under a name of the work's, as
/// what it was waited for.
#[tauri::command(async)]
pub fn take_received(
    state: State<'_, AppState>,
    work_id: String,
    role: MediaRole,
    path: String,
) -> Result<TakenMedia> {
    let media_dir = state.media_dir()?;
    media::take(
        &state.conn(),
        &media_dir,
        &work_id,
        std::path::Path::new(&path),
        role,
        true,
    )
}
