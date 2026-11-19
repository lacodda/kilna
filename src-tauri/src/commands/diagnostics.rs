//! What to send when something went wrong, and whether a newer kilna is out
//! (v0.162).

use tauri::AppHandle;

use crate::assistant::cli;
use crate::diagnostics::{self, Diagnostics, UpdateCheck};

/// This kilna, this machine, its CLI and the log's last errors.
#[tauri::command(async)]
pub fn diagnostics(app: AppHandle) -> Diagnostics {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from);
    Diagnostics {
        version: app.package_info().version.to_string(),
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        webview: tauri::webview_version().ok(),
        cli: cli::probe().version,
        errors: crate::log::path()
            .map(|log| diagnostics::recent_errors(&log, home.as_deref()))
            .unwrap_or_default(),
    }
}

/// The report as text to paste into an issue.
#[tauri::command(async)]
pub fn diagnostics_text(app: AppHandle) -> String {
    diagnostics::text(&diagnostics(app))
}

/// Whether a newer kilna is out.
#[tauri::command(async)]
pub fn check_update(app: AppHandle) -> UpdateCheck {
    diagnostics::check(&app.package_info().version.to_string())
}
