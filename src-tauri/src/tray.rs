//! The window's other door: the tray, while the window is closed and work is
//! still going (v0.107).
//!
//! Closing the window used to end everything it had started. Now a close
//! with something going hides the window instead ([`close_requested`]), and
//! the tray icon appears in its place: a click on it, Open in its menu, a
//! click on a notification, or starting kilna again brings the window back
//! ([`show_main`]); Quit in its menu leaves, stopping what is left. With
//! nothing going a close is a close, as before.
//!
//! The door is here rather than in the page because it must open whatever
//! the page is doing: a hidden webview still runs, but a door that depends on
//! a script answering is one that sticks. What the tray *says* - its menu,
//! its tooltip, the badge on its icon - is the page's, in the page's
//! language: it sends the words with [`set_tray_words`], and draws the icon
//! through the tray's own API.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Deserialize;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime, State, WindowEvent};

use crate::state::AppState;

/// The tray icon's id, which the page finds it by.
pub const ID: &str = "kilna";

/// The window the tray stands in for.
const MAIN: &str = "main";

/// The event telling the page whether the window is behind the tray.
pub const EVENT: &str = "app:background";

/// What the tray's menu says, in the page's language.
#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
pub struct TrayWords {
    pub open: String,
    pub quit: String,
}

impl Default for TrayWords {
    /// What the menu says before the page has spoken - in practice never seen,
    /// since the page sends its words as soon as it has a language, long
    /// before a window can be closed.
    fn default() -> Self {
        Self {
            open: "Open kilna".into(),
            quit: "Quit kilna".into(),
        }
    }
}

/// The words, kept for the tray built later.
#[derive(Default)]
pub struct Words(Mutex<TrayWords>);

impl Words {
    fn get(&self) -> TrayWords {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

/// The window was asked to close: by its own button, by the keyboard, by the
/// taskbar. With something going it hides behind the tray instead, and the
/// work carries on; otherwise the close goes ahead and the application ends.
pub fn close_requested(window: &tauri::Window, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    if window.label() != MAIN {
        return;
    }
    let app = window.app_handle();
    let busy = !crate::commands::activity::current(app.state::<AppState>().inner()).is_idle();
    if busy {
        api.prevent_close();
        hide_main(app);
    }
}

/// Whether the window was put behind the tray since the application started.
/// The start's watchdog reads it: a window the person closed to the tray is
/// not one that failed to appear (wish 6659).
static PUT_AWAY: AtomicBool = AtomicBool::new(false);

/// Whether the window was put behind the tray since the start.
pub fn put_away() -> bool {
    PUT_AWAY.load(Ordering::SeqCst)
}

/// Whether the start's watchdog should show the window: it never appeared -
/// it is not visible and was not put away. A window hidden behind the tray
/// within the first seconds stays hidden.
pub fn watchdog_shows(visible: bool, put_away: bool) -> bool {
    !visible && !put_away
}

/// Put the window behind the tray.
pub fn hide_main<R: Runtime>(app: &AppHandle<R>) {
    PUT_AWAY.store(true, Ordering::SeqCst);
    if let Err(cause) = tray(app).and_then(|tray| tray.set_visible(true)) {
        // Hiding a window nobody can bring back would lose it: with no tray,
        // the window stays where it is.
        crate::log::error("tray", &format!("no tray to hide behind: {cause}"));
        return;
    }
    if let Some(window) = app.get_webview_window(MAIN) {
        let _ = window.hide();
    }
    let _ = app.emit(EVENT, true);
}

/// Bring the window back, in front, and take the tray away.
pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    if let Some(tray) = app.tray_by_id(ID) {
        let _ = tray.set_visible(false);
    }
    let _ = app.emit(EVENT, false);
}

/// The tray, built the first time it is needed.
fn tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::tray::TrayIcon<R>> {
    if let Some(tray) = app.tray_by_id(ID) {
        return Ok(tray);
    }
    let words = app.state::<Words>().get();
    let mut builder = TrayIconBuilder::with_id(ID)
        .tooltip("kilna")
        .menu(&menu(app, &words)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)
}

fn menu<R: Runtime>(app: &AppHandle<R>, words: &TrayWords) -> tauri::Result<Menu<R>> {
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", &words.open, true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", &words.quit, true, None::<&str>)?,
        ],
    )
}

/// The tray's menu in the page's language: sent at start and whenever the
/// language changes, kept for a tray built later and applied to one that
/// stands.
#[tauri::command]
pub fn set_tray_words(app: AppHandle, words: State<'_, Words>, said: TrayWords) {
    *words
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = said.clone();
    if let Some(tray) = app.tray_by_id(ID)
        && let Ok(menu) = menu(&app, &said)
    {
        let _ = tray.set_menu(Some(menu));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The window closed to the tray within the watchdog's three seconds stays
    /// behind the tray; one that never appeared is shown.
    #[test]
    fn the_watchdog_shows_only_a_window_that_never_appeared() {
        assert!(watchdog_shows(false, false), "never painted: shown");
        assert!(!watchdog_shows(false, true), "put away by the person: left");
        assert!(!watchdog_shows(true, false));
        assert!(!watchdog_shows(true, true));
    }
}
