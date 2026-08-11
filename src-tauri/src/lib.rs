pub mod commands;
pub mod db;
pub mod error;
pub mod note;
pub mod profile;
pub mod score;
pub mod state;
pub mod time;
pub mod work;

pub use error::{Error, Result};

use tauri::Manager;

/// Build and run the desktop application.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Per-user application data, resolved by Tauri for the current platform.
            let data_dir = app.path().app_data_dir()?;
            let state = state::AppState::open(&db::default_path(&data_dir))?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_workspace,
            commands::list_profiles,
            commands::activate_profile,
            commands::list_works,
            commands::get_work,
            commands::create_work,
            commands::update_work,
            commands::delete_work,
            commands::list_versions,
            commands::get_version,
            commands::create_version,
            commands::set_current_version,
            commands::delete_version,
            commands::list_notes,
            commands::create_note,
            commands::update_note,
            commands::delete_note,
            commands::list_tags,
            commands::score_work,
            commands::score_history,
            commands::latest_score,
            commands::delete_score,
            commands::catalogue,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start kilna");
}
